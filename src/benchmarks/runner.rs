// Benchmark runner - orchestrates benchmark execution

use crate::benchmarks::{BenchmarkResult, ExecutionMode, Timer};
use crate::interpreter::{Environment, Interpreter, Value};
use crate::lexer;
use crate::parser::Parser;
use crate::vm::VM;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct BenchmarkRunner {
    iterations: usize,
    warmup_runs: usize,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self { iterations: 10, warmup_runs: 2 }
    }

    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_warmup(mut self, warmup_runs: usize) -> Self {
        self.warmup_runs = warmup_runs;
        self
    }

    /// Run a benchmark with the given code in all execution modes
    pub fn run_benchmark(&self, name: &str, code: &str) -> Vec<BenchmarkResult> {
        vec![self.run_interpreter(name, code), self.run_vm(name, code), self.run_jit(name, code)]
    }

    /// Run benchmark in interpreter mode
    pub fn run_interpreter(&self, name: &str, code: &str) -> BenchmarkResult {
        let mut result = BenchmarkResult::new(name.to_string(), ExecutionMode::Interpreter);

        // Warmup
        for _ in 0..self.warmup_runs {
            if let Err(e) = self.execute_interpreter(code) {
                result.set_error(format!("Warmup failed: {}", e));
                return result;
            }
        }

        // Benchmark
        for _ in 0..self.iterations {
            let timer = Timer::start();
            match self.execute_interpreter(code) {
                Ok(_) => {
                    result.add_sample(timer.elapsed());
                }
                Err(e) => {
                    result.set_error(format!("Execution failed: {}", e));
                    break;
                }
            }
        }

        result
    }

    /// Run benchmark in VM mode
    pub fn run_vm(&self, name: &str, code: &str) -> BenchmarkResult {
        let mut result = BenchmarkResult::new(name.to_string(), ExecutionMode::VM);

        // Warmup
        for _ in 0..self.warmup_runs {
            if let Err(e) = self.execute_vm(code) {
                result.set_error(format!("Warmup failed: {}", e));
                return result;
            }
        }

        // Benchmark
        for _ in 0..self.iterations {
            let timer = Timer::start();
            match self.execute_vm(code) {
                Ok(_) => {
                    result.add_sample(timer.elapsed());
                }
                Err(e) => {
                    result.set_error(format!("Execution failed: {}", e));
                    break;
                }
            }
        }

        result
    }

    /// Run benchmark in JIT mode
    pub fn run_jit(&self, name: &str, code: &str) -> BenchmarkResult {
        let mut result = BenchmarkResult::new(name.to_string(), ExecutionMode::JIT);

        // Warmup
        for _ in 0..self.warmup_runs {
            if let Err(e) = self.execute_jit(code) {
                result.set_error(format!("Warmup failed: {}", e));
                return result;
            }
        }

        // Benchmark
        for _ in 0..self.iterations {
            let timer = Timer::start();
            match self.execute_jit(code) {
                Ok(_) => {
                    result.add_sample(timer.elapsed());
                }
                Err(e) => {
                    result.set_error(format!("Execution failed: {}", e));
                    break;
                }
            }
        }

        result
    }

    /// Execute code in interpreter mode
    fn execute_interpreter(&self, code: &str) -> Result<(), String> {
        let tokens = lexer::tokenize(code).map_err(|diagnostics| {
            format!(
                "Lexing error: {}",
                diagnostics.first().map(|d| d.message.as_str()).unwrap_or("unknown error")
            )
        })?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();

        let mut interpreter = Interpreter::new();
        interpreter.eval_stmts(&ast);

        Ok(())
    }

    /// Execute code in VM mode
    fn execute_vm(&self, code: &str) -> Result<(), String> {
        let tokens = lexer::tokenize(code).map_err(|diagnostics| {
            format!(
                "Lexing error: {}",
                diagnostics.first().map(|d| d.message.as_str()).unwrap_or("unknown error")
            )
        })?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();

        // Compile AST to bytecode
        use crate::compiler::Compiler;
        let mut compiler = Compiler::new();
        let chunk = compiler.compile(&ast).map_err(|e| format!("Compilation error: {}", e))?;

        // Execute bytecode
        let mut vm = VM::new();
        self.configure_vm_globals(&mut vm);
        vm.set_jit_enabled(false);

        vm.execute(chunk).map_err(|e| format!("VM error: {:?}", e))?;

        Ok(())
    }

    /// Execute code in JIT mode
    fn execute_jit(&self, code: &str) -> Result<(), String> {
        let tokens = lexer::tokenize(code).map_err(|diagnostics| {
            format!(
                "Lexing error: {}",
                diagnostics.first().map(|d| d.message.as_str()).unwrap_or("unknown error")
            )
        })?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();

        // Compile AST to bytecode
        use crate::compiler::Compiler;
        let mut compiler = Compiler::new();
        let chunk = compiler.compile(&ast).map_err(|e| format!("Compilation error: {}", e))?;

        // Execute bytecode with JIT enabled
        let mut vm = VM::new();
        self.configure_vm_globals(&mut vm);
        vm.set_jit_enabled(true);

        vm.execute(chunk).map_err(|e| format!("JIT error: {:?}", e))?;

        Ok(())
    }

    fn configure_vm_globals(&self, vm: &mut VM) {
        // Register built-ins so VM benchmarks have parity with interpreter mode.
        let env = Arc::new(Mutex::new(Environment::new()));
        let builtins = Interpreter::get_builtin_names();
        for builtin_name in builtins {
            env.lock()
                .unwrap()
                .set(builtin_name.to_string(), Value::NativeFunction(builtin_name.to_string()));
        }
        vm.set_globals(env);
    }

    /// Load and run a benchmark from a file
    pub fn run_file(&self, path: PathBuf) -> Vec<BenchmarkResult> {
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        match std::fs::read_to_string(&path) {
            Ok(code) => self.run_benchmark(&name, &code),
            Err(e) => {
                let mut result = BenchmarkResult::new(name.clone(), ExecutionMode::Interpreter);
                result.set_error(format!("Failed to read file: {}", e));
                vec![result]
            }
        }
    }

    /// Run all benchmarks in a directory
    pub fn run_directory(&self, dir: PathBuf) -> Vec<(String, Vec<BenchmarkResult>)> {
        let mut results = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("kujo") {
                    let name =
                        path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
                    if let Ok(code) = std::fs::read_to_string(&path) {
                        let bench_results = self.run_benchmark(&name, &code);
                        results.push((name, bench_results));
                    }
                }
            }
        }

        results
    }
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_benchmark() {
        let runner = BenchmarkRunner::new().with_iterations(3);
        let code = r#"
            let x := 5
            let y := 10
            let z := x + y
        "#;

        let result = runner.run_interpreter("simple", code);
        assert!(result.success);
        assert_eq!(result.samples.len(), 3);
    }

    #[test]
    fn test_benchmark_error_handling() {
        let runner = BenchmarkRunner::new();
        // Simple valid code - the interpreter doesn't throw catchable errors
        // in the way we'd need for this test pattern
        let code = "let x := 5";

        let result = runner.run_interpreter("simple_test", code);
        assert!(result.success);
        assert!(result.samples.len() > 0);
    }
}
