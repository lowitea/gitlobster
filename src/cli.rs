use clap::Parser;

use crate::cloner::{BackupGitlabOptions, FetchGitlabOptions, clone};

#[derive(Parser)]
#[clap(author, version, about)]
/// A tool for cloning all available repositories in a GitLab instance
struct Cli {
    /// The GitLab instance URL for fetch repositories (example: https://gitlab.local/)
    #[clap(long, value_parser, value_name = "FETCH URL")]
    fu: String,

    /// Your personal GitLab token for fetch repositories
    #[clap(long, value_parser, value_name = "FETCH TOKEN")]
    ft: String,

    #[clap(long, value_parser, value_name = "BACKUP URL")]
    /// The GitLab instance URL for backup repositories (example: https://backup-gitlab.local/)
    bu: Option<String>,

    #[clap(long, value_parser, value_name = "BACKUP TOKEN")]
    /// Your personal GitLab token for backup repositories
    bt: Option<String>,

    #[clap(long, value_parser, value_name = "BACKUP GROUP")]
    /// A target created group on backup GitLab for push repositories
    bg: Option<String>,

    /// A destination local folder for save downloaded repositories
    #[clap(value_parser, value_name = "DIRECTORY")]
    dst: String,
}

pub fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let fetch_gl = FetchGitlabOptions::new(cli.fu, cli.ft)?;

    let backup_gl = if let (Some(url), Some(token), Some(group)) = (cli.bu, cli.bt, cli.bg) {
        Some(BackupGitlabOptions::new(url, token, group)?)
    } else {
        None
    };

    clone(fetch_gl, cli.dst, backup_gl)
}
