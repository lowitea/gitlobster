mod cli;
mod cloner;
mod gitlab;
mod git;

fn main() -> Result<(), String> { cli::run() }
