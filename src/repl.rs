// File: src/repl.rs
//
// Interactive REPL (Read-Eval-Print Loop) for the Kujo programming language.
// Provides an interactive shell for executing Kujo code with features like:
// - Multi-line input support for functions, loops, and control structures
// - Command history with up/down arrow navigation
// - Line editing capabilities
// - Special commands (:help, :clear, :quit, :vars)
// - Persistent state across inputs
// - Proper error handling and display

use crate::ast::Stmt;
use crate::interpreter::{Interpreter, Value};
use crate::lexer;
use crate::parser;
use colored::Colorize;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Context, Editor, Helper};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

struct ReplHelper {
    completion_items: Vec<String>,
}

impl ReplHelper {
    fn new() -> Self {
        let mut completion_items: Vec<String> = vec![
            ":help".to_string(),
            ":quit".to_string(),
            ":clear".to_string(),
            ":vars".to_string(),
            ":reset".to_string(),
            ".help".to_string(),
        ];

        completion_items
            .extend(Interpreter::get_builtin_names().into_iter().map(|name| name.to_string()));
        completion_items.sort();
        completion_items.dedup();

        Self { completion_items }
    }

    fn completion_start(line: &str, pos: usize) -> usize {
        let prefix = &line[..pos];
        prefix
            .rfind(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' || ch == '.'))
            .map(|index| index + 1)
            .unwrap_or(0)
    }
}

impl Helper for ReplHelper {}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let start = Self::completion_start(line, pos);
        let needle = &line[start..pos];

        let candidates: Vec<Pair> = self
            .completion_items
            .iter()
            .filter(|item| item.starts_with(needle))
            .map(|item| Pair { display: item.clone(), replacement: item.clone() })
            .collect();

        Ok((start, candidates))
    }
}

impl Hinter for ReplHelper {
    type Hint = String;
}

impl Highlighter for ReplHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if line.trim_start().starts_with(':') || line.trim_start().starts_with(".help") {
            Cow::Owned(line.bright_blue().to_string())
        } else {
            Cow::Borrowed(line)
        }
    }
}

impl Validator for ReplHelper {
    fn validate(&self, ctx: &mut ValidationContext<'_>) -> Result<ValidationResult, ReadlineError> {
        if is_input_complete_text(ctx.input()) {
            Ok(ValidationResult::Valid(None))
        } else {
            Ok(ValidationResult::Incomplete)
        }
    }
}

/// REPL session that maintains interpreter state and handles user interaction
pub struct Repl {
    interpreter: Interpreter,
    editor: Editor<ReplHelper, DefaultHistory>,
}

impl Repl {
    fn render_banner_text() -> String {
        [
            "╔══════════════════════════════════════════════════════╗",
            "║          Kujo REPL v0.5.0 - Interactive Shell       ║",
            "╚══════════════════════════════════════════════════════╝",
            "",
            "  Welcome! Use :help for commands or :quit",
            "  Tip: Multi-line input: End with unclosed braces",
            "",
        ]
        .join("\n")
    }

    fn render_help_text() -> String {
        [
            "",
            "REPL Commands:",
            "",
            "  :help or :h      Display this help message",
            "  :quit or :q      Exit the REPL",
            "  :clear or :c     Clear the screen",
            "  :vars or :v      Show defined variables",
            "  :reset or :r     Reset environment",
            "  .help <function> Show builtin docs",
            "",
            "Navigation:",
            "",
            "  ↑/↓ arrows  Navigate command history",
            "  Ctrl+C      Interrupt current input",
            "  Ctrl+D      Exit REPL",
            "",
            "Multi-line Input:",
            "",
            "  Leave braces, brackets, or parentheses unclosed to continue",
            "  on the next line. Close them to execute the statement.",
            "",
            "Examples:",
            "",
            "  kujo> let x := 42",
            "  kujo> func greet(name) {",
            "  ....>     print(\"Hello, \" + name)",
            "  ....> }",
            "  kujo> greet(\"World\")",
            "",
        ]
        .join("\n")
    }

    /// Creates a new REPL session with a fresh interpreter
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut editor = Editor::<ReplHelper, DefaultHistory>::new()?;
        editor.set_helper(Some(ReplHelper::new()));
        Ok(Repl { interpreter: Interpreter::new(), editor })
    }

    /// Displays the welcome banner with version and help information
    fn show_banner(&self) {
        if std::env::var_os("NO_COLOR").is_some() {
            println!("{}", Self::render_banner_text());
            return;
        }

        println!("{}", "╔══════════════════════════════════════════════════════╗".bright_cyan());
        println!("{}", "║          Kujo REPL v0.5.0 - Interactive Shell       ║".bright_cyan());
        println!("{}", "╚══════════════════════════════════════════════════════╝".bright_cyan());
        println!();
        println!(
            "  {} Use {}{}{}{}",
            "Welcome!".bright_green(),
            ":".bright_blue(),
            "help".bright_yellow(),
            " for commands or ".bright_blue(),
            ":quit".bright_yellow()
        );
        println!("  {} Multi-line input: End with unclosed braces", "Tip:".bright_magenta());
        println!();
    }

    /// Starts the REPL loop
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.show_banner();

        let mut buffer = String::new();

        loop {
            // Determine prompt based on whether we're in multi-line mode
            let prompt = if buffer.is_empty() {
                "kujo> ".bright_green().to_string()
            } else {
                "....> ".bright_blue().to_string()
            };

            match self.editor.readline(&prompt) {
                Ok(line) => {
                    // Add to history
                    let _ = self.editor.add_history_entry(line.as_str());

                    // Check for special commands (only when not in multi-line mode)
                    if buffer.is_empty()
                        && (line.trim().starts_with(':') || line.trim().starts_with(".help"))
                    {
                        if self.handle_command(line.trim()) {
                            continue;
                        } else {
                            break; // :quit was called
                        }
                    }

                    // Accumulate input
                    buffer.push_str(&line);
                    buffer.push('\n');

                    // Check if input is complete
                    if self.is_input_complete(&buffer) {
                        self.eval_input(&buffer);
                        buffer.clear();
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("{}", "^C (Ctrl+C to interrupt, :quit to exit)".bright_yellow());
                    buffer.clear();
                }
                Err(ReadlineError::Eof) => {
                    println!("{}", "\nGoodbye!".bright_cyan());
                    break;
                }
                Err(err) => {
                    eprintln!("{} {}", "Error:".bright_red(), err);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handles special REPL commands starting with ':'
    /// Returns true to continue REPL, false to quit
    fn handle_command(&mut self, cmd: &str) -> bool {
        if cmd.starts_with(".help") {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.len() == 2 {
                self.show_function_help(parts[1]);
            } else {
                println!(
                    "{} Use {}{}",
                    "Error:".bright_red(),
                    ".help".bright_yellow(),
                    " <function>".bright_blue()
                );
            }
            return true;
        }

        match cmd {
            ":help" | ":h" => {
                self.show_help();
                true
            }
            ":quit" | ":q" | ":exit" => {
                println!("{}", "Goodbye!".bright_cyan());
                false
            }
            ":clear" | ":c" => {
                // Clear the screen
                print!("\x1B[2J\x1B[1;1H");
                self.show_banner();
                true
            }
            ":vars" | ":v" => {
                self.show_variables();
                true
            }
            ":reset" | ":r" => {
                self.interpreter = Interpreter::new();
                println!("{}", "✓ Environment reset".bright_green());
                true
            }
            _ => {
                println!(
                    "{} Unknown command: {}. Type {}{}{}",
                    "Error:".bright_red(),
                    cmd.bright_yellow(),
                    ":".bright_blue(),
                    "help".bright_yellow(),
                    " for available commands.".bright_blue()
                );
                true
            }
        }
    }

    /// Displays help information about available commands
    fn show_help(&self) {
        if std::env::var_os("NO_COLOR").is_some() {
            println!("{}", Self::render_help_text());
            return;
        }

        println!();
        println!("{}", "REPL Commands:".bright_cyan().bold());
        println!();
        println!(
            "  {}{}  Display this help message",
            ":help".bright_yellow(),
            " or :h     ".dimmed()
        );
        println!("  {}{}  Exit the REPL", ":quit".bright_yellow(), " or :q     ".dimmed());
        println!("  {}{}  Clear the screen", ":clear".bright_yellow(), " or :c    ".dimmed());
        println!("  {}{}  Show defined variables", ":vars".bright_yellow(), " or :v    ".dimmed());
        println!("  {}{}  Reset environment", ":reset".bright_yellow(), " or :r   ".dimmed());
        println!("  {}{}  Show builtin docs", ".help".bright_yellow(), " <function>".dimmed());
        println!();
        println!("{}", "Navigation:".bright_cyan().bold());
        println!();
        println!("  {}  Navigate command history", "↑/↓ arrows".bright_blue());
        println!("  {}  Interrupt current input", "Ctrl+C    ".bright_blue());
        println!("  {}  Exit REPL", "Ctrl+D    ".bright_blue());
        println!();
        println!("{}", "Multi-line Input:".bright_cyan().bold());
        println!();
        println!("  Leave braces, brackets, or parentheses unclosed to continue");
        println!("  on the next line. Close them to execute the statement.");
        println!();
        println!("{}", "Examples:".bright_cyan().bold());
        println!();
        println!("  {}", "kujo> let x := 42".dimmed());
        println!("  {}", "kujo> func greet(name) {".dimmed());
        println!("  {}", "....>     print(\"Hello, \" + name)".dimmed());
        println!("  {}", "....> }".dimmed());
        println!("  {}", "kujo> greet(\"World\")".dimmed());
        println!();
    }

    fn show_function_help(&self, function_name: &str) {
        println!();
        println!("{} {}", "Function help:".bright_cyan().bold(), function_name.bright_yellow());

        let description = match function_name {
            "print" => "print(value) -> Writes a value to stdout.",
            "input" => "input(prompt) -> Reads a line from stdin.",
            "len" => "len(value) -> Returns length for strings, arrays, and dictionaries.",
            "range" => "range(start?, end, step?) -> Produces an integer sequence.",
            "read_file" => "read_file(path) -> Reads a UTF-8 file and returns content.",
            "http_get" => {
                "http_get(url) -> Performs an HTTP GET request and returns response data."
            }
            _ => "No dedicated help text yet for this function.",
        };

        println!("  {}", description.bright_white());
        println!();
    }

    /// Displays all currently defined variables in the environment
    fn show_variables(&self) {
        println!();
        println!("{}", "Defined Variables:".bright_cyan().bold());
        println!();

        // Get all variables from all scopes
        let _all_vars: HashMap<String, &Value> = HashMap::new();

        // Access the environment - we need to iterate through scopes
        // For now, we'll show this as a simplified view
        // In a full implementation, we'd expose more of the environment structure

        println!("  {}", "(Variable inspection not yet fully implemented)".dimmed());
        println!("  {}", "Tip: You can still use variables normally in the REPL".dimmed());
        println!();
    }

    /// Checks if the input is syntactically complete
    /// Returns true if all brackets/braces/parentheses are balanced
    fn is_input_complete(&self, input: &str) -> bool {
        is_input_complete_text(input)
    }

    /// Evaluates the input code and displays the result
    fn eval_input(&mut self, input: &str) {
        let trimmed = input.trim();

        // Skip empty input
        if trimmed.is_empty() {
            return;
        }

        // Tokenize and parse
        let tokens = match lexer::tokenize(input) {
            Ok(tokens) => tokens,
            Err(diagnostics) => {
                for diagnostic in diagnostics {
                    println!(
                        "{} {}:{} {}",
                        "Error:".bright_red(),
                        diagnostic.line,
                        diagnostic.column,
                        diagnostic.message
                    );
                }
                return;
            }
        };
        let mut parser = parser::Parser::new(tokens);

        let parse_output = parser.parse_with_diagnostics();
        if !parse_output.diagnostics.is_empty() {
            for diagnostic in parse_output.diagnostics {
                println!(
                    "{} {}:{} {}",
                    "Error:".bright_red(),
                    diagnostic.line,
                    diagnostic.column,
                    diagnostic.message
                );
            }
            return;
        }

        match parse_output.stmts {
            stmts if !stmts.is_empty() => {
                // Execute statements
                for stmt in &stmts {
                    match stmt {
                        // For expression statements in REPL, show the value
                        Stmt::ExprStmt(expr) => match self.interpreter.eval_expr_repl(expr) {
                            Ok(value) => {
                                self.print_value(&value);
                            }
                            Err(err) => {
                                self.print_error(&err);
                            }
                        },
                        // For other statements, just execute
                        _ => {
                            if let Err(err) = self.interpreter.eval_stmt_repl(stmt) {
                                self.print_error(&err);
                            }
                        }
                    }
                }
            }
            _ => {
                // Empty parse result
            }
        }
    }

    /// Formats and displays a value
    fn print_value(&self, value: &Value) {
        match value {
            Value::Int(n) => {
                println!("{} {}", "=>".bright_blue(), n.to_string().bright_white());
            }
            Value::Float(n) => {
                println!("{} {}", "=>".bright_blue(), n.to_string().bright_white());
            }
            Value::Str(s) => {
                println!("{} {}", "=>".bright_blue(), format!("\"{}\"", s).bright_green());
            }
            Value::Bool(b) => {
                println!("{} {}", "=>".bright_blue(), b.to_string().bright_magenta());
            }
            Value::Array(elements) => {
                print!("{} {}", "=>".bright_blue(), "[".bright_white());
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    print!("{}", self.format_value_inline(elem));
                }
                println!("{}", "]".bright_white());
            }
            Value::Dict(map) => {
                print!("{} {}", "=>".bright_blue(), "{".bright_white());
                let mut keys: Vec<&Arc<str>> = map.keys().collect();
                keys.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
                for (i, key) in keys.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    print!(
                        "{}: {}",
                        format!("\"{}\"", key).bright_yellow(),
                        self.format_value_inline(map.get(key.as_ref()).unwrap())
                    );
                }
                println!("{}", "}".bright_white());
            }
            Value::FixedDict { keys, values } => {
                print!("{} {}", "=>".bright_blue(), "{".bright_white());
                let mut pairs: Vec<(&Arc<str>, &Value)> = keys.iter().zip(values.iter()).collect();
                pairs.sort_by(|(a, _), (b, _)| a.as_ref().cmp(b.as_ref()));
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    print!(
                        "{}: {}",
                        format!("\"{}\"", key).bright_yellow(),
                        self.format_value_inline(value)
                    );
                }
                println!("{}", "}".bright_white());
            }
            Value::Function(params, _, _) => {
                println!(
                    "{} {}",
                    "=>".bright_blue(),
                    format!("<function({})>", params.join(", ")).bright_cyan()
                );
            }
            Value::Struct { name, fields } => {
                print!("{} {}", "=>".bright_blue(), format!("{} {{ ", name).bright_cyan());
                for (i, (key, val)) in fields.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    print!("{}: {}", key.bright_yellow(), self.format_value_inline(val));
                }
                println!("{}", " }".bright_cyan());
            }
            _ => {
                // For other types, use debug format
                println!("{} {:?}", "=>".bright_blue(), value);
            }
        }
    }

    /// Formats a value for inline display (used in arrays/dicts)
    fn format_value_inline(&self, value: &Value) -> String {
        match value {
            Value::Int(n) => n.to_string(),
            Value::Float(n) => n.to_string(),
            Value::Str(s) => format!("\"{}\"", s),
            Value::Bool(b) => b.to_string(),
            Value::Array(_) => "[...]".to_string(),
            Value::Dict(_) => "{...}".to_string(),
            Value::Function(params, _, _) => format!("<fn({})>", params.join(", ")),
            Value::Struct { name, .. } => format!("<{}>", name),
            _ => format!("{:?}", value),
        }
    }

    /// Displays an error message
    fn print_error(&self, err: &crate::errors::KujoError) {
        println!("{} {}", "Error:".bright_red().bold(), err.to_string().bright_red());
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new().expect("Failed to create REPL")
    }
}

fn is_input_complete_text(input: &str) -> bool {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return true;
    }

    if trimmed.ends_with('\\') {
        return false;
    }

    let mut brace_count = 0;
    let mut bracket_count = 0;
    let mut paren_count = 0;
    let mut in_string = false;
    let mut escape_next = false;
    let mut in_comment = false;

    for ch in trimmed.chars() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }

        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '#' if !in_string => in_comment = true,
            '{' if !in_string => brace_count += 1,
            '}' if !in_string => brace_count -= 1,
            '[' if !in_string => bracket_count += 1,
            ']' if !in_string => bracket_count -= 1,
            '(' if !in_string => paren_count += 1,
            ')' if !in_string => paren_count -= 1,
            _ => {}
        }
    }

    !in_string && brace_count == 0 && bracket_count == 0 && paren_count == 0
}

#[cfg(test)]
mod tests {
    use super::{is_input_complete_text, Repl};

    #[test]
    fn multiline_validator_detects_unclosed_delimiters() {
        assert!(!is_input_complete_text("func test() {\n"));
        assert!(!is_input_complete_text("print((1 + 2)\n"));
    }

    #[test]
    fn multiline_validator_detects_line_continuation() {
        assert!(!is_input_complete_text("let x := 1 + \\\n"));
    }

    #[test]
    fn multiline_validator_accepts_complete_input() {
        assert!(is_input_complete_text("let x := 1\n"));
        assert!(is_input_complete_text("print(1)\n"));
    }

    #[test]
    fn banner_text_snapshot_is_deterministic_and_no_color() {
        let text = Repl::render_banner_text();
        assert!(text.contains("Kujo REPL v0.5.0 - Interactive Shell"));
        assert!(text.contains("Welcome! Use :help for commands or :quit"));
        assert!(!text.contains("\u{1b}["), "snapshot text should not include ANSI escapes");
    }

    #[test]
    fn help_text_snapshot_is_deterministic_and_no_color() {
        let text = Repl::render_help_text();
        assert!(text.contains("REPL Commands:"));
        assert!(text.contains(":help or :h"));
        assert!(text.contains("Multi-line Input:"));
        assert!(text.contains("kujo> let x := 42"));
        assert!(!text.contains("\u{1b}["), "snapshot text should not include ANSI escapes");
    }
}
