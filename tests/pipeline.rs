//! End-to-end tests for the Convolution pipeline.

use convolution::cipher::{self, CipherError};
use convolution::core::{Op, RuntimeError};
use convolution::zone::ZoneError;
use convolution::zones::lambda::LambdaError;
use convolution::{compile, run_program, run_source, Segment, WildError};

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

// --- encoded outer layer ------------------------------------------------

#[test]
fn cipher_round_trips() {
    let source = "{stack 2 3 + dup * . }";
    let encoded = cipher::encode(source, "hunter2");
    assert!(cipher::looks_encoded(&encoded));
    assert_eq!(cipher::decode(&encoded, "hunter2").unwrap(), source);
}

#[test]
fn cipher_wrong_key_is_rejected() {
    let encoded = cipher::encode("{stack 1 .}", "correct-key");
    assert_eq!(
        cipher::decode(&encoded, "WRONG"),
        Err(CipherError::WrongKey)
    );
}

#[test]
fn cipher_missing_magic_is_rejected() {
    assert_eq!(
        cipher::decode("just some plaintext", "k"),
        Err(CipherError::MissingMagic)
    );
}

#[test]
fn run_program_executes_encoded_source() {
    let encoded = cipher::encode("{stack 6 7 * .}", "s3cret");
    let out = run_program(&encoded, "s3cret").unwrap();
    assert_eq!(out, "42\n");
}

#[test]
fn run_program_runs_plaintext_unchanged() {
    // No magic marker -> treated as plain source regardless of key.
    let out = run_program("{stack 9 .}", "irrelevant").unwrap();
    assert_eq!(out, "9\n");
}

#[test]
fn run_program_wrong_key_surfaces_cipher_error() {
    let encoded = cipher::encode("{stack 1 .}", "real");
    let err = run_program(&encoded, "fake").unwrap_err();
    assert_eq!(err, WildError::Cipher(CipherError::WrongKey));
}

// --- prose dialect ------------------------------------------------------

#[test]
fn prose_reads_like_english() {
    let out =
        run_source("{prose Take six and seven and multiply them, then show the answer.}").unwrap();
    assert_eq!(out, "42\n");
}

#[test]
fn prose_says_characters() {
    let out = run_source("{prose Take seventy two and say it. Take seventy three and say it.}")
        .unwrap();
    assert_eq!(out, "HI");
}

#[test]
fn prose_parses_compound_numbers() {
    let out = run_source("{prose Take one hundred twenty three and show it.}").unwrap();
    assert_eq!(out, "123\n");
}

#[test]
fn prose_parses_hyphenated_numbers() {
    let out = run_source("{prose Take forty-two and show it.}").unwrap();
    assert_eq!(out, "42\n");
}

#[test]
fn prose_parses_negative_numbers() {
    let out = run_source("{prose Take negative five and show it.}").unwrap();
    assert_eq!(out, "-5\n");
}

#[test]
fn prose_filler_only_is_silent() {
    let out = run_source("{prose Once upon a time, nothing in particular happened.}").unwrap();
    assert_eq!(out, "");
}

#[test]
fn prose_compound_program() {
    let out =
        run_source("{prose Take five and four, add them, duplicate the sum, multiply, and show it.}")
            .unwrap();
    assert_eq!(out, "81\n");
}

#[test]
fn prose_shares_stack_with_other_dialects() {
    // A stack zone leaves 40; the prose zone adds 2 and shows -> 42.
    let out = run_source("{stack 40} {prose Take two and add them and show it.}").unwrap();
    assert_eq!(out, "42\n");
}

// --- lambda dialect -----------------------------------------------------

#[test]
fn lambda_nested_arithmetic() {
    let out = run_source("{lambda (print (* (+ 1 2) (- 10 3)))}").unwrap();
    assert_eq!(out, "21\n");
}

#[test]
fn lambda_emits_characters() {
    let out = run_source("{lambda (emit 72) (emit 73)}").unwrap();
    assert_eq!(out, "HI");
}

#[test]
fn lambda_unary_minus_negates() {
    let out = run_source("{lambda (print (- 5))}").unwrap();
    assert_eq!(out, "-5\n");
}

#[test]
fn lambda_variadic_sum() {
    let out = run_source("{lambda (print (+ 1 2 3 4))}").unwrap();
    assert_eq!(out, "10\n");
}

#[test]
fn lambda_empty_sum_is_zero() {
    let out = run_source("{lambda (print (+))}").unwrap();
    assert_eq!(out, "0\n");
}

#[test]
fn lambda_comments_are_ignored() {
    let out = run_source("{lambda (print 7) ; this is ignored\n}").unwrap();
    assert_eq!(out, "7\n");
}

#[test]
fn lambda_leaves_value_for_next_zone() {
    // A lambda zone computes 42 without printing; a stack zone then prints it.
    let out = run_source("{lambda (+ 40 2)} {stack .}").unwrap();
    assert_eq!(out, "42\n");
}

#[test]
fn lambda_unclosed_paren_is_reported() {
    let err = run_source("{lambda (+ 1 2}").unwrap_err();
    assert_eq!(err, WildError::Lambda(LambdaError::UnclosedParen));
}

#[test]
fn lambda_unexpected_close_is_reported() {
    let err = run_source("{lambda )}").unwrap_err();
    assert_eq!(err, WildError::Lambda(LambdaError::UnexpectedClose));
}

#[test]
fn lambda_empty_application_is_reported() {
    let err = run_source("{lambda ()}").unwrap_err();
    assert_eq!(err, WildError::Lambda(LambdaError::EmptyApplication));
}

#[test]
fn lambda_not_callable_is_reported() {
    let err = run_source("{lambda (1 2)}").unwrap_err();
    assert_eq!(err, WildError::Lambda(LambdaError::NotCallable));
}

#[test]
fn lambda_bare_symbol_is_reported() {
    let err = run_source("{lambda +}").unwrap_err();
    assert_eq!(
        err,
        WildError::Lambda(LambdaError::BareSymbol("+".to_string()))
    );
}

#[test]
fn lambda_unknown_operator_is_reported() {
    let err = run_source("{lambda (frobnicate 1 2)}").unwrap_err();
    assert_eq!(
        err,
        WildError::Lambda(LambdaError::UnknownOperator("frobnicate".to_string()))
    );
}

#[test]
fn lambda_bad_arity_is_reported() {
    let err = run_source("{lambda (print 1 2)}").unwrap_err();
    assert_eq!(
        err,
        WildError::Lambda(LambdaError::BadArity("print".to_string()))
    );
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
    let err = run_source("{haiku an old silent pond}").unwrap_err();
    match err {
        WildError::UnknownDialect { kind, .. } => assert_eq!(kind, "haiku"),
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
