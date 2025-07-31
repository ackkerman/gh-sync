use anyhow::{anyhow, Context, Result};
use std::{path::Path, process::Command, process::Output};

/// git コマンドを実行して成功コードを保証
fn run(repo: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .status()
        .with_context(|| format!("failed to spawn git {:?}", args))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("git {:?} exited with {}", args, status))
    }
}

/// git コマンドを実行して Output を返す
fn run_output(repo: &Path, args: &[&str]) -> Result<Output> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed to spawn git {:?}", args))?;
    Ok(out)
}

/// `git remote add|get-url|set-url` 相当
pub fn ensure_remote(repo: &Path, name: &str, url: &str) -> Result<()> {
    let out = Command::new("git")
        .args(["remote", "get-url", name])
        .current_dir(repo)
        .output()?;

    if !out.status.success() {
        // まだ無い → add
        run(repo, &["remote", "add", name, url])
    } else if url.trim() != String::from_utf8_lossy(&out.stdout).trim() {
        // URL が違う → set-url
        run(repo, &["remote", "set-url", name, url])
    } else {
        Ok(())
    }
}

pub fn fetch(repo: &Path, remote: &str, branch: &str) -> Result<()> {
    run(repo, &["fetch", remote, branch])
}

pub fn subtree_pull(
    repo: &Path,
    prefix: &str,
    remote: &str,
    branch: &str,
    message: Option<&str>,
) -> Result<()> {
    let mut args = vec!["subtree", "pull", "--prefix", prefix, remote, branch];
    if let Some(m) = message {
        args.push("-m");
        args.push(m);
    }
    args.push("--squash");

    let out = run_output(repo, &args)?;

    if out.status.success() {
        Ok(())
    } else if String::from_utf8_lossy(&out.stderr).contains("use 'git subtree add'") {
        let mut add_args = vec!["subtree", "add", "--prefix", prefix, remote, branch];
        if let Some(m) = message {
            add_args.push("-m");
            add_args.push(m);
        }
        add_args.push("--squash");
        run(repo, &add_args)
    } else {
        Err(anyhow!(
            "git subtree pull failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

pub fn subtree_push(
    repo: &Path,
    prefix: &str,
    remote: &str,
    branch: &str,
    message: Option<&str>,
) -> Result<()> {
    let mut args = vec!["subtree", "push", "--prefix", prefix, remote, branch];
    if let Some(m) = message {
        args.push("-m");
        args.push(m);
    }
    run(repo, &args)
}
