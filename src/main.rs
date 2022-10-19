mod cli;
mod cloner;
mod git;
mod gitlab;

fn main() -> Result<(), String> {
    cli::run()
}
