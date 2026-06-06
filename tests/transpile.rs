//! Equivalence tests for the transpiler backend.
//!
//! For each program we run the interpreter, transpile the same source to Python,
//! execute that Python, and assert the two outputs are identical. This is what
//! keeps the embedded Python grid runtime faithful to the Rust one.

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use convolution::{run_program, transpile_program};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Run `code` through `python3`, returning its stdout.
fn run_python(code: &str) -> String {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("cnv_{}_{}.py", std::process::id(), id));
    std::fs::write(&path, code).expect("write temp python");

    let output = Command::new("python3")
        .arg(&path)
        .stdin(Stdio::null())
        .output()
        .expect("spawn python3");
    std::fs::remove_file(&path).ok();

    assert!(
        output.status.success(),
        "python exited with failure:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("python stdout is utf-8")
}

/// Assert the interpreter and the transpiled-then-executed program agree.
fn assert_equivalent(source: &str) {
    let key = "convolution";
    let interpreted = run_program(source, key).expect("interpreter run");
    let python = transpile_program(source, key).expect("transpile");
    let executed = run_python(&python);
    assert_eq!(
        executed, interpreted,
        "transpiled output differs from interpreter for:\n{source}\n\n--- python ---\n{python}"
    );
}

#[test]
fn transpiled_examples_match_interpreter() {
    let examples = [
        "hello",
        "arithmetic",
        "hello_grid",
        "grid2d",
        "selfmod",
        "prose",
        "prose_hello",
        "lambda",
        "lambda_hello",
    ];
    for name in examples {
        let source =
            std::fs::read_to_string(format!("examples/{name}.wild")).expect("read example");
        assert_equivalent(&source);
    }
}

#[test]
fn transpiled_encoded_program_matches_interpreter() {
    // The encoded file is decoded (default key) before transpiling.
    let source = std::fs::read_to_string("examples/secret.wild.enc").expect("read encoded example");
    assert_equivalent(&source);
}

#[test]
fn transpiled_mixed_dialect_program_matches() {
    // A single program that uses every dialect and shares one stack across them.
    let source = "\
{stack 2 3 + .}
{lambda (print (* 6 7))}
{prose Take seventy two and say it.}
{grid\n>\"!IH\",,,@\n}
";
    assert_equivalent(source);
}

#[test]
fn transpiled_cross_zone_stack_sharing_matches() {
    // lambda leaves a value; a grid zone consumes it and prints with `.`.
    let source = "{lambda (+ 40 2)} {grid\n>.@\n}";
    assert_equivalent(source);
}

/// A program that hits a runtime error transpiles to Python that fails the same
/// way (non-zero exit), so `run_python`'s success assertion would catch a
/// divergence; here we just confirm the interpreter agrees it's an error.
#[test]
fn division_by_zero_is_an_error_in_both() {
    let source = "{stack 1 0 /}";
    assert!(run_program(source, "k").is_err());
    let python = transpile_program(source, "k").unwrap();
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("cnv_err_{}_{}.py", std::process::id(), id));
    std::fs::write(&path, &python).unwrap();
    let output = Command::new("python3")
        .arg(&path)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    std::fs::remove_file(&path).ok();
    assert!(!output.status.success(), "expected python to fail");
}

/// Every self-modifying surface has no faithful static Python form, so the
/// transpiler must refuse it (rather than emit code that diverges at runtime).
fn assert_refused(source: &str) {
    match transpile_program(source, "k") {
        Err(convolution::WildError::Untranspilable(_)) => {}
        other => panic!("expected Untranspilable for {source:?}, got {other:?}"),
    }
}

#[test]
fn transpiler_refuses_self_rewriting_programs() {
    // The interpreter handles poke fine...
    assert_eq!(
        run_program("{stack 1 5 42 poke}{stack 6 7 + .}", "k").unwrap(),
        "42\n"
    );
    // ...but every cross-zone surface is an honest, explicit transpile error.
    assert_refused("{stack 1 5 42 poke}{stack 6 7 + .}"); // poke
    assert_refused("{stack 1 1 peek .}{stack 7 .}"); // peek
    assert_refused("{stack 2 warp}{stack 9 .}{stack 5 .}"); // warp
    assert_refused("{lambda (warp 9)}{stack 5 .}"); // warp via lambda
    assert_refused("{grid\n1567*=@\n}{stack 6 7 + .}"); // grid cross-zone `=`
}
