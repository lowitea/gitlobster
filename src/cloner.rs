use futures::future::join_all;
use std::fs;

use pbr::ProgressBar;
use regex::Regex;
use tracing::{debug, info};
use url::Url;

use crate::gitlab::types;
use crate::{git, gitlab};

#[derive(Debug)]
pub struct FetchGitlabOptions {
    url: Url,
    token: String,
}

impl FetchGitlabOptions {
    pub fn new(url: String, token: String) -> Result<Self, String> {
        let url = Url::parse(&url).map_err(|e| e.to_string())?;
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
    pub fn new(url: String, token: String, group: String) -> Result<Self, String> {
        let url = Url::parse(&url).map_err(|e| e.to_string())?;
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
) -> Result<Vec<types::Project>, String> {
    let (filter_bit, patterns) = match patterns {
        FilterPatterns::Include(p) => (true, p),
        FilterPatterns::Exclude(p) => (false, p),
    };

    let mut filters: Vec<Regex> = vec![];
    for f in patterns {
        filters.push(Regex::new(&f).map_err(|e| e.to_string())?);
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
) -> Result<(), String> {
    debug!("project path: {}", &project.path_with_namespace);

    let dir_path = project
        .path_with_namespace
        .strip_suffix(&project.path)
        .unwrap();
    let path = format!("{}/{}", &dst, dir_path);
    fs::create_dir_all(path).map_err(|e| e.to_string())?;

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
        .await
        .map_err(|e| e.to_string())?;

    git::push_backup(
        format!("{}/{}", dst, project.path_with_namespace),
        backup_project.ssh_url_to_repo,
    )
    .await
}

#[tokio::main]
pub async fn clone(
    fetch: FetchGitlabOptions,
    dst: String,
    backup: Option<BackupGitlabOptions>,
    patterns: Option<FilterPatterns>,
    dry_run: bool,
    objects_per_page: Option<u32>,
    limit: Option<usize>,
    concurrency_limit: usize,
) -> Result<(), String> {
    let fetch_gl = gitlab::Client::new(fetch.token, fetch.url, objects_per_page)?;
    let mut projects = fetch_gl.get_projects().await?;

    if let Some(patterns) = patterns {
        projects = filter_projects(projects, patterns, limit)?
    }

    if dry_run {
        for p in &projects {
            println!(
                "{: <32} (id: {}, path: {})",
                p.name, p.id, p.path_with_namespace
            );
        }
        return Ok(());
    }

    let backup_data = if let Some(backup) = backup {
        let client = gitlab::Client::new(backup.token, backup.url, None)?;
        let group = client
            .get_group(backup.group)
            .await
            .map_err(|e| e.to_string())?;

        Some(BackupData { client, group })
    } else {
        None
    };

    info!("start pulling");

    let mut pb = ProgressBar::new(projects.len() as u64);
    pb.message("Cloning: ");

    for chunk in projects.chunks(concurrency_limit) {
        join_all(chunk.iter().map(|p| clone_project(p, &dst, &backup_data))).await;
        pb.add(concurrency_limit as u64);
    }

    Ok(())
}
