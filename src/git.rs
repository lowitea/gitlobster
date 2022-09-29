use std::ffi::OsStr;
use std::process::Command;
use std::str::from_utf8;

fn git<S: AsRef<OsStr>>(args: Vec<S>) -> Result<String, String> {
    let cmd = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    // TODO: enable for debug flag
    // if !cmd.stderr.is_empty() {
    //     let err = from_utf8(&cmd.stderr).map_err(|e| e.to_string())?;
    //     println!("Warning: {}", err);
    // }

    Ok(from_utf8(&cmd.stderr).map_err(|e| e.to_string())?.to_string())
}

fn check_status(path: &String) -> Result<(), String> {
    git(vec!("-C", path, "status")).map(|_| ())
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
        .filter(|v| !v.starts_with("*"))
        .filter(|v| !v.starts_with("remotes/upstream/HEAD"));

    let remote_prefix = "remotes/upstream/";
    let mut remote_branches: Vec<&str> = vec![];

    for b in branches {
        if b.starts_with(&remote_prefix) {
            remote_branches.push(b);
            continue;
        }
        git(vec!("-C", path, "branch", "-D", b))?;
    }

    for b in remote_branches {
        let local_branch_name = b.strip_prefix(&remote_prefix)
            .expect("situation is unreachable");

        git(vec!("-C", path, "branch", "--track", local_branch_name, b))?;
    }

    Ok(())
}

fn add_remote_backup(path: &String, remote: String) -> Result<(), String> {
    git(vec!("-C", path, "remote", "remove", "backup"))?;
    git(vec!("-C", path, "remote", "add", "backup", &remote))?;
    Ok(())
}

fn push_all_remote_backup(path: String) -> Result<(), String> {
    git(vec!("-C", &path, "push", "-u", "backup", "--all"))?;
    git(vec!("-C", &path, "push", "-u", "backup", "--tags"))?;
    Ok(())
}

pub fn fetch(src: String, dst: String) -> Result<(), String> {
    check_status(&dst).unwrap_or(clone(&src, &dst)?);
    update(&dst)
}

pub fn push_backup(path: String, remote: String) -> Result<(), String> {
    add_remote_backup(&path, remote)?;
    push_all_remote_backup(path)
}
