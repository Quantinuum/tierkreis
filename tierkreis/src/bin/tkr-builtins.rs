//! Standalone subprocess worker for Tierkreis builtins.

use std::path::PathBuf;

use clap::Parser;
use tierkreis::monitoring::{LoggingConfig, init_logging_and_tracing};

#[derive(Parser)]
#[command(about = "Run Tierkreis builtin tasks")]
struct Args {
    /// Path to a JSON-encoded Tierkreis worker call.
    call_args: Option<PathBuf>,
}

fn main() -> miette::Result<()> {
    let args = Args::parse();
    let logging_config = LoggingConfig::new_with_custom_name("tkr-builtins");
    init_logging_and_tracing(&Some(logging_config));

    let call_args = args
        .call_args
        .ok_or_else(|| miette::miette!("Missing path to worker call arguments"))?;
    tierkreis::worker::run_builtin_worker_call(&call_args)
}
