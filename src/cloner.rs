use std::fs;

use crate::{git, gitlab};

pub fn clone(token: String, url: String, dst: String) -> Result<(), String> {
    let gl = gitlab::Client::new(token, url)?;
    let projects = gl.get_projects().unwrap();

    // TODO: add progress bar
    for p in projects {
        let relative_path = p.path_with_namespace.strip_suffix(&p.path).unwrap();
        let path = format!("{}/{}", &dst, relative_path);
        fs::create_dir_all(path).map_err(|e| e.to_string())?;
        git::fetch(p.ssh_url_to_repo, format!("{}/{}", dst, p.path_with_namespace))?
    }

    Ok(())
}
