//! Dialects. Each submodule is one borrowed language that drives the shared VM.
//! Linear dialects (`stack`) compile to core ops; richer dialects (`grid`) run
//! their own interpreter. New gimmicks (`prose`, `lambda`, ...) get added here
//! as siblings.

pub mod grid;
pub mod prose;
pub mod stack;
