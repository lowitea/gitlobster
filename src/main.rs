mod cli;
mod cloner;
mod git;
mod gitlab;
use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
