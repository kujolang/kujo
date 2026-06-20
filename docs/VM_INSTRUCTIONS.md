# Kujo VM Instruction Set Reference

This document provides a comprehensive reference for all bytecode instructions in the Kujo Virtual Machine.

## Overview

The Kujo VM is a stack-based virtual machine that executes bytecode instructions. Instructions operate on:
- **Value Stack**: Stores temporary computation results
- **Call Stack**: Manages function call frames
- **Global Environment**: Stores global variables
- **Local Variables**: Stored in call frames

## Notation

- `[a, b, c]` - Stack with `a` at bottom, `c` at top
- `[... , x]` - Stack with `x` at top
- `[] -> [value]` - Pops nothing, pushes `value`
- `[a, b] -> [result]` - Pops `a` and `b`, pushes `result`

## Instruction Categories

### Stack Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `LoadConst(index)` | constant pool index | `[] -> [value]` | Load a constant from the constant pool |
| `LoadVar(name)` | variable name | `[] -> [value]` | Load a local or global variable |
| `LoadGlobal(name)` | variable name | `[] -> [value]` | Load a global variable |
| `StoreVar(name)` | variable name | `[value] -> []` | Store to local variable (or global if no local scope) |
| `StoreGlobal(name)` | variable name | `[value] -> []` | Store to global variable |
| `Pop` | none | `[value] -> []` | Discard top value from stack |
| `Dup` | none | `[value] -> [value, value]` | Duplicate top value |

### Arithmetic Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `Add` | none | `[a, b] -> [a + b]` | Add two values (int, float, or string concat) |
| `Sub` | none | `[a, b] -> [a - b]` | Subtract b from a |
| `Mul` | none | `[a, b] -> [a * b]` | Multiply two values |
| `Div` | none | `[a, b] -> [a / b]` | Divide a by b |
| `Mod` | none | `[a, b] -> [a % b]` | Modulo operation |
| `Negate` | none | `[a] -> [-a]` | Negate a number |

### Comparison Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `Equal` | none | `[a, b] -> [bool]` | Check if a == b |
| `NotEqual` | none | `[a, b] -> [bool]` | Check if a != b |
| `LessThan` | none | `[a, b] -> [bool]` | Check if a < b |
| `GreaterThan` | none | `[a, b] -> [bool]` | Check if a > b |
| `LessEqual` | none | `[a, b] -> [bool]` | Check if a <= b |
| `GreaterEqual` | none | `[a, b] -> [bool]` | Check if a >= b |

### Logical Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `Not` | none | `[value] -> [bool]` | Logical NOT |
| `And` | none | `[a, b] -> [bool]` | Logical AND (note: short-circuit handled by jumps) |
| `Or` | none | `[a, b] -> [bool]` | Logical OR (note: short-circuit handled by jumps) |

### Control Flow

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `Jump(target)` | instruction index | no change | Unconditional jump to target |
| `JumpIfFalse(target)` | instruction index | `[condition] -> [condition]` | Jump if top is false (leaves value on stack) |
| `JumpIfTrue(target)` | instruction index | `[condition] -> [condition]` | Jump if top is true (leaves value on stack) |
| `JumpBack(target)` | instruction index | no change | Jump backwards (for loops) |

### Function Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `Call(argc)` | argument count | `[arg1, ..., argN, fn] -> [result]` | Call function with N arguments |
| `CallNative(name, argc)` | function name, arg count | `[arg1, ..., argN] -> [result]` | Call native built-in function |
| `Return` | none | `[value] -> (returns from function)` | Return value from function |
| `ReturnNone` | none | `[] -> (returns None)` | Return None from function |
| `MakeClosure(index)` | function constant index | `[] -> [closure]` | Create a closure from function |

### Collection Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `MakeArray(count)` | element count | `[e1, ..., eN] -> [array]` | Create array from N elements |
| `PushArrayMarker` | none | `[] -> [marker]` | Push array marker for dynamic collection |
| `MakeDict(count)` | pair count | `[k1, v1, ..., kN, vN] -> [dict]` | Create dict from N key-value pairs |
| `IndexGet` | none | `[object, index] -> [value]` | Get value at index |
| `IndexSet` | none | `[value, object, index] -> []` | Set value at index |
| `FieldGet(name)` | field name | `[object] -> [value]` | Get object field |
| `FieldSet(name)` | field name | `[value, object] -> []` | Set object field |

### Spread Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `SpreadArray` | none | `[array] -> [e1, ..., eN]` | Spread array elements onto stack |
| `SpreadArgs` | none | `[array] -> [a1, ..., aN]` | Spread array as function arguments |
| `SpreadDict` | none | `[dict] -> (merges into parent dict)` | Spread dict for merging |

### Pattern Matching

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `MatchPattern(index)` | pattern index | `[value] -> [bool]` | Match value against pattern, bind variables |
| `BeginCase` | none | no change | Mark start of match case |
| `EndCase` | none | no change | Mark end of match case |

### Result/Option Types

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `MakeOk` | none | `[value] -> [Result::Ok(value)]` | Create Ok Result |
| `MakeErr` | none | `[error] -> [Result::Err(error)]` | Create Err Result |
| `MakeSome` | none | `[value] -> [Option::Some(value)]` | Create Some Option |
| `MakeNone` | none | `[] -> [Option::None]` | Create None Option |
| `TryUnwrap` | none | `[Result/Option] -> [value]` | Unwrap or early return |

### Struct Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `MakeStruct(name, fields)` | struct name, field names | `[v1, ..., vN] -> [struct]` | Create struct from field values |

### Environment Management

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `PushScope` | none | no change | Enter new lexical scope |
| `PopScope` | none | no change | Exit lexical scope |

### Iterator Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `MakeIterator` | none | `[collection] -> [iterator]` | Create iterator from collection |
| `IteratorNext` | none | `[iterator] -> [iterator, Some(value)/None]` | Get next value |
| `IteratorHasNext` | none | `[iterator] -> [iterator, bool]` | Check if more values |

### Generator Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `Yield` | none | `[value] -> (suspends)` | Yield value from generator |
| `ResumeGenerator` | none | `[] -> [value]` | Resume generator execution |
| `MakeGenerator` | none | `[function] -> [generator]` | Create generator object |

### Async/Await Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `Await` | none | `[promise] -> [value]` | Await promise resolution |
| `MakePromise` | none | `[value] -> [promise]` | Create resolved promise |

### Exception Handling

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `BeginTry(handler)` | catch block address | no change | Set up exception handler |
| `EndTry` | none | no change | Remove exception handler |
| `Throw` | none | `[error] -> (unwinds)` | Throw exception |
| `BeginCatch(var)` | exception variable name | `[error] -> []` | Begin catch block, bind exception |
| `EndCatch` | none | no change | End catch block |

### Closure & Upvalue Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `CaptureUpvalue(name)` | variable name | `[] -> [upvalue]` | Capture variable as upvalue |
| `LoadUpvalue(index)` | upvalue index | `[] -> [value]` | Load upvalue |
| `StoreUpvalue(index)` | upvalue index | `[value] -> []` | Store to upvalue |
| `CloseUpvalues(slot)` | stack slot | no change | Move upvalues to heap |

### Channel Operations

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `MakeChannel` | none | `[] -> [channel]` | Create communication channel |
| `ChannelSend` | none | `[channel, value] -> []` | Send value through channel |
| `ChannelRecv` | none | `[channel] -> [value]` | Receive from channel (blocking) |

### Debugging

| Instruction | Operands | Stack Effect | Description |
|------------|----------|--------------|-------------|
| `DebugStack` | none | no change | Print stack state (debug mode) |
| `DebugPrint(msg)` | debug message | no change | Print debug message |
| `Nop` | none | no change | No operation |

## Instruction Encoding

Instructions are represented as Rust enums. Operands are encoded inline:

```rust
pub enum OpCode {
    // Simple instructions (no operands)
    Add,
    Pop,
    
    // Instructions with operands
    LoadConst(usize),           // Index operand
    Jump(usize),                // Address operand
    CallNative(String, usize),  // Name + count operands
}
```

## Constant Pool

The constant pool stores immutable values referenced by instructions:

```rust
pub enum Constant {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    None,
    Function(Box<BytecodeChunk>),  // Nested functions
    Pattern(Pattern),              // Match patterns
    Type(TypeAnnotation),          // Type information
    Array(Vec<Constant>),          // Constant arrays
    Dict(Vec<(Constant, Constant)>), // Constant dicts
}
```

## Exception Handling Model

Exception handlers are stored in an exception table:

```rust
pub struct ExceptionHandler {
    pub try_start: usize,       // Start of try block
    pub try_end: usize,         // End of try block
    pub catch_start: usize,     // Start of catch block
    pub exception_var: String,  // Variable to bind exception
}
```

When an exception is thrown:
1. VM searches exception handlers for current IP
2. Unwinds stack to catch block
3. Binds exception to variable
4. Continues execution at catch block

## Call Frames

Each function call creates a call frame:

```rust
struct CallFrame {
    return_ip: usize,              // Return address
    stack_offset: usize,           // Stack base pointer
    locals: HashMap<String, Value>, // Local variables
    prev_chunk: Option<BytecodeChunk>, // Previous chunk
}
```

## Example: Compiling Simple Function

Source code:
```kujo
func add(a, b) {
    return a + b
}

let result := add(5, 3)
```

Generated bytecode:
```
# Function definition
MakeClosure(0)           # Load function from constant pool
StoreGlobal("add")       # Store in global

# Function call
LoadConst(1)             # Load 5
LoadConst(2)             # Load 3
LoadGlobal("add")        # Load function
Call(2)                  # Call with 2 args
StoreGlobal("result")    # Store result

# Function body (in constant pool[0])
LoadVar("a")             # Load parameter a
LoadVar("b")             # Load parameter b
Add                      # a + b
Return                   # Return result
```

## Performance Characteristics

| Operation Type | Typical Cost |
|---------------|--------------|
| Stack operations | O(1) |
| Variable load/store | O(1) |
| Function call | O(1) |
| Array creation | O(n) where n = element count |
| Pattern matching | O(n) where n = pattern complexity |
| Exception throw | O(d) where d = call stack depth |

## Future Optimizations

This section is historical design context. For the current v1.0
release-readiness posture, the VM is the default runtime and JIT is an
experimental `kujo run --jit` opt-in with deterministic VM fallback for
unsupported surfaces. See `docs/PERFORMANCE.md` and
`docs/VM_INTERPRETER_PARITY_MATRIX.md`.

Phase 2 (Basic Optimizations) planned:
- Constant folding at compile time
- Dead code elimination
- Peephole optimization
- Inline caching for polymorphic operations

Phase 3 (JIT Compilation) planned:
- Hot path detection
- Native code generation
- Type specialization
- Escape analysis

---

**Version**: Phase 1 (Complete VM Integration)  
**Status**: Active Development  
**Last Updated**: January 26, 2026
