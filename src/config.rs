use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

/// リポジトリ直下に作成される設定ファイル
pub const CONFIG_FILE: &str = ".gh-sync.json";

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
        let path = repo_root.join(CONFIG_FILE);
        if path.exists() {
            let txt = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&txt)?)
        } else {
            Ok(Self::default())
        }
    }

    /// 保存（pretty print）
    pub fn save(&self, repo_root: &Path) -> anyhow::Result<()> {
        let path = repo_root.join(CONFIG_FILE);
        let txt = serde_json::to_string_pretty(self)?;
        fs::write(path, txt)?;
        Ok(())
    }
}
