// File: src/interpreter/control_flow.rs
//
// Control flow signals for loop statements and early returns.
//
// The interpreter uses ControlFlow to manage break/continue statements
// within loops (for, while). This allows the interpreter to signal
// when execution should exit a loop (Break) or skip to the next iteration
// (Continue) without using exceptions.

/// Control flow signals for loop execution
///
/// Used by the interpreter to handle break and continue statements within loops.
/// The interpreter checks this state after evaluating each statement in a loop body.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ControlFlow {
    /// Normal execution, continue to next statement
    None,
    /// Break statement encountered, exit the innermost loop
    Break,
    /// Continue statement encountered, skip to next loop iteration
    Continue,
}
