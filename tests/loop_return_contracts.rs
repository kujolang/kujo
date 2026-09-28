//! Permanent seeds for optimizer control-flow and observable VM/interpreter parity.
use kujo::bytecode::{BytecodeChunk, Constant, OpCode};
use kujo::compiler::Compiler;
use kujo::interpreter::{Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::optimizer::Optimizer;
use kujo::parser::Parser;
use kujo::vm::VM;
use std::process::Command;
use std::sync::{Arc, Mutex};

#[test]
fn minimized_loop_return_programs_match_interpreter() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (name, expected) in [
        ("A_no_return", "1\n2\n"),
        ("B_unconditional", "1\n9\n"),
        ("C_matching", "1\n"),
        ("D_no_match", "9\n"),
        ("E_later_match", "2\n"),
        ("F_nested_if", "2\n9\n"),
        ("G_nested_loop", "4\n9\n"),
        ("H_caller", "11\n"),
        ("I_fold", "5\n9\n"),
        ("J_break_continue", "2\n"),
        ("historical", "{\"ok\":true}\n"),
    ] {
        for mode in ["vm", "interpreter"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.arg("run");
            if mode == "interpreter" {
                command.arg("--interpreter");
            }
            let result = command
                .arg(root.join(format!("tests/fixtures/loop_return/{name}.kujo")))
                .output()
                .unwrap();
            assert!(result.status.success(), "{name}/{mode}: {:?}", result);
            assert_eq!(String::from_utf8(result.stdout).unwrap(), expected, "{name}/{mode}");
        }
    }
}

fn execute(chunk: BytecodeChunk) -> Result<Value, String> {
    let mut vm = VM::new();
    vm.set_globals(Arc::new(Mutex::new(Interpreter::new().env)));
    vm.execute(chunk)
}

fn dump(chunk: &BytecodeChunk, stage: &str) {
    println!("{stage} {:?}", chunk.name);
    for (i, op) in chunk.instructions.iter().enumerate() {
        println!("{i}: {op:?}");
    }
    for c in &chunk.constants {
        if let Constant::Function(f) = c {
            dump(f, stage);
        }
    }
}

#[test]
fn compiler_before_and_after_optimization() {
    let mut failures = Vec::new();
    for entry in
        std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/loop_return"))
            .unwrap()
    {
        let path = entry.unwrap().path();
        let source = std::fs::read_to_string(&path).unwrap();
        let parsed = Parser::new(tokenize(&source).unwrap()).parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty());
        let pre = Compiler::new().compile_with_optimization(&parsed.stmts, false).unwrap();
        let mut post = pre.clone();
        Optimizer::new().optimize(&mut post);
        println!("CASE {}", path.display());
        dump(&pre, "PRE");
        dump(&post, "POST");
        // Lowering must already be executable; optimization must retain that behavior.
        let before = execute(pre);
        let after = execute(post);
        println!("EXECUTION before={before:?} after={after:?}");
        if before.is_err() || after.is_err() {
            failures.push(path);
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn iterator_exhaustion_is_a_reachable_successor() {
    let mut chunk = BytecodeChunk::new();
    chunk.constants = vec![Constant::Int(7), Constant::Int(9)];
    chunk.instructions = vec![
        OpCode::ForNext(4),
        OpCode::LoadConst(0),
        OpCode::Return,
        OpCode::Nop,
        OpCode::LoadConst(1),
        OpCode::Return,
    ];
    Optimizer::new().optimize(&mut chunk);
    let OpCode::ForNext(target) = chunk.instructions[0] else { panic!() };
    assert_eq!(chunk.instructions[target], OpCode::LoadConst(1));
    assert!(target < 4, "dead instruction must be removed and exit remapped");
}

#[test]
fn folding_and_peephole_remap_edges_and_preserve_entry_points() {
    for ops in [
        vec![
            OpCode::LoadConst(0),
            OpCode::LoadConst(1),
            OpCode::Add,
            OpCode::Pop,
            OpCode::Jump(6),
            OpCode::Nop,
            OpCode::LoadConst(1),
            OpCode::Return,
        ],
        vec![
            OpCode::LoadConst(0),
            OpCode::Pop,
            OpCode::Jump(4),
            OpCode::Nop,
            OpCode::LoadConst(1),
            OpCode::Return,
        ],
        // Folding across an alternate entry to the second operand changes its stack contract.
        vec![
            OpCode::LoadConst(1),
            OpCode::LoadConst(0),
            OpCode::JumpIfTrue(5),
            OpCode::LoadConst(0),
            OpCode::LoadConst(1),
            OpCode::LoadConst(1),
            OpCode::Add,
            OpCode::Return,
        ],
    ] {
        let mut chunk = BytecodeChunk::new();
        chunk.constants = vec![Constant::Int(1), Constant::Int(9)];
        chunk.instructions = ops;
        let expected = VM::new().execute(chunk.clone()).unwrap();
        Optimizer::new().optimize(&mut chunk);
        let actual = VM::new().execute(chunk).unwrap();
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
    }
}

#[test]
fn invalid_return_still_errors() {
    let mut chunk = BytecodeChunk::new();
    chunk.instructions.push(OpCode::Return);
    assert!(VM::new().execute(chunk).unwrap_err().contains("Stack underflow in return"));
}

#[test]
fn every_address_operand_and_metadata_relocates() {
    for branch in [
        OpCode::Jump(6),
        OpCode::JumpBack(6),
        OpCode::JumpIfFalse(6),
        OpCode::JumpIfTrue(6),
        OpCode::ForNext(6),
        OpCode::BeginTry(6),
    ] {
        let mut chunk = BytecodeChunk::new();
        chunk.constants = vec![Constant::Int(1), Constant::Int(2)];
        chunk.instructions = vec![
            OpCode::LoadConst(0),
            OpCode::LoadConst(1),
            OpCode::Add,
            OpCode::Pop,
            branch,
            OpCode::ReturnNone,
            OpCode::LoadConst(1),
            OpCode::Return,
        ];
        chunk.source_map.insert(6, (77, 4));
        chunk.exception_handlers.push(kujo::bytecode::ExceptionHandler {
            try_start: 4,
            try_end: 6,
            catch_start: 6,
            exception_var: "error".into(),
        });
        Optimizer::new().optimize(&mut chunk);
        let target = match chunk.instructions[0] {
            OpCode::Jump(t)
            | OpCode::JumpBack(t)
            | OpCode::JumpIfFalse(t)
            | OpCode::JumpIfTrue(t)
            | OpCode::ForNext(t)
            | OpCode::BeginTry(t) => t,
            ref other => panic!("expected relocated branch, got {other:?}"),
        };
        assert_eq!(chunk.instructions[target], OpCode::LoadConst(1));
        assert_eq!(chunk.source_map.get(&target), Some(&(77, 4)));
        assert_eq!(chunk.exception_handlers[0].try_start, 0);
        assert_eq!(chunk.exception_handlers[0].try_end, target);
        assert_eq!(chunk.exception_handlers[0].catch_start, target);
    }
}
