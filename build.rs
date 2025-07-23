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
   let args: Vec<String> = env::args().skip(1).collect();
   // helper: real git that bypasses this shim
   fn real_git(args: &[String]) -> ! {
       use std::process::{Command, exit};
       // ORIG_PATH はテスト側で注入してある
       let mut cmd = Command::new("git");
       if let Ok(orig) = std::env::var("ORIG_PATH") {
           cmd.env("PATH", orig);  // ← shim を除いた PATH に差し替え
       }
       let status = cmd.args(args).status().expect("spawn real git");
       exit(status.code().unwrap_or(1));
   }
    if args.get(0).map(|s| s == "config").unwrap_or(false) {
        real_git(&args);            // ← 本物へフォワードして終了
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
