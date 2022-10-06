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

    /// Verbose level (one or more, max four)
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

pub fn run() -> Result<(), String> {
    let cli = Cli::parse();

    let log_level = match cli.verbose {
        0 => tracing::Level::ERROR,
        1 => tracing::Level::WARN,
        2 => tracing::Level::INFO,
        3 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };
    tracing_subscriber::fmt().with_max_level(log_level).init();

    let fetch_gl = FetchGitlabOptions::new(cli.fu, cli.ft)?;
    let backup_gl = if let (Some(url), Some(token), Some(group)) = (cli.bu, cli.bt, cli.bg) {
        Some(BackupGitlabOptions::new(url, token, group)?)
    } else {
        None
    };

    clone(fetch_gl, cli.dst, backup_gl)
}
