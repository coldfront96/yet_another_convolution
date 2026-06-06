//! Integration tests that drive the `convolution` binary, covering the
//! repo-bound keyfile workflow end to end: keygen -> encode -> run, and the
//! crucial failure case where the keyfile is absent.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_convolution")
}

/// A unique scratch directory for one test.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cnv_cli_{}_{}", tag, std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run the binary with args, returning (success, stdout).
fn run(args: &[&str], dir: &Path) -> (bool, String) {
    let out = Command::new(bin())
        .args(args)
        // Clear any ambient key so resolution depends only on flags/keyfile.
        .env_remove("CONVOLUTION_KEY")
        .current_dir(dir)
        .output()
        .unwrap();
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn keygen_encode_run_round_trip_is_repo_bound() {
    let dir = scratch("repobound");
    let keyfile = dir.join(".wildkey");
    let prog = dir.join("prog.wild");
    fs::write(&prog, "{stack 6 7 * .}\n").unwrap();

    // 1. Generate a private keyfile.
    let (ok, _) = run(&["keygen", "--keyfile", keyfile.to_str().unwrap()], &dir);
    assert!(ok, "keygen should succeed");
    assert!(keyfile.is_file(), "keygen should create the keyfile");

    // keygen refuses to clobber an existing keyfile.
    let (clobber_ok, _) = run(&["keygen", "--keyfile", keyfile.to_str().unwrap()], &dir);
    assert!(!clobber_ok, "keygen must not overwrite an existing keyfile");

    // 2. Encode the program with that keyfile -> ciphertext.
    let (ok, encoded) = run(
        &[
            "encode",
            "--keyfile",
            keyfile.to_str().unwrap(),
            prog.to_str().unwrap(),
        ],
        &dir,
    );
    assert!(ok, "encode should succeed");
    assert!(encoded.starts_with("CNVL1:"), "output should be encoded");
    let enc = dir.join("prog.enc");
    fs::write(&enc, &encoded).unwrap();

    // 3a. Run it with the keyfile auto-discovered from the program's directory.
    let (ok, out) = run(&["run", enc.to_str().unwrap()], &dir);
    assert!(ok, "run should succeed when the keyfile is present");
    assert_eq!(out, "42\n");

    // 3b. Run it with an explicit --keyfile too.
    let (ok, out) = run(
        &[
            "run",
            "--keyfile",
            keyfile.to_str().unwrap(),
            enc.to_str().unwrap(),
        ],
        &dir,
    );
    assert!(ok);
    assert_eq!(out, "42\n");

    // 4. Without the keyfile, decoding falls back to the wrong key and breaks --
    //    exactly the point of repo-binding.
    fs::remove_file(&keyfile).unwrap();
    let (ok, _) = run(&["run", enc.to_str().unwrap()], &dir);
    assert!(!ok, "run must fail when the keyfile is gone");

    fs::remove_dir_all(&dir).ok();
}
