# Kujo Examples

This directory contains Kujo programs for learning the language, validating features, and exploring larger scripts. Start with the canonical learning path below; the showcase and legacy sections are useful, but they are not the first style source for new code.

## Running Examples

From the repository root:

```bash
cargo run -- run examples/hello.kujo
cargo run -- check examples/arrays.kujo --quiet
```

Use `cargo run --quiet -- ...` when you want to hide Cargo build output.

## Learning Path

1. Hello: `hello.kujo`
2. Variables and output: `01-variables.kujo`, `test_print.kujo`, `string_interpolation.kujo`
3. Control flow: `03-control-flow.kujo`, `test_if_else.kujo`, `for_loops.kujo`
4. Data structures: `04-data.kujo`, `arrays.kujo`, `dictionaries.kujo`, `collections.kujo`
5. Functions: `02-functions.kujo`, `test_simple_func.kujo`, `closures_higher_order.kujo`
6. Modules: `05-modules.kujo`, `basic_import.kujo`, `selective_import.kujo`
7. File I/O: `file_logger.kujo`, `directory_tools.kujo`, `config_manager.kujo`
8. Complete apps and tools: `06-agent-tool.kujo`, `note_taking_app.kujo`, `quiz_game.kujo`, `expense_tracker.kujo`

## Canonical Learning Examples

These files are intended as current, copyable examples. `tests/docs_examples.rs` either runs them directly or checks that they parse.

### First Programs

- `hello.kujo`: smallest runnable hello program.
- `00-hello.kujo`: numbered one-line hello program.
- `01-variables.kujo`: bindings and direct output.
- `02-functions.kujo`: small function plus loop.
- `03-control-flow.kujo`: minimal branch.
- `04-data.kujo`: array and dictionary access.
- `05-modules.kujo`: parse-only import syntax tour.
- `06-agent-tool.kujo`: small JSON-oriented tool shape.
- `math_module.kujo`: self-contained math helper/module-style example.
- `pattern_matching.kujo`: current `match` and enum pattern syntax.
- `test_print.kujo`: direct printing.
- `string_interpolation.kujo`: interpolation basics.
- `comments.kujo`: comment syntax.

### Core Language

- `arrays.kujo`: array operations.
- `dictionaries.kujo`: dictionary/hash map operations.
- `collections.kujo`: collection overview.
- `for_loops.kujo`: for-in iteration.
- `test_if_else.kujo`: conditional branches.
- `test_simple_func.kujo`: function definition and calls.
- `closures_higher_order.kujo`: higher-order function basics.

### Modules and Tools

- `basic_import.kujo`: basic module imports.
- `selective_import.kujo`: importing specific functions.
- `cli_tool.kujo`: command-line tool structure.
- `arg_parser_demo.kujo`: argument parsing patterns.
- `env_config.kujo`: environment-driven configuration.

### File and System Work

- `file_logger.kujo`: write, append, and read a simple log.
- `directory_tools.kujo`: directory creation, listing, and existence checks.
- `config_manager.kujo`: configuration file management.
- `file_operations_demo.kujo`: broader file operation examples.
- `path_utilities.kujo`: path helper usage.

## Showcases

These are larger examples for exploring practical scripts after the learning path.

### Interactive Applications

- `note_taking_app.kujo`: create, read, list, and append notes.
- `student_grade_tracker.kujo`: track grades with persistence and validation.
- `expense_tracker.kujo`: personal expense tracking.
- `quiz_game.kujo`: programming quiz with scoring.
- `password_generator.kujo`: password generation and storage.
- `backup_tool.kujo`: directory backup utility.

### Networking, Data, and Services

- `http_client.kujo`: HTTP client basics.
- `http_download.kujo`: downloading content.
- `http_rest_api.kujo`: REST-oriented HTTP example.
- `json_demo.kujo`: JSON handling.
- `stdlib_crypto.kujo`: hashing and password helper examples.
- `database_postgres.kujo`: PostgreSQL demo.
- `database_transactions.kujo`: transaction flow.
- `examples/projects/todo_manager.kujo`: complete TODO application.
- `examples/projects/blog_api.kujo`: API-oriented project example.
- `examples/projects/log_parser.kujo`: log parsing and summary project.
- `project_markdown_converter.kujo`: compact markdown-to-HTML converter sketch.

### Benchmarks and Runtime Demos

- `examples/benchmarks/`: benchmark fixtures and comparisons.
- `benchmark_demo.kujo`: benchmark helper usage.
- `jit_simple.kujo`: JIT-oriented runtime demo.
- `async_await_demo.kujo`: async/await basics.
- `concurrency_spawn.kujo`: spawn/concurrency demo.

## Verification Status

Every tracked `.kujo` file under `examples/` is covered by `tests/docs_examples.rs`. Safe, deterministic programs are executed; effectful, interactive, long-running, and intentionally diagnostic examples are syntax-checked. `testing_demo.kujo` is executed through `kujo test-run`.

See `VERIFICATION.md` for the exhaustive per-file status table.

## Feature Coverage

| Feature | Current examples |
|---|---|
| Lexical scoping | `scoping.kujo`, `scoping_simple.kujo`, `quiz_game.kujo` |
| User input | `interactive_greeting.kujo`, `guessing_game.kujo`, `quiz_game.kujo` |
| Type conversion | `type_conversion.kujo`, `interactive_calculator.kujo`, `expense_tracker.kujo` |
| File I/O | `file_logger.kujo`, `directory_tools.kujo`, `backup_tool.kujo` |
| Error handling | `error_handling.kujo`, `error_handling_comprehensive.kujo`, `try_throw.kujo` |
| Structs | `struct_basic.kujo`, `struct_methods.kujo`, `struct_nested.kujo`, `structs_comprehensive.kujo` |
| Arrays and dictionaries | `arrays.kujo`, `dictionaries.kujo`, `collections.kujo` |

## Tips

- Interactive examples wait for user input; press Ctrl+C to exit.
- File I/O examples may create temporary files under `/tmp/`.
- Check each example's comments for detailed explanations.
- Prefer `print_lines(lines)`, `section(title)`, `kv(label, value)`, and `ok(message)` helpers when an example has repeated output blocks.
- When in doubt, use `cargo run -- check <path> --quiet` before copying a larger example.
