use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn setup_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap(); // 疑似リポジトリ
    dir
}

/// `git` 実行を丸ごとエコーに差し替えて成功を偽装
fn fake_git_path(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let shim = dir.path().join("git");
    fs::write(
        &shim,
        "#!/usr/bin/env sh\n# pretend success\necho git \"$@\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
    }
    shim
}

#[test]
fn connect_and_list_roundtrip() {
    let repo = setup_repo();
    let git_shim = fake_git_path(&repo);

    let mut cmd = Command::cargo_bin("gh-sync").unwrap();
    cmd.current_dir(repo.path())
        .env(
            "PATH",
            format!(
                "{}:{}",
                git_shim.parent().unwrap().display(),
                std::env::var("PATH").unwrap()
            ),
        )
        .args(&[
            "connect",
            "web-app",
            "git@github.com:a/b.git",
            "--branch",
            "dev_ui",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Connected"));

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("web-app"));
}
