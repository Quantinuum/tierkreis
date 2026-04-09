use std::path::PathBuf;

use clap::{Parser, Subcommand};

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
}

fn main() -> miette::Result<()> {
    miette::set_panic_hook();

    let cli = Cli::parse();
    match cli.command {
        Command::Run { from_file, .. } => {
            tierkreis::runtime::run(&from_file)?;
        }
        Command::Serve {} => {
            tierkreis::server::serve()?;
        }
        _ => {}
    }
    Ok(())
}
