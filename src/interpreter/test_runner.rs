// File: src/interpreter/test_runner.rs
//
// Test runner for executing Kujo test suites.
//
// Provides infrastructure for:
// - Collecting test cases from AST
// - Running tests in isolation with fresh interpreter instances
// - Setup/teardown hooks for test initialization and cleanup
// - Result reporting with colored output
// - Test grouping and organization

use crate::ast::Stmt;
use crate::interpreter::{Interpreter, Value};

/// Test runner for executing Kujo test suites
pub struct TestRunner {
    pub tests: Vec<TestCase>,
    pub setup: Option<Vec<Stmt>>,
    pub teardown: Option<Vec<Stmt>>,
    pub results: Vec<TestResult>,
}

/// Individual test case with name and body
#[derive(Clone)]
pub struct TestCase {
    pub name: String,
    pub body: Vec<Stmt>,
}

/// Result from executing a single test
#[derive(Clone, Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: Option<String>,
    pub duration_ms: u128,
}

impl TestRunner {
    /// Create a new test runner
    pub fn new() -> Self {
        TestRunner { tests: Vec::new(), setup: None, teardown: None, results: Vec::new() }
    }

    /// Collect all test statements from the AST
    pub fn collect_tests(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match stmt {
                Stmt::Test { name, body } => {
                    self.tests.push(TestCase { name: name.clone(), body: body.clone() });
                }
                Stmt::TestSetup { body } => {
                    self.setup = Some(body.clone());
                }
                Stmt::TestTeardown { body } => {
                    self.teardown = Some(body.clone());
                }
                Stmt::TestGroup { name: _, tests } => {
                    // Recursively collect tests from groups
                    self.collect_tests(tests);
                }
                _ => {}
            }
        }
    }

    /// Run all collected tests and return a report
    pub fn run_all(&mut self, base_interp: &Interpreter) -> TestReport {
        let start_time = std::time::Instant::now();

        for test in &self.tests {
            let result = self.run_single_test(&test.name, &test.body, base_interp);
            self.results.push(result);
        }

        let duration = start_time.elapsed();
        TestReport {
            total: self.results.len(),
            passed: self.results.iter().filter(|r| r.passed).count(),
            failed: self.results.iter().filter(|r| !r.passed).count(),
            duration_ms: duration.as_millis(),
            results: self.results.clone(),
        }
    }

    /// Run a single test in isolation
    fn run_single_test(&self, name: &str, body: &[Stmt], base_interp: &Interpreter) -> TestResult {
        let start_time = std::time::Instant::now();

        // Create fresh interpreter for this test
        let mut test_interp = Interpreter::new();

        // Copy environment from base interpreter (for imports, etc.)
        test_interp.env = base_interp.env.clone();

        // Run setup if present
        if let Some(setup_stmts) = &self.setup {
            test_interp.eval_stmts(setup_stmts);

            // Check for errors in setup
            if let Some(Value::Error(msg)) = &test_interp.return_value {
                return TestResult {
                    name: name.to_string(),
                    passed: false,
                    message: Some(format!("Setup failed: {}", msg)),
                    duration_ms: start_time.elapsed().as_millis(),
                };
            }

            // Clear return value after setup
            test_interp.return_value = None;
        }

        // Run test body
        for stmt in body {
            test_interp.eval_stmt(stmt);

            // Check if any statement returned an Error (failed assertion)
            if let Some(ref val) = test_interp.return_value {
                match val {
                    Value::Error(msg) => {
                        // Assertion failed
                        return TestResult {
                            name: name.to_string(),
                            passed: false,
                            message: Some(msg.clone()),
                            duration_ms: start_time.elapsed().as_millis(),
                        };
                    }
                    Value::ErrorObject { message, .. } => {
                        // Error object encountered
                        return TestResult {
                            name: name.to_string(),
                            passed: false,
                            message: Some(message.clone()),
                            duration_ms: start_time.elapsed().as_millis(),
                        };
                    }
                    _ => {}
                }
            }

            // Clear return value for next statement
            test_interp.return_value = None;
        }

        // Run teardown if present
        if let Some(teardown_stmts) = &self.teardown {
            test_interp.eval_stmts(teardown_stmts);
            // We don't fail the test if teardown fails, but we could log it
        }

        let duration = start_time.elapsed();
        TestResult {
            name: name.to_string(),
            passed: true,
            message: None,
            duration_ms: duration.as_millis(),
        }
    }
}

/// Summary report of test execution
#[derive(Debug)]
pub struct TestReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub duration_ms: u128,
    pub results: Vec<TestResult>,
}

impl TestReport {
    /// Print the test report to stdout with colored output
    pub fn print(&self, verbose: bool) {
        use colored::Colorize;

        println!("\n{}", "=".repeat(60));
        println!("{}", "Test Results".bold());
        println!("{}", "=".repeat(60));

        if verbose {
            for result in &self.results {
                if result.passed {
                    println!(
                        "  {} {} ({}ms)",
                        "✓".green().bold(),
                        result.name.green(),
                        result.duration_ms
                    );
                } else {
                    println!(
                        "  {} {} ({}ms)",
                        "✗".red().bold(),
                        result.name.red(),
                        result.duration_ms
                    );
                    if let Some(msg) = &result.message {
                        println!("    {}: {}", "Error".red().bold(), msg.dimmed());
                    }
                }
            }
            println!();
        }

        println!(
            "Tests: {} total, {} passed, {} failed",
            self.total,
            self.passed.to_string().green().bold(),
            self.failed.to_string().red().bold()
        );
        println!("Time:  {}ms", self.duration_ms);
        println!("{}", "=".repeat(60));

        if self.failed == 0 {
            println!("\n{}", "All tests passed! ✨".green().bold());
        } else {
            println!("\n{}", format!("{} test(s) failed", self.failed).red().bold());
        }
    }

    /// Get exit code (0 for success, 1 for failure)
    pub fn exit_code(&self) -> i32 {
        if self.failed == 0 {
            0
        } else {
            1
        }
    }
}
