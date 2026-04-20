/*!
This module defines the Workflow graph representation.
*/

use serde_json::Value;

/// [Node] is a placeholder enum for the kinds of operations that can appear
/// in a Workflow graph. To be replaced with something compatible with the
/// existing python code.
pub enum Node {
    /// An operation performed by a Worker.
    Task {
        /// The name of the Worker to call.
        worker_name: String,
        /// The name of the Task to call.
        task_name: String,
    },
    /// An input to the Workflow.
    Input {
        /// The name of the Workflow input to capture.
        name: String,
    },
    /// The output of the Workflow
    Output {},
    /// A constant Value in the Workflow.
    Const {
        /// The constant value to return.
        value: Value,
    },
    /// A sub-execution.
    Eval {},
}
