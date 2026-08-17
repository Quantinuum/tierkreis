use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tierkreis::monitoring::{LOG_GUARD, init_logging_and_tracing};
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
    if let Some(guard) = LOG_GUARD.get().and_then(|lock| lock.lock().ok()?.take()) {
        drop(guard);
    }
    Ok(())
}
