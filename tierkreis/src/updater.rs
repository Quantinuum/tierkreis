/*!
This module defines the [Updater] struct which provides a way to consume a stream
of [Event] and write each event to a [`WorkflowState`].
*/
use std::sync::Arc;

use futures::{Stream, StreamExt};
use tracing::info;

use crate::{event::Event, state::WorkflowState};

/// [Updater] encapsulates a [`WorkflowState`] and allows the processing of [Event]s.
pub struct Updater {
    state: Arc<dyn WorkflowState>,
}

impl Updater {
    /// Construct a new [Updater].
    pub fn new(state: Arc<dyn WorkflowState>) -> Self {
        Self { state }
    }

    /// Process a stream of events until an event declaring the
    /// workflow is complete.
    ///
    /// # Errors
    ///
    /// Will return Err if the updater fails to write the [Event] to
    /// the contained [`WorkflowState`].
    pub async fn process(
        self,
        mut stream: impl Stream<Item = Event> + Unpin,
    ) -> miette::Result<()> {
        while let Some(event) = stream.next().await {
            info!("got event {event:?}");
            let workflow_finished = event.is_workflow_finished();
            self.state.write(event).await?;
            if workflow_finished {
                return Ok(());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use futures::stream::once;
    use rstest::rstest;

    use crate::{
        event::{NodeEvent, NodeStatus},
        location::Location,
        state::inmemory::InMemoryWorkflowState,
    };

    use super::*;

    #[rstest]
    #[tokio::test]
    async fn consume_single_event() -> miette::Result<()> {
        let (state, _events) = InMemoryWorkflowState::test();
        let updater = Updater::new(Arc::new(state));

        let stream = once(async {
            Event::Node(NodeEvent {
                loc: Location::from_usize_iter([0]),
                status: NodeStatus::Running { state: None },
            })
        })
        .boxed();

        updater.process(stream).await?;

        Ok(())
    }
}
