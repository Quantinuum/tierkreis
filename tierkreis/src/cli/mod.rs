//! Command-line interface implementation for the `tkr` binary.

mod init;
mod run;
mod templates;

use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand};

/// Tierkreis: a workflow engine for quantum HPC.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    /// The operation to perform.
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Create Tierkreis projects and workers.
    Init {
        #[command(subcommand)]
        command: InitCommand,
    },
    /// Generate Python APIs for workers.
    Generate {
        #[command(subcommand)]
        command: GenerateCommand,
    },
    /// Run one workflow and wait for it to finish.
    Run(RunArgs),
    /// Run the long-lived Tierkreis runtime.
    Exec {
        /// Explicit runtime configuration file.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Serve the Tierkreis API and an optional visualization frontend.
    Serve {
        /// Directory containing a built visualization frontend (`index.html` and `assets/`).
        #[arg(long)]
        assets: Option<PathBuf>,
        /// Explicit runtime configuration file.
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum InitCommand {
    /// Create a project with an example graph and worker.
    Project {
        /// Directory in which to create the project.
        #[arg(default_value = ".")]
        directory: PathBuf,
    },
    /// Create a worker in an existing project.
    Worker {
        /// Worker name. Hyphens are converted to underscores in Python modules.
        name: String,
        /// Worker collection directory. Defaults to the discovered project's `tkr/workers`.
        #[arg(long)]
        directory: Option<PathBuf>,
        /// Generate a `TypeSpec` definition instead of a Python implementation.
        #[arg(long)]
        external: bool,
    },
}

#[derive(Subcommand, Debug)]
enum GenerateCommand {
    /// Generate Python API stubs for every worker in a project.
    Stubs {
        /// Worker collection directory. Defaults to the discovered project's `tkr/workers`.
        #[arg(long)]
        workers_directory: Option<PathBuf>,
        /// Stub path relative to each worker directory.
        #[arg(long, default_value = "api/api.py")]
        output: PathBuf,
    },
}

#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("graph_source")
        .required(true)
        .args(["workflow", "python"])
))]
struct RunArgs {
    /// Serialized legacy or current Tierkreis workflow graph.
    workflow: Option<PathBuf>,
    /// Load a Python graph factory (`module:function` or `path.py:function`).
    #[arg(long, value_name = "GRAPH")]
    python: Option<String>,
    /// JSON object containing workflow inputs.
    #[arg(long, short, default_value = "workflow_inputs.json")]
    inputs: PathBuf,
    /// Human-readable workflow name.
    #[arg(long)]
    name: Option<String>,
    /// Print the top-level outputs as JSON.
    #[arg(long, short = 'o')]
    print_output: bool,
    /// Explicit runtime configuration file.
    #[arg(long)]
    config: Option<PathBuf>,
}

impl Cli {
    /// Parse and execute the CLI.
    ///
    /// # Errors
    ///
    /// Returns an error when a command cannot be completed.
    pub fn execute(self) -> miette::Result<()> {
        match self.command {
            Command::Init { command } => match command {
                InitCommand::Project { directory } => init::init_project(&directory),
                InitCommand::Worker {
                    name,
                    directory,
                    external,
                } => init::init_worker(&name, directory.as_deref(), external),
            },
            Command::Generate { command } => match command {
                GenerateCommand::Stubs {
                    workers_directory,
                    output,
                } => init::generate_stubs(workers_directory.as_deref(), &output),
            },
            Command::Run(args) => run::run(run::Options {
                workflow: args.workflow.as_deref(),
                python: args.python.as_deref(),
                inputs: &args.inputs,
                name: args.name,
                print_output: args.print_output,
                config: args.config.as_deref(),
            }),
            Command::Exec { config } => {
                let config = load_config(config.as_deref())?;
                crate::runtime::exec_with_config(&config)
            }
            Command::Serve { assets, config } => {
                let config = load_config(config.as_deref())?;
                crate::server::serve_with_config(&config, assets.as_deref())
            }
        }
    }
}

fn load_config(path: Option<&std::path::Path>) -> miette::Result<crate::runtime::RuntimeConfig> {
    match path {
        Some(path) => crate::runtime::RuntimeConfig::from_file(path),
        None => crate::runtime::RuntimeConfig::load(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_requires_exactly_one_graph_source() {
        assert!(Cli::try_parse_from(["tkr", "run"]).is_err());
        assert!(
            Cli::try_parse_from(["tkr", "run", "graph.json", "--python", "module:graph"]).is_err()
        );
        assert!(Cli::try_parse_from(["tkr", "run", "graph.json"]).is_ok());
        assert!(Cli::try_parse_from(["tkr", "run", "--python", "module:graph"]).is_ok());
    }
}
