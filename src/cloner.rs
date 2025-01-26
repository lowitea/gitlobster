use crate::gitlab::types;
use crate::{git, gitlab};
use anyhow::{bail, Result};
use futures::future::join_all;
use pbr::ProgressBar;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use url::Url;

const TEMP_DIR: &str = "gitlobster";

#[derive(Debug)]
pub struct FetchGitlabOptions {
    url: Url,
    token: String,
}

impl FetchGitlabOptions {
    pub fn new(url: &str, token: &str) -> Result<Self> {
        let url = Url::parse(url)?;
        Ok(Self {
            url,
            token: token.to_string(),
        })
    }
}

#[derive(Debug)]
pub struct BackupGitlabOptions {
    url: Url,
    token: String,
    group: Option<String>,
}

impl BackupGitlabOptions {
    pub fn new(url: &str, token: &str, group: Option<String>) -> Result<Self> {
        let url = Url::parse(url)?;
        Ok(Self {
            url,
            token: token.to_string(),
            group,
        })
    }
}

struct BackupData {
    client: gitlab::Client,
    group: Option<types::Group>,
    git_http_auth: Option<String>,
    force_protocol: ForceProtocol,
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

pub enum ForceProtocol {
    No,
    Http,
    Https,
}

fn make_git_path(
    project: &types::Project,
    git_http_auth: &Option<String>,
    force_protocol: &ForceProtocol,
) -> String {
    if let Some(auth) = git_http_auth {
        let parts: Vec<&str> = project.http_url_to_repo.split("://").collect();
        assert!(parts.len() == 2, "project with incorrect http path");
        let protocol = match force_protocol {
            ForceProtocol::No => parts[0],
            ForceProtocol::Http => "http",
            ForceProtocol::Https => "https",
        };
        format!("{}://{}@{}", protocol, auth, parts[1])
    } else {
        project.ssh_url_to_repo.clone()
    }
}

#[allow(clippy::too_many_arguments)]
async fn clone_project(
    project: &types::Project,
    dst: &str,
    only_master: bool,
    fetch_git_http_auth: &Option<String>,
    backup: &Option<BackupData>,
    disable_hierarchy: bool,
    fetch_force_protocol: &ForceProtocol,
    fetch_gl: &gitlab::Client,
    groups_info: Arc<Mutex<HashMap<String, types::Group>>>,
) -> Result<()> {
    info!("project path: {}", &project.path_with_namespace);

    let src = make_git_path(project, fetch_git_http_auth, fetch_force_protocol);
    let p_path = if disable_hierarchy {
        &project.path
    } else {
        &project.path_with_namespace
    };

    git::fetch(src, format!("{}/{}", dst, &p_path), only_master).await?;

    let (backup_gl, backup_group, backup_git_http_auth, backup_force_protocol) =
        if let Some(backup) = backup {
            (
                &backup.client,
                &backup.group,
                &backup.git_http_auth,
                &backup.force_protocol,
            )
        } else {
            return Ok(());
        };

    info!("start pushing");

    let path: Vec<String> = if disable_hierarchy {
        vec![p_path.clone()]
    } else {
        project
            .path_with_namespace
            .clone()
            .split('/')
            .map(str::to_string)
            .collect()
    };

    let mut last_group = String::new();
    let mut project_groups: Vec<types::Group> = Vec::new();

    for group in &path[..path.len() - 1] {
        last_group += group;
        let g_info = {
            let mut groups_info = groups_info.lock().await;

            if let Some(g_info) = groups_info.get(&last_group) {
                g_info.clone()
            } else {
                let g_info = fetch_gl.get_group(&last_group).await?;
                groups_info.insert(last_group.clone(), g_info.clone());
                g_info
            }
        };
        project_groups.push(g_info);
        last_group += "/";
    }

    let backup_project = backup_gl
        .make_project_with_namespace(path, project_groups, backup_group, project)
        .await?;

    let remote = make_git_path(&backup_project, backup_git_http_auth, backup_force_protocol);
    git::push_backup(format!("{dst}/{p_path}"), remote).await?;
    Ok(())
}

async fn make_git_http_auth(client: &gitlab::Client, token: &str) -> Result<String> {
    let user = client.get_current_user().await?;
    Ok(format!("{}:{}", user.username, token))
}

fn clear_dst(dst: &str) {
    let _ = std::fs::remove_dir_all(dst);
}

pub struct CloneParams {
    pub fetch: FetchGitlabOptions,
    pub dst: Option<String>,
    pub backup: Option<BackupGitlabOptions>,
    pub group: Option<String>,
    pub patterns: Option<FilterPatterns>,
    pub dry_run: bool,
    pub objects_per_page: Option<u32>,
    pub limit: Option<usize>,
    pub concurrency_limit: usize,
    pub only_owned: bool,
    pub only_membership: bool,
    pub download_ssh: bool,
    pub upload_ssh: bool,
    pub disable_hierarchy: bool,
    pub clear_dst: bool,
    pub only_master: bool,
    pub disable_sync_date: bool,
    pub gitlab_timeout: Option<u32>,
    pub download_force_protocol: ForceProtocol,
    pub upload_force_protocol: ForceProtocol,
    pub continue_on_error: bool,
    pub exclude_archived: bool,
}

#[tokio::main]
pub async fn clone(p: CloneParams) -> Result<()> {
    let fetch_gl = gitlab::Client::new(
        &p.fetch.token,
        p.fetch.url,
        p.objects_per_page,
        true,
        p.gitlab_timeout,
    )?;
    let mut projects = fetch_gl
        .get_projects(p.group, p.only_owned, p.only_membership, p.exclude_archived)
        .await?;

    if projects.is_empty() {
        bail!("Projects not found in GitLab");
    }

    if let Some(patterns) = p.patterns {
        projects = filter_projects(projects, patterns, p.limit)?;
    }

    if projects.is_empty() {
        bail!("All projects filtered out");
    }

    let dst = if let Some(dst) = p.dst {
        dst
    } else {
        format!("{}/{}", std::env::temp_dir().display(), TEMP_DIR)
    };

    if p.clear_dst {
        clear_dst(&dst);
    }

    let backup_data = if let Some(backup) = p.backup {
        let client = gitlab::Client::new(
            &backup.token,
            backup.url,
            None,
            p.disable_sync_date,
            p.gitlab_timeout,
        )?;
        let group = if let Some(gr) = backup.group {
            Some(client.get_group(&gr).await?)
        } else {
            None
        };
        let git_http_auth = if p.upload_ssh {
            None
        } else {
            Some(make_git_http_auth(&client, &backup.token).await?)
        };

        Some(BackupData {
            client,
            group,
            git_http_auth,
            force_protocol: p.upload_force_protocol,
        })
    } else {
        None
    };

    let fetch_git_http_auth = if p.download_ssh {
        None
    } else {
        Some(make_git_http_auth(&fetch_gl, &p.fetch.token).await?)
    };

    if p.dry_run {
        if let Some(backup_data) = &backup_data {
            if let Some(g) = backup_data.group.as_ref() {
                println!(
                    "Backup group:   {} (id: {}, path: {})",
                    g.name, g.id, g.full_path
                );
            };
        }
        println!("Local out dir: {}", &dst);
        println!();
        for p in &projects {
            println!(
                "{: <32} (id: {}, path: {})",
                p.name, p.id, p.path_with_namespace
            );
        }
        return Ok(());
    }

    info!("start pulling");

    let mut pb = ProgressBar::new(projects.len() as u64);
    pb.message("Cloning: ");

    let groups_cache = Arc::new(Mutex::new(HashMap::new()));

    for chunk in projects.chunks(p.concurrency_limit) {
        let result: Result<Vec<()>> = join_all(chunk.iter().map(|pr| {
            clone_project(
                pr,
                &dst,
                p.only_master,
                &fetch_git_http_auth,
                &backup_data,
                p.disable_hierarchy,
                &p.download_force_protocol,
                &fetch_gl,
                groups_cache.clone(),
            )
        }))
        .await
        .into_iter()
        .collect();
        if result.is_err() {
            if p.continue_on_error {
                error!(
                    "Error while cloning: {} (please run with `-vv` for more details)",
                    result.unwrap_err()
                );
            } else {
                result?;
            }
        }
        pb.add(chunk.len() as u64);
    }

    Ok(())
}
