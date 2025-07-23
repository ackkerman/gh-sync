#[cfg(target_os = "windows")]
fn main() {
    use std::{env, fs, path::PathBuf, process::Command};

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let shim_src = out_dir.join("git_shim.rs");
    let shim_bin = out_dir.join("git_shim.exe");

    // ① ミニマムなスタブを書き出す
    fs::write(
        &shim_src,
        r#"
use std::{env, process::{Command, exit}};
fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(|s| s == "config").unwrap_or(false) {
        /* git config → 本物を叩く */
        exit(Command::new("git.exe").args(&args).status().unwrap().code().unwrap_or(1));
    }
    if args.len() >= 3 && args[0] == "subtree" && args[1] == "pull" {
        eprintln!("hint: use 'git subtree add'");
        exit(1); // わざと失敗
    }
    println!("git {}", args.join(" "));
}"#,
    )
    .unwrap();

    // ② rustc でコンパイル（-C opt-level=0 で充分）
    let status = Command::new(env::var("RUSTC").unwrap_or_else(|_| "rustc".into()))
        .args(&[
            "-O",
            shim_src.to_str().unwrap(),
            "-o",
            shim_bin.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run rustc");
    assert!(status.success(), "failed to compile stub");

    // ③ テスト側へパスを渡す
    println!("cargo:rustc-env=GIT_SHIM_BIN={}", shim_bin.display());
}

#[cfg(not(target_os = "windows"))]
fn main() {}
