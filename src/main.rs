//! The `convolution` CLI: run a `.wild` file.
//!
//! Usage:
//!   convolution <path-to-file.wild>

use std::process::ExitCode;
use std::{env, fs};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: convolution <file.wild>");
        return ExitCode::from(2);
    }

    let path = &args[1];
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read `{path}`: {e}");
            return ExitCode::from(2);
        }
    };

    match convolution::run_source(&source) {
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
