use clap::Parser;
use tierkreis::monitoring::flush_logs;

fn main() -> miette::Result<()> {
    miette::set_panic_hook();
    tierkreis::cli::Cli::parse().execute()?;
    flush_logs();
    Ok(())
}
