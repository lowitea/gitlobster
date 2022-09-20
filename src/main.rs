mod cli;
mod git;
mod cloner;
mod gitlab;

fn main() -> Result<(), String> { cli::run() }
