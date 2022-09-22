// TODO: handle stderr

use std::process::Command;
use std::str::from_utf8;

fn check_status(path: &String) -> Result<(), String> {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("status")
        .output()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn clone(src: &String, dst: &String) -> Result<(), String> {
    Command::new("git")
        .arg("clone")
        .arg(src)
        .arg(dst)
        .output()
        .map(|_| ())
        .map_err(|e| e.to_string())?;

    Command::new("git")
        .arg("-C")
        .arg(dst)
        .arg("remote")
        .arg("rename")
        .arg("origin")
        .arg("upstream")
        .output()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn update(path: &String) -> Result<(), String> {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("fetch")
        .arg("--all")
        .output()
        .map(|_| ())
        .map_err(|e| e.to_string())?;

    let branches_out = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("branch")
        .arg("-la")
        .output()
        .map_err(|e| e.to_string())?;

    let branches = from_utf8(&branches_out.stdout)
        .map_err(|e| e.to_string())?
        .split("\n")
        .into_iter()
        .map(|v| v.trim())
        .filter(|v| !v.starts_with("*") && !v.is_empty() && !v.starts_with("remotes/upstream/HEAD"));

    let mut remote_branches: Vec<&str> = vec![];

    for b in branches {
        if b.starts_with("remotes/upstream") {
            remote_branches.append(&mut vec![b]);
            continue;
        }
        Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("branch")
            .arg("-D")
            .arg(b)
            .output()
            .map(|_| ())
            .map_err(|e| e.to_string())?;
    }

    for b in remote_branches {
        // TODO: remove unwrap
        let local_branch_name = b.strip_prefix("remotes/upstream/").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("branch")
            .arg("--track")
            .arg(local_branch_name)
            .arg(b)
            .output()
            .map(|_| ())
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn add_remote_backup(path: &String, remote: String) {
    let _ = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("remote")
        .arg("add")
        .arg("backup")
        .arg(remote)
        .output();
}

fn push_all_remote_backup(path: String) -> Result<(), String> {
    Command::new("git")
        .arg("-C")
        .arg(&path)
        .arg("push")
        .arg("-u")
        .arg("backup")
        .arg("--all")
        .output()
        .map_err(|e| e.to_string())?;

    Command::new("git")
        .arg("-C")
        .arg(&path)
        .arg("push")
        .arg("-u")
        .arg("backup")
        .arg("--tags")
        .output()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn fetch(src: String, dst: String) -> Result<(), String> {
    check_status(&dst).unwrap_or(clone(&src, &dst)?);
    update(&dst)
}

pub fn push_backup(path: String, remote: String) -> Result<(), String> {
    add_remote_backup(&path, remote);
    push_all_remote_backup(path)
}
