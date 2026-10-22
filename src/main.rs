use anyhow::Result;
use tui_live_workprogress::cli;

fn main() -> Result<()> {
    cli::run(std::env::args().skip(1).collect())
}
