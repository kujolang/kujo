# Kujo Language - Extension API

This document explains how to add native functions to Kujo by writing Rust code.

**Last Updated**: January 27, 2026  
**Version**: v0.9.0

---

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Native Function Module System](#native-function-module-system)
4. [Step-by-Step Guide](#step-by-step-guide)
5. [Advanced Patterns](#advanced-patterns)
6. [Binding to Rust Libraries](#binding-to-rust-libraries)
7. [Error Handling](#error-handling)
8. [Testing Native Functions](#testing-native-functions)
9. [Best Practices](#best-practices)
10. [Examples](#examples)

---

## Overview

Native functions are Rust functions callable from Kujo code. They provide:

- **Performance**: Native speed for computational tasks
- **System Access**: File I/O, networking, OS operations
- **Library Integration**: Wrap existing Rust crates
- **Custom Functionality**: Extend language capabilities

### Architecture

```
┌─────────────────────────────────────────────────┐
│ Kujo Code                                       │
│  result := custom_function(arg1, arg2)          │
└────────────────────┬────────────────────────────┘
                     │ Function call
                     ▼
┌─────────────────────────────────────────────────┐
│ Interpreter (src/interpreter/mod.rs)            │
│  Expr::Call → eval_call → call_native_function  │
└────────────────────┬────────────────────────────┘
                     │ Dispatch by name
                     ▼
┌─────────────────────────────────────────────────┐
│ Native Function Dispatcher                      │
│  (src/interpreter/native_functions/mod.rs)      │
│   - Try each category module                    │
│   - Return first match                          │
└────────────────────┬────────────────────────────┘
                     │
        ┌────────────┼────────────┬──────────────┐
        ▼            ▼            ▼              ▼
    ┌────────┐  ┌────────┐  ┌─────────┐  ┌─────────┐
    │ math   │  │strings │  │  io     │  │  http   │
    │ module │  │ module │  │ module  │  │ module  │
    └────────┘  └────────┘  └─────────┘  └─────────┘
        │            │            │              │
        └────────────┴────────────┴──────────────┘
                     │
                     ▼ Return Value
┌─────────────────────────────────────────────────┐
│ Back to Interpreter                             │
│  Value returned to Kujo code                    │
└─────────────────────────────────────────────────┘
```

---

## Quick Start

### 1. Choose a Category Module

Native functions are organized by category in `src/interpreter/native_functions/`:

```
native_functions/
├── mod.rs          # Main dispatcher
├── math.rs         # Mathematical functions
├── strings.rs      # String operations
├── collections.rs  # Array/dict operations
├── io.rs           # File I/O
├── filesystem.rs   # Filesystem operations
├── http.rs         # HTTP client
├── system.rs       # OS and process operations
├── type_ops.rs     # Type checking and conversion
├── concurrency.rs  # Channels and threading
├── json.rs         # JSON (stub)
├── crypto.rs       # Encryption (stub)
├── database.rs     # Databases (stub)
└── network.rs      # TCP/UDP (stub)
```

### 2. Add Your Function

**Example**: Add `double(x)` function to `math.rs`:

```rust
// File: src/interpreter/native_functions/math.rs

use crate::interpreter::Value;

pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        // NEW FUNCTION HERE
        "double" => {
            if let Some(val) = arg_values.first() {
                match val {
                    Value::Int(n) => Value::Int(n * 2),
                    Value::Float(n) => Value::Float(n * 2.0),
                    _ => Value::Error("Expected number".to_string()),
                }
            } else {
                Value::Error("double requires 1 argument".to_string())
            }
        }
        
        // ... existing functions
        
        _ => return None,  // Not handled by this module
    };
    Some(result)
}
```

### 3. Use in Kujo Code

```kujo
result := double(21)
print(result)  # Prints: 42
```

That's it! No registration, no boilerplate—just add the case and it works.

---

## Native Function Module System

### Module Structure

Each category module follows this pattern:

```rust
// File: src/interpreter/native_functions/category.rs

use crate::interpreter::{Interpreter, Value};

/// Handle category-specific function calls
/// 
/// Returns:
/// - Some(Value) if function was handled
/// - None if function name not recognized
pub fn handle(
    interp: &mut Interpreter,  // Optional: for functions that need interpreter access
    name: &str,                // Function name
    arg_values: &[Value],      // Arguments
) -> Option<Value> {
    let result = match name {
        "function1" => {
            // Implementation
            Value::Int(42)
        }
        "function2" => {
            // Implementation
            Value::Str("result".to_string())
        }
        _ => return None,  // Not this module's function
    };
    
    Some(result)
}
```

### Dispatcher Pattern

The main dispatcher tries each module until one handles the function:

```rust
// File: src/interpreter/native_functions/mod.rs

pub fn call_native_function(interp: &mut Interpreter, name: &str, arg_values: &[Value]) -> Value {
    // Try each category in order
    if let Some(result) = io::handle(interp, name, arg_values) {
        return result;
    }
    if let Some(result) = math::handle(name, arg_values) {
        return result;
    }
    if let Some(result) = strings::handle(name, arg_values) {
        return result;
    }
    // ... more categories
    
    // Unknown function
    Value::Int(0)  // Default return
}
```

**Key Points**:
1. **First match wins**: Order matters if function names could overlap
2. **Early return**: Once matched, no other modules are checked
3. **None = try next**: Returning `None` continues the search

---

## Step-by-Step Guide

### Example: Add `factorial(n)` Function

**Goal**: Implement `factorial(n)` that computes n!

#### Step 1: Choose Module

Mathematical function → `math.rs`

#### Step 2: Open `src/interpreter/native_functions/math.rs`

```bash
vim src/interpreter/native_functions/math.rs
# or
code src/interpreter/native_functions/math.rs
```

#### Step 3: Add Function Case

```rust
pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        // ADD THIS CASE
        "factorial" => {
            // Extract argument
            if let Some(val) = arg_values.first() {
                match val {
                    Value::Int(n) if *n >= 0 => {
                        // Compute factorial
                        let mut result = 1i64;
                        for i in 1..=*n {
                            result *= i;
                        }
                        Value::Int(result)
                    }
                    Value::Int(_) => {
                        Value::Error("factorial: negative numbers not allowed".to_string())
                    }
                    _ => {
                        Value::Error("factorial: expected integer".to_string())
                    }
                }
            } else {
                Value::Error("factorial requires 1 argument".to_string())
            }
        }
        
        // ... existing cases (abs, sqrt, etc.)
        
        _ => return None,
    };
    Some(result)
}
```

#### Step 4: Build and Test

```bash
cargo build
```

```kujo
# test_factorial.kujo
print(factorial(5))   # 120
print(factorial(10))  # 3628800
print(factorial(0))   # 1
```

```bash
cargo run -- run test_factorial.kujo
```

**Output**:
```
120
3628800
1
```

#### Step 5: Add Tests

Add integration test in `src/interpreter/mod.rs`:

```rust
#[test]
fn test_factorial() {
    let code = r#"
        print(factorial(5))
        print(factorial(0))
        print(factorial(10))
    "#;
    
    let mut interp = Interpreter::new();
    interp.run(code);
    
    // Verify output...
}
```

---

## Advanced Patterns

### Pattern 1: Functions Requiring Interpreter Access

Some functions need to call other Kujo functions or access the environment.

**Example**: `collections.rs` functions that use callbacks:

```rust
pub fn handle(interp: &mut Interpreter, name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        "map" => {
            // arr.map(func)
            if let (Some(Value::Array(arr)), Some(func)) = 
                (arg_values.first(), arg_values.get(1)) {
                
                let mut result = Vec::new();
                for item in arr {
                    // Call Kujo function with interpreter
                    let mapped = interp.call_function_value(
                        func.clone(), 
                        vec![item.clone()]
                    );
                    result.push(mapped);
                }
                Value::Array(result)
            } else {
                Value::Error("map requires array and function".to_string())
            }
        }
        _ => return None,
    };
    Some(result)
}
```

**Usage**:
```kujo
nums := [1, 2, 3]
doubled := nums.map(func(x) { return x * 2 })
print(doubled)  # [2, 4, 6]
```

### Pattern 2: Variable Arguments

Handle functions with any number of arguments:

```rust
"printf" => {
    // Format string + any number of args
    if arg_values.is_empty() {
        return Some(Value::Error("printf requires format string".to_string()));
    }
    
    let format_str = match &arg_values[0] {
        Value::Str(s) => s.clone(),
        _ => return Some(Value::Error("First argument must be string".to_string())),
    };
    
    let args = &arg_values[1..];  // Rest of arguments
    
    // Format string with args...
    Value::Str(formatted)
}
```

### Pattern 3: Optional Arguments

Provide default values for missing arguments:

```rust
"substring" => {
    // substring(str, start, end?)  - end is optional
    let string = match arg_values.first() {
        Some(Value::Str(s)) => s,
        _ => return Some(Value::Error("Expected string".to_string())),
    };
    
    let start = match arg_values.get(1) {
        Some(Value::Int(n)) => *n as usize,
        _ => return Some(Value::Error("Expected start index".to_string())),
    };
    
    let end = match arg_values.get(2) {
        Some(Value::Int(n)) => *n as usize,
        None => string.len(),  // Default: end of string
        _ => return Some(Value::Error("Expected end index".to_string())),
    };
    
    Value::Str(string[start..end].to_string())
}
```

### Pattern 4: Polymorphic Functions

Handle multiple types for the same operation:

```rust
"len" => {
    match arg_values.first() {
        Some(Value::Str(s)) => Value::Int(s.len() as i64),
        Some(Value::Array(arr)) => Value::Int(arr.len() as i64),
        Some(Value::Dict(dict)) => Value::Int(dict.len() as i64),
        Some(Value::Set(set)) => Value::Int(set.len() as i64),
        _ => Value::Error("len: unsupported type".to_string()),
    }
}
```

### Pattern 5: Stateful Functions

Use `Arc<Mutex<T>>` for functions that maintain state:

```rust
"open_file" => {
    let path = match arg_values.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Some(Value::Error("Expected file path".to_string())),
    };
    
    match std::fs::File::open(&path) {
        Ok(file) => {
            // Wrap in Arc<Mutex<>> for thread safety
            let file_handle = Arc::new(Mutex::new(file));
            Value::File(file_handle, path)
        }
        Err(e) => Value::Error(format!("Failed to open file: {}", e)),
    }
}

"read_line" => {
    let file_value = arg_values.first()?;
    match file_value {
        Value::File(handle, _) => {
            let mut file = handle.lock().unwrap();
            let mut line = String::new();
            use std::io::BufRead;
            let mut reader = std::io::BufReader::new(&*file);
            match reader.read_line(&mut line) {
                Ok(_) => Value::Str(line),
                Err(e) => Value::Error(format!("Read error: {}", e)),
            }
        }
        _ => Value::Error("Expected file handle".to_string()),
    }
}
```

---

## Binding to Rust Libraries

### Example: Wrap `reqwest` for HTTP

**Step 1: Add Dependency**

```toml
# Cargo.toml
[dependencies]
reqwest = { version = "0.12", features = ["blocking"] }
```

**Step 2: Create Function**

```rust
// src/interpreter/native_functions/http.rs

use reqwest::blocking::Client;

pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        "http_get" => {
            let url = match arg_values.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Some(Value::Error("Expected URL string".to_string())),
            };
            
            // Use reqwest
            match Client::new().get(&url).send() {
                Ok(response) => {
                    match response.text() {
                        Ok(body) => Value::Str(body),
                        Err(e) => Value::Error(format!("Read error: {}", e)),
                    }
                }
                Err(e) => Value::Error(format!("HTTP error: {}", e)),
            }
        }
        _ => return None,
    };
    Some(result)
}
```

**Step 3: Use in Kujo**

```kujo
response := http_get("https://api.github.com")
print(response)
```

### Example: Wrap `image` Crate

**Step 1: Add Dependency**

```toml
[dependencies]
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "gif", "webp"] }
```

**Step 2: Add Value Variant**

```rust
// src/interpreter/value.rs

use image::DynamicImage;

pub enum Value {
    // ... existing variants
    
    Image {
        data: Arc<Mutex<DynamicImage>>,
        format: String,
    },
}
```

**Step 3: Create Functions**

```rust
// src/interpreter/native_functions/image.rs (new file)

use crate::interpreter::Value;
use image::{DynamicImage, ImageFormat};
use std::sync::{Arc, Mutex};

pub fn handle(_interp: &mut Interpreter, name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        "load_image" => {
            let path = match arg_values.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Some(Value::Error("Expected path".to_string())),
            };
            
            match image::open(&path) {
                Ok(img) => Value::Image {
                    data: Arc::new(Mutex::new(img)),
                    format: "png".to_string(),
                },
                Err(e) => Value::Error(format!("Failed to load image: {}", e)),
            }
        }
        
        "resize_image" => {
            let (img_val, width, height) = match (
                arg_values.first(),
                arg_values.get(1),
                arg_values.get(2),
            ) {
                (Some(Value::Image { data, .. }), Some(Value::Int(w)), Some(Value::Int(h))) => {
                    (data, *w as u32, *h as u32)
                }
                _ => return Some(Value::Error("Invalid arguments".to_string())),
            };
            
            let mut img = img_val.lock().unwrap();
            let resized = img.resize(width, height, image::imageops::FilterType::Lanczos3);
            *img = resized;
            
            Value::Str(format!("Resized to {}x{}", width, height))
        }
        
        _ => return None,
    };
    Some(result)
}
```

**Step 4: Register in Dispatcher**

```rust
// src/interpreter/native_functions/mod.rs

mod image;  // Add module

pub fn call_native_function(...) -> Value {
    // ... existing checks
    
    if let Some(result) = image::handle(interp, name, arg_values) {
        return result;
    }
    
    // ...
}
```

---

## Error Handling

### Error Types

**1. Simple Errors**:
```rust
Value::Error("Something went wrong".to_string())
```

**2. Rich Errors** (with stack trace):
```rust
Value::ErrorObject {
    message: "Division by zero".to_string(),
    stack: vec![
        "at divide (math.rs:42)".to_string(),
        "at calculate (main.kujo:10)".to_string(),
    ],
    line: Some(42),
    cause: None,
}
```

### Error Best Practices

**1. Validate Arguments First**:
```rust
"divide" => {
    // Check argument count
    if arg_values.len() != 2 {
        return Some(Value::Error(format!(
            "divide expects 2 arguments, got {}",
            arg_values.len()
        )));
    }
    
    // Check argument types
    let (a, b) = match (arg_values[0].clone(), arg_values[1].clone()) {
        (Value::Int(x), Value::Int(y)) => (x, y),
        _ => return Some(Value::Error("divide expects integers".to_string())),
    };
    
    // Check for errors (division by zero)
    if b == 0 {
        return Some(Value::Error("Division by zero".to_string()));
    }
    
    Value::Int(a / b)
}
```

**2. Use Result Types**:
```rust
fn parse_int(s: &str) -> Result<i64, String> {
    s.parse().map_err(|e| format!("Parse error: {}", e))
}

"parse_int" => {
    let s = match arg_values.first() {
        Some(Value::Str(s)) => s,
        _ => return Some(Value::Error("Expected string".to_string())),
    };
    
    match parse_int(s) {
        Ok(n) => Value::Int(n),
        Err(e) => Value::Error(e),
    }
}
```

**3. Provide Helpful Error Messages**:
```rust
// Bad
Value::Error("Invalid".to_string())

// Good
Value::Error(format!(
    "substring: index {} out of bounds (string length: {})",
    index, string.len()
))
```

---

## Testing Native Functions

### Integration Tests

Add tests in `src/interpreter/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_custom_function() {
        let code = r#"
            result := custom_function(arg1, arg2)
            print(result)
        "#;
        
        let mut interp = Interpreter::new();
        interp.run(code);
        
        // Check expected output or state
    }
}
```

### Unit Tests in Module

```rust
// src/interpreter/native_functions/math.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_factorial() {
        // Test factorial(5)
        let result = handle("factorial", &[Value::Int(5)]);
        assert_eq!(result, Some(Value::Int(120)));
        
        // Test factorial(0)
        let result = handle("factorial", &[Value::Int(0)]);
        assert_eq!(result, Some(Value::Int(1)));
        
        // Test negative (should error)
        let result = handle("factorial", &[Value::Int(-1)]);
        assert!(matches!(result, Some(Value::Error(_))));
    }
}
```

### Example File Tests

Create `.kujo` file in `examples/`:

```kujo
# examples/factorial_demo.kujo

print("Factorial Examples:")
print(factorial(5))
print(factorial(10))
print(factorial(0))
```

Run:
```bash
cargo run -- run examples/factorial_demo.kujo
```

---

## Best Practices

### 1. Return `Option<Value>`

Always return `Option<Value>`:
- `Some(value)` if function was handled
- `None` if function name not recognized

**Why**: Allows dispatcher to try other modules.

### 2. Validate Arguments

Check count, types, and constraints:

```rust
"sqrt" => {
    // Check count
    if arg_values.len() != 1 {
        return Some(Value::Error("sqrt requires 1 argument".to_string()));
    }
    
    // Check type
    let n = match arg_values[0] {
        Value::Int(n) => n as f64,
        Value::Float(n) => n,
        _ => return Some(Value::Error("sqrt expects number".to_string())),
    };
    
    // Check constraint
    if n < 0.0 {
        return Some(Value::Error("sqrt of negative number".to_string()));
    }
    
    Value::Float(n.sqrt())
}
```

### 3. Use Descriptive Error Messages

```rust
// Bad
Value::Error("Error".to_string())

// Good
Value::Error(format!(
    "file_read: failed to read '{}': {}",
    path, error_message
))
```

### 4. Follow Naming Conventions

- `snake_case` for function names
- Clear, verb-based names: `read_file`, `parse_json`, `send_http`
- Consistent prefixes for categories: `http_get`, `http_post`, `http_delete`

### 5. Document Functions

Add doc comments in the module:

```rust
/// Handle math-related function calls
/// 
/// Functions:
/// - `abs(n)`: Absolute value
/// - `sqrt(n)`: Square root
/// - `pow(base, exp)`: Power
/// - `factorial(n)`: Factorial (n!)
pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    // ...
}
```

### 6. Minimize Interpreter Access

Only request `&mut Interpreter` if you need it:

```rust
// Good: No interpreter needed
pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value>

// Good: Interpreter needed for callbacks
pub fn handle(interp: &mut Interpreter, name: &str, arg_values: &[Value]) -> Option<Value>
```

### 7. Use Thread-Safe Types

For stateful values, use `Arc<Mutex<T>>`:

```rust
Value::File(Arc<Mutex<File>>, path)
Value::Database { connection: Arc<Mutex<Connection>>, ... }
```

---

## Examples

### Example 1: Simple Math Function

```rust
// src/interpreter/native_functions/math.rs

"clamp" => {
    // clamp(value, min, max)
    if arg_values.len() != 3 {
        return Some(Value::Error("clamp requires 3 arguments".to_string()));
    }
    
    let value = match arg_values[0] {
        Value::Int(n) => n as f64,
        Value::Float(n) => n,
        _ => return Some(Value::Error("Expected number".to_string())),
    };
    
    let min = match arg_values[1] {
        Value::Int(n) => n as f64,
        Value::Float(n) => n,
        _ => return Some(Value::Error("Expected number".to_string())),
    };
    
    let max = match arg_values[2] {
        Value::Int(n) => n as f64,
        Value::Float(n) => n,
        _ => return Some(Value::Error("Expected number".to_string())),
    };
    
    let clamped = value.max(min).min(max);
    Value::Float(clamped)
}
```

**Usage**:
```kujo
result := clamp(15, 0, 10)
print(result)  # 10

result := clamp(-5, 0, 10)
print(result)  # 0

result := clamp(5, 0, 10)
print(result)  # 5
```

### Example 2: String Function

```rust
// src/interpreter/native_functions/strings.rs

"reverse" => {
    let s = match arg_values.first() {
        Some(Value::Str(s)) => s,
        _ => return Some(Value::Error("reverse expects string".to_string())),
    };
    
    let reversed: String = s.chars().rev().collect();
    Value::Str(reversed)
}
```

**Usage**:
```kujo
result := reverse("hello")
print(result)  # "olleh"
```

### Example 3: Array Function with Callback

```rust
// src/interpreter/native_functions/collections.rs

"filter" => {
    // arr.filter(predicate_fn)
    if let (Some(Value::Array(arr)), Some(func)) = 
        (arg_values.first(), arg_values.get(1)) {
        
        let mut filtered = Vec::new();
        for item in arr {
            // Call predicate function
            let result = interp.call_function_value(
                func.clone(),
                vec![item.clone()]
            );
            
            // Keep item if predicate returns truthy value
            if matches!(result, Value::Bool(true) | Value::Int(n) if n != 0) {
                filtered.push(item.clone());
            }
        }
        Value::Array(filtered)
    } else {
        Value::Error("filter requires array and function".to_string())
    }
}
```

**Usage**:
```kujo
nums := [1, 2, 3, 4, 5, 6]
evens := nums.filter(func(x) { return x % 2 == 0 })
print(evens)  # [2, 4, 6]
```

---

## Performance Tips

1. **Avoid Cloning Large Structures**: Pass references when possible
2. **Use `&[Value]` Slices**: Avoid allocating new Vecs
3. **Cache Computed Values**: Store expensive results
4. **Use Rust's Standard Library**: It's optimized
5. **Profile Native Functions**: Use `cargo flamegraph` to find bottlenecks

---

## Further Reading

- [ARCHITECTURE.md](ARCHITECTURE.md) - System overview
- [MEMORY.md](MEMORY.md) - Memory management
- [CONCURRENCY.md](CONCURRENCY.md) - Thread safety
- [src/interpreter/native_functions/](../src/interpreter/native_functions/) - Example implementations

---

**Questions?** Open an issue on GitHub or check the documentation.
