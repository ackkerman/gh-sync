#[cfg(target_os = "windows")]
fn main() {
    use std::{fs, path::PathBuf};

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let shim_c = out_dir.join("git_shim.c");
    fs::write(&shim_c, r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <process.h>

int main(int argc, char** argv) {
    if (argc > 1 && strcmp(argv[1], "config") == 0) {
        const char* orig = getenv("ORIG_PATH");
        if (orig) _putenv_s("PATH", orig);
        _spawnvp(_P_WAIT, "git", (const char * const*)(argv + 1));
        return 0;
    }
    printf("git");
    for (int i = 1; i < argc; ++i) printf(" %s", argv[i]);
    printf("\n");
    return 0;
}
"#).unwrap();

    let fail_c = out_dir.join("git_fail_pull.c");
    fs::write(&fail_c, r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <process.h>

int main(int argc, char** argv) {
    if (argc > 1 && strcmp(argv[1], "config") == 0) {
        const char* orig = getenv("ORIG_PATH");
        if (orig) _putenv_s("PATH", orig);
        _spawnvp(_P_WAIT, "git", (const char * const*)(argv + 1));
        return 0;
    }
    if (argc > 2 && strcmp(argv[1], "remote") == 0 && strcmp(argv[2], "get-url") == 0) {
        return 1;
    }
    if (argc > 3 && strcmp(argv[1], "subtree") == 0 && strcmp(argv[2], "pull") == 0) {
        fprintf(stderr, "hint: use 'git subtree add'\n");
        return 1;
    }
    printf("git");
    for (int i = 1; i < argc; ++i) printf(" %s", argv[i]);
    printf("\n");
    return 0;
}
"#).unwrap();

    fn compile_exe(source: &PathBuf, out_name: &str) {
        let compiler = cc::Build::new().get_compiler();
        let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let exe = out_dir.join(out_name);
        let mut cmd = compiler.to_command();
        if compiler.is_like_msvc() {
            cmd.arg(source).arg("/Fe").arg(&exe);
        } else {
            cmd.arg(source).arg("-o").arg(&exe);
        }
        let status = cmd.status().expect("compile");
        assert!(status.success(), "failed to compile stub");
    }

    compile_exe(&shim_c, "git_shim.exe");
    compile_exe(&fail_c, "git_fail_pull.exe");
}

#[cfg(not(target_os = "windows"))]
fn main() {}
