use anyhow::{bail, Result};
use std::ffi::OsStr;
use std::str::from_utf8;
use tokio::process::Command;
use tracing::{error, info, warn};

async fn git<S: AsRef<OsStr>>(args: Vec<S>) -> Result<String> {
    let mut git_cmd = "git".to_string();
    for a in &args {
        git_cmd += &format!(" {}", a.as_ref().to_str().unwrap());
    }
    info!("{}", git_cmd);

    let cmd = Command::new("git").args(args).output().await?;

    let errmsg = from_utf8(&cmd.stderr).unwrap_or_default();
    if !cmd.status.success() {
        warn!("git exit status not success");
        bail!("git error: {}", errmsg);
    }

    if !errmsg.is_empty() {
        info!("{}", errmsg);
    }

    Ok(from_utf8(&cmd.stdout)?.to_string())
}

async fn check_status(path: &String) -> Result<()> {
    git(vec!["-C", path, "rev-parse", "--is-inside-work-tree"])
        .await
        .map(|_| ())
}

async fn clone(src: &String, dst: &String) -> Result<()> {
    git(vec!["clone", src, dst]).await?;
    git(vec!["-C", dst, "config", "pull.rebase", "false"]).await?;

    Ok(())
}

async fn update(path: &String, only_master: bool) -> Result<()> {
    // In older versions of GitLobster, we used to rename the "origin" remote to "upstream".
    // For backward compatibility, we should revert the default remote name back to "origin",
    // while we don't increase the major version number.
    git(vec!["-C", path, "remote", "rename", "upstream", "origin"])
        .await
        .ok();

    if only_master {
        git(vec!["-C", path, "pull"]).await?;
        return Ok(());
    }

    git(vec!["-C", path, "fetch", "--all"]).await?;

    let branches_out = git(vec!["-C", path, "branch", "-la"]).await?;
    let branches = branches_out
        .split('\n')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .filter(|v| !v.starts_with("remotes/origin/HEAD"))
        .filter(|v| !v.starts_with("remotes/backup"));

    let remote_prefix = "remotes/origin/";
    let mut remote_branches: Vec<&str> = vec![];
    let mut default_branch = "";

    for b in branches {
        if b.starts_with(remote_prefix) {
            remote_branches.push(b);
            continue;
        }
        if b.starts_with('*') {
            default_branch = b
                .strip_prefix('*')
                .expect("situation is unreachable")
                .trim();
            continue;
        }
        git(vec!["-C", path, "branch", "-D", b]).await?;
    }

    for b in remote_branches {
        let local_branch_name = b
            .strip_prefix(remote_prefix)
            .expect("situation is unreachable");

        if !b.ends_with(&default_branch) {
            git(vec!["-C", path, "branch", "--track", local_branch_name, b]).await?;
        }
    }

    git(vec!["-C", path, "pull", "origin", default_branch]).await?;

    Ok(())
}

async fn add_remote_backup(path: &String, remote: String) -> Result<()> {
    let _ = git(vec!["-C", path, "remote", "remove", "backup"]).await;
    git(vec!["-C", path, "remote", "add", "backup", &remote]).await?;
    Ok(())
}

async fn push_all_remote_backup(path: String) -> Result<()> {
    if let Err(e) = git(vec!["-C", &path, "push", "-u", "backup", "--all"]).await {
        error!("{}", e);
    };
    if let Err(e) = git(vec!["-C", &path, "push", "-u", "backup", "--tags"]).await {
        error!("{}", e);
    };
    Ok(())
}

pub async fn fetch(src: String, dst: String, only_master: bool) -> Result<()> {
    match check_status(&dst).await {
        Ok(()) => (),
        Err(_) => clone(&src, &dst).await?,
    };
    update(&dst, only_master).await
}

pub async fn push_backup(path: String, remote: String) -> Result<()> {
    add_remote_backup(&path, remote).await?;
    push_all_remote_backup(path).await
}
