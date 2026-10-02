//! Native ahead-of-time compilation for Kujo's allocation-free scalar core.
//!
//! The backend deliberately rejects unsupported syntax instead of changing its
//! meaning. It is intended for numeric command-line programs that use scalar
//! functions, local bindings, branches, loops, checked integer arithmetic, and
//! `print`/`to_string` output.

use crate::ast::{Expr, Pattern, Stmt};
use crate::lexer::tokenize_with_file;
use crate::parser::Parser;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const C_PREAMBLE: &str = r#"#include <stddef.h>
#include <stdint.h>

#if defined(__APPLE__) && defined(__aarch64__)
static inline long kujo_syscall3(long number, long a, long b, long c) {
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    register long x2 __asm__("x2") = c;
    register long x16 __asm__("x16") = number;
    __asm__ volatile("svc #0x80" : "+r"(x0) : "r"(x1), "r"(x2), "r"(x16) : "memory");
    return x0;
}
static inline __attribute__((noreturn)) void kujo_exit(long status) {
    register long x0 __asm__("x0") = status;
    register long x16 __asm__("x16") = 1;
    __asm__ volatile("svc #0x80" : : "r"(x0), "r"(x16) : "memory");
    __builtin_unreachable();
}
#define KUJO_WRITE 4
#define KUJO_ENTRY __attribute__((noreturn)) void start(void)
#elif defined(__linux__) && defined(__aarch64__)
static inline long kujo_syscall3(long number, long a, long b, long c) {
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    register long x2 __asm__("x2") = c;
    register long x8 __asm__("x8") = number;
    __asm__ volatile("svc #0" : "+r"(x0) : "r"(x1), "r"(x2), "r"(x8) : "memory");
    return x0;
}
static inline __attribute__((noreturn)) void kujo_exit(long status) {
    register long x0 __asm__("x0") = status;
    register long x8 __asm__("x8") = 93;
    __asm__ volatile("svc #0" : : "r"(x0), "r"(x8) : "memory");
    __builtin_unreachable();
}
#define KUJO_WRITE 64
#define KUJO_ENTRY __attribute__((noreturn)) void _start(void)
#elif defined(__APPLE__) && defined(__x86_64__)
static inline long kujo_syscall3(long number, long a, long b, long c) {
    long result;
    __asm__ volatile("syscall" : "=a"(result) : "a"(number), "D"(a), "S"(b), "d"(c) : "rcx", "r11", "memory");
    return result;
}
static inline __attribute__((noreturn)) void kujo_exit(long status) {
    __asm__ volatile("syscall" : : "a"(0x2000001L), "D"(status) : "rcx", "r11", "memory");
    __builtin_unreachable();
}
#define KUJO_WRITE 0x2000004L
#define KUJO_ENTRY __attribute__((noreturn)) void start(void)
#elif defined(__linux__) && defined(__x86_64__)
static inline long kujo_syscall3(long number, long a, long b, long c) {
    long result;
    __asm__ volatile("syscall" : "=a"(result) : "a"(number), "D"(a), "S"(b), "d"(c) : "rcx", "r11", "memory");
    return result;
}
static inline __attribute__((noreturn)) void kujo_exit(long status) {
    __asm__ volatile("syscall" : : "a"(60L), "D"(status) : "rcx", "r11", "memory");
    __builtin_unreachable();
}
#define KUJO_WRITE 1
#define KUJO_ENTRY __attribute__((noreturn)) void _start(void)
#else
#error "kujo-aot currently supports macOS and Linux on arm64 and x86_64"
#endif

static char kujo_output[4096];
static size_t kujo_output_len;

static inline void kujo_flush(void) {
    size_t offset = 0;
    while (offset < kujo_output_len) {
        long written = kujo_syscall3(KUJO_WRITE, 1, (long)(kujo_output + offset), (long)(kujo_output_len - offset));
        if (written <= 0) { kujo_exit(1); }
        offset += (size_t)written;
    }
    kujo_output_len = 0;
}
static inline void kujo_write_bytes(const char *value, size_t length) {
    if (length > sizeof(kujo_output) - kujo_output_len) { kujo_flush(); }
    if (length > sizeof(kujo_output)) {
        if (kujo_syscall3(KUJO_WRITE, 1, (long)value, (long)length) != (long)length) { kujo_exit(1); }
        return;
    }
    for (size_t index = 0; index < length; index++) { kujo_output[kujo_output_len + index] = value[index]; }
    kujo_output_len += length;
}
static inline void kujo_write_int(int64_t value) {
    char digits[21];
    char *cursor = digits + sizeof(digits);
    uint64_t magnitude = value < 0 ? (uint64_t)(-(value + 1)) + 1 : (uint64_t)value;
    do { *--cursor = (char)('0' + magnitude % 10); magnitude /= 10; } while (magnitude != 0);
    if (value < 0) { *--cursor = '-'; }
    kujo_write_bytes(cursor, (size_t)(digits + sizeof(digits) - cursor));
}
static inline void kujo_abort(const char *message, size_t length) {
    (void)kujo_syscall3(KUJO_WRITE, 2, (long)message, (long)length);
    kujo_exit(1);
}

static inline int64_t kujo_add(int64_t a, int64_t b) {
    int64_t out;
    if (__builtin_add_overflow(a, b, &out)) { kujo_abort("integer overflow\n", 17); }
    return out;
}
static inline int64_t kujo_sub(int64_t a, int64_t b) {
    int64_t out;
    if (__builtin_sub_overflow(a, b, &out)) { kujo_abort("integer overflow\n", 17); }
    return out;
}
static inline int64_t kujo_mul(int64_t a, int64_t b) {
    int64_t out;
    if (__builtin_mul_overflow(a, b, &out)) { kujo_abort("integer overflow\n", 17); }
    return out;
}
static inline int64_t kujo_neg(int64_t value) {
    int64_t out;
    if (__builtin_sub_overflow((int64_t)0, value, &out)) { kujo_abort("integer overflow\n", 17); }
    return out;
}
static inline int64_t kujo_div(int64_t a, int64_t b) {
    if (b == 0) { kujo_abort("division by zero\n", 17); }
    if (a == INT64_MIN && b == -1) { kujo_abort("integer overflow\n", 17); }
    return a / b;
}
static inline int64_t kujo_mod(int64_t a, int64_t b) {
    if (b == 0) { kujo_abort("division by zero\n", 17); }
    if (a == INT64_MIN && b == -1) { return 0; }
    return a % b;
}

"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScalarType {
    Int,
    Bool,
    String,
}

#[derive(Default)]
struct FunctionContext {
    locals: HashMap<String, ScalarType>,
}

pub fn compile_file_to_c(path: &Path) -> Result<String, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let filename = path.to_string_lossy();
    let tokens = tokenize_with_file(&source, Some(&filename))
        .map_err(|diagnostics| format!("lexing failed: {diagnostics:?}"))?;
    let mut parser = Parser::new(tokens);
    let parsed = parser.parse_with_diagnostics();
    if !parsed.diagnostics.is_empty() {
        return Err(format!("parsing failed: {:?}", parsed.diagnostics));
    }
    NativeCodegen::new().compile_program(&parsed.stmts)
}

pub fn compile_file_to_executable(
    source: &Path,
    output: &Path,
    cc: Option<&Path>,
) -> Result<(), String> {
    let c_source = compile_file_to_c(source)?;
    let output_parent =
        output.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
    fs::create_dir_all(output_parent)
        .map_err(|error| format!("failed to create '{}': {error}", output_parent.display()))?;
    let temp = tempfile::Builder::new()
        .prefix(".kujo-aot-")
        .tempdir_in(output_parent)
        .map_err(|error| format!("failed to create compiler workspace: {error}"))?;
    let c_path = temp.path().join("program.c");
    let native_path = temp.path().join("program");
    fs::write(&c_path, c_source)
        .map_err(|error| format!("failed to write generated C: {error}"))?;

    let compiler: OsString = cc
        .map(|path| path.as_os_str().to_owned())
        .or_else(|| std::env::var_os("CC"))
        .unwrap_or_else(|| OsString::from("cc"));
    let result = Command::new(&compiler)
        .args([
            "-x",
            "c",
            "-std=c11",
            "-O3",
            "-DNDEBUG",
            "-flto",
            "-ffreestanding",
            "-fno-builtin",
            "-fno-stack-protector",
            "-nostdlib",
            "-Wl,-e,_start",
            "-Wl,-static",
        ])
        .arg(&c_path)
        .arg("-o")
        .arg(&native_path)
        .output()
        .map_err(|error| format!("failed to launch {:?}: {error}", compiler))?;
    if !result.status.success() {
        return Err(format!(
            "native compiler failed ({}): {}",
            result.status,
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }

    if output.exists() {
        fs::remove_file(output)
            .map_err(|error| format!("failed to replace '{}': {error}", output.display()))?;
    }
    fs::rename(&native_path, output)
        .map_err(|error| format!("failed to publish '{}': {error}", output.display()))?;
    Ok(())
}

struct NativeCodegen {
    functions: HashMap<String, usize>,
    output: String,
}

impl NativeCodegen {
    fn new() -> Self {
        Self { functions: HashMap::new(), output: String::from(C_PREAMBLE) }
    }

    fn compile_program(mut self, statements: &[Stmt]) -> Result<String, String> {
        for statement in statements {
            if let Stmt::FuncDef { name, params, is_async, is_generator, .. } = statement {
                if *is_async || *is_generator {
                    return Err(format!(
                        "native AOT does not support async/generator function '{name}'"
                    ));
                }
                self.functions.insert(name.clone(), params.len());
            }
        }
        for statement in statements {
            if let Stmt::FuncDef { name, params, .. } = statement {
                self.output.push_str("static int64_t ");
                self.output.push_str(&c_name(name));
                self.output.push('(');
                self.output.push_str(
                    &params
                        .iter()
                        .map(|param| format!("int64_t {}", c_name(param)))
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                self.output.push_str(");\n");
            }
        }
        self.output.push('\n');
        for statement in statements {
            if let Stmt::FuncDef { name, params, body, .. } = statement {
                self.emit_function(name, params, body)?;
            }
        }
        self.output.push_str("KUJO_ENTRY {\n");
        let mut context = FunctionContext::default();
        for statement in statements {
            if !matches!(statement, Stmt::FuncDef { .. }) {
                self.emit_stmt(statement, 1, &mut context)?;
            }
        }
        self.output.push_str("    kujo_flush();\n    kujo_exit(0);\n}\n");
        Ok(self.output)
    }

    fn emit_function(
        &mut self,
        name: &str,
        params: &[String],
        body: &[Stmt],
    ) -> Result<(), String> {
        self.output.push_str("static int64_t ");
        self.output.push_str(&c_name(name));
        self.output.push('(');
        self.output.push_str(
            &params
                .iter()
                .map(|param| format!("int64_t {}", c_name(param)))
                .collect::<Vec<_>>()
                .join(", "),
        );
        self.output.push_str(") {\n");
        let mut context = FunctionContext::default();
        for param in params {
            context.locals.insert(param.clone(), ScalarType::Int);
        }
        for statement in body {
            self.emit_stmt(statement, 1, &mut context)?;
        }
        self.output.push_str("}\n\n");
        Ok(())
    }

    fn emit_stmt(
        &mut self,
        statement: &Stmt,
        indent: usize,
        context: &mut FunctionContext,
    ) -> Result<(), String> {
        let pad = "    ".repeat(indent);
        match statement {
            Stmt::Let { pattern: Pattern::Identifier(name), value, .. } => {
                let ty = self.expr_type(value, context)?;
                if ty == ScalarType::String {
                    return Err(format!("native AOT does not support stored string '{name}'"));
                }
                let value = self.emit_expr(value, context)?;
                context.locals.insert(name.clone(), ty);
                self.output.push_str(&format!("{pad}int64_t {} = {value};\n", c_name(name)));
            }
            Stmt::Const { name, value, .. } => {
                let ty = self.expr_type(value, context)?;
                if ty == ScalarType::String {
                    return Err(format!("native AOT does not support stored string '{name}'"));
                }
                let value = self.emit_expr(value, context)?;
                context.locals.insert(name.clone(), ty);
                self.output.push_str(&format!("{pad}const int64_t {} = {value};\n", c_name(name)));
            }
            Stmt::Assign { target: Expr::Identifier(name), value } => {
                if !context.locals.contains_key(name) {
                    return Err(format!("assignment to unknown native local '{name}'"));
                }
                let value = self.emit_expr(value, context)?;
                self.output.push_str(&format!("{pad}{} = {value};\n", c_name(name)));
            }
            Stmt::ExprStmt(Expr::Call { function, args }) if matches!(function.as_ref(), Expr::Identifier(name) if name == "print") =>
            {
                if args.len() != 1 {
                    return Err(
                        "native AOT print currently requires exactly one argument".to_string()
                    );
                }
                self.emit_print(&args[0], indent, context)?;
            }
            Stmt::ExprStmt(expression) => {
                let expression = self.emit_expr(expression, context)?;
                self.output.push_str(&format!("{pad}(void)({expression});\n"));
            }
            Stmt::Return(Some(expression)) => {
                let expression = self.emit_expr(expression, context)?;
                self.output.push_str(&format!("{pad}return {expression};\n"));
            }
            Stmt::Return(None) => self.output.push_str(&format!("{pad}return 0;\n")),
            Stmt::If { condition, then_branch, else_branch } => {
                let condition = self.emit_expr(condition, context)?;
                self.output.push_str(&format!("{pad}if ({condition}) {{\n"));
                let mut then_context = context.clone();
                for child in then_branch {
                    self.emit_stmt(child, indent + 1, &mut then_context)?;
                }
                self.output.push_str(&format!("{pad}}}"));
                if let Some(else_branch) = else_branch {
                    self.output.push_str(" else {\n");
                    let mut else_context = context.clone();
                    for child in else_branch {
                        self.emit_stmt(child, indent + 1, &mut else_context)?;
                    }
                    self.output.push_str(&format!("{pad}}}\n"));
                } else {
                    self.output.push('\n');
                }
            }
            Stmt::While { condition, body } => {
                let condition = self.emit_expr(condition, context)?;
                self.output.push_str(&format!("{pad}while ({condition}) {{\n"));
                let mut body_context = context.clone();
                for child in body {
                    self.emit_stmt(child, indent + 1, &mut body_context)?;
                }
                self.output.push_str(&format!("{pad}}}\n"));
            }
            Stmt::Loop { condition, body } => {
                let condition = condition
                    .as_ref()
                    .map(|value| self.emit_expr(value, context))
                    .transpose()?
                    .unwrap_or_else(|| "1".to_string());
                self.output.push_str(&format!("{pad}while ({condition}) {{\n"));
                let mut body_context = context.clone();
                for child in body {
                    self.emit_stmt(child, indent + 1, &mut body_context)?;
                }
                self.output.push_str(&format!("{pad}}}\n"));
            }
            Stmt::Break => self.output.push_str(&format!("{pad}break;\n")),
            Stmt::Continue => self.output.push_str(&format!("{pad}continue;\n")),
            Stmt::Block(body) => {
                self.output.push_str(&format!("{pad}{{\n"));
                let mut body_context = context.clone();
                for child in body {
                    self.emit_stmt(child, indent + 1, &mut body_context)?;
                }
                self.output.push_str(&format!("{pad}}}\n"));
            }
            Stmt::FuncDef { .. } => {
                return Err("nested functions are not supported by native AOT".to_string())
            }
            other => return Err(format!("native AOT does not support statement: {other:?}")),
        }
        Ok(())
    }

    fn emit_expr(&self, expression: &Expr, context: &FunctionContext) -> Result<String, String> {
        match expression {
            Expr::Identifier(name) => {
                if context.locals.contains_key(name) {
                    Ok(c_name(name))
                } else {
                    Err(format!("unknown native scalar '{name}'"))
                }
            }
            Expr::Int(value) => Ok(format!("((int64_t){value})")),
            Expr::Bool(value) => Ok(if *value { "1" } else { "0" }.to_string()),
            Expr::UnaryOp { op, operand } => {
                let operand = self.emit_expr(operand, context)?;
                match op.as_str() {
                    "-" => Ok(format!("kujo_neg({operand})")),
                    "!" => Ok(format!("(!({operand}))")),
                    _ => Err(format!("native AOT does not support unary operator '{op}'")),
                }
            }
            Expr::BinaryOp { left, op, right } => {
                let left = self.emit_expr(left, context)?;
                let right = self.emit_expr(right, context)?;
                match op.as_str() {
                    "+" => Ok(format!("kujo_add({left}, {right})")),
                    "-" => Ok(format!("kujo_sub({left}, {right})")),
                    "*" => Ok(format!("kujo_mul({left}, {right})")),
                    "/" => Ok(format!("kujo_div({left}, {right})")),
                    "%" => Ok(format!("kujo_mod({left}, {right})")),
                    "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||" => {
                        Ok(format!("(({left}) {op} ({right}))"))
                    }
                    _ => Err(format!("native AOT does not support binary operator '{op}'")),
                }
            }
            Expr::Call { function, args } => {
                let Expr::Identifier(name) = function.as_ref() else {
                    return Err("native AOT requires direct function calls".to_string());
                };
                let Some(arity) = self.functions.get(name) else {
                    return Err(format!("native AOT does not support builtin call '{name}' here"));
                };
                if *arity != args.len() {
                    return Err(format!(
                        "native call '{name}' expected {arity} arguments, got {}",
                        args.len()
                    ));
                }
                let args = args
                    .iter()
                    .map(|arg| self.emit_expr(arg, context))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("{}({args})", c_name(name)))
            }
            other => Err(format!("native AOT does not support expression: {other:?}")),
        }
    }

    fn expr_type(
        &self,
        expression: &Expr,
        context: &FunctionContext,
    ) -> Result<ScalarType, String> {
        match expression {
            Expr::String(_) | Expr::InterpolatedString(_) => Ok(ScalarType::String),
            Expr::Bool(_) => Ok(ScalarType::Bool),
            Expr::Identifier(name) => context
                .locals
                .get(name)
                .copied()
                .ok_or_else(|| format!("unknown native scalar '{name}'")),
            Expr::UnaryOp { op, .. } if op == "!" => Ok(ScalarType::Bool),
            Expr::BinaryOp { op, left, right } => {
                if matches!(op.as_str(), "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||") {
                    Ok(ScalarType::Bool)
                } else if op == "+"
                    && (self.expr_type(left, context)? == ScalarType::String
                        || self.expr_type(right, context)? == ScalarType::String)
                {
                    Ok(ScalarType::String)
                } else {
                    Ok(ScalarType::Int)
                }
            }
            Expr::Call { function, .. } if matches!(function.as_ref(), Expr::Identifier(name) if name == "to_string") => {
                Ok(ScalarType::String)
            }
            Expr::Int(_) | Expr::UnaryOp { .. } | Expr::Call { .. } => Ok(ScalarType::Int),
            other => Err(format!("native AOT cannot infer scalar type for: {other:?}")),
        }
    }

    fn emit_print(
        &mut self,
        expression: &Expr,
        indent: usize,
        context: &FunctionContext,
    ) -> Result<(), String> {
        let pad = "    ".repeat(indent);
        self.emit_print_piece(expression, &pad, context)?;
        self.output.push_str(&format!("{pad}kujo_write_bytes(\"\\n\", 1);\n"));
        Ok(())
    }

    fn emit_print_piece(
        &mut self,
        expression: &Expr,
        pad: &str,
        context: &FunctionContext,
    ) -> Result<(), String> {
        if let Expr::BinaryOp { left, op, right } = expression {
            if op == "+" && self.expr_type(expression, context)? == ScalarType::String {
                self.emit_print_piece(left, pad, context)?;
                self.emit_print_piece(right, pad, context)?;
                return Ok(());
            }
        }
        match expression {
            Expr::String(value) => {
                let encoded = c_string(value);
                self.output
                    .push_str(&format!("{pad}kujo_write_bytes(\"{encoded}\", {});\n", value.len()));
            }
            Expr::Call { function, args } if matches!(function.as_ref(), Expr::Identifier(name) if name == "to_string") =>
            {
                if args.len() != 1 {
                    return Err("to_string requires exactly one argument".to_string());
                }
                let value = self.emit_expr(&args[0], context)?;
                self.output.push_str(&format!("{pad}kujo_write_int({value});\n"));
            }
            _ => {
                let value = self.emit_expr(expression, context)?;
                let format = if self.expr_type(expression, context)? == ScalarType::Bool {
                    "kujo_write_bytes(({value}) ? \"true\" : \"false\", ({value}) ? 4 : 5);"
                } else {
                    "kujo_write_int({value});"
                };
                self.output.push_str(&format!("{pad}{}\n", format.replace("{value}", &value)));
            }
        }
        Ok(())
    }
}

impl Clone for FunctionContext {
    fn clone(&self) -> Self {
        Self { locals: self.locals.clone() }
    }
}

fn c_name(name: &str) -> String {
    let mut output = String::from("kujo_");
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'_' {
            output.push(char::from(byte));
        } else {
            output.push_str(&format!("_{byte:02x}"));
        }
    }
    output
}

fn c_string(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        match byte {
            b'\\' => output.push_str("\\\\"),
            b'\"' => output.push_str("\\\""),
            b'\n' => output.push_str("\\n"),
            b'\r' => output.push_str("\\r"),
            b'\t' => output.push_str("\\t"),
            0x20..=0x7e => output.push(char::from(byte)),
            _ => output.push_str(&format!("\\{byte:03o}")),
        }
    }
    output
}

pub fn default_output_path(source: &Path) -> PathBuf {
    let mut output = source.to_path_buf();
    output.set_extension("");
    output
}
