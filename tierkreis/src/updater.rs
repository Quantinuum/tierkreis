/*!
This module defines the [Updater] struct which provides a way to consume a stream
of [Event] and write each event to a [`WorkflowRunState`].
*/
use std::sync::Arc;

use futures::{Stream, StreamExt};

use crate::{event::Event, state::WorkflowRunState};

/// [Updater] encapsulates a [`WorkflowRunState`] and allows the processing of [Event]s.
pub struct Updater<WS: WorkflowRunState> {
    state: Arc<WS>,
}

impl<WS: WorkflowRunState> Updater<WS> {
    /// Construct a new [Updater].
    pub fn new(state: Arc<WS>) -> Self {
        Self { state }
    }

    /// Process a stream of events until an event declaring the
    /// workflow is complete.
    ///
    /// # Errors
    ///
    /// Will return Err if the updater fails to write the [Event] to
    /// the contained [`WorkflowRunState`].
    pub async fn process(
        self,
        mut stream: impl Stream<Item = Event> + Unpin,
    ) -> miette::Result<()> {
        while let Some(event) = stream.next().await {
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
        state::inmemory::InMemoryWorkflowRunState,
    };

    use super::*;

    #[rstest]
    #[tokio::test]
    async fn consume_single_event() -> miette::Result<()> {
        let (state, _events) = InMemoryWorkflowRunState::test();
        let updater = Updater::new(Arc::new(state));

        let stream = once(async {
            Event::Node(NodeEvent {
                locs: vec![Location::from_usize_iter([0])],
                status: NodeStatus::Running { state_update: None },
            })
        })
        .boxed();

        updater.process(stream).await?;

        Ok(())
    }
}
