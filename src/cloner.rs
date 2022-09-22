use std::fs;
use pbr::ProgressBar;

use url::Url;

use crate::{git, gitlab};

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

pub fn clone(
    fetch: FetchGitlabOptions,
    dst: String,
    backup: Option<BackupGitlabOptions>,
) -> Result<(), String> {
    let fetch_gl = gitlab::Client::new(fetch.token, fetch.url)?;
    let projects = fetch_gl.get_projects().unwrap();

    let mut pb = ProgressBar::new(projects.len() as u64);
    pb.message("Pulling: ");

    for p in &projects {
        let relative_path = p.path_with_namespace.strip_suffix(&p.path).unwrap();
        let path = format!("{}/{}", &dst, relative_path);
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

    let mut pb = ProgressBar::new(projects.len() as u64);
    pb.message("Pushing: ");

    for p in projects {
        let path = p.path_with_namespace.split("/").map(str::to_string).collect();

        let backup_project = backup_gl
            .make_project_with_namespace(path, &root_group)
            .map_err(|e| e.to_string())?;

        git::push_backup(format!("{}/{}", dst, p.path_with_namespace), backup_project.ssh_url_to_repo)?;

        pb.inc();
    };

    Ok(())
}
