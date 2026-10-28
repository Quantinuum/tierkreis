//! Standalone subprocess worker for Tierkreis builtins.

use std::path::PathBuf;

use clap::Parser;
use miette::IntoDiagnostic;

#[derive(Parser)]
#[command(about = "Run Tierkreis builtin tasks")]
struct Args {
    /// List the builtin task names supported by this worker.
    #[arg(long)]
    list: bool,

    /// Path to a JSON-encoded Tierkreis worker call.
    call_args: Option<PathBuf>,
}

fn main() -> miette::Result<()> {
    let args = Args::parse();
    if args.list {
        println!(
            "{}",
            serde_json::to_string(tierkreis::builtins::supported_tasks()).into_diagnostic()?
        );
        return Ok(());
    }

    let call_args = args
        .call_args
        .ok_or_else(|| miette::miette!("Missing path to worker call arguments"))?;
    tierkreis::builtins::run_worker_call(&call_args)
}
