use std::fs;

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

    // TODO: add progress bar
    for p in projects {
        let relative_path = p.path_with_namespace.strip_suffix(&p.path).unwrap();
        let path = format!("{}/{}", &dst, relative_path);
        fs::create_dir_all(path).map_err(|e| e.to_string())?;
        git::fetch(p.ssh_url_to_repo, format!("{}/{}", dst, p.path_with_namespace))?
    }

    Ok(())
}
