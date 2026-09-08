use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tierkreis::monitoring::{flush_logs, init_logging_and_tracing};
/// Tierkreis: a workflow engine for quantum HPC.
///
/// This is the main tierkreis command-line tool.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Run {
        from_file: PathBuf,

        #[clap(long, short, default_value_t = false)]
        verbose: bool,

        #[clap(long, default_value = None)]
        run_id: Option<u64>,

        #[clap(long, short = 'o', default_value_t = false)]
        print_output: bool,
    },
    Init {},
    Viz {},
    Serve {},
    Exec {},
}

fn main() -> miette::Result<()> {
    miette::set_panic_hook();
    init_logging_and_tracing(None);
    let cli = Cli::parse();
    match cli.command {
        Command::Run {
            from_file: _from_file,
            ..
        } => {}
        Command::Serve {} => {
            tierkreis::server::serve()?;
        }
        Command::Exec {} => {
            tierkreis::runtime::exec()?;
        }
        _ => {}
    }
    flush_logs();
    Ok(())
}

#[cfg(test)]
mod tests {
    use miette::IntoDiagnostic;
    use std::collections::HashMap;
    use std::sync::Arc;

    use tierkreis::runtime::Runtime;
    use tierkreis::runtime::asset_storage_registry_from_config;
    use tierkreis::runtime::vis_config;
    use tierkreis::server::server;
    use tierkreis::state::SqliteRuntimeState;

    use hugr::Hugr;
    use std::fs::File;
    use std::io::BufReader;
    use tierkreis::graph::WorkflowGraph;

    #[tokio::test(flavor = "multi_thread")]
    pub async fn run_and_vis_single_workflow() -> miette::Result<()> {
        // Runtime setup

        let config = vis_config();
        let mut runtime = Runtime::from_config(&config).await?;
        // Graph definition
        let hugr =
            Hugr::load(BufReader::new(File::open("../doubler.hugr").unwrap()), None).unwrap();
        let workflow_graph = WorkflowGraph::try_from(hugr).unwrap();
        // Workflow run
        let workflow_id = runtime.save_workflow(None, workflow_graph).await?;
        let mut inputs = HashMap::new();
        inputs.insert("in0".to_string(), serde_json::to_vec(&5).into_diagnostic()?);
        inputs.insert("in1".to_string(), serde_json::to_vec(&7).into_diagnostic()?);
        let (run_id, attempt) = runtime.start_new_run(workflow_id, inputs).await?;
        runtime.dedicated_run_id = Some(run_id);
        runtime.run().await?;
        let outputs = runtime.outputs(run_id, attempt).await?;
        dbg!(&outputs);
        // visualize
        dbg!("starting server");
        let runtime_state = Arc::new(SqliteRuntimeState::try_new().await?);
        let asset_registry = asset_storage_registry_from_config(&config);
        server(runtime_state, asset_registry).await
    }
}
