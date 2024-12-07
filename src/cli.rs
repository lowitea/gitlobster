use clap::Parser;

use crate::cloner::{
    clone, BackupGitlabOptions, CloneParams, FetchGitlabOptions, FilterPatterns, ForceProtocol,
};
use anyhow::{bail, Result};

#[derive(Parser)]
#[command(author, version, about)]
/// A tool for cloning all available repositories in a GitLab instance
struct Cli {
    /// The GitLab instance URL for fetch repositories (example: <https://gitlab.local>)
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

    /// The GitLab instance URL for backup repositories (example: <https://backup-gitlab.local>)
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
    #[arg(short = 'i', long, env = "GTLBSTR_INCLUDE", value_name = "PATTERN")]
    include: Option<Vec<String>>,

    /// Comma separated exclude regexp patterns (cannot be used together with --include flag, may be repeated)
    #[arg(short = 'x', long, env = "GTLBSTR_EXCLUDE", value_name = "PATTERN")]
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
        value_parser=clap::value_parser!(u32).range(1..101),
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

    /// Download projects only in group
    #[arg(long, env = "GTLBSTR_GROUP")]
    group: Option<String>,

    /// Enable download by ssh instead of http. An authorized ssh key is required
    #[arg(long, env = "GTLBSTR_DOWNLOAD_SSH")]
    download_ssh: bool,

    /// Enable upload by ssh instead of http. An authorized ssh key is required
    #[arg(long, env = "GTLBSTR_UPLOAD_SSH")]
    upload_ssh: bool,

    /// Force download repositories by insecure protocol. Does not work with the `download_ssh` flag
    #[arg(long, env = "GTLBSTR_DOWNLOAD_FORCE_HTTP")]
    download_force_http: bool,

    /// Force download repositories by secure protocol. Does not work with the `download_ssh` flag
    #[arg(long, env = "GTLBSTR_DOWNLOAD_FORCE_HTTPS")]
    download_force_https: bool,

    /// Force upload repositories by insecure protocol. Does not work with the `upload_ssh` flag
    #[arg(long, env = "GTLBSTR_UPLOAD_FORCE_HTTP")]
    upload_force_http: bool,

    /// Force upload repositories by secure protocol. Does not work with the `upload_ssh` flag
    #[arg(long, env = "GTLBSTR_UPLOAD_FORCE_HTTPS")]
    upload_force_https: bool,

    /// Disable saving the directory hierarchy
    #[arg(long, env = "GTLBSTR_DISABLE_HIERARCHY")]
    disable_hierarchy: bool,

    /// Clear dst path before cloning
    #[arg(long, env = "GTLBSTR_CLEAR_DST")]
    clear_dst: bool,

    /// Download only default branch
    #[arg(long, env = "GTLBSTR_ONLY_MASTER")]
    only_master: bool,

    /// Disable adding sync dates in project descriptions
    #[arg(long, env = "GTLBSTR_DISABLE_SYNC_DATE")]
    disable_sync_date: bool,

    /// Timeout for requests to GitLab instances in seconds
    #[arg(long, env = "GTLBSTR_GITLAB_TIMEOUT")]
    gitlab_timeout: Option<u32>,
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

    let fetch_gl = FetchGitlabOptions::new(&cli.fu, &cli.ft)?;

    let patterns = if cli.exclude.is_some() && cli.include.is_some() {
        bail!("You cannot use the --include and --exclude flag together");
    } else if let Some(patterns) = cli.exclude {
        Some(FilterPatterns::Exclude(patterns))
    } else {
        cli.include.map(FilterPatterns::Include)
    };

    let upl_err = "For upload to another gitlab, you must specify both the --bt and --bu flags";
    let backup_gl = if let (Some(url), Some(token)) = (&cli.bu, &cli.bt) {
        Some(BackupGitlabOptions::new(url, token, cli.bg.clone())?)
    } else {
        if cli.bu.is_some() || cli.bt.is_some() {
            bail!(upl_err);
        };
        None
    };

    if backup_gl.is_none() && cli.bg.is_some() {
        bail!(upl_err);
    }

    if cli.download_force_http && cli.download_force_https {
        bail!("You cannot use --download-force-http and --download-force-https flags together");
    }

    if cli.upload_force_http && cli.upload_force_https {
        bail!("You cannot use --upload-force-http and --upload-force-https flags together");
    }

    let download_force_protocol = if cli.download_force_http {
        ForceProtocol::Http
    } else if cli.download_force_https {
        ForceProtocol::Https
    } else {
        ForceProtocol::No
    };

    let upload_force_protocol = if cli.upload_force_http {
        ForceProtocol::Http
    } else if cli.upload_force_https {
        ForceProtocol::Https
    } else {
        ForceProtocol::No
    };

    let clone_params = CloneParams {
        fetch: fetch_gl,
        dst: cli.dst,
        backup: backup_gl,
        patterns,
        dry_run: cli.dry_run,
        group: cli.group,
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
        disable_sync_date: cli.disable_sync_date,
        gitlab_timeout: cli.gitlab_timeout,
        download_force_protocol,
        upload_force_protocol,
    };

    clone(clone_params)
}
