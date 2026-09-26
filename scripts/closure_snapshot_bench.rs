//! Standalone VM measurement harness; compile against a selected libkujo rlib.
//! Kujo workloads are identical across revisions. Parsing/compilation and VM
//! construction are excluded; report medians, never use wall time as a CI gate.
use kujo::compiler::Compiler;
use kujo::interpreter::{Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use kujo::vm::VM;
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn main() {
    let cases = [
        ("ordinary_locals", "func work() { mut n := 0; mut total := 0; while n < 20000 { total += n; n += 1 }; return total }; bench_result := work()", 199990000),
        ("ordinary_calls", "func work(n) { return n + 1 }; mut i := 0; mut total := 0; while i < 2000 { total += work(i); i += 1 }; bench_result := total", 2001000),
        ("closure_creation", "func factory(n) { return func() { return n } }; mut i := 0; mut total := 0; while i < 2000 { let f := factory(i); total += f(); i += 1 }; bench_result := total", 1999000),
        ("captured_reads", "func factory(n) { return func() { return n } }; let f := factory(7); mut i := 0; mut total := 0; while i < 2000 { total += f(); i += 1 }; bench_result := total", 14000),
        ("captured_writes", "func factory() { mut n := 0; return func() { n += 1; return n } }; let f := factory(); mut i := 0; mut total := 0; while i < 2000 { total += f(); i += 1 }; bench_result := total", 2001000),
        ("nested_creation", "func factory(n) { return func() { return func() { return n } } }; mut i := 0; mut total := 0; while i < 1000 { let middle := factory(i); let f := middle(); total += f(); i += 1 }; bench_result := total", 499500),
    ];
    for (name, source, expected) in cases {
        let mut parser = Parser::new(tokenize(source).unwrap());
        let parsed = parser.parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let chunk = Compiler::new().compile(&parsed.stmts).unwrap();
        let mut samples = Vec::new();
        for sample in 0..8 {
            let globals = Arc::new(Mutex::new(Interpreter::new().env));
            let mut vm = VM::new();
            vm.set_globals(Arc::clone(&globals));
            let input = chunk.clone();
            let started = Instant::now();
            vm.execute(input).unwrap();
            let elapsed = started.elapsed().as_nanos();
            assert!(
                matches!(globals.lock().unwrap().get("bench_result"), Some(Value::Int(actual)) if actual == expected)
            );
            if sample > 0 {
                samples.push(elapsed);
            }
        }
        samples.sort_unstable();
        println!("{name},{}", samples[samples.len() / 2]);
    }
}
