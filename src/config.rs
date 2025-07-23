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
                        let entry =
                            cfg.mappings
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
