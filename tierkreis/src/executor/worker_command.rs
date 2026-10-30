//! Shared worker command selection for process-based executors.

use std::{collections::HashMap, path::Path};

use miette::{Result, miette};

pub(crate) type WorkerCommands = HashMap<String, Vec<String>>;
pub(crate) type ArchitectureWorkerCommands = HashMap<String, WorkerCommands>;

/// A resolved worker invocation, including the worker call-definition path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkerInvocation {
    argv: Vec<String>,
}

impl WorkerInvocation {
    pub(crate) fn resolve(
        commands: &WorkerCommands,
        worker_name: &str,
        worker_args_path: &Path,
    ) -> Self {
        let default = [format!("tkr-{}", worker_name.replace('_', "-"))];
        let command = commands
            .get(worker_name)
            .map_or(default.as_slice(), Vec::as_slice);
        let argv = command
            .iter()
            .cloned()
            .chain(std::iter::once(
                worker_args_path.to_string_lossy().into_owned(),
            ))
            .collect();
        Self { argv }
    }

    pub(crate) fn program_and_args(&self) -> (&str, &[String]) {
        self.argv
            .split_first()
            .map(|(program, args)| (program.as_str(), args))
            .expect("worker invocations always have a program")
    }

    pub(crate) fn shell_command(&self) -> String {
        self.argv
            .iter()
            .map(|argument| format!("'{}'", argument.replace('\'', "'\"'\"'")))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub(crate) fn validate_registry(registry: &ArchitectureWorkerCommands) -> Result<()> {
    for (architecture, commands) in registry {
        if architecture.is_empty()
            || !architecture
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "_.-".contains(character))
        {
            return Err(miette!("Invalid worker architecture `{architecture}`"));
        }
        validate_commands(commands)
            .map_err(|error| miette!("Invalid architecture `{architecture}`: {error}"))?;
    }
    Ok(())
}

pub(crate) fn validate_commands(commands: &WorkerCommands) -> Result<()> {
    for (worker, command) in commands {
        if worker.is_empty() {
            return Err(miette!("Worker command contains an empty worker name"));
        }
        if command.first().is_none_or(String::is_empty) {
            return Err(miette!("Worker `{worker}` requires a nonempty executable"));
        }
    }
    Ok(())
}

pub(crate) fn select_architecture(
    registry: &ArchitectureWorkerCommands,
    architecture: Option<&str>,
) -> Result<WorkerCommands> {
    let architecture = architecture.unwrap_or(std::env::consts::ARCH);
    if architecture.is_empty()
        || !architecture
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_.-".contains(character))
    {
        return Err(miette!("Invalid worker architecture `{architecture}`"));
    }
    Ok(registry.get(architecture).cloned().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_architecture_override_and_appends_call_definition() -> Result<()> {
        let registry = ArchitectureWorkerCommands::from([(
            "aarch64".to_string(),
            WorkerCommands::from([(
                "simulation_worker".to_string(),
                vec![
                    "uv".to_string(),
                    "run".to_string(),
                    "/worker/main.py".to_string(),
                ],
            )]),
        )]);
        validate_registry(&registry)?;
        let commands = select_architecture(&registry, Some("aarch64"))?;
        let invocation = WorkerInvocation::resolve(
            &commands,
            "simulation_worker",
            Path::new("/tmp/call args.json"),
        );
        assert_eq!(
            invocation.argv,
            ["uv", "run", "/worker/main.py", "/tmp/call args.json"]
        );
        assert_eq!(
            invocation.shell_command(),
            "'uv' 'run' '/worker/main.py' '/tmp/call args.json'"
        );
        Ok(())
    }

    #[test]
    fn falls_back_to_standard_worker_command() -> Result<()> {
        let commands = select_architecture(&ArchitectureWorkerCommands::new(), None)?;
        let invocation =
            WorkerInvocation::resolve(&commands, "hello_world", Path::new("args.json"));
        assert_eq!(invocation.argv, ["tkr-hello-world", "args.json"]);
        Ok(())
    }

    #[test]
    fn shell_quotes_single_quotes() {
        let commands = WorkerCommands::from([(
            "worker".to_string(),
            vec!["worker's executable".to_string()],
        )]);
        let invocation = WorkerInvocation::resolve(&commands, "worker", Path::new("args.json"));
        assert_eq!(
            invocation.shell_command(),
            "'worker'\"'\"'s executable' 'args.json'"
        );
    }

    #[test]
    fn rejects_invalid_registry_entries() {
        let registry = ArchitectureWorkerCommands::from([(
            "aarch64".to_string(),
            WorkerCommands::from([("worker".to_string(), Vec::new())]),
        )]);
        assert!(validate_registry(&registry).is_err());
        assert!(select_architecture(&registry, Some("bad architecture")).is_err());
    }
}
