use clap::Parser;

use crate::cloner;

#[derive(Parser)]
#[clap(author, version, about)]
/// A tool for cloning all available repositories in a GitLab instance
struct Cli {
    /// Your personal GitLab token
    #[clap(short, long)]
    token: String,

    /// The GitLab instance URL (example: https://gitlab.local/)
    #[clap(short, long)]
    url: String,

    /// A destination folder
    #[clap()]
    dst: String,
}

pub fn run() -> Result<(), String> {
    let cli = Cli::parse();
    cloner::clone(cli.token, cli.url, cli.dst)
}
