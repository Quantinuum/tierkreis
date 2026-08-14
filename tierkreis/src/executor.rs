/*!
This module defines the interface and some standard implementations of [Executor]
as well as some utility functions and an [`ExecutorRegistry`] type.
*/
pub mod inmemory;
pub mod interface;
pub mod nexus;
pub mod subprocess;

pub use crate::executor::inmemory::InMemoryExecutor;
pub use crate::executor::interface::Executor;
pub use crate::executor::nexus::NexusExecutor;
pub use crate::executor::subprocess::SubprocessExecutor;

use std::{collections::HashMap, sync::Arc};

/// [`ExecutorRegistry`] is sharable mapping of configured [Executor] names
/// to various implementations.
///
/// Note that it is possible to have multiple instances of the same [Executor]
/// implementation with different names in order to use different configurations.
pub type ExecutorRegistry = Arc<HashMap<String, Box<dyn Executor>>>;
