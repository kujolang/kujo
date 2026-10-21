// File: src/bytecode.rs
//
// Bytecode instruction definitions and structures for the Kujo VM.
// Defines OpCode enum representing all bytecode instructions and supporting types.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BytecodeBindingKind {
    Mutable,
    LetImmutable,
    Const,
}

/// Definition-site source of a v1 snapshot capture. Slots identify bindings,
/// never the last occurrence of a name in a completed function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureSource {
    Local(usize),
    Upvalue(usize),
    /// Compatibility for runtime-created named bindings (bare assignments,
    /// destructuring and legacy receiver fields). Globals are not captured.
    Named(String),
    /// A lexically scoped script binding, snapshotted before its scope exits.
    Environment(String, BytecodeBindingKind),
}

/// A binding whose value can be changed after all observable expressions run.
#[derive(Debug, Clone, PartialEq)]
pub enum MutationBinding {
    Local(usize),
    Named(String),
}

/// Bytecode instruction opcodes for the Kujo VM
/// Stack-based virtual machine with separate value and call stacks
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Many opcodes not yet used - VM is work in progress
pub enum OpCode {
    // === Stack Operations ===
    /// Load a constant from the constant pool onto the stack
    /// Operand: constant pool index
    LoadConst(usize),

    /// Load a variable value onto the stack
    /// Operand: variable name
    LoadVar(String),

    /// Read/write an explicitly resolved, frame-owned snapshot cell.
    LoadCapture(usize),
    StoreCapture(usize),

    /// Load a local variable value onto the stack
    /// Operand: local slot index
    LoadLocal(usize),

    /// Load a global variable value onto the stack
    /// Operand: variable name
    LoadGlobal(String),

    /// Store top of stack to a variable (creates or updates)
    /// Operand: variable name
    StoreVar(String),

    /// Store top of stack to a local variable slot
    /// Operand: local slot index
    StoreLocal(usize),

    /// Initialize a fresh lexical binding, including each loop iteration.
    DefineLocal(usize),

    /// Store top of stack to a global variable
    /// Operand: variable name
    StoreGlobal(String),

    /// Define a global binding with explicit mutability metadata.
    /// Operand: variable name and binding kind.
    DefineGlobal(String, BytecodeBindingKind),

    /// Ensure a global binding allows in-place mutation.
    /// Operand: variable name.
    EnsureMutableGlobalForMutation(String),

    /// Pop top value from stack (discard result)
    Pop,

    /// Duplicate top value on stack
    Dup,

    // === Arithmetic Operations ===
    /// Pop two values, add them, push result
    Add,

    /// Add to variable in-place (avoids LoadVar/StoreVar)
    /// Operand: local slot index
    /// Stack: [rhs] -> [result]
    AddInPlace(usize),

    /// Append constant string to local string in-place
    /// Operand: local slot index and constant string payload
    /// Stack: [] -> []
    AppendConstStringInPlace(usize, Arc<str>),

    /// Append constant character to local string in-place
    /// Operand: local slot index and character payload
    /// Stack: [] -> []
    AppendConstCharInPlace(usize, char),

    /// Append constant character repeatedly while index < limit, then set index = limit
    /// Operand: target string slot, index slot, limit slot, character payload
    /// Stack: [] -> []
    AppendConstCharUntilLocalInPlace(usize, usize, usize, char),

    /// Accumulate map[index] into sum while index < limit, then set index = limit
    /// Operand: map slot, sum slot, index slot, limit slot
    /// Stack: [] -> []
    SumIntMapUntilLocalInPlace(usize, usize, usize, usize),

    /// Set map[index] = index * 2 while index < limit, then set index = limit
    /// Operand: map slot, index slot, limit slot
    /// Stack: [] -> []
    FillIntMapWithDoubleUntilLocalInPlace(usize, usize, usize),

    /// Pop two values, subtract (top from second), push result
    Sub,

    /// Pop two values, multiply them, push result
    Mul,

    /// Pop two values, divide (second by top), push result
    Div,

    /// Pop two values, modulo (second % top), push result
    Mod,

    /// Pop one value, negate it, push result
    Negate,

    // === Comparison Operations ===
    /// Pop two values, compare equal, push bool result
    Equal,

    /// Pop two values, compare not equal, push bool result
    NotEqual,

    /// Pop two values, compare less than, push bool result
    LessThan,

    /// Pop two values, compare greater than, push bool result
    GreaterThan,

    /// Pop two values, compare less than or equal, push bool result
    LessEqual,

    /// Pop two values, compare greater than or equal, push bool result
    GreaterEqual,

    // === Logical Operations ===
    /// Pop one value, logical NOT, push result
    Not,

    /// Pop two values, logical AND (short-circuit handled by jumps)
    And,

    /// Pop two values, logical OR (short-circuit handled by jumps)
    Or,

    // === Control Flow ===
    /// Unconditional jump to instruction
    /// Operand: target instruction index
    Jump(usize),

    /// Pop value, jump if false
    /// Operand: target instruction index
    JumpIfFalse(usize),

    /// Consume iterable and index; push one item or jump to exhausted target.
    /// Generators resume once, so break never drains a remaining continuation.
    ForNext(usize),

    /// Pop value, jump if true
    /// Operand: target instruction index
    JumpIfTrue(usize),

    /// Jump backwards (for loops)
    /// Operand: target instruction index
    JumpBack(usize),

    // === Function Operations ===
    /// Call a function with N arguments
    /// Arguments are already on stack (bottom to top)
    /// Operand: number of arguments
    Call(usize),

    /// Return from function with value on stack
    Return,

    /// Return None/null from function (for void functions)
    ReturnNone,

    /// Create a closure and push onto stack
    /// Operand: constant pool index (contains function code)
    MakeClosure(usize),

    // === Collection Operations ===
    /// Create an array from N values on stack
    /// Operand: number of elements (excluding spread expansions)
    /// Note: SpreadArray operations expand before this, so actual count may differ
    MakeArray(usize),

    /// Create an array by collecting stack values until ArrayMarker
    /// Used for array literals that include spread elements
    MakeArrayFromMarker,

    /// Push a marker value onto the stack (for dynamic collection)
    PushArrayMarker,

    /// Create a dict from 2N values on stack (key1, val1, key2, val2, ...)
    /// Operand: number of key-value pairs
    MakeDict(usize),

    /// Create a dict by collecting key/value pairs until marker sentinel pair
    /// Used for dict literals that include spread elements
    MakeDictFromMarker,

    /// Create a dict from N values on stack with constant string keys
    /// Operand: ordered list of string keys
    /// Stack: [val1, val2, ...] -> [dict]
    MakeDictWithKeys(Arc<Vec<Arc<str>>>),

    /// Pop index and object, push object[index]
    IndexGet,

    /// Pop value, index, and object, set object[index] = value
    IndexSet,

    /// Pop object, push object.field
    /// Operand: field name
    FieldGet(String),

    /// Pop value and object, set object.field = value
    /// Operand: field name
    FieldSet(String),

    /// Update a binding-rooted nested path on an owned root value.
    /// Some(field) selects a struct field; None consumes a dynamic index.
    /// Stack: [value, indices in path order..., root] -> [updated root].
    UpdatePath(Vec<Option<String>>),
    /// Stack: [value, indices...] -> [null]. Validate before borrowing for writing.
    UpdateBindingPath(MutationBinding, Vec<Option<String>>),
    /// Conditional fast path for x = f(x, item). On success consume three call
    /// operands and jump past call/store; otherwise leave the stack unchanged.
    TryAppendAssign(MutationBinding, usize),
    /// Conditional string x = x + rhs. Preserve evaluated operands on fallback.
    TryStringAppendAssign(MutationBinding, usize),
    /// Drop an unobservable caller-local snapshot after evaluating all operands.
    ReleaseRebindLocal(usize, usize),
    /// Release a proven dead parameter after loading its initial local alias.
    ReleaseDeadLocal(usize),
    /// Check a native append call before evaluating assignment indices.
    GuardAppendPath(usize),
    /// Conditional pop/remove followed by rebinding: binding, dead pair slot,
    /// argument count, success jump. Other callables use ordinary call semantics.
    TryPopRebind(MutationBinding, usize, usize, Vec<Option<String>>, usize),
    /// Stack: [array, item, native, indices...] -> [null].
    AppendBindingPath(MutationBinding, Vec<Option<String>>),

    /// Get value from local variable dict/array in-place (no cloning)
    /// Operand: local slot index
    /// Stack: [index] -> [value]
    /// Optimized version of LoadVar + IndexGet for local variables
    IndexGetInPlace(usize),

    /// Set value in local variable dict/array in-place (no cloning)
    /// Operand: local slot index
    /// Stack: [index, value] -> []
    /// Optimized version of LoadVar + IndexSet + StoreVar for local variables
    IndexSetInPlace(usize),

    // === Spread Operations ===
    /// Spread an array/dict onto stack for collection construction
    /// Pops one collection, pushes all its elements
    SpreadArray,

    /// Spread an array as function arguments
    SpreadArgs,

    /// Spread a dict into another dict (merge operation)
    SpreadDict,

    // === Pattern Matching ===
    /// Match a value against a pattern and bind variables
    /// Stack: [value] -> [success: bool]
    /// If match succeeds, variables are bound in current scope
    /// Operand: pattern index in constant pool and binding kind for introduced names
    MatchPattern(usize, BytecodeBindingKind),

    /// Match a value against a `match case` pattern string, with optional binding support
    /// Stack: [value] -> [success: bool]
    /// If match succeeds and the case has a binding (e.g. `Result::Ok(v)`), bind it
    /// Operand: match-case pattern string
    MatchCasePattern(String),

    /// Start a new match case branch
    /// Used for organizing match statement bytecode
    BeginCase,

    /// End a match case branch
    EndCase,

    // === Result/Option Operations ===
    /// Create an Ok(value) Result
    /// Stack: [value] -> [Result::Ok(value)]
    MakeOk,

    /// Create an Err(value) Result  
    /// Stack: [value] -> [Result::Err(value)]
    MakeErr,

    /// Create a Some(value) Option
    /// Stack: [value] -> [Option::Some(value)]
    MakeSome,

    /// Create a None Option
    /// Stack: [] -> [Option::None]
    MakeNone,

    /// Try operator: propagate errors or unwrap success
    /// If Result::Err or Option::None, early return with that value
    /// Otherwise, unwrap the inner value
    /// Stack: [Result/Option] -> [inner_value]
    TryUnwrap,

    // === Struct Operations ===
    /// Create a struct instance from N field values on stack
    /// Operand: (struct_name, field_names)
    MakeStruct(String, Vec<String>),

    // === Environment Management ===
    /// Push a new scope (for blocks, functions)
    PushScope,

    /// Pop a scope
    PopScope,

    // === Iterator Operations ===
    /// Create an iterator from a collection
    /// Stack: [collection] -> [iterator]
    MakeIterator,

    /// Get the next value from an iterator
    /// Stack: [iterator] -> [iterator, Some(value)] or [iterator, None]
    IteratorNext,

    /// Check if iterator has more values
    /// Stack: [iterator] -> [iterator, bool]
    IteratorHasNext,

    // === Generator Operations ===
    /// Yield a value from a generator function
    /// Saves current execution state (IP, stack, locals) and returns value
    /// Stack: [value] -> [] (generator returns to caller)
    Yield,

    /// Resume a generator from a yield point
    /// Restores execution state and continues from where it yielded
    /// Stack: [] -> [value] (value that was yielded)
    ResumeGenerator,

    /// Create a generator object from a function
    /// Stack: [function] -> [generator]
    MakeGenerator,

    /// Submit a detached callable after strict capture-transfer preflight.
    SpawnDetached,

    // === Async/Await Operations ===
    /// Await a promise/async value
    /// Suspends execution until the promise resolves
    /// Stack: [promise] -> [resolved_value]
    Await,

    /// Create a promise that resolves with a value
    /// Stack: [value] -> [promise]
    MakePromise,

    /// Mark function as async (used during compilation)
    /// Not executed at runtime, used for function metadata
    #[allow(dead_code)]
    MarkAsync,

    // === Exception Handling ===
    /// Begin a try block
    /// Sets up exception handler at the given instruction
    /// Operand: catch block instruction index
    BeginTry(usize),

    /// End a try block
    /// Removes the exception handler from the stack
    EndTry,

    /// Throw an exception
    /// Stack: [error_value] -> unwinds to nearest catch handler
    Throw,

    /// Begin a catch block
    /// Binds the caught exception to a variable
    /// Operand: variable name for exception
    BeginCatch(String),

    /// End a catch block
    EndCatch,

    // === Native Function Calls ===
    /// Call a native (built-in) function by name
    /// Arguments are already on stack
    /// Operand: (function_name, arg_count)
    CallNative(String, usize),

    // === Closure & Upvalue Operations ===
    /// Capture an upvalue (closed-over variable)
    /// Stack: [] -> [upvalue]
    /// Operand: variable name to capture
    CaptureUpvalue(String),

    /// Load an upvalue onto the stack
    /// Stack: [] -> [value]
    /// Operand: upvalue index
    LoadUpvalue(usize),

    /// Store top of stack to an upvalue
    /// Stack: [value] -> []
    /// Operand: upvalue index
    StoreUpvalue(usize),

    /// Close upvalues up to the stack slot
    /// Moves upvalues from stack to heap when they go out of scope
    /// Operand: stack slot index
    CloseUpvalues(usize),

    // === Channel Operations (for concurrency) ===
    /// Create a new channel for communication
    /// Stack: [] -> [channel]
    MakeChannel,

    /// Send a value through a channel
    /// Stack: [channel, value] -> []
    ChannelSend,

    /// Receive a value from a channel (blocking)
    /// Stack: [channel] -> [value]
    ChannelRecv,

    // === Debugging ===
    /// Print current stack state (for debugging)
    #[allow(dead_code)]
    DebugStack,

    /// Print a message for debugging
    /// Operand: debug message
    #[allow(dead_code)]
    DebugPrint(String),

    /// No operation (placeholder)
    Nop,
}

/// Compiled code is shared by callable values and suspended frames. Compiler
/// edits and generator flag changes detach storage before mutation.
#[derive(Debug, Clone, PartialEq)]
pub struct BytecodeChunk(Arc<BytecodeStorage>);

#[derive(Debug, Clone)]
struct BytecodeStorage {
    data: BytecodeData,
    named_bindings: OnceLock<HashSet<String>>,
}

impl PartialEq for BytecodeStorage {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl std::ops::Deref for BytecodeChunk {
    type Target = BytecodeData;

    fn deref(&self) -> &Self::Target {
        &self.0.data
    }
}

impl std::ops::DerefMut for BytecodeChunk {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let storage = Arc::make_mut(&mut self.0);
        storage.named_bindings.take();
        &mut storage.data
    }
}

/// The copy-on-write contents of a compiled bytecode chunk.
#[derive(Debug, Clone, PartialEq)]
pub struct BytecodeData {
    /// The bytecode instructions
    pub instructions: Vec<OpCode>,

    /// Constant pool containing literals and other constants
    pub constants: Vec<Constant>,

    /// Source location mapping for error reporting
    /// Maps instruction index to (line, column)
    pub source_map: HashMap<usize, (usize, usize)>,

    /// Optional name (for functions)
    pub name: Option<String>,

    /// Parameter names for functions
    pub params: Vec<String>,

    /// Local variable names by slot index
    pub local_names: Vec<String>,

    /// Local binding kinds by slot index.
    pub local_binding_kinds: Vec<BytecodeBindingKind>,

    /// Number of local slots allocated for this chunk
    pub local_count: usize,

    /// Exception handlers for try/catch blocks
    pub exception_handlers: Vec<ExceptionHandler>,

    /// Upvalue names for closures (variables captured from outer scope)
    pub upvalues: Vec<String>,

    /// Explicit sources for compiler-produced captures; None supports legacy chunks.
    pub capture_sources: Option<Vec<CaptureSource>>,

    /// Nested named function self-binding, installed only for an active call.
    pub lexical_self: Option<String>,

    /// Whether this is a generator function
    pub is_generator: bool,

    /// Whether this is an async function
    pub is_async: bool,
}

#[allow(dead_code)] // Methods not yet used - VM integration incomplete
impl BytecodeChunk {
    pub fn new() -> Self {
        Self(Arc::new(BytecodeStorage {
            data: BytecodeData {
                instructions: Vec::new(),
                constants: Vec::new(),
                source_map: HashMap::new(),
                name: None,
                params: Vec::new(),
                local_names: Vec::new(),
                local_binding_kinds: Vec::new(),
                local_count: 0,
                exception_handlers: Vec::new(),
                upvalues: Vec::new(),
                capture_sources: None,
                lexical_self: None,
                is_generator: false,
                is_async: false,
            },
            named_bindings: OnceLock::new(),
        }))
    }

    /// Legacy hand-built chunks can address a slotted parameter by name. Scan
    /// once per immutable code allocation, not once per argument on every call.
    pub(crate) fn uses_named_binding(&self, name: &str) -> bool {
        self.0
            .named_bindings
            .get_or_init(|| {
                self.instructions
                    .iter()
                    .filter_map(|op| match op {
                        OpCode::LoadVar(name) | OpCode::StoreVar(name) => Some(name.clone()),
                        _ => None,
                    })
                    .collect()
            })
            .contains(name)
    }

    /// Add a constant to the pool and return its index
    pub fn add_constant(&mut self, constant: Constant) -> usize {
        // Check if constant already exists (simple optimization)
        if let Some(index) = self.constants.iter().position(|c| match (c, &constant) {
            // Numeric equality merges signed zeros; constant storage must not.
            (Constant::Float(left), Constant::Float(right)) => left.to_bits() == right.to_bits(),
            _ => c == &constant,
        }) {
            return index;
        }

        let index = self.constants.len();
        self.constants.push(constant);
        index
    }

    /// Emit an instruction and return its index
    pub fn emit(&mut self, instruction: OpCode) -> usize {
        let index = self.instructions.len();
        self.instructions.push(instruction);
        index
    }

    /// Patch a jump instruction at the given index with the current position
    pub fn patch_jump(&mut self, jump_index: usize) {
        let target = self.instructions.len();
        match &mut self.instructions[jump_index] {
            OpCode::Jump(ref mut addr)
            | OpCode::JumpIfFalse(ref mut addr)
            | OpCode::ForNext(ref mut addr)
            | OpCode::TryAppendAssign(_, ref mut addr)
            | OpCode::TryStringAppendAssign(_, ref mut addr)
            | OpCode::GuardAppendPath(ref mut addr)
            | OpCode::TryPopRebind(_, _, _, _, ref mut addr)
            | OpCode::JumpIfTrue(ref mut addr) => {
                *addr = target;
            }
            _ => panic!("Attempted to patch non-jump instruction"),
        }
    }

    /// Set the jump target for a jump instruction
    pub fn set_jump_target(&mut self, jump_index: usize, target: usize) {
        match &mut self.instructions[jump_index] {
            OpCode::Jump(ref mut addr)
            | OpCode::JumpIfFalse(ref mut addr)
            | OpCode::ForNext(ref mut addr)
            | OpCode::JumpIfTrue(ref mut addr)
            | OpCode::JumpBack(ref mut addr)
            | OpCode::TryAppendAssign(_, ref mut addr)
            | OpCode::TryStringAppendAssign(_, ref mut addr)
            | OpCode::GuardAppendPath(ref mut addr)
            | OpCode::TryPopRebind(_, _, _, _, ref mut addr)
            | OpCode::BeginTry(ref mut addr) => {
                *addr = target;
            }
            _ => panic!("Attempted to set target on non-jump instruction"),
        }
    }
}

/// Constants that can be stored in the constant pool
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Not all variants used yet - VM is work in progress
pub enum Constant {
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Bool(bool),
    None,
    /// A compiled function (stored as bytecode chunk)
    Function(Box<BytecodeChunk>),
    /// Pattern for matching (stored AST pattern)
    Pattern(crate::ast::Pattern),
    /// Type annotation for runtime type checking
    Type(crate::ast::TypeAnnotation),
    /// Array of constants (for nested structures)
    Array(Vec<Constant>),
    /// Dict of constants (key-value pairs)
    Dict(Vec<(Constant, Constant)>),
}

/// Represents a compiled function with its bytecode and metadata
#[derive(Debug, Clone)]
#[allow(dead_code)] // Not yet used - function compilation incomplete
pub struct CompiledFunction {
    pub name: String,
    pub arity: usize, // Number of parameters
    pub chunk: BytecodeChunk,
    pub upvalues: Vec<String>, // Captured variables for closures
    pub is_generator: bool,
    pub is_async: bool,
}

/// Exception handler entry for try/catch
#[derive(Debug, Clone, PartialEq)]
pub struct ExceptionHandler {
    /// Start of try block (instruction index)
    pub try_start: usize,
    /// End of try block (instruction index)
    pub try_end: usize,
    /// Start of catch block (instruction index)
    pub catch_start: usize,
    /// Variable name to bind caught exception
    pub exception_var: String,
}

#[cfg(test)]
mod sharing_tests {
    use super::*;

    #[test]
    fn named_binding_cache_is_shared_invalidated_and_not_part_of_code_equality() {
        let mut original = BytecodeChunk::new();
        original.emit(OpCode::LoadVar("x".into()));
        let mut edited = original.clone();
        assert!(original.uses_named_binding("x"));
        assert!(edited.0.named_bindings.get().is_some());
        edited.instructions[0] = OpCode::LoadVar("y".into());
        assert!(edited.0.named_bindings.get().is_none());
        assert!(!edited.uses_named_binding("x"));
        assert!(edited.uses_named_binding("y"));
        assert!(original.uses_named_binding("x"));
        assert!(!original.uses_named_binding("y"));
        edited.instructions[0] = OpCode::LoadVar("x".into());
        assert_eq!(original, edited);
        // Unique mutation must invalidate metadata too.
        edited.instructions.clear();
        assert!(!edited.uses_named_binding("x"));
    }

    #[test]
    fn bytecode_clones_share_storage_and_edits_detach() {
        let mut original = BytecodeChunk::new();
        original.emit(OpCode::Return);
        original.add_constant(Constant::Int(42));
        original.source_map.insert(0, (1, 1));
        let mut copy = original.clone();
        assert!(Arc::ptr_eq(&original.0, &copy.0));
        copy.is_generator = true;
        copy.instructions[0] = OpCode::Nop;
        copy.constants[0] = Constant::Int(7);
        copy.source_map.insert(0, (2, 3));
        assert!(!Arc::ptr_eq(&original.0, &copy.0));
        assert!(!original.is_generator);
        assert_eq!(original.instructions, vec![OpCode::Return]);
        assert_eq!(original.constants, vec![Constant::Int(42)]);
        assert_eq!(original.source_map[&0], (1, 1));
    }

    #[test]
    fn editing_nested_bytecode_does_not_change_callable_snapshots() {
        let mut child = BytecodeChunk::new();
        child.emit(OpCode::Return);
        let mut parent = BytecodeChunk::new();
        parent.add_constant(Constant::Function(Box::new(child.clone())));
        let snapshot = parent.clone();
        let Constant::Function(nested) = &mut parent.constants[0] else { panic!() };
        nested.emit(OpCode::Nop);
        let Constant::Function(saved) = &snapshot.constants[0] else { panic!() };
        assert!(Arc::ptr_eq(&saved.0, &child.0));
        assert_eq!(saved.instructions.len(), 1);
        assert_eq!(nested.instructions.len(), 2);
    }
}
