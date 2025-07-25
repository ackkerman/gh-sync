use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn path_vars(shim: &PathBuf) -> (std::ffi::OsString, std::ffi::OsString) {
    let orig = std::env::var_os("PATH").unwrap();
    let mut paths = std::env::split_paths(&orig).collect::<Vec<_>>();
    paths.insert(0, shim.parent().unwrap().into());
    let joined = std::env::join_paths(paths).unwrap();
    (joined, orig)
}

fn setup_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    // 実際に git init して最低限のリポジトリを用意
    std::process::Command::new("git")
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("git init");
    dir
}

/// `git` 実行を丸ごとエコーに差し替えて成功を偽装
fn fake_git_path(dir: &tempfile::TempDir) -> PathBuf {
    #[cfg(windows)]
    {
        let src = PathBuf::from(env!("GIT_SHIM_BIN"));
        let dst = dir.path().join("git.exe");
        fs::copy(src, &dst).unwrap();
        return dst;
    }

    #[cfg(not(windows))]
    {
        let shim = dir.path().join("git");
        fs::write(
            &shim,
            "#!/usr/bin/env sh\nif [ \"$1\" = \"config\" ]; then\n  PATH=\"$ORIG_PATH\" git \"$@\"\nelse\n  echo git \"$@\"\nfi\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
        shim
    }
}

/// `git subtree pull` を失敗させ、add へフォールバックさせるためのシム
fn fake_git_fail_pull(dir: &tempfile::TempDir) -> PathBuf {
    #[cfg(windows)]
    {
        let src = PathBuf::from(env!("GIT_SHIM_BIN"));
        let dst = dir.path().join("git.exe");
        fs::copy(src, &dst).unwrap();
        return dst;
    }

    #[cfg(not(windows))]
    {
        let shim = dir.path().join("git");
        fs::write(
            &shim,
            r#"#!/usr/bin/env sh
if [ "$1" = "config" ]; then
  PATH="$ORIG_PATH" git "$@"
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
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
        shim
    }
}

#[test]
fn connect_and_list_roundtrip() {
    let repo = setup_repo();
    let git_shim = fake_git_path(&repo);

    let (path_env, orig_path) = path_vars(&git_shim);
    let mut cmd = Command::cargo_bin("gh-sync").unwrap();
    cmd.current_dir(repo.path())
        .env("PATH", &path_env)
        .env("ORIG_PATH", &orig_path)
        .args(&[
            "connect",
            "app",
            "git@github.com:a/b.gitt",
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
        .stdout(predicate::str::contains("app"));
}

#[test]
fn pull_falls_back_to_add() {
    let repo = setup_repo();
    let git_shim = fake_git_fail_pull(&repo);

    let (path_env, orig_path) = path_vars(&git_shim);

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .env("ORIG_PATH", &orig_path)
        .args(&[
            "connect",
            "app",
            "git@github.com:a/b.gitt",
        ])
        .assert()
        .success();

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .env("ORIG_PATH", &orig_path)
        .args(&["pull", "app"])
        .assert()
        .success()
        .stdout(predicate::str::contains("subtree add"));
}

#[test]
fn pull_with_custom_message() {
    let repo = setup_repo();
    // use failing pull shim so that fallback to add prints the command
    let git_shim = fake_git_fail_pull(&repo);

    let (path_env, orig_path) = path_vars(&git_shim);

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .env("ORIG_PATH", &orig_path)
        .args(&[
            "connect",
            "app",
            "git@github.com:a/b.gitt",
        ])
        .assert()
        .success();

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .env("ORIG_PATH", &orig_path)
        .args(&["pull", "app", "-m", "custom"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-m custom"));
}

#[test]
fn remove_mapping() {
    let repo = setup_repo();
    let git_shim = fake_git_path(&repo);

    let (path_env, orig_path) = path_vars(&git_shim);

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .env("ORIG_PATH", &orig_path)
        .args(&[
            "connect",
            "app",
            "git@github.com:a/b.gitt",
        ])
        .assert()
        .success();

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .env("ORIG_PATH", &orig_path)
        .args(&["remove", "app"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"));

    Command::cargo_bin("gh-sync")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", &path_env)
        .env("ORIG_PATH", &orig_path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No mappings"));
}
