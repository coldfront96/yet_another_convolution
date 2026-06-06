//! The `convolution` CLI.
//!
//! Usage:
//!   convolution <file.wild>           run a program (auto-decodes if encoded)
//!   convolution run <file.wild>       same as above
//!   convolution encode <file.wild>    print the encoded form to stdout
//!   convolution decode <file>         print the decoded source to stdout
//!   convolution transpile <file>      print an equivalent Python program
//!   convolution keygen                create a random .wildkey keyfile
//!
//! Key resolution (highest precedence first):
//!   --key <KEY>  |  --keyfile <PATH>  |  $CONVOLUTION_KEY  |  a discovered
//!   `.wildkey` (walking up from the program's directory)  |  the built-in
//!   default.
//!
//! Generate a keyfile with `keygen`, keep it private (it's gitignored), and
//! carry it between your own repos: encoded programs only run where it's found.

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;
use std::{env, fs};

use convolution::{cipher, discover_keyfile, run_program, transpile_program, KEYFILE_NAME};

/// Last-resort key when nothing else is configured. Override it for real
/// secrecy (the shipped example and tests rely on this default).
const DEFAULT_KEY: &str = "convolution";

fn main() -> ExitCode {
    let raw: Vec<String> = env::args().skip(1).collect();

    let mut key_flag: Option<String> = None;
    let mut keyfile_flag: Option<String> = None;
    let mut positional: Vec<String> = Vec::new();
    let mut args = raw.into_iter();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--key" | "-k" => match args.next() {
                Some(k) => key_flag = Some(k),
                None => return fail("`--key` needs a value"),
            },
            "--keyfile" => match args.next() {
                Some(p) => keyfile_flag = Some(p),
                None => return fail("`--keyfile` needs a value"),
            },
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            _ => positional.push(a),
        }
    }

    // `keygen` takes no program file; handle it before the file commands.
    if matches!(positional.as_slice(), [cmd] if cmd == "keygen") {
        let target = keyfile_flag.unwrap_or_else(|| KEYFILE_NAME.to_string());
        return keygen(Path::new(&target));
    }

    let (command, path) = match positional.as_slice() {
        [file] => ("run", file.as_str()),
        [cmd, file] if matches!(cmd.as_str(), "run" | "encode" | "decode" | "transpile") => {
            (cmd.as_str(), file.as_str())
        }
        _ => {
            print_usage();
            return ExitCode::from(2);
        }
    };

    let key = match resolve_key(Path::new(path), key_flag, keyfile_flag) {
        Ok(k) => k,
        Err(e) => return fail(&e),
    };

    let contents = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return fail(&format!("cannot read `{path}`: {e}")),
    };

    match command {
        "run" => emit_or_fail(run_program(&contents, &key)),
        "encode" => {
            println!("{}", cipher::encode(&contents, &key));
            ExitCode::SUCCESS
        }
        "decode" => emit_or_fail(cipher::decode(&contents, &key).map_err(|e| e.to_string())),
        "transpile" => emit_or_fail(transpile_program(&contents, &key)),
        _ => unreachable!("command was validated above"),
    }
}

/// Print a successful result, or its error to stderr with a failure exit code.
fn emit_or_fail<E: std::fmt::Display>(result: Result<String, E>) -> ExitCode {
    match result {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(2)
}

/// Resolve the encryption key for an operation on `target`.
fn resolve_key(
    target: &Path,
    key_flag: Option<String>,
    keyfile_flag: Option<String>,
) -> Result<String, String> {
    if let Some(k) = key_flag {
        return Ok(k);
    }
    if let Some(p) = keyfile_flag {
        return read_keyfile(Path::new(&p));
    }
    if let Ok(k) = env::var("CONVOLUTION_KEY") {
        return Ok(k);
    }
    let start = target.parent().unwrap_or_else(|| Path::new("."));
    let start = if start.as_os_str().is_empty() {
        Path::new(".")
    } else {
        start
    };
    if let Some(found) = discover_keyfile(start) {
        return read_keyfile(&found);
    }
    Ok(DEFAULT_KEY.to_string())
}

fn read_keyfile(path: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("cannot read keyfile `{}`: {e}", path.display()))
}

/// Create a new random keyfile, refusing to clobber an existing one.
fn keygen(target: &Path) -> ExitCode {
    if target.exists() {
        return fail(&format!(
            "`{}` already exists; refusing to overwrite it",
            target.display()
        ));
    }
    let key = match random_key() {
        Ok(k) => k,
        Err(e) => return fail(&format!("could not gather randomness: {e}")),
    };
    if let Err(e) = fs::write(target, format!("{key}\n")) {
        return fail(&format!("cannot write `{}`: {e}", target.display()));
    }
    eprintln!(
        "wrote a new keyfile to `{}`.\n\
         keep it private -- it's gitignored by default. Carry it between your own\n\
         repos to run encoded programs; without it, they won't decode.",
        target.display()
    );
    ExitCode::SUCCESS
}

/// 64 hex chars from 32 bytes of OS randomness.
fn random_key() -> std::io::Result<String> {
    let mut buf = [0u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut buf)?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

fn print_usage() {
    eprintln!(
        "convolution -- run a deliberately convoluted esolang\n\
         \n\
         usage:\n\
         \x20 convolution <file.wild>          run (auto-decodes if encoded)\n\
         \x20 convolution run <file.wild>      run explicitly\n\
         \x20 convolution encode <file.wild>   print the encoded form\n\
         \x20 convolution decode <file>        print the decoded source\n\
         \x20 convolution transpile <file>     print an equivalent Python program\n\
         \x20 convolution keygen               create a random .wildkey keyfile\n\
         \n\
         options:\n\
         \x20 -k, --key <KEY>     key for the encoded layer\n\
         \x20 --keyfile <PATH>    read the key from a file\n\
         \x20                     (or set $CONVOLUTION_KEY, or place a .wildkey file)"
    );
}
