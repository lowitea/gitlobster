use std::fs;

use pbr::ProgressBar;
use regex::Regex;
use tracing::{debug, info, instrument};
use url::Url;

use crate::{git, gitlab};
use crate::gitlab::types;

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

fn filter_projects(
    mut projects: Vec<types::Project>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
) -> Result<Vec<types::Project>, String> {
    if exclude.is_some() && include.is_some() {
        panic!("you cannot use include and exclude filters together")
    }

    if let Some(i_filters) = include {
        let mut filters: Vec<Regex> = vec![];
        for f in i_filters {
            filters.push(Regex::new(&f).map_err(|e|e.to_string())?);
        }
        projects = projects.into_iter().filter(
            move |p| {
                for filter in filters.clone() {
                    if filter.is_match(&p.path_with_namespace) {
                        return true;
                    }
                }
                false
            }
        ).collect();
    } else if let Some(e_filters) = exclude {
        let mut filters: Vec<Regex> = vec![];
        for f in e_filters {
            filters.push(Regex::new(&f).map_err(|e|e.to_string())?);
        }
        projects = projects.into_iter().filter(
            move |p| {
                for filter in filters.clone() {
                    if filter.is_match(&p.path_with_namespace) {
                        return false;
                    }
                }
                true
            }

        ).collect();
    }

    Ok(projects)
}

#[instrument(level = "trace")]
pub fn clone(
    fetch: FetchGitlabOptions,
    dst: String,
    backup: Option<BackupGitlabOptions>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
) -> Result<(), String> {
    let fetch_gl = gitlab::Client::new(fetch.token, fetch.url)?;
    let projects = filter_projects(fetch_gl.get_projects()?, include, exclude)?;

    info!("start pulling");

    let mut pb = ProgressBar::new(projects.len() as u64);
    pb.message("Pulling: ");

    for p in &projects {
        debug!("project path: {}", &p.path_with_namespace);

        let dir_path = p.path_with_namespace.strip_suffix(&p.path).unwrap();
        let path = format!("{}/{}", &dst, dir_path);
        fs::create_dir_all(path).map_err(|e| e.to_string())?;

        git::fetch(p.ssh_url_to_repo.clone(), format!("{}/{}", dst, p.path_with_namespace))?;

        pb.inc();
    }

    let (backup_gl, backup_group) = if let Some(backup) = backup {
        (gitlab::Client::new(backup.token, backup.url)?, backup.group)
    } else {
        return Ok(());
    };

    let root_group = backup_gl
        .get_group(backup_group)
        .map_err(|e| e.to_string())?;

    info!("start pushing");

    let mut pb = ProgressBar::new(projects.len() as u64);
    pb.message("Pushing: ");

    for p in projects {
        let path = p.path_with_namespace.split("/").map(str::to_string).collect();

        let backup_project = backup_gl
            .make_project_with_namespace(path, &root_group, &p)
            .map_err(|e| e.to_string())?;

        git::push_backup(format!("{}/{}", dst, p.path_with_namespace), backup_project.ssh_url_to_repo)?;

        pb.inc();
    };

    Ok(())
}
