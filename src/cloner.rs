use futures::future::join_all;
use std::fs;

use pbr::ProgressBar;
use regex::Regex;
use tracing::{debug, info};
use url::Url;

use crate::gitlab::types;
use crate::{git, gitlab};
use anyhow::Result;

#[derive(Debug)]
pub struct FetchGitlabOptions {
    url: Url,
    token: String,
}

impl FetchGitlabOptions {
    pub fn new(url: String, token: String) -> Result<Self> {
        let url = Url::parse(&url)?;
        Ok(Self { url, token })
    }
}

#[derive(Debug)]
pub struct BackupGitlabOptions {
    url: Url,
    token: String,
    group: String,
}

impl BackupGitlabOptions {
    pub fn new(url: String, token: String, group: String) -> Result<Self> {
        let url = Url::parse(&url)?;
        Ok(Self { url, token, group })
    }
}

struct BackupData {
    client: gitlab::Client,
    group: types::Group,
}

pub enum FilterPatterns {
    Include(Vec<String>),
    Exclude(Vec<String>),
}

fn filter_projects(
    projects: Vec<types::Project>,
    patterns: FilterPatterns,
    limit: Option<usize>,
) -> Result<Vec<types::Project>> {
    let (filter_bit, patterns) = match patterns {
        FilterPatterns::Include(p) => (true, p),
        FilterPatterns::Exclude(p) => (false, p),
    };

    let mut filters: Vec<Regex> = vec![];
    for f in patterns {
        filters.push(Regex::new(&f)?);
    }

    let filter_func = |project: &types::Project| -> bool {
        for filter in filters.clone() {
            if filter.is_match(&project.path_with_namespace) {
                return filter_bit;
            }
        }
        !filter_bit
    };

    let mut projects: Vec<types::Project> = projects.into_iter().filter(filter_func).collect();

    if let Some(limit) = limit {
        if projects.len() > limit {
            projects = projects[0..limit].to_vec();
        }
    }

    Ok(projects)
}

async fn clone_project(
    project: &types::Project,
    dst: &str,
    backup: &Option<BackupData>,
) -> Result<()> {
    debug!("project path: {}", &project.path_with_namespace);

    let dir_path = project
        .path_with_namespace
        .strip_suffix(&project.path)
        .unwrap();
    let path = format!("{}/{}", &dst, dir_path);
    fs::create_dir_all(path)?;

    git::fetch(
        project.ssh_url_to_repo.clone(),
        format!("{}/{}", dst, project.path_with_namespace),
    )
    .await?;

    info!("start pushing");

    let (backup_gl, backup_group) = if let Some(backup) = backup {
        (&backup.client, &backup.group)
    } else {
        return Ok(());
    };

    let path: Vec<String> = project
        .path_with_namespace
        .clone()
        .split('/')
        .map(str::to_string)
        .collect();

    let backup_project = backup_gl
        .make_project_with_namespace(path, backup_group, project)
        .await?;

    git::push_backup(
        format!("{}/{}", dst, project.path_with_namespace),
        backup_project.ssh_url_to_repo,
    )
    .await
}

pub struct CloneParams {
    pub fetch: FetchGitlabOptions,
    pub dst: String,
    pub backup: Option<BackupGitlabOptions>,
    pub patterns: Option<FilterPatterns>,
    pub dry_run: bool,
    pub objects_per_page: Option<u32>,
    pub limit: Option<usize>,
    pub concurrency_limit: usize,
    pub only_owned: bool,
    pub only_membership: bool,
}

#[tokio::main]
pub async fn clone(p: CloneParams) -> Result<()> {
    let fetch_gl = gitlab::Client::new(p.fetch.token, p.fetch.url, p.objects_per_page)?;
    let mut projects = fetch_gl
        .get_projects(p.only_owned, p.only_membership)
        .await?;

    if let Some(patterns) = p.patterns {
        projects = filter_projects(projects, patterns, p.limit)?
    }

    if p.dry_run {
        for p in &projects {
            println!(
                "{: <32} (id: {}, path: {})",
                p.name, p.id, p.path_with_namespace
            );
        }
        return Ok(());
    }

    let backup_data = if let Some(backup) = p.backup {
        let client = gitlab::Client::new(backup.token, backup.url, None)?;
        let group = client.get_group(backup.group).await?;

        Some(BackupData { client, group })
    } else {
        None
    };

    info!("start pulling");

    let mut pb = ProgressBar::new(projects.len() as u64);
    pb.message("Cloning: ");

    for chunk in projects.chunks(p.concurrency_limit) {
        join_all(
            chunk
                .iter()
                .map(|pr| clone_project(pr, &p.dst, &backup_data)),
        )
        .await;
        pb.add(chunk.len() as u64);
    }

    Ok(())
}
