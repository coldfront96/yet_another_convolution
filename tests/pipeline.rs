//! End-to-end tests for the Convolution pipeline.

use convolution::core::{Op, RuntimeError};
use convolution::zone::ZoneError;
use convolution::{compile, run_source, Segment, WildError};

#[test]
fn arithmetic_prints_expected() {
    let out = run_source("{stack 2 3 + dup * . }").unwrap();
    assert_eq!(out, "25\n");
}

#[test]
fn hello_emits_characters() {
    let out = run_source("{stack 72 , 73 , 33 , }").unwrap();
    assert_eq!(out, "HI!");
}

#[test]
fn comments_are_ignored() {
    let out = run_source("{stack 1 2 + . # adds to 3\n}").unwrap();
    assert_eq!(out, "3\n");
}

#[test]
fn zones_share_one_stack_in_order() {
    // First zone leaves 5; second zone adds 1 and prints 6.
    let out = run_source("{stack 5} {stack 1 + .}").unwrap();
    assert_eq!(out, "6\n");
}

#[test]
fn stack_shuffles() {
    // over: a b -> a b a ; here 3 4 over -> 3 4 3, leaving 3 on top
    let program = compile("{stack 3 4 over}").unwrap();
    assert_eq!(
        program,
        vec![Segment::Ops(vec![Op::Push(3), Op::Push(4), Op::Over])]
    );
}

// --- grid dialect -------------------------------------------------------

#[test]
fn grid_string_mode_prints_in_order() {
    // Push 'I' then 'H'; two commas pop H then I -> "HI".
    let out = run_source("{grid\n>\"IH\",,@\n}").unwrap();
    assert_eq!(out, "HI");
}

#[test]
fn grid_hello_world() {
    let out = run_source("{grid\n>\"!dlroW ,olleH\",,,,,,,,,,,,,@\n}").unwrap();
    assert_eq!(out, "Hello, World!");
}

#[test]
fn grid_direction_change_does_arithmetic() {
    // Walk right computing 3*4, turn down at `v`, print -> "12 ".
    let out = run_source("{grid\n>34*v\n    .\n    @\n}").unwrap();
    assert_eq!(out, "12 ");
}

#[test]
fn grid_self_modification_with_p_and_g() {
    // Put char 9 into cell (0,0), then read it back and print -> "9 ".
    let out = run_source("{grid\n>900p00g.@\n}").unwrap();
    assert_eq!(out, "9 ");
}

#[test]
fn grid_get_reads_own_source() {
    // `g` of (0,0) reads the '>' cell, whose code is 62.
    let out = run_source("{grid\n>00g.@\n}").unwrap();
    assert_eq!(out, "62 ");
}

#[test]
fn grid_and_stack_share_one_stack() {
    // A stack zone leaves 5; a following grid zone adds 4 and prints -> "9 ".
    let out = run_source("{stack 5} {grid\n>4+.@\n}").unwrap();
    assert_eq!(out, "9 ");
}

#[test]
fn grid_runaway_hits_step_limit() {
    // No `@`: the IP loops forever and must fail loudly, not hang.
    let err = run_source("{grid\n><\n}").unwrap_err();
    assert_eq!(err, WildError::Runtime(RuntimeError::StepLimit));
}

#[test]
fn underflow_is_reported() {
    let err = run_source("{stack +}").unwrap_err();
    assert_eq!(err, WildError::Runtime(RuntimeError::StackUnderflow("+")));
}

#[test]
fn divide_by_zero_is_reported() {
    let err = run_source("{stack 1 0 /}").unwrap_err();
    assert_eq!(err, WildError::Runtime(RuntimeError::DivideByZero));
}

#[test]
fn unknown_word_is_reported() {
    let err = run_source("{stack frobnicate}").unwrap_err();
    match err {
        WildError::Stack(_) => {}
        other => panic!("expected a stack error, got {other:?}"),
    }
}

#[test]
fn unknown_dialect_is_reported() {
    let err = run_source("{prose once upon a time}").unwrap_err();
    match err {
        WildError::UnknownDialect { kind, .. } => assert_eq!(kind, "prose"),
        other => panic!("expected unknown dialect, got {other:?}"),
    }
}

#[test]
fn unterminated_zone_is_reported() {
    let err = run_source("{stack 1 2 +").unwrap_err();
    assert_eq!(err, WildError::Zone(ZoneError::Unterminated { line: 1 }));
}

#[test]
fn negative_literals_push() {
    let out = run_source("{stack -5 -3 + .}").unwrap();
    assert_eq!(out, "-8\n");
}
