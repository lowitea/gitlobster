use std::ffi::OsStr;
use std::process::Command;
use std::str::from_utf8;

use tracing::{info, error, warn};

fn git<S: AsRef<OsStr>>(args: Vec<S>) -> Result<String, String> {
    let mut git_cmd = "git".to_string();
    for a in &args {
        git_cmd += &format!(" {}", a.as_ref().to_str().unwrap());
    }
    info!("{}", git_cmd);

    let cmd = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    let errmsg = if !cmd.stderr.is_empty() {
        let err = from_utf8(&cmd.stderr).map_err(|e| e.to_string())?;
        warn!(err);
        err
    } else { "" };

    if !cmd.status.success() {
        warn!("git exit status not success");
        return Err(format!("git error: {}", errmsg));
    }

    Ok(from_utf8(&cmd.stdout).map_err(|e| e.to_string())?.to_string())
}

fn check_status(path: &String) -> Result<(), String> {
    git(vec!("-C", path, "rev-parse", "--is-inside-work-tree")).map(|_| ())
}

fn clone(src: &String, dst: &String) -> Result<(), String> {
    git(vec!("clone", src, dst))?;
    git(vec!("-C", dst, "remote", "rename", "origin", "upstream"))?;

    Ok(())
}

fn update(path: &String) -> Result<(), String> {
    git(vec!("-C", path, "fetch", "--all"))?;

    let branches_out = git(vec!("-C", path, "branch", "-la"))?;
    let branches = branches_out
        .split("\n")
        .into_iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .filter(|v| !v.starts_with("remotes/upstream/HEAD"))
        .filter(|v| !v.starts_with("remotes/backup"));

    let remote_prefix = "remotes/upstream/";
    let mut remote_branches: Vec<&str> = vec![];
    let mut default_branch = "";

    for b in branches {
        if b.starts_with(&remote_prefix) {
            remote_branches.push(b);
            continue;
        }
        if b.starts_with("*") {
            default_branch = b.strip_prefix("*").expect("situation is unreachable").trim();
            continue;
        }
        git(vec!("-C", path, "branch", "-D", b))?;
    }

    for b in remote_branches {
        let local_branch_name = b.strip_prefix(&remote_prefix)
            .expect("situation is unreachable");

        if !b.ends_with(&default_branch) {
            git(vec!("-C", path, "branch", "--track", local_branch_name, b))?;
        }
    }

    git(vec!["-C", path, "pull", "upstream", default_branch])?;

    Ok(())
}

fn add_remote_backup(path: &String, remote: String) -> Result<(), String> {
    let _ = git(vec!("-C", path, "remote", "remove", "backup"));
    git(vec!("-C", path, "remote", "add", "backup", &remote))?;
    Ok(())
}

fn push_all_remote_backup(path: String) -> Result<(), String> {
    if let Err(e) = git(vec!("-C", &path, "push", "-u", "backup", "--all")) {
        error!(e)
    };
    if let Err(e) = git(vec!("-C", &path, "push", "-u", "backup", "--tags")) {
        error!(e)
    };
    Ok(())
}

pub fn fetch(src: String, dst: String) -> Result<(), String> {
    match check_status(&dst) {
        Ok(_) => (),
        Err(_) => clone(&src, &dst)?,
    };
    update(&dst)
}

pub fn push_backup(path: String, remote: String) -> Result<(), String> {
    add_remote_backup(&path, remote)?;
    push_all_remote_backup(path)
}
