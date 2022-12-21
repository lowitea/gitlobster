use clap::Parser;

use crate::cloner::{clone, BackupGitlabOptions, CloneParams, FetchGitlabOptions, FilterPatterns};
use anyhow::{bail, Result};

#[derive(Parser)]
#[command(author, version, about)]
/// A tool for cloning all available repositories in a GitLab instance
struct Cli {
    /// The GitLab instance URL for fetch repositories (example: https://gitlab.local/)
    #[arg(
        long,
        value_parser,
        env = "GTLBSTR_FETCH_URL",
        value_name = "FETCH URL"
    )]
    fu: String,

    /// Your personal GitLab token for fetch repositories
    #[arg(
        long,
        value_parser,
        env = "GTLBSTR_FETCH_TOKEN",
        value_name = "FETCH TOKEN"
    )]
    ft: String,

    /// The GitLab instance URL for backup repositories (example: https://backup-gitlab.local/)
    #[arg(
        long,
        value_parser,
        env = "GTLBSTR_BACKUP_URL",
        value_name = "BACKUP URL"
    )]
    bu: Option<String>,

    /// Your personal GitLab token for backup repositories
    #[arg(
        long,
        value_parser,
        env = "GTLBSTR_BACKUP_TOKEN",
        value_name = "BACKUP TOKEN"
    )]
    bt: Option<String>,

    /// A target created group on backup GitLab for push repositories
    #[arg(
        long,
        value_parser,
        env = "GTLBSTR_BACKUP_GROUP",
        value_name = "BACKUP GROUP"
    )]
    bg: Option<String>,

    /// Include regexp patterns (cannot be used together with --exclude flag, may be repeated)
    #[arg(long, env = "GTLBSTR_INCLUDE", value_name = "PATTERN")]
    include: Option<Vec<String>>,

    /// Comma separated exclude regexp patterns (cannot be used together with --include flag, may be repeated)
    #[arg(long, env = "GTLBSTR_EXCLUDE", value_name = "PATTERN")]
    exclude: Option<Vec<String>>,

    /// A destination local folder for save downloaded repositories
    #[arg(
        long,
        short,
        value_parser,
        env = "GTLBSTR_DST",
        value_name = "DIRECTORY"
    )]
    dst: Option<String>,

    /// Verbose level (one or more, max four)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Show all projects to download
    #[arg(long)]
    dry_run: bool,

    /// Low-level option, how many projects can fetch in one request
    #[arg(
        long,
        value_parser,
        env = "GTLBSTR_OBJECTS_PER_PAGE",
        value_name = "COUNT"
    )]
    objects_per_page: Option<u32>,

    /// Maximum projects to download
    #[arg(long, value_parser, env = "GTLBSTR_LIMIT", value_name = "COUNT")]
    limit: Option<usize>,

    /// Limit concurrency download
    #[arg(
        long,
        value_parser,
        env = "GTLBSTR_CONCURRENCY_LIMIT",
        default_value_t = 21,
        value_name = "LIMIT"
    )]
    concurrency_limit: usize,

    /// Download projects explicitly owned by user
    #[arg(long, env = "GTLBSTR_ONLY_OWNED")]
    only_owned: bool,

    /// Download only user's projects
    #[arg(long, env = "GTLBSTR_ONLY_MEMBERSHIP")]
    only_membership: bool,

    /// Enable download by ssh instead of http. An authorized ssh key is required
    #[arg(long, env = "GTLBSTR_DOWNLOAD_SSH")]
    download_ssh: bool,

    /// Enable upload by ssh instead of http. An authorized ssh key is required
    #[arg(long, env = "GTLBSTR_UPLOAD_SSH")]
    upload_ssh: bool,

    /// Disable saving the directory hierarchy
    #[arg(long, env = "GTLBSTR_DISABLE_HIERARCHY")]
    disable_hierarchy: bool,

    /// Clear dst path before cloning
    #[arg(long, env = "GTLBSTR_CLEAR_DST")]
    clear_dst: bool,

    /// Download only default branch
    #[arg(long, env = "GTLBSTR_ONLY_MASTER")]
    only_master: bool,
}

pub fn run() -> Result<()> {
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

    let patterns = if cli.exclude.is_some() && cli.include.is_some() {
        bail!("You cannot use the --include and --exclude flag together");
    } else if let Some(patterns) = cli.exclude {
        Some(FilterPatterns::Exclude(patterns))
    } else {
        cli.include.map(FilterPatterns::Include)
    };

    let backup_gl = if let (Some(url), Some(token), Some(group)) = (cli.bu, cli.bt, cli.bg) {
        Some(BackupGitlabOptions::new(url, token, group)?)
    } else {
        None
    };

    let clone_params = CloneParams {
        fetch: fetch_gl,
        dst: cli.dst,
        backup: backup_gl,
        patterns,
        dry_run: cli.dry_run,
        objects_per_page: cli.objects_per_page,
        limit: cli.limit,
        concurrency_limit: cli.concurrency_limit,
        only_owned: cli.only_owned,
        only_membership: cli.only_membership,
        download_ssh: cli.download_ssh,
        upload_ssh: cli.upload_ssh,
        disable_hierarchy: cli.disable_hierarchy,
        clear_dst: cli.clear_dst,
        only_master: cli.only_master,
    };

    clone(clone_params)
}
