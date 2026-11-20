/*!
This module defines the interface and some standard implementations of [Executor]
as well as some utility functions and an [`ExecutorRegistry`] type.
*/
pub(crate) mod commands;
pub mod hpc;
pub mod inmemory;
pub mod interface;
pub mod nexus;
#[cfg(unix)]
pub mod qrmi;
pub mod subprocess;

pub use crate::executor::hpc::HPCExecutor;
pub use crate::executor::hpc::{
    pbs::PbsWrapper,
    pjsub::PjsubWrapper,
    slurm::SlurmWrapper,
    spec::{JobSpec, SchedulerWrapper},
};
pub use crate::executor::inmemory::InMemoryExecutor;
pub use crate::executor::interface::Executor;
pub use crate::executor::nexus::NexusExecutor;
#[cfg(unix)]
pub use crate::executor::qrmi::{QrmiExecutor, QrmiExecutorConfig, QrmiPayload, QrmiResourceType};
pub use crate::executor::subprocess::SubprocessExecutor;

use std::{any::Any, collections::HashMap, future::Future, panic::AssertUnwindSafe, sync::Arc};

use futures::FutureExt;
use tokio::{sync::watch, task::AbortHandle};

/// [`ExecutorRegistry`] is sharable mapping of configured [Executor] names
/// to various implementations.
///
/// Note that it is possible to have multiple instances of the same [Executor]
/// implementation with different names in order to use different configurations.
pub type ExecutorRegistry = Arc<HashMap<String, Box<dyn Executor>>>;

/// Health of an executor's long-lived dispatcher task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutorHealth {
    /// The dispatcher is accepting and monitoring tasks.
    Running,
    /// The dispatcher returned without the executor being dropped.
    Stopped,
    /// The dispatcher panicked.
    Panicked(String),
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload.downcast_ref::<&str>().map_or_else(
        || {
            payload
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| "unknown panic payload".to_string())
        },
        |message| (*message).to_string(),
    )
}

pub(crate) fn spawn_supervised_executor<F>(
    future: F,
) -> (AbortHandle, watch::Receiver<ExecutorHealth>)
where
    F: Future<Output = ()> + Send + 'static,
{
    let (health_sender, health_receiver) = watch::channel(ExecutorHealth::Running);
    let task = tokio::spawn(async move {
        let health = match AssertUnwindSafe(future).catch_unwind().await {
            Ok(()) => ExecutorHealth::Stopped,
            Err(payload) => ExecutorHealth::Panicked(panic_message(&*payload)),
        };
        health_sender.send_replace(health);
    });
    (task.abort_handle(), health_receiver)
}

#[cfg(test)]
mod tests {
    use std::future;

    use super::*;

    #[tokio::test]
    async fn executor_supervisor_reports_stopped() {
        let (_abort, mut health) = spawn_supervised_executor(async {});
        health
            .changed()
            .await
            .expect("supervisor should report completion");
        assert_eq!(*health.borrow(), ExecutorHealth::Stopped);
    }

    #[tokio::test]
    async fn executor_supervisor_reports_panic() {
        let (_abort, mut health) = spawn_supervised_executor(async {
            panic!("dispatcher failed");
        });
        health
            .changed()
            .await
            .expect("supervisor should report panic");
        assert_eq!(
            *health.borrow(),
            ExecutorHealth::Panicked("dispatcher failed".to_string())
        );
    }

    #[tokio::test]
    async fn aborting_executor_supervisor_is_not_a_failure() {
        let (abort, health) = spawn_supervised_executor(future::pending());
        abort.abort();
        tokio::task::yield_now().await;
        assert_eq!(*health.borrow(), ExecutorHealth::Running);
    }
}
