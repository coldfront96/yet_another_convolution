//! The `convolution` CLI.
//!
//! Usage:
//!   convolution <file.wild>           run a program (auto-decodes if encoded)
//!   convolution run <file.wild>       same as above
//!   convolution encode <file.wild>    print the encoded form to stdout
//!   convolution decode <file>         print the decoded source to stdout
//!
//! Key resolution (highest precedence first):
//!   --key <KEY> / -k <KEY>  |  $CONVOLUTION_KEY  |  the built-in default
//!
//! Set your own key to make a program only your tooling (and repos) can read.

use std::process::ExitCode;
use std::{env, fs};

use convolution::{cipher, run_program};

/// Used when neither `--key` nor `$CONVOLUTION_KEY` is provided. Override it for
/// real secrecy.
const DEFAULT_KEY: &str = "convolution";

fn main() -> ExitCode {
    let raw: Vec<String> = env::args().skip(1).collect();

    // Resolve the key: --key flag wins, then the env var, then the default.
    let mut key = env::var("CONVOLUTION_KEY").unwrap_or_else(|_| DEFAULT_KEY.to_string());
    let mut positional: Vec<String> = Vec::new();
    let mut args = raw.into_iter();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--key" | "-k" => match args.next() {
                Some(k) => key = k,
                None => {
                    eprintln!("error: `--key` needs a value");
                    return ExitCode::from(2);
                }
            },
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            _ => positional.push(a),
        }
    }

    // Figure out the command and target file.
    let (command, path) = match positional.as_slice() {
        [file] => ("run", file.as_str()),
        [cmd, file] if matches!(cmd.as_str(), "run" | "encode" | "decode") => {
            (cmd.as_str(), file.as_str())
        }
        _ => {
            print_usage();
            return ExitCode::from(2);
        }
    };

    let contents = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read `{path}`: {e}");
            return ExitCode::from(2);
        }
    };

    match command {
        "run" => match run_program(&contents, &key) {
            Ok(output) => {
                print!("{output}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        "encode" => {
            println!("{}", cipher::encode(&contents, &key));
            ExitCode::SUCCESS
        }
        "decode" => match cipher::decode(&contents, &key) {
            Ok(source) => {
                print!("{source}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        _ => unreachable!("command was validated above"),
    }
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
         \n\
         options:\n\
         \x20 -k, --key <KEY>   key for the encoded layer\n\
         \x20                   (or set $CONVOLUTION_KEY)"
    );
}
