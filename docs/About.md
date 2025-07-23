USER: 

Github Enterpriseの1レポジトリのうちの１ディレクトリをGithub.comの1リポジトリとするようなツールを作りたい。

例えばGithub Enterprise側をroot/web-app, Github.com側をackkerman/ibm-nlo.git とする場合、現在は以下のようなコマンドになる
```
# 初回
git remote add ibm-nlo git@github.com:ackkerman/ibm-nlo.git 

# Pull
git fetch ibm-nlo dev_ui
git subtree pull --prefix=web-app ibm-nlo dev_ui --squash

# 逆Push
git subtree push --prefix=web-app ibm-nlo dev_ui
```
これを
<新コマンド> connect xxx xxx
<新コマンド> pull xxx xxx
<新コマンド> push xxx xxx
<新コマンド> list 
みたいな簡潔なコマンドで実行できるようにしたい


Answer: 

以下は **git subtree** をラップして

* `connect`
* `pull`
* `push`
* `list`

の 4 コマンドで扱える軽量 CLI ツール **subsync** の最小実装例や。

ほな、Python 版 **subsync/gh‑sync** を **Rust (cargo)** で書き直した “最小実用” 実装を置いとくで。
`clap` で CLI、`serde`/`serde_json` で設定ファイル、`std::process::Command` で git を呼び出す構成や。テストは `assert_cmd` と `tempfile` で外部 Git をモックしてるから、CI でも安全に回せる。

---

### 1. ディレクトリ構成

```
gh-sync-rs/
├── Cargo.toml
├── src/
│   ├── main.rs          # CLI 定義 & エントリポイント
│   ├── config.rs        # git config 読み書き
│   └── gitops.rs        # git サブコマンドを薄ラップ
└── tests/
    └── cli.rs           # end‑to‑end テスト
```

---

### 2. Cargo.toml

```toml
[package]
name = "gh-sync"
version = "0.1.0"
edition = "2021"

[dependencies]
clap        = { version = "4.5", features = ["derive"] }
serde       = { version = "1.0", features = ["derive"] }
serde_json  = "1.0"
anyhow      = "1.0"
thiserror   = "1.0"

[dev-dependencies]
assert_cmd  = "2.0"
tempfile    = "3.10"
predicates  = "3.1"
```

---

### 3. src/config.rs

```rust
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, process::Command};

/// git config に保存するプレフィックス
pub const CONFIG_PREFIX: &str = "gh-sync";

/// 1 マッピング = 1 サブディレクトリ ↔ 1 リモート
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mapping {
    pub subdir: String,
    pub remote: String,
    pub url: String,
    pub branch: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub mappings: HashMap<String, Mapping>,
}

impl Config {
    /// 読み込み（無ければ空設定）
    pub fn load(repo_root: &Path) -> anyhow::Result<Self> {
        let out = Command::new("git")
            .args(["config", "--local", "--get-regexp", "^gh-sync\\."])
            .current_dir(repo_root)
            .output()?;

        if !out.status.success() {
            return Ok(Self::default());
        }

        let stdout = String::from_utf8(out.stdout)?;
        let mut cfg = Config::default();
        for line in stdout.lines() {
            if let Some((key, value)) = line.split_once(' ') {
                let mut parts = key.split('.');
                if parts.next() != Some(CONFIG_PREFIX) {
                    continue;
                }
                if let Some(name) = parts.next() {
                    if let Some(field) = parts.next() {
                        let entry = cfg
                            .mappings
                            .entry(name.to_string())
                            .or_insert_with(|| Mapping {
                                subdir: name.to_string(),
                                remote: String::new(),
                                url: String::new(),
                                branch: String::new(),
                            });
                        match field {
                            "remote" => entry.remote = value.to_string(),
                            "url" => entry.url = value.to_string(),
                            "branch" => entry.branch = value.to_string(),
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(cfg)
    }

    /// 保存
    pub fn save(&self, repo_root: &Path) -> anyhow::Result<()> {
        for (name, m) in &self.mappings {
            let key_remote = format!("{CONFIG_PREFIX}.{}.remote", name);
            Command::new("git")
                .args(["config", "--local", "--replace-all", &key_remote, &m.remote])
                .current_dir(repo_root)
                .status()?;

            let key_url = format!("{CONFIG_PREFIX}.{}.url", name);
            Command::new("git")
                .args(["config", "--local", "--replace-all", &key_url, &m.url])
                .current_dir(repo_root)
                .status()?;

            let key_branch = format!("{CONFIG_PREFIX}.{}.branch", name);
            Command::new("git")
                .args(["config", "--local", "--replace-all", &key_branch, &m.branch])
                .current_dir(repo_root)
                .status()?;
        }
        Ok(())
    }
}
```

---

### 4. src/gitops.rs

```rust
use anyhow::{anyhow, Context, Result};
use std::{path::Path, process::Command};

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

pub fn subtree_pull(repo: &Path, prefix: &str, remote: &str, branch: &str) -> Result<()> {
    run(
        repo,
        &["subtree", "pull", "--prefix", prefix, remote, branch, "--squash"],
    )
}

pub fn subtree_push(repo: &Path, prefix: &str, remote: &str, branch: &str) -> Result<()> {
    run(
        repo,
        &["subtree", "push", "--prefix", prefix, remote, branch],
    )
}
```

---

### 5. src/main.rs

```rust
mod config;
mod gitops;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use config::{Config, Mapping};
use gitops::*;
use std::{path::PathBuf, process};

/// gh-sync: Git subtree を簡潔コマンドで操作
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// SUBDIR ↔ REMOTE_URL を登録
    Connect {
        subdir: String,
        remote_url: String,
        #[arg(short, long, default_value = "main")]
        branch: String,
        /// git remote 名 (省略時は URL の basename)
        #[arg(short = 'n', long)]
        remote: Option<String>,
    },
    /// git fetch → subtree pull
    Pull {
        subdir: String,
        #[arg(short, long)]
        branch: Option<String>,
    },
    /// subtree push
    Push {
        subdir: String,
        #[arg(short, long)]
        branch: Option<String>,
    },
    /// 現在のマッピングを表示
    List,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        process::exit(1);
    }
}

fn repo_root() -> Result<PathBuf> {
    Ok(std::env::current_dir()?.canonicalize()?)
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let repo = repo_root()?;
    let mut cfg = Config::load(&repo)?;

    match cli.command {
        Commands::Connect {
            subdir,
            remote_url,
            branch,
            remote,
        } => {
            let remote_name = remote.unwrap_or_else(|| {
                PathBuf::from(remote_url.clone())
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .into()
            });

            let mapping = Mapping {
                subdir: subdir.clone(),
                remote: remote_name.clone(),
                url: remote_url.clone(),
                branch: branch.clone(),
            };
            ensure_remote(&repo, &remote_name, &remote_url)?;
            cfg.mappings.insert(subdir.clone(), mapping);
            cfg.save(&repo)?;

            println!("Connected {subdir} ↔ {remote_url} ({branch})");
        }

        Commands::Pull { subdir, branch } => {
            let m = cfg
                .mappings
                .get(&subdir)
                .with_context(|| format!("{subdir} not connected"))?;

            let branch = branch.unwrap_or_else(|| m.branch.clone());
            fetch(&repo, &m.remote, &branch)?;
            subtree_pull(&repo, &m.subdir, &m.remote, &branch)?;
            println!("Pulled {subdir} from {}/{}", m.remote, branch);
        }

        Commands::Push { subdir, branch } => {
            let m = cfg
                .mappings
                .get(&subdir)
                .with_context(|| format!("{subdir} not connected"))?;
            let branch = branch.unwrap_or_else(|| m.branch.clone());
            subtree_push(&repo, &m.subdir, &m.remote, &branch)?;
            println!("Pushed {subdir} to {}/{}", m.remote, branch);
        }

        Commands::List => {
            if cfg.mappings.is_empty() {
                println!("No mappings defined.");
            } else {
                for m in cfg.mappings.values() {
                    println!(
                        "{} ↔ {} [{} / {}]",
                        m.subdir, m.url, m.remote, m.branch
                    );
                }
            }
        }
    }
    Ok(())
}
```

---

### 6. tests/cli.rs

```rust
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
    let shim = dir.path().join("git.sh");
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
        .env("PATH", format!("{}:{}", git_shim.parent().unwrap().display(), std::env::var("PATH").unwrap()))
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
```

---

### 7. 使い方（人間向けメモ）

```bash
# 1 回だけ接続を登録
cargo install --path .          # or `cargo build --release`
gh-sync connect web-app git@github.com:ackkerman/ibm-nlo.git --branch dev_ui

# 取り込み
gh-sync pull web-app

# 逆方向
gh-sync push web-app

# 確認
gh-sync list
```

実行すると `.git/config` 内に次のようなエントリが保存されるで。

```
[gh-sync "web-app"]
    remote = ibm-nlo
    url = git@github.com:ackkerman/ibm-nlo.git
    branch = dev_ui
```

複数サブディレクトリもカバーできる仕組みや。

---

### 8. 今後の拡張アイデア（Rust 版でも同じ）

| 機能              | 概要                               |
| --------------- | -------------------------------- |
| `status` サブコマンド | `git diff --subtree` を噛ませて差分チェック |
| `sync`          | pull → push のワンライナー              |
| コンフリクト検知        | エラーコードを解析して自動 abort & 手順提示       |
| GitHub Actions  | Git config の `gh-sync.*` を読み取って定期同期 |

---

ほな、Rust 版の叩き台はこんな感じや。
「ここもうちょい凝りたい」「serde\_yaml で YAML 化したい」「Git2-Rs バインディング経由で pure Rust にしたい」などあったら気軽に言うてな！
