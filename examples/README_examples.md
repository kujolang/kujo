# Kujo Examples - Comprehensive Feature Showcase

This directory contains example programs demonstrating Kujo's features.

## 🎯 Featured Examples

These examples showcase core Kujo features such as **lexical scoping**, **user input**, **type conversion**, and **file I/O**.

### Interactive Applications

#### 📝 **note_taking_app.kujo**
A complete note-taking application with menu system.
- **Features**: Create, read, list, and append to notes
- **Demonstrates**: File I/O, user input, directory operations, loops, lexical scoping
- **Try it**: `cargo run --quiet -- run examples/note_taking_app.kujo`

#### 📊 **student_grade_tracker.kujo**
Track student grades with data persistence.
- **Features**: Add grades, view all entries, validate input
- **Demonstrates**: File I/O, parse_int, error handling, input validation
- **Try it**: `cargo run --quiet -- run examples/student_grade_tracker.kujo`

#### 💰 **expense_tracker.kujo**
Personal expense tracking system.
- **Features**: Add expenses, view history, accumulator pattern
- **Demonstrates**: parse_float, file operations, lexical scoping
- **Try it**: `cargo run --quiet -- run examples/expense_tracker.kujo`

#### 🎮 **quiz_game.kujo**
Interactive programming quiz with score tracking.
- **Features**: Multiple question types, score calculation, percentage display
- **Demonstrates**: User input, parse_int, lexical scoping, accumulators
- **Try it**: `cargo run --quiet -- run examples/quiz_game.kujo`

#### 🔐 **password_generator.kujo**
Simple password generator and storage manager.
- **Features**: Generate passwords, store credentials, view entries
- **Demonstrates**: Loops, file I/O, parse_int, conditional logic
- **Try it**: `cargo run --quiet -- run examples/password_generator.kujo`

#### 💾 **backup_tool.kujo**
Automated backup utility for directories.
- **Features**: List files, copy files, create backup logs
- **Demonstrates**: Directory operations, list_dir, file_exists, error handling
- **Try it**: `cargo run --quiet -- run examples/backup_tool.kujo`

### Quick Start Examples

#### 🎲 **guessing_game.kujo** _(Original)_
Number guessing game with input validation.
- `cargo run --quiet -- run examples/guessing_game.kujo`

#### 🧮 **interactive_calculator.kujo** _(Original)_
Calculator supporting +, -, *, / operations.
- `cargo run --quiet -- run examples/interactive_calculator.kujo`

#### 👋 **interactive_greeting.kujo** _(Original)_
Simple greeting with name and age input.
- `cargo run --quiet -- run examples/interactive_greeting.kujo`

### File I/O Basics

#### 📄 **file_logger.kujo**
Simple logging with write, append, and read operations.
- `cargo run --quiet -- run examples/file_logger.kujo`

#### 📁 **directory_tools.kujo**
Directory creation, listing, and file existence checks.
- `cargo run --quiet -- run examples/directory_tools.kujo`

#### ⚙️ **config_manager.kujo**
Configuration file management with error handling.
- `cargo run --quiet -- run examples/config_manager.kujo`

## 📚 Core Language Examples

### Data Structures
- **arrays.kujo** - Array operations and methods
- **dictionaries.kujo** - Dictionary/hash map operations
- **collections.kujo** - Collections overview

### Control Flow
- **for_loops.kujo** - For-in iteration
- **test_if_else.kujo** - Conditional statements
- **pattern_matching.kujo** - Match/case statements

### Structs & Methods
- **struct_basic.kujo** - Basic struct definitions
- **struct_methods.kujo** - Methods on structs
- **structs_comprehensive.kujo** - Complete struct features
- **struct_nested.kujo** - Nested struct instances

### Error Handling
- **error_handling.kujo** - Try/except basics
- **error_handling_comprehensive.kujo** - Advanced error handling
- **try_throw.kujo** - Throwing and catching errors

### Type System
- **type_annotations.kujo** - Type annotation examples
- **type_inference.kujo** - Type inference demonstration
- **type_errors.kujo** - Type checking errors

### Functions & Modules
- **basic_import.kujo** - Module imports
- **selective_import.kujo** - Importing specific functions
- **math_module.kujo** - Using math functions

## 🎨 Advanced Examples

### Project Templates (examples/projects/)
- **todo_manager.kujo** - Complete TODO list application
- **contact_manager.kujo** - Contact management system

## 🚀 Running Examples

```bash
# Interactive examples (with user input)
cargo run --quiet -- run examples/note_taking_app.kujo
cargo run --quiet -- run examples/quiz_game.kujo
cargo run --quiet -- run examples/expense_tracker.kujo

# Non-interactive demonstrations
cargo run --quiet -- run examples/file_logger.kujo
cargo run --quiet -- run examples/directory_tools.kujo
cargo run --quiet -- run examples/scoping.kujo
```

## 💡 Learning Path

1. **Start Here**: `hello.kujo`, `basic_import.kujo`
2. **Control Flow**: `test_if_else.kujo`, `for_loops.kujo`
3. **Data Structures**: `arrays.kujo`, `dictionaries.kujo`
4. **User Input**: `interactive_greeting.kujo`, `guessing_game.kujo`
5. **File I/O**: `file_logger.kujo`, `config_manager.kujo`
6. **Complete Apps**: `note_taking_app.kujo`, `quiz_game.kujo`

## 📖 Feature Coverage

| Feature | Examples |
|---------|----------|
| **Lexical Scoping** | scoping.kujo, quiz_game.kujo, note_taking_app.kujo |
| **User Input** | All interactive_*.kujo, quiz_game.kujo |
| **Type Conversion** | parse_int: guessing_game.kujo, student_grade_tracker.kujo |
|  | parse_float: interactive_calculator.kujo, expense_tracker.kujo |
| **File I/O (Read/Write)** | file_logger.kujo, note_taking_app.kujo, backup_tool.kujo |
| **Directory Operations** | directory_tools.kujo, backup_tool.kujo |
| **Error Handling** | All try/except examples, config_manager.kujo |
| **Structs** | struct_*.kujo examples |
| **Pattern Matching** | pattern_matching.kujo |
| **Arrays & Dicts** | arrays.kujo, dictionaries.kujo, collections.kujo |

## 🔧 Tips

- Use `--quiet` flag with cargo to hide compilation messages
- Interactive examples wait for user input - press Ctrl+C to exit
- File I/O examples create temporary files in `/tmp/`
- Check each example's comments for detailed explanations

---

**New to Kujo?** Start with `hello.kujo` and `interactive_greeting.kujo` to get a feel for the language!
