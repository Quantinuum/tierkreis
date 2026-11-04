//! Utilities related to executable commands.

use std::{collections::HashMap, path::Path};

use miette::miette;

pub type WorkerCommands = HashMap<String, Vec<String>>;

/// Utility function to quote components for shell invocations.
pub fn shell_quote(argument: &str) -> String {
    format!("'{}'", argument.replace('\'', "'\"'\"'"))
}

/// Builds the worker invocation command escaping each argument for the shell.
/// Falls back to `tkr-<worker_name>` if no custom command is found.
/// Otherwise uses the command specified for the worker in the commands map.
/// For example:
///
/// ```toml
/// [executors.slurm.worker_commands]
/// mpi_worker = ["uv", "run", "/mpi_worker/main.py"]
/// ```
///
/// This returns `("uv", ["run", "/mpi_worker/main.py", "<worker_args_path>"])`.
///
/// While passing `qulacs_worker` would return:
/// `("tkr-qulacs-worker", "<worker_args_path>")`
///
/// # Errors
///
/// Will return Err if the provided override has no executable component.
pub fn worker_invocation(
    worker_commands: &WorkerCommands,
    worker_name: &str,
    worker_args_path: &Path,
) -> miette::Result<(String, Vec<String>)> {
    Ok(match worker_commands.get(worker_name) {
        Some(components) => (
            components
                .first()
                .ok_or_else(|| miette!("Missing command for worker override: `{worker_name}`"))?
                .clone(),
            components
                .iter()
                .skip(1)
                .cloned()
                .chain(std::iter::once(
                    worker_args_path.to_string_lossy().to_string(),
                ))
                .collect(),
        ),
        None => (
            format!("tkr-{}", worker_name.replace('_', "-")),
            vec![worker_args_path.to_string_lossy().to_string()],
        ),
    })
}

pub fn validate_worker_commands(worker_commands: &WorkerCommands) -> miette::Result<()> {
    for (worker, command) in worker_commands {
        if command.first().is_none_or(std::string::String::is_empty) {
            return Err(miette!("Worker `{worker}` requires a nonempty executable"));
        }
    }

    Ok(())
}
