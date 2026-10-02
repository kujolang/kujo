//! Lean, trusted launcher for latency-sensitive Kujo VM/JIT programs.
//!
//! This deliberately avoids the general-purpose `kujo` command router and its
//! broad command surface. The language parser, compiler, VM, builtins, imports,
//! async scheduler, and optional regional JIT are the same library components
//! used by `kujo run`.

use kujo::compiler::Compiler;
use kujo::interpreter::{Environment, Value};
use kujo::parser::Parser;
use kujo::vm::{VmExecutionResult, VM};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const EXECUTION_STACK_SIZE: usize = 64 * 1024 * 1024;
const USAGE: &str = "Usage: kujo-run [--jit] FILE";

struct RunArgs {
    file: PathBuf,
    jit: bool,
}

fn parse_args() -> Result<Option<RunArgs>, String> {
    let mut args = std::env::args_os().skip(1);
    let Some(first) = args.next() else {
        return Err(USAGE.to_string());
    };

    if first == OsStr::new("--help") || first == OsStr::new("-h") {
        println!("{USAGE}");
        println!();
        println!("Run a trusted Kujo program with the low-latency VM launcher.");
        return Ok(None);
    }
    if first == OsStr::new("--version") || first == OsStr::new("-V") {
        println!("kujo-run {}", env!("CARGO_PKG_VERSION"));
        return Ok(None);
    }

    let (jit, file) = if first == OsStr::new("--jit") {
        let file = args.next().ok_or_else(|| USAGE.to_string())?;
        (true, PathBuf::from(file))
    } else {
        (false, PathBuf::from(first))
    };
    if args.next().is_some() {
        return Err(USAGE.to_string());
    }

    Ok(Some(RunArgs { file, jit }))
}

fn entry_script_search_paths(entry_file: &Path) -> Vec<PathBuf> {
    let mut search_paths = Vec::with_capacity(2);
    if let Some(parent) = entry_file.parent() {
        let normalized_parent = if parent.as_os_str().is_empty() { Path::new(".") } else { parent };
        search_paths.push(normalized_parent.to_path_buf());
        if parent.file_name().and_then(|name| name.to_str()) == Some("src") {
            if let Some(project_root) = parent.parent() {
                search_paths.push(project_root.to_path_buf());
            }
        }
    }
    search_paths
}

fn run(args: RunArgs) -> Result<(), String> {
    let source = fs::read_to_string(&args.file)
        .map_err(|error| format!("Failed to read '{}': {error}", args.file.display()))?;
    let filename = args.file.to_string_lossy();
    let tokens = kujo::lexer::tokenize_with_file(&source, Some(&filename))
        .map_err(|diagnostics| format!("Lexing failed: {diagnostics:?}"))?;
    let mut parser = Parser::new(tokens);
    let parsed = parser.parse_with_diagnostics();
    if !parsed.diagnostics.is_empty() {
        return Err(format!("Parsing failed: {:?}", parsed.diagnostics));
    }
    let chunk = Compiler::new()
        .compile(&parsed.stmts)
        .map_err(|error| format!("Compilation failed: {error}"))?;

    let runner = std::thread::Builder::new()
        .name("kujo-runtime".to_string())
        .stack_size(EXECUTION_STACK_SIZE)
        .spawn(move || -> Result<(), String> {
            let mut vm = VM::new();
            for search_path in entry_script_search_paths(&args.file) {
                vm.add_module_search_path(search_path);
            }
            vm.set_jit_enabled(args.jit && std::env::var("DISABLE_JIT").is_err());

            let env = Arc::new(Mutex::new(Environment::new()));
            {
                let mut globals = env.lock().map_err(|error| error.to_string())?;
                for (name, value) in kujo::builtins::get_builtins() {
                    globals.set(name, value);
                }
                globals.set("null".to_string(), Value::Null);
            }
            vm.set_globals(env);

            match vm.execute_until_suspend(chunk)? {
                VmExecutionResult::Completed => Ok(()),
                VmExecutionResult::Suspended { .. } => vm.run_scheduler_until_complete_unbounded(),
            }
        })
        .map_err(|error| format!("Failed to start runtime worker: {error}"))?;

    runner.join().map_err(|_| "Runtime worker panicked".to_string())?
}

fn main() {
    let result = parse_args().and_then(|args| match args {
        Some(args) => run(args),
        None => Ok(()),
    });
    if let Err(error) = result {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
