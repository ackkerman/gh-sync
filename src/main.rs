mod config;
mod gitops;

use anyhow::{Context, Result};
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
                    println!("{} ↔ {} [{} / {}]", m.subdir, m.url, m.remote, m.branch);
                }
            }
        }
    }
    Ok(())
}
