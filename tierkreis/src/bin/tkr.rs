use clap::{Parser, Subcommand};
use miette::IntoDiagnostic;
use std::net::IpAddr;
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
    Serve {
        #[clap(long, default_value = "127.0.0.1")]
        host: IpAddr,

        #[clap(long, short, default_value_t = 3000)]
        port: u16,
    },
    Openapi {
        out: PathBuf,
    },
    Exec {},
}

fn main() -> miette::Result<()> {
    miette::set_panic_hook();
    init_logging_and_tracing(&None);
    let cli = Cli::parse();
    match cli.command {
        Command::Run {
            from_file: _from_file,
            ..
        } => {}
        Command::Serve { host, port } => {
            tierkreis::server::serve(host, port)?;
        }
        Command::Openapi { out } => {
            let spec = tierkreis::server::openapi_spec();
            let json = serde_json::to_string_pretty(&spec).into_diagnostic()?;
            std::fs::write(out, json).into_diagnostic()?;
        }
        Command::Exec {} => {
            tierkreis::runtime::exec()?;
        }
        _ => {}
    }
    flush_logs();
    Ok(())
}
