// File: src/vm.rs
//
// Virtual Machine for executing Kujo bytecode.
// Stack-based VM with support for function calls, closures, and all Kujo features.

use crate::ast::Pattern;
use crate::bytecode::{BytecodeBindingKind, BytecodeChunk, Constant, OpCode};
use crate::http_request_utils;
use crate::interpreter::{
    BindingKind, CallableArity, DenseIntDict, DenseIntDictInt, DictMap, Environment, IntDictMap,
    Interpreter, NativeCapability, RuntimeCapabilityPolicy, Value,
};
use crate::jit::{
    invoke_compiled_fn, invoke_compiled_fn_with_arg, CompiledFn, CompiledFnInfo, JitCompiler,
    UnsupportedJitSurface,
};
use crate::runtime_limits;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

/// JIT compilation threshold for functions
/// A function will be JIT-compiled after being called this many times
const JIT_FUNCTION_THRESHOLD: usize = 100;
const DENSE_INT_DICT_MIN_CAPACITY: usize = 131072;

/// Stable identifier for a suspendable VM execution context.
pub type VmContextId = u64;

const VM_SUSPEND_ERROR_PREFIX: &str = "__VM_SUSPENDED_CONTEXT__:";

/// Result of cooperative VM execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmExecutionResult {
    Completed,
    Suspended { context_id: VmContextId },
}

/// Result of a single scheduler round over registered execution contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmSchedulerRoundResult {
    /// Number of contexts that completed during this round.
    pub completed_contexts: usize,
    /// Number of contexts that remain pending after this round.
    pub pending_contexts: usize,
}

/// Upvalue: heap-allocated captured variable for closures
#[derive(Debug, Clone)]
/// Upvalue - captured variable for closures
/// Infrastructure for closure variable capture
struct Upvalue {
    /// The captured value
    value: Arc<Mutex<Value>>,

    /// Whether the upvalue is still on the stack (false) or has been closed (true)
    #[allow(dead_code)] // Exposed for regression assertions around closure state.
    is_closed: bool,
}

/// Virtual Machine for executing bytecode
pub struct VM {
    /// Value stack for computation
    stack: Vec<Value>,

    /// Call frames for function calls
    pub(crate) call_frames: Vec<CallFrame>,

    /// Global environment (must be Mutex for interior mutability)
    globals: Arc<Mutex<Environment>>,

    /// Current instruction pointer
    ip: usize,

    /// Current bytecode chunk
    chunk: BytecodeChunk,

    /// Interpreter instance for calling native functions
    interpreter: Interpreter,

    /// Upvalues (captured variables) - indexed by upvalue ID
    /// These are heap-allocated shared references to captured variables
    upvalues: Vec<Upvalue>,

    /// Exception handler stack for try/catch blocks
    /// Tracks nested try blocks and their catch handlers
    exception_handlers: Vec<ExceptionHandlerFrame>,

    /// JIT compiler for hot code paths
    jit_compiler: JitCompiler,

    /// JIT enabled flag (can be disabled for debugging)
    jit_enabled: bool,

    /// Function call stack for error reporting (tracks function names)
    function_call_stack: Vec<String>,

    /// Function call counts for JIT compilation threshold
    /// Maps function name to number of times it has been called
    function_call_counts: HashMap<String, usize>,

    /// Cache of JIT-compiled functions
    /// Maps function name to compiled native code
    compiled_functions: HashMap<String, CompiledFn>,

    /// Enhanced cache with direct-arg variants for recursive functions
    /// Maps function name to CompiledFnInfo (includes both standard and direct-arg)
    compiled_fn_info: HashMap<String, CompiledFnInfo>,

    /// Cache of var_names for JIT-compiled functions
    /// This avoids re-computing hash mappings on every call
    jit_var_names_cache: HashMap<String, HashMap<u64, String>>,

    /// Current recursion depth (for optimization and debugging)
    recursion_depth: usize,

    /// Maximum recursion depth observed (for profiling)
    max_recursion_depth: usize,

    /// Inline cache for function calls at specific call sites
    /// Key: (chunk_id, instruction_pointer) uniquely identifies a call site
    /// Value: Cached function pointer and metadata for fast dispatch
    inline_cache: HashMap<CallSiteId, InlineCacheEntry>,

    /// Cache of integer keys converted to strings for dict operations
    int_key_cache: HashMap<i64, Arc<str>>,

    /// Object stack for JIT non-int values (strings, dicts)
    jit_obj_stack: Vec<Value>,

    /// Tokio runtime handle for spawning async tasks
    /// This allows the VM to spawn truly concurrent async tasks
    runtime_handle: tokio::runtime::Handle,

    /// Saved execution contexts used for cooperative VM context switching.
    execution_contexts: HashMap<VmContextId, VmExecutionSnapshot>,

    /// Currently active execution context ID, if any.
    active_execution_context: Option<VmContextId>,

    /// Monotonic context ID generator.
    next_execution_context_id: VmContextId,

    /// Enables cooperative suspend/resume semantics for Await.
    cooperative_suspend_enabled: bool,

    /// Skip reset branch on the next execute() call (used for resume paths).
    skip_execute_reset_once: bool,
}

/// Unique identifier for a call site (location in bytecode where a Call occurs)
/// Used as key for inline cache lookups
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CallSiteId {
    /// Unique identifier for the chunk/function containing this call site
    /// We use the chunk name's hash for stability across executions
    chunk_id: u64,
    /// Instruction pointer within the chunk where the Call opcode is
    ip: usize,
}

static HASHMAP_PROFILE_ENABLED: OnceLock<bool> = OnceLock::new();
static HASHMAP_GET_INTDICT: AtomicU64 = AtomicU64::new(0);
static HASHMAP_GET_DENSE: AtomicU64 = AtomicU64::new(0);
static HASHMAP_GET_DENSE_INT: AtomicU64 = AtomicU64::new(0);
static HASHMAP_GET_DICT_INTKEY: AtomicU64 = AtomicU64::new(0);
static HASHMAP_SET_INTDICT: AtomicU64 = AtomicU64::new(0);
static HASHMAP_SET_DENSE: AtomicU64 = AtomicU64::new(0);
static HASHMAP_SET_DENSE_INT: AtomicU64 = AtomicU64::new(0);
static HASHMAP_SET_DICT_INTKEY: AtomicU64 = AtomicU64::new(0);

fn hashmap_profile_enabled() -> bool {
    *HASHMAP_PROFILE_ENABLED.get_or_init(|| std::env::var("KUJO_HASHMAP_PROFILE").is_ok())
}

fn hashmap_profile_bump(counter: &AtomicU64) {
    if hashmap_profile_enabled() {
        counter.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn hashmap_profile_bump_get_intdict() {
    hashmap_profile_bump(&HASHMAP_GET_INTDICT);
}

pub(crate) fn hashmap_profile_bump_get_dense() {
    hashmap_profile_bump(&HASHMAP_GET_DENSE);
}

pub(crate) fn hashmap_profile_bump_get_dense_int() {
    hashmap_profile_bump(&HASHMAP_GET_DENSE_INT);
}

pub(crate) fn hashmap_profile_bump_get_dict_intkey() {
    hashmap_profile_bump(&HASHMAP_GET_DICT_INTKEY);
}

pub(crate) fn hashmap_profile_bump_set_intdict() {
    hashmap_profile_bump(&HASHMAP_SET_INTDICT);
}

pub(crate) fn hashmap_profile_bump_set_dense() {
    hashmap_profile_bump(&HASHMAP_SET_DENSE);
}

pub(crate) fn hashmap_profile_bump_set_dense_int() {
    hashmap_profile_bump(&HASHMAP_SET_DENSE_INT);
}

pub(crate) fn hashmap_profile_bump_set_dict_intkey() {
    hashmap_profile_bump(&HASHMAP_SET_DICT_INTKEY);
}

fn hashmap_profile_print() {
    if !hashmap_profile_enabled() {
        return;
    }

    eprintln!("=== HASHMAP PROFILE ===");
    eprintln!("GET IntDict: {}", HASHMAP_GET_INTDICT.load(Ordering::Relaxed));
    eprintln!("GET DenseIntDict: {}", HASHMAP_GET_DENSE.load(Ordering::Relaxed));
    eprintln!("GET DenseIntDictInt: {}", HASHMAP_GET_DENSE_INT.load(Ordering::Relaxed));
    eprintln!("GET Dict(IntKey): {}", HASHMAP_GET_DICT_INTKEY.load(Ordering::Relaxed));
    eprintln!("SET IntDict: {}", HASHMAP_SET_INTDICT.load(Ordering::Relaxed));
    eprintln!("SET DenseIntDict: {}", HASHMAP_SET_DENSE.load(Ordering::Relaxed));
    eprintln!("SET DenseIntDictInt: {}", HASHMAP_SET_DENSE_INT.load(Ordering::Relaxed));
    eprintln!("SET Dict(IntKey): {}", HASHMAP_SET_DICT_INTKEY.load(Ordering::Relaxed));
}

struct HashMapProfileGuard {
    enabled: bool,
}

impl HashMapProfileGuard {
    fn new() -> Self {
        Self { enabled: hashmap_profile_enabled() }
    }
}

impl Drop for HashMapProfileGuard {
    fn drop(&mut self) {
        if self.enabled {
            hashmap_profile_print();
        }
    }
}

impl CallSiteId {
    fn new(chunk_name: Option<&str>, ip: usize) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let chunk_id = match chunk_name {
            Some(name) => {
                let mut hasher = DefaultHasher::new();
                name.hash(&mut hasher);
                hasher.finish()
            }
            None => 0, // Anonymous/top-level chunk
        };

        Self { chunk_id, ip }
    }
}

/// Cached information for a call site to enable fast function dispatch
#[derive(Clone)]
struct InlineCacheEntry {
    /// The expected function name at this call site
    /// Used to validate cache hit (guard against function reassignment)
    expected_func_name: String,

    /// Cached JIT-compiled function pointer (if available)
    /// None if function is not JIT-compiled yet
    compiled_fn: Option<CompiledFn>,

    /// Cached var_names HashMap for this function (avoids rebuilding on every call)
    var_names: HashMap<u64, String>,

    /// Cache hit count (for profiling and debugging)
    hit_count: usize,

    /// Cache miss count (for profiling - indicates polymorphic call sites)
    miss_count: usize,
}

impl InlineCacheEntry {
    fn new(
        func_name: &str,
        compiled_fn: Option<CompiledFn>,
        var_names: HashMap<u64, String>,
    ) -> Self {
        Self {
            expected_func_name: func_name.to_string(),
            compiled_fn,
            var_names,
            hit_count: 0,
            miss_count: 0,
        }
    }
}

/// Exception handler frame for active try blocks
#[derive(Debug, Clone)]
struct ExceptionHandlerFrame {
    /// Instruction pointer for catch block
    catch_ip: usize,

    /// Stack size when entering try block (for unwinding)
    stack_offset: usize,

    /// Call frame depth when entering try block (for unwinding)
    frame_offset: usize,
}

/// Snapshot of VM execution state for suspend/resume workflows.
///
/// This is the foundational data structure for async VM context switching.
/// It captures all mutable execution state required to pause and later resume
/// bytecode execution from the same instruction pointer and stack state.
#[derive(Debug, Clone)]
pub struct VmExecutionSnapshot {
    ip: usize,
    stack: Vec<Value>,
    call_frames: Vec<CallFrame>,
    chunk: BytecodeChunk,
    upvalues: Vec<Upvalue>,
    exception_handlers: Vec<ExceptionHandlerFrame>,
    function_call_stack: Vec<String>,
    function_call_counts: HashMap<String, usize>,
    recursion_depth: usize,
    max_recursion_depth: usize,
    int_key_cache: HashMap<i64, Arc<str>>,
    jit_obj_stack: Vec<Value>,
    globals: Arc<Mutex<Environment>>,
}

/// Generator state for suspended execution
/// Infrastructure for generator resume functionality
// Deferred post-v1 runtime backlog: full generator-state restoration (see docs/V1_SCOPE.md deferred runtime execution section)
#[allow(dead_code)] // Generator resume metadata is kept for parity with the future restoration path.
#[derive(Debug, Clone)]
pub struct GeneratorState {
    /// Instruction pointer where generator yielded
    pub ip: usize,

    /// Stack snapshot at yield point
    pub stack: Vec<Value>,

    /// Call frame stack at yield point (stored as separate values to avoid circular dependency)
    pub call_frames_data: Vec<CallFrameData>,

    /// Bytecode chunk being executed
    pub chunk: BytecodeChunk,

    /// Local variables at yield point
    pub locals: HashMap<String, Value>,

    /// Captured variables at yield point
    pub captured: HashMap<String, Arc<Mutex<Value>>>,

    /// Binding mutability metadata for captured variables.
    pub captured_binding_kinds: HashMap<String, BytecodeBindingKind>,

    /// Whether the generator has finished
    pub is_exhausted: bool,
}

/// Serializable call frame data for generator state
#[derive(Debug, Clone)]
pub struct CallFrameData {
    pub return_ip: usize,
    pub stack_offset: usize,
    pub locals: HashMap<String, Value>,
    pub locals_binding_kinds: HashMap<String, BytecodeBindingKind>,
    pub local_slots: Vec<Value>,
    pub local_slot_binding_kinds: Vec<BytecodeBindingKind>,
    pub local_slot_initialized: Vec<bool>,
    pub captured: HashMap<String, Arc<Mutex<Value>>>,
    pub captured_binding_kinds: HashMap<String, BytecodeBindingKind>,
}

/// Call frame for function calls
#[derive(Debug, Clone)]
pub(crate) struct CallFrame {
    /// Return address (instruction pointer)
    return_ip: usize,

    /// Stack offset for this frame
    stack_offset: usize,

    /// Local environment for this frame (parameters and local variables)
    locals: HashMap<String, Value>,

    /// Binding mutability metadata for local named variables.
    locals_binding_kinds: HashMap<String, BytecodeBindingKind>,

    /// Local slot storage for fast variable access
    pub(crate) local_slots: Vec<Value>,

    /// Binding mutability metadata for local slots.
    local_slot_binding_kinds: Vec<BytecodeBindingKind>,

    /// Tracks whether a slot has been initialized at least once.
    local_slot_initialized: Vec<bool>,

    /// Captured variables (upvalues) with shared mutable state
    captured: HashMap<String, Arc<Mutex<Value>>>,

    /// Binding mutability metadata for captured variables.
    captured_binding_kinds: HashMap<String, BytecodeBindingKind>,

    /// Previous chunk (for returning)
    prev_chunk: Option<BytecodeChunk>,

    /// Whether this function is async (for wrapping return values in Promises)
    is_async: bool,
}

#[allow(dead_code)] // These VM helpers are retained for scheduler/JIT/debug follow-through paths.
impl VM {
    fn env_binding_kind(kind: BytecodeBindingKind) -> BindingKind {
        match kind {
            BytecodeBindingKind::Mutable => BindingKind::Mutable,
            BytecodeBindingKind::LetImmutable => BindingKind::LetImmutable,
            BytecodeBindingKind::Const => BindingKind::Const,
        }
    }

    fn local_reassignment_error(kind: BytecodeBindingKind, name: &str) -> String {
        match kind {
            BytecodeBindingKind::Mutable => unreachable!("mutable bindings allow reassignment"),
            BytecodeBindingKind::LetImmutable => {
                format!("Cannot reassign immutable let binding: {}", name)
            }
            BytecodeBindingKind::Const => format!("Cannot reassign const binding: {}", name),
        }
    }

    fn local_mutation_error(kind: BytecodeBindingKind, name: &str) -> String {
        match kind {
            BytecodeBindingKind::Mutable => unreachable!("mutable bindings allow mutation"),
            BytecodeBindingKind::LetImmutable => {
                format!("Cannot mutate immutable let binding: {}", name)
            }
            BytecodeBindingKind::Const => format!("Cannot mutate const binding: {}", name),
        }
    }

    fn undefined_variable_message(name: &str) -> String {
        format!("Undefined variable: {}", name)
    }

    fn value_error_message(value: &Value) -> Option<String> {
        match value {
            Value::Error(message) => Some(message.clone()),
            Value::ErrorObject { message, .. } => Some(message.clone()),
            _ => None,
        }
    }

    fn non_callable_error_message(context: &str) -> String {
        format!(
            "Cannot call non-function; {}. Pass a function, closure, or imported callable value instead.",
            context
        )
    }

    fn throw_runtime_value(&mut self, error_value: Value) -> Result<(), String> {
        let throw_stack = self.function_call_stack.clone();

        let normalized_error = match error_value {
            Value::ErrorObject { message, mut stack, line, cause } => {
                if stack.is_empty() && !throw_stack.is_empty() {
                    stack = throw_stack;
                }
                Value::ErrorObject { message, stack, line, cause }
            }
            Value::Str(message) => Value::ErrorObject {
                message: message.as_ref().clone(),
                stack: throw_stack,
                line: None,
                cause: None,
            },
            Value::Error(message) => {
                Value::ErrorObject { message, stack: throw_stack, line: None, cause: None }
            }
            Value::Struct { name, fields } => {
                let message = match fields.get("message") {
                    Some(Value::Str(msg)) => msg.as_ref().clone(),
                    _ => format!("{name} error"),
                };
                let cause = fields.get("cause").cloned().map(Box::new);
                Value::ErrorObject { message, stack: throw_stack, line: None, cause }
            }
            other => Value::ErrorObject {
                message: format!("{:?}", other),
                stack: throw_stack,
                line: None,
                cause: None,
            },
        };

        if let Some(handler) = self.exception_handlers.pop() {
            while self.call_frames.len() > handler.frame_offset {
                if let Some(frame) = self.call_frames.pop() {
                    self.function_call_stack.pop();
                    if self.recursion_depth > 0 {
                        self.recursion_depth -= 1;
                    }
                    if self.call_frames.len() == handler.frame_offset {
                        if let Some(prev_chunk) = frame.prev_chunk {
                            self.set_chunk(prev_chunk);
                        }
                    }
                }
            }

            self.stack.truncate(handler.stack_offset);
            self.stack.push(normalized_error);
            self.ip = handler.catch_ip;
            Ok(())
        } else {
            let error_msg = match normalized_error {
                Value::ErrorObject { message, .. } => message,
                Value::Error(message) => message,
                Value::Str(value) => value.as_ref().clone(),
                Value::Secret(_) => "***".to_string(),
                other => format!("Uncaught exception: {:?}", other),
            };
            Err(error_msg)
        }
    }

    pub fn new() -> Self {
        let vm = Self {
            stack: Vec::new(),
            call_frames: Vec::new(),
            globals: Arc::new(Mutex::new(Environment::new())),
            ip: 0,
            chunk: BytecodeChunk::new(),
            interpreter: Interpreter::new(),
            upvalues: Vec::new(),
            exception_handlers: Vec::new(),
            jit_compiler: JitCompiler::new().unwrap_or_else(|e| {
                eprintln!("Warning: Failed to initialize JIT compiler: {}", e);
                eprintln!("Falling back to interpreter-only mode");
                JitCompiler::default()
            }),
            jit_enabled: false,
            function_call_stack: Vec::new(),
            function_call_counts: HashMap::new(),
            compiled_functions: HashMap::new(),
            compiled_fn_info: HashMap::new(),
            jit_var_names_cache: HashMap::new(),
            recursion_depth: 0,
            max_recursion_depth: 0,
            inline_cache: HashMap::new(),
            int_key_cache: HashMap::new(),
            jit_obj_stack: Vec::new(),
            runtime_handle: tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
                // If not in a tokio runtime, create one
                crate::interpreter::AsyncRuntime::runtime().handle().clone()
            }),
            execution_contexts: HashMap::new(),
            active_execution_context: None,
            next_execution_context_id: 1,
            cooperative_suspend_enabled: true,
            skip_execute_reset_once: false,
        };

        vm
    }

    pub fn set_capability_policy(&mut self, capability_policy: RuntimeCapabilityPolicy) {
        self.interpreter.set_capability_policy(capability_policy);
    }

    fn set_chunk(&mut self, chunk: BytecodeChunk) {
        self.chunk = chunk;
    }

    /// Capture a full execution snapshot for later restoration.
    ///
    /// This enables suspendable VM execution by preserving instruction pointer,
    /// stacks, frames, and runtime metadata needed to continue execution
    /// without restarting from the beginning.
    pub fn save_execution_state(&self) -> VmExecutionSnapshot {
        VmExecutionSnapshot {
            ip: self.ip,
            stack: self.stack.clone(),
            call_frames: self.call_frames.clone(),
            chunk: self.chunk.clone(),
            upvalues: self.upvalues.clone(),
            exception_handlers: self.exception_handlers.clone(),
            function_call_stack: self.function_call_stack.clone(),
            function_call_counts: self.function_call_counts.clone(),
            recursion_depth: self.recursion_depth,
            max_recursion_depth: self.max_recursion_depth,
            int_key_cache: self.int_key_cache.clone(),
            jit_obj_stack: self.jit_obj_stack.clone(),
            globals: Arc::clone(&self.globals),
        }
    }

    /// Restore a previously saved execution snapshot.
    ///
    /// This rewinds the VM to the exact saved execution state so that
    /// bytecode execution can resume from the captured instruction pointer.
    pub fn restore_execution_state(&mut self, snapshot: VmExecutionSnapshot) {
        self.ip = snapshot.ip;
        self.stack = snapshot.stack;
        self.call_frames = snapshot.call_frames;
        self.chunk = snapshot.chunk;
        self.upvalues = snapshot.upvalues;
        self.exception_handlers = snapshot.exception_handlers;
        self.function_call_stack = snapshot.function_call_stack;
        self.function_call_counts = snapshot.function_call_counts;
        self.recursion_depth = snapshot.recursion_depth;
        self.max_recursion_depth = snapshot.max_recursion_depth;
        self.int_key_cache = snapshot.int_key_cache;
        self.jit_obj_stack = snapshot.jit_obj_stack;
        self.globals = snapshot.globals;
        self.interpreter.set_env(Arc::clone(&self.globals));
    }

    fn allocate_execution_context_id(&mut self) -> VmContextId {
        let id = self.next_execution_context_id;
        self.next_execution_context_id = self.next_execution_context_id.saturating_add(1);
        id
    }

    fn suspend_error(context_id: VmContextId) -> String {
        format!("{}{}", VM_SUSPEND_ERROR_PREFIX, context_id)
    }

    fn parse_suspend_error(error: &str) -> Option<VmContextId> {
        error
            .strip_prefix(VM_SUSPEND_ERROR_PREFIX)
            .and_then(|value| value.parse::<VmContextId>().ok())
    }

    /// Register an execution context from an existing snapshot.
    pub fn create_execution_context(&mut self, snapshot: VmExecutionSnapshot) -> VmContextId {
        let context_id = self.allocate_execution_context_id();
        self.execution_contexts.insert(context_id, snapshot);
        context_id
    }

    /// Register an execution context by snapshotting current VM state.
    pub fn create_execution_context_from_current(&mut self) -> VmContextId {
        let snapshot = self.save_execution_state();
        let context_id = self.create_execution_context(snapshot);

        if self.active_execution_context.is_none() {
            self.active_execution_context = Some(context_id);
        }

        context_id
    }

    /// Returns the currently active execution context ID.
    pub fn active_execution_context_id(&self) -> Option<VmContextId> {
        self.active_execution_context
    }

    /// Returns true if an execution context with the given ID exists.
    pub fn has_execution_context(&self, context_id: VmContextId) -> bool {
        self.execution_contexts.contains_key(&context_id)
    }

    /// List all registered execution context IDs in sorted order.
    pub fn list_execution_context_ids(&self) -> Vec<VmContextId> {
        let mut ids: Vec<VmContextId> = self.execution_contexts.keys().cloned().collect();
        ids.sort_unstable();
        ids
    }

    /// Switch VM execution to a previously registered context.
    ///
    /// Saves the currently active context (if any), restores target context state,
    /// and marks target as active.
    pub fn switch_execution_context(
        &mut self,
        target_context_id: VmContextId,
    ) -> Result<(), String> {
        let target_snapshot = self
            .execution_contexts
            .get(&target_context_id)
            .cloned()
            .ok_or_else(|| format!("Execution context {} not found", target_context_id))?;

        if let Some(active_context_id) = self.active_execution_context {
            if active_context_id != target_context_id {
                let current_snapshot = self.save_execution_state();
                self.execution_contexts.insert(active_context_id, current_snapshot);
            }
        }

        self.restore_execution_state(target_snapshot);
        self.active_execution_context = Some(target_context_id);
        Ok(())
    }

    /// Remove a non-active execution context.
    pub fn remove_execution_context(&mut self, context_id: VmContextId) -> Result<(), String> {
        if self.active_execution_context == Some(context_id) {
            return Err(format!("Cannot remove active execution context {}", context_id));
        }

        if self.execution_contexts.remove(&context_id).is_none() {
            return Err(format!("Execution context {} not found", context_id));
        }

        Ok(())
    }

    /// Set the global environment (for accessing built-in functions)
    pub fn set_globals(&mut self, env: Arc<Mutex<Environment>>) {
        self.globals = env.clone();
        // Also set the interpreter's environment so it can resolve native functions
        self.interpreter.set_env(env);
    }

    /// Adds a module search path used by VM import helpers.
    pub fn add_module_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.interpreter.module_loader.add_search_path(path);
    }

    /// Enable or disable JIT compilation
    pub fn set_jit_enabled(&mut self, enabled: bool) {
        self.jit_enabled = enabled;
        self.jit_compiler.set_enabled(enabled);
    }

    /// Returns whether JIT is currently enabled for this VM.
    pub fn jit_enabled(&self) -> bool {
        self.jit_enabled
    }

    /// Get JIT compilation statistics
    pub fn jit_stats(&self) -> crate::jit::JitStats {
        self.jit_compiler.stats()
    }

    /// Validate whether a bytecode chunk (including nested function chunks) is JIT-compatible.
    pub fn validate_jit_supported_surfaces(&self, chunk: &BytecodeChunk) -> Result<(), String> {
        self.first_unsupported_jit_surface(chunk)
            .map_or(Ok(()), |unsupported| Err(unsupported.describe()))
    }

    fn first_unsupported_jit_surface(
        &self,
        chunk: &BytecodeChunk,
    ) -> Option<UnsupportedJitSurface> {
        self.jit_compiler.first_unsupported_surface(chunk)
    }

    /// Attempt to JIT-compile a bytecode function immediately.
    ///
    /// Returns `Ok(true)` when compilation succeeds, `Ok(false)` when compilation is skipped
    /// (e.g. JIT disabled or non-bytecode input), and `Err` on hard failures.
    pub fn jit_compile_bytecode_function(&mut self, function: &Value) -> Result<bool, String> {
        if !self.jit_enabled {
            return Ok(false);
        }

        let (chunk, func_name) = match function {
            Value::BytecodeFunction { chunk, .. } => {
                let name = chunk.name.as_deref().unwrap_or("<anonymous>").to_string();
                (chunk, name)
            }
            _ => return Ok(false),
        };

        let has_map_fusion_op = chunk.instructions.iter().any(|op| {
            matches!(
                op,
                OpCode::SumIntMapUntilLocalInPlace(_, _, _, _)
                    | OpCode::FillIntMapWithDoubleUntilLocalInPlace(_, _, _)
            )
        });

        if std::env::var("DISABLE_FUNCTION_JIT").is_ok() || has_map_fusion_op {
            return Ok(false);
        }

        if self.compiled_functions.contains_key(func_name.as_str()) {
            return Ok(true);
        }

        let info = self
            .jit_compiler
            .compile_function_with_info(chunk, func_name.as_str())
            .map_err(|e| format!("Failed to JIT-compile function '{}': {}", func_name, e))?;

        self.compiled_functions.insert(func_name.clone(), info.fn_ptr);
        self.compiled_fn_info.insert(func_name, info);

        Ok(true)
    }

    /// Get the VM's function call stack for error reporting
    pub fn get_call_stack(&self) -> Vec<String> {
        self.function_call_stack.clone()
    }

    /// Get or cache the string form of an integer dict key
    pub(crate) fn int_key_string(&mut self, key: i64) -> Arc<str> {
        if let Some(value) = self.int_key_cache.get(&key) {
            return Arc::clone(value);
        }

        let value: Arc<str> = Arc::from(key.to_string());
        self.int_key_cache.insert(key, Arc::clone(&value));
        value
    }

    fn index_key_description(index: &Value) -> String {
        match index {
            Value::Str(key) => format!("{:?}", key.as_ref()),
            Value::Int(key) => key.to_string(),
            other => Self::value_to_string(other),
        }
    }

    fn missing_map_key_error(index: &Value) -> String {
        format!("Missing map key: {}", Self::index_key_description(index))
    }

    fn value_nesting_depth(value: &Value) -> usize {
        let mut stack: Vec<(&Value, usize)> = vec![(value, 0)];
        let mut max_depth = 0;

        while let Some((value, depth)) = stack.pop() {
            max_depth = max_depth.max(depth);

            match value {
                Value::Array(items) => {
                    for item in items.iter() {
                        stack.push((item, depth + 1));
                    }
                }
                Value::Dict(dict) => {
                    for nested in dict.values() {
                        stack.push((nested, depth + 1));
                    }
                }
                Value::FixedDict { values, .. } => {
                    for nested in values.iter() {
                        stack.push((nested, depth + 1));
                    }
                }
                Value::Struct { fields, .. } | Value::Tagged { fields, .. } => {
                    for nested in fields.values() {
                        stack.push((nested, depth + 1));
                    }
                }
                Value::IntDict(dict) => {
                    for nested in dict.values() {
                        stack.push((nested, depth + 1));
                    }
                }
                Value::DenseIntDict(values) => {
                    for nested in values.iter() {
                        stack.push((nested, depth + 1));
                    }
                }
                Value::Set(items) | Value::Stack(items) => {
                    for nested in items.iter() {
                        stack.push((nested, depth + 1));
                    }
                }
                Value::Queue(items) => {
                    for nested in items.iter() {
                        stack.push((nested, depth + 1));
                    }
                }
                Value::HttpServer { routes, .. } => {
                    for (_, _, nested) in routes.iter() {
                        stack.push((nested, depth + 1));
                    }
                }
                Value::Result { value, .. }
                | Value::Option { value, .. }
                | Value::Return(value) => {
                    stack.push((value.as_ref(), depth + 1));
                }
                Value::ErrorObject { cause: Some(cause), .. } => {
                    stack.push((cause.as_ref(), depth + 1));
                }
                #[cfg(feature = "runtime-db")]
                Value::Database { .. } | Value::DatabasePool { .. } => {}
                #[cfg(feature = "runtime-image")]
                Value::Image { .. } => {}
                #[cfg(feature = "runtime-archive")]
                Value::ZipArchive { .. } => {}
                Value::HttpResponse { .. }
                | Value::Int(_)
                | Value::Float(_)
                | Value::Str(_)
                | Value::Secret(_)
                | Value::Bool(_)
                | Value::Null
                | Value::Bytes(_)
                | Value::Function(_, _, _)
                | Value::AsyncFunction(_, _, _)
                | Value::NativeFunction(_)
                | Value::BytecodeFunction { .. }
                | Value::BytecodeGenerator { .. }
                | Value::ArrayMarker
                | Value::Error(_)
                | Value::ErrorObject { .. }
                | Value::Enum(_)
                | Value::DenseIntDictInt(_)
                | Value::DenseIntDictIntFull(_)
                | Value::TcpListener { .. }
                | Value::TcpStream { .. }
                | Value::UdpSocket { .. }
                | Value::Channel(_)
                | Value::GeneratorDef(_, _)
                | Value::Generator { .. }
                | Value::Iterator { .. }
                | Value::Promise { .. }
                | Value::TaskHandle { .. }
                | Value::StructDef { .. } => {}
            }
        }

        max_depth
    }

    fn ensure_value_nesting_depth(value: &Value, context: &str) -> Result<(), String> {
        let max_depth = runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH;
        let depth = Self::value_nesting_depth(value);
        if depth > max_depth {
            Err(format!("Maximum data nesting depth of {} exceeded while {}", max_depth, context))
        } else {
            Ok(())
        }
    }

    fn push_checked_value(&mut self, value: Value, context: &str) -> Result<bool, String> {
        if let Err(message) = Self::ensure_value_nesting_depth(&value, context) {
            if self.exception_handlers.is_empty() {
                return Err(message);
            }

            self.throw_runtime_value(Value::Error(message))?;
            return Ok(false);
        }

        self.stack.push(value);
        Ok(true)
    }

    fn get_indexed_value(object: &Value, index: &Value) -> Result<Value, String> {
        match (object, index) {
            (Value::Array(arr), Value::Int(i)) => {
                let idx = if *i < 0 { (arr.len() as i64 + i) as usize } else { *i as usize };
                arr.get(idx).cloned().ok_or_else(|| format!("Index out of bounds: {}", i))
            }
            (Value::Str(s), Value::Int(i)) => {
                let char_len = s.chars().count();
                let idx = if *i < 0 { (char_len as i64 + i) as usize } else { *i as usize };
                s.chars()
                    .nth(idx)
                    .map(|c| Value::Str(Arc::new(c.to_string())))
                    .ok_or_else(|| format!("Index out of bounds: {}", i))
            }
            (Value::Dict(dict), Value::Str(key)) => {
                dict.get(key.as_str()).cloned().ok_or_else(|| Self::missing_map_key_error(index))
            }
            (Value::FixedDict { keys, values }, Value::Str(key)) => {
                let idx = keys.iter().position(|k| k.as_ref() == key.as_str());
                idx.and_then(|i| values.get(i).cloned())
                    .ok_or_else(|| Self::missing_map_key_error(index))
            }
            (Value::Dict(dict), Value::Int(i)) => {
                hashmap_profile_bump(&HASHMAP_GET_DICT_INTKEY);
                let key = i.to_string();
                dict.get(key.as_str()).cloned().ok_or_else(|| Self::missing_map_key_error(index))
            }
            (Value::FixedDict { keys, values }, Value::Int(i)) => {
                let key = i.to_string();
                let idx = keys.iter().position(|k| k.as_ref() == key.as_str());
                idx.and_then(|i| values.get(i).cloned())
                    .ok_or_else(|| Self::missing_map_key_error(index))
            }
            (Value::IntDict(dict), Value::Int(i)) => {
                hashmap_profile_bump(&HASHMAP_GET_INTDICT);
                dict.get(i).cloned().ok_or_else(|| Self::missing_map_key_error(index))
            }
            (Value::DenseIntDict(values), Value::Int(i)) => {
                hashmap_profile_bump(&HASHMAP_GET_DENSE);
                if *i < 0 {
                    Err(Self::missing_map_key_error(index))
                } else {
                    values
                        .get(*i as usize)
                        .cloned()
                        .ok_or_else(|| Self::missing_map_key_error(index))
                }
            }
            (Value::DenseIntDictInt(values), Value::Int(i)) => {
                hashmap_profile_bump(&HASHMAP_GET_DENSE_INT);
                if *i < 0 {
                    Err(Self::missing_map_key_error(index))
                } else {
                    match values.get(*i as usize) {
                        Some(Some(value)) => Ok(Value::Int(*value)),
                        _ => Err(Self::missing_map_key_error(index)),
                    }
                }
            }
            (Value::DenseIntDictIntFull(values), Value::Int(i)) => {
                hashmap_profile_bump(&HASHMAP_GET_DENSE_INT);
                if *i < 0 {
                    Err(Self::missing_map_key_error(index))
                } else {
                    values
                        .get(*i as usize)
                        .map(|value| Value::Int(*value))
                        .ok_or_else(|| Self::missing_map_key_error(index))
                }
            }
            (Value::IntDict(dict), Value::Str(key)) => match key.parse::<i64>() {
                Ok(int_key) => {
                    dict.get(&int_key).cloned().ok_or_else(|| Self::missing_map_key_error(index))
                }
                Err(_) => Err("Invalid index operation".to_string()),
            },
            (Value::DenseIntDict(values), Value::Str(key)) => match key.parse::<i64>() {
                Ok(int_key) if int_key >= 0 => values
                    .get(int_key as usize)
                    .cloned()
                    .ok_or_else(|| Self::missing_map_key_error(index)),
                Ok(_) => Err(Self::missing_map_key_error(index)),
                Err(_) => Err("Invalid index operation".to_string()),
            },
            (Value::DenseIntDictInt(values), Value::Str(key)) => match key.parse::<i64>() {
                Ok(int_key) if int_key >= 0 => match values.get(int_key as usize) {
                    Some(Some(value)) => Ok(Value::Int(*value)),
                    _ => Err(Self::missing_map_key_error(index)),
                },
                Ok(_) => Err(Self::missing_map_key_error(index)),
                Err(_) => Err("Invalid index operation".to_string()),
            },
            (Value::DenseIntDictIntFull(values), Value::Str(key)) => match key.parse::<i64>() {
                Ok(int_key) if int_key >= 0 => values
                    .get(int_key as usize)
                    .map(|value| Value::Int(*value))
                    .ok_or_else(|| Self::missing_map_key_error(index)),
                Ok(_) => Err(Self::missing_map_key_error(index)),
                Err(_) => Err("Invalid index operation".to_string()),
            },
            _ => Err("Invalid index operation".to_string()),
        }
    }

    fn dense_int_dict_to_int_dict(values: &[Value]) -> IntDictMap {
        let mut dict = IntDictMap::default();
        dict.reserve(values.len());
        for (index, value) in values.iter().enumerate() {
            dict.insert(index as i64, value.clone());
        }
        dict
    }

    fn dense_int_dict_int_to_int_dict(values: &[Option<i64>]) -> IntDictMap {
        let mut dict = IntDictMap::default();
        dict.reserve(values.len());
        for (index, value) in values.iter().enumerate() {
            dict.insert(index as i64, (*value).map(Value::Int).unwrap_or(Value::Null));
        }
        dict
    }

    fn dense_int_dict_int_full_to_int_dict(values: &[i64]) -> IntDictMap {
        let mut dict = IntDictMap::default();
        dict.reserve(values.len());
        for (index, value) in values.iter().enumerate() {
            dict.insert(index as i64, Value::Int(*value));
        }
        dict
    }

    fn dense_int_dict_int_full_to_sparse(values: &[i64]) -> DenseIntDictInt {
        let mut sparse = Vec::with_capacity(values.len());
        for value in values.iter() {
            sparse.push(Some(*value));
        }
        sparse
    }

    fn dense_int_dict_int_full_to_dense(values: &[i64]) -> DenseIntDict {
        let mut dense = Vec::with_capacity(values.len());
        for value in values.iter() {
            dense.push(Value::Int(*value));
        }
        dense
    }

    fn dense_int_dict_to_dict(values: &[Value]) -> DictMap {
        let mut dict = DictMap::default();
        dict.reserve(values.len());
        for (index, value) in values.iter().enumerate() {
            dict.insert(Arc::from(index.to_string().as_str()), value.clone());
        }
        dict
    }

    fn dense_int_dict_int_to_dict(values: &[Option<i64>]) -> DictMap {
        let mut dict = DictMap::default();
        dict.reserve(values.len());
        for (index, value) in values.iter().enumerate() {
            dict.insert(
                Arc::from(index.to_string().as_str()),
                (*value).map(Value::Int).unwrap_or(Value::Null),
            );
        }
        dict
    }

    fn dense_int_dict_int_to_dense_int_dict(values: &[Option<i64>]) -> Vec<Value> {
        let mut dict = Vec::with_capacity(values.len());
        for value in values.iter() {
            dict.push((*value).map(Value::Int).unwrap_or(Value::Null));
        }
        dict
    }

    fn dense_int_dict_int_with_len(len: usize) -> Vec<Option<i64>> {
        let mut values = Vec::with_capacity(len.max(DENSE_INT_DICT_MIN_CAPACITY));
        values.resize(len, None);
        values
    }

    /// Execute bytecode cooperatively until completion or Await suspension.
    pub fn execute_until_suspend(
        &mut self,
        chunk: BytecodeChunk,
    ) -> Result<VmExecutionResult, String> {
        self.cooperative_suspend_enabled = true;
        self.active_execution_context = None;

        let result = self.execute(chunk);
        self.cooperative_suspend_enabled = false;

        match result {
            Ok(value) => {
                if let Some(error) = Self::value_error_message(&value) {
                    Err(error)
                } else {
                    Ok(VmExecutionResult::Completed)
                }
            }
            Err(error) => {
                if let Some(context_id) = Self::parse_suspend_error(&error) {
                    Ok(VmExecutionResult::Suspended { context_id })
                } else {
                    Err(error)
                }
            }
        }
    }

    /// Resume a previously suspended execution context.
    pub fn resume_execution_context(
        &mut self,
        context_id: VmContextId,
    ) -> Result<VmExecutionResult, String> {
        self.switch_execution_context(context_id)?;
        self.cooperative_suspend_enabled = true;
        self.skip_execute_reset_once = true;

        let result = self.execute(BytecodeChunk::new());
        self.cooperative_suspend_enabled = false;

        match result {
            Ok(value) => {
                if let Some(error) = Self::value_error_message(&value) {
                    return Err(error);
                }
                self.execution_contexts.remove(&context_id);
                if self.active_execution_context == Some(context_id) {
                    self.active_execution_context = None;
                }
                Ok(VmExecutionResult::Completed)
            }
            Err(error) => {
                if let Some(suspended_context_id) = Self::parse_suspend_error(&error) {
                    Ok(VmExecutionResult::Suspended { context_id: suspended_context_id })
                } else {
                    Err(error)
                }
            }
        }
    }

    /// Returns the number of pending cooperative execution contexts.
    pub fn pending_execution_context_count(&self) -> usize {
        self.execution_contexts.len()
    }

    /// Run one cooperative round over all currently registered execution contexts.
    ///
    /// Each pending context is resumed once in deterministic ID order.
    /// Completed contexts are removed automatically.
    pub fn run_scheduler_round(&mut self) -> Result<VmSchedulerRoundResult, String> {
        let context_ids = self.list_execution_context_ids();
        let mut completed_contexts = 0;

        for context_id in context_ids {
            if !self.has_execution_context(context_id) {
                continue;
            }

            match self.resume_execution_context(context_id)? {
                VmExecutionResult::Completed => {
                    completed_contexts += 1;
                }
                VmExecutionResult::Suspended { .. } => {}
            }
        }

        Ok(VmSchedulerRoundResult {
            completed_contexts,
            pending_contexts: self.pending_execution_context_count(),
        })
    }

    /// Run the cooperative scheduler until all contexts complete or a round limit is hit.
    ///
    /// Returns an error when pending contexts remain after `max_rounds`.
    pub fn run_scheduler_until_complete(&mut self, max_rounds: usize) -> Result<(), String> {
        if 0 == max_rounds {
            return Err("Scheduler max_rounds must be greater than 0".to_string());
        }

        for _ in 0..max_rounds {
            if 0 == self.pending_execution_context_count() {
                return Ok(());
            }

            let round_result = self.run_scheduler_round()?;

            if 0 == round_result.pending_contexts {
                return Ok(());
            }

            if 0 == round_result.completed_contexts {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }

        Err(format!(
            "Scheduler did not complete all contexts within {} rounds ({} pending)",
            max_rounds,
            self.pending_execution_context_count()
        ))
    }

    /// Run the cooperative scheduler until all contexts complete or a timeout is reached.
    ///
    /// Returns an error when pending contexts remain after the timeout budget elapses.
    pub fn run_scheduler_until_complete_with_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<(), String> {
        if timeout.is_zero() {
            return Err("Scheduler timeout must be greater than 0ms".to_string());
        }

        let start = std::time::Instant::now();
        let mut rounds: usize = 0;

        loop {
            if 0 == self.pending_execution_context_count() {
                return Ok(());
            }

            if start.elapsed() >= timeout {
                return Err(format!(
                    "Scheduler timed out after {}ms ({} rounds, {} pending)",
                    timeout.as_millis(),
                    rounds,
                    self.pending_execution_context_count()
                ));
            }

            let round_result = self.run_scheduler_round()?;
            rounds += 1;

            if 0 == round_result.pending_contexts {
                return Ok(());
            }

            if 0 == round_result.completed_contexts {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }

    /// Execute a bytecode chunk
    pub fn execute(&mut self, chunk: BytecodeChunk) -> Result<Value, String> {
        let _hashmap_profile_guard = HashMapProfileGuard::new();
        if self.skip_execute_reset_once {
            self.skip_execute_reset_once = false;
        } else {
            self.set_chunk(chunk);
            self.ip = 0;
            self.stack.clear();
        }

        // Try to JIT-compile the entire script for maximum performance
        let allow_script_jit = self.jit_enabled && std::env::var("DISABLE_SCRIPT_JIT").is_err();
        if allow_script_jit {
            let mut script_jit_safe = true;

            for constant in &self.chunk.constants {
                if matches!(constant, Constant::String(_)) {
                    script_jit_safe = false;
                    break;
                }
            }

            if script_jit_safe {
                for instruction in &self.chunk.instructions {
                    if matches!(
                        instruction,
                        OpCode::MakeDict(_)
                            | OpCode::MakeDictWithKeys(_)
                            | OpCode::Add
                            | OpCode::Sub
                            | OpCode::Mul
                            | OpCode::Div
                            | OpCode::Mod
                            | OpCode::Negate
                            | OpCode::Not
                            | OpCode::IndexGet
                            | OpCode::IndexSet
                            | OpCode::IndexGetInPlace(_)
                            | OpCode::IndexSetInPlace(_)
                            | OpCode::LoadVar(_)
                            | OpCode::LoadGlobal(_)
                            | OpCode::DefineGlobal(_, _)
                            | OpCode::EnsureMutableGlobalForMutation(_)
                            | OpCode::SumIntMapUntilLocalInPlace(_, _, _, _)
                            | OpCode::FillIntMapWithDoubleUntilLocalInPlace(_, _, _)
                    ) {
                        script_jit_safe = false;
                        break;
                    }
                }
            }

            // Attempt to compile the entire script
            if script_jit_safe {
                match self.jit_compiler.compile_script(&self.chunk, "__main__") {
                    Ok(compiled_fn) => {
                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!("JIT: Successfully compiled top-level script - EXECUTING!");
                        }

                        // Execute the JIT-compiled script
                        let vm_ptr: *mut std::ffi::c_void = self as *mut _ as *mut std::ffi::c_void;
                        let stack_ptr: *mut Vec<Value> = &mut self.stack;

                        let mut globals_guard = self.globals.lock().unwrap();
                        let globals_ptr: *mut HashMap<String, Value> = &mut globals_guard.scopes[0];

                        // For top-level scripts, globals = locals
                        let locals_ptr: *mut HashMap<String, Value> = globals_ptr;

                        let mut vm_context = crate::jit::VMContext::new_with_vm(
                            stack_ptr,
                            locals_ptr,
                            globals_ptr,
                            vm_ptr,
                        );
                        vm_context.local_slots_ptr = match self.call_frames.last_mut() {
                            Some(frame) => &mut frame.local_slots as *mut Vec<Value>,
                            None => std::ptr::null_mut(),
                        };
                        vm_context.obj_stack_ptr = &mut self.jit_obj_stack as *mut Vec<Value>;

                        let chunk_name = self.chunk.name.as_deref().unwrap_or("<script>");
                        let cache_key = format!("script:{}", chunk_name);
                        if !self.jit_var_names_cache.contains_key(&cache_key) {
                            let mut cached_var_names = HashMap::new();

                            for instr in &self.chunk.instructions {
                                match instr {
                                    OpCode::LoadVar(name)
                                    | OpCode::StoreVar(name)
                                    | OpCode::LoadGlobal(name)
                                    | OpCode::StoreGlobal(name) => {
                                        use std::collections::hash_map::DefaultHasher;
                                        use std::hash::{Hash, Hasher};
                                        let mut hasher = DefaultHasher::new();
                                        name.hash(&mut hasher);
                                        let hash = hasher.finish();
                                        cached_var_names.insert(hash, name.clone());
                                    }
                                    _ => {}
                                }
                            }

                            self.jit_var_names_cache.insert(cache_key.clone(), cached_var_names);
                        }

                        let var_names_ptr: *mut HashMap<u64, String> = self
                            .jit_var_names_cache
                            .get_mut(&cache_key)
                            .map(|v| v as *mut HashMap<u64, String>)
                            .unwrap_or(std::ptr::null_mut());
                        vm_context.var_names_ptr = var_names_ptr;

                        drop(globals_guard); // Release lock before calling compiled code

                        let status_code = invoke_compiled_fn(compiled_fn, &mut vm_context);

                        if status_code < 0 {
                            if std::env::var("DEBUG_JIT").is_ok() {
                                eprintln!(
                                    "JIT: Script execution returned error code: {}",
                                    status_code
                                );
                            }
                            // Fall back to interpreter on JIT error
                        } else {
                            // Script executed successfully
                            // Top of stack is the result (if any)
                            return Ok(self.stack.last().cloned().unwrap_or(Value::Null));
                        }
                    }
                    Err(e) => {
                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!("JIT: Could not compile script: {}", e);
                            eprintln!("JIT: Falling back to interpreter");
                        }
                        // Fall through to interpreter
                    }
                }
            }
        }

        // Interpreter fallback or JIT disabled
        let contains_map_fusion_op = self.chunk.instructions.iter().any(|instruction| {
            matches!(
                instruction,
                OpCode::SumIntMapUntilLocalInPlace(_, _, _, _)
                    | OpCode::FillIntMapWithDoubleUntilLocalInPlace(_, _, _)
            )
        });

        loop {
            if self.ip >= self.chunk.instructions.len() {
                // Reached end of program
                return Ok(Value::Null);
            }

            // Check if we should JIT compile this hot path
            if self.jit_enabled && !contains_map_fusion_op {
                // For loops (backward jumps), check if we should compile
                if let Some(OpCode::JumpBack(jump_target)) = self.chunk.instructions.get(self.ip) {
                    if self.jit_compiler.is_loop_jit_blocked(*jump_target) {
                        // Loop is known to be incompatible with JIT
                    } else if self.jit_compiler.should_compile(*jump_target) {
                        // PRE-SCAN: Check if loop contains only supported opcodes
                        // This prevents compilation failures and maintains correctness
                        let mut int_dict_slots = std::collections::HashSet::new();
                        let mut int_dict_loop_valid =
                            !std::env::var("DISABLE_INT_DICT_LOOP_JIT").is_ok();

                        if int_dict_loop_valid {
                            for instr in
                                self.chunk.instructions.iter().take(self.ip + 1).skip(*jump_target)
                            {
                                match instr {
                                    OpCode::IndexGetInPlace(slot)
                                    | OpCode::IndexSetInPlace(slot) => {
                                        int_dict_slots.insert(*slot);
                                    }
                                    _ => {}
                                }
                            }

                            if !int_dict_slots.is_empty() {
                                for instr in self
                                    .chunk
                                    .instructions
                                    .iter()
                                    .take(self.ip + 1)
                                    .skip(*jump_target)
                                {
                                    match instr {
                                        OpCode::LoadLocal(slot) | OpCode::StoreLocal(slot) => {
                                            if int_dict_slots.contains(slot) {
                                                int_dict_loop_valid = false;
                                                break;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            if int_dict_loop_valid && !int_dict_slots.is_empty() {
                                if let Some(frame) = self.call_frames.last() {
                                    for slot in &int_dict_slots {
                                        let object = match frame.local_slots.get(*slot) {
                                            Some(value) => value,
                                            None => {
                                                int_dict_loop_valid = false;
                                                break;
                                            }
                                        };

                                        match object {
                                            Value::IntDict(dict) => {
                                                if Arc::strong_count(dict) != 1 {
                                                    int_dict_loop_valid = false;
                                                    break;
                                                }
                                            }
                                            Value::DenseIntDict(values) => {
                                                if Arc::strong_count(values) != 1 {
                                                    int_dict_loop_valid = false;
                                                    break;
                                                }
                                            }
                                            Value::DenseIntDictInt(values) => {
                                                if Arc::strong_count(values) != 1 {
                                                    int_dict_loop_valid = false;
                                                    break;
                                                }
                                            }
                                            Value::DenseIntDictIntFull(values) => {
                                                if Arc::strong_count(values) != 1 {
                                                    int_dict_loop_valid = false;
                                                    break;
                                                }
                                            }
                                            Value::Dict(dict) => {
                                                if !dict.is_empty() {
                                                    int_dict_loop_valid = false;
                                                    break;
                                                }
                                            }
                                            _ => {
                                                int_dict_loop_valid = false;
                                                break;
                                            }
                                        }
                                    }
                                } else {
                                    int_dict_loop_valid = false;
                                }
                            }
                        }

                        let can_compile_loop = if int_dict_loop_valid && !int_dict_slots.is_empty()
                        {
                            self.jit_compiler.can_compile_loop_with_int_dicts(
                                &self.chunk,
                                *jump_target,
                                self.ip,
                            )
                        } else {
                            self.jit_compiler.can_compile_loop(&self.chunk, *jump_target, self.ip)
                        };

                        if can_compile_loop {
                            let mut store_vars = std::collections::HashSet::new();
                            for instr in
                                self.chunk.instructions.iter().take(self.ip + 1).skip(*jump_target)
                            {
                                match instr {
                                    OpCode::StoreVar(name) | OpCode::StoreGlobal(name) => {
                                        store_vars.insert(name.clone());
                                    }
                                    _ => {}
                                }
                            }

                            if store_vars.len() > 2 {
                                // Skip loop JIT for complex update patterns to preserve correctness
                                // (e.g., multiple dependent variable updates per iteration)
                                self.jit_compiler.mark_loop_jit_blocked(*jump_target);
                            } else {
                                // Try to compile this hot loop
                                // IMPORTANT: Compile from the loop START (jump_target), not from the JumpBack!
                                // The JumpBack just marks the end of the loop
                                let compile_result =
                                    if int_dict_loop_valid && !int_dict_slots.is_empty() {
                                        self.jit_compiler.compile_loop_with_int_dicts(
                                            &self.chunk,
                                            *jump_target,
                                            self.ip,
                                            int_dict_slots,
                                        )
                                    } else {
                                        self.jit_compiler.compile(&self.chunk, *jump_target)
                                    };

                                match compile_result {
                                    Ok(compiled_fn) => {
                                        if std::env::var("DEBUG_JIT_LOOP").is_ok() {
                                            eprintln!(
                                                "JIT: Compiled loop ({}..={})",
                                                jump_target, self.ip
                                            );
                                        }
                                        let jump_target = *jump_target;
                                        let loop_exit_ip;
                                        let mut max_target = self.ip;

                                        for instr in self
                                            .chunk
                                            .instructions
                                            .iter()
                                            .take(self.ip + 1)
                                            .skip(jump_target)
                                        {
                                            match instr {
                                                OpCode::Jump(target)
                                                | OpCode::JumpIfFalse(target)
                                                | OpCode::JumpIfTrue(target)
                                                | OpCode::JumpBack(target) => {
                                                    if *target > max_target {
                                                        max_target = *target;
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }

                                        loop_exit_ip = max_target + 1;

                                        // Successfully compiled! Now EXECUTE the compiled function
                                        if std::env::var("DEBUG_JIT").is_ok() {
                                            eprintln!(
                                                "JIT: Successfully compiled hot loop starting at offset {} - EXECUTING NOW!",
                                                jump_target
                                            );
                                        }
                                        if std::env::var("DEBUG_JIT_LOOP").is_ok() {
                                            eprintln!(
                                                "JIT: Executing compiled loop ({}..={})",
                                                jump_target, self.ip
                                            );
                                        }

                                        // Get VM pointer early (before any borrows)
                                        let vm_ptr: *mut std::ffi::c_void =
                                            self as *mut _ as *mut std::ffi::c_void;

                                        // Execute the JIT-compiled function
                                        // Get mutable pointers to VM state for VMContext
                                        let stack_ptr: *mut Vec<Value> = &mut self.stack;

                                        // Get globals - lock and get mutable reference to the first scope
                                        let mut globals_guard = self.globals.lock().unwrap();
                                        let globals_ptr: *mut HashMap<String, Value> =
                                            &mut globals_guard.scopes[0];

                                        // Get locals from current call frame, or use globals if at top level
                                        let locals_ptr: *mut HashMap<String, Value> =
                                            if let Some(frame) = self.call_frames.last_mut() {
                                                &mut frame.locals
                                            } else {
                                                // Top-level: use globals as locals
                                                globals_ptr
                                            };

                                        // Create VMContext with VM pointer for Call opcode support
                                        let mut vm_context = crate::jit::VMContext::new_with_vm(
                                            stack_ptr,
                                            locals_ptr,
                                            globals_ptr,
                                            vm_ptr,
                                        );
                                        vm_context.local_slots_ptr =
                                            match self.call_frames.last_mut() {
                                                Some(frame) => {
                                                    &mut frame.local_slots as *mut Vec<Value>
                                                }
                                                None => std::ptr::null_mut(),
                                            };
                                        vm_context.obj_stack_ptr =
                                            &mut self.jit_obj_stack as *mut Vec<Value>;

                                        let chunk_name =
                                            self.chunk.name.as_deref().unwrap_or("<script>");
                                        let cache_key = format!("loop:{}", chunk_name);
                                        if !self.jit_var_names_cache.contains_key(&cache_key) {
                                            let mut cached_var_names = HashMap::new();

                                            for instr in &self.chunk.instructions {
                                                match instr {
                                                    OpCode::LoadVar(name)
                                                    | OpCode::StoreVar(name)
                                                    | OpCode::LoadGlobal(name)
                                                    | OpCode::StoreGlobal(name) => {
                                                        use std::collections::hash_map::DefaultHasher;
                                                        use std::hash::{Hash, Hasher};
                                                        let mut hasher = DefaultHasher::new();
                                                        name.hash(&mut hasher);
                                                        let hash = hasher.finish();
                                                        cached_var_names.insert(hash, name.clone());
                                                    }
                                                    _ => {}
                                                }
                                            }

                                            self.jit_var_names_cache
                                                .insert(cache_key.clone(), cached_var_names);
                                        }

                                        let var_names_ptr: *mut HashMap<u64, String> = self
                                            .jit_var_names_cache
                                            .get_mut(&cache_key)
                                            .map(|v| v as *mut HashMap<u64, String>)
                                            .unwrap_or(std::ptr::null_mut());
                                        vm_context.var_names_ptr = var_names_ptr;

                                        // Execute the compiled function
                                        let result_code =
                                            invoke_compiled_fn(compiled_fn, &mut vm_context);

                                        // Drop the globals lock
                                        drop(globals_guard);

                                        if result_code != 0 {
                                            return Err(format!(
                                                "JIT execution failed with code: {}",
                                                result_code
                                            ));
                                        }

                                        if std::env::var("DEBUG_JIT").is_ok() {
                                            eprintln!("JIT: Execution completed successfully!");
                                        }

                                        // The JIT function executed the loop completely
                                        // Skip past the entire compiled loop range (including exit block)
                                        self.ip = loop_exit_ip;
                                        continue;
                                    }
                                    Err(e) => {
                                        // Compilation failed - this shouldn't happen if pre-scan worked
                                        if std::env::var("DEBUG_JIT").is_ok() {
                                            eprintln!(
                                                "JIT: Unexpected compilation failure at offset {}: {}",
                                                jump_target, e
                                            );
                                        }
                                        self.jit_compiler.mark_loop_jit_blocked(*jump_target);
                                    }
                                }
                            }
                        } else {
                            // Loop contains unsupported opcodes - skip JIT compilation
                            // This is normal for loops with function calls, strings, etc.
                            if std::env::var("DEBUG_JIT").is_ok() {
                                eprintln!(
                                    "JIT: Loop at offset {} contains unsupported opcodes, using interpreter",
                                    jump_target
                                );
                            }
                            self.jit_compiler.mark_loop_jit_blocked(*jump_target);
                        }
                    }
                }
            }

            let instruction = self.chunk.instructions[self.ip].clone();
            self.ip += 1;

            match instruction {
                OpCode::LoadConst(index) => {
                    let constant = &self.chunk.constants[index];
                    let value = self.constant_to_value(constant)?;
                    self.stack.push(value);
                }

                OpCode::LoadLocal(slot) => {
                    let frame = self.call_frames.last().ok_or("LoadLocal requires call frame")?;
                    let value = frame
                        .local_slots
                        .get(slot)
                        .cloned()
                        .ok_or_else(|| format!("Invalid local slot: {}", slot))?;
                    self.stack.push(value);
                }

                OpCode::LoadVar(name) => {
                    // Look in current call frame first - check captured variables (Arc<Mutex<Value>>) first, then locals
                    let value = if let Some(frame) = self.call_frames.last() {
                        if std::env::var("DEBUG_VM").is_ok() {
                            eprintln!("LoadVar('{}'):  checking frame captured ({} entries) and locals ({} entries)", 
                                name, frame.captured.len(), frame.locals.len());
                        }

                        // Check captured variables first (these are shared mutable references)
                        if let Some(captured_ref) = frame.captured.get(&name) {
                            if std::env::var("DEBUG_VM").is_ok() {
                                eprintln!("LoadVar('{}'): found in captured", name);
                            }
                            Some(captured_ref.lock().unwrap().clone())
                        } else {
                            // Fall back to locals
                            frame.locals.get(&name).cloned()
                        }
                    } else {
                        if std::env::var("DEBUG_VM").is_ok() {
                            eprintln!("LoadVar('{}'): no call frame", name);
                        }
                        None
                    };

                    let value = value
                        .or_else(|| {
                            let global_val = self.globals.lock().unwrap().get(&name);
                            if std::env::var("DEBUG_VM").is_ok() {
                                eprintln!(
                                    "LoadVar('{}'): checking globals -> {:?}",
                                    name,
                                    global_val.is_some()
                                );
                            }
                            global_val
                        })
                        .ok_or_else(|| {
                            if std::env::var("DEBUG_VM").is_ok() {
                                eprintln!(
                                    "LoadVar('{}'): FAILED - not in captured, locals or globals",
                                    name
                                );
                                eprintln!(
                                    "  Current frame captured: {:?}",
                                    self.call_frames
                                        .last()
                                        .map(|f| f.captured.keys().collect::<Vec<_>>())
                                );
                                eprintln!(
                                    "  Current frame locals: {:?}",
                                    self.call_frames
                                        .last()
                                        .map(|f| f.locals.keys().collect::<Vec<_>>())
                                );
                            }
                            Self::undefined_variable_message(&name)
                        })?;

                    self.stack.push(value);
                }

                OpCode::LoadGlobal(name) => {
                    let value = self
                        .globals
                        .lock()
                        .unwrap()
                        .get(&name)
                        .ok_or_else(|| Self::undefined_variable_message(&name))?;
                    self.stack.push(value);
                }

                OpCode::StoreVar(name) => {
                    let value = self.stack.last().ok_or("Stack underflow")?.clone();
                    let global_exists = self.globals.lock().unwrap().get(&name).is_some();
                    let mut assign_global = false;

                    if let Some(frame) = self.call_frames.last_mut() {
                        // Check if this is a captured variable first
                        if let Some(captured_ref) = frame.captured.get(&name) {
                            if let Some(kind) = frame.captured_binding_kinds.get(&name).copied() {
                                if !matches!(kind, BytecodeBindingKind::Mutable) {
                                    return Err(Self::local_reassignment_error(kind, &name));
                                }
                            }
                            if std::env::var("DEBUG_VM").is_ok() {
                                eprintln!("StoreVar('{}'): updating captured variable", name);
                            }
                            *captured_ref.lock().unwrap() = value.clone();
                        } else if frame.locals.contains_key(&name) {
                            if let Some(kind) = frame.locals_binding_kinds.get(&name).copied() {
                                if !matches!(kind, BytecodeBindingKind::Mutable) {
                                    return Err(Self::local_reassignment_error(kind, &name));
                                }
                            }
                            if std::env::var("DEBUG_VM").is_ok() {
                                eprintln!("StoreVar('{}'): updating frame local", name);
                            }
                            frame.locals.insert(name.clone(), value.clone());
                        } else if global_exists {
                            if std::env::var("DEBUG_VM").is_ok() {
                                eprintln!("StoreVar('{}'): updating global binding", name);
                            }
                            assign_global = true;
                        } else {
                            // Assignment to an unresolved name inside a frame defines
                            // a new mutable local, mirroring interpreter assign_checked.
                            if std::env::var("DEBUG_VM").is_ok() {
                                eprintln!("StoreVar('{}'): storing in frame locals", name);
                            }
                            frame
                                .locals_binding_kinds
                                .entry(name.clone())
                                .or_insert(BytecodeBindingKind::Mutable);
                            frame.locals.insert(name.clone(), value.clone());
                        }
                    } else {
                        assign_global = true;
                        if std::env::var("DEBUG_VM").is_ok() {
                            eprintln!("StoreVar('{}'): storing in globals (no frame)", name);
                        }
                    }

                    if assign_global {
                        self.globals.lock().unwrap().assign_checked(name, value)?;
                    }
                }

                OpCode::StoreLocal(slot) => {
                    let value = self.stack.last().ok_or("Stack underflow")?.clone();
                    let binding_name = self
                        .chunk
                        .local_names
                        .get(slot)
                        .cloned()
                        .unwrap_or_else(|| format!("<local:{}>", slot));
                    let frame =
                        self.call_frames.last_mut().ok_or("StoreLocal requires call frame")?;
                    if let Some(target) = frame.local_slots.get_mut(slot) {
                        let kind = frame
                            .local_slot_binding_kinds
                            .get(slot)
                            .copied()
                            .unwrap_or(BytecodeBindingKind::Mutable);
                        let initialized =
                            frame.local_slot_initialized.get(slot).copied().unwrap_or(false);
                        if initialized && !matches!(kind, BytecodeBindingKind::Mutable) {
                            return Err(Self::local_reassignment_error(kind, &binding_name));
                        }

                        *target = value;
                        if let Some(initialized_flag) = frame.local_slot_initialized.get_mut(slot) {
                            *initialized_flag = true;
                        }
                    } else {
                        return Err(format!("Invalid local slot: {}", slot));
                    }
                }

                OpCode::StoreGlobal(name) => {
                    let value = self.stack.last().ok_or("Stack underflow")?.clone();
                    self.globals.lock().unwrap().assign_checked(name, value)?;
                }

                OpCode::DefineGlobal(name, kind) => {
                    let value = self.stack.last().ok_or("Stack underflow")?.clone();
                    self.globals.lock().unwrap().define_with_kind_checked(
                        name,
                        value,
                        Self::env_binding_kind(kind),
                    )?;
                }

                OpCode::EnsureMutableGlobalForMutation(name) => {
                    self.globals.lock().unwrap().ensure_mutable_for_mutation(name.as_str())?;
                }

                OpCode::Pop => {
                    self.stack.pop().ok_or("Stack underflow")?;
                }

                OpCode::Dup => {
                    let value = self.stack.last().ok_or("Stack underflow")?.clone();
                    self.stack.push(value);
                }

                // Arithmetic operations
                OpCode::Add => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = match (left, right) {
                        (Value::Str(mut left_str), Value::Str(right_str)) => {
                            // ALWAYS use make_mut to get mutable access (clones if shared)
                            // This handles shadowing where old Arc is about to be dropped anyway
                            let result_str = Arc::make_mut(&mut left_str);
                            let needed = right_str.len();
                            let current_len = result_str.len();
                            let available = result_str.capacity() - current_len;

                            if available < needed {
                                // Aggressive growth for loop patterns: assume many more concatenations
                                // Python's string concat is fast because it opportunistically reuses buffers
                                let new_capacity = if current_len < 1000 {
                                    // Small strings: grow exponentially
                                    (result_str.capacity() * 2).max(current_len + needed * 50)
                                } else {
                                    // Large strings: grow by at least 50% or needed * 10
                                    result_str.capacity() + result_str.capacity() / 2 + needed * 10
                                };
                                result_str.reserve(new_capacity - result_str.capacity());
                            }
                            result_str.push_str(right_str.as_ref());
                            Value::Str(left_str)
                        }
                        (left_val, right_val) => self.binary_op(&left_val, "+", &right_val)?,
                    };
                    self.stack.push(result);
                }

                OpCode::AddInPlace(slot) => {
                    let rhs = self.stack.pop().ok_or("Stack underflow")?;
                    let apply_add = |target: &mut Value| -> Result<(), String> {
                        match (target, &rhs) {
                            (Value::Int(left), Value::Int(right)) => {
                                *left = Value::checked_int_arithmetic(*left, "+", *right)?;
                                Ok(())
                            }
                            (Value::Float(left), Value::Float(right)) => {
                                *left += *right;
                                Ok(())
                            }
                            (Value::Str(left), Value::Str(right)) => {
                                let left_str = Arc::make_mut(left);
                                let needed = right.len();
                                let available = left_str.capacity() - left_str.len();

                                if available < needed {
                                    // Aggressive pre-allocation for repeated concatenation
                                    let reserve_amount = (needed * 1000).max(left_str.capacity());
                                    left_str.reserve(reserve_amount);
                                }
                                left_str.push_str(right.as_ref());
                                Ok(())
                            }
                            _ => Err("Type mismatch in AddInPlace".to_string()),
                        }
                    };

                    let frame =
                        self.call_frames.last_mut().ok_or("AddInPlace requires call frame")?;
                    let target = frame
                        .local_slots
                        .get_mut(slot)
                        .ok_or_else(|| format!("Invalid local slot: {}", slot))?;
                    apply_add(target)?;
                    self.stack.push(Value::Null);
                }

                OpCode::AppendConstStringInPlace(slot, rhs) => {
                    let frame = self
                        .call_frames
                        .last_mut()
                        .ok_or("AppendConstStringInPlace requires call frame")?;
                    let target = frame
                        .local_slots
                        .get_mut(slot)
                        .ok_or_else(|| format!("Invalid local slot: {}", slot))?;

                    match target {
                        Value::Str(left) => {
                            let left_str = Arc::make_mut(left);
                            let needed = rhs.len();
                            let available = left_str.capacity() - left_str.len();

                            if available < needed {
                                let reserve_amount = (needed * 1000).max(left_str.capacity());
                                left_str.reserve(reserve_amount);
                            }

                            left_str.push_str(rhs.as_ref());
                        }
                        _ => return Err("Type mismatch in AppendConstStringInPlace".to_string()),
                    }
                }

                OpCode::AppendConstCharInPlace(slot, rhs) => {
                    let frame = self
                        .call_frames
                        .last_mut()
                        .ok_or("AppendConstCharInPlace requires call frame")?;
                    let target = frame
                        .local_slots
                        .get_mut(slot)
                        .ok_or_else(|| format!("Invalid local slot: {}", slot))?;

                    match target {
                        Value::Str(left) => {
                            let left_str = Arc::make_mut(left);
                            let needed = rhs.len_utf8();
                            let available = left_str.capacity() - left_str.len();

                            if available < needed {
                                let reserve_amount = (needed * 1000).max(left_str.capacity());
                                left_str.reserve(reserve_amount);
                            }

                            left_str.push(rhs);
                        }
                        _ => return Err("Type mismatch in AppendConstCharInPlace".to_string()),
                    }
                }

                OpCode::AppendConstCharUntilLocalInPlace(
                    target_slot,
                    index_slot,
                    limit_slot,
                    rhs,
                ) => {
                    let frame = self
                        .call_frames
                        .last_mut()
                        .ok_or("AppendConstCharUntilLocalInPlace requires call frame")?;

                    let current_index = match frame.local_slots.get(index_slot) {
                        Some(Value::Int(value)) => *value,
                        Some(_) => {
                            return Err("Type mismatch in AppendConstCharUntilLocalInPlace index"
                                .to_string())
                        }
                        None => return Err(format!("Invalid local slot: {}", index_slot)),
                    };

                    let limit_index = match frame.local_slots.get(limit_slot) {
                        Some(Value::Int(value)) => *value,
                        Some(_) => {
                            return Err("Type mismatch in AppendConstCharUntilLocalInPlace limit"
                                .to_string())
                        }
                        None => return Err(format!("Invalid local slot: {}", limit_slot)),
                    };

                    if limit_index > current_index {
                        let repeat_count_i64 = limit_index - current_index;
                        let repeat_count = usize::try_from(repeat_count_i64).map_err(|_| {
                            "Repeat count overflow in AppendConstCharUntilLocalInPlace".to_string()
                        })?;

                        let target = frame
                            .local_slots
                            .get_mut(target_slot)
                            .ok_or_else(|| format!("Invalid local slot: {}", target_slot))?;

                        match target {
                            Value::Str(left) => {
                                let left_str = Arc::make_mut(left);
                                let needed = rhs.len_utf8().saturating_mul(repeat_count);
                                let available = left_str.capacity() - left_str.len();

                                if available < needed {
                                    let reserve_amount = needed.max(left_str.capacity());
                                    left_str.reserve(reserve_amount);
                                }

                                for _ in 0..repeat_count {
                                    left_str.push(rhs);
                                }
                            }
                            _ => {
                                return Err(
                                    "Type mismatch in AppendConstCharUntilLocalInPlace target"
                                        .to_string(),
                                )
                            }
                        }
                    }

                    if let Some(index_value) = frame.local_slots.get_mut(index_slot) {
                        *index_value = Value::Int(limit_index);
                    } else {
                        return Err(format!("Invalid local slot: {}", index_slot));
                    }
                }

                OpCode::FillIntMapWithDoubleUntilLocalInPlace(map_slot, index_slot, limit_slot) => {
                    let frame = self
                        .call_frames
                        .last_mut()
                        .ok_or("FillIntMapWithDoubleUntilLocalInPlace requires call frame")?;

                    let current_index = match frame.local_slots.get(index_slot) {
                        Some(Value::Int(value)) => *value,
                        Some(_) => {
                            return Err(
                                "Type mismatch in FillIntMapWithDoubleUntilLocalInPlace index"
                                    .to_string(),
                            )
                        }
                        None => return Err(format!("Invalid local slot: {}", index_slot)),
                    };

                    let limit_index = match frame.local_slots.get(limit_slot) {
                        Some(Value::Int(value)) => *value,
                        Some(_) => {
                            return Err(
                                "Type mismatch in FillIntMapWithDoubleUntilLocalInPlace limit"
                                    .to_string(),
                            )
                        }
                        None => return Err(format!("Invalid local slot: {}", limit_slot)),
                    };

                    if limit_index > current_index {
                        let target = frame
                            .local_slots
                            .get_mut(map_slot)
                            .ok_or_else(|| format!("Invalid local slot: {}", map_slot))?;

                        match target {
                            Value::DenseIntDictIntFull(values) => {
                                let values_mut = Arc::make_mut(values);
                                let start = usize::try_from(current_index).map_err(|_| {
                                    "Negative index in FillIntMapWithDoubleUntilLocalInPlace"
                                        .to_string()
                                })?;
                                let end = usize::try_from(limit_index).map_err(|_| {
                                    "Negative limit in FillIntMapWithDoubleUntilLocalInPlace"
                                        .to_string()
                                })?;

                                if values_mut.len() < end {
                                    values_mut.reserve(end - values_mut.len());
                                }

                                for index in start..end {
                                    let index_i64 = index as i64;
                                    let value = Value::checked_int_arithmetic(index_i64, "*", 2)?;
                                    if index == values_mut.len() {
                                        values_mut.push(value);
                                    } else if index < values_mut.len() {
                                        values_mut[index] = value;
                                    } else {
                                        values_mut.resize(index + 1, 0);
                                        values_mut[index] = value;
                                    }
                                }
                            }
                            Value::DenseIntDictInt(values) => {
                                let values_mut = Arc::make_mut(values);
                                let start = usize::try_from(current_index).map_err(|_| {
                                    "Negative index in FillIntMapWithDoubleUntilLocalInPlace"
                                        .to_string()
                                })?;
                                let end = usize::try_from(limit_index).map_err(|_| {
                                    "Negative limit in FillIntMapWithDoubleUntilLocalInPlace"
                                        .to_string()
                                })?;

                                if values_mut.len() < end {
                                    values_mut.resize(end, None);
                                }

                                for index in start..end {
                                    let index_i64 = index as i64;
                                    values_mut[index] =
                                        Some(Value::checked_int_arithmetic(index_i64, "*", 2)?);
                                }
                            }
                            Value::DenseIntDict(values) => {
                                let values_mut = Arc::make_mut(values);
                                let start = usize::try_from(current_index).map_err(|_| {
                                    "Negative index in FillIntMapWithDoubleUntilLocalInPlace"
                                        .to_string()
                                })?;
                                let end = usize::try_from(limit_index).map_err(|_| {
                                    "Negative limit in FillIntMapWithDoubleUntilLocalInPlace"
                                        .to_string()
                                })?;

                                if values_mut.len() < end {
                                    values_mut.resize(end, Value::Null);
                                }

                                for index in start..end {
                                    let index_i64 = index as i64;
                                    values_mut[index] = Value::Int(Value::checked_int_arithmetic(
                                        index_i64, "*", 2,
                                    )?);
                                }
                            }
                            Value::IntDict(dict) => {
                                let dict_mut = Arc::make_mut(dict);
                                for index in current_index..limit_index {
                                    let doubled = Value::checked_int_arithmetic(index, "*", 2)?;
                                    dict_mut.insert(index, Value::Int(doubled));
                                }
                            }
                            Value::Dict(dict) => {
                                if dict.is_empty() && current_index >= 0 {
                                    let end = usize::try_from(limit_index).map_err(|_| {
                                        "Negative limit in FillIntMapWithDoubleUntilLocalInPlace"
                                            .to_string()
                                    })?;
                                    let start = usize::try_from(current_index).map_err(|_| {
                                        "Negative index in FillIntMapWithDoubleUntilLocalInPlace"
                                            .to_string()
                                    })?;

                                    let mut values = vec![0; end];
                                    for index in start..end {
                                        let index_i64 = index as i64;
                                        values[index] =
                                            Value::checked_int_arithmetic(index_i64, "*", 2)?;
                                    }
                                    *target = Value::DenseIntDictIntFull(Arc::new(values));
                                } else {
                                    let dict_mut = Arc::make_mut(dict);
                                    for index in current_index..limit_index {
                                        let doubled = Value::checked_int_arithmetic(index, "*", 2)?;
                                        dict_mut.insert(
                                            Arc::from(index.to_string().as_str()),
                                            Value::Int(doubled),
                                        );
                                    }
                                }
                            }
                            Value::FixedDict { keys, values } => {
                                if keys.is_empty() && values.is_empty() && current_index >= 0 {
                                    let end = usize::try_from(limit_index).map_err(|_| {
                                        "Negative limit in FillIntMapWithDoubleUntilLocalInPlace"
                                            .to_string()
                                    })?;
                                    let start = usize::try_from(current_index).map_err(|_| {
                                        "Negative index in FillIntMapWithDoubleUntilLocalInPlace"
                                            .to_string()
                                    })?;

                                    let mut dense_values = vec![0; end];
                                    for index in start..end {
                                        let index_i64 = index as i64;
                                        dense_values[index] =
                                            Value::checked_int_arithmetic(index_i64, "*", 2)?;
                                    }
                                    *target = Value::DenseIntDictIntFull(Arc::new(dense_values));
                                } else {
                                    let mut dict = DictMap::default();
                                    for (key, value) in
                                        keys.iter().cloned().zip(values.iter().cloned())
                                    {
                                        dict.insert(key, value);
                                    }
                                    for index in current_index..limit_index {
                                        let doubled = Value::checked_int_arithmetic(index, "*", 2)?;
                                        dict.insert(
                                            Arc::from(index.to_string().as_str()),
                                            Value::Int(doubled),
                                        );
                                    }
                                    *target = Value::Dict(Arc::new(dict));
                                }
                            }
                            _ => {
                                return Err(
                                    "Type mismatch in FillIntMapWithDoubleUntilLocalInPlace map"
                                        .to_string(),
                                )
                            }
                        }
                    }

                    if let Some(index_value) = frame.local_slots.get_mut(index_slot) {
                        *index_value = Value::Int(limit_index);
                    } else {
                        return Err(format!("Invalid local slot: {}", index_slot));
                    }
                }

                OpCode::SumIntMapUntilLocalInPlace(map_slot, sum_slot, index_slot, limit_slot) => {
                    let frame = self
                        .call_frames
                        .last_mut()
                        .ok_or("SumIntMapUntilLocalInPlace requires call frame")?;

                    let current_index = match frame.local_slots.get(index_slot) {
                        Some(Value::Int(value)) => *value,
                        Some(_) => {
                            return Err(
                                "Type mismatch in SumIntMapUntilLocalInPlace index".to_string()
                            )
                        }
                        None => return Err(format!("Invalid local slot: {}", index_slot)),
                    };

                    let limit_index = match frame.local_slots.get(limit_slot) {
                        Some(Value::Int(value)) => *value,
                        Some(_) => {
                            return Err(
                                "Type mismatch in SumIntMapUntilLocalInPlace limit".to_string()
                            )
                        }
                        None => return Err(format!("Invalid local slot: {}", limit_slot)),
                    };

                    let mut running_sum = match frame.local_slots.get(sum_slot) {
                        Some(Value::Int(value)) => *value,
                        Some(_) => {
                            return Err(
                                "Type mismatch in SumIntMapUntilLocalInPlace sum".to_string()
                            )
                        }
                        None => return Err(format!("Invalid local slot: {}", sum_slot)),
                    };

                    let map_value = frame
                        .local_slots
                        .get(map_slot)
                        .cloned()
                        .ok_or_else(|| format!("Invalid local slot: {}", map_slot))?;

                    if limit_index > current_index {
                        let start = usize::try_from(current_index).map_err(|_| {
                            "Negative index in SumIntMapUntilLocalInPlace".to_string()
                        })?;
                        let end = usize::try_from(limit_index).map_err(|_| {
                            "Negative limit in SumIntMapUntilLocalInPlace".to_string()
                        })?;

                        match map_value {
                            Value::DenseIntDictIntFull(values) => {
                                if end > values.len() {
                                    return Err("Type mismatch in binary operation".to_string());
                                }

                                for value in values[start..end].iter() {
                                    running_sum =
                                        Value::checked_int_arithmetic(running_sum, "+", *value)?;
                                }
                            }
                            Value::DenseIntDictInt(values) => {
                                if end > values.len() {
                                    return Err("Type mismatch in binary operation".to_string());
                                }

                                for value in values[start..end].iter() {
                                    match value {
                                        Some(int_value) => {
                                            running_sum = Value::checked_int_arithmetic(
                                                running_sum,
                                                "+",
                                                *int_value,
                                            )?;
                                        }
                                        None => {
                                            return Err(
                                                "Type mismatch in binary operation".to_string()
                                            )
                                        }
                                    }
                                }
                            }
                            Value::DenseIntDict(values) => {
                                if end > values.len() {
                                    return Err("Type mismatch in binary operation".to_string());
                                }

                                for value in values[start..end].iter() {
                                    match value {
                                        Value::Int(int_value) => {
                                            running_sum = Value::checked_int_arithmetic(
                                                running_sum,
                                                "+",
                                                *int_value,
                                            )?;
                                        }
                                        _ => {
                                            return Err(
                                                "Type mismatch in binary operation".to_string()
                                            )
                                        }
                                    }
                                }
                            }
                            Value::IntDict(dict) => {
                                for key in current_index..limit_index {
                                    match dict.get(&key) {
                                        Some(Value::Int(int_value)) => {
                                            running_sum = Value::checked_int_arithmetic(
                                                running_sum,
                                                "+",
                                                *int_value,
                                            )?;
                                        }
                                        _ => {
                                            return Err(
                                                "Type mismatch in binary operation".to_string()
                                            )
                                        }
                                    }
                                }
                            }
                            Value::Dict(dict) => {
                                for key in current_index..limit_index {
                                    let key_string = key.to_string();
                                    match dict.get(key_string.as_str()) {
                                        Some(Value::Int(int_value)) => {
                                            running_sum = Value::checked_int_arithmetic(
                                                running_sum,
                                                "+",
                                                *int_value,
                                            )?;
                                        }
                                        _ => {
                                            return Err(
                                                "Type mismatch in binary operation".to_string()
                                            )
                                        }
                                    }
                                }
                            }
                            Value::FixedDict { keys, values } => {
                                for key in current_index..limit_index {
                                    let key_string = key.to_string();
                                    let match_index = keys
                                        .iter()
                                        .position(|item| item.as_ref() == key_string.as_str());

                                    match match_index.and_then(|idx| values.get(idx)) {
                                        Some(Value::Int(int_value)) => {
                                            running_sum = Value::checked_int_arithmetic(
                                                running_sum,
                                                "+",
                                                *int_value,
                                            )?;
                                        }
                                        _ => {
                                            return Err(
                                                "Type mismatch in binary operation".to_string()
                                            )
                                        }
                                    }
                                }
                            }
                            _ => {
                                return Err(
                                    "Type mismatch in SumIntMapUntilLocalInPlace map".to_string()
                                )
                            }
                        }
                    }

                    if let Some(sum_value) = frame.local_slots.get_mut(sum_slot) {
                        *sum_value = Value::Int(running_sum);
                    } else {
                        return Err(format!("Invalid local slot: {}", sum_slot));
                    }

                    if let Some(index_value) = frame.local_slots.get_mut(index_slot) {
                        *index_value = Value::Int(limit_index);
                    } else {
                        return Err(format!("Invalid local slot: {}", index_slot));
                    }
                }

                OpCode::Sub => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.binary_op(&left, "-", &right)?;
                    self.stack.push(result);
                }

                OpCode::Mul => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.binary_op(&left, "*", &right)?;
                    self.stack.push(result);
                }

                OpCode::Div => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.binary_op(&left, "/", &right)?;
                    self.stack.push(result);
                }

                OpCode::Mod => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.binary_op(&left, "%", &right)?;
                    self.stack.push(result);
                }

                OpCode::Negate => {
                    let value = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.unary_op("-", &value)?;
                    self.stack.push(result);
                }

                // Comparison operations
                OpCode::Equal => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = Value::Bool(self.values_equal(&left, &right));
                    self.stack.push(result);
                }

                OpCode::NotEqual => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = Value::Bool(!self.values_equal(&left, &right));
                    self.stack.push(result);
                }

                OpCode::LessThan => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.compare_op(&left, "<", &right)?;
                    self.stack.push(result);
                }

                OpCode::GreaterThan => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.compare_op(&left, ">", &right)?;
                    self.stack.push(result);
                }

                OpCode::LessEqual => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.compare_op(&left, "<=", &right)?;
                    self.stack.push(result);
                }

                OpCode::GreaterEqual => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.compare_op(&left, ">=", &right)?;
                    self.stack.push(result);
                }

                // Logical operations
                OpCode::Not => {
                    let value = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.unary_op("!", &value)?;
                    self.stack.push(result);
                }

                OpCode::And => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = Value::Bool(self.is_truthy(&left) && self.is_truthy(&right));
                    self.stack.push(result);
                }

                OpCode::Or => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = Value::Bool(self.is_truthy(&left) || self.is_truthy(&right));
                    self.stack.push(result);
                }

                // Control flow
                OpCode::Jump(target) => {
                    self.ip = target;
                }

                OpCode::JumpIfFalse(target) => {
                    let condition = self.stack.last().ok_or("Stack underflow")?;
                    if !self.is_truthy(condition) {
                        self.ip = target;
                    }
                }

                OpCode::JumpIfTrue(target) => {
                    let condition = self.stack.last().ok_or("Stack underflow")?;
                    if self.is_truthy(condition) {
                        self.ip = target;
                    }
                }

                OpCode::JumpBack(target) => {
                    self.ip = target;
                }

                // Function operations
                OpCode::Call(arg_count) => {
                    // Create call site ID for inline cache lookup
                    // This identifies where in the bytecode this call occurs
                    let call_site_id = CallSiteId::new(self.chunk.name.as_deref(), self.ip);

                    // Function is on top of stack, then arguments below it
                    // Stack layout: [... arg1, arg2, ..., argN, function]
                    let function = self.stack.pop().ok_or("Stack underflow in Call")?;

                    // Collect arguments
                    let mut args = Vec::new();
                    for _ in 0..arg_count {
                        args.push(self.stack.pop().ok_or("Stack underflow in Call args")?);
                    }
                    args.reverse(); // Arguments were pushed in order

                    // Check if this is a bytecode function or native function
                    match &function {
                        Value::BytecodeFunction {
                            chunk,
                            captured: _,
                            captured_binding_kinds: _,
                        } => {
                            let raw_args = args.clone();
                            let args = self.prepare_bytecode_call_args(chunk, args.clone())?;

                            // Track function calls for JIT compilation
                            if self.jit_enabled && !chunk.is_generator {
                                let func_name = chunk.name.as_deref().unwrap_or("<anonymous>");

                                // Get VM pointer early (before any borrows)
                                let vm_ptr: *mut std::ffi::c_void =
                                    self as *mut _ as *mut std::ffi::c_void;

                                // === INLINE CACHE FAST PATH ===
                                // Check inline cache for this specific call site
                                // This is faster than HashMap lookups because:
                                // 1. We cache the compiled_fn pointer directly (no string hash)
                                // 2. We cache var_names to avoid rebuilding on every call
                                // 3. We validate with a simple string comparison (guard)
                                if let Some(cache_entry) = self.inline_cache.get_mut(&call_site_id)
                                {
                                    // Cache hit! Validate that function hasn't changed (guard)
                                    if cache_entry.expected_func_name == func_name {
                                        cache_entry.hit_count += 1;

                                        // If we have a compiled function, use it directly
                                        if let Some(compiled_fn) = cache_entry.compiled_fn {
                                            // Create locals HashMap for the function parameters
                                            let mut func_locals = HashMap::new();

                                            let has_loop = chunk
                                                .instructions
                                                .iter()
                                                .any(|op| matches!(op, OpCode::JumpBack(_)));
                                            // Check if we can use fast arg passing (≤4 integer args)
                                            let use_fast_args = !has_loop
                                                && args.len() <= 4
                                                && args.iter().all(|a| matches!(a, Value::Int(_)));

                                            // Bind arguments to parameter names
                                            for (i, param_name) in chunk.params.iter().enumerate() {
                                                if let Some(arg) = args.get(i) {
                                                    func_locals
                                                        .insert(param_name.clone(), arg.clone());
                                                }
                                            }

                                            // OPTIMIZATION: Get mutable pointer directly to cached var_names
                                            // This avoids HashMap clone on every call!
                                            let var_names_ptr: *mut HashMap<u64, String> =
                                                &mut cache_entry.var_names;

                                            // Save stack size to detect return value
                                            let stack_size_before = self.stack.len();

                                            // Execute the JIT-compiled function
                                            let stack_ptr: *mut Vec<Value> = &mut self.stack;

                                            // Get globals - drop lock before execution
                                            let globals_ptr: *mut HashMap<String, Value> = {
                                                let mut globals_guard =
                                                    self.globals.lock().unwrap();
                                                let ptr = &mut globals_guard.scopes[0]
                                                    as *mut HashMap<String, Value>;
                                                drop(globals_guard);
                                                ptr
                                            };

                                            let locals_ptr: *mut HashMap<String, Value> =
                                                &mut func_locals;
                                            let local_slots_ptr: *mut Vec<Value> =
                                                match self.call_frames.last_mut() {
                                                    Some(frame) => {
                                                        &mut frame.local_slots as *mut Vec<Value>
                                                    }
                                                    None => std::ptr::null_mut(),
                                                };
                                            // var_names_ptr already created above from cache entry

                                            // Create VMContext with fast arg fields
                                            let mut vm_context = crate::jit::VMContext {
                                                stack_ptr,
                                                locals_ptr,
                                                globals_ptr,
                                                var_names_ptr,
                                                local_slots_ptr,
                                                obj_stack_ptr: &mut self.jit_obj_stack
                                                    as *mut Vec<Value>,
                                                vm_ptr,
                                                return_value: 0,
                                                has_return_value: false,
                                                arg0: if use_fast_args && args.len() > 0 {
                                                    if let Value::Int(n) = args[0] {
                                                        n
                                                    } else {
                                                        0
                                                    }
                                                } else {
                                                    0
                                                },
                                                arg1: if use_fast_args && args.len() > 1 {
                                                    if let Value::Int(n) = args[1] {
                                                        n
                                                    } else {
                                                        0
                                                    }
                                                } else {
                                                    0
                                                },
                                                arg2: if use_fast_args && args.len() > 2 {
                                                    if let Value::Int(n) = args[2] {
                                                        n
                                                    } else {
                                                        0
                                                    }
                                                } else {
                                                    0
                                                },
                                                arg3: if use_fast_args && args.len() > 3 {
                                                    if let Value::Int(n) = args[3] {
                                                        n
                                                    } else {
                                                        0
                                                    }
                                                } else {
                                                    0
                                                },
                                                arg_count: args.len() as i64,
                                            };

                                            let result_code =
                                                invoke_compiled_fn(compiled_fn, &mut vm_context);

                                            if result_code != 0 {
                                                return Err(format!(
                                                    "JIT execution failed with code: {}",
                                                    result_code
                                                ));
                                            }

                                            if vm_context.has_return_value {
                                                self.stack
                                                    .push(Value::Int(vm_context.return_value));
                                            } else if self.stack.len() > stack_size_before {
                                                // Return value was pushed to stack
                                            } else {
                                                return Err(
                                                    "JIT-compiled function did not return a value"
                                                        .to_string(),
                                                );
                                            }

                                            continue; // Skip to next instruction
                                        }
                                    } else {
                                        // Cache miss - function at this call site changed (polymorphic)
                                        cache_entry.miss_count += 1;
                                    }
                                }

                                // === SLOW PATH - populate cache and execute ===
                                // PHASE 7 STEP 12: Check for direct-arg version FIRST for single-int-arg calls
                                // This is the key optimization for recursive functions - avoids FFI on each call
                                if args.len() == 1 {
                                    if let Value::Int(arg_val) = args[0] {
                                        if let Some(fn_info) = self.compiled_fn_info.get(func_name)
                                        {
                                            if let Some(direct_fn) = fn_info.fn_with_arg {
                                                // ULTRA-FAST PATH: Use direct-arg JIT variant!
                                                // This function takes an i64 directly and returns the result
                                                // WITHOUT going through VMContext arg fields or FFI for recursion

                                                // Create minimal VMContext for the function
                                                let stack_ptr: *mut Vec<Value> = &mut self.stack;
                                                let globals_ptr: *mut HashMap<String, Value> = {
                                                    let mut globals_guard =
                                                        self.globals.lock().unwrap();
                                                    let ptr = &mut globals_guard.scopes[0]
                                                        as *mut HashMap<String, Value>;
                                                    drop(globals_guard);
                                                    ptr
                                                };
                                                let mut func_locals = HashMap::new();
                                                let locals_ptr: *mut HashMap<String, Value> =
                                                    &mut func_locals;
                                                let local_slots_ptr: *mut Vec<Value> =
                                                    match self.call_frames.last_mut() {
                                                        Some(frame) => {
                                                            &mut frame.local_slots
                                                                as *mut Vec<Value>
                                                        }
                                                        None => std::ptr::null_mut(),
                                                    };

                                                let mut vm_context = crate::jit::VMContext {
                                                    stack_ptr,
                                                    locals_ptr,
                                                    globals_ptr,
                                                    var_names_ptr: std::ptr::null_mut(),
                                                    local_slots_ptr,
                                                    obj_stack_ptr: &mut self.jit_obj_stack
                                                        as *mut Vec<Value>,
                                                    vm_ptr,
                                                    return_value: 0,
                                                    has_return_value: false,
                                                    arg0: arg_val,
                                                    arg1: 0,
                                                    arg2: 0,
                                                    arg3: 0,
                                                    arg_count: 1,
                                                };

                                                // Execute direct-arg function - result is returned directly!
                                                let result = invoke_compiled_fn_with_arg(
                                                    direct_fn,
                                                    &mut vm_context,
                                                    arg_val,
                                                );

                                                if std::env::var("DEBUG_JIT").is_ok() {
                                                    eprintln!("JIT: Interpreter direct-arg call to '{}' with arg {} returned {}", 
                                                        func_name, arg_val, result);
                                                }

                                                self.stack.push(Value::Int(result));
                                                continue; // Skip to next instruction
                                            }
                                        }
                                    }
                                }

                                // Check if we have a JIT-compiled version (standard path)
                                if let Some(compiled_fn) = self.compiled_functions.get(func_name) {
                                    // Fast path: Call JIT-compiled version

                                    // Create locals HashMap for the function parameters
                                    let mut func_locals = HashMap::new();

                                    // Bind arguments to parameter names
                                    for (i, param_name) in chunk.params.iter().enumerate() {
                                        if let Some(arg) = args.get(i) {
                                            func_locals.insert(param_name.clone(), arg.clone());
                                        }
                                    }

                                    // Get or create var_names from cache
                                    let func_name_owned = func_name.to_string();
                                    let var_names = if let Some(cached) =
                                        self.jit_var_names_cache.get(&func_name_owned)
                                    {
                                        cached.clone()
                                    } else {
                                        // Build var_names once and cache it
                                        let mut cached_var_names = HashMap::new();

                                        // Register parameter names
                                        for param_name in &chunk.params {
                                            use std::collections::hash_map::DefaultHasher;
                                            use std::hash::{Hash, Hasher};
                                            let mut hasher = DefaultHasher::new();
                                            param_name.hash(&mut hasher);
                                            let hash = hasher.finish();
                                            cached_var_names.insert(hash, param_name.clone());
                                        }

                                        // Register all LoadVar names
                                        for instr in &chunk.instructions {
                                            if let OpCode::LoadVar(name) = instr {
                                                use std::collections::hash_map::DefaultHasher;
                                                use std::hash::{Hash, Hasher};
                                                let mut hasher = DefaultHasher::new();
                                                name.hash(&mut hasher);
                                                let hash = hasher.finish();
                                                cached_var_names.insert(hash, name.clone());
                                            }
                                        }

                                        self.jit_var_names_cache.insert(
                                            func_name_owned.clone(),
                                            cached_var_names.clone(),
                                        );
                                        cached_var_names
                                    };

                                    // === POPULATE INLINE CACHE ===
                                    // Store in inline cache for faster lookup next time
                                    self.inline_cache.insert(
                                        call_site_id,
                                        InlineCacheEntry::new(
                                            func_name,
                                            Some(*compiled_fn),
                                            var_names.clone(),
                                        ),
                                    );

                                    let mut var_names_mut = var_names;

                                    // Save stack size to detect return value
                                    let stack_size_before = self.stack.len();

                                    // Execute the JIT-compiled function
                                    // Get mutable pointers to VM state for VMContext
                                    let stack_ptr: *mut Vec<Value> = &mut self.stack;

                                    // Get globals - drop lock before execution to avoid deadlock on recursive calls
                                    let globals_ptr: *mut HashMap<String, Value> = {
                                        let mut globals_guard = self.globals.lock().unwrap();
                                        let ptr = &mut globals_guard.scopes[0]
                                            as *mut HashMap<String, Value>;
                                        drop(globals_guard);
                                        ptr
                                    };

                                    // Use the function's locals (with bound parameters)
                                    let locals_ptr: *mut HashMap<String, Value> = &mut func_locals;
                                    let local_slots_ptr: *mut Vec<Value> = match self
                                        .call_frames
                                        .last_mut()
                                    {
                                        Some(frame) => &mut frame.local_slots as *mut Vec<Value>,
                                        None => std::ptr::null_mut(),
                                    };

                                    // Set up var_names for JIT variable resolution
                                    let var_names_ptr: *mut HashMap<u64, String> =
                                        &mut var_names_mut;

                                    let has_loop = chunk
                                        .instructions
                                        .iter()
                                        .any(|op| matches!(op, OpCode::JumpBack(_)));
                                    // Check if we can use fast arg passing (≤4 integer args)
                                    let use_fast_args = !has_loop
                                        && args.len() <= 4
                                        && args.iter().all(|a| matches!(a, Value::Int(_)));

                                    // Create VMContext with fast arg fields
                                    let mut vm_context = crate::jit::VMContext {
                                        stack_ptr,
                                        locals_ptr,
                                        globals_ptr,
                                        var_names_ptr,
                                        local_slots_ptr,
                                        obj_stack_ptr: &mut self.jit_obj_stack as *mut Vec<Value>,
                                        vm_ptr,
                                        return_value: 0,
                                        has_return_value: false,
                                        arg0: if use_fast_args && args.len() > 0 {
                                            if let Value::Int(n) = args[0] {
                                                n
                                            } else {
                                                0
                                            }
                                        } else {
                                            0
                                        },
                                        arg1: if use_fast_args && args.len() > 1 {
                                            if let Value::Int(n) = args[1] {
                                                n
                                            } else {
                                                0
                                            }
                                        } else {
                                            0
                                        },
                                        arg2: if use_fast_args && args.len() > 2 {
                                            if let Value::Int(n) = args[2] {
                                                n
                                            } else {
                                                0
                                            }
                                        } else {
                                            0
                                        },
                                        arg3: if use_fast_args && args.len() > 3 {
                                            if let Value::Int(n) = args[3] {
                                                n
                                            } else {
                                                0
                                            }
                                        } else {
                                            0
                                        },
                                        arg_count: args.len() as i64,
                                    };

                                    // Execute the compiled function!
                                    // Lock is NOT held during execution to allow recursive calls
                                    let result_code =
                                        invoke_compiled_fn(*compiled_fn, &mut vm_context);

                                    if result_code != 0 {
                                        return Err(format!(
                                            "JIT execution failed with code: {}",
                                            result_code
                                        ));
                                    }

                                    // Check for return value - prefer optimized VMContext.return_value
                                    // This is the FAST PATH from Phase 7 Step 8 optimization
                                    if vm_context.has_return_value {
                                        // Use the optimized return value directly
                                        self.stack.push(Value::Int(vm_context.return_value));
                                    } else if self.stack.len() > stack_size_before {
                                        // Fallback: return value was pushed to stack (old path)
                                        // No action needed - value is already on stack
                                    } else {
                                        return Err("JIT-compiled function did not return a value"
                                            .to_string());
                                    }

                                    // Skip the normal bytecode execution
                                    continue;
                                }

                                // Increment call counter
                                let count = self
                                    .function_call_counts
                                    .entry(func_name.to_string())
                                    .or_insert(0);
                                *count += 1;

                                // Check if we should JIT-compile this function
                                let has_loop = chunk
                                    .instructions
                                    .iter()
                                    .any(|op| matches!(op, OpCode::JumpBack(_)));

                                let has_map_fusion_op = chunk.instructions.iter().any(|op| {
                                    matches!(
                                        op,
                                        OpCode::SumIntMapUntilLocalInPlace(_, _, _, _)
                                            | OpCode::FillIntMapWithDoubleUntilLocalInPlace(
                                                _,
                                                _,
                                                _
                                            )
                                    )
                                });

                                let allow_function_jit = std::env::var("DISABLE_FUNCTION_JIT")
                                    .is_err()
                                    && !has_map_fusion_op;
                                if allow_function_jit
                                    && (*count == JIT_FUNCTION_THRESHOLD
                                        || (has_loop && *count == 1))
                                {
                                    if std::env::var("DEBUG_JIT").is_ok() {
                                        eprintln!(
                                            "JIT: Function '{}' hit threshold ({} calls), attempting compilation...",
                                            func_name, JIT_FUNCTION_THRESHOLD
                                        );
                                        // Dump bytecode for debugging
                                        eprintln!("JIT: Bytecode for '{}':", func_name);
                                        for (pc, instr) in chunk.instructions.iter().enumerate() {
                                            eprintln!("  {:3}: {:?}", pc, instr);
                                        }
                                        // Dump constants
                                        eprintln!("JIT: Constants for '{}':", func_name);
                                        for (idx, constant) in chunk.constants.iter().enumerate() {
                                            eprintln!("  {:3}: {:?}", idx, constant);
                                        }
                                    }

                                    // Attempt to compile the function with enhanced info
                                    // This creates both standard and direct-arg variants for recursion
                                    match self
                                        .jit_compiler
                                        .compile_function_with_info(chunk, func_name)
                                    {
                                        Ok(info) => {
                                            if std::env::var("DEBUG_JIT").is_ok() {
                                                eprintln!(
                                                    "JIT: Successfully compiled function '{}'",
                                                    func_name
                                                );
                                            }

                                            self.compiled_functions
                                                .insert(func_name.to_string(), info.fn_ptr);
                                            self.compiled_fn_info
                                                .insert(func_name.to_string(), info);
                                        }
                                        Err(e) => {
                                            if std::env::var("DEBUG_JIT").is_ok() {
                                                eprintln!(
                                                    "JIT: Failed to compile function '{}': {}",
                                                    func_name, e
                                                );
                                            }
                                        }
                                    }
                                }
                            }

                            self.call_bytecode_function(function.clone(), raw_args, args)?;
                        }
                        Value::NativeFunction(_) => {
                            match self.call_native_function_vm(function.clone(), args) {
                                Ok(result) => self.stack.push(result),
                                Err(err) => {
                                    self.throw_runtime_value(Value::Error(err))?;
                                }
                            }
                        }
                        Value::Function(..) | Value::GeneratorDef(..) => {
                            let result = self.call_interpreter_callable(&function, &args)?;
                            self.stack.push(result);
                        }
                        _ => {
                            return Err(Self::non_callable_error_message(
                                "the value being called is not callable",
                            ));
                        }
                    }
                }

                OpCode::Return => {
                    let return_value = self.stack.pop().ok_or("Stack underflow in return")?;

                    if let Some(frame) = self.call_frames.pop() {
                        // Pop from function call stack for error reporting
                        self.function_call_stack.pop();

                        // Decrement recursion depth
                        if self.recursion_depth > 0 {
                            self.recursion_depth -= 1;
                        }

                        // Restore previous state
                        self.ip = frame.return_ip;
                        if let Some(prev_chunk) = frame.prev_chunk {
                            self.set_chunk(prev_chunk);
                        }

                        // Clear stack to frame offset
                        self.stack.truncate(frame.stack_offset);

                        // If this was an async function, wrap the return value in a Promise
                        let value_to_push = if frame.is_async {
                            // Create a tokio oneshot channel with the result already available
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            tx.send(Ok(return_value))
                                .map_err(|_| "Failed to send to promise channel")?;

                            Value::Promise {
                                receiver: Arc::new(Mutex::new(rx)),
                                is_polled: Arc::new(Mutex::new(false)),
                                cached_result: Arc::new(Mutex::new(None)),
                                task_handle: None,
                            }
                        } else {
                            return_value
                        };

                        // Push return value (or promise)
                        self.stack.push(value_to_push);
                    } else {
                        // Top-level return
                        return Ok(return_value);
                    }
                }

                OpCode::ReturnNone => {
                    if let Some(frame) = self.call_frames.pop() {
                        // Decrement recursion depth
                        if self.recursion_depth > 0 {
                            self.recursion_depth -= 1;
                        }

                        self.ip = frame.return_ip;
                        if let Some(prev_chunk) = frame.prev_chunk {
                            self.set_chunk(prev_chunk);
                        }
                        self.stack.truncate(frame.stack_offset);

                        // If this was an async function, wrap None in a Promise
                        let value_to_push = if frame.is_async {
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            tx.send(Ok(Value::Null))
                                .map_err(|_| "Failed to send to promise channel")?;

                            Value::Promise {
                                receiver: Arc::new(Mutex::new(rx)),
                                is_polled: Arc::new(Mutex::new(false)),
                                cached_result: Arc::new(Mutex::new(None)),
                                task_handle: None,
                            }
                        } else {
                            Value::Null
                        };

                        self.stack.push(value_to_push);
                    } else {
                        return Ok(Value::Null);
                    }
                }

                OpCode::MakeClosure(func_index) => {
                    let constant = &self.chunk.constants[func_index];
                    if let Constant::Function(chunk) = constant {
                        // Capture upvalues listed in the function's chunk
                        let mut captured = HashMap::new();
                        let mut captured_binding_kinds = HashMap::new();

                        if std::env::var("DEBUG_VM").is_ok() {
                            eprintln!(
                                "MakeClosure: function has {} upvalues: {:?}",
                                chunk.upvalues.len(),
                                chunk.upvalues
                            );
                            eprintln!("  Call stack depth: {}", self.call_frames.len());
                            if let Some(frame) = self.call_frames.last() {
                                eprintln!(
                                    "  Current frame has {} locals: {:?}, {} captured: {:?}",
                                    frame.locals.len(),
                                    frame.locals.keys().collect::<Vec<_>>(),
                                    frame.captured.len(),
                                    frame.captured.keys().collect::<Vec<_>>()
                                );
                            } else {
                                eprintln!("  No current frame!");
                            }
                        }

                        for upvalue_name in &chunk.upvalues {
                            // Find the variable in current scope (locals only - NOT globals)
                            // Prefer local slots (authoritative for locals) and fall back to locals map
                            let capture_entry = if let Some(frame) = self.call_frames.last() {
                                if let Some(existing) = frame.captured.get(upvalue_name) {
                                    let value = existing.lock().unwrap().clone();
                                    let kind = frame
                                        .captured_binding_kinds
                                        .get(upvalue_name)
                                        .copied()
                                        .unwrap_or(BytecodeBindingKind::Mutable);
                                    Some((value, kind))
                                } else if let Some(slot) = self
                                    .chunk
                                    .local_names
                                    .iter()
                                    .position(|name| name == upvalue_name)
                                {
                                    let value = frame
                                        .local_slots
                                        .get(slot)
                                        .cloned()
                                        .or_else(|| frame.locals.get(upvalue_name).cloned());
                                    let kind = frame
                                        .local_slot_binding_kinds
                                        .get(slot)
                                        .copied()
                                        .or_else(|| {
                                            frame.locals_binding_kinds.get(upvalue_name).copied()
                                        })
                                        .unwrap_or(BytecodeBindingKind::Mutable);
                                    value.map(|value| (value, kind))
                                } else {
                                    frame.locals.get(upvalue_name).cloned().map(|value| {
                                        let kind = frame
                                            .locals_binding_kinds
                                            .get(upvalue_name)
                                            .copied()
                                            .unwrap_or(BytecodeBindingKind::Mutable);
                                        (value, kind)
                                    })
                                }
                            } else {
                                None
                            };

                            if let Some((val, binding_kind)) = capture_entry {
                                if std::env::var("DEBUG_VM").is_ok() {
                                    eprintln!(
                                        "  Captured '{}' from locals = {:?}",
                                        upvalue_name, val
                                    );
                                }
                                // Wrap in Arc<Mutex<>> for shared mutable state
                                captured.insert(upvalue_name.clone(), Arc::new(Mutex::new(val)));
                                captured_binding_kinds.insert(upvalue_name.clone(), binding_kind);
                            } else {
                                if std::env::var("DEBUG_VM").is_ok() {
                                    eprintln!(
                                        "  Skipped '{}' (not in locals, will resolve at runtime)",
                                        upvalue_name
                                    );
                                }
                                // Variable not in locals - it's either a global or undefined
                                // Don't capture it - let it be resolved at runtime
                            }
                        }

                        // Create a closure value with captured variables
                        let value = Value::BytecodeFunction {
                            chunk: (**chunk).clone(),
                            captured,
                            captured_binding_kinds,
                        };
                        self.stack.push(value);
                    } else {
                        return Err("Expected function constant".to_string());
                    }
                }

                // Collection operations
                OpCode::MakeArray(count) => {
                    // Collect elements from stack
                    // If the bottom-most element is ArrayMarker, collect until marker
                    // Otherwise, collect exactly 'count' elements
                    let mut elements = Vec::with_capacity(count);
                    let mut found_marker = false;

                    for _ in 0..count {
                        let value = self.stack.pop().ok_or("Stack underflow in MakeArray")?;
                        if matches!(value, Value::ArrayMarker) {
                            found_marker = true;
                            break;
                        }
                        elements.push(value);
                    }

                    // If we found a marker, that's it
                    // Otherwise we need to check if there are more elements (from spreads)
                    if found_marker {
                        // Collect any remaining elements until we reach the marker
                        // Actually, we already hit the marker, so we're done
                    }

                    elements.reverse();
                    if !self.push_checked_value(
                        Value::Array(Arc::new(elements)),
                        "building array literal",
                    )? {
                        continue;
                    }
                }

                OpCode::MakeArrayFromMarker => {
                    let mut elements = Vec::new();

                    loop {
                        let value = self
                            .stack
                            .pop()
                            .ok_or("Missing array marker in MakeArrayFromMarker")?;
                        if matches!(value, Value::ArrayMarker) {
                            break;
                        }
                        elements.push(value);
                    }

                    elements.reverse();
                    if !self.push_checked_value(
                        Value::Array(Arc::new(elements)),
                        "building array literal",
                    )? {
                        continue;
                    }
                }

                OpCode::PushArrayMarker => {
                    self.stack.push(Value::ArrayMarker);
                }

                OpCode::MakeDict(count) => {
                    let mut dict = DictMap::default();
                    dict.reserve(count);
                    let mut entries = Vec::with_capacity(count);
                    for _ in 0..count {
                        let value = self.stack.pop().ok_or("Stack underflow")?;
                        let key = self.stack.pop().ok_or("Stack underflow")?;

                        let key_str = match key {
                            Value::Str(s) => Arc::from(s.as_str()),
                            _ => return Err("Dict keys must be strings".to_string()),
                        };

                        entries.push((key_str, value));
                    }

                    // Preserve source order so later key occurrences override earlier ones.
                    entries.reverse();
                    for (key, value) in entries {
                        dict.insert(key, value);
                    }
                    if !self.push_checked_value(
                        Value::Dict(Arc::new(dict)),
                        "building dictionary literal",
                    )? {
                        continue;
                    }
                }

                OpCode::MakeDictFromMarker => {
                    let mut dict = DictMap::default();
                    let mut entries = Vec::new();

                    loop {
                        let value =
                            self.stack.pop().ok_or("Missing dict marker in MakeDictFromMarker")?;
                        let key =
                            self.stack.pop().ok_or("Missing dict marker in MakeDictFromMarker")?;

                        if matches!(key, Value::ArrayMarker) && matches!(value, Value::ArrayMarker)
                        {
                            break;
                        }

                        let key_str = match key {
                            Value::Str(s) => Arc::from(s.as_str()),
                            _ => return Err("Dict keys must be strings".to_string()),
                        };

                        entries.push((key_str, value));
                    }

                    // Preserve source order so spread/literal override behavior is deterministic.
                    entries.reverse();
                    for (key, value) in entries {
                        dict.insert(key, value);
                    }

                    if !self.push_checked_value(
                        Value::Dict(Arc::new(dict)),
                        "building dictionary literal",
                    )? {
                        continue;
                    }
                }

                OpCode::MakeDictWithKeys(keys) => {
                    let mut values = Vec::with_capacity(keys.len());
                    for _ in 0..keys.len() {
                        values.push(self.stack.pop().ok_or("Stack underflow")?);
                    }
                    values.reverse();
                    if !self.push_checked_value(
                        Value::FixedDict { keys: Arc::clone(&keys), values },
                        "building fixed dictionary literal",
                    )? {
                        continue;
                    }
                }

                OpCode::IndexGet => {
                    let index = self.stack.pop().ok_or("Stack underflow")?;
                    let object = self.stack.pop().ok_or("Stack underflow")?;
                    let result = Self::get_indexed_value(&object, &index)?;

                    self.stack.push(result);
                }

                OpCode::IndexSet => {
                    let index = self.stack.pop().ok_or("Stack underflow")?;
                    let object = self.stack.pop().ok_or("Stack underflow")?;
                    let value = self.stack.pop().ok_or("Stack underflow")?;

                    match (object, index) {
                        (Value::Array(arr), Value::Int(i)) => {
                            let mut arr_clone = arr;
                            let arr_mut = Arc::make_mut(&mut arr_clone);
                            let idx = if i < 0 {
                                (arr_mut.len() as i64 + i) as usize
                            } else {
                                i as usize
                            };

                            if idx < arr_mut.len() {
                                arr_mut[idx] = value;
                                if !self.push_checked_value(
                                    Value::Array(arr_clone),
                                    "updating array element",
                                )? {
                                    continue;
                                }
                            } else {
                                return Err(format!("Index out of bounds: {}", i));
                            }
                        }
                        (Value::Dict(dict), Value::Str(key)) => {
                            let mut dict_clone = dict;
                            let dict_mut = Arc::make_mut(&mut dict_clone);
                            dict_mut.insert(Arc::from(key.as_str()), value);
                            if !self.push_checked_value(
                                Value::Dict(dict_clone),
                                "updating dictionary entry",
                            )? {
                                continue;
                            }
                        }
                        (Value::FixedDict { keys, mut values }, Value::Str(key)) => {
                            if let Some(idx) = keys.iter().position(|k| k.as_ref() == key.as_str())
                            {
                                values[idx] = value;
                                if !self.push_checked_value(
                                    Value::FixedDict { keys, values },
                                    "updating fixed dictionary entry",
                                )? {
                                    continue;
                                }
                            } else {
                                let mut dict = DictMap::default();
                                for (k, v) in keys.iter().cloned().zip(values.into_iter()) {
                                    dict.insert(k, v);
                                }
                                dict.insert(Arc::from(key.as_str()), value);
                                if !self.push_checked_value(
                                    Value::Dict(Arc::new(dict)),
                                    "updating dictionary entry",
                                )? {
                                    continue;
                                }
                            }
                        }
                        (Value::Dict(dict), Value::Int(i)) => {
                            hashmap_profile_bump(&HASHMAP_SET_DICT_INTKEY);
                            if dict.is_empty() {
                                if i >= 0 {
                                    match value {
                                        Value::Int(int_value) => {
                                            if i == 0 {
                                                if !self.push_checked_value(
                                                    Value::DenseIntDictIntFull(Arc::new(vec![
                                                        int_value,
                                                    ])),
                                                    "updating integer dictionary entry",
                                                )? {
                                                    continue;
                                                }
                                            } else {
                                                let mut values = Self::dense_int_dict_int_with_len(
                                                    (i as usize) + 1,
                                                );
                                                values[i as usize] = Some(int_value);
                                                if !self.push_checked_value(
                                                    Value::DenseIntDictInt(Arc::new(values)),
                                                    "updating integer dictionary entry",
                                                )? {
                                                    continue;
                                                }
                                            }
                                        }
                                        Value::Null => {
                                            let values =
                                                Self::dense_int_dict_int_with_len((i as usize) + 1);
                                            if !self.push_checked_value(
                                                Value::DenseIntDictInt(Arc::new(values)),
                                                "updating integer dictionary entry",
                                            )? {
                                                continue;
                                            }
                                        }
                                        other => {
                                            let mut values = vec![Value::Null; (i as usize) + 1];
                                            values[i as usize] = other;
                                            if !self.push_checked_value(
                                                Value::DenseIntDict(Arc::new(values)),
                                                "updating integer dictionary entry",
                                            )? {
                                                continue;
                                            }
                                        }
                                    }
                                } else {
                                    let mut int_dict = IntDictMap::default();
                                    int_dict.reserve(1024);
                                    int_dict.insert(i, value);
                                    if !self.push_checked_value(
                                        Value::IntDict(Arc::new(int_dict)),
                                        "updating integer dictionary entry",
                                    )? {
                                        continue;
                                    }
                                }
                            } else {
                                let mut dict_clone = dict;
                                let dict_mut = Arc::make_mut(&mut dict_clone);
                                // Support integer keys by converting to string
                                let key = self.int_key_string(i);
                                dict_mut.insert(Arc::clone(&key), value);
                                if !self.push_checked_value(
                                    Value::Dict(dict_clone),
                                    "updating dictionary entry",
                                )? {
                                    continue;
                                }
                            }
                        }
                        (Value::FixedDict { keys, values }, Value::Int(i)) => {
                            if keys.is_empty() && values.is_empty() {
                                if i >= 0 {
                                    match value {
                                        Value::Int(int_value) => {
                                            if i == 0 {
                                                if !self.push_checked_value(
                                                    Value::DenseIntDictIntFull(Arc::new(vec![
                                                        int_value,
                                                    ])),
                                                    "updating integer dictionary entry",
                                                )? {
                                                    continue;
                                                }
                                            } else {
                                                let mut values = Self::dense_int_dict_int_with_len(
                                                    (i as usize) + 1,
                                                );
                                                values[i as usize] = Some(int_value);
                                                if !self.push_checked_value(
                                                    Value::DenseIntDictInt(Arc::new(values)),
                                                    "updating integer dictionary entry",
                                                )? {
                                                    continue;
                                                }
                                            }
                                        }
                                        Value::Null => {
                                            let values =
                                                Self::dense_int_dict_int_with_len((i as usize) + 1);
                                            if !self.push_checked_value(
                                                Value::DenseIntDictInt(Arc::new(values)),
                                                "updating integer dictionary entry",
                                            )? {
                                                continue;
                                            }
                                        }
                                        other => {
                                            let mut values = vec![Value::Null; (i as usize) + 1];
                                            values[i as usize] = other;
                                            if !self.push_checked_value(
                                                Value::DenseIntDict(Arc::new(values)),
                                                "updating integer dictionary entry",
                                            )? {
                                                continue;
                                            }
                                        }
                                    }
                                } else {
                                    let mut int_dict = IntDictMap::default();
                                    int_dict.reserve(1024);
                                    int_dict.insert(i, value);
                                    if !self.push_checked_value(
                                        Value::IntDict(Arc::new(int_dict)),
                                        "updating integer dictionary entry",
                                    )? {
                                        continue;
                                    }
                                }
                            } else {
                                let key = self.int_key_string(i);
                                if let Some(idx) =
                                    keys.iter().position(|k| k.as_ref() == key.as_ref())
                                {
                                    let mut values = values;
                                    values[idx] = value;
                                    if !self.push_checked_value(
                                        Value::FixedDict { keys, values },
                                        "updating fixed dictionary entry",
                                    )? {
                                        continue;
                                    }
                                } else {
                                    let mut dict = DictMap::default();
                                    for (k, v) in keys.iter().cloned().zip(values.into_iter()) {
                                        dict.insert(k, v);
                                    }
                                    dict.insert(Arc::clone(&key), value);
                                    if !self.push_checked_value(
                                        Value::Dict(Arc::new(dict)),
                                        "updating dictionary entry",
                                    )? {
                                        continue;
                                    }
                                }
                            }
                        }
                        (Value::IntDict(dict), Value::Int(i)) => {
                            hashmap_profile_bump(&HASHMAP_SET_INTDICT);
                            let mut dict_clone = dict;
                            let dict_mut = Arc::make_mut(&mut dict_clone);
                            dict_mut.insert(i, value);
                            if !self.push_checked_value(
                                Value::IntDict(dict_clone),
                                "updating integer dictionary entry",
                            )? {
                                continue;
                            }
                        }
                        (Value::DenseIntDict(mut values), Value::Int(i)) => {
                            hashmap_profile_bump(&HASHMAP_SET_DENSE);
                            if i < 0 {
                                let mut int_dict = Self::dense_int_dict_to_int_dict(&values);
                                int_dict.insert(i, value);
                                if !self.push_checked_value(
                                    Value::IntDict(Arc::new(int_dict)),
                                    "updating integer dictionary entry",
                                )? {
                                    continue;
                                }
                            } else {
                                let values_mut = Arc::make_mut(&mut values);
                                let index = i as usize;
                                if index >= values_mut.len() {
                                    values_mut.resize(index + 1, Value::Null);
                                }
                                values_mut[index] = value;
                                if !self.push_checked_value(
                                    Value::DenseIntDict(values),
                                    "updating dense integer dictionary entry",
                                )? {
                                    continue;
                                }
                            }
                        }
                        (Value::DenseIntDictInt(mut values), Value::Int(i)) => {
                            hashmap_profile_bump(&HASHMAP_SET_DENSE_INT);
                            if i < 0 {
                                let mut int_dict = Self::dense_int_dict_int_to_int_dict(&values);
                                int_dict.insert(i, value);
                                if !self.push_checked_value(
                                    Value::IntDict(Arc::new(int_dict)),
                                    "updating integer dictionary entry",
                                )? {
                                    continue;
                                }
                            } else {
                                let index = i as usize;
                                match value {
                                    Value::Int(int_value) => {
                                        let values_mut = Arc::make_mut(&mut values);
                                        let len = values_mut.len();
                                        if index == len {
                                            values_mut.push(Some(int_value));
                                        } else if index < len {
                                            values_mut[index] = Some(int_value);
                                        } else {
                                            values_mut.resize(index + 1, None);
                                            values_mut[index] = Some(int_value);
                                        }
                                        if !self.push_checked_value(
                                            Value::DenseIntDictInt(values),
                                            "updating dense integer dictionary entry",
                                        )? {
                                            continue;
                                        }
                                    }
                                    Value::Null => {
                                        let values_mut = Arc::make_mut(&mut values);
                                        let len = values_mut.len();
                                        if index == len {
                                            values_mut.push(None);
                                        } else if index < len {
                                            values_mut[index] = None;
                                        } else {
                                            values_mut.resize(index + 1, None);
                                            values_mut[index] = None;
                                        }
                                        if !self.push_checked_value(
                                            Value::DenseIntDictInt(values),
                                            "updating dense integer dictionary entry",
                                        )? {
                                            continue;
                                        }
                                    }
                                    other => {
                                        let mut dense_values =
                                            Self::dense_int_dict_int_to_dense_int_dict(&values);
                                        if index >= dense_values.len() {
                                            dense_values.resize(index + 1, Value::Null);
                                        }
                                        dense_values[index] = other;
                                        self.stack
                                            .push(Value::DenseIntDict(Arc::new(dense_values)));
                                    }
                                }
                            }
                        }
                        (Value::DenseIntDictIntFull(mut values), Value::Int(i)) => {
                            hashmap_profile_bump(&HASHMAP_SET_DENSE_INT);
                            if i < 0 {
                                let mut int_dict =
                                    Self::dense_int_dict_int_full_to_int_dict(&values);
                                int_dict.insert(i, value);
                                if !self.push_checked_value(
                                    Value::IntDict(Arc::new(int_dict)),
                                    "updating integer dictionary entry",
                                )? {
                                    continue;
                                }
                            } else {
                                let index = i as usize;
                                match value {
                                    Value::Int(int_value) => {
                                        let values_mut = Arc::make_mut(&mut values);
                                        let len = values_mut.len();
                                        if index == len {
                                            values_mut.push(int_value);
                                            if !self.push_checked_value(
                                                Value::DenseIntDictIntFull(values),
                                                "updating dense integer dictionary entry",
                                            )? {
                                                continue;
                                            }
                                        } else if index < len {
                                            values_mut[index] = int_value;
                                            if !self.push_checked_value(
                                                Value::DenseIntDictIntFull(values),
                                                "updating dense integer dictionary entry",
                                            )? {
                                                continue;
                                            }
                                        } else {
                                            let mut sparse =
                                                Self::dense_int_dict_int_full_to_sparse(&values);
                                            sparse.resize(index + 1, None);
                                            sparse[index] = Some(int_value);
                                            if !self.push_checked_value(
                                                Value::DenseIntDictInt(Arc::new(sparse)),
                                                "updating integer dictionary entry",
                                            )? {
                                                continue;
                                            }
                                        }
                                    }
                                    Value::Null => {
                                        let mut sparse =
                                            Self::dense_int_dict_int_full_to_sparse(&values);
                                        if index >= sparse.len() {
                                            sparse.resize(index + 1, None);
                                        }
                                        sparse[index] = None;
                                        if !self.push_checked_value(
                                            Value::DenseIntDictInt(Arc::new(sparse)),
                                            "updating integer dictionary entry",
                                        )? {
                                            continue;
                                        }
                                    }
                                    other => {
                                        let mut dense_values =
                                            Self::dense_int_dict_int_full_to_dense(&values);
                                        if index >= dense_values.len() {
                                            dense_values.resize(index + 1, Value::Null);
                                        }
                                        dense_values[index] = other;
                                        if !self.push_checked_value(
                                            Value::DenseIntDict(Arc::new(dense_values)),
                                            "updating integer dictionary entry",
                                        )? {
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        (Value::IntDict(dict), Value::Str(key)) => {
                            let mut dict_clone = DictMap::default();
                            for (k, v) in dict.iter() {
                                dict_clone.insert(k.to_string().into(), v.clone());
                            }
                            dict_clone.insert(Arc::from(key.as_str()), value);
                            if !self.push_checked_value(
                                Value::Dict(Arc::new(dict_clone)),
                                "updating dictionary entry",
                            )? {
                                continue;
                            }
                        }
                        (Value::DenseIntDict(values), Value::Str(key)) => {
                            match key.parse::<i64>() {
                                Ok(int_key) => {
                                    if int_key < 0 {
                                        let mut int_dict =
                                            Self::dense_int_dict_to_int_dict(&values);
                                        int_dict.insert(int_key, value);
                                        if !self.push_checked_value(
                                            Value::IntDict(Arc::new(int_dict)),
                                            "updating integer dictionary entry",
                                        )? {
                                            continue;
                                        }
                                    } else {
                                        let mut values = values;
                                        let values_mut = Arc::make_mut(&mut values);
                                        let index = int_key as usize;
                                        if index >= values_mut.len() {
                                            values_mut.resize(index + 1, Value::Null);
                                        }
                                        values_mut[index] = value;
                                        if !self.push_checked_value(
                                            Value::DenseIntDict(values),
                                            "updating dense integer dictionary entry",
                                        )? {
                                            continue;
                                        }
                                    }
                                }
                                Err(_) => {
                                    let mut dict = Self::dense_int_dict_to_dict(&values);
                                    dict.insert(Arc::from(key.as_str()), value);
                                    if !self.push_checked_value(
                                        Value::Dict(Arc::new(dict)),
                                        "updating dictionary entry",
                                    )? {
                                        continue;
                                    }
                                }
                            }
                        }
                        (Value::DenseIntDictInt(values), Value::Str(key)) => {
                            match key.parse::<i64>() {
                                Ok(int_key) => {
                                    if int_key < 0 {
                                        let mut int_dict =
                                            Self::dense_int_dict_int_to_int_dict(&values);
                                        int_dict.insert(int_key, value);
                                        if !self.push_checked_value(
                                            Value::IntDict(Arc::new(int_dict)),
                                            "updating integer dictionary entry",
                                        )? {
                                            continue;
                                        }
                                    } else {
                                        let index = int_key as usize;
                                        match value {
                                            Value::Int(int_value) => {
                                                let mut values = values;
                                                let values_mut = Arc::make_mut(&mut values);
                                                let len = values_mut.len();
                                                if index == len {
                                                    values_mut.push(Some(int_value));
                                                } else if index < len {
                                                    values_mut[index] = Some(int_value);
                                                } else {
                                                    values_mut.resize(index + 1, None);
                                                    values_mut[index] = Some(int_value);
                                                }
                                                if !self.push_checked_value(
                                                    Value::DenseIntDictInt(values),
                                                    "updating dense integer dictionary entry",
                                                )? {
                                                    continue;
                                                }
                                            }
                                            Value::Null => {
                                                let mut values = values;
                                                let values_mut = Arc::make_mut(&mut values);
                                                let len = values_mut.len();
                                                if index == len {
                                                    values_mut.push(None);
                                                } else if index < len {
                                                    values_mut[index] = None;
                                                } else {
                                                    values_mut.resize(index + 1, None);
                                                    values_mut[index] = None;
                                                }
                                                if !self.push_checked_value(
                                                    Value::DenseIntDictInt(values),
                                                    "updating dense integer dictionary entry",
                                                )? {
                                                    continue;
                                                }
                                            }
                                            other => {
                                                let mut dense_values =
                                                    Self::dense_int_dict_int_to_dense_int_dict(
                                                        &values,
                                                    );
                                                if index >= dense_values.len() {
                                                    dense_values.resize(index + 1, Value::Null);
                                                }
                                                dense_values[index] = other;
                                                if !self.push_checked_value(
                                                    Value::DenseIntDict(Arc::new(dense_values)),
                                                    "updating integer dictionary entry",
                                                )? {
                                                    continue;
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(_) => {
                                    let mut dict = Self::dense_int_dict_int_to_dict(&values);
                                    dict.insert(Arc::from(key.as_str()), value);
                                    if !self.push_checked_value(
                                        Value::Dict(Arc::new(dict)),
                                        "updating dictionary entry",
                                    )? {
                                        continue;
                                    }
                                }
                            }
                        }
                        (Value::DenseIntDictIntFull(values), Value::Str(key)) => {
                            match key.parse::<i64>() {
                                Ok(int_key) => {
                                    if int_key < 0 {
                                        let mut int_dict =
                                            Self::dense_int_dict_int_full_to_int_dict(&values);
                                        int_dict.insert(int_key, value);
                                        if !self.push_checked_value(
                                            Value::IntDict(Arc::new(int_dict)),
                                            "updating integer dictionary entry",
                                        )? {
                                            continue;
                                        }
                                    } else {
                                        let index = int_key as usize;
                                        match value {
                                            Value::Int(int_value) => {
                                                let mut values = values;
                                                let values_mut = Arc::make_mut(&mut values);
                                                let len = values_mut.len();
                                                if index == len {
                                                    values_mut.push(int_value);
                                                    if !self.push_checked_value(
                                                        Value::DenseIntDictIntFull(values),
                                                        "updating integer dictionary entry",
                                                    )? {
                                                        continue;
                                                    }
                                                } else if index < len {
                                                    values_mut[index] = int_value;
                                                    if !self.push_checked_value(
                                                        Value::DenseIntDictIntFull(values),
                                                        "updating integer dictionary entry",
                                                    )? {
                                                        continue;
                                                    }
                                                } else {
                                                    let mut sparse =
                                                        Self::dense_int_dict_int_full_to_sparse(
                                                            &values,
                                                        );
                                                    sparse.resize(index + 1, None);
                                                    sparse[index] = Some(int_value);
                                                    if !self.push_checked_value(
                                                        Value::DenseIntDictInt(Arc::new(sparse)),
                                                        "updating integer dictionary entry",
                                                    )? {
                                                        continue;
                                                    }
                                                }
                                            }
                                            Value::Null => {
                                                let mut sparse =
                                                    Self::dense_int_dict_int_full_to_sparse(
                                                        &values,
                                                    );
                                                if index >= sparse.len() {
                                                    sparse.resize(index + 1, None);
                                                }
                                                sparse[index] = None;
                                                if !self.push_checked_value(
                                                    Value::DenseIntDictInt(Arc::new(sparse)),
                                                    "updating integer dictionary entry",
                                                )? {
                                                    continue;
                                                }
                                            }
                                            other => {
                                                let mut dense_values =
                                                    Self::dense_int_dict_int_full_to_dense(&values);
                                                if index >= dense_values.len() {
                                                    dense_values.resize(index + 1, Value::Null);
                                                }
                                                dense_values[index] = other;
                                                if !self.push_checked_value(
                                                    Value::DenseIntDict(Arc::new(dense_values)),
                                                    "updating integer dictionary entry",
                                                )? {
                                                    continue;
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(_) => {
                                    let mut dict = DictMap::default();
                                    for (index, value) in values.iter().enumerate() {
                                        dict.insert(
                                            Arc::from(index.to_string().as_str()),
                                            Value::Int(*value),
                                        );
                                    }
                                    dict.insert(Arc::from(key.as_str()), value);
                                    if !self.push_checked_value(
                                        Value::Dict(Arc::new(dict)),
                                        "updating dictionary entry",
                                    )? {
                                        continue;
                                    }
                                }
                            }
                        }
                        _ => return Err("Invalid index assignment".to_string()),
                    }
                }

                OpCode::IndexGetInPlace(slot) => {
                    // Pop index from stack
                    let index = self.stack.pop().ok_or("Stack underflow")?;

                    let frame =
                        self.call_frames.last().ok_or("IndexGetInPlace requires call frame")?;

                    let object = frame
                        .local_slots
                        .get(slot)
                        .cloned()
                        .ok_or_else(|| format!("Invalid local slot: {}", slot))?;
                    let result = Self::get_indexed_value(&object, &index)?;

                    self.stack.push(result);
                }

                OpCode::IndexSetInPlace(slot) => {
                    // Stack layout: [... value, index] (index on top)
                    // Pop index and value from stack
                    let index = self.stack.pop().ok_or("Stack underflow")?;
                    let value = self.stack.pop().ok_or("Stack underflow")?;
                    let binding_name = self
                        .chunk
                        .local_names
                        .get(slot)
                        .cloned()
                        .unwrap_or_else(|| format!("<local:{}>", slot));

                    let frame =
                        self.call_frames.last_mut().ok_or("IndexSetInPlace requires call frame")?;

                    let kind = frame
                        .local_slot_binding_kinds
                        .get(slot)
                        .copied()
                        .unwrap_or(BytecodeBindingKind::Mutable);
                    if !matches!(kind, BytecodeBindingKind::Mutable) {
                        return Err(Self::local_mutation_error(kind, &binding_name));
                    }

                    {
                        let object = frame
                            .local_slots
                            .get_mut(slot)
                            .ok_or_else(|| format!("Invalid local slot: {}", slot))?;

                        match index {
                            Value::Int(i) => match object {
                                Value::Array(arr) => {
                                    let arr_mut = Arc::make_mut(arr);
                                    let idx = if i < 0 {
                                        (arr_mut.len() as i64 + i) as usize
                                    } else {
                                        i as usize
                                    };
                                    if idx < arr_mut.len() {
                                        arr_mut[idx] = value;
                                    } else {
                                        return Err(format!("Index out of bounds: {}", i));
                                    }
                                }
                                Value::Dict(dict) => {
                                    hashmap_profile_bump(&HASHMAP_SET_DICT_INTKEY);
                                    if dict.is_empty() {
                                        if i >= 0 {
                                            match value {
                                                Value::Int(int_value) => {
                                                    if i == 0 {
                                                        *object = Value::DenseIntDictIntFull(
                                                            Arc::new(vec![int_value]),
                                                        );
                                                    } else {
                                                        let mut values =
                                                            Self::dense_int_dict_int_with_len(
                                                                (i as usize) + 1,
                                                            );
                                                        values[i as usize] = Some(int_value);
                                                        *object = Value::DenseIntDictInt(Arc::new(
                                                            values,
                                                        ));
                                                    }
                                                }
                                                Value::Null => {
                                                    let values = Self::dense_int_dict_int_with_len(
                                                        (i as usize) + 1,
                                                    );
                                                    *object =
                                                        Value::DenseIntDictInt(Arc::new(values));
                                                }
                                                other => {
                                                    let mut values =
                                                        vec![Value::Null; (i as usize) + 1];
                                                    values[i as usize] = other;
                                                    *object = Value::DenseIntDict(Arc::new(values));
                                                }
                                            }
                                        } else {
                                            let mut int_dict = IntDictMap::default();
                                            int_dict.reserve(1024);
                                            int_dict.insert(i, value);
                                            *object = Value::IntDict(Arc::new(int_dict));
                                        }
                                    } else {
                                        let dict_mut = Arc::make_mut(dict);
                                        dict_mut.insert(Arc::from(i.to_string().as_str()), value);
                                    }
                                }
                                Value::FixedDict { keys, values } => {
                                    if keys.is_empty() && values.is_empty() {
                                        if i >= 0 {
                                            match value {
                                                Value::Int(int_value) => {
                                                    if i == 0 {
                                                        *object = Value::DenseIntDictIntFull(
                                                            Arc::new(vec![int_value]),
                                                        );
                                                    } else {
                                                        let mut values =
                                                            Self::dense_int_dict_int_with_len(
                                                                (i as usize) + 1,
                                                            );
                                                        values[i as usize] = Some(int_value);
                                                        *object = Value::DenseIntDictInt(Arc::new(
                                                            values,
                                                        ));
                                                    }
                                                }
                                                Value::Null => {
                                                    let values = Self::dense_int_dict_int_with_len(
                                                        (i as usize) + 1,
                                                    );
                                                    *object =
                                                        Value::DenseIntDictInt(Arc::new(values));
                                                }
                                                other => {
                                                    let mut values =
                                                        vec![Value::Null; (i as usize) + 1];
                                                    values[i as usize] = other;
                                                    *object = Value::DenseIntDict(Arc::new(values));
                                                }
                                            }
                                        } else {
                                            let mut int_dict = IntDictMap::default();
                                            int_dict.reserve(1024);
                                            int_dict.insert(i, value);
                                            *object = Value::IntDict(Arc::new(int_dict));
                                        }
                                    } else {
                                        let key = i.to_string();
                                        if let Some(idx) =
                                            keys.iter().position(|k| k.as_ref() == key.as_str())
                                        {
                                            let mut values = values.clone();
                                            values[idx] = value;
                                            *object =
                                                Value::FixedDict { keys: Arc::clone(keys), values };
                                        } else {
                                            let mut dict = DictMap::default();
                                            for (k, v) in
                                                keys.iter().cloned().zip(values.iter().cloned())
                                            {
                                                dict.insert(k, v);
                                            }
                                            dict.insert(Arc::from(key.as_str()), value);
                                            *object = Value::Dict(Arc::new(dict));
                                        }
                                    }
                                }
                                Value::IntDict(dict) => {
                                    hashmap_profile_bump(&HASHMAP_SET_INTDICT);
                                    let dict_mut = Arc::make_mut(dict);
                                    dict_mut.insert(i, value);
                                }
                                Value::DenseIntDict(values) => {
                                    hashmap_profile_bump(&HASHMAP_SET_DENSE);
                                    if i < 0 {
                                        let mut int_dict = Self::dense_int_dict_to_int_dict(values);
                                        int_dict.insert(i, value);
                                        *object = Value::IntDict(Arc::new(int_dict));
                                    } else {
                                        let values_mut = Arc::make_mut(values);
                                        let index = i as usize;
                                        if index >= values_mut.len() {
                                            values_mut.resize(index + 1, Value::Null);
                                        }
                                        values_mut[index] = value;
                                    }
                                }
                                Value::DenseIntDictInt(values) => {
                                    hashmap_profile_bump(&HASHMAP_SET_DENSE_INT);
                                    if i < 0 {
                                        let mut int_dict =
                                            Self::dense_int_dict_int_to_int_dict(values);
                                        int_dict.insert(i, value);
                                        *object = Value::IntDict(Arc::new(int_dict));
                                    } else {
                                        let index = i as usize;
                                        match value {
                                            Value::Int(int_value) => {
                                                let values_mut = Arc::make_mut(values);
                                                let len = values_mut.len();
                                                if index == len {
                                                    values_mut.push(Some(int_value));
                                                } else if index < len {
                                                    values_mut[index] = Some(int_value);
                                                } else {
                                                    values_mut.resize(index + 1, None);
                                                    values_mut[index] = Some(int_value);
                                                }
                                            }
                                            Value::Null => {
                                                let values_mut = Arc::make_mut(values);
                                                let len = values_mut.len();
                                                if index == len {
                                                    values_mut.push(None);
                                                } else if index < len {
                                                    values_mut[index] = None;
                                                } else {
                                                    values_mut.resize(index + 1, None);
                                                    values_mut[index] = None;
                                                }
                                            }
                                            other => {
                                                let mut dense_values =
                                                    Self::dense_int_dict_int_to_dense_int_dict(
                                                        values,
                                                    );
                                                if index >= dense_values.len() {
                                                    dense_values.resize(index + 1, Value::Null);
                                                }
                                                dense_values[index] = other;
                                                *object =
                                                    Value::DenseIntDict(Arc::new(dense_values));
                                            }
                                        }
                                    }
                                }
                                Value::DenseIntDictIntFull(values) => {
                                    hashmap_profile_bump(&HASHMAP_SET_DENSE_INT);
                                    if i < 0 {
                                        let mut int_dict =
                                            Self::dense_int_dict_int_full_to_int_dict(values);
                                        int_dict.insert(i, value);
                                        *object = Value::IntDict(Arc::new(int_dict));
                                    } else {
                                        let index = i as usize;
                                        match value {
                                            Value::Int(int_value) => {
                                                let values_mut = Arc::make_mut(values);
                                                let len = values_mut.len();
                                                if index == len {
                                                    values_mut.push(int_value);
                                                } else if index < len {
                                                    values_mut[index] = int_value;
                                                } else {
                                                    let mut sparse =
                                                        Self::dense_int_dict_int_full_to_sparse(
                                                            values,
                                                        );
                                                    sparse.resize(index + 1, None);
                                                    sparse[index] = Some(int_value);
                                                    *object =
                                                        Value::DenseIntDictInt(Arc::new(sparse));
                                                }
                                            }
                                            Value::Null => {
                                                let mut sparse =
                                                    Self::dense_int_dict_int_full_to_sparse(values);
                                                if index >= sparse.len() {
                                                    sparse.resize(index + 1, None);
                                                }
                                                sparse[index] = None;
                                                *object = Value::DenseIntDictInt(Arc::new(sparse));
                                            }
                                            other => {
                                                let mut dense_values =
                                                    Self::dense_int_dict_int_full_to_dense(values);
                                                if index >= dense_values.len() {
                                                    dense_values.resize(index + 1, Value::Null);
                                                }
                                                dense_values[index] = other;
                                                *object =
                                                    Value::DenseIntDict(Arc::new(dense_values));
                                            }
                                        }
                                    }
                                }
                                _ => return Err("Invalid index assignment".to_string()),
                            },

                            Value::Str(key) => match object {
                                Value::Dict(dict) => {
                                    let dict_mut = Arc::make_mut(dict);
                                    dict_mut.insert(Arc::from(key.as_str()), value);
                                }
                                Value::FixedDict { keys, values } => {
                                    if let Some(idx) =
                                        keys.iter().position(|k| k.as_ref() == key.as_str())
                                    {
                                        let mut values = values.clone();
                                        values[idx] = value;
                                        *object =
                                            Value::FixedDict { keys: Arc::clone(keys), values };
                                    } else {
                                        let mut dict = DictMap::default();
                                        for (k, v) in
                                            keys.iter().cloned().zip(values.iter().cloned())
                                        {
                                            dict.insert(k, v);
                                        }
                                        dict.insert(Arc::from(key.as_str()), value);
                                        *object = Value::Dict(Arc::new(dict));
                                    }
                                }
                                Value::IntDict(dict) => {
                                    let mut dict_clone = DictMap::default();
                                    for (k, v) in dict.iter() {
                                        dict_clone.insert(k.to_string().into(), v.clone());
                                    }
                                    dict_clone.insert(Arc::from(key.as_str()), value);
                                    *object = Value::Dict(Arc::new(dict_clone));
                                }
                                Value::DenseIntDict(values) => match key.parse::<i64>() {
                                    Ok(int_key) => {
                                        if int_key < 0 {
                                            let mut int_dict =
                                                Self::dense_int_dict_to_int_dict(values);
                                            int_dict.insert(int_key, value);
                                            *object = Value::IntDict(Arc::new(int_dict));
                                        } else {
                                            let values_mut = Arc::make_mut(values);
                                            let index = int_key as usize;
                                            if index >= values_mut.len() {
                                                values_mut.resize(index + 1, Value::Null);
                                            }
                                            values_mut[index] = value;
                                        }
                                    }
                                    Err(_) => {
                                        let mut dict = Self::dense_int_dict_to_dict(values);
                                        dict.insert(Arc::from(key.as_str()), value);
                                        *object = Value::Dict(Arc::new(dict));
                                    }
                                },
                                Value::DenseIntDictInt(values) => match key.parse::<i64>() {
                                    Ok(int_key) => {
                                        if int_key < 0 {
                                            let mut int_dict =
                                                Self::dense_int_dict_int_to_int_dict(values);
                                            int_dict.insert(int_key, value);
                                            *object = Value::IntDict(Arc::new(int_dict));
                                        } else {
                                            let index = int_key as usize;
                                            match value {
                                                Value::Int(int_value) => {
                                                    let values_mut = Arc::make_mut(values);
                                                    if index >= values_mut.len() {
                                                        values_mut.resize(index + 1, None);
                                                    }
                                                    values_mut[index] = Some(int_value);
                                                }
                                                Value::Null => {
                                                    let values_mut = Arc::make_mut(values);
                                                    if index >= values_mut.len() {
                                                        values_mut.resize(index + 1, None);
                                                    }
                                                    values_mut[index] = None;
                                                }
                                                other => {
                                                    let mut dense_values =
                                                        Self::dense_int_dict_int_to_dense_int_dict(
                                                            values,
                                                        );
                                                    if index >= dense_values.len() {
                                                        dense_values.resize(index + 1, Value::Null);
                                                    }
                                                    dense_values[index] = other;
                                                    *object =
                                                        Value::DenseIntDict(Arc::new(dense_values));
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        let mut dict = Self::dense_int_dict_int_to_dict(values);
                                        dict.insert(Arc::from(key.as_str()), value);
                                        *object = Value::Dict(Arc::new(dict));
                                    }
                                },
                                Value::DenseIntDictIntFull(values) => match key.parse::<i64>() {
                                    Ok(int_key) => {
                                        if int_key < 0 {
                                            let mut int_dict =
                                                Self::dense_int_dict_int_full_to_int_dict(values);
                                            int_dict.insert(int_key, value);
                                            *object = Value::IntDict(Arc::new(int_dict));
                                        } else {
                                            let index = int_key as usize;
                                            match value {
                                                Value::Int(int_value) => {
                                                    let values_mut = Arc::make_mut(values);
                                                    let len = values_mut.len();
                                                    if index == len {
                                                        values_mut.push(int_value);
                                                    } else if index < len {
                                                        values_mut[index] = int_value;
                                                    } else {
                                                        let mut sparse =
                                                            Self::dense_int_dict_int_full_to_sparse(
                                                                values,
                                                            );
                                                        sparse.resize(index + 1, None);
                                                        sparse[index] = Some(int_value);
                                                        *object = Value::DenseIntDictInt(Arc::new(
                                                            sparse,
                                                        ));
                                                    }
                                                }
                                                Value::Null => {
                                                    let mut sparse =
                                                        Self::dense_int_dict_int_full_to_sparse(
                                                            values,
                                                        );
                                                    if index >= sparse.len() {
                                                        sparse.resize(index + 1, None);
                                                    }
                                                    sparse[index] = None;
                                                    *object =
                                                        Value::DenseIntDictInt(Arc::new(sparse));
                                                }
                                                other => {
                                                    let mut dense_values =
                                                        Self::dense_int_dict_int_full_to_dense(
                                                            values,
                                                        );
                                                    if index >= dense_values.len() {
                                                        dense_values.resize(index + 1, Value::Null);
                                                    }
                                                    dense_values[index] = other;
                                                    *object =
                                                        Value::DenseIntDict(Arc::new(dense_values));
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        let mut dict = DictMap::default();
                                        for (index, value) in values.iter().enumerate() {
                                            dict.insert(
                                                Arc::from(index.to_string().as_str()),
                                                Value::Int(*value),
                                            );
                                        }
                                        dict.insert(Arc::from(key.as_str()), value);
                                        *object = Value::Dict(Arc::new(dict));
                                    }
                                },
                                _ => return Err("Invalid index assignment".to_string()),
                            },
                            _ => return Err("Invalid index assignment".to_string()),
                        }
                    }

                    // Push a null value to keep stack balanced (will be popped by following Pop instruction)
                    self.stack.push(Value::Null);
                }

                OpCode::FieldGet(field) => {
                    let object = self.stack.pop().ok_or("Stack underflow")?;

                    let result = match &object {
                        Value::Struct { name, fields } => {
                            if name == "ArgParser" {
                                match field.as_str() {
                                    "add_argument" | "parse" | "help" => {
                                        self.stack.push(object.clone());
                                        Value::NativeFunction(format!(
                                            "__arg_parser_method_{}",
                                            field
                                        ))
                                    }
                                    _ => {
                                        if let Some(value) = fields.get(&field) {
                                            value.clone()
                                        } else {
                                            let method_name = format!("{}.{}", name, field);
                                            let global =
                                                self.globals.lock().unwrap().get(&method_name);
                                            global
                                                .ok_or_else(|| {
                                                    format!("Field not found: {}", field)
                                                })?
                                                .clone()
                                        }
                                    }
                                }
                            } else if let Some(value) = fields.get(&field) {
                                value.clone()
                            } else {
                                let method_name = format!("{}.{}", name, field);
                                let global = self.globals.lock().unwrap().get(&method_name);
                                global.ok_or_else(|| format!("Field not found: {}", field))?.clone()
                            }
                        }
                        Value::Dict(dict) => {
                            dict.get(field.as_str()).cloned().unwrap_or(Value::Null)
                        }
                        Value::FixedDict { keys, values } => {
                            let idx = keys.iter().position(|k| k.as_ref() == field.as_str());
                            idx.and_then(|i| values.get(i).cloned()).unwrap_or(Value::Null)
                        }
                        Value::IntDict(dict) => match field.parse::<i64>() {
                            Ok(int_key) => dict.get(&int_key).cloned().unwrap_or(Value::Null),
                            Err(_) => Value::Null,
                        },
                        Value::DenseIntDict(values) => match field.parse::<i64>() {
                            Ok(int_key) => {
                                if int_key < 0 {
                                    Value::Null
                                } else {
                                    values.get(int_key as usize).cloned().unwrap_or(Value::Null)
                                }
                            }
                            Err(_) => Value::Null,
                        },
                        Value::Channel(_) => {
                            // For channels, we need to push the channel back on the stack
                            // and return a marker that this is a method call
                            // The Call opcode will handle the actual method invocation
                            self.stack.push(object.clone());
                            Value::NativeFunction(format!("__channel_method_{}", field))
                        }
                        #[cfg(feature = "runtime-image")]
                        Value::Image { .. } => {
                            // Mirror channel method marker behavior for image method dispatch.
                            self.stack.push(object.clone());
                            Value::NativeFunction(format!("__image_method_{}", field))
                        }
                        Value::HttpServer { .. } => match field.as_str() {
                            "route" | "listen" | "start" => {
                                // Mirror method marker behavior used by channel/image dispatch.
                                self.stack.push(object.clone());
                                Value::NativeFunction(format!("__http_server_method_{}", field))
                            }
                            _ => return Err(format!("HttpServer has no method '{}'", field)),
                        },
                        _ => {
                            return Err(format!(
                                "Cannot access field or method '{}' on non-struct value",
                                field
                            ))
                        }
                    };

                    self.stack.push(result);
                }

                OpCode::FieldSet(field) => {
                    let object = self.stack.pop().ok_or("Stack underflow")?;
                    let value = self.stack.pop().ok_or("Stack underflow")?;

                    match object {
                        Value::Struct { name, mut fields } => {
                            fields.insert(field, value);
                            if !self.push_checked_value(
                                Value::Struct { name, fields },
                                "building struct value",
                            )? {
                                continue;
                            }
                        }
                        Value::Dict(dict) => {
                            let mut dict_clone = dict;
                            let dict_mut = Arc::make_mut(&mut dict_clone);
                            dict_mut.insert(Arc::from(field), value);
                            if !self.push_checked_value(
                                Value::Dict(dict_clone),
                                "updating dictionary entry",
                            )? {
                                continue;
                            }
                        }
                        Value::FixedDict { keys, mut values } => {
                            if let Some(idx) =
                                keys.iter().position(|k| k.as_ref() == field.as_str())
                            {
                                values[idx] = value;
                                if !self.push_checked_value(
                                    Value::FixedDict { keys, values },
                                    "updating fixed dictionary entry",
                                )? {
                                    continue;
                                }
                            } else {
                                let mut dict = DictMap::default();
                                for (k, v) in keys.iter().cloned().zip(values.into_iter()) {
                                    dict.insert(k, v);
                                }
                                dict.insert(Arc::from(field), value);
                                if !self.push_checked_value(
                                    Value::Dict(Arc::new(dict)),
                                    "updating dictionary entry",
                                )? {
                                    continue;
                                }
                            }
                        }
                        Value::IntDict(dict) => {
                            let mut dict_clone = DictMap::default();
                            for (k, v) in dict.iter() {
                                dict_clone.insert(k.to_string().into(), v.clone());
                            }
                            dict_clone.insert(Arc::from(field), value);
                            if !self.push_checked_value(
                                Value::Dict(Arc::new(dict_clone)),
                                "updating dictionary entry",
                            )? {
                                continue;
                            }
                        }
                        Value::DenseIntDict(values) => match field.parse::<i64>() {
                            Ok(int_key) => {
                                if int_key < 0 {
                                    let mut int_dict = Self::dense_int_dict_to_int_dict(&values);
                                    int_dict.insert(int_key, value);
                                    if !self.push_checked_value(
                                        Value::IntDict(Arc::new(int_dict)),
                                        "updating integer dictionary entry",
                                    )? {
                                        continue;
                                    }
                                } else {
                                    let mut values = values;
                                    let values_mut = Arc::make_mut(&mut values);
                                    let index = int_key as usize;
                                    if index >= values_mut.len() {
                                        values_mut.resize(index + 1, Value::Null);
                                    }
                                    values_mut[index] = value;
                                    if !self.push_checked_value(
                                        Value::DenseIntDict(values),
                                        "updating dense integer dictionary entry",
                                    )? {
                                        continue;
                                    }
                                }
                            }
                            Err(_) => {
                                let mut dict = Self::dense_int_dict_to_dict(&values);
                                dict.insert(Arc::from(field), value);
                                if !self.push_checked_value(
                                    Value::Dict(Arc::new(dict)),
                                    "updating dictionary entry",
                                )? {
                                    continue;
                                }
                            }
                        },
                        _ => return Err("Cannot set field on non-struct".to_string()),
                    }
                }

                // Spread operations
                OpCode::SpreadArray => {
                    let array = self.stack.pop().ok_or("Stack underflow")?;

                    match array {
                        Value::Array(arr) => {
                            for elem in arr.iter() {
                                self.stack.push(elem.clone());
                            }
                        }
                        _ => return Err("Can only spread arrays".to_string()),
                    }
                }

                OpCode::SpreadDict => {
                    let dict = self.stack.pop().ok_or("Stack underflow")?;

                    match dict {
                        Value::Dict(d) => {
                            for (key, value) in d.iter() {
                                self.stack.push(Value::Str(Arc::new(key.to_string())));
                                self.stack.push(value.clone());
                            }
                        }
                        Value::FixedDict { keys, values } => {
                            for (key, value) in keys.iter().zip(values.iter()) {
                                self.stack.push(Value::Str(Arc::new(key.to_string())));
                                self.stack.push(value.clone());
                            }
                        }
                        Value::IntDict(d) => {
                            for (key, value) in d.iter() {
                                self.stack.push(Value::Str(Arc::new(key.to_string())));
                                self.stack.push(value.clone());
                            }
                        }
                        Value::DenseIntDict(values) => {
                            for (index, value) in values.iter().enumerate() {
                                self.stack.push(Value::Str(Arc::new(index.to_string())));
                                self.stack.push(value.clone());
                            }
                        }
                        _ => return Err("Can only spread dicts".to_string()),
                    }
                }

                OpCode::SpreadArgs => {
                    // Similar to SpreadArray but for function arguments
                    let array = self.stack.pop().ok_or("Stack underflow")?;

                    match array {
                        Value::Array(arr) => {
                            for elem in arr.iter() {
                                self.stack.push(elem.clone());
                            }
                        }
                        _ => return Err("Can only spread arrays as arguments".to_string()),
                    }
                }

                // Pattern matching
                OpCode::MatchPattern(pattern_index, binding_kind) => {
                    let constant = self.chunk.constants[pattern_index].clone();
                    if let Constant::Pattern(pattern) = constant {
                        let value = self.stack.last().ok_or("Stack underflow")?.clone();
                        let success = self.match_pattern(&pattern, &value, binding_kind)?;
                        self.stack.push(Value::Bool(success));
                    } else {
                        return Err("Expected pattern constant".to_string());
                    }
                }

                OpCode::MatchCasePattern(pattern) => {
                    let value = self.stack.last().ok_or("Stack underflow")?.clone();
                    let success = self.match_case_pattern(pattern.as_str(), &value);
                    self.stack.push(Value::Bool(success));
                }

                OpCode::BeginCase | OpCode::EndCase => {
                    // These are markers for debugging/disassembly
                }

                // Result/Option operations
                OpCode::MakeOk => {
                    let value = self.stack.pop().ok_or("Stack underflow")?;
                    if !self.push_checked_value(
                        Value::Result { is_ok: true, value: Box::new(value) },
                        "building result value",
                    )? {
                        continue;
                    }
                }

                OpCode::MakeErr => {
                    let value = self.stack.pop().ok_or("Stack underflow")?;
                    if !self.push_checked_value(
                        Value::Result { is_ok: false, value: Box::new(value) },
                        "building result value",
                    )? {
                        continue;
                    }
                }

                OpCode::MakeSome => {
                    let value = self.stack.pop().ok_or("Stack underflow")?;
                    if !self.push_checked_value(
                        Value::Option { is_some: true, value: Box::new(value) },
                        "building option value",
                    )? {
                        continue;
                    }
                }

                OpCode::MakeNone => {
                    if !self.push_checked_value(
                        Value::Option { is_some: false, value: Box::new(Value::Null) },
                        "building option value",
                    )? {
                        continue;
                    }
                }

                OpCode::TryUnwrap => {
                    let value = self.stack.pop().ok_or("Stack underflow")?;

                    match value {
                        Value::Result { is_ok, value } => {
                            if is_ok {
                                self.stack.push(*value);
                            } else {
                                // Early return from current function with Err(...)
                                let early_return = Value::Result { is_ok: false, value };
                                if let Some(frame) = self.call_frames.pop() {
                                    self.function_call_stack.pop();
                                    if self.recursion_depth > 0 {
                                        self.recursion_depth -= 1;
                                    }
                                    self.ip = frame.return_ip;
                                    if let Some(prev_chunk) = frame.prev_chunk {
                                        self.set_chunk(prev_chunk);
                                    }
                                    self.stack.truncate(frame.stack_offset);
                                    self.stack.push(early_return);
                                } else {
                                    return Ok(early_return);
                                }
                            }
                        }
                        Value::Option { is_some, value } => {
                            if is_some {
                                self.stack.push(*value);
                            } else {
                                // Early return from current function with None
                                let early_return =
                                    Value::Option { is_some: false, value: Box::new(Value::Null) };
                                if let Some(frame) = self.call_frames.pop() {
                                    self.function_call_stack.pop();
                                    if self.recursion_depth > 0 {
                                        self.recursion_depth -= 1;
                                    }
                                    self.ip = frame.return_ip;
                                    if let Some(prev_chunk) = frame.prev_chunk {
                                        self.set_chunk(prev_chunk);
                                    }
                                    self.stack.truncate(frame.stack_offset);
                                    self.stack.push(early_return);
                                } else {
                                    return Ok(early_return);
                                }
                            }
                        }
                        _ => return Err("Try operator requires Result or Option".to_string()),
                    }
                }

                // Struct operations
                OpCode::MakeStruct(name, fields) => {
                    let mut field_map = HashMap::with_capacity(fields.len());

                    for field_name in fields.iter().rev() {
                        let value = self.stack.pop().ok_or("Stack underflow")?;
                        field_map.insert(field_name.clone(), value);
                    }

                    if !self.push_checked_value(
                        Value::Struct { name, fields: field_map },
                        "building struct value",
                    )? {
                        continue;
                    }
                }

                // Environment management
                OpCode::PushScope => {
                    self.globals.lock().unwrap().push_scope();
                }
                OpCode::PopScope => {
                    self.globals.lock().unwrap().pop_scope();
                }

                // Iterator operations
                OpCode::MakeIterator => {
                    let collection = self.stack.pop().ok_or("Stack underflow")?;
                    // Call interpreter's built-in iterator function
                    let result = self.interpreter.call_native_function_impl("iter", &[collection]);
                    self.stack.push(result);
                }

                OpCode::IteratorNext => {
                    let iterator = self.stack.pop().ok_or("Stack underflow")?;
                    // Call next() on the iterator
                    let result = self
                        .interpreter
                        .call_native_function_impl("iterator_next", &[iterator.clone()]);
                    self.stack.push(iterator); // Keep iterator on stack
                    self.stack.push(result); // Push result (Some/None)
                }

                OpCode::IteratorHasNext => {
                    let iterator = self.stack.last().ok_or("Stack underflow")?.clone();
                    // Check if iterator has more values
                    let has_next = match &iterator {
                        Value::Iterator { index, source, .. } => match source.as_ref() {
                            Value::Array(arr) => *index < arr.len(),
                            _ => false,
                        },
                        _ => false,
                    };
                    self.stack.push(Value::Bool(has_next));
                }

                // Generator operations
                OpCode::MakeGenerator => {
                    // Pop the function from stack and convert it to a generator
                    let function = self.stack.pop().ok_or("Stack underflow in MakeGenerator")?;

                    if let Value::BytecodeFunction { chunk, captured, captured_binding_kinds } =
                        function
                    {
                        // Create initial generator state (not yet started)
                        let state = GeneratorState {
                            ip: 0,
                            stack: Vec::new(),
                            call_frames_data: Vec::new(),
                            chunk: chunk.clone(),
                            locals: HashMap::new(),
                            captured: captured.clone(),
                            captured_binding_kinds: captured_binding_kinds.clone(),
                            is_exhausted: false,
                        };

                        let generator =
                            Value::BytecodeGenerator { state: Arc::new(Mutex::new(state)) };

                        self.stack.push(generator);
                    } else {
                        return Err("MakeGenerator requires a BytecodeFunction".to_string());
                    }
                }

                OpCode::Yield => {
                    // Yield is handled specially in generator_next() method
                    // This opcode serves as a marker for the generator execution loop
                    // When we reach here in normal execution, it's an error
                    return Err("Yield can only be used inside generator functions".to_string());
                }

                OpCode::ResumeGenerator => {
                    // Resume generator by calling generator_next()
                    // This pops the generator from stack and pushes the result (Some(value) or None)
                    let generator = self.stack.pop().ok_or("Stack underflow in ResumeGenerator")?;
                    let result = self.generator_next(generator)?;
                    self.stack.push(result);
                }

                // Async/await operations
                OpCode::Await => {
                    // Pop promise from stack and await it
                    let promise = self.stack.pop().ok_or("Stack underflow in Await")?;

                    match promise {
                        Value::Promise { receiver, is_polled, cached_result, task_handle } => {
                            // Check if we've already polled this promise
                            {
                                let polled = is_polled.lock().unwrap();
                                let cached = cached_result.lock().unwrap();

                                if *polled {
                                    // Use cached result
                                    match cached.as_ref() {
                                        Some(Ok(val)) => {
                                            self.stack.push(val.clone());
                                            continue;
                                        }
                                        Some(Err(err)) => {
                                            return Err(format!("Promise rejected: {}", err));
                                        }
                                        None => {
                                            return Err(
                                                "Promise polled but no result cached".to_string()
                                            );
                                        }
                                    }
                                }
                            }

                            if self.cooperative_suspend_enabled {
                                let try_result = {
                                    let mut recv_guard = receiver.lock().unwrap();
                                    recv_guard.try_recv()
                                };

                                match try_result {
                                    Ok(Ok(value)) => {
                                        let mut polled = is_polled.lock().unwrap();
                                        let mut cached = cached_result.lock().unwrap();
                                        *cached = Some(Ok(value.clone()));
                                        *polled = true;
                                        self.stack.push(value);
                                    }
                                    Ok(Err(error)) => {
                                        let mut polled = is_polled.lock().unwrap();
                                        let mut cached = cached_result.lock().unwrap();
                                        *cached = Some(Err(error.clone()));
                                        *polled = true;
                                        return Err(format!("Promise rejected: {}", error));
                                    }
                                    Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                                        let pending_promise = Value::Promise {
                                            receiver,
                                            is_polled,
                                            cached_result,
                                            task_handle,
                                        };

                                        self.stack.push(pending_promise);
                                        self.ip = self.ip.saturating_sub(1);

                                        let context_id = if let Some(active_context) =
                                            self.active_execution_context
                                        {
                                            active_context
                                        } else {
                                            let new_context =
                                                self.create_execution_context_from_current();
                                            self.active_execution_context = Some(new_context);
                                            new_context
                                        };

                                        let snapshot = self.save_execution_state();
                                        self.execution_contexts.insert(context_id, snapshot);
                                        return Err(Self::suspend_error(context_id));
                                    }
                                    Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                                        let mut polled = is_polled.lock().unwrap();
                                        let mut cached = cached_result.lock().unwrap();
                                        *cached = Some(Err("Promise never resolved".to_string()));
                                        *polled = true;
                                        return Err(
                                            "Promise never resolved (channel closed)".to_string()
                                        );
                                    }
                                }

                                continue;
                            }

                            // Poll the promise using tokio runtime - blocks until result is ready
                            let result = {
                                let mut recv_guard = receiver.lock().unwrap();
                                // Take ownership by replacing with a dummy closed channel
                                let (dummy_tx, dummy_rx) = tokio::sync::oneshot::channel();
                                drop(dummy_tx); // Close immediately
                                let actual_rx = std::mem::replace(&mut *recv_guard, dummy_rx);
                                drop(recv_guard); // Release lock before blocking

                                // Debug logging
                                if std::env::var("DEBUG_ASYNC").is_ok() {
                                    eprintln!("VM Await: about to block_on receiver");
                                }

                                // Use the runtime handle to block on the receiver
                                // This works even when we're already inside a tokio runtime
                                let result = self.runtime_handle.block_on(actual_rx);

                                if std::env::var("DEBUG_ASYNC").is_ok() {
                                    eprintln!(
                                        "VM Await: block_on completed with result: {:?}",
                                        match &result {
                                            Ok(Ok(_)) => "Ok(Ok(value))",
                                            Ok(Err(e)) => {
                                                eprintln!("VM Await: Promise rejected: {}", e);
                                                "Ok(Err(...))"
                                            }
                                            Err(_) => "Err (channel closed)",
                                        }
                                    );
                                }

                                result
                            };
                            let mut polled = is_polled.lock().unwrap();
                            let mut cached = cached_result.lock().unwrap();

                            match result {
                                Ok(Ok(value)) => {
                                    *cached = Some(Ok(value.clone()));
                                    *polled = true;
                                    self.stack.push(value);
                                }
                                Ok(Err(error)) => {
                                    *cached = Some(Err(error.clone()));
                                    *polled = true;
                                    return Err(format!("Promise rejected: {}", error));
                                }
                                Err(_) => {
                                    *cached = Some(Err("Promise never resolved".to_string()));
                                    *polled = true;
                                    return Err(
                                        "Promise never resolved (channel closed)".to_string()
                                    );
                                }
                            }
                        }
                        _ => {
                            // Not a promise - just push it back (treat as already resolved)
                            self.stack.push(promise);
                        }
                    }
                }

                OpCode::MakePromise => {
                    // Pop value from stack and wrap it in a resolved promise
                    let value = self.stack.pop().ok_or("Stack underflow in MakePromise")?;

                    // Create a tokio oneshot channel that immediately sends the value
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    tx.send(Ok(value.clone())).map_err(|_| "Failed to send to promise channel")?;

                    // Create promise with the result already available
                    let promise = Value::Promise {
                        receiver: Arc::new(Mutex::new(rx)),
                        is_polled: Arc::new(Mutex::new(false)),
                        cached_result: Arc::new(Mutex::new(None)),
                        task_handle: None,
                    };

                    self.stack.push(promise);
                }

                OpCode::MarkAsync => {
                    // This is a no-op marker used during compilation
                    // It marks that the current context is async but doesn't generate runtime code
                }

                // Exception handling
                OpCode::BeginTry(catch_ip) => {
                    // Push exception handler onto stack
                    self.exception_handlers.push(ExceptionHandlerFrame {
                        catch_ip,
                        stack_offset: self.stack.len(),
                        frame_offset: self.call_frames.len(),
                    });
                }

                OpCode::EndTry => {
                    // Pop exception handler (normal exit from try block)
                    if self.exception_handlers.is_empty() {
                        return Err("EndTry without matching BeginTry".to_string());
                    }
                    self.exception_handlers.pop();
                }

                OpCode::Throw => {
                    let error_value = self.stack.pop().ok_or("Stack underflow in Throw")?;
                    self.throw_runtime_value(error_value)?;
                }

                OpCode::BeginCatch(var_name) => {
                    // Pop error from stack and bind to local variable
                    let error_value = self.stack.pop().ok_or("Stack underflow in BeginCatch")?;

                    // Convert error to structured error object if needed
                    let error_obj = match error_value {
                        Value::Str(msg) => {
                            // Simple string error - wrap in error struct
                            let mut fields = HashMap::new();
                            fields.insert("message".to_string(), Value::Str(msg));
                            fields.insert("stack".to_string(), Value::Array(Arc::new(Vec::new())));
                            fields.insert("line".to_string(), Value::Int(0));
                            Value::Struct { name: "Error".to_string(), fields }
                        }
                        Value::Error(msg) => {
                            // Legacy Error type - wrap in struct
                            let mut fields = HashMap::new();
                            fields.insert("message".to_string(), Value::Str(Arc::new(msg)));
                            fields.insert("stack".to_string(), Value::Array(Arc::new(Vec::new())));
                            fields.insert("line".to_string(), Value::Int(0));
                            Value::Struct { name: "Error".to_string(), fields }
                        }
                        Value::ErrorObject { message, stack, line, cause } => {
                            // Full error object - convert to struct
                            let mut fields = HashMap::new();
                            fields.insert("message".to_string(), Value::Str(Arc::new(message)));
                            fields.insert(
                                "stack".to_string(),
                                Value::Array(Arc::new(
                                    stack.iter().map(|s| Value::Str(Arc::new(s.clone()))).collect(),
                                )),
                            );
                            fields.insert("line".to_string(), Value::Int(line.unwrap_or(0) as i64));
                            if let Some(cause_val) = cause {
                                fields.insert("cause".to_string(), *cause_val);
                            }
                            Value::Struct { name: "Error".to_string(), fields }
                        }
                        other => {
                            // Any other value - wrap as message
                            let mut fields = HashMap::new();
                            fields.insert(
                                "message".to_string(),
                                Value::Str(Arc::new(format!("{:?}", other))),
                            );
                            fields.insert("stack".to_string(), Value::Array(Arc::new(Vec::new())));
                            fields.insert("line".to_string(), Value::Int(0));
                            Value::Struct { name: "Error".to_string(), fields }
                        }
                    };

                    // Bind error to variable in current frame
                    if let Some(frame) = self.call_frames.last_mut() {
                        frame.locals.insert(var_name, error_obj);
                    } else {
                        // No call frame - store in globals
                        self.globals.lock().unwrap().set(var_name, error_obj);
                    }
                }

                OpCode::EndCatch => {
                    // Nothing to do - handler already removed by Throw
                    // This opcode marks the end of the catch block for debugging/profiling
                }

                // Native function calls
                OpCode::CallNative(name, arg_count) => {
                    // Collect arguments from stack
                    let mut args = Vec::new();
                    for _ in 0..arg_count {
                        args.push(self.stack.pop().ok_or("Stack underflow in CallNative")?);
                    }
                    args.reverse();

                    let result: Result<Value, Value> = match name.as_str() {
                        "__vm_import_all" => self.vm_import_all(&args).map_err(Value::Error),
                        "__vm_import_symbol" => self.vm_import_symbol(&args).map_err(Value::Error),
                        _ => {
                            let native_result =
                                self.interpreter.call_native_function_impl(&name, &args);
                            match native_result {
                                Value::Error(msg) => Err(Value::Error(msg)),
                                Value::ErrorObject { .. } => Err(native_result),
                                other => Ok(other),
                            }
                        }
                    };

                    match result {
                        Ok(value) => self.stack.push(value),
                        Err(error_value) => self.throw_runtime_value(error_value)?,
                    }
                }

                // Closure & upvalue operations
                OpCode::CaptureUpvalue(name) => {
                    // Find the variable in the current scope (locals or globals)
                    let value = if let Some(frame) = self.call_frames.last() {
                        frame.locals.get(&name).cloned()
                    } else {
                        None
                    }
                    .or_else(|| {
                        // Try globals
                        self.globals.lock().unwrap().get(&name)
                    })
                    .ok_or_else(|| format!("Variable '{}' not found for capture", name))?;

                    // Create a new upvalue with the captured value
                    let upvalue = Upvalue {
                        value: Arc::new(Mutex::new(value)),
                        is_closed: true, // Immediately close it (move to heap)
                    };

                    let upvalue_index = self.upvalues.len();
                    self.upvalues.push(upvalue);

                    // Push the upvalue index onto the stack (for MakeClosure to use)
                    self.stack.push(Value::Int(upvalue_index as i64));
                }

                OpCode::LoadUpvalue(index) => {
                    // Load the value from the upvalue
                    if index >= self.upvalues.len() {
                        return Err(format!("Invalid upvalue index: {}", index));
                    }

                    let upvalue = &self.upvalues[index];
                    let value = upvalue.value.lock().unwrap().clone();
                    self.stack.push(value);
                }

                OpCode::StoreUpvalue(index) => {
                    // Store the top of stack to the upvalue
                    if index >= self.upvalues.len() {
                        return Err(format!("Invalid upvalue index: {}", index));
                    }

                    let value = self.stack.pop().ok_or("Stack underflow in StoreUpvalue")?;
                    let upvalue = &self.upvalues[index];
                    *upvalue.value.lock().unwrap() = value;
                }

                OpCode::CloseUpvalues(_slot) => {
                    // In our simplified implementation, upvalues are immediately closed
                    // (moved to heap) when captured. This operation is a no-op.
                    // A more sophisticated implementation would keep upvalues on the stack
                    // until they go out of scope, then move them to the heap.
                }

                // Channel operations
                OpCode::MakeChannel | OpCode::ChannelSend | OpCode::ChannelRecv => {
                    // Channels require concurrent runtime support
                    // For now, return an error - will implement in Week 5-6
                    return Err("Channel operations not yet implemented in VM".to_string());
                }

                // Debug operations
                OpCode::DebugPrint(msg) => {
                    eprintln!("DEBUG: {}", msg);
                }

                OpCode::Nop => {
                    // Do nothing
                }

                OpCode::DebugStack => {
                    eprintln!("Stack: {:?}", self.stack);
                }
            }
        }
    }

    fn define_import_binding_in_current_scope(&mut self, name: String, value: Value) {
        if let Some(frame) = self.call_frames.last_mut() {
            frame.locals_binding_kinds.insert(name.clone(), BytecodeBindingKind::Mutable);
            frame.locals.insert(name, value);
        } else {
            self.globals.lock().unwrap().define_with_kind(name, value, BindingKind::Mutable);
        }
    }

    fn module_binding_name(module_name: &str) -> String {
        module_name.rsplit('.').next().unwrap_or(module_name).to_string()
    }

    fn wrap_module_export_for_method_call(value: &Value) -> Value {
        match value {
            Value::Function(params, body, captured_env) => {
                let mut method_params = Vec::with_capacity(params.len() + 1);
                method_params.push("__module_receiver".to_string());
                method_params.extend(params.iter().cloned());
                Value::Function(method_params, body.clone(), captured_env.clone())
            }
            Value::AsyncFunction(params, body, captured_env) => {
                let mut method_params = Vec::with_capacity(params.len() + 1);
                method_params.push("__module_receiver".to_string());
                method_params.extend(params.iter().cloned());
                Value::AsyncFunction(method_params, body.clone(), captured_env.clone())
            }
            Value::GeneratorDef(params, body) => {
                let mut method_params = Vec::with_capacity(params.len() + 1);
                method_params.push("__module_receiver".to_string());
                method_params.extend(params.iter().cloned());
                Value::GeneratorDef(method_params, body.clone())
            }
            other => other.clone(),
        }
    }

    fn module_namespace_value(module_name: &str, exports: &HashMap<String, Value>) -> Value {
        let mut module_fields = HashMap::with_capacity(exports.len());
        for (name, value) in exports {
            module_fields.insert(name.clone(), Self::wrap_module_export_for_method_call(value));
        }
        Value::Struct { name: format!("__module_namespace_{}", module_name), fields: module_fields }
    }

    fn vm_import_all(&mut self, args: &[Value]) -> Result<Value, String> {
        if args.len() != 1 {
            return Err(format!("__vm_import_all expects 1 arguments, got {}", args.len()));
        }

        let module_name = match args.first() {
            Some(Value::Str(name)) => name.as_ref().clone(),
            _ => return Err("__vm_import_all expects module name as string".to_string()),
        };

        let exports = self
            .interpreter
            .module_loader
            .get_all_exports(&module_name)
            .map_err(|err| err.message)?;

        let module_binding_name = Self::module_binding_name(&module_name);
        let module_namespace_value = if exports.contains_key(module_binding_name.as_str()) {
            None
        } else {
            Some(Self::module_namespace_value(&module_name, &exports))
        };

        for (name, value) in exports {
            self.define_import_binding_in_current_scope(name, value);
        }

        if let Some(module_value) = module_namespace_value {
            self.define_import_binding_in_current_scope(module_binding_name, module_value);
        }

        Ok(Value::Null)
    }

    fn vm_import_symbol(&mut self, args: &[Value]) -> Result<Value, String> {
        if args.len() != 2 {
            return Err(format!("__vm_import_symbol expects 2 arguments, got {}", args.len()));
        }

        let module_name = match args.first() {
            Some(Value::Str(name)) => name.as_ref().clone(),
            _ => return Err("__vm_import_symbol expects module name as string".to_string()),
        };
        let symbol_name = match args.get(1) {
            Some(Value::Str(name)) => name.as_ref().clone(),
            _ => return Err("__vm_import_symbol expects symbol name as string".to_string()),
        };

        let value = self
            .interpreter
            .module_loader
            .get_symbol(&module_name, &symbol_name)
            .map_err(|err| err.message)?;

        self.define_import_binding_in_current_scope(symbol_name, value);
        Ok(Value::Null)
    }

    fn normalize_value_for_interpreter(value: Value) -> Value {
        match value {
            Value::Array(items) => {
                let normalized = items
                    .iter()
                    .cloned()
                    .map(Self::normalize_value_for_interpreter)
                    .collect::<Vec<_>>();
                Value::Array(Arc::new(normalized))
            }
            Value::Dict(map) => {
                let mut normalized = DictMap::default();
                for (key, entry) in map.iter() {
                    normalized.insert(
                        Arc::from(key.as_ref()),
                        Self::normalize_value_for_interpreter(entry.clone()),
                    );
                }
                Value::Dict(Arc::new(normalized))
            }
            Value::FixedDict { keys, values } => {
                let mut normalized = DictMap::default();
                for (key, entry) in keys.iter().zip(values.iter()) {
                    normalized.insert(
                        Arc::from(key.as_ref()),
                        Self::normalize_value_for_interpreter(entry.clone()),
                    );
                }
                Value::Dict(Arc::new(normalized))
            }
            Value::IntDict(map) => {
                let mut normalized = DictMap::default();
                for (key, entry) in map.iter() {
                    normalized.insert(
                        Arc::from(key.to_string().as_str()),
                        Self::normalize_value_for_interpreter(entry.clone()),
                    );
                }
                Value::Dict(Arc::new(normalized))
            }
            Value::DenseIntDict(values) => {
                let mut normalized = DictMap::default();
                for (index, entry) in values.iter().enumerate() {
                    normalized.insert(
                        Arc::from(index.to_string().as_str()),
                        Self::normalize_value_for_interpreter(entry.clone()),
                    );
                }
                Value::Dict(Arc::new(normalized))
            }
            Value::DenseIntDictInt(values) => {
                let mut normalized = DictMap::default();
                for (index, entry) in values.iter().enumerate() {
                    let value = entry.map(Value::Int).unwrap_or(Value::Null);
                    normalized.insert(Arc::from(index.to_string().as_str()), value);
                }
                Value::Dict(Arc::new(normalized))
            }
            Value::Struct { name, fields } => {
                let mut normalized_fields = HashMap::new();
                for (key, entry) in fields {
                    normalized_fields.insert(key, Self::normalize_value_for_interpreter(entry));
                }
                Value::Struct { name, fields: normalized_fields }
            }
            other => other,
        }
    }

    fn call_interpreter_callable(
        &mut self,
        function: &Value,
        args: &[Value],
    ) -> Result<Value, String> {
        let normalized_args =
            args.iter().cloned().map(Self::normalize_value_for_interpreter).collect::<Vec<_>>();
        let result = self.interpreter.call_user_function(function, &normalized_args);
        match result {
            Value::Error(message) => Err(message),
            Value::ErrorObject { message, .. } => Err(message),
            other => Ok(other),
        }
    }

    /// Convert a constant to a runtime value
    fn constant_to_value(&self, constant: &Constant) -> Result<Value, String> {
        match constant {
            Constant::Int(n) => Ok(Value::Int(*n)),
            Constant::Float(f) => Ok(Value::Float(*f)),
            Constant::String(s) => Ok(Value::Str(Arc::new(s.clone()))),
            Constant::Bool(b) => Ok(Value::Bool(*b)),
            Constant::None => Ok(Value::Null),
            Constant::Function(chunk) => Ok(Value::BytecodeFunction {
                chunk: (**chunk).clone(),
                captured: HashMap::new(),
                captured_binding_kinds: HashMap::new(),
            }),
            Constant::Pattern(_) => Err("Cannot convert pattern to value".to_string()),
            Constant::Type(_) => Err("Cannot convert type annotation to value".to_string()),
            Constant::Array(elements) => {
                let mut array = Vec::new();
                for elem in elements {
                    array.push(self.constant_to_value(elem)?);
                }
                Ok(Value::Array(Arc::new(array)))
            }
            Constant::Dict(pairs) => {
                let mut dict = DictMap::default();
                for (key_const, value_const) in pairs {
                    let key = self.constant_to_value(key_const)?;
                    let value = self.constant_to_value(value_const)?;

                    // Key must be a string
                    if let Value::Str(key_str) = key {
                        dict.insert(Arc::from(key_str.as_str()), value);
                    } else {
                        return Err("Dict constant keys must be strings".to_string());
                    }
                }
                Ok(Value::Dict(Arc::new(dict)))
            }
        }
    }

    fn prepare_bytecode_call_args(
        &self,
        chunk: &BytecodeChunk,
        mut args: Vec<Value>,
    ) -> Result<Vec<Value>, String> {
        let param_names = &chunk.params;

        let callable_name = chunk.name.clone().unwrap_or_else(|| "<lambda>".to_string());
        let is_method = chunk.name.as_deref().map(|name| name.contains('.')).unwrap_or(false)
            && matches!(args.first(), Some(Value::Struct { .. }));

        if is_method {
            let has_self_param = param_names.first().map(|p| p == "self").unwrap_or(false);
            let external_params: Vec<String> = if has_self_param {
                param_names.iter().skip(1).cloned().collect()
            } else {
                param_names.clone()
            };

            let arity = CallableArity::exact(callable_name, external_params);
            let external_args_count = args.len().saturating_sub(1);
            arity.validate(external_args_count)?;

            // Compatibility: allow legacy methods compiled without explicit self.
            if !has_self_param && args.len() == param_names.len() + 1 {
                args.remove(0);
            }
        } else {
            let arity = CallableArity::exact(callable_name, param_names.clone());
            arity.validate(args.len())?;
        }

        Ok(args)
    }

    /// Call a function
    /// Set up a call frame for a bytecode function (doesn't return - Return opcode will handle that)
    fn call_bytecode_function(
        &mut self,
        function: Value,
        raw_args: Vec<Value>,
        call_args: Vec<Value>,
    ) -> Result<(), String> {
        if let Value::BytecodeFunction { chunk, captured, captured_binding_kinds } = function {
            let max_depth = runtime_limits::DEFAULT_MAX_VM_CALL_DEPTH;
            if self.call_frames.len() >= max_depth || self.recursion_depth >= max_depth {
                let callable = chunk.name.as_deref().unwrap_or("<anonymous>");
                return Err(format!(
                    "Maximum VM call stack depth of {} exceeded while calling {}",
                    max_depth, callable
                ));
            }

            // Create new call frame with parameters bound
            let mut locals = HashMap::new();
            let mut locals_binding_kinds = HashMap::new();

            // Backward-compat method support:
            // For methods compiled without explicit `self`, interpreter mode exposes
            // receiver fields as lexical names inside the method body. Mirror that
            // behavior in VM mode so legacy method code like `return x * 2` remains
            // additive and parity-safe.
            let is_legacy_method =
                chunk.name.as_deref().map(|name| name.contains('.')).unwrap_or(false)
                    && chunk.params.first().map(|p| p == "self").unwrap_or(false) == false;

            let compat_receiver_fields =
                if is_legacy_method && raw_args.len() == chunk.params.len() + 1 {
                    match raw_args.first() {
                        Some(Value::Struct { fields, .. }) => Some(fields.clone()),
                        _ => None,
                    }
                } else if is_legacy_method && raw_args.is_empty() {
                    // Some call lowerings can leave the receiver on the VM value stack
                    // instead of in explicit call arguments for field-access calls.
                    match self.stack.last() {
                        Some(Value::Struct { fields, .. }) => Some(fields.clone()),
                        _ => None,
                    }
                } else {
                    None
                };
            let param_names = &chunk.params;

            let mut local_slots = vec![Value::Null; chunk.local_count];
            let mut local_slot_binding_kinds =
                if chunk.local_binding_kinds.len() < chunk.local_count {
                    let mut kinds = chunk.local_binding_kinds.clone();
                    kinds.resize(chunk.local_count, BytecodeBindingKind::Mutable);
                    kinds
                } else {
                    chunk.local_binding_kinds.clone()
                };
            let mut local_slot_initialized = vec![false; chunk.local_count];

            if let Some(fields) = compat_receiver_fields {
                for (field_name, field_value) in fields {
                    locals.insert(field_name.clone(), field_value.clone());
                    locals_binding_kinds.insert(field_name.clone(), BytecodeBindingKind::Mutable);
                    if let Some(slot) =
                        chunk.local_names.iter().position(|name| name == &field_name)
                    {
                        if slot < local_slots.len() {
                            local_slots[slot] = field_value.clone();
                            local_slot_binding_kinds[slot] = BytecodeBindingKind::Mutable;
                            local_slot_initialized[slot] = true;
                        }
                    }
                }
            }

            // Bind each argument to its corresponding parameter name
            for (param_name, arg_value) in param_names.iter().zip(call_args.iter()) {
                locals.insert(param_name.clone(), arg_value.clone());
                locals_binding_kinds.insert(param_name.clone(), BytecodeBindingKind::Mutable);
                if let Some(slot) = chunk.local_names.iter().position(|name| name == param_name) {
                    if slot < local_slots.len() {
                        local_slots[slot] = arg_value.clone();
                        local_slot_binding_kinds[slot] = BytecodeBindingKind::Mutable;
                        local_slot_initialized[slot] = true;
                    }
                }
            }

            // Prepare captured variables HashMap for mutable access
            let mut captured_map = HashMap::new();
            for (name, value_ref) in &captured {
                captured_map.insert(name.clone(), value_ref.clone());
            }
            let captured_binding_kinds_map = captured_binding_kinds.clone();

            if std::env::var("DEBUG_VM").is_ok() {
                eprintln!(
                    "CallFrame has {} captured variables: {:?}",
                    captured_map.len(),
                    captured_map.keys().collect::<Vec<_>>()
                );
            }

            if chunk.is_generator {
                let frame_data = CallFrameData {
                    return_ip: 0,
                    stack_offset: 0,
                    locals: locals.clone(),
                    locals_binding_kinds: locals_binding_kinds.clone(),
                    local_slots: local_slots.clone(),
                    local_slot_binding_kinds: local_slot_binding_kinds.clone(),
                    local_slot_initialized: local_slot_initialized.clone(),
                    captured: captured_map.clone(),
                    captured_binding_kinds: captured_binding_kinds_map.clone(),
                };

                let state = GeneratorState {
                    ip: 0,
                    stack: Vec::new(),
                    call_frames_data: vec![frame_data],
                    chunk,
                    locals,
                    captured: captured_map,
                    captured_binding_kinds: captured_binding_kinds_map,
                    is_exhausted: false,
                };

                self.stack.push(Value::BytecodeGenerator { state: Arc::new(Mutex::new(state)) });
                return Ok(());
            }

            let frame = CallFrame {
                return_ip: self.ip,
                stack_offset: self.stack.len(),
                locals,
                locals_binding_kinds,
                local_slots,
                local_slot_binding_kinds,
                local_slot_initialized,
                captured: captured_map,
                captured_binding_kinds: captured_binding_kinds_map,
                prev_chunk: Some(self.chunk.clone()),
                is_async: chunk.is_async,
            };

            self.call_frames.push(frame);

            // Track recursion depth for optimization and profiling
            self.recursion_depth += 1;
            if self.recursion_depth > self.max_recursion_depth {
                self.max_recursion_depth = self.recursion_depth;
            }

            // Track function call for error reporting
            let func_name = chunk.name.as_deref().unwrap_or("<anonymous>").to_string();
            self.function_call_stack.push(func_name);

            // Switch to function's chunk and reset IP
            self.set_chunk(chunk);
            self.ip = 0;

            Ok(())
        } else {
            Err("Expected BytecodeFunction".to_string())
        }
    }

    fn match_http_route_pattern(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        if pattern_parts.len() != path_parts.len() {
            return None;
        }

        let mut params = HashMap::new();
        for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
            if let Some(param_name) = pattern_part.strip_prefix(':') {
                params.insert(param_name.to_string(), path_part.to_string());
            } else if *pattern_part != *path_part {
                return None;
            }
        }

        Some(params)
    }

    fn http_value_to_constant(value: &Value) -> Result<Constant, String> {
        match value {
            Value::Null => Ok(Constant::None),
            Value::Int(n) => Ok(Constant::Int(*n)),
            Value::Float(f) => Ok(Constant::Float(*f)),
            Value::Str(s) => Ok(Constant::String(s.as_ref().clone())),
            Value::Bool(b) => Ok(Constant::Bool(*b)),
            Value::Array(items) => {
                let mut constants = Vec::with_capacity(items.len());
                for item in items.iter() {
                    constants.push(Self::http_value_to_constant(item)?);
                }
                Ok(Constant::Array(constants))
            }
            Value::Dict(dict) => {
                let mut pairs = Vec::with_capacity(dict.len());
                for (key, value) in dict.iter() {
                    pairs.push((
                        Constant::String(key.as_ref().to_string()),
                        Self::http_value_to_constant(value)?,
                    ));
                }
                Ok(Constant::Dict(pairs))
            }
            Value::FixedDict { keys, values } => {
                let mut pairs = Vec::with_capacity(keys.len());
                for (key, value) in keys.iter().zip(values.iter()) {
                    pairs.push((
                        Constant::String(key.as_ref().to_string()),
                        Self::http_value_to_constant(value)?,
                    ));
                }
                Ok(Constant::Dict(pairs))
            }
            _ => Err(format!(
                "Unsupported request value type for HTTP handler invocation: {:?}",
                value
            )),
        }
    }

    fn call_http_handler_vm(&mut self, handler: Value, req_obj: Value) -> Result<Value, String> {
        match handler {
            Value::BytecodeFunction { .. } | Value::NativeFunction(_) => {
                let handler_name = format!("__http_handler_tmp_{}", self.next_execution_context_id);
                self.next_execution_context_id = self.next_execution_context_id.wrapping_add(1);

                {
                    let mut globals = self.globals.lock().unwrap();
                    globals.set(handler_name.clone(), handler);
                }

                let req_constant = Self::http_value_to_constant(&req_obj)?;

                let mut wrapper_chunk = BytecodeChunk::new();
                wrapper_chunk.name = Some("__http_handler_wrapper".to_string());
                let request_constant_idx = wrapper_chunk.add_constant(req_constant);

                wrapper_chunk.emit(OpCode::LoadConst(request_constant_idx));
                wrapper_chunk.emit(OpCode::LoadGlobal(handler_name.clone()));
                wrapper_chunk.emit(OpCode::Call(1));
                wrapper_chunk.emit(OpCode::Return);

                let mut temp_vm = VM::new();
                temp_vm.jit_enabled = false;
                temp_vm.set_capability_policy(self.interpreter.capability_policy().clone());
                temp_vm.set_globals(Arc::clone(&self.globals));
                let result = temp_vm.execute(wrapper_chunk);

                {
                    let mut globals = self.globals.lock().unwrap();
                    globals.set(handler_name, Value::Null);
                }

                result
            }
            _ => Err(Self::non_callable_error_message(
                "route handlers must evaluate to a callable function",
            )),
        }
    }

    fn start_http_server_vm(
        &mut self,
        host: String,
        port: u16,
        routes: Vec<(String, String, Value)>,
    ) -> Result<Value, String> {
        use tiny_http::{Response, Server};
        if let Err(error) = self
            .interpreter
            .require_capability(NativeCapability::NetworkServer, "http_server.listen")
        {
            if let Value::Error(message) = error {
                return Err(message);
            }
            return Err("Capability denied: network-server required for http_server.listen; rerun with --allow-net-server".to_string());
        }

        println!("Starting HTTP server on {}:{}...", host, port);
        let server = Server::http(format!("{}:{}", host, port))
            .map_err(|e| format!("Failed to start server: {}", e))?;

        println!("Server listening on http://{}:{}", host, port);
        println!("Press Ctrl+C to stop");

        for mut request in server.incoming_requests() {
            let method = request.method().to_string();
            let request_url = request.url().to_string();
            let (url_path, query_params, decoded_query_params, raw_query) =
                http_request_utils::split_http_path_and_query_with_decoded(&request_url);

            let body_content = {
                let mut reader = request.as_reader();
                let mut buffer = Vec::new();
                std::io::Read::read_to_end(&mut reader, &mut buffer).ok();
                String::from_utf8_lossy(&buffer).to_string()
            };

            let mut matched_handler: Option<(Value, HashMap<String, String>)> = None;

            for (route_method, route_path, handler) in &routes {
                if method == *route_method && url_path == *route_path {
                    matched_handler = Some((handler.clone(), HashMap::new()));
                    break;
                }
            }

            if matched_handler.is_none() {
                for (route_method, route_path, handler) in &routes {
                    if method != *route_method {
                        continue;
                    }
                    if route_path.contains(':') {
                        if let Some(path_params) =
                            Self::match_http_route_pattern(route_path, &url_path)
                        {
                            matched_handler = Some((handler.clone(), path_params));
                            break;
                        }
                    }
                }
            }

            let response = if let Some((handler, path_params)) = matched_handler {
                let mut params_dict = DictMap::default();
                for (key, value) in &path_params {
                    params_dict
                        .insert(Arc::from(key.as_str()), Value::Str(Arc::new(value.clone())));
                }

                let mut req_fields = DictMap::default();
                req_fields.insert("method".into(), Value::Str(Arc::new(method.clone())));
                req_fields.insert("path".into(), Value::Str(Arc::new(url_path.clone())));
                req_fields.insert("raw_path".into(), Value::Str(Arc::new(request_url.clone())));
                req_fields.insert("body".into(), Value::Str(Arc::new(body_content.clone())));
                req_fields.insert("params".into(), Value::Dict(Arc::new(params_dict)));

                let mut query_dict = DictMap::default();
                for (key, value) in &query_params {
                    query_dict.insert(Arc::from(key.as_str()), Value::Str(Arc::new(value.clone())));
                }
                req_fields.insert("query".into(), Value::Dict(Arc::new(query_dict)));

                let mut decoded_query_dict = DictMap::default();
                for (key, value) in &decoded_query_params {
                    decoded_query_dict
                        .insert(Arc::from(key.as_str()), Value::Str(Arc::new(value.clone())));
                }
                req_fields
                    .insert("query_decoded".into(), Value::Dict(Arc::new(decoded_query_dict)));
                req_fields.insert("query_string".into(), Value::Str(Arc::new(raw_query.clone())));

                let mut headers_dict = DictMap::default();
                for header in request.headers() {
                    let header_name = header.field.as_str().to_string();
                    let header_value = header.value.as_str().to_string();
                    headers_dict.insert(header_name.into(), Value::Str(Arc::new(header_value)));
                }
                req_fields.insert("headers".into(), Value::Dict(Arc::new(headers_dict)));

                match self.call_http_handler_vm(handler, Value::Dict(Arc::new(req_fields))) {
                    Ok(Value::HttpResponse { status, body, headers }) => {
                        let mut response = Response::from_string(body).with_status_code(status);
                        for (key, value) in headers {
                            if let Ok(header) =
                                tiny_http::Header::from_bytes(key.as_bytes(), value.as_bytes())
                            {
                                response = response.with_header(header);
                            }
                        }
                        response
                    }
                    Ok(_other) => Response::from_string(
                        "Internal Server Error: route handler must return an HTTP response",
                    )
                    .with_status_code(500),
                    Err(error) => Response::from_string(error).with_status_code(500),
                }
            } else {
                Response::from_string("Not Found").with_status_code(404)
            };

            let _ = request.respond(response);
        }

        Ok(Value::Int(0))
    }

    /// Call a native function (returns synchronously)
    fn call_native_function_vm(
        &mut self,
        function: Value,
        mut args: Vec<Value>,
    ) -> Result<Value, String> {
        if let Value::NativeFunction(name) = function {
            if name == "__vm_for_iterable" {
                if args.len() != 1 {
                    return Err(format!(
                        "__vm_for_iterable expects 1 argument, got {}",
                        args.len()
                    ));
                }
                if let Value::BytecodeGenerator { .. } = &args[0] {
                    let mut values = Vec::new();
                    let generator_value = args[0].clone();
                    loop {
                        let step = self.generator_next(generator_value.clone())?;
                        match step {
                            Value::Option { is_some: true, value } => values.push(*value),
                            Value::Option { is_some: false, .. } => break,
                            other => {
                                return Err(format!(
                                    "__vm_for_iterable expected generator step to return Option, got {:?}",
                                    other
                                ));
                            }
                        }
                    }
                    return Ok(Value::Array(Arc::new(values)));
                }
            }

            // Handle channel method calls
            if name.starts_with("__channel_method_") {
                let method_name = name.strip_prefix("__channel_method_").unwrap();
                // The channel object was pushed onto the stack before the function marker
                // So it's at the bottom of our "args" - but actually, it's still on the stack
                // because FieldGet pushed it back. We need to pop it from the stack!
                let channel = self.stack.pop().ok_or("Stack underflow getting channel")?;

                if let Value::Channel(chan) = channel {
                    return match method_name {
                        "send" => {
                            if args.len() != 1 {
                                return Err(format!(
                                    "Channel.send expects 1 arguments, got {}",
                                    args.len()
                                ));
                            }
                            let value = args.remove(0);
                            let chan_lock = chan.lock().unwrap();
                            let (sender, _) = &*chan_lock;
                            match sender.send(value) {
                                Ok(_) => Ok(Value::Bool(true)),
                                Err(_) => Err("Failed to send to channel".to_string()),
                            }
                        }
                        "receive" => {
                            if !args.is_empty() {
                                return Err(format!(
                                    "Channel.receive expects 0 arguments, got {}",
                                    args.len()
                                ));
                            }
                            let chan_lock = chan.lock().unwrap();
                            let (_, receiver) = &*chan_lock;
                            match receiver.try_recv() {
                                Ok(value) => Ok(value),
                                Err(std::sync::mpsc::TryRecvError::Empty) => Ok(Value::Null),
                                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                    Err("Channel disconnected".to_string())
                                }
                            }
                        }
                        _ => Err(format!("Channel has no method '{}'", method_name)),
                    };
                } else {
                    return Err("Expected Channel for channel method call".to_string());
                }
            }

            // Handle image method calls.
            if name.starts_with("__image_method_") {
                let method_name = name.strip_prefix("__image_method_").unwrap();

                if method_name == "save" {
                    if let Err(error) = self
                        .interpreter
                        .require_capability(NativeCapability::FilesystemWrite, "save")
                    {
                        if let Value::Error(message) = error {
                            return Err(message);
                        }
                        return Err("Capability denied: filesystem-write required for save; rerun with --allow-fs-write".to_string());
                    }
                }

                // Remove the duplicate receiver argument emitted by MethodCall compilation.
                if !args.is_empty() {
                    args.pop();
                }

                // Retrieve receiver value left on the VM stack.
                let image = self.stack.pop().ok_or("Stack underflow getting image")?;

                match Interpreter::call_image_method_impl(&image, method_name, &args) {
                    Some(Value::Error(msg)) => return Err(msg),
                    Some(other) => return Ok(other),
                    None => return Err("Expected Image for image method call".to_string()),
                }
            }

            // Handle HttpServer method calls.
            if name.starts_with("__http_server_method_") {
                let method_name = name.strip_prefix("__http_server_method_").unwrap();

                // Remove duplicate receiver argument emitted by MethodCall compilation.
                if !args.is_empty() {
                    args.pop();
                }

                let server = self.stack.pop().ok_or("Stack underflow getting HTTP server")?;

                if let Value::HttpServer { host, port, routes } = server {
                    return match method_name {
                        "route" => {
                            if args.len() != 3 {
                                Err("route() requires (method, path, handler_function)".to_string())
                            } else {
                                match (&args[0], &args[1], &args[2]) {
                                    (
                                        Value::Str(method),
                                        Value::Str(path),
                                        Value::BytecodeFunction { .. } | Value::NativeFunction(_),
                                    ) => {
                                        let mut new_routes = routes.clone();
                                        new_routes.push((
                                            method.as_ref().clone(),
                                            path.as_ref().clone(),
                                            args[2].clone(),
                                        ));
                                        Ok(Value::HttpServer { host, port, routes: new_routes })
                                    }
                                    _ => Err("route() requires (method, path, handler_function)"
                                        .to_string()),
                                }
                            }
                        }
                        "listen" | "start" => {
                            if !args.is_empty() {
                                Err(format!(
                                    "HttpServer.{} expects 0 arguments, got {}",
                                    method_name,
                                    args.len()
                                ))
                            } else {
                                if let Err(error) = self.interpreter.require_capability(
                                    NativeCapability::NetworkServer,
                                    "http_server.listen",
                                ) {
                                    if let Value::Error(message) = error {
                                        return Err(message);
                                    }
                                    return Err("Capability denied: network-server required for http_server.listen; rerun with --allow-net-server".to_string());
                                }
                                self.start_http_server_vm(host, port, routes)
                            }
                        }
                        _ => Err(format!("HttpServer has no method '{}'", method_name)),
                    };
                } else {
                    return Err("Expected HttpServer for HTTP server method call".to_string());
                }
            }

            // Handle ArgParser method calls.
            if name.starts_with("__arg_parser_method_") {
                let method_name = name.strip_prefix("__arg_parser_method_").unwrap();

                // Remove duplicate receiver argument emitted by MethodCall compilation.
                if !args.is_empty() {
                    args.pop();
                }

                let parser = self.stack.pop().ok_or("Stack underflow getting ArgParser")?;

                if let Value::Struct { name, mut fields } = parser {
                    if name != "ArgParser" {
                        return Err("Expected ArgParser for arg_parser method call".to_string());
                    }

                    return match method_name {
                        "add_argument" => {
                            if args.is_empty() {
                                return Err("add_argument requires at least a long argument name"
                                    .to_string());
                            }

                            let mut long_name = String::new();
                            let mut short_name: Option<String> = None;
                            let mut arg_type = String::from("string");
                            let mut required = false;
                            let mut help = String::new();
                            let mut default: Option<String> = None;

                            if let Some(Value::Str(s)) = args.first() {
                                long_name = s.as_ref().clone();
                            }

                            let mut index = 1;
                            if let Some(Value::Str(s)) = args.get(1) {
                                let short_candidate = s.as_ref();
                                if short_candidate.starts_with('-')
                                    && short_candidate != "short"
                                    && short_candidate != "type"
                                    && short_candidate != "required"
                                    && short_candidate != "help"
                                    && short_candidate != "default"
                                {
                                    short_name = Some(short_candidate.to_string());
                                    index = 2;
                                }
                            }

                            while index + 1 < args.len() {
                                if let Value::Str(key) = &args[index] {
                                    let value = &args[index + 1];
                                    match key.as_ref().as_str() {
                                        "short" => {
                                            if let Value::Str(s) = value {
                                                short_name = Some(s.as_ref().clone());
                                            }
                                        }
                                        "type" => {
                                            if let Value::Str(s) = value {
                                                arg_type = s.as_ref().clone();
                                            }
                                        }
                                        "required" => {
                                            if let Value::Bool(b) = value {
                                                required = *b;
                                            }
                                        }
                                        "help" => {
                                            if let Value::Str(s) = value {
                                                help = s.as_ref().clone();
                                            }
                                        }
                                        "default" => {
                                            if let Value::Str(s) = value {
                                                default = Some(s.as_ref().clone());
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                index += 2;
                            }

                            let mut arg_def = DictMap::default();
                            arg_def.insert("long".into(), Value::Str(Arc::new(long_name)));
                            if let Some(short) = short_name {
                                arg_def.insert("short".into(), Value::Str(Arc::new(short)));
                            }
                            arg_def.insert("type".into(), Value::Str(Arc::new(arg_type)));
                            arg_def.insert("required".into(), Value::Bool(required));
                            arg_def.insert("help".into(), Value::Str(Arc::new(help)));
                            if let Some(def) = default {
                                arg_def.insert("default".into(), Value::Str(Arc::new(def)));
                            }

                            if let Some(Value::Array(arg_list)) = fields.get("_args").cloned() {
                                let mut arg_list_vec =
                                    Arc::try_unwrap(arg_list).unwrap_or_else(|arc| (*arc).clone());
                                arg_list_vec.push(Value::Dict(Arc::new(arg_def)));
                                fields.insert(
                                    "_args".to_string(),
                                    Value::Array(Arc::new(arg_list_vec)),
                                );
                            }

                            Ok(Value::Struct { name, fields })
                        }
                        "parse" => {
                            let mut arg_defs: Vec<crate::builtins::ArgumentDef> = Vec::new();

                            if let Some(Value::Array(arg_list)) = fields.get("_args") {
                                for arg_val in arg_list.iter() {
                                    if let Value::Dict(arg_dict) = arg_val {
                                        let long_name = match arg_dict.get("long") {
                                            Some(Value::Str(s)) => s.as_ref().clone(),
                                            _ => continue,
                                        };

                                        let short_name = match arg_dict.get("short") {
                                            Some(Value::Str(s)) => Some(s.as_ref().clone()),
                                            _ => None,
                                        };

                                        let arg_type = match arg_dict.get("type") {
                                            Some(Value::Str(s)) => s.as_ref().clone(),
                                            _ => "string".to_string(),
                                        };

                                        let required = match arg_dict.get("required") {
                                            Some(Value::Bool(b)) => *b,
                                            _ => false,
                                        };

                                        let help = match arg_dict.get("help") {
                                            Some(Value::Str(s)) => s.as_ref().clone(),
                                            _ => String::new(),
                                        };

                                        let default = match arg_dict.get("default") {
                                            Some(Value::Str(s)) => Some(s.as_ref().clone()),
                                            _ => None,
                                        };

                                        arg_defs.push(crate::builtins::ArgumentDef {
                                            long_name,
                                            short_name,
                                            arg_type,
                                            required,
                                            help,
                                            default,
                                        });
                                    }
                                }
                            }

                            let cli_args = crate::builtins::get_args();
                            match crate::builtins::parse_arguments(&arg_defs, &cli_args) {
                                Ok(parsed) => Ok(Value::Dict(Arc::new(parsed))),
                                Err(msg) => Err(msg),
                            }
                        }
                        "help" => {
                            let mut arg_defs: Vec<crate::builtins::ArgumentDef> = Vec::new();

                            if let Some(Value::Array(arg_list)) = fields.get("_args") {
                                for arg_val in arg_list.iter() {
                                    if let Value::Dict(arg_dict) = arg_val {
                                        let long_name = match arg_dict.get("long") {
                                            Some(Value::Str(s)) => s.as_ref().clone(),
                                            _ => continue,
                                        };

                                        let short_name = match arg_dict.get("short") {
                                            Some(Value::Str(s)) => Some(s.as_ref().clone()),
                                            _ => None,
                                        };

                                        let arg_type = match arg_dict.get("type") {
                                            Some(Value::Str(s)) => s.as_ref().clone(),
                                            _ => "string".to_string(),
                                        };

                                        let required = match arg_dict.get("required") {
                                            Some(Value::Bool(b)) => *b,
                                            _ => false,
                                        };

                                        let help = match arg_dict.get("help") {
                                            Some(Value::Str(s)) => s.as_ref().clone(),
                                            _ => String::new(),
                                        };

                                        let default = match arg_dict.get("default") {
                                            Some(Value::Str(s)) => Some(s.as_ref().clone()),
                                            _ => None,
                                        };

                                        arg_defs.push(crate::builtins::ArgumentDef {
                                            long_name,
                                            short_name,
                                            arg_type,
                                            required,
                                            help,
                                            default,
                                        });
                                    }
                                }
                            }

                            let app_name = match fields.get("_app_name") {
                                Some(Value::Str(s)) => s.as_ref().clone(),
                                _ => "program".to_string(),
                            };

                            let description = match fields.get("_description") {
                                Some(Value::Str(s)) => s.as_ref().clone(),
                                _ => String::new(),
                            };

                            let help_text =
                                crate::builtins::generate_help(&arg_defs, &app_name, &description);
                            Ok(Value::Str(Arc::new(help_text)))
                        }
                        _ => Err(format!("ArgParser has no method '{}'", method_name)),
                    };
                } else {
                    return Err("Expected ArgParser for arg_parser method call".to_string());
                }
            }

            if name == "dict" {
                if !args.is_empty() {
                    return Err("dict() expects 0 arguments".to_string());
                }
                return Ok(Value::Dict(Arc::new(DictMap::default())));
            }

            if let Some(result) = self.call_vm_higher_order(&name, &args) {
                return result;
            }

            // Use the interpreter's native function implementation
            // This gives us access to ALL 100+ built-in functions automatically
            let result = self.interpreter.call_native_function_impl(&name, &args);

            // Check if the result is an error
            match result {
                Value::Error(msg) => Err(msg),
                Value::ErrorObject { message, .. } => Err(message),
                other => Ok(other),
            }
        } else {
            Err("Expected NativeFunction".to_string())
        }
    }

    /// Handle higher-order array functions that receive bytecode closures
    fn call_vm_higher_order(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> Option<Result<Value, String>> {
        match name {
            "map" => {
                if args.len() < 2 {
                    return Some(Err("map requires two arguments: array and function".to_string()));
                }

                let (array, func) = match (args.first(), args.get(1)) {
                    (Some(Value::Array(arr)), Some(func @ Value::BytecodeFunction { .. })) => {
                        (arr.clone(), func.clone())
                    }
                    _ => return None,
                };

                let mut result = Vec::with_capacity(array.len());
                for element in array.iter() {
                    let func_result =
                        match self.call_function_from_jit(func.clone(), vec![element.clone()]) {
                            Ok(value) => value,
                            Err(message) => return Some(Err(message)),
                        };
                    result.push(func_result);
                }

                Some(Ok(Value::Array(Arc::new(result))))
            }
            "filter" => {
                if args.len() < 2 {
                    return Some(Err(
                        "filter requires two arguments: array and function".to_string()
                    ));
                }

                let (array, func) = match (args.first(), args.get(1)) {
                    (Some(Value::Array(arr)), Some(func @ Value::BytecodeFunction { .. })) => {
                        (arr.clone(), func.clone())
                    }
                    _ => return None,
                };

                let mut result = Vec::new();
                for element in array.iter() {
                    let func_result =
                        match self.call_function_from_jit(func.clone(), vec![element.clone()]) {
                            Ok(value) => value,
                            Err(message) => return Some(Err(message)),
                        };

                    if self.is_truthy(&func_result) {
                        result.push(element.clone());
                    }
                }

                Some(Ok(Value::Array(Arc::new(result))))
            }
            "reduce" => {
                if args.len() < 3 {
                    return Some(Err(
                        "reduce requires three arguments: array, initial value, and function"
                            .to_string(),
                    ));
                }

                let (array, initial, func) = match (args.first(), args.get(1), args.get(2)) {
                    (
                        Some(Value::Array(arr)),
                        Some(init),
                        Some(func @ Value::BytecodeFunction { .. }),
                    ) => (arr.clone(), init.clone(), func.clone()),
                    _ => return None,
                };

                if let Some((op, swap_operands)) = self.match_simple_binary_reduce(&func) {
                    let mut accumulator = initial;
                    for element in array.iter() {
                        let element_value = element.clone();
                        let (left, right) = if swap_operands {
                            (element_value, accumulator)
                        } else {
                            (accumulator, element_value)
                        };

                        accumulator = match self.binary_op(&left, op, &right) {
                            Ok(value) => value,
                            Err(message) => return Some(Err(message)),
                        };
                    }

                    return Some(Ok(accumulator));
                }

                let mut accumulator = initial;
                for element in array.iter() {
                    accumulator = match self
                        .call_function_from_jit(func.clone(), vec![accumulator, element.clone()])
                    {
                        Ok(value) => value,
                        Err(message) => return Some(Err(message)),
                    };
                }

                Some(Ok(accumulator))
            }
            "find" => {
                if args.len() < 2 {
                    return Some(
                        Err("find requires two arguments: array and function".to_string()),
                    );
                }

                let (array, func) = match (args.first(), args.get(1)) {
                    (Some(Value::Array(arr)), Some(func @ Value::BytecodeFunction { .. })) => {
                        (arr.clone(), func.clone())
                    }
                    _ => return None,
                };

                for element in array.iter() {
                    let func_result =
                        match self.call_function_from_jit(func.clone(), vec![element.clone()]) {
                            Ok(value) => value,
                            Err(message) => return Some(Err(message)),
                        };

                    if self.is_truthy(&func_result) {
                        return Some(Ok(element.clone()));
                    }
                }

                Some(Ok(Value::Int(0)))
            }
            "any" => {
                if args.len() < 2 {
                    return Some(Err("any requires two arguments: array and function".to_string()));
                }

                let (array, func) = match (args.first(), args.get(1)) {
                    (Some(Value::Array(arr)), Some(func @ Value::BytecodeFunction { .. })) => {
                        (arr.clone(), func.clone())
                    }
                    _ => return None,
                };

                for element in array.iter() {
                    let func_result =
                        match self.call_function_from_jit(func.clone(), vec![element.clone()]) {
                            Ok(value) => value,
                            Err(message) => return Some(Err(message)),
                        };

                    if self.is_truthy(&func_result) {
                        return Some(Ok(Value::Bool(true)));
                    }
                }

                Some(Ok(Value::Bool(false)))
            }
            "all" => {
                if args.len() < 2 {
                    return Some(Err("all requires two arguments: array and function".to_string()));
                }

                let (array, func) = match (args.first(), args.get(1)) {
                    (Some(Value::Array(arr)), Some(func @ Value::BytecodeFunction { .. })) => {
                        (arr.clone(), func.clone())
                    }
                    _ => return None,
                };

                for element in array.iter() {
                    let func_result =
                        match self.call_function_from_jit(func.clone(), vec![element.clone()]) {
                            Ok(value) => value,
                            Err(message) => return Some(Err(message)),
                        };

                    if !self.is_truthy(&func_result) {
                        return Some(Ok(Value::Bool(false)));
                    }
                }

                Some(Ok(Value::Bool(true)))
            }
            _ => None,
        }
    }

    /// Detect a simple binary reducer of the form `return a <op> b`.
    /// Returns (operator, swap_operands) when it matches.
    fn match_simple_binary_reduce(&self, func: &Value) -> Option<(&'static str, bool)> {
        let (chunk, captured) = match func {
            Value::BytecodeFunction { chunk, captured, captured_binding_kinds: _ } => {
                (chunk, captured)
            }
            _ => return None,
        };

        if !captured.is_empty() || chunk.params.len() != 2 {
            return None;
        }

        let param0 = &chunk.params[0];
        let param1 = &chunk.params[1];
        let instructions = &chunk.instructions;

        if instructions.len() != 4 {
            return None;
        }

        let op = match instructions[2] {
            OpCode::Add => "+",
            OpCode::Sub => "-",
            OpCode::Mul => "*",
            OpCode::Div => "/",
            OpCode::Mod => "%",
            _ => return None,
        };

        match (&instructions[0], &instructions[1], &instructions[3]) {
            (OpCode::LoadVar(a), OpCode::LoadVar(b), OpCode::Return)
                if a == param0 && b == param1 =>
            {
                Some((op, false))
            }
            (OpCode::LoadVar(a), OpCode::LoadVar(b), OpCode::Return)
                if a == param1 && b == param0 =>
            {
                Some((op, true))
            }
            _ => None,
        }
    }

    /// Call a function from JIT-compiled code
    /// This is invoked by the jit_call_function runtime helper
    pub fn call_function_from_jit(
        &mut self,
        function: Value,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        match &function {
            Value::BytecodeFunction { chunk, captured: _, captured_binding_kinds: _ } => {
                // OPTIMIZATION: Check if target function is JIT-compiled
                // If so, make direct JIT → JIT call for maximum performance
                if self.jit_enabled {
                    let func_name = chunk.name.as_deref().unwrap_or("<anonymous>");

                    // PHASE 7 STEP 12: Check for direct-arg optimized variant first
                    // This enables direct JIT recursion without FFI boundary crossing
                    if args.len() == 1 {
                        if let Value::Int(arg_val) = args[0] {
                            // Copy the function info to avoid borrow checker issues
                            let fn_info_opt = self.compiled_fn_info.get(func_name).copied();

                            if let Some(fn_info) = fn_info_opt {
                                if let Some(direct_fn) = fn_info.fn_with_arg {
                                    // ULTRA-FAST PATH: Call the direct-arg variant!
                                    // This is the key optimization for recursive functions

                                    // Get VM pointer for VMContext
                                    let vm_ptr: *mut std::ffi::c_void =
                                        self as *mut _ as *mut std::ffi::c_void;
                                    let stack_ptr: *mut Vec<Value> = &mut self.stack;

                                    // Get globals pointer
                                    let globals_ptr: *mut HashMap<String, Value> = {
                                        let mut globals_guard = self.globals.lock().unwrap();
                                        let ptr = &mut globals_guard.scopes[0]
                                            as *mut HashMap<String, Value>;
                                        drop(globals_guard);
                                        ptr
                                    };

                                    // Create minimal VMContext - direct-arg functions don't need HashMap
                                    let mut func_locals = HashMap::new();
                                    let locals_ptr: *mut HashMap<String, Value> = &mut func_locals;
                                    let local_slots_ptr: *mut Vec<Value> = match self
                                        .call_frames
                                        .last_mut()
                                    {
                                        Some(frame) => &mut frame.local_slots as *mut Vec<Value>,
                                        None => std::ptr::null_mut(),
                                    };

                                    let mut vm_context = crate::jit::VMContext {
                                        stack_ptr,
                                        locals_ptr,
                                        globals_ptr,
                                        var_names_ptr: std::ptr::null_mut(),
                                        local_slots_ptr,
                                        obj_stack_ptr: &mut self.jit_obj_stack as *mut Vec<Value>,
                                        vm_ptr,
                                        return_value: 0,
                                        has_return_value: false,
                                        arg0: arg_val,
                                        arg1: 0,
                                        arg2: 0,
                                        arg3: 0,
                                        arg_count: 1,
                                    };

                                    // Execute the direct-arg variant!
                                    // The function returns the actual result (not a status code)
                                    let result = invoke_compiled_fn_with_arg(
                                        direct_fn,
                                        &mut vm_context,
                                        arg_val,
                                    );

                                    if std::env::var("DEBUG_JIT").is_ok() {
                                        eprintln!(
                                            "JIT: Direct-arg call to '{}' with arg {} returned {}",
                                            func_name, arg_val, result
                                        );
                                    }

                                    return Ok(Value::Int(result));
                                }
                            }
                        }
                    }

                    // Copy the function pointer to avoid borrow checker issues
                    let compiled_fn_opt = self.compiled_functions.get(func_name).copied();

                    if let Some(compiled_fn) = compiled_fn_opt {
                        // Fast path: Direct JIT → JIT call!

                        // OPTIMIZATION: For simple integer-only functions with ≤4 args,
                        // pass arguments directly via VMContext fields instead of HashMap
                        let has_loop =
                            chunk.instructions.iter().any(|op| matches!(op, OpCode::JumpBack(_)));
                        let use_fast_args = !has_loop
                            && args.len() <= 4
                            && args.iter().all(|a| matches!(a, Value::Int(_)));

                        // ULTRA-FAST PATH: Skip HashMap entirely for simple integer functions
                        // The JIT uses jit_get_arg to read parameters directly from VMContext
                        // We only need the HashMap for functions with non-integer args or >4 args
                        let mut func_locals: HashMap<String, Value>;
                        let use_empty_locals = use_fast_args && chunk.params.len() == args.len();

                        if use_empty_locals {
                            // Ultra-fast: Use empty HashMap - JIT will use VMContext.argN
                            func_locals = HashMap::new();
                        } else {
                            // Normal path: Create locals HashMap for the function parameters
                            func_locals = HashMap::new();

                            // Bind arguments to parameter names
                            for (i, param_name) in chunk.params.iter().enumerate() {
                                if let Some(arg) = args.get(i) {
                                    func_locals.insert(param_name.clone(), arg.clone());
                                }
                            }
                        }

                        // Get or create var_names from cache (avoids re-hashing on every call)
                        let func_name_owned = func_name.to_string();

                        // OPTIMIZATION: Get reference to cached var_names instead of cloning
                        // This avoids HashMap clone on every recursive call
                        let cached_var_names_exists =
                            self.jit_var_names_cache.contains_key(&func_name_owned);
                        if !cached_var_names_exists {
                            // Build var_names once and cache it
                            let mut cached_var_names = HashMap::new();

                            // Register parameter names
                            for param_name in &chunk.params {
                                use std::collections::hash_map::DefaultHasher;
                                use std::hash::{Hash, Hasher};
                                let mut hasher = DefaultHasher::new();
                                param_name.hash(&mut hasher);
                                let hash = hasher.finish();
                                cached_var_names.insert(hash, param_name.clone());
                            }

                            // Register all LoadVar names
                            for instr in &chunk.instructions {
                                if let OpCode::LoadVar(name) = instr {
                                    use std::collections::hash_map::DefaultHasher;
                                    use std::hash::{Hash, Hasher};
                                    let mut hasher = DefaultHasher::new();
                                    name.hash(&mut hasher);
                                    let hash = hasher.finish();
                                    cached_var_names.insert(hash, name.clone());
                                }
                            }

                            self.jit_var_names_cache
                                .insert(func_name_owned.clone(), cached_var_names);
                        }

                        // Get pointer to cached var_names (no clone!)
                        let var_names_ptr: *mut HashMap<u64, String> = self
                            .jit_var_names_cache
                            .get_mut(&func_name_owned)
                            .map(|v| v as *mut HashMap<u64, String>)
                            .unwrap_or(std::ptr::null_mut());

                        let vm_ptr: *mut std::ffi::c_void = self as *mut _ as *mut std::ffi::c_void;

                        // Save stack size to detect return value
                        let stack_size_before = self.stack.len();

                        // Get mutable pointers to VM state for VMContext
                        let stack_ptr: *mut Vec<Value> = &mut self.stack;

                        // Get globals - we need to drop the lock before executing
                        // to avoid deadlock on recursive calls. Since JIT execution
                        // is single-threaded, we can safely use a raw pointer.
                        let globals_ptr: *mut HashMap<String, Value> = {
                            let mut globals_guard = self.globals.lock().unwrap();
                            let ptr = &mut globals_guard.scopes[0] as *mut HashMap<String, Value>;
                            // Explicitly drop to release lock before JIT execution
                            drop(globals_guard);
                            ptr
                        };

                        let locals_ptr: *mut HashMap<String, Value> = &mut func_locals;
                        let local_slots_ptr: *mut Vec<Value> = match self.call_frames.last_mut() {
                            Some(frame) => &mut frame.local_slots as *mut Vec<Value>,
                            None => std::ptr::null_mut(),
                        };

                        // Create VMContext with fast argument fields
                        let mut vm_context = crate::jit::VMContext {
                            stack_ptr,
                            locals_ptr,
                            globals_ptr,
                            var_names_ptr,
                            local_slots_ptr,
                            obj_stack_ptr: &mut self.jit_obj_stack as *mut Vec<Value>,
                            vm_ptr,
                            return_value: 0,
                            has_return_value: false,
                            arg0: if use_fast_args && args.len() > 0 {
                                if let Value::Int(n) = args[0] {
                                    n
                                } else {
                                    0
                                }
                            } else {
                                0
                            },
                            arg1: if use_fast_args && args.len() > 1 {
                                if let Value::Int(n) = args[1] {
                                    n
                                } else {
                                    0
                                }
                            } else {
                                0
                            },
                            arg2: if use_fast_args && args.len() > 2 {
                                if let Value::Int(n) = args[2] {
                                    n
                                } else {
                                    0
                                }
                            } else {
                                0
                            },
                            arg3: if use_fast_args && args.len() > 3 {
                                if let Value::Int(n) = args[3] {
                                    n
                                } else {
                                    0
                                }
                            } else {
                                0
                            },
                            arg_count: args.len() as i64,
                        };

                        // Execute the compiled function!
                        // Lock is NOT held during execution to allow recursive calls
                        let result_code = invoke_compiled_fn(compiled_fn, &mut vm_context);

                        if result_code != 0 {
                            return Err(format!("JIT execution failed with code: {}", result_code));
                        }

                        // Check for return value - prefer optimized VMContext.return_value
                        // This is the FAST PATH from Phase 7 Step 8 optimization
                        if vm_context.has_return_value {
                            // Use the optimized return value directly
                            return Ok(Value::Int(vm_context.return_value));
                        } else if self.stack.len() > stack_size_before {
                            // Fallback: return value was pushed to stack (old path)
                            let result = self.stack.pop().unwrap();
                            return Ok(result);
                        } else {
                            return Err("JIT-compiled function did not return a value".to_string());
                        }
                    }
                }

                // Slow path: Execute through interpreter
                // Save current execution state
                let saved_ip = self.ip;
                let saved_chunk = self.chunk.clone();
                let call_frame_depth = self.call_frames.len();

                // Set up the call (creates call frame, switches chunk, resets IP)
                let prepared_args = self.prepare_bytecode_call_args(chunk, args.clone())?;
                self.call_bytecode_function(function, args, prepared_args)?;

                // Execute until this function returns
                // (call_frames will pop back to call_frame_depth)
                while self.call_frames.len() > call_frame_depth {
                    // Check bounds
                    if self.ip >= self.chunk.instructions.len() {
                        return Err("Function execution reached end without return".to_string());
                    }

                    // Get instruction (clone to avoid borrow checker issues)
                    let instruction = self.chunk.instructions[self.ip].clone();
                    self.ip += 1;

                    // Execute the instruction
                    // We need to handle the most common opcodes inline
                    // For complex ones, we could call back to the main run loop
                    match instruction {
                        OpCode::LoadConst(idx) => {
                            let constant = &self.chunk.constants[idx];
                            let value = self.constant_to_value(constant)?;
                            self.stack.push(value);
                        }

                        OpCode::LoadLocal(slot) => {
                            let frame =
                                self.call_frames.last().ok_or("LoadLocal requires call frame")?;
                            let value = frame
                                .local_slots
                                .get(slot)
                                .cloned()
                                .ok_or_else(|| format!("Invalid local slot: {}", slot))?;
                            self.stack.push(value);
                        }

                        OpCode::LoadVar(name) => {
                            if let Some(frame) = self.call_frames.last() {
                                if let Some(value) = frame.locals.get(&name) {
                                    self.stack.push(value.clone());
                                } else if let Some(value_ref) = frame.captured.get(&name) {
                                    let value = value_ref.lock().unwrap().clone();
                                    self.stack.push(value);
                                } else if let Some(value) = self.globals.lock().unwrap().get(&name)
                                {
                                    self.stack.push(value.clone());
                                } else {
                                    return Err(Self::undefined_variable_message(&name));
                                }
                            } else {
                                return Err("No call frame for LoadVar".to_string());
                            }
                        }

                        OpCode::LoadGlobal(name) => {
                            let value = self
                                .globals
                                .lock()
                                .unwrap()
                                .get(&name)
                                .ok_or_else(|| Self::undefined_variable_message(&name))?;
                            self.stack.push(value);
                        }

                        OpCode::StoreVar(name) => {
                            let value = self.stack.pop().ok_or("Stack underflow")?;
                            let global_exists = self.globals.lock().unwrap().get(&name).is_some();
                            let mut assign_global = false;

                            if let Some(frame) = self.call_frames.last_mut() {
                                if let Some(captured_ref) = frame.captured.get(&name) {
                                    if let Some(kind) =
                                        frame.captured_binding_kinds.get(&name).copied()
                                    {
                                        if !matches!(kind, BytecodeBindingKind::Mutable) {
                                            return Err(Self::local_reassignment_error(
                                                kind, &name,
                                            ));
                                        }
                                    }
                                    *captured_ref.lock().unwrap() = value.clone();
                                } else if frame.locals.contains_key(&name) {
                                    if let Some(kind) =
                                        frame.locals_binding_kinds.get(&name).copied()
                                    {
                                        if !matches!(kind, BytecodeBindingKind::Mutable) {
                                            return Err(Self::local_reassignment_error(
                                                kind, &name,
                                            ));
                                        }
                                    }
                                    frame.locals.insert(name.clone(), value.clone());
                                } else if global_exists {
                                    assign_global = true;
                                } else {
                                    frame
                                        .locals_binding_kinds
                                        .entry(name.clone())
                                        .or_insert(BytecodeBindingKind::Mutable);
                                    frame.locals.insert(name.clone(), value.clone());
                                }
                            } else {
                                assign_global = true;
                            }

                            if assign_global {
                                self.globals.lock().unwrap().assign_checked(name, value)?;
                            }
                        }

                        OpCode::StoreLocal(slot) => {
                            let value = self.stack.pop().ok_or("Stack underflow")?;
                            let binding_name = self
                                .chunk
                                .local_names
                                .get(slot)
                                .cloned()
                                .unwrap_or_else(|| format!("<local:{}>", slot));
                            if let Some(frame) = self.call_frames.last_mut() {
                                if slot >= frame.local_slots.len() {
                                    frame.local_slots.resize(slot + 1, Value::Null);
                                    frame
                                        .local_slot_binding_kinds
                                        .resize(slot + 1, BytecodeBindingKind::Mutable);
                                    frame.local_slot_initialized.resize(slot + 1, false);
                                }
                                let kind = frame
                                    .local_slot_binding_kinds
                                    .get(slot)
                                    .copied()
                                    .unwrap_or(BytecodeBindingKind::Mutable);
                                let initialized = frame
                                    .local_slot_initialized
                                    .get(slot)
                                    .copied()
                                    .unwrap_or(false);
                                if initialized && !matches!(kind, BytecodeBindingKind::Mutable) {
                                    return Err(Self::local_reassignment_error(
                                        kind,
                                        &binding_name,
                                    ));
                                }
                                frame.local_slots[slot] = value;
                                frame.local_slot_initialized[slot] = true;
                            }
                        }

                        OpCode::StoreGlobal(name) => {
                            let value = self.stack.pop().ok_or("Stack underflow")?;
                            self.globals.lock().unwrap().assign_checked(name, value)?;
                        }

                        OpCode::DefineGlobal(name, kind) => {
                            let value = self.stack.pop().ok_or("Stack underflow")?;
                            self.globals.lock().unwrap().define_with_kind_checked(
                                name,
                                value,
                                Self::env_binding_kind(kind),
                            )?;
                        }

                        OpCode::EnsureMutableGlobalForMutation(name) => {
                            self.globals
                                .lock()
                                .unwrap()
                                .ensure_mutable_for_mutation(name.as_str())?;
                        }

                        OpCode::Pop => {
                            self.stack.pop().ok_or("Stack underflow")?;
                        }

                        OpCode::Dup => {
                            let value = self.stack.last().ok_or("Stack underflow")?.clone();
                            self.stack.push(value);
                        }

                        OpCode::Add => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result = self.binary_op(&left, "+", &right)?;
                            self.stack.push(result);
                        }

                        OpCode::Sub => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result = self.binary_op(&left, "-", &right)?;
                            self.stack.push(result);
                        }

                        OpCode::Mul => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result = self.binary_op(&left, "*", &right)?;
                            self.stack.push(result);
                        }

                        OpCode::Div => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result = self.binary_op(&left, "/", &right)?;
                            self.stack.push(result);
                        }

                        OpCode::Mod => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result = self.binary_op(&left, "%", &right)?;
                            self.stack.push(result);
                        }

                        OpCode::Negate => {
                            let value = self.stack.pop().ok_or("Stack underflow")?;
                            let result = self.unary_op("-", &value)?;
                            self.stack.push(result);
                        }

                        OpCode::Not => {
                            let value = self.stack.pop().ok_or("Stack underflow")?;
                            let result = self.unary_op("!", &value)?;
                            self.stack.push(result);
                        }

                        OpCode::And => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result =
                                Value::Bool(self.is_truthy(&left) && self.is_truthy(&right));
                            self.stack.push(result);
                        }

                        OpCode::Or => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result =
                                Value::Bool(self.is_truthy(&left) || self.is_truthy(&right));
                            self.stack.push(result);
                        }

                        OpCode::LessThan => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result = self.compare_op(&left, "<", &right)?;
                            self.stack.push(result);
                        }

                        OpCode::GreaterThan => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result = self.compare_op(&left, ">", &right)?;
                            self.stack.push(result);
                        }

                        OpCode::LessEqual => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result = self.compare_op(&left, "<=", &right)?;
                            self.stack.push(result);
                        }

                        OpCode::GreaterEqual => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result = self.compare_op(&left, ">=", &right)?;
                            self.stack.push(result);
                        }

                        OpCode::Equal => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result = Value::Bool(self.values_equal(&left, &right));
                            self.stack.push(result);
                        }

                        OpCode::NotEqual => {
                            let right = self.stack.pop().ok_or("Stack underflow")?;
                            let left = self.stack.pop().ok_or("Stack underflow")?;
                            let result = Value::Bool(!self.values_equal(&left, &right));
                            self.stack.push(result);
                        }

                        OpCode::Return => {
                            let return_value =
                                self.stack.pop().ok_or("Stack underflow in return")?;

                            if let Some(frame) = self.call_frames.pop() {
                                // Pop from function call stack
                                self.function_call_stack.pop();

                                // Restore saved state
                                self.ip = saved_ip;
                                self.set_chunk(saved_chunk);

                                // Clear stack to frame offset
                                self.stack.truncate(frame.stack_offset);

                                // Return the value
                                return Ok(return_value);
                            } else {
                                return Ok(return_value);
                            }
                        }

                        OpCode::ReturnNone => {
                            if let Some(frame) = self.call_frames.pop() {
                                self.function_call_stack.pop();
                                self.ip = saved_ip;
                                self.set_chunk(saved_chunk);
                                self.stack.truncate(frame.stack_offset);
                                return Ok(Value::Null);
                            } else {
                                return Ok(Value::Null);
                            }
                        }

                        OpCode::Call(arg_count) => {
                            // Nested function call from within JIT-called function
                            // We can handle this recursively
                            let func = self.stack.pop().ok_or("Stack underflow")?;
                            let mut args = Vec::new();
                            for _ in 0..arg_count {
                                args.push(self.stack.pop().ok_or("Stack underflow")?);
                            }
                            args.reverse();

                            // Recursive call
                            let result = self.call_function_from_jit(func, args)?;
                            self.stack.push(result);
                        }

                        OpCode::CallNative(name, arg_count) => {
                            let mut args = Vec::new();
                            for _ in 0..arg_count {
                                args.push(self.stack.pop().ok_or("Stack underflow")?);
                            }
                            args.reverse();

                            let result = match name.as_str() {
                                "__vm_import_all" => self.vm_import_all(&args),
                                "__vm_import_symbol" => self.vm_import_symbol(&args),
                                _ => {
                                    let native_result =
                                        self.interpreter.call_native_function_impl(&name, &args);
                                    match native_result {
                                        Value::Error(msg) => Err(msg),
                                        Value::ErrorObject { message, .. } => Err(message),
                                        other => Ok(other),
                                    }
                                }
                            }?;

                            self.stack.push(result);
                        }

                        OpCode::FieldGet(field) => {
                            let object = self.stack.pop().ok_or("Stack underflow")?;
                            let result = match &object {
                                Value::Struct { name, fields } => {
                                    if let Some(value) = fields.get(&field) {
                                        value.clone()
                                    } else {
                                        let method_name = format!("{}.{}", name, field);
                                        self.globals
                                            .lock()
                                            .unwrap()
                                            .get(&method_name)
                                            .ok_or_else(|| format!("Field not found: {}", field))?
                                    }
                                }
                                Value::Dict(dict) => {
                                    dict.get(field.as_str()).cloned().unwrap_or(Value::Null)
                                }
                                Value::FixedDict { keys, values } => {
                                    let idx =
                                        keys.iter().position(|k| k.as_ref() == field.as_str());
                                    idx.and_then(|i| values.get(i).cloned()).unwrap_or(Value::Null)
                                }
                                Value::IntDict(dict) => match field.parse::<i64>() {
                                    Ok(int_key) => {
                                        dict.get(&int_key).cloned().unwrap_or(Value::Null)
                                    }
                                    Err(_) => Value::Null,
                                },
                                Value::DenseIntDict(values) => match field.parse::<i64>() {
                                    Ok(int_key) if int_key >= 0 => {
                                        values.get(int_key as usize).cloned().unwrap_or(Value::Null)
                                    }
                                    _ => Value::Null,
                                },
                                _ => {
                                    return Err(format!(
                                        "Cannot access field or method '{}' on non-struct value",
                                        field
                                    ))
                                }
                            };
                            self.stack.push(result);
                        }

                        OpCode::MakeStruct(name, field_names) => {
                            let mut fields = HashMap::new();
                            for field_name in field_names.iter().rev() {
                                let value = self.stack.pop().ok_or("Stack underflow")?;
                                fields.insert(field_name.clone(), value);
                            }
                            if !self.push_checked_value(
                                Value::Struct { name, fields },
                                "building struct value",
                            )? {
                                continue;
                            }
                        }

                        OpCode::Jump(target) => {
                            self.ip = target;
                        }

                        OpCode::JumpIfFalse(target) => {
                            let condition = self.stack.last().ok_or("Stack underflow")?;
                            if !self.is_truthy(condition) {
                                self.ip = target;
                            }
                        }

                        OpCode::JumpIfTrue(target) => {
                            let condition = self.stack.last().ok_or("Stack underflow")?;
                            if self.is_truthy(condition) {
                                self.ip = target;
                            }
                        }

                        OpCode::JumpBack(target) => {
                            self.ip = target;
                        }

                        OpCode::IndexGetInPlace(slot) => {
                            let index = self.stack.pop().ok_or("Stack underflow")?;
                            let frame = self
                                .call_frames
                                .last()
                                .ok_or("IndexGetInPlace requires call frame")?;
                            let object = frame
                                .local_slots
                                .get(slot)
                                .cloned()
                                .ok_or_else(|| format!("Invalid local slot: {}", slot))?;
                            let result = Self::get_indexed_value(&object, &index)?;

                            self.stack.push(result);
                        }

                        _ => {
                            // For now, unsupported opcodes in nested calls
                            return Err(format!(
                                "Unsupported opcode in JIT function call: {:?}",
                                instruction
                            ));
                        }
                    }
                }

                // Should not reach here - function should have returned
                Err("Function execution completed without explicit return".to_string())
            }
            Value::NativeFunction(_) => {
                // Native function - call it directly
                self.call_native_function_vm(function, args)
            }
            Value::Function(..) | Value::GeneratorDef(..) => {
                self.call_interpreter_callable(&function, &args)
            }
            _ => Err(Self::non_callable_error_message("the value being called is not callable")),
        }
    }

    /// Call a bytecode function from interpreter context while preserving VM state
    pub fn call_bytecode_function_from_interpreter(
        &mut self,
        function: Value,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let saved_ip = self.ip;
        let saved_chunk = self.chunk.clone();
        let saved_stack = std::mem::take(&mut self.stack);
        let saved_call_frames = std::mem::take(&mut self.call_frames);
        let saved_exception_handlers = std::mem::take(&mut self.exception_handlers);
        let saved_function_call_stack = std::mem::take(&mut self.function_call_stack);
        let saved_recursion_depth = self.recursion_depth;
        let saved_max_recursion_depth = self.max_recursion_depth;

        let result = self.call_function_from_jit(function, args);

        self.ip = saved_ip;
        self.set_chunk(saved_chunk);
        self.stack = saved_stack;
        self.call_frames = saved_call_frames;
        self.exception_handlers = saved_exception_handlers;
        self.function_call_stack = saved_function_call_stack;
        self.recursion_depth = saved_recursion_depth;
        self.max_recursion_depth = saved_max_recursion_depth;

        result
    }

    /// Convert a value to string representation for printing
    fn value_to_string(value: &Value) -> String {
        match value {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Str(s) => s.as_ref().clone(),
            Value::Secret(_) => "***".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(Self::value_to_string).collect();
                format!("[{}]", items.join(", "))
            }
            Value::FixedDict { keys, values } => {
                let mut pairs: Vec<(&Arc<str>, &Value)> = keys.iter().zip(values.iter()).collect();
                pairs.sort_by(|(a, _), (b, _)| a.as_ref().cmp(b.as_ref()));
                let items: Vec<String> = pairs
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, Self::value_to_string(v)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Dict(dict) => {
                let mut keys: Vec<&Arc<str>> = dict.keys().collect();
                keys.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
                let items: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        format!("{}: {}", k, Self::value_to_string(dict.get(k.as_ref()).unwrap()))
                    })
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::IntDict(dict) => {
                let mut keys: Vec<i64> = dict.keys().cloned().collect();
                keys.sort();
                let items: Vec<String> = keys
                    .iter()
                    .map(|k| format!("{}: {}", k, Self::value_to_string(dict.get(k).unwrap())))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::DenseIntDict(values) => {
                let items: Vec<String> = values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| format!("{}: {}", index, Self::value_to_string(value)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            _ => format!("{:?}", value),
        }
    }

    fn value_type_name(value: &Value) -> &'static str {
        match value {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Secret(_) => "secret",
            Value::Array(_) => "array",
            Value::Dict(_) => "dict",
            Value::Struct { .. } => "struct",
            Value::Function(..) => "function",
            Value::NativeFunction(_) => "native_function",
            Value::Null => "null",
            Value::Error(_) | Value::ErrorObject { .. } => "error",
            _ => "value",
        }
    }

    fn invalid_binary_operation(op: &str, left: &Value, right: &Value) -> String {
        format!(
            "Invalid binary operation: {} {} {}",
            Self::value_type_name(left),
            op,
            Self::value_type_name(right)
        )
    }

    fn try_call_vm_binary_operator_method(
        &mut self,
        left: &Value,
        op: &str,
        right: &Value,
    ) -> Option<Result<Value, String>> {
        let method_name = crate::ast::operator_methods::binary_op_method(op)?;
        let struct_name = match left {
            Value::Struct { name, .. } => name,
            _ => return None,
        };

        let method_global_name = format!("{}.{}", struct_name, method_name);
        let method_value = self.globals.lock().unwrap().get(&method_global_name)?;

        Some(self.call_function_from_jit(method_value, vec![left.clone(), right.clone()]))
    }

    fn try_call_vm_unary_operator_method(
        &mut self,
        value: &Value,
        op: &str,
    ) -> Option<Result<Value, String>> {
        let method_name = crate::ast::operator_methods::unary_op_method(op)?;
        let struct_name = match value {
            Value::Struct { name, .. } => name,
            _ => return None,
        };

        let method_global_name = format!("{}.{}", struct_name, method_name);
        let method_value = self.globals.lock().unwrap().get(&method_global_name)?;
        Some(self.call_function_from_jit(method_value, vec![value.clone()]))
    }

    /// Binary operation
    fn binary_op(&mut self, left: &Value, op: &str, right: &Value) -> Result<Value, String> {
        if let Some(result) = self.try_call_vm_binary_operator_method(left, op, right) {
            return result;
        }

        match (left, right) {
            (Value::Int(a), Value::Int(b)) => match op {
                "+" | "-" | "*" | "/" | "%" => {
                    Value::checked_int_arithmetic(*a, op, *b).map(Value::Int)
                }
                _ => Err(Self::invalid_binary_operation(op, left, right)),
            },
            (Value::Float(a), Value::Float(b)) => match op {
                "+" | "-" | "*" | "/" | "%" => {
                    Value::checked_float_arithmetic(*a, op, *b).map(Value::Float)
                }
                _ => Err(Self::invalid_binary_operation(op, left, right)),
            },
            (Value::Int(a), Value::Float(b)) => match op {
                "+" | "-" | "*" | "/" | "%" => {
                    Value::checked_float_arithmetic(*a as f64, op, *b).map(Value::Float)
                }
                _ => Err(Self::invalid_binary_operation(op, left, right)),
            },
            (Value::Float(a), Value::Int(b)) => match op {
                "+" | "-" | "*" | "/" | "%" => {
                    Value::checked_float_arithmetic(*a, op, *b as f64).map(Value::Float)
                }
                _ => Err(Self::invalid_binary_operation(op, left, right)),
            },
            (Value::Str(a), Value::Str(b)) if op == "+" => {
                let mut result = a.clone();
                let result_str = Arc::make_mut(&mut result);
                result_str.push_str(b.as_ref());
                Ok(Value::Str(result))
            }
            _ => Err(Self::invalid_binary_operation(op, left, right)),
        }
    }

    /// Unary operation
    fn unary_op(&mut self, op: &str, value: &Value) -> Result<Value, String> {
        if let Some(result) = self.try_call_vm_unary_operator_method(value, op) {
            return result;
        }

        match (op, value) {
            ("-", Value::Int(n)) => {
                n.checked_neg().map(Value::Int).ok_or_else(|| format!("Integer overflow: -({})", n))
            }
            ("-", Value::Float(f)) => Ok(Value::Float(-f)),
            ("!", Value::Bool(b)) => Ok(Value::Bool(!b)),
            _ => Err(format!("Invalid unary operation: {} {:?}", op, value)),
        }
    }

    /// Comparison operation
    fn compare_op(&self, left: &Value, op: &str, right: &Value) -> Result<Value, String> {
        Value::compare_order(left, op, right).map(Value::Bool)
    }

    /// Check if value is truthy
    fn is_truthy(&self, value: &Value) -> bool {
        value.is_truthy()
    }

    /// Check if two values are equal
    fn values_equal(&self, left: &Value, right: &Value) -> bool {
        Value::equals(left, right)
    }

    /// Match a pattern against a value
    fn match_pattern(
        &mut self,
        pattern: &Pattern,
        value: &Value,
        binding_kind: BytecodeBindingKind,
    ) -> Result<bool, String> {
        match pattern {
            Pattern::Identifier(name) => {
                self.bind_pattern_name(name, value.clone(), binding_kind);
                Ok(true)
            }

            Pattern::Ignore => Ok(true),

            Pattern::Array { elements, rest } => {
                if let Value::Array(arr) = value {
                    if rest.is_none() && arr.len() != elements.len() {
                        return Ok(false);
                    }

                    if arr.len() < elements.len() {
                        return Ok(false);
                    }

                    for (index, element_pattern) in elements.iter().enumerate() {
                        if !self.match_pattern(element_pattern, &arr[index], binding_kind)? {
                            return Ok(false);
                        }
                    }

                    if let Some(rest_name) = rest {
                        let rest_values = arr[elements.len()..].to_vec();
                        self.bind_pattern_name(
                            rest_name,
                            Value::Array(std::sync::Arc::new(rest_values)),
                            binding_kind,
                        );
                    }

                    Ok(true)
                } else {
                    Ok(false)
                }
            }

            Pattern::Dict { keys, rest } => match value {
                Value::Dict(dict) => {
                    for key in keys {
                        let Some(dict_value) = dict.get(key.as_str()) else {
                            return Ok(false);
                        };
                        self.bind_pattern_name(key, dict_value.clone(), binding_kind);
                    }

                    if let Some(rest_name) = rest {
                        let mut rest_dict = DictMap::default();
                        for (key, dict_value) in dict.iter() {
                            if !keys.iter().any(|existing| existing.as_str() == key.as_ref()) {
                                rest_dict.insert(key.clone(), dict_value.clone());
                            }
                        }
                        self.bind_pattern_name(
                            rest_name,
                            Value::Dict(std::sync::Arc::new(rest_dict)),
                            binding_kind,
                        );
                    }

                    Ok(true)
                }
                Value::FixedDict { keys: dict_keys, values } => {
                    for key in keys {
                        let key_index =
                            dict_keys.iter().position(|dict_key| dict_key.as_ref() == key.as_str());
                        let Some(index) = key_index else {
                            return Ok(false);
                        };
                        self.bind_pattern_name(key, values[index].clone(), binding_kind);
                    }

                    if let Some(rest_name) = rest {
                        let mut rest_dict = DictMap::default();
                        for (dict_key, dict_value) in dict_keys.iter().zip(values.iter()) {
                            if !keys.iter().any(|existing| existing.as_str() == dict_key.as_ref()) {
                                rest_dict.insert(dict_key.clone(), dict_value.clone());
                            }
                        }
                        self.bind_pattern_name(
                            rest_name,
                            Value::Dict(std::sync::Arc::new(rest_dict)),
                            binding_kind,
                        );
                    }

                    Ok(true)
                }
                _ => Ok(false),
            },
        }
    }

    fn match_case_pattern(&mut self, pattern: &str, value: &Value) -> bool {
        let (case_tag, binding_name) = if let Some(open_paren) = pattern.find('(') {
            if pattern.ends_with(')') {
                let tag = pattern[..open_paren].trim();
                let binding = pattern[open_paren + 1..pattern.len() - 1].trim();
                if !binding.is_empty() {
                    (tag.to_string(), Some(binding.to_string()))
                } else {
                    (tag.to_string(), None)
                }
            } else {
                (pattern.trim().to_string(), None)
            }
        } else {
            (pattern.trim().to_string(), None)
        };

        match value {
            Value::Result { is_ok, value } => {
                let short_tag = if *is_ok { "Ok" } else { "Err" };
                let full_tag = format!("Result::{}", short_tag);
                if case_tag == short_tag || case_tag == full_tag {
                    if let Some(name) = binding_name {
                        self.bind_pattern_name(
                            name.as_str(),
                            (**value).clone(),
                            BytecodeBindingKind::Mutable,
                        );
                    }
                    true
                } else {
                    false
                }
            }
            Value::Option { is_some, value } => {
                let short_tag = if *is_some { "Some" } else { "None" };
                let full_tag = format!("Option::{}", short_tag);
                if case_tag == short_tag || case_tag == full_tag {
                    if let Some(name) = binding_name {
                        if *is_some {
                            self.bind_pattern_name(
                                name.as_str(),
                                (**value).clone(),
                                BytecodeBindingKind::Mutable,
                            );
                        }
                    }
                    true
                } else {
                    false
                }
            }
            Value::Tagged { tag, fields } => {
                if case_tag == *tag {
                    if let Some(name) = binding_name {
                        if let Some(bound) = fields.get("$0") {
                            self.bind_pattern_name(
                                name.as_str(),
                                bound.clone(),
                                BytecodeBindingKind::Mutable,
                            );
                        }
                    }
                    true
                } else {
                    false
                }
            }
            Value::Enum(enum_name) => case_tag == *enum_name,
            Value::Str(s) => case_tag == s.as_ref().as_str(),
            Value::Float(n) => case_tag == n.to_string(),
            _ => false,
        }
    }

    fn bind_pattern_name(&mut self, name: &str, value: Value, binding_kind: BytecodeBindingKind) {
        if self.call_frames.len() <= 1 {
            self.globals.lock().unwrap().define_with_kind(
                name.to_string(),
                value.clone(),
                Self::env_binding_kind(binding_kind),
            );
        }

        if let Some(frame) = self.call_frames.last_mut() {
            frame.locals_binding_kinds.insert(name.to_string(), binding_kind);
            frame.locals.insert(name.to_string(), value);
        }
    }

    /// Execute generator until next yield or completion
    /// Returns Some(value) if yielded, None if exhausted
    pub fn generator_next(&mut self, generator: Value) -> Result<Value, String> {
        if let Value::BytecodeGenerator { state } = generator {
            let gen_state = state.lock().unwrap();

            // Check if generator is exhausted
            if gen_state.is_exhausted {
                return Ok(Value::Option { is_some: false, value: Box::new(Value::Null) });
            }

            // Save current VM state
            let saved_ip = self.ip;
            let saved_chunk = self.chunk.clone();
            let saved_stack = self.stack.clone();
            let saved_frames = self.call_frames.clone();

            // Restore generator state
            self.ip = gen_state.ip;
            self.set_chunk(gen_state.chunk.clone());
            self.stack = gen_state.stack.clone();

            // Restore call frames
            self.call_frames.clear();
            for frame_data in &gen_state.call_frames_data {
                self.call_frames.push(CallFrame {
                    return_ip: frame_data.return_ip,
                    stack_offset: frame_data.stack_offset,
                    locals: frame_data.locals.clone(),
                    locals_binding_kinds: frame_data.locals_binding_kinds.clone(),
                    local_slots: frame_data.local_slots.clone(),
                    local_slot_binding_kinds: frame_data.local_slot_binding_kinds.clone(),
                    local_slot_initialized: frame_data.local_slot_initialized.clone(),
                    captured: frame_data.captured.clone(),
                    captured_binding_kinds: frame_data.captured_binding_kinds.clone(),
                    prev_chunk: None,
                    is_async: false, // Generators are not async
                });
            }

            // Drop the lock before executing
            drop(gen_state);

            // Execute until yield or completion
            let result = loop {
                if self.ip >= self.chunk.instructions.len() {
                    // Generator completed without explicit return
                    break Ok(Value::Option { is_some: false, value: Box::new(Value::Null) });
                }

                let instruction = self.chunk.instructions[self.ip].clone();
                self.ip += 1;

                // Check for Yield opcode
                if matches!(instruction, OpCode::Yield) {
                    // Peek the yielded value but keep it on the stack.
                    // Expression-statement lowering emits a following Pop; preserving the
                    // value here keeps stack shape aligned for the resumed instruction stream.
                    let yielded_value =
                        self.stack.last().cloned().ok_or("Stack underflow in Yield")?;
                    let has_loop_backedge =
                        self.chunk.instructions.iter().any(|op| matches!(op, OpCode::JumpBack(_)));

                    // Preserve current interpreter parity for loop-bodied generators:
                    // generator state advances at statement granularity, so a yield inside
                    // a loop body exhausts after the first yielded value.
                    if has_loop_backedge {
                        let mut gen_state = state.lock().unwrap();
                        gen_state.is_exhausted = true;
                    } else {
                        // Save current state back to generator
                        let mut gen_state = state.lock().unwrap();
                        gen_state.ip = self.ip;
                        gen_state.stack = self.stack.clone();

                        // Save call frames
                        gen_state.call_frames_data.clear();
                        for frame in &self.call_frames {
                            gen_state.call_frames_data.push(CallFrameData {
                                return_ip: frame.return_ip,
                                stack_offset: frame.stack_offset,
                                locals: frame.locals.clone(),
                                locals_binding_kinds: frame.locals_binding_kinds.clone(),
                                local_slots: frame.local_slots.clone(),
                                local_slot_binding_kinds: frame.local_slot_binding_kinds.clone(),
                                local_slot_initialized: frame.local_slot_initialized.clone(),
                                captured: frame.captured.clone(),
                                captured_binding_kinds: frame.captured_binding_kinds.clone(),
                            });
                        }
                    }

                    // Restore original VM state
                    self.ip = saved_ip;
                    self.set_chunk(saved_chunk);
                    self.stack = saved_stack;
                    self.call_frames = saved_frames;

                    // Return the yielded value
                    break Ok(Value::Option { is_some: true, value: Box::new(yielded_value) });
                }

                // Check for Return opcodes (generator completed)
                if matches!(instruction, OpCode::Return | OpCode::ReturnNone) {
                    let mut gen_state = state.lock().unwrap();
                    gen_state.is_exhausted = true;
                    drop(gen_state);

                    // Restore original VM state
                    self.ip = saved_ip;
                    self.set_chunk(saved_chunk);
                    self.stack = saved_stack;
                    self.call_frames = saved_frames;

                    break Ok(Value::Option { is_some: false, value: Box::new(Value::Null) });
                }

                // Execute the instruction normally (by backing up IP and calling execute on single instruction)
                // This is inefficient but simple - a better approach would be to extract instruction execution
                // For now, we'll manually handle key instructions
                match instruction {
                    OpCode::LoadConst(index) => {
                        let constant = &self.chunk.constants[index];
                        let value = self.constant_to_value(constant)?;
                        self.stack.push(value);
                    }
                    OpCode::LoadVar(name) => {
                        let value = if let Some(frame) = self.call_frames.last() {
                            frame
                                .captured
                                .get(&name)
                                .map(|r| r.lock().unwrap().clone())
                                .or_else(|| frame.locals.get(&name).cloned())
                        } else {
                            None
                        };

                        let value = value
                            .or_else(|| self.globals.lock().unwrap().get(&name))
                            .ok_or_else(|| Self::undefined_variable_message(&name))?;
                        self.stack.push(value);
                    }
                    OpCode::LoadLocal(slot) => {
                        let frame =
                            self.call_frames.last().ok_or("LoadLocal requires call frame")?;
                        let value = frame
                            .local_slots
                            .get(slot)
                            .cloned()
                            .ok_or_else(|| format!("Invalid local slot: {}", slot))?;
                        self.stack.push(value);
                    }
                    OpCode::StoreVar(name) => {
                        let value = self.stack.last().ok_or("Stack underflow")?.clone();
                        let global_exists = self.globals.lock().unwrap().get(&name).is_some();
                        let mut assign_global = false;
                        if let Some(frame) = self.call_frames.last_mut() {
                            if let Some(captured_ref) = frame.captured.get(&name) {
                                if let Some(kind) = frame.captured_binding_kinds.get(&name).copied()
                                {
                                    if !matches!(kind, BytecodeBindingKind::Mutable) {
                                        return Err(Self::local_reassignment_error(kind, &name));
                                    }
                                }
                                *captured_ref.lock().unwrap() = value.clone();
                            } else if frame.locals.contains_key(&name) {
                                if let Some(kind) = frame.locals_binding_kinds.get(&name).copied() {
                                    if !matches!(kind, BytecodeBindingKind::Mutable) {
                                        return Err(Self::local_reassignment_error(kind, &name));
                                    }
                                }
                                frame.locals.insert(name.clone(), value.clone());
                            } else if global_exists {
                                assign_global = true;
                            } else {
                                frame
                                    .locals_binding_kinds
                                    .entry(name.clone())
                                    .or_insert(BytecodeBindingKind::Mutable);
                                frame.locals.insert(name.clone(), value.clone());
                            }
                        } else {
                            assign_global = true;
                        }

                        if assign_global {
                            self.globals.lock().unwrap().assign_checked(name, value)?;
                        }
                    }
                    OpCode::StoreLocal(slot) => {
                        let value = self.stack.last().ok_or("Stack underflow")?.clone();
                        let frame =
                            self.call_frames.last_mut().ok_or("StoreLocal requires call frame")?;
                        if let Some(target) = frame.local_slots.get_mut(slot) {
                            let kind = frame
                                .local_slot_binding_kinds
                                .get(slot)
                                .copied()
                                .unwrap_or(BytecodeBindingKind::Mutable);
                            let initialized =
                                frame.local_slot_initialized.get(slot).copied().unwrap_or(false);
                            if initialized && !matches!(kind, BytecodeBindingKind::Mutable) {
                                return Err(Self::local_reassignment_error(
                                    kind,
                                    "<generator-local>",
                                ));
                            }
                            *target = value;
                            if let Some(initialized_flag) =
                                frame.local_slot_initialized.get_mut(slot)
                            {
                                *initialized_flag = true;
                            }
                        } else {
                            return Err(format!("Invalid local slot: {}", slot));
                        }
                    }
                    OpCode::Pop => {
                        self.stack.pop().ok_or("Stack underflow")?;
                    }
                    OpCode::PushScope => {
                        self.globals.lock().unwrap().push_scope();
                    }
                    OpCode::PopScope => {
                        self.globals.lock().unwrap().pop_scope();
                    }

                    // Arithmetic operations
                    OpCode::Add => {
                        let right = self.stack.pop().ok_or("Stack underflow")?;
                        let left = self.stack.pop().ok_or("Stack underflow")?;
                        let result = self.binary_op(&left, "+", &right)?;
                        self.stack.push(result);
                    }
                    OpCode::Sub => {
                        let right = self.stack.pop().ok_or("Stack underflow")?;
                        let left = self.stack.pop().ok_or("Stack underflow")?;
                        let result = self.binary_op(&left, "-", &right)?;
                        self.stack.push(result);
                    }
                    OpCode::Mul => {
                        let right = self.stack.pop().ok_or("Stack underflow")?;
                        let left = self.stack.pop().ok_or("Stack underflow")?;
                        let result = self.binary_op(&left, "*", &right)?;
                        self.stack.push(result);
                    }
                    OpCode::Div => {
                        let right = self.stack.pop().ok_or("Stack underflow")?;
                        let left = self.stack.pop().ok_or("Stack underflow")?;
                        let result = self.binary_op(&left, "/", &right)?;
                        self.stack.push(result);
                    }
                    OpCode::Mod => {
                        let right = self.stack.pop().ok_or("Stack underflow")?;
                        let left = self.stack.pop().ok_or("Stack underflow")?;
                        let result = self.binary_op(&left, "%", &right)?;
                        self.stack.push(result);
                    }

                    // Comparison operations
                    OpCode::Equal => {
                        let right = self.stack.pop().ok_or("Stack underflow")?;
                        let left = self.stack.pop().ok_or("Stack underflow")?;
                        self.stack.push(Value::Bool(self.values_equal(&left, &right)));
                    }
                    OpCode::NotEqual => {
                        let right = self.stack.pop().ok_or("Stack underflow")?;
                        let left = self.stack.pop().ok_or("Stack underflow")?;
                        self.stack.push(Value::Bool(!self.values_equal(&left, &right)));
                    }
                    OpCode::LessThan => {
                        let right = self.stack.pop().ok_or("Stack underflow")?;
                        let left = self.stack.pop().ok_or("Stack underflow")?;
                        let result = self.compare_op(&left, "<", &right)?;
                        self.stack.push(result);
                    }
                    OpCode::GreaterThan => {
                        let right = self.stack.pop().ok_or("Stack underflow")?;
                        let left = self.stack.pop().ok_or("Stack underflow")?;
                        let result = self.compare_op(&left, ">", &right)?;
                        self.stack.push(result);
                    }
                    OpCode::LessEqual => {
                        let right = self.stack.pop().ok_or("Stack underflow")?;
                        let left = self.stack.pop().ok_or("Stack underflow")?;
                        let result = self.compare_op(&left, "<=", &right)?;
                        self.stack.push(result);
                    }
                    OpCode::GreaterEqual => {
                        let right = self.stack.pop().ok_or("Stack underflow")?;
                        let left = self.stack.pop().ok_or("Stack underflow")?;
                        let result = self.compare_op(&left, ">=", &right)?;
                        self.stack.push(result);
                    }

                    // Control flow
                    OpCode::Jump(target) => {
                        self.ip = target;
                    }
                    OpCode::JumpIfFalse(target) => {
                        let condition = self.stack.last().ok_or("Stack underflow")?;
                        if !self.is_truthy(condition) {
                            self.ip = target;
                        }
                    }
                    OpCode::JumpIfTrue(target) => {
                        let condition = self.stack.last().ok_or("Stack underflow")?;
                        if self.is_truthy(condition) {
                            self.ip = target;
                        }
                    }
                    OpCode::JumpBack(target) => {
                        self.ip = target;
                    }

                    // For now, return error for other unhandled instructions
                    // Full implementation would need to handle all opcodes
                    _ => {
                        return Err(format!(
                            "Instruction {:?} not yet handled in generator execution",
                            instruction
                        ));
                    }
                }
            };

            result
        } else {
            Err("generator_next() requires a BytecodeGenerator".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::Compiler;
    use crate::lexer;
    use crate::parser::Parser;
    use std::thread;
    use std::time::Duration;

    /// Helper to compile and run Kujo code through the VM
    fn run_vm_code(code: &str) -> Result<Value, String> {
        let tokens = lexer::tokenize(code).map_err(|diagnostics| {
            diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.clone())
                .unwrap_or_else(|| "unknown lexer error".to_string())
        })?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();

        let mut compiler = Compiler::new();
        let chunk = compiler.compile(&ast)?;

        let mut vm = VM::new();
        vm.execute(chunk)
    }

    fn run_vm_code_with_natives(code: &str, native_names: &[&str]) -> Result<Value, String> {
        let tokens = lexer::tokenize(code).map_err(|diagnostics| {
            diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.clone())
                .unwrap_or_else(|| "unknown lexer error".to_string())
        })?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();

        let mut compiler = Compiler::new();
        let chunk = compiler.compile(&ast)?;

        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            for native_name in native_names {
                globals.define(
                    (*native_name).to_string(),
                    Value::NativeFunction((*native_name).to_string()),
                );
            }
        }

        vm.execute(chunk)
    }

    fn compile_chunk(code: &str) -> BytecodeChunk {
        let tokens = lexer::tokenize(code).expect("test source should tokenize");
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        let mut compiler = Compiler::new();
        compiler.compile(&ast).expect("compile should succeed")
    }

    #[test]
    fn test_jit_is_disabled_by_default() {
        let vm = VM::new();
        assert!(!vm.jit_enabled(), "JIT should require explicit opt-in");
    }

    #[test]
    fn test_vm_for_numeric_iteration_matches_interpreter_range_contract() {
        let int_result = run_vm_code_with_natives(
            r#"
            total := 0
            for i in 3 {
                total := total + i
            }
            return total
            "#,
            &["len", "__vm_for_iterable"],
        )
        .expect("VM should execute integer for-range");
        assert!(matches!(int_result, Value::Int(3)));

        let float_result = run_vm_code_with_natives(
            r#"
            total := 0
            for i in 3.9 {
                total := total + i
            }
            return total
            "#,
            &["len", "__vm_for_iterable"],
        )
        .expect("VM should execute float for-range");
        assert!(matches!(float_result, Value::Int(3)));
    }

    #[test]
    fn test_validate_jit_supported_surfaces_reports_unsupported_opcode() {
        let chunk = compile_chunk(
            r#"
            values := [1, 2, 3]
            return values[0]
            "#,
        );

        let vm = VM::new();
        let error =
            vm.validate_jit_supported_surfaces(&chunk).expect_err("array ops are not JIT-safe");
        assert!(error.contains("unsupported opcode"));
        assert!(error.contains("MakeArray") || error.contains("IndexGet"));
    }

    #[test]
    fn test_jit_supported_arithmetic_program_executes_when_enabled() {
        let chunk = compile_chunk(
            r#"
            return (1 + 2) * 5
            "#,
        );

        let mut vm = VM::new();
        vm.set_jit_enabled(true);
        vm.validate_jit_supported_surfaces(&chunk)
            .expect("pure arithmetic script should be JIT-compatible");

        match vm.execute(chunk) {
            Ok(Value::Int(value)) => assert_eq!(value, 15),
            Ok(other) => panic!("Expected int 15, got {:?}", other),
            Err(error) => panic!("Expected success, got VM error: {}", error),
        }
    }

    #[test]
    fn test_jit_supported_function_call_path_matches_interpreter_result() {
        let chunk = compile_chunk(
            r#"
            func inc(x) {
                return x + 1
            }

            return inc(41)
            "#,
        );

        let function_chunk = chunk
            .constants
            .iter()
            .find_map(|constant| match constant {
                Constant::Function(function_chunk) => Some((**function_chunk).clone()),
                _ => None,
            })
            .expect("expected compiled function constant");

        let mut vm = VM::new();
        vm.set_jit_enabled(true);

        let compiled = vm
            .jit_compile_bytecode_function(&Value::BytecodeFunction {
                chunk: function_chunk,
                captured: HashMap::new(),
                captured_binding_kinds: HashMap::new(),
            })
            .expect("jit compilation attempt should not hard-fail");
        assert!(compiled, "expected JIT to compile supported function chunk");

        match vm.execute(chunk) {
            Ok(Value::Int(value)) => assert_eq!(value, 42),
            Ok(other) => panic!("Expected int 42, got {:?}", other),
            Err(error) => panic!("Expected success, got VM error: {}", error),
        }
    }

    #[test]
    fn test_async_function_definition() {
        let code = r#"
            async func fetch_data(id) {
                return "Data for ID";
            }
            
            let promise = fetch_data(42);
            return await promise;
        "#;

        match run_vm_code(code) {
            Ok(Value::Str(s)) => assert_eq!(s.as_ref(), "Data for ID"),
            Ok(other) => panic!("Expected string, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_save_restore_execution_state_round_trip() {
        let mut vm = VM::new();

        let mut snapshot_chunk = BytecodeChunk::new();
        snapshot_chunk.name = Some("snapshot_chunk".to_string());
        snapshot_chunk.instructions.push(OpCode::Return);

        vm.ip = 7;
        vm.stack = vec![Value::Int(99), Value::Bool(true)];
        vm.chunk = snapshot_chunk.clone();
        vm.upvalues =
            vec![Upvalue { value: Arc::new(Mutex::new(Value::Int(123))), is_closed: true }];
        vm.exception_handlers =
            vec![ExceptionHandlerFrame { catch_ip: 19, stack_offset: 2, frame_offset: 1 }];
        vm.function_call_stack = vec!["outer".to_string(), "inner".to_string()];
        vm.function_call_counts.insert("hot_fn".to_string(), 42);
        vm.recursion_depth = 3;
        vm.max_recursion_depth = 5;
        vm.int_key_cache.insert(7, Arc::from("7"));
        vm.jit_obj_stack = vec![Value::Str(Arc::new("jit-object".to_string()))];

        let mut frame_chunk = BytecodeChunk::new();
        frame_chunk.name = Some("frame_chunk".to_string());
        vm.call_frames.push(CallFrame {
            return_ip: 11,
            stack_offset: 1,
            locals: HashMap::from([("local_x".to_string(), Value::Int(10))]),
            locals_binding_kinds: HashMap::from([(
                "local_x".to_string(),
                BytecodeBindingKind::Mutable,
            )]),
            local_slots: vec![Value::Int(10), Value::Bool(false)],
            local_slot_binding_kinds: vec![
                BytecodeBindingKind::Mutable,
                BytecodeBindingKind::Mutable,
            ],
            local_slot_initialized: vec![true, true],
            captured: HashMap::new(),
            captured_binding_kinds: HashMap::new(),
            prev_chunk: Some(frame_chunk),
            is_async: false,
        });

        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define("saved_var".to_string(), Value::Int(314));
        }

        let snapshot = vm.save_execution_state();

        vm.ip = 0;
        vm.stack.clear();
        vm.call_frames.clear();
        vm.chunk = BytecodeChunk::new();
        vm.upvalues.clear();
        vm.exception_handlers.clear();
        vm.function_call_stack.clear();
        vm.function_call_counts.clear();
        vm.recursion_depth = 0;
        vm.max_recursion_depth = 0;
        vm.int_key_cache.clear();
        vm.jit_obj_stack.clear();
        vm.set_globals(Arc::new(Mutex::new(Environment::new())));

        vm.restore_execution_state(snapshot);

        assert_eq!(vm.ip, 7);
        assert_eq!(vm.stack.len(), 2);
        assert!(matches!(vm.stack.first(), Some(Value::Int(99))));
        assert!(matches!(vm.stack.get(1), Some(Value::Bool(true))));
        assert_eq!(vm.call_frames.len(), 1);
        assert_eq!(vm.call_frames[0].return_ip, 11);
        assert_eq!(vm.call_frames[0].stack_offset, 1);
        assert!(matches!(vm.call_frames[0].locals.get("local_x"), Some(Value::Int(10))));
        assert_eq!(vm.call_frames[0].local_slots.len(), 2);
        assert_eq!(vm.chunk.name.as_deref(), Some("snapshot_chunk"));
        assert_eq!(vm.upvalues.len(), 1);
        assert!(vm.upvalues[0].is_closed);
        assert_eq!(vm.exception_handlers.len(), 1);
        assert_eq!(vm.exception_handlers[0].catch_ip, 19);
        assert_eq!(vm.function_call_stack, vec!["outer".to_string(), "inner".to_string()]);
        assert_eq!(vm.function_call_counts.get("hot_fn"), Some(&42));
        assert_eq!(vm.recursion_depth, 3);
        assert_eq!(vm.max_recursion_depth, 5);
        assert_eq!(vm.int_key_cache.get(&7), Some(&Arc::from("7")));
        assert_eq!(vm.jit_obj_stack.len(), 1);
        match vm.jit_obj_stack.first() {
            Some(Value::Str(s)) => assert_eq!(s.as_ref(), "jit-object"),
            other => panic!("Expected Value::Str for jit_obj_stack[0], got: {:?}", other),
        }
        let globals = vm.globals.lock().unwrap();
        assert!(matches!(globals.get("saved_var"), Some(Value::Int(314))));
    }

    #[test]
    fn test_execution_snapshot_isolated_from_mutations_after_save() {
        let mut vm = VM::new();
        vm.ip = 2;
        vm.stack.push(Value::Int(1));
        vm.function_call_stack.push("original".to_string());

        let snapshot = vm.save_execution_state();

        vm.ip = 99;
        vm.stack.push(Value::Int(2));
        vm.function_call_stack.push("mutated".to_string());

        assert_eq!(snapshot.ip, 2);
        assert_eq!(snapshot.stack.len(), 1);
        assert!(matches!(snapshot.stack.first(), Some(Value::Int(1))));
        assert_eq!(snapshot.function_call_stack, vec!["original".to_string()]);
    }

    #[test]
    fn test_restore_execution_state_replaces_globals_reference() {
        let mut vm = VM::new();

        let snapshot_globals = Arc::new(Mutex::new(Environment::new()));
        {
            let mut globals = snapshot_globals.lock().unwrap();
            globals.define("from_snapshot".to_string(), Value::Int(88));
        }

        let snapshot = VmExecutionSnapshot {
            ip: 0,
            stack: Vec::new(),
            call_frames: Vec::new(),
            chunk: BytecodeChunk::new(),
            upvalues: Vec::new(),
            exception_handlers: Vec::new(),
            function_call_stack: Vec::new(),
            function_call_counts: HashMap::new(),
            recursion_depth: 0,
            max_recursion_depth: 0,
            int_key_cache: HashMap::new(),
            jit_obj_stack: Vec::new(),
            globals: Arc::clone(&snapshot_globals),
        };

        vm.set_globals(Arc::new(Mutex::new(Environment::new())));
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define("other".to_string(), Value::Int(1));
        }

        vm.restore_execution_state(snapshot);

        let globals = vm.globals.lock().unwrap();
        assert!(matches!(globals.get("from_snapshot"), Some(Value::Int(88))));
        assert!(globals.get("other").is_none());
    }

    #[test]
    fn test_create_and_list_execution_context_ids() {
        let mut vm = VM::new();
        vm.ip = 10;
        let id_one = vm.create_execution_context_from_current();

        vm.ip = 20;
        let id_two = vm.create_execution_context_from_current();

        assert_ne!(id_one, id_two);
        assert_eq!(vm.active_execution_context_id(), Some(id_one));
        assert!(vm.has_execution_context(id_one));
        assert!(vm.has_execution_context(id_two));
        assert_eq!(vm.list_execution_context_ids(), vec![id_one, id_two]);
    }

    #[test]
    fn test_switch_execution_context_restores_target_and_saves_previous() {
        let mut vm = VM::new();

        vm.ip = 100;
        vm.stack = vec![Value::Int(1)];
        let snapshot_one = vm.save_execution_state();

        vm.ip = 200;
        vm.stack = vec![Value::Int(2)];
        let snapshot_two = vm.save_execution_state();

        let context_one = vm.create_execution_context(snapshot_one);
        let context_two = vm.create_execution_context(snapshot_two);

        vm.switch_execution_context(context_one).expect("switch to context_one should succeed");
        assert_eq!(vm.active_execution_context_id(), Some(context_one));
        assert_eq!(vm.ip, 100);
        assert!(matches!(vm.stack.first(), Some(Value::Int(1))));

        vm.ip = 150;
        vm.stack = vec![Value::Int(15)];

        vm.switch_execution_context(context_two).expect("switch to context_two should succeed");
        assert_eq!(vm.active_execution_context_id(), Some(context_two));
        assert_eq!(vm.ip, 200);
        assert!(matches!(vm.stack.first(), Some(Value::Int(2))));

        vm.switch_execution_context(context_one)
            .expect("switch back to context_one should succeed");
        assert_eq!(vm.active_execution_context_id(), Some(context_one));
        assert_eq!(vm.ip, 150);
        assert!(matches!(vm.stack.first(), Some(Value::Int(15))));
    }

    #[test]
    fn test_switch_execution_context_missing_context_returns_error() {
        let mut vm = VM::new();
        let error =
            vm.switch_execution_context(999).expect_err("switching to missing context should fail");
        assert!(error.contains("not found"));
    }

    #[test]
    fn test_remove_execution_context_rejects_active_context() {
        let mut vm = VM::new();
        vm.ip = 5;

        let active_context = vm.create_execution_context_from_current();
        let extra_context = vm.create_execution_context(vm.save_execution_state());

        let active_err = vm
            .remove_execution_context(active_context)
            .expect_err("active context removal should fail");
        assert!(active_err.contains("Cannot remove active"));

        vm.remove_execution_context(extra_context)
            .expect("non-active context removal should succeed");
        assert!(!vm.has_execution_context(extra_context));
    }

    #[test]
    fn test_execute_until_suspend_and_resume_for_pending_await() {
        let code = r#"
            p := async_sleep(25)
            await p
            return 7
        "#;

        let chunk = compile_chunk(code);

        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define(
                "async_sleep".to_string(),
                Value::NativeFunction("async_sleep".to_string()),
            );
        }
        let first_result =
            vm.execute_until_suspend(chunk).expect("cooperative execution should not error");

        let context_id = match first_result {
            VmExecutionResult::Suspended { context_id } => context_id,
            VmExecutionResult::Completed => {
                panic!("expected suspension for async_sleep await, got completion")
            }
        };

        let mut completed = false;
        for _ in 0..30 {
            thread::sleep(Duration::from_millis(3));
            match vm.resume_execution_context(context_id).expect("resume should not fail") {
                VmExecutionResult::Suspended { context_id: resumed_context_id } => {
                    assert_eq!(resumed_context_id, context_id);
                }
                VmExecutionResult::Completed => {
                    completed = true;
                    break;
                }
            }
        }

        assert!(completed, "execution did not complete after resume attempts");
    }

    #[test]
    fn test_execute_until_suspend_completes_without_pending_await() {
        let code = r#"
            return 42
        "#;

        let chunk = compile_chunk(code);

        let mut vm = VM::new();
        let result =
            vm.execute_until_suspend(chunk).expect("cooperative execution should not fail");

        assert!(matches!(result, VmExecutionResult::Completed));
    }

    #[test]
    fn test_resume_execution_context_removes_completed_context() {
        let code = r#"
            p := async_sleep(5)
            await p
            return 1
        "#;

        let chunk = compile_chunk(code);
        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define(
                "async_sleep".to_string(),
                Value::NativeFunction("async_sleep".to_string()),
            );
        }

        let suspended_context = match vm.execute_until_suspend(chunk).expect("initial run") {
            VmExecutionResult::Suspended { context_id } => context_id,
            VmExecutionResult::Completed => panic!("expected suspension"),
        };

        let mut done = false;
        for _ in 0..50 {
            thread::sleep(Duration::from_millis(2));
            match vm.resume_execution_context(suspended_context).expect("resume should not fail") {
                VmExecutionResult::Suspended { .. } => {}
                VmExecutionResult::Completed => {
                    done = true;
                    break;
                }
            }
        }

        assert!(done, "context did not complete in expected window");
        assert_eq!(vm.pending_execution_context_count(), 0);
        assert!(!vm.has_execution_context(suspended_context));
        assert_eq!(vm.active_execution_context_id(), None);
    }

    #[test]
    fn test_run_scheduler_round_for_completed_contexts() {
        let mut vm = VM::new();

        let mut chunk_a = BytecodeChunk::new();
        chunk_a.name = Some("ctx_a".to_string());
        chunk_a.instructions = vec![OpCode::Return];

        let context_a = vm.create_execution_context(VmExecutionSnapshot {
            ip: 0,
            stack: vec![Value::Int(11)],
            call_frames: Vec::new(),
            chunk: chunk_a,
            upvalues: Vec::new(),
            exception_handlers: Vec::new(),
            function_call_stack: Vec::new(),
            function_call_counts: HashMap::new(),
            recursion_depth: 0,
            max_recursion_depth: 0,
            int_key_cache: HashMap::new(),
            jit_obj_stack: Vec::new(),
            globals: Arc::clone(&vm.globals),
        });

        let mut chunk_b = BytecodeChunk::new();
        chunk_b.name = Some("ctx_b".to_string());
        chunk_b.instructions = vec![OpCode::Return];

        let context_b = vm.create_execution_context(VmExecutionSnapshot {
            ip: 0,
            stack: vec![Value::Int(22)],
            call_frames: Vec::new(),
            chunk: chunk_b,
            upvalues: Vec::new(),
            exception_handlers: Vec::new(),
            function_call_stack: Vec::new(),
            function_call_counts: HashMap::new(),
            recursion_depth: 0,
            max_recursion_depth: 0,
            int_key_cache: HashMap::new(),
            jit_obj_stack: Vec::new(),
            globals: Arc::clone(&vm.globals),
        });

        let round_result = vm.run_scheduler_round().expect("scheduler round should succeed");

        assert_eq!(round_result.completed_contexts, 2);
        assert_eq!(round_result.pending_contexts, 0);
        assert!(!vm.has_execution_context(context_a));
        assert!(!vm.has_execution_context(context_b));
        assert_eq!(vm.pending_execution_context_count(), 0);
    }

    #[test]
    fn test_run_scheduler_until_complete_with_pending_async_contexts() {
        let code_one = r#"
            p := async_sleep(8)
            await p
            return 1
        "#;

        let code_two = r#"
            p := async_sleep(12)
            await p
            return 2
        "#;

        let chunk_one = compile_chunk(code_one);
        let chunk_two = compile_chunk(code_two);

        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define(
                "async_sleep".to_string(),
                Value::NativeFunction("async_sleep".to_string()),
            );
        }

        let context_one = match vm.execute_until_suspend(chunk_one).expect("chunk one run") {
            VmExecutionResult::Suspended { context_id } => context_id,
            VmExecutionResult::Completed => panic!("chunk one should suspend"),
        };

        let context_two = match vm.execute_until_suspend(chunk_two).expect("chunk two run") {
            VmExecutionResult::Suspended { context_id } => context_id,
            VmExecutionResult::Completed => panic!("chunk two should suspend"),
        };

        assert!(vm.has_execution_context(context_one));
        assert!(vm.has_execution_context(context_two));
        assert_eq!(vm.pending_execution_context_count(), 2);

        vm.run_scheduler_until_complete(300).expect("scheduler should complete both contexts");

        assert_eq!(vm.pending_execution_context_count(), 0);
        assert!(!vm.has_execution_context(context_one));
        assert!(!vm.has_execution_context(context_two));
        assert_eq!(vm.active_execution_context_id(), None);
    }

    #[test]
    fn test_run_scheduler_until_complete_with_timeout_with_pending_async_contexts() {
        let code_one = r#"
            p := async_sleep(8)
            await p
            return 1
        "#;

        let code_two = r#"
            p := async_sleep(12)
            await p
            return 2
        "#;

        let chunk_one = compile_chunk(code_one);
        let chunk_two = compile_chunk(code_two);

        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define(
                "async_sleep".to_string(),
                Value::NativeFunction("async_sleep".to_string()),
            );
        }

        let context_one = match vm.execute_until_suspend(chunk_one).expect("chunk one run") {
            VmExecutionResult::Suspended { context_id } => context_id,
            VmExecutionResult::Completed => panic!("chunk one should suspend"),
        };

        let context_two = match vm.execute_until_suspend(chunk_two).expect("chunk two run") {
            VmExecutionResult::Suspended { context_id } => context_id,
            VmExecutionResult::Completed => panic!("chunk two should suspend"),
        };

        assert!(vm.has_execution_context(context_one));
        assert!(vm.has_execution_context(context_two));
        assert_eq!(vm.pending_execution_context_count(), 2);

        vm.run_scheduler_until_complete_with_timeout(Duration::from_millis(300))
            .expect("scheduler should complete both contexts");

        assert_eq!(vm.pending_execution_context_count(), 0);
        assert!(!vm.has_execution_context(context_one));
        assert!(!vm.has_execution_context(context_two));
        assert_eq!(vm.active_execution_context_id(), None);
    }

    #[test]
    fn test_run_scheduler_until_complete_rejects_zero_rounds() {
        let mut vm = VM::new();
        let err = vm.run_scheduler_until_complete(0).expect_err("zero rounds should fail");
        assert!(err.contains("max_rounds"));
    }

    #[test]
    fn test_run_scheduler_until_complete_with_timeout_rejects_zero_timeout() {
        let mut vm = VM::new();
        let err = vm
            .run_scheduler_until_complete_with_timeout(Duration::from_millis(0))
            .expect_err("zero timeout should fail");
        assert!(err.contains("timeout"));
    }

    #[test]
    fn test_run_scheduler_round_reports_pending_for_unready_context() {
        let code = r#"
            p := async_sleep(40)
            await p
            return 9
        "#;

        let chunk = compile_chunk(code);
        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define(
                "async_sleep".to_string(),
                Value::NativeFunction("async_sleep".to_string()),
            );
        }

        let context_id = match vm.execute_until_suspend(chunk).expect("initial run") {
            VmExecutionResult::Suspended { context_id } => context_id,
            VmExecutionResult::Completed => panic!("expected suspension"),
        };

        let round_result = vm.run_scheduler_round().expect("scheduler round should succeed");
        assert_eq!(round_result.completed_contexts, 0);
        assert_eq!(round_result.pending_contexts, 1);
        assert!(vm.has_execution_context(context_id));
    }

    #[test]
    fn test_run_scheduler_until_complete_errors_when_round_budget_exhausted() {
        let code = r#"
            p := async_sleep(80)
            await p
            return 3
        "#;

        let chunk = compile_chunk(code);
        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define(
                "async_sleep".to_string(),
                Value::NativeFunction("async_sleep".to_string()),
            );
        }

        match vm.execute_until_suspend(chunk).expect("initial run") {
            VmExecutionResult::Suspended { .. } => {}
            VmExecutionResult::Completed => panic!("expected suspension"),
        }

        let err =
            vm.run_scheduler_until_complete(1).expect_err("single round should be insufficient");
        assert!(err.contains("did not complete"));
        assert!(err.contains("pending"));
    }

    #[test]
    fn test_run_scheduler_until_complete_with_timeout_errors_when_budget_exhausted() {
        let code = r#"
            p := async_sleep(80)
            await p
            return 3
        "#;

        let chunk = compile_chunk(code);
        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define(
                "async_sleep".to_string(),
                Value::NativeFunction("async_sleep".to_string()),
            );
        }

        match vm.execute_until_suspend(chunk).expect("initial run") {
            VmExecutionResult::Suspended { .. } => {}
            VmExecutionResult::Completed => panic!("expected suspension"),
        }

        let err = vm
            .run_scheduler_until_complete_with_timeout(Duration::from_millis(1))
            .expect_err("tiny timeout budget should be insufficient");
        assert!(err.contains("timed out"));
        assert!(err.contains("pending"));
    }

    #[test]
    fn test_simple_return() {
        let code = r#"
            func get_number() {
                return 42;
            }
            
            return get_number();
        "#;

        let result = run_vm_code(code);
        eprintln!("Simple return test: {:?}", result);

        match result {
            Ok(Value::Int(n)) => assert_eq!(n, 42),
            Ok(other) => panic!("Expected int 42, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_vm_to_json_serializes_fixed_dict_literals() {
        let code = r#"
            payload := {"title": "Kujo", "count": 2}
            return to_json(payload)
        "#;

        match run_vm_code_with_natives(code, &["to_json"]) {
            Ok(Value::Str(json)) => {
                let decoded: serde_json::Value =
                    serde_json::from_str(json.as_ref()).expect("serialized JSON should parse");
                assert_eq!(decoded["title"], serde_json::Value::String("Kujo".to_string()));
                assert_eq!(decoded["count"], serde_json::Value::Number(2.into()));
            }
            Ok(other) => panic!("Expected JSON string, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_vm_to_json_array_does_not_duplicate_entries() {
        let code = r#"
            values := [1, 2, 3]
            return to_json(values)
        "#;

        match run_vm_code_with_natives(code, &["to_json"]) {
            Ok(Value::Str(json)) => {
                let decoded: serde_json::Value =
                    serde_json::from_str(json.as_ref()).expect("serialized JSON should parse");
                let items = decoded.as_array().expect("serialized array should decode to an array");
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], serde_json::Value::Number(1.into()));
                assert_eq!(items[1], serde_json::Value::Number(2.into()));
                assert_eq!(items[2], serde_json::Value::Number(3.into()));
            }
            Ok(other) => panic!("Expected JSON string, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_vm_string_len_and_indexing_use_character_counts() {
        let code = r#"
            s := "Free Tools → Plan"
            mut i := 0
            while i < len(s) {
                ch := s[i]
                i = i + 1
            }
            return "done"
        "#;

        match run_vm_code_with_natives(code, &["len"]) {
            Ok(Value::Str(result)) => assert_eq!(result.as_ref(), "done"),
            Ok(other) => panic!("Expected string result, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_vm_http_server_route_method_returns_updated_server() {
        let code = r#"
            server := http_server(4123)
            server := server.route("GET", "/health", func(req) {
                return http_response(200, "ok")
            })
            return server
        "#;

        match run_vm_code_with_natives(code, &["http_server", "http_response"]) {
            Ok(Value::HttpServer { host, port, routes }) => {
                assert_eq!(host, "0.0.0.0");
                assert_eq!(port, 4123);
                assert_eq!(routes.len(), 1);
                assert_eq!(routes[0].0, "GET");
                assert_eq!(routes[0].1, "/health");
            }
            Ok(other) => panic!("Expected HttpServer, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_vm_http_handler_wrapper_executes_lambda_response_correctly() {
        let code = r#"
            func with_cors(response) {
                response := set_header(response, "Access-Control-Allow-Origin", "*")
                response := set_header(response, "Access-Control-Allow-Methods", "GET, POST, PUT, PATCH, DELETE, OPTIONS")
                response := set_header(response, "Access-Control-Allow-Headers", "Content-Type, Authorization")
                return response
            }

            func api_json(status, payload) {
                return with_cors(json_response(status, payload))
            }

            return func(request) {
                return api_json(200, {
                    "service": "Kujo CRUD API Showcase",
                    "status_values": ["draft", "published", "archived"]
                })
            }
        "#;

        let chunk = compile_chunk(code);
        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define(
                "json_response".to_string(),
                Value::NativeFunction("json_response".to_string()),
            );
            globals
                .define("set_header".to_string(), Value::NativeFunction("set_header".to_string()));
        }

        let handler = vm.execute(chunk).expect("handler should compile and execute");

        let mut req_fields = DictMap::default();
        req_fields.insert("method".into(), Value::Str(Arc::new("GET".to_string())));
        req_fields.insert("path".into(), Value::Str(Arc::new("/".to_string())));
        req_fields.insert("body".into(), Value::Str(Arc::new(String::new())));
        req_fields.insert("params".into(), Value::Dict(Arc::new(DictMap::default())));
        req_fields.insert("headers".into(), Value::Dict(Arc::new(DictMap::default())));

        match vm.call_http_handler_vm(handler, Value::Dict(Arc::new(req_fields))) {
            Ok(Value::HttpResponse { status, body, .. }) => {
                assert_eq!(status, 200);
                assert!(body.contains("\"service\":\"Kujo CRUD API Showcase\""));
                assert!(body.contains("\"status_values\""));
                assert!(body.contains("\"archived\""));
            }
            Ok(other) => panic!("Expected HttpResponse, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_async_await_basic() {
        let code = r#"
            async func get_number() {
                return 42;
            }
            
            let p = get_number();
            return await p;
        "#;

        let result = run_vm_code(code);
        eprintln!("Test result: {:?}", result);

        match result {
            Ok(Value::Int(n)) => assert_eq!(n, 42),
            Ok(other) => panic!("Expected int 42, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_async_multiple_await() {
        let code = r#"
            async func double(x) {
                return x * 2;
            }
            
            let p1 = double(5);
            let p2 = double(10);
            let p3 = double(15);
            
            let r1 = await p1;
            let r2 = await p2;
            let r3 = await p3;
            
            return r1 + r2 + r3;
        "#;

        match run_vm_code(code) {
            Ok(Value::Int(n)) => assert_eq!(n, 10 + 20 + 30),
            Ok(other) => panic!("Expected int 60, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_async_nested_calls() {
        let code = r#"
            async func inner(x) {
                return x + 10;
            }
            
            async func outer(x) {
                let p = inner(x);
                let result = await p;
                return result * 2;
            }
            
            let p = outer(5);
            return await p;
        "#;

        match run_vm_code(code) {
            Ok(Value::Int(n)) => assert_eq!(n, (5 + 10) * 2),
            Ok(other) => panic!("Expected int 30, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_async_with_computation() {
        let code = r#"
            async func calculate_sum(a, b, c) {
                let sum = a + b + c;
                return sum;
            }
            
            let promise = calculate_sum(10, 20, 30);
            return await promise;
        "#;

        match run_vm_code(code) {
            Ok(Value::Int(n)) => assert_eq!(n, 60),
            Ok(other) => panic!("Expected int 60, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_make_promise_opcode() {
        let code = r#"
            # Test MakePromise opcode (though not directly accessible in syntax)
            # We can test it indirectly through async functions
            async func simple() {
                return 123;
            }
            
            return await simple();
        "#;

        match run_vm_code(code) {
            Ok(Value::Int(n)) => assert_eq!(n, 123),
            Ok(other) => panic!("Expected int 123, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_async_promise_reuse() {
        let code = r#"
            async func get_value() {
                return 999;
            }
            
            let promise = get_value();
            
            # Await the same promise multiple times
            let first = await promise;
            let second = await promise;
            
            return first == second;
        "#;

        match run_vm_code(code) {
            Ok(Value::Bool(b)) => assert!(b, "Promise should return same value on multiple awaits"),
            Ok(other) => panic!("Expected bool true, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_await_non_promise() {
        let code = r#"
            # Awaiting a non-promise should just return the value
            let x = 42;
            return await x;
        "#;

        match run_vm_code(code) {
            Ok(Value::Int(n)) => assert_eq!(n, 42),
            Ok(other) => panic!("Expected int 42, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_sum_int_map_until_local_in_place_result() {
        let code = r#"
            n := 10000
            map := {}
            i := 0
            while i < n {
                map[i] := i * 2
                i := i + 1
            }

            sum := 0
            i := 0
            while i < n {
                sum := sum + map[i]
                i := i + 1
            }

            return sum
        "#;

        match run_vm_code(code) {
            Ok(Value::Int(n)) => assert_eq!(n, 99_990_000),
            Ok(other) => panic!("Expected int 99990000, got: {:?}", other),
            Err(e) => panic!("VM error: {}", e),
        }
    }

    #[test]
    fn test_sum_int_map_until_local_in_place_missing_key_errors() {
        let code = r#"
            map := {}
            map[0] := 1

            sum := 0
            i := 0
            n := 2

            while i < n {
                sum := sum + map[i]
                i := i + 1
            }

            return sum
        "#;

        match run_vm_code(code) {
            Ok(value) => panic!("Expected runtime error, got value: {:?}", value),
            Err(error) => assert!(error.contains("Missing map key")),
        }
    }

    #[test]
    fn test_vm_recursion_boundary_succeeds() {
        let code = r#"
            func countdown(n) {
                if n == 0 {
                    return 0
                }
                return 1 + countdown(n - 1)
            }

            return countdown(64)
        "#;

        match run_vm_code(code) {
            Ok(Value::Int(value)) => assert_eq!(value, 64),
            Ok(other) => panic!("Expected int 64, got: {:?}", other),
            Err(error) => panic!("Expected recursion boundary success, got VM error: {}", error),
        }
    }

    #[test]
    fn test_vm_recursion_exceeding_limit_errors() {
        let code = r#"
            func loop_forever(n) {
                return loop_forever(n + 1)
            }

            return loop_forever(0)
        "#;

        match run_vm_code(code) {
            Ok(value) => panic!("Expected call stack depth error, got value: {:?}", value),
            Err(error) => assert!(error.contains(&format!(
                "Maximum VM call stack depth of {} exceeded",
                runtime_limits::DEFAULT_MAX_VM_CALL_DEPTH
            ))),
        }
    }

    #[test]
    fn test_cooperative_suspend_enabled_by_default() {
        let vm = VM::new();
        // Cooperative suspend should be enabled by default
        assert!(vm.cooperative_suspend_enabled);
    }

    #[test]
    fn test_execute_until_suspend_with_single_awaiting_context() {
        let code = r#"
            p := async_sleep(50)
            await p
            return 42
        "#;

        let chunk = compile_chunk(code);
        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define(
                "async_sleep".to_string(),
                Value::NativeFunction("async_sleep".to_string()),
            );
        }

        // Initial execution should suspend
        let result = vm.execute_until_suspend(chunk).expect("execute_until_suspend should succeed");

        match result {
            VmExecutionResult::Suspended { context_id } => {
                // Verify the context was created
                assert!(vm.has_execution_context(context_id));
            }
            VmExecutionResult::Completed => {
                panic!("Expected suspension, but execution completed immediately")
            }
        }
    }

    #[test]
    fn test_cooperative_scheduler_resumes_and_completes_pending_await() {
        let code = r#"
            p := async_sleep(25)
            await p
            return 42
        "#;

        let chunk = compile_chunk(code);
        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define(
                "async_sleep".to_string(),
                Value::NativeFunction("async_sleep".to_string()),
            );
        }

        // Initial execution should suspend
        let initial_result =
            vm.execute_until_suspend(chunk).expect("initial execution should work");
        match initial_result {
            VmExecutionResult::Suspended { .. } => {
                // Suspended as expected - now use scheduler to complete it
                let scheduler_result =
                    vm.run_scheduler_until_complete(50).expect("scheduler should complete");
                assert_eq!(scheduler_result, ()); // No error means success
            }
            VmExecutionResult::Completed => {
                panic!("Expected suspension on initial execution")
            }
        }
    }

    #[test]
    fn test_cooperative_multiple_concurrent_awaits() {
        let code1 = r#"
            p := async_sleep(10)
            await p
            return 1
        "#;

        let code2 = r#"
            p := async_sleep(15)
            await p
            return 2
        "#;

        let chunk1 = compile_chunk(code1);
        let chunk2 = compile_chunk(code2);

        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define(
                "async_sleep".to_string(),
                Value::NativeFunction("async_sleep".to_string()),
            );
        }

        // Execute first context until suspension
        let ctx1 = match vm.execute_until_suspend(chunk1.clone()).expect("first chunk") {
            VmExecutionResult::Suspended { context_id } => context_id,
            VmExecutionResult::Completed => panic!("expected suspension"),
        };

        // Execute second context until suspension
        let ctx2 = match vm.execute_until_suspend(chunk2.clone()).expect("second chunk") {
            VmExecutionResult::Suspended { context_id } => context_id,
            VmExecutionResult::Completed => panic!("expected suspension"),
        };

        // Verify both contexts exist
        assert!(vm.has_execution_context(ctx1));
        assert!(vm.has_execution_context(ctx2));

        // Scheduler should complete both
        vm.run_scheduler_until_complete(100).expect("scheduler should complete both contexts");

        // Both should be removed after completion
        assert!(!vm.has_execution_context(ctx1));
        assert!(!vm.has_execution_context(ctx2));
    }

    #[test]
    fn test_non_async_code_completes_immediately_even_in_cooperative_mode() {
        let code = r#"
            x := 5
            y := 10
            return x + y
        "#;

        let chunk = compile_chunk(code);
        let mut vm = VM::new();

        // Cooperative mode is enabled by default
        assert!(vm.cooperative_suspend_enabled);

        // Non-async code should still complete immediately
        let result = vm.execute_until_suspend(chunk).expect("execution should not error");
        match result {
            VmExecutionResult::Completed => {
                // Expected - no suspension for non-async code
            }
            VmExecutionResult::Suspended { .. } => {
                panic!("Non-async code should not suspend")
            }
        }
    }

    #[test]
    fn test_mixed_async_and_sync_operations_in_cooperative_mode() {
        let code = r#"
            x := 10
            p := async_sleep(10)
            await p
            y := x + 5
            return y
        "#;

        let chunk = compile_chunk(code);
        let mut vm = VM::new();
        {
            let mut globals = vm.globals.lock().unwrap();
            globals.define(
                "async_sleep".to_string(),
                Value::NativeFunction("async_sleep".to_string()),
            );
        }

        // Should suspend on await
        let result = vm.execute_until_suspend(chunk).expect("execution should work");
        match result {
            VmExecutionResult::Suspended { context_id } => {
                // Resume with scheduler
                vm.run_scheduler_until_complete(50).expect("scheduler should complete");
                assert!(!vm.has_execution_context(context_id));
            }
            VmExecutionResult::Completed => panic!("expected suspension at await"),
        }
    }

    #[test]
    fn test_cooperative_default_preserves_backward_compat_with_non_async_code() {
        // Even with cooperative mode enabled, non-async code should work fine
        let codes = vec![
            r#"return 42"#,
            r#"x := 1 + 2; return x"#,
            r#"arr := [1, 2, 3]; return arr[0]"#,
            r#"func add(a, b) { return a + b }; return add(2, 3)"#,
        ];

        for code in codes {
            let chunk = compile_chunk(code);
            let mut vm = VM::new();
            let result = vm.execute_until_suspend(chunk).expect("execution should not error");
            match result {
                VmExecutionResult::Completed => {
                    // Expected for all non-async code
                }
                VmExecutionResult::Suspended { .. } => {
                    panic!("Non-async code '{}' should not suspend", code)
                }
            }
        }
    }
}
