use clap::{Parser, Subcommand};
use tierkreis::runtime::{RuntimeConfig, asset_storage_registry_from_config};
use std::path::PathBuf;
use std::sync::Arc;
use tierkreis::state::SqliteRuntimeState;

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

    let cli = Cli::parse();
    match cli.command {
        Command::Run {
            from_file: _from_file,
            ..
        } => {}
        Command::Serve {} => {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| miette::miette!("Failed to build tokio runtime: {e}"))?
                .block_on(async {
                    // TODO: potentially use from config here?
                    let runtime_state = Arc::new(SqliteRuntimeState::try_new().await?);
                    let asset_registry = asset_storage_registry_from_config(&RuntimeConfig::default());
                    tierkreis::server::serve(runtime_state, asset_registry).await
                })?;
        }
        Command::Exec {} => {
            tierkreis::runtime::exec()?;
        }
        _ => {}
    }
    Ok(())
}
