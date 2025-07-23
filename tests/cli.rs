use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn setup_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    // 実際に git init して最低限のリポジトリを用意
    std::process::Command::new("/usr/bin/git")
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("git init");
    dir
}

/// `git` 実行を丸ごとエコーに差し替えて成功を偽装
fn fake_git_path(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let shim = dir.path().join("git");
    fs::write(
        &shim,
        "#!/usr/bin/env sh\nif [ \"$1\" = \"config\" ]; then\n    /usr/bin/git \"$@\"\nelse\n    echo git \"$@\"\nfi\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
    }
    shim
}

/// `git subtree pull` を失敗させ、add へフォールバックさせるためのシム
fn fake_git_fail_pull(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let shim = dir.path().join("git");
    fs::write(
        &shim,
        r#"#!/usr/bin/env sh
if [ "$1" = "config" ]; then
    /usr/bin/git "$@"
elif [ "$1" = "remote" ] && [ "$2" = "get-url" ]; then
    exit 1
elif [ "$1" = "subtree" ] && [ "$2" = "pull" ]; then
    echo >&2 "hint: use 'git subtree add'"
    exit 1
else
    echo git "$@"
    exit 0
fi
"#,
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

#[test]
fn pull_falls_back_to_add() {
    let repo = setup_repo();
    let git_shim = fake_git_fail_pull(&repo);

    let path_env = format!(
        "{}:{}",
        git_shim.parent().unwrap().display(),
        std::env::var("PATH").unwrap()
    );

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .args(&["connect", "web-app", "git@github.com:a/b.git"])
        .assert()
        .success();

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .args(&["pull", "web-app"])
        .assert()
        .success()
        .stdout(predicate::str::contains("subtree add"));
}

#[test]
fn pull_with_custom_message() {
    let repo = setup_repo();
    // use failing pull shim so that fallback to add prints the command
    let git_shim = fake_git_fail_pull(&repo);

    let path_env = format!(
        "{}:{}",
        git_shim.parent().unwrap().display(),
        std::env::var("PATH").unwrap()
    );

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .args(&["connect", "web-app", "git@github.com:a/b.git"])
        .assert()
        .success();

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .args(&["pull", "web-app", "-m", "custom"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-m custom"));
}

#[test]
fn remove_mapping() {
    let repo = setup_repo();
    let git_shim = fake_git_path(&repo);

    let path_env = format!(
        "{}:{}",
        git_shim.parent().unwrap().display(),
        std::env::var("PATH").unwrap()
    );

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .args(&["connect", "web-app", "git@github.com:a/b.git"])
        .assert()
        .success();

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .args(&["remove", "web-app"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"));

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No mappings"));
}

