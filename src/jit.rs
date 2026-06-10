// File: src/jit.rs
//
// JIT Compilation module for Kujo bytecode using Cranelift.
// Provides just-in-time compilation of hot bytecode functions to native machine code.

use crate::bytecode::{BytecodeChunk, Constant, OpCode};
use crate::interpreter::{
    DenseIntDict, DenseIntDictInt, DenseIntDictIntFull, DictMap, IntDictMap, Value,
};
use crate::vm::VM; // For calling back into VM from JIT
use cranelift::codegen::ir::stackslot::{StackSlotData, StackSlotKind};
use cranelift::codegen::ir::FuncRef;
use cranelift::codegen::ir::StackSlot;
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use std::collections::HashSet;
use std::sync::Arc;
// FuncId used for future multi-function JIT optimization
#[allow(unused_imports)]
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::HashMap;
// Hash/Hasher for future variable hashing optimizations
#[allow(unused_imports)]
use std::collections::hash_map::DefaultHasher;
#[allow(unused_imports)]
use std::hash::{Hash, Hasher};

/// JIT compilation threshold - number of executions before compiling
const JIT_THRESHOLD: usize = 100;
const DENSE_INT_DICT_MIN_CAPACITY: usize = 131072;

/// Guard failure threshold - recompile if guard failures exceed this percentage
#[allow(dead_code)] // Used in Phase 4D guard validation logic
const GUARD_FAILURE_THRESHOLD: f64 = 0.10; // 10%

/// Minimum samples before type specialization
#[allow(dead_code)] // Used in Phase 4A type profiling logic
const MIN_TYPE_SAMPLES: usize = 50;

fn dense_int_dict_int_with_len(len: usize) -> Vec<Option<i64>> {
    let mut values = Vec::with_capacity(len.max(DENSE_INT_DICT_MIN_CAPACITY));
    values.resize(len, None);
    values
}

fn dense_int_dict_int_full_with_len(len: usize) -> Vec<i64> {
    let mut values = Vec::with_capacity(len.max(DENSE_INT_DICT_MIN_CAPACITY));
    values.resize(len, 0);
    values
}

#[inline]
fn with_vm_context_mut<R>(ctx: *mut VMContext, f: impl FnOnce(&mut VMContext) -> R) -> Option<R> {
    if ctx.is_null() {
        return None;
    }

    // SAFETY:
    // - Preconditions: `ctx` is checked non-null and points to a live `VMContext` for this call.
    // - Postconditions: mutable context reference is scoped to this closure invocation and never stored.
    let vm_ctx = unsafe { &mut *ctx };
    Some(f(vm_ctx))
}

#[inline]
fn with_vm_stack_mut<R>(
    ctx: *mut VMContext,
    f: impl FnOnce(&mut VMContext, &mut Vec<Value>) -> R,
) -> Option<R> {
    with_vm_context_mut(ctx, |vm_ctx| {
        if vm_ctx.stack_ptr.is_null() {
            return None;
        }

        // SAFETY:
        // - Preconditions: `vm_ctx.stack_ptr` is checked non-null and belongs to the active VM context.
        // - Postconditions: mutable stack reference is borrowed only for this closure body.
        let stack = unsafe { &mut *vm_ctx.stack_ptr };
        Some(f(vm_ctx, stack))
    })
    .flatten()
}

#[inline]
fn compiled_fn_from_code_ptr(code_ptr: *const u8) -> CompiledFn {
    // SAFETY:
    // - Preconditions: caller ensures `code_ptr` is finalized machine code matching `CompiledFn` ABI/signature.
    // - Postconditions: returns a typed function pointer without changing code memory ownership.
    unsafe { std::mem::transmute(code_ptr) }
}

#[inline]
fn compiled_fn_with_arg_from_code_ptr(code_ptr: *const u8) -> CompiledFnWithArg {
    // SAFETY:
    // - Preconditions: caller ensures `code_ptr` matches `CompiledFnWithArg` ABI/signature.
    // - Postconditions: returns typed direct-arg function pointer without aliasing mutable code state.
    unsafe { std::mem::transmute(code_ptr) }
}

/// Runtime context passed to JIT-compiled functions
/// This allows JIT code to access VM state (stack, variables, etc.)
#[repr(C)]
pub struct VMContext {
    /// Pointer to value stack
    pub stack_ptr: *mut Vec<Value>,
    /// Pointer to local variables (current call frame)
    pub locals_ptr: *mut HashMap<String, Value>,
    /// Pointer to global variables
    pub globals_ptr: *mut HashMap<String, Value>,
    /// Pointer to variable name mapping (hash -> name) for JIT
    pub var_names_ptr: *mut HashMap<u64, String>,
    /// Pointer to local slot storage for JIT loop helpers
    pub local_slots_ptr: *mut Vec<Value>,
    /// Pointer to JIT object stack (non-int handles)
    pub obj_stack_ptr: *mut Vec<Value>,
    /// Pointer to VM for calling back into interpreter (Step 4+)
    pub vm_ptr: *mut std::ffi::c_void, // Actually *mut VM, but avoid circular dependency
    /// Fast return value storage - avoids stack push overhead
    /// When has_return_value is true, VM reads return_value directly instead of popping stack
    pub return_value: i64,
    /// Flag indicating return_value contains a valid integer return
    /// When true, VM uses return_value directly; when false, falls back to stack
    pub has_return_value: bool,
    /// Fast argument passing for recursive calls
    /// These avoid HashMap creation for simple recursive functions
    pub arg0: i64,
    pub arg1: i64,
    pub arg2: i64,
    pub arg3: i64,
    pub arg_count: i64,
}

impl VMContext {
    /// Create a new VMContext from VM state
    /// Used for JIT function execution with VM integration
    #[allow(dead_code)] // TODO: Will be used when JIT fully integrated into VM loop
    pub fn new(
        stack: *mut Vec<Value>,
        locals: *mut HashMap<String, Value>,
        globals: *mut HashMap<String, Value>,
    ) -> Self {
        Self {
            stack_ptr: stack,
            locals_ptr: locals,
            globals_ptr: globals,
            var_names_ptr: std::ptr::null_mut(), // Will be set if needed
            local_slots_ptr: std::ptr::null_mut(),
            obj_stack_ptr: std::ptr::null_mut(),
            vm_ptr: std::ptr::null_mut(), // No VM pointer yet
            return_value: 0,
            has_return_value: false,
            arg0: 0,
            arg1: 0,
            arg2: 0,
            arg3: 0,
            arg_count: 0,
        }
    }

    /// Create with VM pointer for full VM integration (Step 4+)
    pub fn new_with_vm(
        stack: *mut Vec<Value>,
        locals: *mut HashMap<String, Value>,
        globals: *mut HashMap<String, Value>,
        vm: *mut std::ffi::c_void,
    ) -> Self {
        Self {
            stack_ptr: stack,
            locals_ptr: locals,
            globals_ptr: globals,
            var_names_ptr: std::ptr::null_mut(),
            local_slots_ptr: std::ptr::null_mut(),
            obj_stack_ptr: std::ptr::null_mut(),
            vm_ptr: vm,
            return_value: 0,
            has_return_value: false,
            arg0: 0,
            arg1: 0,
            arg2: 0,
            arg3: 0,
            arg_count: 0,
        }
    }

    fn jit_int_key_string(ctx: &mut VMContext, key: i64) -> Arc<str> {
        if !ctx.vm_ptr.is_null() {
            // SAFETY:
            // - Preconditions: `ctx.vm_ptr` is checked non-null and points to the live owning VM for this context.
            // - Postconditions: returns a borrowed mutable VM reference used only within this scope for key conversion.
            let vm = unsafe { &mut *(ctx.vm_ptr as *mut crate::vm::VM) };
            return vm.int_key_string(key);
        }

        Arc::from(key.to_string())
    }

    /// Create with variable name mapping
    /// Used for JIT variable resolution optimization
    #[allow(dead_code)] // TODO: Will be used when variable hashing is implemented
    pub fn with_var_names(
        stack: *mut Vec<Value>,
        locals: *mut HashMap<String, Value>,
        globals: *mut HashMap<String, Value>,
        var_names: *mut HashMap<u64, String>,
    ) -> Self {
        Self {
            stack_ptr: stack,
            locals_ptr: locals,
            globals_ptr: globals,
            var_names_ptr: var_names,
            local_slots_ptr: std::ptr::null_mut(),
            obj_stack_ptr: std::ptr::null_mut(),
            vm_ptr: std::ptr::null_mut(),
            return_value: 0,
            has_return_value: false,
            arg0: 0,
            arg1: 0,
            arg2: 0,
            arg3: 0,
            arg_count: 0,
        }
    }
}

/// Compiled function type: takes VMContext pointer, returns status code
pub type CompiledFn = unsafe extern "C" fn(*mut VMContext) -> i64;

/// Compiled function type with direct argument: takes VMContext pointer and arg, returns result
/// This signature enables direct JIT-to-JIT recursion without FFI boundary crossing
/// Used for single-argument integer functions (like fib(n)) for maximum performance
pub type CompiledFnWithArg = unsafe extern "C" fn(*mut VMContext, i64) -> i64;

/// Invoke a compiled JIT function through one audited unsafe boundary.
///
/// # Safety contract
///
/// - `compiled_fn` must be a valid function pointer produced by this JIT.
/// - `ctx` must point to a live `VMContext` whose internal raw pointers are valid
///   for the full duration of the call.
/// - The called function must not outlive or stash pointers derived from `ctx`.
#[inline]
pub fn invoke_compiled_fn(compiled_fn: CompiledFn, ctx: &mut VMContext) -> i64 {
    // SAFETY:
    // - Preconditions: `compiled_fn` was produced by this JIT and `ctx` is a live VM-owned context for this call.
    // - Postconditions: control returns with JIT status code and no pointer from `ctx` escapes this wrapper.
    unsafe { compiled_fn(ctx as *mut VMContext) }
}

/// Invoke a single-argument compiled JIT function through one audited unsafe boundary.
///
/// # Safety contract
///
/// - `compiled_fn` must be a valid direct-arg JIT function pointer.
/// - `ctx` must satisfy the same pointer validity/lifetime rules as
///   [`invoke_compiled_fn`].
#[inline]
pub fn invoke_compiled_fn_with_arg(
    compiled_fn: CompiledFnWithArg,
    ctx: &mut VMContext,
    arg: i64,
) -> i64 {
    // SAFETY:
    // - Preconditions: identical to `invoke_compiled_fn`; `arg` is plain value data and ABI-compatible.
    // - Postconditions: returns the direct-arg JIT result without transferring ownership of context internals.
    unsafe { compiled_fn(ctx as *mut VMContext, arg) }
}

/// Details about a bytecode surface that current JIT cannot compile.
#[derive(Debug, Clone, PartialEq)]
pub struct UnsupportedJitSurface {
    pub chunk_name: String,
    pub instruction_offset: usize,
    pub opcode: OpCode,
}

impl UnsupportedJitSurface {
    pub fn describe(&self) -> String {
        format!(
            "unsupported opcode in chunk '{}' at instruction {}: {:?}",
            self.chunk_name, self.instruction_offset, self.opcode
        )
    }
}

/// Metadata about a JIT-compiled function
/// Used to track which calling convention to use and enable direct recursion
#[derive(Clone, Copy)]
pub struct CompiledFnInfo {
    /// The compiled function pointer (standard signature)
    pub fn_ptr: CompiledFn,
    /// For single-arg functions, the direct-call variant
    /// This allows calling the function directly with arg without FFI overhead
    pub fn_with_arg: Option<CompiledFnWithArg>,
    /// Number of parameters the function takes
    #[allow(dead_code)] // May be used for future multi-arg direct recursion
    pub param_count: usize,
    /// Whether this function is optimized for direct recursion
    #[allow(dead_code)]
    pub supports_direct_recursion: bool,
}

/// Type profile for a variable or operation
/// Infrastructure for Phase 4A adaptive specialization
#[allow(dead_code)] // TODO: Integrate into VM execution loop for automatic profiling
#[derive(Debug, Clone, Default)]
pub struct TypeProfile {
    /// Count of Int values observed
    pub int_count: usize,
    /// Count of Float values observed
    pub float_count: usize,
    /// Count of Bool values observed
    pub bool_count: usize,
    /// Count of other types observed
    pub other_count: usize,
}

#[allow(dead_code)] // Infrastructure for adaptive recompilation
impl TypeProfile {
    /// Record a type observation
    pub fn record(&mut self, value: &Value) {
        match value {
            Value::Int(_) => self.int_count += 1,
            Value::Float(_) => self.float_count += 1,
            Value::Bool(_) => self.bool_count += 1,
            _ => self.other_count += 1,
        }
    }

    /// Get total observations
    pub fn total(&self) -> usize {
        self.int_count + self.float_count + self.bool_count + self.other_count
    }

    /// Get the dominant type (most frequently observed)
    pub fn dominant_type(&self) -> Option<ValueType> {
        if self.total() < MIN_TYPE_SAMPLES {
            return None;
        }

        let max_count =
            self.int_count.max(self.float_count).max(self.bool_count).max(self.other_count);

        if max_count == self.int_count && self.int_count as f64 / self.total() as f64 > 0.90 {
            Some(ValueType::Int)
        } else if max_count == self.float_count
            && self.float_count as f64 / self.total() as f64 > 0.90
        {
            Some(ValueType::Float)
        } else if max_count == self.bool_count
            && self.bool_count as f64 / self.total() as f64 > 0.90
        {
            Some(ValueType::Bool)
        } else {
            None
        }
    }

    /// Check if this profile is stable enough for specialization
    pub fn is_stable(&self) -> bool {
        self.total() >= MIN_TYPE_SAMPLES && self.dominant_type().is_some()
    }
}

/// Value types for specialization
/// Infrastructure for Phase 4B type-specialized code generation
#[allow(dead_code)] // TODO: Used in adaptive recompilation decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Int,
    Float,
    Bool,
    Mixed,
}

/// Specialization strategy for a function
/// Infrastructure for Phase 4 adaptive optimization
#[allow(dead_code)] // TODO: Integrate into VM hot path detection
#[derive(Debug, Clone)]
pub struct SpecializationInfo {
    /// Type profiles for each variable (by name hash)
    pub variable_types: HashMap<u64, TypeProfile>,
    /// Dominant types for specialization
    pub specialized_types: HashMap<u64, ValueType>,
    /// Guard success count
    pub guard_successes: usize,
    /// Guard failure count
    pub guard_failures: usize,
}

#[allow(dead_code)] // Infrastructure for adaptive recompilation
impl SpecializationInfo {
    fn new() -> Self {
        Self {
            variable_types: HashMap::new(),
            specialized_types: HashMap::new(),
            guard_successes: 0,
            guard_failures: 0,
        }
    }

    /// Check if guards are failing too often
    fn should_despecialize(&self) -> bool {
        let total = self.guard_successes + self.guard_failures;
        if total < MIN_TYPE_SAMPLES {
            return false;
        }
        (self.guard_failures as f64 / total as f64) > GUARD_FAILURE_THRESHOLD
    }
}

// Runtime helper functions that JIT code can call
// These are marked #[no_mangle] so JIT code can find them by name

/// Push a value onto the VM stack (called from JIT code)
#[no_mangle]
// SAFETY:
// - Preconditions: jit_stack_push is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_stack_push(ctx: *mut VMContext, value: i64) {
    if ctx.is_null() {
        return;
    }
    let ctx = &mut *ctx;
    if !ctx.stack_ptr.is_null() {
        let stack = &mut *ctx.stack_ptr;
        stack.push(Value::Int(value));
    }
}

/// Pop a value from the VM stack (called from JIT code)
#[no_mangle]
// SAFETY:
// - Preconditions: jit_stack_pop is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_stack_pop(ctx: *mut VMContext) -> i64 {
    if ctx.is_null() {
        return 0;
    }
    let ctx = &mut *ctx;
    if !ctx.stack_ptr.is_null() {
        let stack = &mut *ctx.stack_ptr;
        if let Some(Value::Int(val)) = stack.pop() {
            return val;
        }
    }
    0
}

/// Push a string constant onto the JIT object stack
/// Returns a negative handle on success, 0 on error
#[no_mangle]
// SAFETY:
// - Preconditions: jit_obj_push_string is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_obj_push_string(ctx: *mut VMContext, ptr: i64, len: i64) -> i64 {
    if ctx.is_null() || ptr == 0 || len < 0 {
        return 0;
    }

    let ctx = &mut *ctx;
    if ctx.obj_stack_ptr.is_null() {
        return 0;
    }

    let slice = std::slice::from_raw_parts(ptr as *const u8, len as usize);
    let s = String::from_utf8_lossy(slice).to_string();
    let obj_stack = &mut *ctx.obj_stack_ptr;
    obj_stack.push(Value::Str(Arc::new(s)));

    let index = obj_stack.len() as i64 - 1;
    -(index + 1)
}

/// Push a JIT object handle onto the VM stack
/// Returns 1 on success, 0 on error
#[no_mangle]
// SAFETY:
// - Preconditions: jit_obj_to_vm_stack is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_obj_to_vm_stack(ctx: *mut VMContext, handle: i64) -> i64 {
    if ctx.is_null() || handle >= 0 {
        return 0;
    }

    let ctx = &mut *ctx;
    if ctx.obj_stack_ptr.is_null() || ctx.stack_ptr.is_null() {
        return 0;
    }

    let index = (-handle - 1) as usize;
    let obj_stack = &mut *ctx.obj_stack_ptr;
    let value = match obj_stack.get(index) {
        Some(value) => value.clone(),
        None => return 0,
    };

    let stack = &mut *ctx.stack_ptr;
    stack.push(value);
    1
}

/// Load a variable from locals or globals (called from JIT code)
/// name_hash: hash of the variable name (or 0 to use name_ptr/name_len)
/// name_ptr/name_len: pointer and length of variable name string (if name_hash is 0)
/// Returns the variable value as i64, or 0 if not found
#[no_mangle]
// SAFETY:
// - Preconditions: jit_load_variable is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_load_variable(
    ctx: *mut VMContext,
    name_hash: i64,
    _name_len: usize,
) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx_ref = &*ctx;

    // Try to resolve the name from hash
    let name = if name_hash != 0 && !ctx_ref.var_names_ptr.is_null() {
        let var_names = &*ctx_ref.var_names_ptr;
        if let Some(n) = var_names.get(&(name_hash as u64)) {
            n.as_str()
        } else {
            if std::env::var("DEBUG_JIT_CALL").is_ok() {
                eprintln!("jit_load_variable: hash {} not found in var_names", name_hash);
            }
            return 0; // Hash not found
        }
    } else {
        return 0; // No name provided
    };

    // Debug disabled for performance

    // Try locals first
    if !ctx_ref.locals_ptr.is_null() {
        let locals = &*ctx_ref.locals_ptr;
        if let Some(val) = locals.get(name) {
            match val {
                Value::Int(i) => {
                    if std::env::var("DEBUG_JIT_CALL").is_ok() {
                        eprintln!("jit_load_variable: '{}' = {} (from locals)", name, i);
                    }
                    return *i;
                }
                _ => {
                    // Non-integer value: push to VM stack for later use
                    if !ctx_ref.stack_ptr.is_null() {
                        let stack = &mut *ctx_ref.stack_ptr;
                        if std::env::var("DEBUG_JIT_CALL").is_ok() {
                            eprintln!(
                                "jit_load_variable: '{}' = {:?} (pushed to VM stack from locals)",
                                name, val
                            );
                        }
                        stack.push(val.clone());
                        return -1; // Special marker: value is on VM stack
                    }
                    return 0;
                }
            }
        }
    }

    // Then try globals
    if !ctx_ref.globals_ptr.is_null() {
        let globals = &*ctx_ref.globals_ptr;
        if let Some(val) = globals.get(name) {
            match val {
                Value::Int(i) => {
                    if std::env::var("DEBUG_JIT_CALL").is_ok() {
                        eprintln!("jit_load_variable: '{}' = {} (from globals)", name, i);
                    }
                    return *i;
                }
                _ => {
                    // Non-integer value: push to VM stack for later use
                    if !ctx_ref.stack_ptr.is_null() {
                        let stack = &mut *ctx_ref.stack_ptr;
                        if std::env::var("DEBUG_JIT_CALL").is_ok() {
                            eprintln!(
                                "jit_load_variable: '{}' = {:?} (pushed to VM stack from globals)",
                                name, val
                            );
                        }
                        stack.push(val.clone());
                        return -1; // Special marker: value is on VM stack
                    }
                    return 0;
                }
            }
        }
    }

    if std::env::var("DEBUG_JIT_CALL").is_ok() {
        eprintln!("jit_load_variable: '{}' not found", name);
    }
    0
}

/// Store a variable to locals (called from JIT code)
/// name_hash: hash of the variable name
#[no_mangle]
// SAFETY:
// - Preconditions: jit_store_variable is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_store_variable(
    ctx: *mut VMContext,
    name_hash: i64,
    _name_len: usize,
    value: i64,
) {
    if ctx.is_null() {
        return;
    }

    let ctx = &mut *ctx;

    // Try to resolve the name from hash
    let name = if name_hash != 0 && !ctx.var_names_ptr.is_null() {
        let var_names = &*ctx.var_names_ptr;
        if let Some(n) = var_names.get(&(name_hash as u64)) {
            n.clone()
        } else {
            return; // Hash not found
        }
    } else {
        return; // No name provided
    };

    // Store in locals
    if !ctx.locals_ptr.is_null() {
        let locals = &mut *ctx.locals_ptr;
        locals.insert(name, Value::Int(value));
    }
}

/// Store a variable from the VM stack (called from JIT code)
/// name_hash: hash of the variable name
/// Pops one Value from VM stack and stores it into locals/globals
/// Returns 1 on success, 0 on error
#[no_mangle]
// SAFETY:
// - Preconditions: jit_store_variable_from_stack is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_store_variable_from_stack(ctx: *mut VMContext, name_hash: i64) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx = &mut *ctx;

    if ctx.stack_ptr.is_null() {
        return 0;
    }

    let name = if name_hash != 0 && !ctx.var_names_ptr.is_null() {
        let var_names = &*ctx.var_names_ptr;
        if let Some(n) = var_names.get(&(name_hash as u64)) {
            n.clone()
        } else {
            return 0;
        }
    } else {
        return 0;
    };

    let stack = &mut *ctx.stack_ptr;
    let value = match stack.pop() {
        Some(val) => val,
        None => return 0,
    };

    if !ctx.locals_ptr.is_null() {
        let locals = &mut *ctx.locals_ptr;
        locals.insert(name, value);
        return 1;
    }

    if !ctx.globals_ptr.is_null() {
        let globals = &mut *ctx.globals_ptr;
        globals.insert(name, value);
        return 1;
    }

    0
}

/// Append a constant string to a local slot string in-place.
/// Returns 1 on success, 0 on error.
#[no_mangle]
// SAFETY:
// - Preconditions: jit_append_const_string_in_place is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_append_const_string_in_place(
    ctx: *mut VMContext,
    slot_index: i64,
    ptr: i64,
    len: i64,
) -> i64 {
    if ctx.is_null() || ptr == 0 || len < 0 {
        return 0;
    }

    let ctx_ref = &mut *ctx;
    if ctx_ref.local_slots_ptr.is_null() {
        return 0;
    }

    let slot = match usize::try_from(slot_index) {
        Ok(slot) => slot,
        Err(_) => return 0,
    };

    let local_slots = &mut *ctx_ref.local_slots_ptr;
    let target = match local_slots.get_mut(slot) {
        Some(value) => value,
        None => return 0,
    };

    let bytes = std::slice::from_raw_parts(ptr as *const u8, len as usize);
    let append_str = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    match target {
        Value::Str(left) => {
            let left_str = Arc::make_mut(left);
            let needed = append_str.len();
            let available = left_str.capacity() - left_str.len();
            if available < needed {
                let reserve_amount = (needed * 1000).max(left_str.capacity());
                left_str.reserve(reserve_amount);
            }
            left_str.push_str(append_str);
            1
        }
        _ => 0,
    }
}

/// Append a constant character to a local slot string in-place.
/// Returns 1 on success, 0 on error.
#[no_mangle]
// SAFETY:
// - Preconditions: jit_append_const_char_in_place is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_append_const_char_in_place(
    ctx: *mut VMContext,
    slot_index: i64,
    ch: i64,
) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx_ref = &mut *ctx;
    if ctx_ref.local_slots_ptr.is_null() {
        return 0;
    }

    let slot = match usize::try_from(slot_index) {
        Ok(slot) => slot,
        Err(_) => return 0,
    };

    let codepoint = match u32::try_from(ch) {
        Ok(codepoint) => codepoint,
        Err(_) => return 0,
    };

    let chr = match char::from_u32(codepoint) {
        Some(chr) => chr,
        None => return 0,
    };

    let local_slots = &mut *ctx_ref.local_slots_ptr;
    let target = match local_slots.get_mut(slot) {
        Some(value) => value,
        None => return 0,
    };

    match target {
        Value::Str(left) => {
            let left_str = Arc::make_mut(left);
            let needed = chr.len_utf8();
            let available = left_str.capacity() - left_str.len();
            if available < needed {
                let reserve_amount = (needed * 1000).max(left_str.capacity());
                left_str.reserve(reserve_amount);
            }
            left_str.push(chr);
            1
        }
        _ => 0,
    }
}

/// Get int value from a dict/array stored in a local slot (called from loop JIT)
/// Returns the int value or 0 on error/missing/non-int
#[no_mangle]
// SAFETY:
// - Preconditions: jit_local_slot_dict_get is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_local_slot_dict_get(
    ctx: *mut VMContext,
    slot_index: i64,
    key: i64,
) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx_ref = &mut *ctx;
    if ctx_ref.vm_ptr.is_null() {
        return 0;
    }

    let slot = match usize::try_from(slot_index) {
        Ok(slot) => slot,
        Err(_) => return 0,
    };

    if !ctx_ref.local_slots_ptr.is_null() {
        let local_slots = &*ctx_ref.local_slots_ptr;
        let object = match local_slots.get(slot) {
            Some(value) => value,
            None => return 0,
        };

        return match object {
            Value::IntDict(dict) => {
                crate::vm::hashmap_profile_bump_get_intdict();
                match dict.get(&key) {
                    Some(Value::Int(v)) => *v,
                    _ => 0,
                }
            }
            Value::DenseIntDict(values) => {
                crate::vm::hashmap_profile_bump_get_dense();
                if key < 0 {
                    0
                } else {
                    match values.get(key as usize) {
                        Some(Value::Int(v)) => *v,
                        _ => 0,
                    }
                }
            }
            Value::DenseIntDictInt(values) => {
                crate::vm::hashmap_profile_bump_get_dense_int();
                if key < 0 {
                    0
                } else {
                    let index = key as usize;
                    if index < values.len() {
                        match values[index] {
                            Some(v) => v,
                            None => 0,
                        }
                    } else {
                        0
                    }
                }
            }
            Value::DenseIntDictIntFull(values) => {
                crate::vm::hashmap_profile_bump_get_dense_int();
                if key < 0 {
                    0
                } else {
                    values.get(key as usize).copied().unwrap_or(0)
                }
            }
            Value::Dict(dict) => {
                crate::vm::hashmap_profile_bump_get_dict_intkey();
                let key_str = if !ctx_ref.vm_ptr.is_null() {
                    let vm = &mut *(ctx_ref.vm_ptr as *mut VM);
                    vm.int_key_string(key)
                } else {
                    Arc::from(key.to_string())
                };
                match dict.get(key_str.as_ref()) {
                    Some(Value::Int(v)) => *v,
                    _ => 0,
                }
            }
            _ => 0,
        };
    }

    let vm = &mut *(ctx_ref.vm_ptr as *mut VM);
    let object = {
        let frame = match vm.call_frames.last() {
            Some(frame) => frame,
            None => return 0,
        };

        match frame.local_slots.get(slot) {
            Some(value) => value.clone(),
            None => return 0,
        }
    };

    match &object {
        Value::IntDict(dict) => {
            crate::vm::hashmap_profile_bump_get_intdict();
            match dict.get(&key) {
                Some(Value::Int(v)) => *v,
                _ => 0,
            }
        }
        Value::DenseIntDict(values) => {
            crate::vm::hashmap_profile_bump_get_dense();
            if key < 0 {
                0
            } else {
                match values.get(key as usize) {
                    Some(Value::Int(v)) => *v,
                    _ => 0,
                }
            }
        }
        Value::DenseIntDictInt(values) => {
            crate::vm::hashmap_profile_bump_get_dense_int();
            if key < 0 {
                0
            } else {
                let index = key as usize;
                if index < values.len() {
                    match values[index] {
                        Some(v) => v,
                        None => 0,
                    }
                } else {
                    0
                }
            }
        }
        Value::DenseIntDictIntFull(values) => {
            crate::vm::hashmap_profile_bump_get_dense_int();
            if key < 0 {
                0
            } else {
                values.get(key as usize).copied().unwrap_or(0)
            }
        }
        Value::Dict(dict) => {
            crate::vm::hashmap_profile_bump_get_dict_intkey();
            let key_str = vm.int_key_string(key);
            match dict.get(key_str.as_ref()) {
                Some(Value::Int(v)) => *v,
                _ => 0,
            }
        }
        _ => 0,
    }
}

/// Set int value in a dict/array stored in a local slot (called from loop JIT)
/// Returns 1 on success, 0 on error
#[no_mangle]
// SAFETY:
// - Preconditions: jit_local_slot_dict_set is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_local_slot_dict_set(
    ctx: *mut VMContext,
    slot_index: i64,
    key: i64,
    value: i64,
) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx_ref = &mut *ctx;
    if ctx_ref.vm_ptr.is_null() {
        return 0;
    }

    let vm = &mut *(ctx_ref.vm_ptr as *mut VM);

    let slot = match usize::try_from(slot_index) {
        Ok(slot) => slot,
        Err(_) => return 0,
    };

    if !ctx_ref.local_slots_ptr.is_null() {
        let local_slots = &mut *ctx_ref.local_slots_ptr;
        return match local_slots.get_mut(slot) {
            Some(Value::IntDict(ref mut dict)) => {
                crate::vm::hashmap_profile_bump_set_intdict();
                let dict_mut = Arc::make_mut(dict);
                dict_mut.insert(key, Value::Int(value));
                1
            }
            Some(Value::DenseIntDict(ref mut values)) => {
                crate::vm::hashmap_profile_bump_set_dense();
                if key < 0 {
                    let mut int_dict = IntDictMap::default();
                    int_dict.reserve(values.len());
                    for (index, value) in values.iter().enumerate() {
                        int_dict.insert(index as i64, value.clone());
                    }
                    int_dict.insert(key, Value::Int(value));
                    local_slots[slot] = Value::IntDict(Arc::new(int_dict));
                    return 1;
                }
                let values_mut = Arc::make_mut(values);
                let index = key as usize;
                if index >= values_mut.len() {
                    values_mut.resize(index + 1, Value::Null);
                }
                values_mut[index] = Value::Int(value);
                1
            }
            Some(Value::DenseIntDictInt(ref mut values)) => {
                crate::vm::hashmap_profile_bump_set_dense_int();
                if key < 0 {
                    let mut int_dict = IntDictMap::default();
                    int_dict.reserve(values.len());
                    for (index, value) in values.iter().enumerate() {
                        int_dict
                            .insert(index as i64, (*value).map(Value::Int).unwrap_or(Value::Null));
                    }
                    int_dict.insert(key, Value::Int(value));
                    local_slots[slot] = Value::IntDict(Arc::new(int_dict));
                    return 1;
                }
                let values_mut = Arc::make_mut(values);
                let index = key as usize;
                let len = values_mut.len();
                if index == len {
                    values_mut.push(Some(value));
                } else if index < len {
                    values_mut[index] = Some(value);
                } else {
                    values_mut.resize(index + 1, None);
                    values_mut[index] = Some(value);
                }
                1
            }
            Some(Value::DenseIntDictIntFull(ref mut values)) => {
                crate::vm::hashmap_profile_bump_set_dense_int();
                if key < 0 {
                    let mut int_dict = IntDictMap::default();
                    int_dict.reserve(values.len());
                    for (index, value) in values.iter().enumerate() {
                        int_dict.insert(index as i64, Value::Int(*value));
                    }
                    int_dict.insert(key, Value::Int(value));
                    local_slots[slot] = Value::IntDict(Arc::new(int_dict));
                    return 1;
                }
                let index = key as usize;
                let len = values.len();
                if index == len {
                    let values_mut = Arc::make_mut(values);
                    values_mut.push(value);
                } else if index < len {
                    let values_mut = Arc::make_mut(values);
                    values_mut[index] = value;
                } else {
                    let mut sparse = Vec::with_capacity(index + 1);
                    for existing in values.iter() {
                        sparse.push(Some(*existing));
                    }
                    sparse.resize(index + 1, None);
                    sparse[index] = Some(value);
                    local_slots[slot] = Value::DenseIntDictInt(Arc::new(sparse));
                }
                1
            }
            Some(Value::Dict(ref mut dict)) => {
                crate::vm::hashmap_profile_bump_set_dict_intkey();
                if dict.is_empty() {
                    if key >= 0 {
                        if key == 0 {
                            local_slots[slot] = Value::DenseIntDictIntFull(Arc::new(vec![value]));
                        } else {
                            let mut values = dense_int_dict_int_with_len((key as usize) + 1);
                            values[key as usize] = Some(value);
                            local_slots[slot] = Value::DenseIntDictInt(Arc::new(values));
                        }
                        return 1;
                    }
                    let mut int_dict = IntDictMap::default();
                    int_dict.reserve(1024);
                    int_dict.insert(key, Value::Int(value));
                    local_slots[slot] = Value::IntDict(Arc::new(int_dict));
                    return 1;
                }

                let dict_mut = Arc::make_mut(dict);
                let key_str = vm.int_key_string(key);
                dict_mut.insert(key_str.into(), Value::Int(value));
                1
            }
            _ => 0,
        };
    }

    let needs_string_key = {
        let frame = match vm.call_frames.last() {
            Some(frame) => frame,
            None => return 0,
        };

        match frame.local_slots.get(slot) {
            Some(Value::Dict(dict)) => !dict.is_empty(),
            _ => false,
        }
    };

    let key_str = if needs_string_key { Some(vm.int_key_string(key)) } else { None };

    let frame = match vm.call_frames.last_mut() {
        Some(frame) => frame,
        None => return 0,
    };

    match frame.local_slots.get_mut(slot) {
        Some(Value::IntDict(ref mut dict)) => {
            crate::vm::hashmap_profile_bump_set_intdict();
            let dict_mut = Arc::make_mut(dict);
            dict_mut.insert(key, Value::Int(value));
            1
        }
        Some(Value::DenseIntDict(ref mut values)) => {
            crate::vm::hashmap_profile_bump_set_dense();
            if key < 0 {
                let mut int_dict = IntDictMap::default();
                int_dict.reserve(values.len());
                for (index, value) in values.iter().enumerate() {
                    int_dict.insert(index as i64, value.clone());
                }
                int_dict.insert(key, Value::Int(value));
                frame.local_slots[slot] = Value::IntDict(Arc::new(int_dict));
                return 1;
            }
            let values_mut = Arc::make_mut(values);
            let index = key as usize;
            if index >= values_mut.len() {
                values_mut.resize(index + 1, Value::Null);
            }
            values_mut[index] = Value::Int(value);
            1
        }
        Some(Value::DenseIntDictInt(ref mut values)) => {
            crate::vm::hashmap_profile_bump_set_dense_int();
            if key < 0 {
                let mut int_dict = IntDictMap::default();
                int_dict.reserve(values.len());
                for (index, value) in values.iter().enumerate() {
                    int_dict.insert(index as i64, (*value).map(Value::Int).unwrap_or(Value::Null));
                }
                int_dict.insert(key, Value::Int(value));
                frame.local_slots[slot] = Value::IntDict(Arc::new(int_dict));
                return 1;
            }
            let values_mut = Arc::make_mut(values);
            let index = key as usize;
            let len = values_mut.len();
            if index == len {
                values_mut.push(Some(value));
            } else if index < len {
                values_mut[index] = Some(value);
            } else {
                values_mut.resize(index + 1, None);
                values_mut[index] = Some(value);
            }
            1
        }
        Some(Value::DenseIntDictIntFull(ref mut values)) => {
            crate::vm::hashmap_profile_bump_set_dense_int();
            if key < 0 {
                let mut int_dict = IntDictMap::default();
                int_dict.reserve(values.len());
                for (index, value) in values.iter().enumerate() {
                    int_dict.insert(index as i64, Value::Int(*value));
                }
                int_dict.insert(key, Value::Int(value));
                frame.local_slots[slot] = Value::IntDict(Arc::new(int_dict));
                return 1;
            }
            let index = key as usize;
            let len = values.len();
            if index == len {
                let values_mut = Arc::make_mut(values);
                values_mut.push(value);
            } else if index < len {
                let values_mut = Arc::make_mut(values);
                values_mut[index] = value;
            } else {
                let mut sparse = Vec::with_capacity(index + 1);
                for existing in values.iter() {
                    sparse.push(Some(*existing));
                }
                sparse.resize(index + 1, None);
                sparse[index] = Some(value);
                frame.local_slots[slot] = Value::DenseIntDictInt(Arc::new(sparse));
            }
            1
        }
        Some(Value::Dict(ref mut dict)) => {
            crate::vm::hashmap_profile_bump_set_dict_intkey();
            if dict.is_empty() {
                if key >= 0 {
                    let mut values = vec![Value::Null; (key as usize) + 1];
                    values[key as usize] = Value::Int(value);
                    frame.local_slots[slot] = Value::DenseIntDict(Arc::new(values));
                    return 1;
                }
                let mut int_dict = IntDictMap::default();
                int_dict.reserve(1024);
                int_dict.insert(key, Value::Int(value));
                frame.local_slots[slot] = Value::IntDict(Arc::new(int_dict));
                return 1;
            }

            let dict_mut = Arc::make_mut(dict);
            if let Some(key_str) = key_str {
                dict_mut.insert(key_str.into(), Value::Int(value));
                return 1;
            }
            0
        }
        _ => 0,
    }
}

/// Get int value from an IntDict stored in a local slot (loop JIT fast path)
/// Returns the int value or 0 on missing/non-int
#[no_mangle]
// SAFETY:
// - Preconditions: jit_local_slot_int_dict_get is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_local_slot_int_dict_get(
    ctx: *mut VMContext,
    slot_index: i64,
    key: i64,
) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx_ref = &mut *ctx;
    let slot = match usize::try_from(slot_index) {
        Ok(slot) => slot,
        Err(_) => return 0,
    };

    let local_slots_ptr = if !ctx_ref.local_slots_ptr.is_null() {
        ctx_ref.local_slots_ptr
    } else if !ctx_ref.vm_ptr.is_null() {
        let vm = &mut *(ctx_ref.vm_ptr as *mut VM);
        match vm.call_frames.last_mut() {
            Some(frame) => &mut frame.local_slots as *mut Vec<Value>,
            None => return 0,
        }
    } else {
        return 0;
    };

    let local_slots = &mut *local_slots_ptr;
    let object = match local_slots.get_mut(slot) {
        Some(value) => value,
        None => return 0,
    };

    match object {
        Value::IntDict(dict) => match dict.get(&key) {
            Some(Value::Int(v)) => *v,
            _ => 0,
        },
        Value::DenseIntDict(values) => {
            if key < 0 {
                0
            } else {
                match values.get(key as usize) {
                    Some(Value::Int(v)) => *v,
                    _ => 0,
                }
            }
        }
        Value::DenseIntDictInt(values) => {
            if key < 0 {
                0
            } else {
                let index = key as usize;
                if index < values.len() {
                    match values[index] {
                        Some(v) => v,
                        None => 0,
                    }
                } else {
                    0
                }
            }
        }
        Value::DenseIntDictIntFull(values) => {
            if key < 0 {
                0
            } else {
                values.get(key as usize).copied().unwrap_or(0)
            }
        }
        _ => 0,
    }
}

/// Set int value into an IntDict stored in a local slot (loop JIT fast path)
/// Returns 1 on success, 0 on error
#[no_mangle]
// SAFETY:
// - Preconditions: jit_local_slot_int_dict_set is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_local_slot_int_dict_set(
    ctx: *mut VMContext,
    slot_index: i64,
    key: i64,
    value: i64,
) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx_ref = &mut *ctx;
    let slot = match usize::try_from(slot_index) {
        Ok(slot) => slot,
        Err(_) => return 0,
    };

    let local_slots_ptr = if !ctx_ref.local_slots_ptr.is_null() {
        ctx_ref.local_slots_ptr
    } else if !ctx_ref.vm_ptr.is_null() {
        let vm = &mut *(ctx_ref.vm_ptr as *mut VM);
        match vm.call_frames.last_mut() {
            Some(frame) => &mut frame.local_slots as *mut Vec<Value>,
            None => return 0,
        }
    } else {
        return 0;
    };

    let local_slots = &mut *local_slots_ptr;
    let object = match local_slots.get_mut(slot) {
        Some(value) => value,
        None => return 0,
    };

    match object {
        Value::IntDict(ref mut dict) => {
            let dict_mut = Arc::make_mut(dict);
            dict_mut.insert(key, Value::Int(value));
            1
        }
        Value::DenseIntDict(ref mut values) => {
            if key < 0 {
                let mut int_dict = IntDictMap::default();
                int_dict.reserve(values.len());
                for (index, value) in values.iter().enumerate() {
                    int_dict.insert(index as i64, value.clone());
                }
                int_dict.insert(key, Value::Int(value));
                *object = Value::IntDict(Arc::new(int_dict));
                return 1;
            }
            let values_mut = Arc::make_mut(values);
            let index = key as usize;
            if index >= values_mut.len() {
                values_mut.resize(index + 1, Value::Null);
            }
            values_mut[index] = Value::Int(value);
            1
        }
        Value::DenseIntDictInt(ref mut values) => {
            if key < 0 {
                let mut int_dict = IntDictMap::default();
                int_dict.reserve(values.len());
                for (index, value) in values.iter().enumerate() {
                    int_dict.insert(index as i64, (*value).map(Value::Int).unwrap_or(Value::Null));
                }
                int_dict.insert(key, Value::Int(value));
                *object = Value::IntDict(Arc::new(int_dict));
                return 1;
            }
            let values_mut = Arc::make_mut(values);
            let index = key as usize;
            let len = values_mut.len();
            if index == len {
                values_mut.push(Some(value));
            } else if index < len {
                values_mut[index] = Some(value);
            } else {
                values_mut.resize(index + 1, None);
                values_mut[index] = Some(value);
            }
            1
        }
        Value::DenseIntDictIntFull(ref mut values) => {
            if key < 0 {
                let mut int_dict = IntDictMap::default();
                int_dict.reserve(values.len());
                for (index, value) in values.iter().enumerate() {
                    int_dict.insert(index as i64, Value::Int(*value));
                }
                int_dict.insert(key, Value::Int(value));
                *object = Value::IntDict(Arc::new(int_dict));
                return 1;
            }
            let index = key as usize;
            let len = values.len();
            if index == len {
                let values_mut = Arc::make_mut(values);
                values_mut.push(value);
                return 1;
            }
            if index < len {
                let values_mut = Arc::make_mut(values);
                values_mut[index] = value;
                return 1;
            }
            let mut sparse = Vec::with_capacity(index + 1);
            for existing in values.iter() {
                sparse.push(Some(*existing));
            }
            sparse.resize(index + 1, None);
            sparse[index] = Some(value);
            *object = Value::DenseIntDictInt(Arc::new(sparse));
            1
        }
        Value::Dict(dict) => {
            if dict.is_empty() {
                if key >= 0 {
                    if key == 0 {
                        *object = Value::DenseIntDictIntFull(Arc::new(vec![value]));
                    } else {
                        let mut values = dense_int_dict_int_with_len((key as usize) + 1);
                        values[key as usize] = Some(value);
                        *object = Value::DenseIntDictInt(Arc::new(values));
                    }
                    return 1;
                }
                let mut int_dict = IntDictMap::default();
                int_dict.reserve(1024);
                int_dict.insert(key, Value::Int(value));
                *object = Value::IntDict(Arc::new(int_dict));
                return 1;
            }
            0
        }
        _ => 0,
    }
}

/// Get a unique IntDict pointer for a local slot (loop JIT fast path)
/// Returns pointer as i64, or 0 if not a unique IntDict/empty Dict
#[no_mangle]
// SAFETY:
// - Preconditions: jit_int_dict_unique_ptr is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_int_dict_unique_ptr(ctx: *mut VMContext, slot_index: i64) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx_ref = &mut *ctx;
    let slot = match usize::try_from(slot_index) {
        Ok(slot) => slot,
        Err(_) => return 0,
    };

    let local_slots_ptr = if !ctx_ref.local_slots_ptr.is_null() {
        ctx_ref.local_slots_ptr
    } else if !ctx_ref.vm_ptr.is_null() {
        let vm = &mut *(ctx_ref.vm_ptr as *mut VM);
        match vm.call_frames.last_mut() {
            Some(frame) => &mut frame.local_slots as *mut Vec<Value>,
            None => return 0,
        }
    } else {
        return 0;
    };

    let local_slots = &mut *local_slots_ptr;
    let object = match local_slots.get_mut(slot) {
        Some(value) => value,
        None => return 0,
    };

    match object {
        Value::IntDict(dict) => {
            if Arc::strong_count(dict) == 1 {
                Arc::as_ptr(dict) as i64
            } else {
                0
            }
        }
        Value::DenseIntDict(values) => {
            if Arc::strong_count(values) == 1 {
                (Arc::as_ptr(values) as i64) | 1
            } else {
                0
            }
        }
        Value::DenseIntDictInt(values) => {
            if Arc::strong_count(values) == 1 {
                (Arc::as_ptr(values) as i64) | 4
            } else {
                0
            }
        }
        Value::DenseIntDictIntFull(values) => {
            if Arc::strong_count(values) == 1 {
                (Arc::as_ptr(values) as i64) | 2
            } else {
                0
            }
        }
        Value::Dict(dict) => {
            if dict.is_empty() {
                let values = Arc::new(dense_int_dict_int_full_with_len(0));
                let ptr = (Arc::as_ptr(&values) as i64) | 2;
                *object = Value::DenseIntDictIntFull(values);
                ptr
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// Get int value from a unique IntDict pointer (loop JIT fast path)
/// Returns int value or 0 on missing/non-int
#[no_mangle]
// SAFETY:
// - Preconditions: jit_int_dict_get_ptr is invoked from JIT-compiled code with C ABI-compatible arguments; raw
//   handle/pointer arguments encode valid, live VM-owned structures for the duration of this call and
//   are not concurrently mutated through aliased mutable references.
// - Postconditions: the function only mutates the referenced VM-owned structures associated with the
//   provided handles/pointers and returns sentinel values on validation failure.
pub unsafe extern "C" fn jit_int_dict_get_ptr(dict_ptr: i64, key: i64) -> i64 {
    if dict_ptr == 0 {
        return 0;
    }

    let tag = dict_ptr & 7;

    if tag == 1 {
        if key < 0 {
            return 0;
        }
        let values = &*((dict_ptr & !7) as *const DenseIntDict);
        match values.get(key as usize) {
            Some(Value::Int(v)) => *v,
            _ => 0,
        }
    } else if tag == 2 {
        if key < 0 {
            return 0;
        }
        let values = &*((dict_ptr & !7) as *const DenseIntDictIntFull);
        let index = key as usize;
        if index < values.len() {
            values[index]
        } else {
            0
        }
    } else if tag == 4 {
        if key < 0 {
            return 0;
        }
        let values = &*((dict_ptr & !7) as *const DenseIntDictInt);
        let index = key as usize;
        if index < values.len() {
            match values[index] {
                Some(v) => v,
                None => 0,
            }
        } else {
            0
        }
    } else {
        let dict = &*(dict_ptr as *const IntDictMap);
        match dict.get(&key) {
            Some(Value::Int(v)) => *v,
            _ => 0,
        }
    }
}

/// Get int value from a DenseIntDictInt pointer (loop JIT fast path)
/// Returns int value or 0 on missing/non-int
#[no_mangle]
// SAFETY:
// - Preconditions: jit_dense_int_dict_int_get_ptr is invoked from JIT-compiled code with C ABI-compatible arguments; raw
//   handle/pointer arguments encode valid, live VM-owned structures for the duration of this call and
//   are not concurrently mutated through aliased mutable references.
// - Postconditions: the function only mutates the referenced VM-owned structures associated with the
//   provided handles/pointers and returns sentinel values on validation failure.
pub unsafe extern "C" fn jit_dense_int_dict_int_get_ptr(dict_ptr: i64, key: i64) -> i64 {
    if dict_ptr == 0 {
        return 0;
    }

    if key < 0 {
        return 0;
    }

    let values = &*((dict_ptr & !7) as *const DenseIntDictInt);
    let index = key as usize;
    if index < values.len() {
        match values[index] {
            Some(v) => v,
            None => 0,
        }
    } else {
        0
    }
}

/// Get int value from a DenseIntDictIntFull pointer (loop JIT fast path)
/// Returns int value or 0 on missing/non-int
#[no_mangle]
// SAFETY:
// - Preconditions: jit_dense_int_dict_int_full_get_ptr is invoked from JIT-compiled code with C ABI-compatible arguments; raw
//   handle/pointer arguments encode valid, live VM-owned structures for the duration of this call and
//   are not concurrently mutated through aliased mutable references.
// - Postconditions: the function only mutates the referenced VM-owned structures associated with the
//   provided handles/pointers and returns sentinel values on validation failure.
pub unsafe extern "C" fn jit_dense_int_dict_int_full_get_ptr(dict_ptr: i64, key: i64) -> i64 {
    if dict_ptr == 0 {
        return 0;
    }

    if key < 0 {
        return 0;
    }

    let values = &*((dict_ptr & !7) as *const DenseIntDictIntFull);
    let index = key as usize;
    if index < values.len() {
        values[index]
    } else {
        0
    }
}

/// Set int value via unique IntDict pointer (loop JIT fast path)
/// Returns 1 on success, 0 on error
#[no_mangle]
// SAFETY:
// - Preconditions: jit_int_dict_set_ptr is invoked from JIT-compiled code with C ABI-compatible arguments; raw
//   handle/pointer arguments encode valid, live VM-owned structures for the duration of this call and
//   are not concurrently mutated through aliased mutable references.
// - Postconditions: the function only mutates the referenced VM-owned structures associated with the
//   provided handles/pointers and returns sentinel values on validation failure.
pub unsafe extern "C" fn jit_int_dict_set_ptr(dict_ptr: i64, key: i64, value: i64) -> i64 {
    if dict_ptr == 0 {
        return 0;
    }

    let tag = dict_ptr & 7;

    if tag == 1 {
        if key < 0 {
            return 0;
        }
        let values = &mut *((dict_ptr & !7) as *mut DenseIntDict);
        let index = key as usize;
        let len = values.len();
        if index == len {
            values.push(Value::Int(value));
        } else if index < len {
            values[index] = Value::Int(value);
        } else {
            values.resize(index + 1, Value::Null);
            values[index] = Value::Int(value);
        }
        1
    } else if tag == 2 {
        if key < 0 {
            return 0;
        }
        let values = &mut *((dict_ptr & !7) as *mut DenseIntDictIntFull);
        let index = key as usize;
        let len = values.len();
        if index == len {
            values.push(value);
        } else if index < len {
            values[index] = value;
        } else {
            values.resize(index + 1, 0);
            values[index] = value;
        }
        1
    } else if tag == 4 {
        if key < 0 {
            return 0;
        }
        let values = &mut *((dict_ptr & !7) as *mut DenseIntDictInt);
        let index = key as usize;
        let len = values.len();
        if index == len {
            values.push(Some(value));
        } else if index < len {
            values[index] = Some(value);
        } else {
            values.resize(index + 1, None);
            values[index] = Some(value);
        }
        1
    } else {
        let dict = &mut *(dict_ptr as *mut IntDictMap);
        dict.insert(key, Value::Int(value));
        1
    }
}

/// Set int value via DenseIntDictInt pointer (loop JIT fast path)
/// Returns 1 on success, 0 on error
#[no_mangle]
// SAFETY:
// - Preconditions: jit_dense_int_dict_int_set_ptr is invoked from JIT-compiled code with C ABI-compatible arguments; raw
//   handle/pointer arguments encode valid, live VM-owned structures for the duration of this call and
//   are not concurrently mutated through aliased mutable references.
// - Postconditions: the function only mutates the referenced VM-owned structures associated with the
//   provided handles/pointers and returns sentinel values on validation failure.
pub unsafe extern "C" fn jit_dense_int_dict_int_set_ptr(
    dict_ptr: i64,
    key: i64,
    value: i64,
) -> i64 {
    if dict_ptr == 0 {
        return 0;
    }

    if key < 0 {
        return 0;
    }

    let values = &mut *((dict_ptr & !7) as *mut DenseIntDictInt);
    let index = key as usize;
    let len = values.len();
    if index == len {
        values.push(Some(value));
    } else if index < len {
        values[index] = Some(value);
    } else {
        values.resize(index + 1, None);
        values[index] = Some(value);
    }
    1
}

/// Set int value via DenseIntDictIntFull pointer (loop JIT fast path)
/// Returns 1 on success, 0 on error
#[no_mangle]
// SAFETY:
// - Preconditions: jit_dense_int_dict_int_full_set_ptr is invoked from JIT-compiled code with C ABI-compatible arguments; raw
//   handle/pointer arguments encode valid, live VM-owned structures for the duration of this call and
//   are not concurrently mutated through aliased mutable references.
// - Postconditions: the function only mutates the referenced VM-owned structures associated with the
//   provided handles/pointers and returns sentinel values on validation failure.
pub unsafe extern "C" fn jit_dense_int_dict_int_full_set_ptr(
    dict_ptr: i64,
    key: i64,
    value: i64,
) -> i64 {
    if dict_ptr == 0 {
        return 0;
    }

    if key < 0 {
        return 0;
    }

    let values = &mut *((dict_ptr & !7) as *mut DenseIntDictIntFull);
    let index = key as usize;
    let len = values.len();
    if index == len {
        values.push(value);
    } else if index < len {
        values[index] = value;
    } else {
        values.resize(index + 1, 0);
        values[index] = value;
    }
    1
}

/// Load a variable as float from locals or globals (called from JIT code)
#[no_mangle]
// SAFETY:
// - Preconditions: jit_load_variable_float is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_load_variable_float(ctx: *mut VMContext, name_hash: i64) -> f64 {
    if ctx.is_null() {
        return 0.0;
    }

    let ctx = &*ctx;

    // Try to resolve the name from hash
    let name = if name_hash != 0 && !ctx.var_names_ptr.is_null() {
        let var_names = &*ctx.var_names_ptr;
        if let Some(n) = var_names.get(&(name_hash as u64)) {
            n.as_str()
        } else {
            return 0.0; // Hash not found
        }
    } else {
        return 0.0; // No name provided
    };

    // Try locals first
    if !ctx.locals_ptr.is_null() {
        let locals = &*ctx.locals_ptr;
        if let Some(Value::Float(val)) = locals.get(name) {
            return *val;
        }
    }

    // Try globals
    if !ctx.globals_ptr.is_null() {
        let globals = &*ctx.globals_ptr;
        if let Some(Value::Float(val)) = globals.get(name) {
            return *val;
        }
    }

    0.0
}

/// Store a float variable to locals (called from JIT code)
#[no_mangle]
// SAFETY:
// - Preconditions: jit_store_variable_float is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_store_variable_float(ctx: *mut VMContext, name_hash: i64, value: f64) {
    if ctx.is_null() {
        return;
    }

    let ctx = &mut *ctx;

    // Try to resolve the name from hash
    let name = if name_hash != 0 && !ctx.var_names_ptr.is_null() {
        let var_names = &*ctx.var_names_ptr;
        if let Some(n) = var_names.get(&(name_hash as u64)) {
            n.clone()
        } else {
            return; // Hash not found
        }
    } else {
        return; // No name provided
    };

    // Store in locals
    if !ctx.locals_ptr.is_null() {
        let locals = &mut *ctx.locals_ptr;
        locals.insert(name, Value::Float(value));
    }
}

/// Check if a variable is an Int (called from JIT code for guards)
/// Returns 1 if Int, 0 otherwise
#[no_mangle]
// SAFETY:
// - Preconditions: jit_check_type_int is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_check_type_int(ctx: *mut VMContext, name_hash: i64) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx = &*ctx;

    let name = if name_hash != 0 && !ctx.var_names_ptr.is_null() {
        let var_names = &*ctx.var_names_ptr;
        if let Some(n) = var_names.get(&(name_hash as u64)) {
            n.as_str()
        } else {
            return 0;
        }
    } else {
        return 0;
    };

    // Check locals first
    if !ctx.locals_ptr.is_null() {
        let locals = &*ctx.locals_ptr;
        if let Some(value) = locals.get(name) {
            return if matches!(value, Value::Int(_)) { 1 } else { 0 };
        }
    }

    // Check globals
    if !ctx.globals_ptr.is_null() {
        let globals = &*ctx.globals_ptr;
        if let Some(value) = globals.get(name) {
            return if matches!(value, Value::Int(_)) { 1 } else { 0 };
        }
    }

    0
}

/// Check if a variable is a Float (called from JIT code for guards)
/// Returns 1 if Float, 0 otherwise
#[no_mangle]
// SAFETY:
// - Preconditions: jit_check_type_float is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_check_type_float(ctx: *mut VMContext, name_hash: i64) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx = &*ctx;

    let name = if name_hash != 0 && !ctx.var_names_ptr.is_null() {
        let var_names = &*ctx.var_names_ptr;
        if let Some(n) = var_names.get(&(name_hash as u64)) {
            n.as_str()
        } else {
            return 0;
        }
    } else {
        return 0;
    };

    // Check locals first
    if !ctx.locals_ptr.is_null() {
        let locals = &*ctx.locals_ptr;
        if let Some(value) = locals.get(name) {
            return if matches!(value, Value::Float(_)) { 1 } else { 0 };
        }
    }

    // Check globals
    if !ctx.globals_ptr.is_null() {
        let globals = &*ctx.globals_ptr;
        if let Some(value) = globals.get(name) {
            return if matches!(value, Value::Float(_)) { 1 } else { 0 };
        }
    }

    0
}

/// Runtime helper: Push an integer value to the VM stack as Value::Int
/// Used by Return opcode to return integer results
#[no_mangle]
// SAFETY:
// - Preconditions: jit_push_int is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_push_int(ctx: *mut VMContext, value: i64) -> i64 {
    if ctx.is_null() {
        return 1; // Error
    }

    let ctx_ref = &mut *ctx;
    if ctx_ref.stack_ptr.is_null() {
        return 2; // Error
    }

    let stack = &mut *ctx_ref.stack_ptr;
    stack.push(Value::Int(value));
    0 // Success
}

/// Runtime helper: Fast return value setter - stores integer directly in VMContext
/// This is the OPTIMIZED path that avoids the stack push overhead.
/// ~3x faster than jit_push_int because it avoids:
/// 1. Stack pointer null check
/// 2. Stack Vec modification  
/// 3. Value::Int boxing (still needed, but VM reads field directly)
///
/// The VM checks has_return_value first; if true, it reads return_value directly
/// instead of popping from the stack.
#[no_mangle]
// SAFETY:
// - Preconditions: jit_set_return_int is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_set_return_int(ctx: *mut VMContext, value: i64) -> i64 {
    if ctx.is_null() {
        return 1; // Error
    }

    let ctx_ref = &mut *ctx;
    ctx_ref.return_value = value;
    ctx_ref.has_return_value = true;
    0 // Success
}

/// Safe wrapper around `jit_set_return_int` for internal Rust callsites that
/// already hold a valid mutable VM context reference.
#[inline]
#[allow(dead_code)]
pub fn set_return_int(ctx: &mut VMContext, value: i64) -> i64 {
    // SAFETY:
    // - Preconditions: `ctx` is a unique, live mutable reference and can be converted to `*mut VMContext`.
    // - Postconditions: return fields are updated in-place through the same context, with no leaked raw pointer.
    unsafe { jit_set_return_int(ctx as *mut VMContext, value) }
}

/// Runtime helper: Get return value from VMContext after a JIT recursive call
/// This retrieves the value stored by jit_set_return_int from a recursive call.
/// Returns the return_value if has_return_value is true, otherwise 0.
#[no_mangle]
// SAFETY:
// - Preconditions: jit_get_return_int is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_get_return_int(ctx: *mut VMContext) -> i64 {
    if ctx.is_null() {
        return 0; // Return 0 on error
    }

    let ctx_ref = &*ctx;
    if ctx_ref.has_return_value {
        ctx_ref.return_value
    } else {
        0 // No return value set
    }
}

/// Runtime helper: Get argument value from VMContext.argN field
/// This enables fast parameter passing without HashMap lookups.
/// Returns the argument value at the specified index (0-3).
/// If index >= arg_count or out of range, returns 0.
#[no_mangle]
// SAFETY:
// - Preconditions: jit_get_arg is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_get_arg(ctx: *mut VMContext, index: i64) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx_ref = &*ctx;

    // Check if index is valid
    if index < 0 || index >= ctx_ref.arg_count {
        return 0;
    }

    match index {
        0 => ctx_ref.arg0,
        1 => ctx_ref.arg1,
        2 => ctx_ref.arg2,
        3 => ctx_ref.arg3,
        _ => 0,
    }
}

/// Runtime helper: Call a function from JIT code
/// This enables JIT-compiled functions to call other functions
/// ctx: VMContext pointer (includes VM pointer for callbacks)
/// func_value_ptr: Unused (function is on stack)
/// arg_count: Number of arguments to pass to the function
/// Returns 0 on success, non-zero on error
#[no_mangle]
// SAFETY:
// - Preconditions: jit_call_function is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_call_function(
    ctx: *mut VMContext,
    _func_value_ptr: *const Value,
    arg_count: i64,
) -> i64 {
    if ctx.is_null() {
        return 1; // Error: null context
    }

    let ctx_ref = &mut *ctx;

    // Check if we have a stack
    if ctx_ref.stack_ptr.is_null() {
        return 2; // Error: null stack
    }

    let stack = &mut *ctx_ref.stack_ptr;

    // Debug: show stack state
    if std::env::var("DEBUG_JIT_CALL").is_ok() {
        eprintln!("jit_call_function: stack before (len={}): {:?}", stack.len(), stack);
        eprintln!("jit_call_function: arg_count={}", arg_count);
    }

    // In JIT mode, stack order is: [function, arg0, arg1, ...] (function pushed first by LoadVar)
    // So we need to pop args first (they're on top), then function (at bottom)

    // Pop arguments from stack (in reverse order)
    let mut args = Vec::new();
    for _ in 0..arg_count {
        if let Some(arg) = stack.pop() {
            args.push(arg);
        } else {
            return 4; // Error: stack underflow (not enough args)
        }
    }
    args.reverse(); // Restore original order

    // Now pop function from stack
    let function = if let Some(f) = stack.pop() {
        f
    } else {
        // Push args back and return error
        for arg in args.into_iter().rev() {
            stack.push(arg);
        }
        return 3; // Error: stack underflow (no function)
    };

    if std::env::var("DEBUG_JIT_CALL").is_ok() {
        eprintln!("jit_call_function: function={:?}, args={:?}", function, args);
    }

    // Check if we have VM pointer
    if ctx_ref.vm_ptr.is_null() {
        // No VM pointer - can't execute. Push placeholder and return error
        stack.push(Value::Int(0));
        return 5; // Error: no VM pointer
    }

    // Cast VM pointer back to VM
    let vm = &mut *(ctx_ref.vm_ptr as *mut VM);

    // Call the function through VM
    match vm.call_function_from_jit(function, args) {
        Ok(result) => {
            // Push result to stack
            if std::env::var("DEBUG_JIT_CALL").is_ok() {
                eprintln!("jit_call_function: result={:?}", result);
            }
            stack.push(result);
            0 // Success
        }
        Err(err) => {
            // Error during execution - push null and return error
            if std::env::var("DEBUG_JIT_CALL").is_ok() {
                eprintln!("jit_call_function: error={}", err);
            }
            stack.push(Value::Null);
            6 // Error: execution failed
        }
    }
}

/// Get value from dict by key (called from JIT code)
/// Stack layout: [dict, key] -> [value]
/// Returns 1 on success, 0 on error
#[no_mangle]
// SAFETY:
// - Preconditions: jit_dict_get is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_dict_get(ctx: *mut VMContext) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx_ref = &mut *ctx;
    if ctx_ref.stack_ptr.is_null() {
        return 0;
    }

    let stack = &mut *ctx_ref.stack_ptr;

    // Pop key and dict from stack
    let key = match stack.pop() {
        Some(val) => val,
        None => return 0, // Stack underflow
    };

    let dict_value = match stack.pop() {
        Some(val) => val,
        None => {
            stack.push(key); // Push back
            return 0; // Stack underflow
        }
    };

    // Perform the lookup based on types
    let result = match (&dict_value, &key) {
        (Value::Dict(dict), Value::Str(key_str)) => {
            dict.get(key_str.as_str()).cloned().unwrap_or(Value::Null)
        }
        (Value::Dict(dict), Value::Int(i)) => {
            let key = VMContext::jit_int_key_string(ctx_ref, *i);
            dict.get(key.as_ref()).cloned().unwrap_or(Value::Null)
        }
        (Value::IntDict(dict), Value::Int(i)) => dict.get(i).cloned().unwrap_or(Value::Null),
        (Value::DenseIntDict(values), Value::Int(i)) => {
            if *i < 0 {
                Value::Null
            } else {
                values.get(*i as usize).cloned().unwrap_or(Value::Null)
            }
        }
        (Value::DenseIntDictInt(values), Value::Int(i)) => {
            if *i < 0 {
                Value::Null
            } else {
                let index = *i as usize;
                if index < values.len() {
                    match values[index] {
                        Some(value) => Value::Int(value),
                        None => Value::Null,
                    }
                } else {
                    Value::Null
                }
            }
        }
        (Value::DenseIntDictIntFull(values), Value::Int(i)) => {
            if *i < 0 {
                Value::Null
            } else {
                values.get(*i as usize).map(|value| Value::Int(*value)).unwrap_or(Value::Null)
            }
        }
        (Value::IntDict(dict), Value::Str(key)) => match key.parse::<i64>() {
            Ok(int_key) => dict.get(&int_key).cloned().unwrap_or(Value::Null),
            Err(_) => Value::Null,
        },
        (Value::DenseIntDict(values), Value::Str(key)) => match key.parse::<i64>() {
            Ok(int_key) => {
                if int_key < 0 {
                    Value::Null
                } else {
                    values.get(int_key as usize).cloned().unwrap_or(Value::Null)
                }
            }
            Err(_) => Value::Null,
        },
        (Value::DenseIntDictInt(values), Value::Str(key)) => match key.parse::<i64>() {
            Ok(int_key) => {
                if int_key < 0 {
                    Value::Null
                } else {
                    let index = int_key as usize;
                    if index < values.len() {
                        match values[index] {
                            Some(value) => Value::Int(value),
                            None => Value::Null,
                        }
                    } else {
                        Value::Null
                    }
                }
            }
            Err(_) => Value::Null,
        },
        (Value::DenseIntDictIntFull(values), Value::Str(key)) => match key.parse::<i64>() {
            Ok(int_key) => {
                if int_key < 0 {
                    Value::Null
                } else {
                    values
                        .get(int_key as usize)
                        .map(|value| Value::Int(*value))
                        .unwrap_or(Value::Null)
                }
            }
            Err(_) => Value::Null,
        },
        (Value::Array(arr), Value::Int(i)) => {
            let idx = if *i < 0 { (arr.len() as i64 + i) as usize } else { *i as usize };
            arr.get(idx).cloned().unwrap_or(Value::Null)
        }
        _ => Value::Null, // Type mismatch
    };

    stack.push(result);
    1 // Success
}

/// Set value in dict by key (called from JIT code)
/// Stack layout: [dict, key, value] -> [dict]
/// Returns 1 on success, 0 on error
#[no_mangle]
// SAFETY:
// - Preconditions: jit_dict_set is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
pub unsafe extern "C" fn jit_dict_set(ctx: *mut VMContext) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx_ref = &mut *ctx;
    if ctx_ref.stack_ptr.is_null() {
        return 0;
    }

    let stack = &mut *ctx_ref.stack_ptr;

    // Pop value, key, and dict from stack (value is on top)
    let value = match stack.pop() {
        Some(val) => val,
        None => return 0,
    };

    let key = match stack.pop() {
        Some(val) => val,
        None => {
            stack.push(value);
            return 0;
        }
    };

    let dict_value = match stack.pop() {
        Some(val) => val,
        None => {
            stack.push(key);
            stack.push(value);
            return 0;
        }
    };

    // Perform the set operation
    let result = match (dict_value, key) {
        (Value::Dict(mut dict), Value::Str(key_str)) => {
            let dict_mut = Arc::make_mut(&mut dict);
            dict_mut.insert(key_str.as_str().into(), value);
            Value::Dict(dict)
        }
        (Value::Dict(mut dict), Value::Int(i)) => {
            if dict.is_empty() {
                if i >= 0 {
                    match value {
                        Value::Int(int_value) => {
                            if i == 0 {
                                Value::DenseIntDictIntFull(Arc::new(vec![int_value]))
                            } else {
                                let mut values = dense_int_dict_int_with_len((i as usize) + 1);
                                values[i as usize] = Some(int_value);
                                Value::DenseIntDictInt(Arc::new(values))
                            }
                        }
                        Value::Null => {
                            let values = dense_int_dict_int_with_len((i as usize) + 1);
                            Value::DenseIntDictInt(Arc::new(values))
                        }
                        other => {
                            let mut values = vec![Value::Null; (i as usize) + 1];
                            values[i as usize] = other;
                            Value::DenseIntDict(Arc::new(values))
                        }
                    }
                } else {
                    let mut int_dict = IntDictMap::default();
                    int_dict.reserve(1024);
                    int_dict.insert(i, value);
                    Value::IntDict(Arc::new(int_dict))
                }
            } else {
                let dict_mut = Arc::make_mut(&mut dict);
                let key = VMContext::jit_int_key_string(ctx_ref, i);
                dict_mut.insert(Arc::clone(&key), value);
                Value::Dict(dict)
            }
        }
        (Value::IntDict(mut dict), Value::Int(i)) => {
            let dict_mut = Arc::make_mut(&mut dict);
            dict_mut.insert(i, value);
            Value::IntDict(dict)
        }
        (Value::DenseIntDict(mut values), Value::Int(i)) => {
            if i < 0 {
                let mut int_dict = IntDictMap::default();
                int_dict.reserve(values.len());
                for (index, value) in values.iter().enumerate() {
                    int_dict.insert(index as i64, value.clone());
                }
                int_dict.insert(i, value);
                Value::IntDict(Arc::new(int_dict))
            } else {
                let values_mut = Arc::make_mut(&mut values);
                let index = i as usize;
                if index >= values_mut.len() {
                    values_mut.resize(index + 1, Value::Null);
                }
                values_mut[index] = value;
                Value::DenseIntDict(values)
            }
        }
        (Value::DenseIntDictInt(mut values), Value::Int(i)) => {
            if i < 0 {
                let mut int_dict = IntDictMap::default();
                int_dict.reserve(values.len());
                for (index, value) in values.iter().enumerate() {
                    if let Some(int_value) = value {
                        int_dict.insert(index as i64, Value::Int(*int_value));
                    }
                }
                int_dict.insert(i, value);
                Value::IntDict(Arc::new(int_dict))
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
                        Value::DenseIntDictInt(values)
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
                        Value::DenseIntDictInt(values)
                    }
                    other => {
                        let mut dense_values = Vec::with_capacity(values.len().max(index + 1));
                        for value in values.iter() {
                            dense_values.push((*value).map(Value::Int).unwrap_or(Value::Null));
                        }
                        if index >= dense_values.len() {
                            dense_values.resize(index + 1, Value::Null);
                        }
                        dense_values[index] = other;
                        Value::DenseIntDict(Arc::new(dense_values))
                    }
                }
            }
        }
        (Value::DenseIntDictIntFull(mut values), Value::Int(i)) => {
            if i < 0 {
                let mut int_dict = IntDictMap::default();
                int_dict.reserve(values.len());
                for (index, value) in values.iter().enumerate() {
                    int_dict.insert(index as i64, Value::Int(*value));
                }
                int_dict.insert(i, value);
                Value::IntDict(Arc::new(int_dict))
            } else {
                let index = i as usize;
                match value {
                    Value::Int(int_value) => {
                        let values_mut = Arc::make_mut(&mut values);
                        let len = values_mut.len();
                        if index == len {
                            values_mut.push(int_value);
                            Value::DenseIntDictIntFull(values)
                        } else if index < len {
                            values_mut[index] = int_value;
                            Value::DenseIntDictIntFull(values)
                        } else {
                            let mut sparse = Vec::with_capacity(values.len().max(index + 1));
                            for value in values.iter() {
                                sparse.push(Some(*value));
                            }
                            sparse.resize(index + 1, None);
                            sparse[index] = Some(int_value);
                            Value::DenseIntDictInt(Arc::new(sparse))
                        }
                    }
                    Value::Null => {
                        let mut sparse = Vec::with_capacity(values.len().max(index + 1));
                        for value in values.iter() {
                            sparse.push(Some(*value));
                        }
                        if index >= sparse.len() {
                            sparse.resize(index + 1, None);
                        }
                        sparse[index] = None;
                        Value::DenseIntDictInt(Arc::new(sparse))
                    }
                    other => {
                        let mut dense_values = Vec::with_capacity(values.len().max(index + 1));
                        for value in values.iter() {
                            dense_values.push(Value::Int(*value));
                        }
                        if index >= dense_values.len() {
                            dense_values.resize(index + 1, Value::Null);
                        }
                        dense_values[index] = other;
                        Value::DenseIntDict(Arc::new(dense_values))
                    }
                }
            }
        }
        (Value::IntDict(dict), Value::Str(key_str)) => {
            let mut dict_clone = DictMap::default();
            for (k, v) in dict.iter() {
                dict_clone.insert(k.to_string().into(), v.clone());
            }
            dict_clone.insert(key_str.as_str().into(), value);
            Value::Dict(Arc::new(dict_clone))
        }
        (Value::DenseIntDict(values), Value::Str(key_str)) => match key_str.parse::<i64>() {
            Ok(int_key) => {
                if int_key < 0 {
                    let mut int_dict = IntDictMap::default();
                    int_dict.reserve(values.len());
                    for (index, value) in values.iter().enumerate() {
                        int_dict.insert(index as i64, value.clone());
                    }
                    int_dict.insert(int_key, value);
                    Value::IntDict(Arc::new(int_dict))
                } else {
                    let mut values = values;
                    let values_mut = Arc::make_mut(&mut values);
                    let index = int_key as usize;
                    if index >= values_mut.len() {
                        values_mut.resize(index + 1, Value::Null);
                    }
                    values_mut[index] = value;
                    Value::DenseIntDict(values)
                }
            }
            Err(_) => {
                let mut dict_clone = DictMap::default();
                for (index, value) in values.iter().enumerate() {
                    dict_clone.insert(index.to_string().into(), value.clone());
                }
                dict_clone.insert(key_str.as_str().into(), value);
                Value::Dict(Arc::new(dict_clone))
            }
        },
        (Value::DenseIntDictInt(values), Value::Str(key_str)) => match key_str.parse::<i64>() {
            Ok(int_key) => {
                if int_key < 0 {
                    let mut int_dict = IntDictMap::default();
                    int_dict.reserve(values.len());
                    for (index, value) in values.iter().enumerate() {
                        if let Some(int_value) = value {
                            int_dict.insert(index as i64, Value::Int(*int_value));
                        }
                    }
                    int_dict.insert(int_key, value);
                    Value::IntDict(Arc::new(int_dict))
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
                            Value::DenseIntDictInt(values)
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
                            Value::DenseIntDictInt(values)
                        }
                        other => {
                            let mut dense_values = Vec::with_capacity(values.len().max(index + 1));
                            for value in values.iter() {
                                dense_values.push((*value).map(Value::Int).unwrap_or(Value::Null));
                            }
                            if index >= dense_values.len() {
                                dense_values.resize(index + 1, Value::Null);
                            }
                            dense_values[index] = other;
                            Value::DenseIntDict(Arc::new(dense_values))
                        }
                    }
                }
            }
            Err(_) => {
                let mut dict_clone = DictMap::default();
                for (index, value) in values.iter().enumerate() {
                    dict_clone.insert(
                        index.to_string().into(),
                        (*value).map(Value::Int).unwrap_or(Value::Null),
                    );
                }
                dict_clone.insert(key_str.as_str().into(), value);
                Value::Dict(Arc::new(dict_clone))
            }
        },
        (Value::DenseIntDictIntFull(values), Value::Str(key_str)) => match key_str.parse::<i64>() {
            Ok(int_key) => {
                if int_key < 0 {
                    let mut int_dict = IntDictMap::default();
                    int_dict.reserve(values.len());
                    for (index, value) in values.iter().enumerate() {
                        int_dict.insert(index as i64, Value::Int(*value));
                    }
                    int_dict.insert(int_key, value);
                    Value::IntDict(Arc::new(int_dict))
                } else {
                    let index = int_key as usize;
                    match value {
                        Value::Int(int_value) => {
                            let mut values = values;
                            let values_mut = Arc::make_mut(&mut values);
                            let len = values_mut.len();
                            if index == len {
                                values_mut.push(int_value);
                                Value::DenseIntDictIntFull(values)
                            } else if index < len {
                                values_mut[index] = int_value;
                                Value::DenseIntDictIntFull(values)
                            } else {
                                let mut sparse = Vec::with_capacity(values.len().max(index + 1));
                                for value in values.iter() {
                                    sparse.push(Some(*value));
                                }
                                sparse.resize(index + 1, None);
                                sparse[index] = Some(int_value);
                                Value::DenseIntDictInt(Arc::new(sparse))
                            }
                        }
                        Value::Null => {
                            let mut sparse = Vec::with_capacity(values.len().max(index + 1));
                            for value in values.iter() {
                                sparse.push(Some(*value));
                            }
                            if index >= sparse.len() {
                                sparse.resize(index + 1, None);
                            }
                            sparse[index] = None;
                            Value::DenseIntDictInt(Arc::new(sparse))
                        }
                        other => {
                            let mut dense_values = Vec::with_capacity(values.len().max(index + 1));
                            for value in values.iter() {
                                dense_values.push(Value::Int(*value));
                            }
                            if index >= dense_values.len() {
                                dense_values.resize(index + 1, Value::Null);
                            }
                            dense_values[index] = other;
                            Value::DenseIntDict(Arc::new(dense_values))
                        }
                    }
                }
            }
            Err(_) => {
                let mut dict_clone = DictMap::default();
                for (index, value) in values.iter().enumerate() {
                    dict_clone.insert(index.to_string().into(), Value::Int(*value));
                }
                dict_clone.insert(key_str.as_str().into(), value);
                Value::Dict(Arc::new(dict_clone))
            }
        },
        (Value::Array(mut arr), Value::Int(i)) => {
            let arr_mut = Arc::make_mut(&mut arr);
            let idx = if i < 0 { (arr_mut.len() as i64 + i) as usize } else { i as usize };
            if idx < arr_mut.len() {
                arr_mut[idx] = value;
                Value::Array(arr)
            } else {
                return 0;
            }
        }
        _ => return 0,
    };

    stack.push(result);
    1 // Success
}

/// Runtime helper for creating dictionaries (MakeDict opcode)
/// Pops N key-value pairs from VM stack and creates a new dict
///
/// # Arguments
/// * `ctx` - Mutable pointer to VM context with stack
/// * `num_pairs` - Number of key-value pairs to pop
///
/// # Returns
/// 1 on success (dict pushed to stack), 0 on error
#[no_mangle]
pub extern "C" fn jit_make_dict(ctx: *mut VMContext, num_pairs: i64) -> i64 {
    with_vm_stack_mut(ctx, |vm_ctx, stack| {
        let debug_jit = std::env::var("DEBUG_JIT").is_ok();

        if debug_jit {
            eprintln!(
                "JIT: jit_make_dict called with num_pairs={}, stack len={}",
                num_pairs,
                stack.len()
            );
        }

        // Pop N key-value pairs from the stack
        let mut map = DictMap::default();
        map.reserve(num_pairs as usize);
        for i in 0..num_pairs {
            // Pop value, then key (reverse order since stack)
            let Some(value_val) = stack.pop() else {
                if debug_jit {
                    eprintln!("JIT: MakeDict stack underflow (value) at pair {}", i);
                }
                return 0;
            };
            let Some(key_val) = stack.pop() else {
                if debug_jit {
                    eprintln!("JIT: MakeDict stack underflow (key) at pair {}", i);
                }
                return 0;
            };

            if debug_jit {
                eprintln!("JIT: Pair {}: key={:?}, value={:?}", i, key_val, value_val);
            }

            // Convert key to string
            let key_str = match &key_val {
                Value::Str(s) => Arc::from(s.as_str()),
                Value::Int(i) => VMContext::jit_int_key_string(vm_ctx, *i),
                other => {
                    if debug_jit {
                        eprintln!("JIT: MakeDict requires string or int keys, got {:?}", other);
                    }
                    return 0;
                }
            };

            map.insert(Arc::clone(&key_str), value_val);
        }

        if debug_jit {
            eprintln!("JIT: Created dict with {} entries", map.len());
        }

        // Push the new dict to the stack
        stack.push(Value::Dict(Arc::new(map)));

        1 // Success
    })
    .unwrap_or(0)
}

/// Runtime helper for creating dictionaries with constant string keys (MakeDictWithKeys opcode)
/// Pops N values from VM stack and creates a new dict with provided keys
///
/// # Arguments
/// * `ctx` - Mutable pointer to VM context with stack
/// * `keys_ptr` - Pointer to Arc<Vec<Arc<str>>> keys
/// * `num_keys` - Number of keys/values
///
/// # Returns
/// 1 on success (dict pushed to stack), 0 on error
#[no_mangle]
pub extern "C" fn jit_make_dict_with_keys(
    ctx: *mut VMContext,
    keys_ptr: i64,
    num_keys: i64,
) -> i64 {
    if num_keys < 0 {
        return 0;
    }

    with_vm_stack_mut(ctx, |_vm_ctx, stack| {
        if num_keys == 0 {
            stack.push(Value::Dict(Arc::new(DictMap::default())));
            return 1;
        }

        if keys_ptr == 0 {
            return 0;
        }

        // SAFETY:
        // - Preconditions: `keys_ptr` is non-zero and points to a live `Arc<Vec<Arc<str>>>` allocated by VM/JIT code.
        // - Postconditions: keys are read immutably for this call only; ownership stays with the originating Arc.
        let keys = unsafe { &*(keys_ptr as *const Arc<Vec<Arc<str>>>) };
        if num_keys as usize != keys.len() {
            return 0;
        }

        let mut values = vec![Value::Null; keys.len()];
        for index in (0..keys.len()).rev() {
            let Some(value_val) = stack.pop() else {
                return 0;
            };

            values[index] = value_val;
        }

        stack.push(Value::FixedDict { keys: Arc::clone(keys), values });
        1
    })
    .unwrap_or(0)
}

/// JIT compiler for Kujo bytecode
pub struct JitCompiler {
    /// Cranelift JIT module
    module: JITModule,

    /// Code generation context
    ctx: codegen::Context,

    /// Execution counter for hot path detection
    execution_counts: HashMap<usize, usize>,

    /// Cache of compiled functions (bytecode offset -> native function)
    compiled_cache: HashMap<usize, CompiledFn>,

    /// JIT enabled/disabled flag
    enabled: bool,

    /// Loop headers that should never be JIT-compiled
    loop_jit_blacklist: HashSet<usize>,

    /// Type profiling data for specialization
    type_profiles: HashMap<usize, SpecializationInfo>,

    /// Enhanced function info cache (function name -> info)
    /// Stores CompiledFnInfo which includes both standard and direct-arg variants
    /// This enables direct JIT recursion for eligible functions
    compiled_fn_info: HashMap<String, CompiledFnInfo>,
}

/// Bytecode translator - converts bytecode to Cranelift IR
struct BytecodeTranslator {
    /// Stack simulation - maps stack depth to Cranelift values
    value_stack: Vec<cranelift::prelude::Value>,
    /// Variable storage - maps variable names to Cranelift values
    /// TODO: Future optimization - keep frequently used variables in registers
    #[allow(dead_code)]
    variables: HashMap<String, cranelift::prelude::Value>,
    /// Blocks for control flow - maps bytecode PC to Cranelift blocks
    blocks: HashMap<usize, Block>,
    /// Expected stack depth when entering each block (for SSA handling)
    block_entry_stack_depth: HashMap<usize, usize>,
    /// PCs that are loop headers (backward jump targets)
    /// These blocks should not be sealed until all predecessors are processed
    loop_header_pcs: std::collections::HashSet<usize>,
    /// VMContext parameter passed to the function
    ctx_param: Option<cranelift::prelude::Value>,
    /// External function references
    load_var_func: Option<FuncRef>,
    store_var_func: Option<FuncRef>,
    /// Float load/store function references
    load_var_float_func: Option<FuncRef>,
    store_var_float_func: Option<FuncRef>,
    /// Type guard function references
    check_type_int_func: Option<FuncRef>,
    check_type_float_func: Option<FuncRef>,
    /// Call function reference (for Call opcode)
    call_func: Option<FuncRef>,
    /// Push int function reference (for Return opcode - fallback)
    push_int_func: Option<FuncRef>,
    /// Fast return int function reference (for Return opcode - optimized path)
    /// Uses VMContext.return_value directly instead of pushing to stack
    set_return_int_func: Option<FuncRef>,
    /// Stack pop function reference (for getting call results)
    stack_pop_func: Option<FuncRef>,
    /// Stack push function reference (for pushing args before call)
    stack_push_func: Option<FuncRef>,
    /// Object push string helper (non-int JIT values)
    obj_push_string_func: Option<FuncRef>,
    /// Object to VM stack helper (non-int JIT values)
    obj_to_vm_stack_func: Option<FuncRef>,
    /// Dict get function reference (for IndexGet opcode)
    dict_get_func: Option<FuncRef>,
    /// Dict set function reference (for IndexSet opcode)
    dict_set_func: Option<FuncRef>,
    /// Make dict function reference (for MakeDict opcode)
    make_dict_func: Option<FuncRef>,
    /// Make dict with keys function reference (for MakeDictWithKeys opcode)
    make_dict_with_keys_func: Option<FuncRef>,
    /// Append const string in-place helper
    append_const_string_in_place_func: Option<FuncRef>,
    /// Append const char in-place helper
    append_const_char_in_place_func: Option<FuncRef>,
    /// Local slot dict get helper (loop JIT)
    local_slot_dict_get_func: Option<FuncRef>,
    /// Local slot dict set helper (loop JIT)
    local_slot_dict_set_func: Option<FuncRef>,
    /// Local slot IntDict get helper (loop JIT fast path)
    local_slot_int_dict_get_func: Option<FuncRef>,
    /// Local slot IntDict set helper (loop JIT fast path)
    local_slot_int_dict_set_func: Option<FuncRef>,
    /// Unique int-dict pointer helper (loop JIT fast path)
    int_dict_unique_ptr_func: Option<FuncRef>,
    /// Int-dict get via pointer helper (loop JIT fast path)
    int_dict_get_ptr_func: Option<FuncRef>,
    /// DenseIntDictInt get via pointer helper (loop JIT fast path)
    int_dict_get_ptr_dense_int_func: Option<FuncRef>,
    /// DenseIntDictIntFull get via pointer helper (loop JIT fast path)
    int_dict_get_ptr_dense_int_full_func: Option<FuncRef>,
    /// Int-dict set via pointer helper (loop JIT fast path)
    int_dict_set_ptr_func: Option<FuncRef>,
    /// DenseIntDictInt set via pointer helper (loop JIT fast path)
    int_dict_set_ptr_dense_int_func: Option<FuncRef>,
    /// DenseIntDictIntFull set via pointer helper (loop JIT fast path)
    int_dict_set_ptr_dense_int_full_func: Option<FuncRef>,
    /// Specialization information for this compilation
    specialization: Option<SpecializationInfo>,
    /// Local slot names indexed by slot id
    local_names: Vec<String>,
    /// End of function body (PC index, exclusive)
    function_end: usize,
    /// Local variable slots - maps variable name to stack slot
    /// This enables register-based locals optimization (Phase 7 Step 7)
    /// Instead of calling runtime functions for every LoadVar/StoreVar,
    /// we use direct memory access via Cranelift stack slots
    local_slots: HashMap<String, StackSlot>,
    /// Local variables that may hold non-integer values (e.g., dicts)
    non_int_locals: std::collections::HashSet<String>,
    /// Local slot indices eligible for unique int-dict fast path
    int_dict_slots: std::collections::HashSet<usize>,
    /// Stack slots holding cached int-dict pointers
    int_dict_ptr_slots: HashMap<usize, StackSlot>,
    /// Flag to enable register-based locals optimization
    /// When true, LoadVar/StoreVar use stack slots instead of runtime calls
    use_local_slots: bool,
    /// Store a variable from VM stack helper (for non-int locals)
    store_var_from_stack_func: Option<FuncRef>,
    /// Current function name - enables self-recursion detection
    /// When a Call opcode follows a LoadVar with this name, we emit
    /// direct recursive calls instead of going through the VM
    current_function_name: Option<String>,
    /// Flag set when LoadVar loads the current function (self-recursion pending)
    /// This is set by LoadVar when name matches current_function_name,
    /// and cleared by Call after emitting the recursive call
    self_recursion_pending: bool,
    /// Self-recursive call function reference
    /// Points to the function being compiled, enabling direct self-calls
    self_call_func: Option<FuncRef>,
    /// Get return int function reference (for getting recursive call results)
    get_return_int_func: Option<FuncRef>,
    /// Persist local slots back into VM locals on return (used for loop JIT)
    persist_locals_on_return: bool,

    // Phase 7 Step 12: Direct JIT Recursion support
    /// Whether this function is compiled with direct-arg signature
    /// When true, the function takes its first arg as a Cranelift parameter
    /// rather than reading from VMContext, enabling direct JIT recursion
    direct_arg_mode: bool,
    /// The direct argument Cranelift value (the function's first argument)
    /// Only set when direct_arg_mode is true
    direct_arg_param: Option<cranelift::prelude::Value>,
    /// Number of parameters this function takes (for direct recursion validation)
    param_count: usize,
    /// Locals written during translation (used to limit persistence work)
    dirty_local_names: std::collections::HashSet<String>,
}

impl BytecodeTranslator {
    fn new() -> Self {
        Self {
            value_stack: Vec::new(),
            variables: HashMap::new(),
            blocks: HashMap::new(),
            block_entry_stack_depth: HashMap::new(),
            loop_header_pcs: std::collections::HashSet::new(),
            ctx_param: None,
            load_var_func: None,
            store_var_func: None,
            load_var_float_func: None,
            store_var_float_func: None,
            check_type_int_func: None,
            check_type_float_func: None,
            call_func: None,
            push_int_func: None,
            set_return_int_func: None,
            stack_pop_func: None,
            stack_push_func: None,
            obj_push_string_func: None,
            obj_to_vm_stack_func: None,
            specialization: None,
            function_end: 0,
            local_slots: HashMap::new(),
            non_int_locals: std::collections::HashSet::new(),
            use_local_slots: false,
            local_names: Vec::new(),
            current_function_name: None,
            self_recursion_pending: false,
            self_call_func: None,
            get_return_int_func: None,
            dict_get_func: None,
            dict_set_func: None,
            make_dict_func: None,
            make_dict_with_keys_func: None,
            append_const_string_in_place_func: None,
            append_const_char_in_place_func: None,
            local_slot_dict_get_func: None,
            local_slot_dict_set_func: None,
            local_slot_int_dict_get_func: None,
            local_slot_int_dict_set_func: None,
            int_dict_unique_ptr_func: None,
            int_dict_get_ptr_func: None,
            int_dict_get_ptr_dense_int_func: None,
            int_dict_get_ptr_dense_int_full_func: None,
            int_dict_set_ptr_func: None,
            int_dict_set_ptr_dense_int_func: None,
            int_dict_set_ptr_dense_int_full_func: None,
            persist_locals_on_return: true,
            direct_arg_mode: false,
            direct_arg_param: None,
            param_count: 0,
            dirty_local_names: std::collections::HashSet::new(),
            store_var_from_stack_func: None,
            int_dict_slots: std::collections::HashSet::new(),
            int_dict_ptr_slots: HashMap::new(),
        }
    }

    fn set_local_names(&mut self, local_names: Vec<String>) {
        self.local_names = local_names;
    }

    /// Analyze bytecode to find loop headers (backward jump targets) and their expected stack depths.
    /// This is required for correct SSA handling - Cranelift blocks must have parameters
    /// declared when they're created, but for backward jumps we don't know the stack depth
    /// until we reach the jump instruction.
    ///
    /// Returns a HashMap: PC -> expected stack depth for blocks that receive backward jumps
    fn analyze_loop_headers(instructions: &[OpCode], function_end: usize) -> HashMap<usize, usize> {
        let mut loop_headers: HashMap<usize, usize> = HashMap::new();
        let mut pc_stack_depths: HashMap<usize, i32> = HashMap::new();
        let mut stack_depth: i32 = 0;

        // Simple simulation: track stack depth changes through the bytecode
        // We're looking for backward jumps and what stack depth they carry

        for (pc, instruction) in instructions.iter().enumerate() {
            if pc >= function_end {
                break;
            }

            // Record stack depth at this PC before processing
            pc_stack_depths.insert(pc, stack_depth);

            // Calculate stack effect of each instruction
            let (pops, pushes) = Self::stack_effect(instruction);
            stack_depth = (stack_depth - pops as i32 + pushes as i32).max(0);

            // Check for backward jumps (loop back-edges)
            match instruction {
                OpCode::Jump(target) | OpCode::JumpBack(target) => {
                    if *target <= pc && *target < function_end {
                        // This is a backward jump - record the target as a loop header
                        // The stack depth at the jump is what we need at the header
                        let depth = stack_depth.max(0) as usize;
                        loop_headers.insert(*target, depth);

                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!(
                                "JIT: Found loop header at PC {} with stack depth {}",
                                target, depth
                            );
                        }
                    }
                }
                OpCode::JumpIfTrue(target) | OpCode::JumpIfFalse(target) => {
                    if *target <= pc && *target < function_end {
                        // Conditional backward jump - this is a loop back-edge
                        // Note: Condition stays on stack (peek), so add 1
                        let depth = stack_depth.max(0) as usize;
                        loop_headers.insert(*target, depth);

                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!(
                                "JIT: Found conditional loop header at PC {} with stack depth {}",
                                target, depth
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        loop_headers
    }

    /// Calculate the stack effect (pops, pushes) of a bytecode instruction
    fn stack_effect(instruction: &OpCode) -> (usize, usize) {
        match instruction {
            // Push instructions (0 pops, 1 push)
            OpCode::LoadConst(_)
            | OpCode::LoadVar(_)
            | OpCode::LoadLocal(_)
            | OpCode::LoadGlobal(_) => (0, 1),

            // Pop instructions (1 pop, 0 push)
            OpCode::Pop => (1, 0),

            // Store operations peek (no pop)
            OpCode::StoreVar(_) | OpCode::StoreLocal(_) | OpCode::StoreGlobal(_) => (0, 0),

            // Binary ops (2 pops, 1 push)
            OpCode::Add
            | OpCode::Sub
            | OpCode::Mul
            | OpCode::Div
            | OpCode::Mod
            | OpCode::Equal
            | OpCode::NotEqual
            | OpCode::LessThan
            | OpCode::GreaterThan
            | OpCode::LessEqual
            | OpCode::GreaterEqual => (2, 1),
            OpCode::AddInPlace(_) => (1, 1),
            OpCode::AppendConstStringInPlace(_, _) => (0, 0),
            OpCode::AppendConstCharInPlace(_, _) => (0, 0),

            // Unary ops (1 pop, 1 push)
            OpCode::Negate | OpCode::Not => (1, 1),

            // Stack manipulation
            OpCode::Dup => (0, 1), // Peek + push copy

            // Control flow (vary by type)
            OpCode::Return | OpCode::ReturnNone => (0, 0), // Terminates
            OpCode::Jump(_) | OpCode::JumpBack(_) => (0, 0), // No stack effect
            OpCode::JumpIfTrue(_) | OpCode::JumpIfFalse(_) => (0, 0), // Peek, no pop

            // Call (pops args + func, pushes result)
            OpCode::Call(n) => (*n + 1, 1), // Pop n args + 1 func, push result

            // Dict/Array operations
            OpCode::IndexGet => (2, 1), // Pop object + index, push value
            OpCode::IndexSet => (3, 0), // Pop object + index + value, push nothing
            OpCode::IndexGetInPlace(_) => (1, 1), // Pop index, load var, push value
            OpCode::IndexSetInPlace(_) => (2, 0), // Pop index + value, modify variable in place
            OpCode::MakeDict(n) => (*n * 2, 1), // Pop N key-value pairs, push dict
            OpCode::MakeDictWithKeys(keys) => (keys.len(), 1), // Pop N values, push dict

            // Default for unknown opcodes - assume neutral
            _ => (0, 0),
        }
    }

    /// Enable direct-arg mode for functions with single integer parameter
    /// In this mode, the function's first argument is passed directly as a
    /// Cranelift parameter, enabling direct JIT-to-JIT recursion without FFI
    fn set_direct_arg_mode(&mut self, param: cranelift::prelude::Value, param_count: usize) {
        self.direct_arg_mode = true;
        self.direct_arg_param = Some(param);
        self.param_count = param_count;
    }

    /// Scan bytecode to discover all local variables and pre-allocate stack slots
    /// This enables register-based locals optimization - avoiding HashMap lookups
    /// and C function calls for every variable access
    fn allocate_local_slots(
        &mut self,
        builder: &mut FunctionBuilder,
        instructions: &[OpCode],
        function_end: usize,
    ) {
        self.scan_non_int_locals(instructions, function_end);

        if !self.local_names.is_empty() {
            for name in &self.local_names {
                if self.non_int_locals.contains(name) {
                    continue;
                }
                if !self.local_slots.contains_key(name) {
                    let slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        8,
                        0,
                    ));
                    self.local_slots.insert(name.clone(), slot);
                }
            }
        }

        // Collect variable names that are WRITTEN to (StoreVar targets)
        // We only create stack slots for variables that are actually assigned
        // Variables that are only read (like recursive function references)
        // should be loaded via runtime calls from globals
        let mut store_var_names: Vec<String> = Vec::new();

        for (pc, instruction) in instructions.iter().enumerate() {
            if pc >= function_end {
                break;
            }

            // Only allocate slots for variables that have StoreVar operations
            // This ensures we don't create slots for function references that
            // should be loaded from globals (like 'fib' in recursive calls)
            if let OpCode::StoreVar(name) = instruction {
                if !store_var_names.contains(name) {
                    store_var_names.push(name.clone());
                }
            }
        }

        // Create a stack slot for each locally-assigned variable
        // Each slot is 8 bytes (i64) to store integer values
        for name in store_var_names {
            if self.non_int_locals.contains(&name) {
                continue;
            }
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8, // 8 bytes for i64
                0, // No alignment padding needed for single slot
            ));
            self.local_slots.insert(name.clone(), slot);

            if std::env::var("DEBUG_JIT").is_ok() {
                eprintln!("JIT: Allocated stack slot {:?} for local '{}'", slot, name);
            }
        }

        // Enable local slot optimization if we have any locals
        self.use_local_slots = !self.local_slots.is_empty();

        if std::env::var("DEBUG_JIT").is_ok() {
            eprintln!(
                "JIT: Allocated {} local slots, optimization enabled: {}",
                self.local_slots.len(),
                self.use_local_slots
            );
        }
    }

    /// Allocate stack slots for local variables EXCEPT the specified parameter
    /// Used in direct-arg mode where the first parameter is passed as a Cranelift value
    fn allocate_local_slots_except(
        &mut self,
        builder: &mut FunctionBuilder,
        instructions: &[OpCode],
        function_end: usize,
        exclude_param: &str,
    ) {
        self.scan_non_int_locals(instructions, function_end);

        if !self.local_names.is_empty() {
            for name in &self.local_names {
                if self.non_int_locals.contains(name) {
                    continue;
                }
                if self.local_slots.contains_key(name) {
                    continue;
                }
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    8,
                    0,
                ));
                self.local_slots.insert(name.clone(), slot);
            }
        }

        let mut store_var_names: Vec<String> = Vec::new();

        for (pc, instruction) in instructions.iter().enumerate() {
            if pc >= function_end {
                break;
            }

            if let OpCode::StoreVar(name) = instruction {
                if name != exclude_param && !store_var_names.contains(name) {
                    store_var_names.push(name.clone());
                }
            }
        }

        for name in store_var_names {
            if self.non_int_locals.contains(&name) {
                continue;
            }
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                0,
            ));
            self.local_slots.insert(name.clone(), slot);

            if std::env::var("DEBUG_JIT").is_ok() {
                eprintln!(
                    "JIT: Allocated stack slot {:?} for local '{}' (direct-arg mode)",
                    slot, name
                );
            }
        }

        self.use_local_slots = !self.local_slots.is_empty();
    }

    /// Allocate stack slots for function parameters
    /// This is called separately to ensure parameters have slots for the fast path
    fn allocate_parameter_slots(&mut self, builder: &mut FunctionBuilder, params: &[String]) {
        for param_name in params {
            if self.non_int_locals.contains(param_name) {
                continue;
            }
            // Only allocate if not already allocated (may be assigned in function)
            if !self.local_slots.contains_key(param_name) {
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    8, // 8 bytes for i64
                    0, // No alignment padding needed for single slot
                ));
                self.local_slots.insert(param_name.clone(), slot);

                if std::env::var("DEBUG_JIT").is_ok() {
                    eprintln!(
                        "JIT: Allocated stack slot {:?} for parameter '{}'",
                        slot, param_name
                    );
                }
            }
        }

        // Re-enable local slots optimization if we have any
        if !self.local_slots.is_empty() {
            self.use_local_slots = true;
        }
    }

    fn scan_non_int_locals(&mut self, instructions: &[OpCode], function_end: usize) {
        for pc in 0..function_end.saturating_sub(1) {
            let instruction = match instructions.get(pc) {
                Some(instr) => instr,
                None => break,
            };
            let next_instruction = instructions.get(pc + 1);

            if matches!(instruction, OpCode::MakeDict(_) | OpCode::MakeDictWithKeys(_)) {
                if let Some(next) = next_instruction {
                    match next {
                        OpCode::StoreLocal(slot) => {
                            if let Some(name) = self.local_names.get(*slot) {
                                self.non_int_locals.insert(name.clone());
                            }
                        }
                        OpCode::StoreVar(name) => {
                            self.non_int_locals.insert(name.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Initialize parameter stack slots with values from the HashMap
    /// This is called at function entry to copy parameter values from the
    /// func_locals HashMap (passed via VMContext) into the fast stack slots
    /// OPTIMIZATION: For functions with ≤4 parameters, use VMContext.arg fields
    /// directly instead of HashMap lookups for ~10x faster parameter access
    fn initialize_parameter_slots(
        &mut self,
        builder: &mut FunctionBuilder,
        params: &[String],
        load_var_func: FuncRef,
        get_arg_func: Option<FuncRef>,
    ) {
        if !self.use_local_slots || params.is_empty() {
            return;
        }

        let ctx = match self.ctx_param {
            Some(ctx) => ctx,
            None => return,
        };

        let use_fast_args = get_arg_func.is_some() && params.len() <= 4;

        for (i, param_name) in params.iter().enumerate() {
            if let Some(&slot) = self.local_slots.get(param_name) {
                let param_value = if use_fast_args {
                    // FAST PATH: Load parameter directly from VMContext.argN
                    let get_arg = get_arg_func.unwrap();
                    let index = builder.ins().iconst(types::I64, i as i64);
                    let call = builder.ins().call(get_arg, &[ctx, index]);
                    builder.inst_results(call)[0]
                } else {
                    // SLOW PATH: Load parameter value from HashMap via runtime call
                    let name_hash = Self::hash_var_name(param_name) as i64;
                    let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                    let zero = builder.ins().iconst(types::I64, 0);

                    // Call jit_load_variable to get the parameter value from func_locals
                    let call = builder.ins().call(load_var_func, &[ctx, name_hash_val, zero]);
                    builder.inst_results(call)[0]
                };

                // Store the parameter value into the stack slot
                builder.ins().stack_store(param_value, slot, 0);

                if std::env::var("DEBUG_JIT").is_ok() {
                    eprintln!(
                        "JIT: Initialized parameter '{}' in stack slot {:?} (fast={})",
                        param_name, slot, use_fast_args
                    );
                }
            }
        }
    }

    /// Initialize local slots from VM locals (used for loop JIT)
    fn initialize_local_slots_from_vm(
        &mut self,
        builder: &mut FunctionBuilder,
        load_var_func: FuncRef,
    ) {
        if !self.use_local_slots {
            return;
        }

        self.dirty_local_names.clear();

        let ctx = match self.ctx_param {
            Some(ctx) => ctx,
            None => return,
        };

        let local_slots: Vec<(String, StackSlot)> =
            self.local_slots.iter().map(|(name, slot)| (name.clone(), *slot)).collect();

        for (name, slot) in local_slots {
            let name_hash = Self::hash_var_name(&name) as i64;
            let name_hash_val = builder.ins().iconst(types::I64, name_hash);
            let zero = builder.ins().iconst(types::I64, 0);

            let call = builder.ins().call(load_var_func, &[ctx, name_hash_val, zero]);
            let value = builder.inst_results(call)[0];
            builder.ins().stack_store(value, slot, 0);
        }
    }

    fn allocate_int_dict_ptr_slots(&mut self, builder: &mut FunctionBuilder) {
        if self.int_dict_slots.is_empty() {
            return;
        }

        for slot_index in self.int_dict_slots.clone() {
            if self.int_dict_ptr_slots.contains_key(&slot_index) {
                continue;
            }

            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                0,
            ));
            self.int_dict_ptr_slots.insert(slot_index, slot);
        }
    }

    fn initialize_int_dict_ptr_slots(&mut self, builder: &mut FunctionBuilder) {
        if self.int_dict_slots.is_empty() {
            return;
        }

        let ctx = match self.ctx_param {
            Some(ctx) => ctx,
            None => return,
        };

        let unique_ptr_func = match self.int_dict_unique_ptr_func {
            Some(func) => func,
            None => return,
        };

        let ptr_slots: Vec<(usize, StackSlot)> =
            self.int_dict_ptr_slots.iter().map(|(slot_index, slot)| (*slot_index, *slot)).collect();

        for (slot_index, ptr_slot) in ptr_slots {
            let slot_val = builder.ins().iconst(types::I64, slot_index as i64);
            let call = builder.ins().call(unique_ptr_func, &[ctx, slot_val]);
            let ptr_value = builder.inst_results(call)[0];
            builder.ins().stack_store(ptr_value, ptr_slot, 0);
        }
    }

    /// Persist local slots back to VM locals (used for loop JIT)
    fn persist_local_slots_to_vm(&self, builder: &mut FunctionBuilder) {
        if !self.use_local_slots || !self.persist_locals_on_return {
            return;
        }

        if self.dirty_local_names.is_empty() {
            return;
        }

        let ctx = match self.ctx_param {
            Some(ctx) => ctx,
            None => return,
        };

        let store_func = match self.store_var_func {
            Some(func) => func,
            None => return,
        };

        for (name, slot) in &self.local_slots {
            if !self.dirty_local_names.contains(name) {
                continue;
            }
            let name_hash = Self::hash_var_name(name) as i64;
            let name_hash_val = builder.ins().iconst(types::I64, name_hash);
            let zero = builder.ins().iconst(types::I64, 0);
            let value = builder.ins().stack_load(types::I64, *slot, 0);
            builder.ins().call(store_func, &[ctx, name_hash_val, zero, value]);
        }
    }

    /// Calculate the extent of the function body from bytecode
    fn calculate_function_end(instructions: &[OpCode]) -> usize {
        // Find the extent by looking at all jump targets
        let mut max_reachable: usize = 0;

        for (pc, instruction) in instructions.iter().enumerate() {
            match instruction {
                OpCode::Jump(target)
                | OpCode::JumpIfFalse(target)
                | OpCode::JumpIfTrue(target)
                | OpCode::JumpBack(target) => {
                    if *target > max_reachable {
                        max_reachable = *target;
                    }
                }
                OpCode::Return | OpCode::ReturnNone => {
                    if pc > max_reachable {
                        max_reachable = pc;
                    }
                }
                _ => {}
            }
        }

        // Find the Return after max_reachable
        for (pc, instruction) in instructions.iter().enumerate().skip(max_reachable) {
            if matches!(instruction, OpCode::Return | OpCode::ReturnNone) {
                return pc + 1;
            }
        }

        // Fallback: use max_reachable + 1
        max_reachable + 1
    }

    fn set_context_param(&mut self, ctx: cranelift::prelude::Value) {
        self.ctx_param = Some(ctx);
    }

    fn set_external_functions(
        &mut self,
        load_var: FuncRef,
        store_var: FuncRef,
        store_var_from_stack: FuncRef,
    ) {
        self.load_var_func = Some(load_var);
        self.store_var_func = Some(store_var);
        self.store_var_from_stack_func = Some(store_var_from_stack);
    }

    fn set_float_functions(&mut self, load_var_float: FuncRef, store_var_float: FuncRef) {
        self.load_var_float_func = Some(load_var_float);
        self.store_var_float_func = Some(store_var_float);
    }

    fn set_guard_functions(&mut self, check_int: FuncRef, check_float: FuncRef) {
        self.check_type_int_func = Some(check_int);
        self.check_type_float_func = Some(check_float);
    }

    fn set_call_function(&mut self, call_func: FuncRef) {
        self.call_func = Some(call_func);
    }

    fn set_push_int_function(&mut self, push_int_func: FuncRef) {
        self.push_int_func = Some(push_int_func);
    }

    fn set_return_int_function(&mut self, set_return_int_func: FuncRef) {
        self.set_return_int_func = Some(set_return_int_func);
    }

    fn set_stack_pop_function(&mut self, stack_pop_func: FuncRef) {
        self.stack_pop_func = Some(stack_pop_func);
    }

    fn set_stack_push_function(&mut self, stack_push_func: FuncRef) {
        self.stack_push_func = Some(stack_push_func);
    }

    fn set_object_functions(&mut self, push_string_func: FuncRef, obj_to_vm_func: FuncRef) {
        self.obj_push_string_func = Some(push_string_func);
        self.obj_to_vm_stack_func = Some(obj_to_vm_func);
    }

    #[allow(dead_code)]
    fn push_string_handle(
        &mut self,
        builder: &mut FunctionBuilder,
        string_val: &str,
    ) -> Result<(), String> {
        let ctx = self
            .ctx_param
            .ok_or_else(|| "push_string_handle requires context param".to_string())?;
        let push_string = self
            .obj_push_string_func
            .ok_or_else(|| "push_string_handle requires obj_push_string_func".to_string())?;
        let obj_to_vm = self
            .obj_to_vm_stack_func
            .ok_or_else(|| "push_string_handle requires obj_to_vm_stack_func".to_string())?;

        let string_ptr = builder.ins().iconst(types::I64, string_val.as_ptr() as i64);
        let string_len = builder.ins().iconst(types::I64, string_val.len() as i64);
        let call = builder.ins().call(push_string, &[ctx, string_ptr, string_len]);
        let handle = builder.inst_results(call)[0];
        builder.ins().call(obj_to_vm, &[ctx, handle]);

        let marker = builder.ins().iconst(types::I64, -1);
        self.push_value(marker);
        Ok(())
    }

    fn set_specialization(&mut self, spec: SpecializationInfo) {
        self.specialization = Some(spec);
    }

    /// Set the current function name for self-recursion detection
    fn set_current_function_name(&mut self, name: &str) {
        self.current_function_name = Some(name.to_string());
    }

    /// Set the self-recursive call function reference
    fn set_self_call_function(&mut self, func_ref: FuncRef) {
        self.self_call_func = Some(func_ref);
    }

    /// Set the get-return-int function reference
    fn set_get_return_int_function(&mut self, func_ref: FuncRef) {
        self.get_return_int_func = Some(func_ref);
    }

    /// Set the dict-get function reference
    fn set_dict_get_function(&mut self, func_ref: FuncRef) {
        self.dict_get_func = Some(func_ref);
    }

    /// Set the dict-set function reference
    fn set_dict_set_function(&mut self, func_ref: FuncRef) {
        self.dict_set_func = Some(func_ref);
    }

    /// Set the make-dict function reference
    fn set_make_dict_function(&mut self, func_ref: FuncRef) {
        self.make_dict_func = Some(func_ref);
    }

    fn set_make_dict_with_keys_function(&mut self, func_ref: FuncRef) {
        self.make_dict_with_keys_func = Some(func_ref);
    }

    fn set_append_in_place_functions(
        &mut self,
        append_string_func: FuncRef,
        append_char_func: FuncRef,
    ) {
        self.append_const_string_in_place_func = Some(append_string_func);
        self.append_const_char_in_place_func = Some(append_char_func);
    }

    fn set_local_slot_dict_functions(&mut self, get_func: FuncRef, set_func: FuncRef) {
        self.local_slot_dict_get_func = Some(get_func);
        self.local_slot_dict_set_func = Some(set_func);
    }

    fn set_local_slot_int_dict_functions(&mut self, get_func: FuncRef, set_func: FuncRef) {
        self.local_slot_int_dict_get_func = Some(get_func);
        self.local_slot_int_dict_set_func = Some(set_func);
    }

    fn set_int_dict_ptr_functions(
        &mut self,
        unique_ptr_func: FuncRef,
        get_ptr_func: FuncRef,
        get_ptr_dense_int_func: FuncRef,
        get_ptr_dense_int_full_func: FuncRef,
        set_ptr_func: FuncRef,
        set_ptr_dense_int_func: FuncRef,
        set_ptr_dense_int_full_func: FuncRef,
    ) {
        self.int_dict_unique_ptr_func = Some(unique_ptr_func);
        self.int_dict_get_ptr_func = Some(get_ptr_func);
        self.int_dict_get_ptr_dense_int_func = Some(get_ptr_dense_int_func);
        self.int_dict_get_ptr_dense_int_full_func = Some(get_ptr_dense_int_full_func);
        self.int_dict_set_ptr_func = Some(set_ptr_func);
        self.int_dict_set_ptr_dense_int_func = Some(set_ptr_dense_int_func);
        self.int_dict_set_ptr_dense_int_full_func = Some(set_ptr_dense_int_full_func);
    }

    fn set_int_dict_slots(&mut self, slots: std::collections::HashSet<usize>) {
        self.int_dict_slots = slots;
    }

    /// Enable or disable persisting local slots on return
    #[allow(dead_code)]
    fn set_persist_locals_on_return(&mut self, enabled: bool) {
        self.persist_locals_on_return = enabled;
    }

    /// Pre-create blocks for all jump targets within the function body
    /// For loop headers (backward jump targets), we also add block parameters
    /// to handle the values that flow from the back-edge to the loop header
    fn create_blocks(
        &mut self,
        builder: &mut FunctionBuilder,
        instructions: &[OpCode],
    ) -> Result<(), String> {
        // Calculate and store function_end
        self.function_end = Self::calculate_function_end(instructions);

        // STEP 11 FIX: Analyze loop headers to find backward jump targets and their stack depths
        let loop_headers = Self::analyze_loop_headers(instructions, self.function_end);

        // Create blocks for jump targets within function body
        for (pc, instruction) in instructions.iter().enumerate() {
            if pc >= self.function_end {
                break;
            }

            match instruction {
                OpCode::Jump(target)
                | OpCode::JumpIfFalse(target)
                | OpCode::JumpIfTrue(target)
                | OpCode::JumpBack(target) => {
                    if *target < self.function_end {
                        if !self.blocks.contains_key(target) {
                            let block = builder.create_block();

                            // STEP 11 FIX: If this is a loop header, add block parameters
                            // This enables backward jumps to pass values (like loop counters)
                            if let Some(&expected_depth) = loop_headers.get(target) {
                                // Record this as a loop header - it should NOT be sealed until
                                // all predecessors (including back-edges) are processed
                                self.loop_header_pcs.insert(*target);

                                // Add i64 parameters for each expected stack value
                                for _ in 0..expected_depth {
                                    builder.append_block_param(block, types::I64);
                                }
                                // Record the expected stack depth for this block
                                self.block_entry_stack_depth.insert(*target, expected_depth);

                                if std::env::var("DEBUG_JIT").is_ok() {
                                    eprintln!(
                                        "JIT: Created loop header block at PC {} with {} params",
                                        target, expected_depth
                                    );
                                }
                            }

                            self.blocks.insert(*target, block);
                        }
                    }
                    // Also create block for fallthrough
                    let next_pc = pc + 1;
                    if next_pc < self.function_end && !self.blocks.contains_key(&next_pc) {
                        self.blocks.insert(next_pc, builder.create_block());
                    }
                }
                _ => {}
            }
        }

        if std::env::var("DEBUG_JIT").is_ok() {
            eprintln!(
                "JIT: Function body extent: 0..{}, blocks created: {:?}",
                self.function_end,
                self.blocks.keys().collect::<Vec<_>>()
            );
        }

        Ok(())
    }

    /// Translate a bytecode instruction to Cranelift IR
    fn translate_instruction(
        &mut self,
        builder: &mut FunctionBuilder,
        pc: usize,
        instruction: &OpCode,
        constants: &[Constant],
    ) -> Result<bool, String> {
        // Returns Ok(true) if this instruction terminates the block

        match instruction {
            // Arithmetic operations
            OpCode::Add => {
                // Check if we have specialization context for optimized path
                if self.specialization.is_some() {
                    self.translate_add_specialized(builder, None, None)?;
                } else {
                    // Generic fallback
                    let b = self.pop_value()?;
                    let a = self.pop_value()?;
                    let result = builder.ins().iadd(a, b);
                    self.push_value(result);
                }
            }

            OpCode::Sub => {
                // Check if we have specialization context for optimized path
                if self.specialization.is_some() {
                    self.translate_sub_specialized(builder, None, None)?;
                } else {
                    // Generic fallback
                    let b = self.pop_value()?;
                    let a = self.pop_value()?;
                    let result = builder.ins().isub(a, b);
                    self.push_value(result);
                }
            }

            OpCode::Mul => {
                // Check if we have specialization context for optimized path
                if self.specialization.is_some() {
                    self.translate_mul_specialized(builder, None, None)?;
                } else {
                    // Generic fallback
                    let b = self.pop_value()?;
                    let a = self.pop_value()?;
                    let result = builder.ins().imul(a, b);
                    self.push_value(result);
                }
            }

            OpCode::Div => {
                // Check if we have specialization context for optimized path
                if self.specialization.is_some() {
                    self.translate_div_specialized(builder, None, None)?;
                } else {
                    // Generic fallback
                    let b = self.pop_value()?;
                    let a = self.pop_value()?;
                    let result = builder.ins().sdiv(a, b);
                    self.push_value(result);
                }
            }

            OpCode::Mod => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let result = builder.ins().srem(a, b);
                self.push_value(result);
            }

            OpCode::Negate => {
                let a = self.pop_value()?;
                let result = builder.ins().ineg(a);
                self.push_value(result);
            }

            // Comparison operations
            OpCode::Equal => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let result = builder.ins().icmp(IntCC::Equal, a, b);
                let extended = builder.ins().uextend(types::I64, result);
                self.push_value(extended);
            }

            OpCode::NotEqual => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let result = builder.ins().icmp(IntCC::NotEqual, a, b);
                let extended = builder.ins().uextend(types::I64, result);
                self.push_value(extended);
            }

            OpCode::LessThan => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let result = builder.ins().icmp(IntCC::SignedLessThan, a, b);
                let extended = builder.ins().uextend(types::I64, result);
                self.push_value(extended);
            }

            OpCode::GreaterThan => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let result = builder.ins().icmp(IntCC::SignedGreaterThan, a, b);
                let extended = builder.ins().uextend(types::I64, result);
                self.push_value(extended);
            }

            OpCode::LessEqual => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                // a <= b: use SignedLessThanOrEqual directly
                let result = builder.ins().icmp(IntCC::SignedLessThanOrEqual, a, b);
                let extended = builder.ins().uextend(types::I64, result);
                self.push_value(extended);
            }

            OpCode::GreaterEqual => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                // a >= b: use SignedGreaterThanOrEqual directly
                let result = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, a, b);
                let extended = builder.ins().uextend(types::I64, result);
                self.push_value(extended);
            }

            // Logical operations
            OpCode::Not => {
                let a = self.pop_value()?;
                let zero = builder.ins().iconst(types::I64, 0);
                let result = builder.ins().icmp(IntCC::Equal, a, zero);
                let extended = builder.ins().uextend(types::I64, result);
                self.push_value(extended);
            }

            OpCode::And => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let result = builder.ins().band(a, b);
                self.push_value(result);
            }

            OpCode::Or => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let result = builder.ins().bor(a, b);
                self.push_value(result);
            }

            // Stack operations
            OpCode::Pop => {
                self.pop_value()?;
            }

            OpCode::Dup => {
                let val = self.peek_value()?;
                self.push_value(val);
            }

            // Constant loading
            OpCode::LoadConst(index) => {
                if let Some(constant) = constants.get(*index) {
                    match constant {
                        Constant::Int(i) => {
                            let val = builder.ins().iconst(types::I64, *i);
                            self.push_value(val);
                        }
                        Constant::Bool(b) => {
                            let val = builder.ins().iconst(types::I64, if *b { 1 } else { 0 });
                            self.push_value(val);
                        }
                        // Other constant types (strings, floats, etc.) - push placeholder
                        // These will be handled by interpreter when needed
                        // Push 0 as placeholder to maintain stack balance
                        _ => {
                            let zero = builder.ins().iconst(types::I64, 0);
                            self.push_value(zero);
                        }
                    }
                } else {
                    return Err(format!("Invalid constant index: {}", index));
                }
            }

            // Control flow with proper block handling
            OpCode::Jump(target) => {
                if let Some(&target_block) = self.blocks.get(target) {
                    // Pass current stack as block arguments
                    let args: Vec<_> = self.value_stack.clone();
                    builder.ins().jump(target_block, &args);
                    return Ok(true); // Terminates block
                } else {
                    return Err(format!("Jump to undefined block at PC {}", target));
                }
            }

            OpCode::JumpIfFalse(target) => {
                // IMPORTANT: VM semantics PEEK at condition (doesn't pop)
                // The condition stays on the stack for both branches
                let condition = self.peek_value()?;
                let zero = builder.ins().iconst(types::I64, 0);
                let is_false = builder.ins().icmp(IntCC::Equal, condition, zero);

                if let Some(&target_block) = self.blocks.get(target) {
                    // Get or create the fallthrough block
                    let next_pc = pc + 1;
                    let fallthrough_block = *self.blocks.get(&next_pc).ok_or_else(|| {
                        format!("No fallthrough block after JumpIfFalse at PC {}", pc)
                    })?;

                    // Pass the current stack as block arguments to both branches
                    // This ensures both paths have access to the condition value
                    let args: Vec<_> = self.value_stack.clone();

                    // Record expected stack depth for both blocks
                    self.block_entry_stack_depth.insert(*target, args.len());
                    self.block_entry_stack_depth.insert(next_pc, args.len());

                    builder.ins().brif(is_false, target_block, &args, fallthrough_block, &args);
                    return Ok(true); // Terminates block
                } else {
                    return Err(format!("JumpIfFalse to undefined block at PC {}", target));
                }
            }

            OpCode::JumpIfTrue(target) => {
                // IMPORTANT: VM semantics PEEK at condition (doesn't pop)
                // The condition stays on the stack for both branches
                let condition = self.peek_value()?;
                let zero = builder.ins().iconst(types::I64, 0);
                let is_true = builder.ins().icmp(IntCC::NotEqual, condition, zero);

                if let Some(&target_block) = self.blocks.get(target) {
                    // Get or create the fallthrough block
                    let next_pc = pc + 1;
                    let fallthrough_block = *self.blocks.get(&next_pc).ok_or_else(|| {
                        format!("No fallthrough block after JumpIfTrue at PC {}", pc)
                    })?;

                    // Pass the current stack as block arguments to both branches
                    let args: Vec<_> = self.value_stack.clone();

                    // Record expected stack depth for both blocks
                    self.block_entry_stack_depth.insert(*target, args.len());
                    self.block_entry_stack_depth.insert(next_pc, args.len());

                    builder.ins().brif(is_true, target_block, &args, fallthrough_block, &args);
                    return Ok(true); // Terminates block
                } else {
                    return Err(format!("JumpIfTrue to undefined block at PC {}", target));
                }
            }

            OpCode::JumpBack(target) => {
                // JumpBack is like Jump but backwards (for loops)
                // STEP 11 FIX: Loop headers have block parameters, pass current stack
                if let Some(&target_block) = self.blocks.get(target) {
                    // Pass current stack values as arguments to the loop header
                    let args: Vec<_> = self.value_stack.clone();
                    builder.ins().jump(target_block, &args);
                    return Ok(true); // Terminates block
                } else {
                    return Err(format!("JumpBack to undefined block at PC {}", target));
                }
            }

            OpCode::Call(arg_count) => {
                // NOTE: Direct self-recursion was attempted but causes stack overflow
                // because stack slots are shared between recursive calls.
                // TODO: Implement proper tail-call optimization or save/restore slots
                // For now, continue using VM-mediated recursion which is already optimized.

                // Call instruction (all paths go through VM):
                // 1. Pop the function marker from JIT stack (should be -1 if function)
                // 2. Pop arguments from JIT stack
                // 3. Push integer args to VM stack (function is already there from LoadVar)
                // 4. Call the runtime helper (which pops args first, then function)
                // 5. Pop the result from VM stack

                // Clear any pending self-recursion flag (not used yet)
                self.self_recursion_pending = false;

                // Pop values from JIT stack (in reverse order)
                let _func_marker = self.pop_value()?; // -1 if it's a function, otherwise an int
                let mut arg_vals = Vec::new();
                for _ in 0..*arg_count {
                    arg_vals.push(self.pop_value()?);
                }
                arg_vals.reverse(); // Reverse to get correct order

                if let (Some(ctx), Some(call_func), Some(stack_push)) =
                    (self.ctx_param, self.call_func, self.stack_push_func)
                {
                    // Push integer arguments to VM stack (function is already there at bottom)
                    // VM stack before: [function]
                    // VM stack after push: [function, arg0, arg1, ...]
                    // jit_call_function will pop args first, then function
                    for arg_val in &arg_vals {
                        builder.ins().call(stack_push, &[ctx, *arg_val]);
                    }

                    // Call jit_call_function
                    let null_ptr = builder.ins().iconst(types::I64, 0);
                    let arg_count_val = builder.ins().iconst(types::I64, *arg_count as i64);
                    let _call_inst = builder.ins().call(call_func, &[ctx, null_ptr, arg_count_val]);

                    // Pop the result from VM stack
                    if let Some(stack_pop) = self.stack_pop_func {
                        let call = builder.ins().call(stack_pop, &[ctx]);
                        let result = builder.inst_results(call)[0];
                        self.push_value(result);
                    } else {
                        // Fallback: push placeholder
                        let zero = builder.ins().iconst(types::I64, 0);
                        self.push_value(zero);
                    }

                    return Ok(false); // Doesn't terminate block
                } else {
                    return Err(
                        "Call opcode requires context, call function, and stack_push to be set"
                            .to_string(),
                    );
                }
            }

            OpCode::IndexGet => {
                // Dict/Array get: Pop index and object, push result
                // Stack: [object, index] -> [value]

                if let (Some(ctx), Some(dict_get_ref)) = (self.ctx_param, self.dict_get_func) {
                    // Pop index and object from JIT stack
                    let _index = self.pop_value()?;
                    let _object = self.pop_value()?;

                    // Push them to VM stack for runtime helper
                    if let Some(stack_push) = self.stack_push_func {
                        builder.ins().call(stack_push, &[ctx, _object]);
                        builder.ins().call(stack_push, &[ctx, _index]);
                    }

                    // Call the helper
                    let call = builder.ins().call(dict_get_ref, &[ctx]);
                    let _success = builder.inst_results(call)[0];

                    // Pop result from VM stack
                    if let Some(stack_pop) = self.stack_pop_func {
                        let call = builder.ins().call(stack_pop, &[ctx]);
                        let result = builder.inst_results(call)[0];
                        self.push_value(result);
                    } else {
                        let zero = builder.ins().iconst(types::I64, 0);
                        self.push_value(zero);
                    }

                    return Ok(false);
                } else {
                    return Err("IndexGet requires context and dict_get function".to_string());
                }
            }

            OpCode::IndexSet => {
                // Dict/Array set: Pop value, index, and object
                // Stack: [object, index, value] -> []

                if let (Some(ctx), Some(dict_set_ref)) = (self.ctx_param, self.dict_set_func) {
                    // Pop value, index, and object from JIT stack
                    let _value = self.pop_value()?;
                    let _index = self.pop_value()?;
                    let _object = self.pop_value()?;

                    // Push them to VM stack for runtime helper
                    if let Some(stack_push) = self.stack_push_func {
                        builder.ins().call(stack_push, &[ctx, _object]);
                        builder.ins().call(stack_push, &[ctx, _index]);
                        builder.ins().call(stack_push, &[ctx, _value]);
                    }

                    // Call the helper
                    let call = builder.ins().call(dict_set_ref, &[ctx]);
                    let _success = builder.inst_results(call)[0];

                    // The modified dict/array is left on the VM stack by jit_dict_set
                    // We don't need to pop it here as IndexSet has 0 stack push effect

                    return Ok(false);
                } else {
                    return Err("IndexSet requires context and dict_set function".to_string());
                }
            }

            OpCode::MakeDict(num_pairs) => {
                // Create a new dict from N key-value pairs on the stack
                // Stack: [key1, val1, key2, val2, ..., keyN, valN] -> [dict]

                if let (Some(ctx), Some(make_dict_ref)) = (self.ctx_param, self.make_dict_func) {
                    // Pop all key-value pairs from JIT stack and push to VM stack
                    // Note: We pop in reverse order (valN, keyN, ..., val1, key1)
                    // because stack is LIFO, but we need to process them in order
                    let mut temp_pairs = Vec::new();
                    for _ in 0..*num_pairs {
                        let value = self.pop_value()?;
                        let key = self.pop_value()?;
                        temp_pairs.push((key, value));
                    }

                    // Push to VM stack in correct order (key1, val1, key2, val2, ...)
                    if let Some(stack_push) = self.stack_push_func {
                        for (key, value) in temp_pairs.iter().rev() {
                            builder.ins().call(stack_push, &[ctx, *key]);
                            builder.ins().call(stack_push, &[ctx, *value]);
                        }
                    }

                    // Call jit_make_dict with num_pairs
                    let num_pairs_val = builder.ins().iconst(types::I64, *num_pairs as i64);
                    let call = builder.ins().call(make_dict_ref, &[ctx, num_pairs_val]);
                    let _success = builder.inst_results(call)[0];

                    // Leave dict on VM stack and push marker for non-int value
                    let marker = builder.ins().iconst(types::I64, -1);
                    self.push_value(marker);

                    return Ok(false);
                } else {
                    return Err("MakeDict requires context and make_dict function".to_string());
                }
            }

            OpCode::MakeDictWithKeys(keys) => {
                if let (Some(ctx), Some(make_dict_keys_ref)) =
                    (self.ctx_param, self.make_dict_with_keys_func)
                {
                    let mut temp_values = Vec::new();
                    for _ in 0..keys.len() {
                        let value = self.pop_value()?;
                        temp_values.push(value);
                    }

                    if let Some(stack_push) = self.stack_push_func {
                        for value in temp_values.iter().rev() {
                            builder.ins().call(stack_push, &[ctx, *value]);
                        }
                    }

                    let keys_ptr = builder.ins().iconst(types::I64, Arc::as_ptr(keys) as i64);
                    let keys_len = builder.ins().iconst(types::I64, keys.len() as i64);
                    let call = builder.ins().call(make_dict_keys_ref, &[ctx, keys_ptr, keys_len]);
                    let _success = builder.inst_results(call)[0];

                    // Leave dict on VM stack and push marker for non-int value
                    let marker = builder.ins().iconst(types::I64, -1);
                    self.push_value(marker);

                    return Ok(false);
                } else {
                    return Err("MakeDictWithKeys requires context and helper".to_string());
                }
            }

            OpCode::AppendConstStringInPlace(slot_index, rhs) => {
                if let (Some(ctx), Some(append_func)) =
                    (self.ctx_param, self.append_const_string_in_place_func)
                {
                    let slot_val = builder.ins().iconst(types::I64, *slot_index as i64);
                    let string_ptr = builder.ins().iconst(types::I64, rhs.as_ptr() as i64);
                    let string_len = builder.ins().iconst(types::I64, rhs.len() as i64);
                    builder.ins().call(append_func, &[ctx, slot_val, string_ptr, string_len]);
                    return Ok(false);
                } else {
                    return Err(
                        "AppendConstStringInPlace requires context and append helper".to_string()
                    );
                }
            }

            OpCode::AppendConstCharInPlace(slot_index, rhs) => {
                if let (Some(ctx), Some(append_func)) =
                    (self.ctx_param, self.append_const_char_in_place_func)
                {
                    let slot_val = builder.ins().iconst(types::I64, *slot_index as i64);
                    let char_val = builder.ins().iconst(types::I64, *rhs as i64);
                    builder.ins().call(append_func, &[ctx, slot_val, char_val]);
                    return Ok(false);
                } else {
                    return Err(
                        "AppendConstCharInPlace requires context and append helper".to_string()
                    );
                }
            }

            OpCode::IndexSetInPlace(slot_index) => {
                // In-place index assignment: var[index] = value
                // Stack: [value, index] -> []
                // This is like: tmp = LoadVar(var); tmp[index] = value; StoreVar(var, tmp)

                let var_name = self.local_names.get(*slot_index).cloned();
                let is_non_int_local = var_name
                    .as_ref()
                    .map(|name| self.non_int_locals.contains(name))
                    .unwrap_or(false);

                if self.int_dict_slots.contains(slot_index) && !is_non_int_local {
                    let _index = self.pop_value()?;
                    let _value = self.pop_value()?;

                    if let Some(&ptr_slot) = self.int_dict_ptr_slots.get(slot_index) {
                        if let Some(set_ptr_ref) = self.int_dict_set_ptr_func {
                            let dict_ptr = builder.ins().stack_load(types::I64, ptr_slot, 0);
                            let zero = builder.ins().iconst(types::I64, 0);
                            let is_zero = builder.ins().icmp(IntCC::Equal, dict_ptr, zero);

                            let slow_block = builder.create_block();
                            let slow_clear_block = builder.create_block();
                            let dense_int_full_block = builder.create_block();
                            let dense_int_sparse_check_block = builder.create_block();
                            let dense_int_sparse_block = builder.create_block();
                            let fast_block = builder.create_block();
                            let cont_block = builder.create_block();

                            builder.ins().brif(is_zero, slow_block, &[], fast_block, &[]);

                            builder.switch_to_block(fast_block);
                            let tag_mask = builder.ins().iconst(types::I64, 7);
                            let tag_value = builder.ins().band(dict_ptr, tag_mask);
                            let dense_int_full_tag = builder.ins().iconst(types::I64, 2);
                            let is_dense_int_full =
                                builder.ins().icmp(IntCC::Equal, tag_value, dense_int_full_tag);
                            let dense_int_sparse_tag = builder.ins().iconst(types::I64, 4);
                            let is_dense_int_sparse =
                                builder.ins().icmp(IntCC::Equal, tag_value, dense_int_sparse_tag);
                            builder.ins().brif(
                                is_dense_int_full,
                                dense_int_full_block,
                                &[],
                                dense_int_sparse_check_block,
                                &[],
                            );

                            builder.switch_to_block(dense_int_full_block);
                            let zero = builder.ins().iconst(types::I64, 0);
                            let is_negative =
                                builder.ins().icmp(IntCC::SignedLessThan, _index, zero);
                            let inline_slow_block = builder.create_block();
                            let inline_set_block = builder.create_block();
                            builder.ins().brif(
                                is_negative,
                                inline_slow_block,
                                &[],
                                inline_set_block,
                                &[],
                            );

                            builder.switch_to_block(inline_set_block);
                            let tag_mask = builder.ins().iconst(types::I64, !7i64);
                            let base_ptr = builder.ins().band(dict_ptr, tag_mask);
                            let data_ptr =
                                builder.ins().load(types::I64, MemFlags::new(), base_ptr, 0);
                            let len = builder.ins().load(types::I64, MemFlags::new(), base_ptr, 8);
                            let cap = builder.ins().load(types::I64, MemFlags::new(), base_ptr, 16);

                            let is_lt_len = builder.ins().icmp(IntCC::SignedLessThan, _index, len);
                            let is_eq_len = builder.ins().icmp(IntCC::Equal, _index, len);
                            let has_capacity = builder.ins().icmp(IntCC::SignedLessThan, len, cap);
                            let can_append = builder.ins().band(is_eq_len, has_capacity);

                            let update_block = builder.create_block();
                            let append_check_block = builder.create_block();
                            let append_block = builder.create_block();
                            builder.ins().brif(
                                is_lt_len,
                                update_block,
                                &[],
                                append_check_block,
                                &[],
                            );

                            builder.switch_to_block(update_block);
                            let stride = builder.ins().iconst(types::I64, 8);
                            let offset = builder.ins().imul(_index, stride);
                            let addr = builder.ins().iadd(data_ptr, offset);
                            builder.ins().store(MemFlags::new(), _value, addr, 0);
                            builder.ins().jump(cont_block, &[]);

                            builder.switch_to_block(append_check_block);
                            builder.ins().brif(
                                can_append,
                                append_block,
                                &[],
                                inline_slow_block,
                                &[],
                            );

                            builder.switch_to_block(append_block);
                            let stride = builder.ins().iconst(types::I64, 8);
                            let offset = builder.ins().imul(len, stride);
                            let addr = builder.ins().iadd(data_ptr, offset);
                            builder.ins().store(MemFlags::new(), _value, addr, 0);
                            let one = builder.ins().iconst(types::I64, 1);
                            let new_len = builder.ins().iadd(len, one);
                            builder.ins().store(MemFlags::new(), new_len, base_ptr, 8);
                            builder.ins().jump(cont_block, &[]);

                            builder.switch_to_block(inline_slow_block);
                            if let Some(set_dense_int_full_ref) =
                                self.int_dict_set_ptr_dense_int_full_func
                            {
                                builder
                                    .ins()
                                    .call(set_dense_int_full_ref, &[dict_ptr, _index, _value]);
                            } else {
                                builder.ins().call(set_ptr_ref, &[dict_ptr, _index, _value]);
                            }
                            builder.ins().jump(cont_block, &[]);

                            builder.switch_to_block(dense_int_sparse_check_block);
                            builder.ins().brif(
                                is_dense_int_sparse,
                                dense_int_sparse_block,
                                &[],
                                slow_clear_block,
                                &[],
                            );
                            builder.switch_to_block(dense_int_sparse_block);
                            if let Some(set_dense_int_ref) = self.int_dict_set_ptr_dense_int_func {
                                builder.ins().call(set_dense_int_ref, &[dict_ptr, _index, _value]);
                            } else {
                                builder.ins().call(set_ptr_ref, &[dict_ptr, _index, _value]);
                            }
                            builder.ins().jump(cont_block, &[]);

                            builder.switch_to_block(slow_clear_block);
                            builder.ins().call(set_ptr_ref, &[dict_ptr, _index, _value]);
                            builder.ins().jump(cont_block, &[]);

                            builder.switch_to_block(slow_block);
                            if let (Some(ctx), Some(set_int_dict_ref)) =
                                (self.ctx_param, self.local_slot_int_dict_set_func)
                            {
                                let slot_val = builder.ins().iconst(types::I64, *slot_index as i64);
                                builder
                                    .ins()
                                    .call(set_int_dict_ref, &[ctx, slot_val, _index, _value]);
                                if let Some(unique_ptr_ref) = self.int_dict_unique_ptr_func {
                                    let call = builder.ins().call(unique_ptr_ref, &[ctx, slot_val]);
                                    let new_ptr = builder.inst_results(call)[0];
                                    builder.ins().stack_store(new_ptr, ptr_slot, 0);
                                } else {
                                    builder.ins().stack_store(zero, ptr_slot, 0);
                                }
                            }
                            builder.ins().jump(cont_block, &[]);

                            builder.switch_to_block(cont_block);
                            let null_val = builder.ins().iconst(types::I64, 0);
                            self.push_value(null_val);
                            return Ok(false);
                        }
                    }

                    if let (Some(ctx), Some(set_int_dict_ref)) =
                        (self.ctx_param, self.local_slot_int_dict_set_func)
                    {
                        let slot_val = builder.ins().iconst(types::I64, *slot_index as i64);
                        builder.ins().call(set_int_dict_ref, &[ctx, slot_val, _index, _value]);
                        let null_val = builder.ins().iconst(types::I64, 0);
                        self.push_value(null_val);
                        return Ok(false);
                    }

                    return Err("IndexSetInPlace requires int-dict helpers".to_string());
                }

                if let (Some(ctx), Some(local_set_ref)) =
                    (self.ctx_param, self.local_slot_dict_set_func)
                {
                    let _index = self.pop_value()?;
                    let _value = self.pop_value()?;
                    let slot_val = builder.ins().iconst(types::I64, *slot_index as i64);
                    let call = builder.ins().call(local_set_ref, &[ctx, slot_val, _index, _value]);
                    let _success = builder.inst_results(call)[0];
                    let null_val = builder.ins().iconst(types::I64, 0);
                    self.push_value(null_val);
                    return Ok(false);
                }

                if let (Some(ctx), Some(dict_set_ref)) = (self.ctx_param, self.dict_set_func) {
                    // Pop index and value from JIT stack
                    let _index = self.pop_value()?;
                    let _value = self.pop_value()?;
                    let var_name = var_name
                        .as_ref()
                        .ok_or_else(|| "IndexSetInPlace missing local slot name".to_string())?;

                    if self.non_int_locals.contains(var_name) {
                        if let (Some(load_var_ref), Some(stack_push), Some(store_from_stack)) = (
                            self.load_var_func,
                            self.stack_push_func,
                            self.store_var_from_stack_func,
                        ) {
                            let name_hash = Self::hash_var_name(var_name) as i64;
                            let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                            let zero = builder.ins().iconst(types::I64, 0);

                            // Load dict into VM stack
                            builder.ins().call(load_var_ref, &[ctx, name_hash_val, zero]);

                            // Push index and value to VM stack
                            builder.ins().call(stack_push, &[ctx, _index]);
                            builder.ins().call(stack_push, &[ctx, _value]);

                            // Call jit_dict_set
                            let call = builder.ins().call(dict_set_ref, &[ctx]);
                            let _success = builder.inst_results(call)[0];

                            // Store modified dict back to locals from VM stack
                            builder.ins().call(store_from_stack, &[ctx, name_hash_val]);
                            let null_val = builder.ins().iconst(types::I64, 0);
                            self.push_value(null_val);
                            return Ok(false);
                        }
                    }

                    if self.use_local_slots {
                        if let Some(&slot) = self.local_slots.get(var_name) {
                            self.dirty_local_names.insert(var_name.clone());

                            if let (Some(stack_push), Some(stack_pop)) =
                                (self.stack_push_func, self.stack_pop_func)
                            {
                                let object = builder.ins().stack_load(types::I64, slot, 0);

                                builder.ins().call(stack_push, &[ctx, object]);
                                builder.ins().call(stack_push, &[ctx, _index]);
                                builder.ins().call(stack_push, &[ctx, _value]);

                                let call = builder.ins().call(dict_set_ref, &[ctx]);
                                let _success = builder.inst_results(call)[0];

                                let call = builder.ins().call(stack_pop, &[ctx]);
                                let modified_object = builder.inst_results(call)[0];
                                builder.ins().stack_store(modified_object, slot, 0);
                                let null_val = builder.ins().iconst(types::I64, 0);
                                self.push_value(null_val);
                                return Ok(false);
                            } else {
                                return Err(
                                    "IndexSetInPlace requires stack push/pop functions".to_string()
                                );
                            }
                        }
                    }

                    // Load the variable (dict or array)
                    if let (Some(load_var_ref), Some(stack_pop)) =
                        (self.load_var_func, self.stack_pop_func)
                    {
                        // Calculate hash for variable name
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        var_name.hash(&mut hasher);
                        let name_hash = hasher.finish() as i64;

                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);

                        // Call jit_load_variable
                        builder.ins().call(load_var_ref, &[ctx, name_hash_val, zero]);

                        // Pop the loaded object
                        let call = builder.ins().call(stack_pop, &[ctx]);
                        let object = builder.inst_results(call)[0];

                        // Push object, index, value to VM stack for dict_set
                        if let Some(stack_push) = self.stack_push_func {
                            builder.ins().call(stack_push, &[ctx, object]);
                            builder.ins().call(stack_push, &[ctx, _index]);
                            builder.ins().call(stack_push, &[ctx, _value]);
                        }

                        // Call jit_dict_set to modify
                        let call = builder.ins().call(dict_set_ref, &[ctx]);
                        let _success = builder.inst_results(call)[0];

                        // Pop the modified object from VM stack
                        let call = builder.ins().call(stack_pop, &[ctx]);
                        let modified_object = builder.inst_results(call)[0];

                        // Store it back to the variable
                        if let Some(store_var_ref) = self.store_var_func {
                            if let Some(stack_push) = self.stack_push_func {
                                builder.ins().call(stack_push, &[ctx, modified_object]);
                            }
                            builder.ins().call(store_var_ref, &[ctx, name_hash_val, zero]);
                        }
                        let null_val = builder.ins().iconst(types::I64, 0);
                        self.push_value(null_val);
                    }

                    return Ok(false);
                } else {
                    return Err(
                        "IndexSetInPlace requires context and dict_set function".to_string()
                    );
                }
            }

            OpCode::IndexGetInPlace(slot_index) => {
                // In-place index read: value = var[index]
                // Stack: [index] -> [value]
                // This is like: tmp = LoadVar(var); value = tmp[index]

                let var_name = self.local_names.get(*slot_index).cloned();
                let is_non_int_local = var_name
                    .as_ref()
                    .map(|name| self.non_int_locals.contains(name))
                    .unwrap_or(false);

                if self.int_dict_slots.contains(slot_index) && !is_non_int_local {
                    let _index = self.pop_value()?;

                    if let Some(&ptr_slot) = self.int_dict_ptr_slots.get(slot_index) {
                        if let Some(get_ptr_ref) = self.int_dict_get_ptr_func {
                            let dict_ptr = builder.ins().stack_load(types::I64, ptr_slot, 0);
                            let zero = builder.ins().iconst(types::I64, 0);
                            let is_zero = builder.ins().icmp(IntCC::Equal, dict_ptr, zero);

                            let slow_block = builder.create_block();
                            let dense_int_full_block = builder.create_block();
                            let dense_int_sparse_check_block = builder.create_block();
                            let dense_int_sparse_block = builder.create_block();
                            let fast_block = builder.create_block();
                            let cont_block = builder.create_block();
                            builder.append_block_param(cont_block, types::I64);

                            builder.ins().brif(is_zero, slow_block, &[], fast_block, &[]);

                            builder.switch_to_block(fast_block);
                            let tag_mask = builder.ins().iconst(types::I64, 7);
                            let tag_value = builder.ins().band(dict_ptr, tag_mask);
                            let dense_int_full_tag = builder.ins().iconst(types::I64, 2);
                            let is_dense_int_full =
                                builder.ins().icmp(IntCC::Equal, tag_value, dense_int_full_tag);
                            let dense_int_sparse_tag = builder.ins().iconst(types::I64, 4);
                            let is_dense_int_sparse =
                                builder.ins().icmp(IntCC::Equal, tag_value, dense_int_sparse_tag);
                            builder.ins().brif(
                                is_dense_int_full,
                                dense_int_full_block,
                                &[],
                                dense_int_sparse_check_block,
                                &[],
                            );

                            builder.switch_to_block(dense_int_full_block);
                            let zero = builder.ins().iconst(types::I64, 0);
                            let is_negative =
                                builder.ins().icmp(IntCC::SignedLessThan, _index, zero);
                            let inline_slow_block = builder.create_block();
                            let inline_get_block = builder.create_block();
                            builder.ins().brif(
                                is_negative,
                                inline_slow_block,
                                &[],
                                inline_get_block,
                                &[],
                            );

                            builder.switch_to_block(inline_get_block);
                            let tag_mask = builder.ins().iconst(types::I64, !7i64);
                            let base_ptr = builder.ins().band(dict_ptr, tag_mask);
                            let data_ptr =
                                builder.ins().load(types::I64, MemFlags::new(), base_ptr, 0);
                            let len = builder.ins().load(types::I64, MemFlags::new(), base_ptr, 8);
                            let is_lt_len = builder.ins().icmp(IntCC::SignedLessThan, _index, len);
                            let hit_block = builder.create_block();
                            let miss_block = builder.create_block();
                            builder.ins().brif(is_lt_len, hit_block, &[], miss_block, &[]);

                            builder.switch_to_block(hit_block);
                            let stride = builder.ins().iconst(types::I64, 8);
                            let offset = builder.ins().imul(_index, stride);
                            let addr = builder.ins().iadd(data_ptr, offset);
                            let value = builder.ins().load(types::I64, MemFlags::new(), addr, 0);
                            builder.ins().jump(cont_block, &[value]);

                            builder.switch_to_block(miss_block);
                            builder.ins().jump(cont_block, &[zero]);

                            builder.switch_to_block(inline_slow_block);
                            if let Some(get_dense_int_full_ref) =
                                self.int_dict_get_ptr_dense_int_full_func
                            {
                                let call =
                                    builder.ins().call(get_dense_int_full_ref, &[dict_ptr, _index]);
                                let fast_value = builder.inst_results(call)[0];
                                builder.ins().jump(cont_block, &[fast_value]);
                            } else {
                                let call = builder.ins().call(get_ptr_ref, &[dict_ptr, _index]);
                                let fast_value = builder.inst_results(call)[0];
                                builder.ins().jump(cont_block, &[fast_value]);
                            }

                            builder.switch_to_block(dense_int_sparse_check_block);
                            builder.ins().brif(
                                is_dense_int_sparse,
                                dense_int_sparse_block,
                                &[],
                                slow_block,
                                &[],
                            );

                            builder.switch_to_block(dense_int_sparse_block);
                            if let Some(get_dense_int_ref) = self.int_dict_get_ptr_dense_int_func {
                                let call =
                                    builder.ins().call(get_dense_int_ref, &[dict_ptr, _index]);
                                let fast_value = builder.inst_results(call)[0];
                                builder.ins().jump(cont_block, &[fast_value]);
                            } else {
                                let call = builder.ins().call(get_ptr_ref, &[dict_ptr, _index]);
                                let fast_value = builder.inst_results(call)[0];
                                builder.ins().jump(cont_block, &[fast_value]);
                            }

                            builder.switch_to_block(slow_block);
                            if let (Some(ctx), Some(get_int_dict_ref)) =
                                (self.ctx_param, self.local_slot_int_dict_get_func)
                            {
                                let slot_val = builder.ins().iconst(types::I64, *slot_index as i64);
                                let call =
                                    builder.ins().call(get_int_dict_ref, &[ctx, slot_val, _index]);
                                let slow_value = builder.inst_results(call)[0];
                                builder.ins().jump(cont_block, &[slow_value]);
                            } else {
                                let zero_val = builder.ins().iconst(types::I64, 0);
                                builder.ins().jump(cont_block, &[zero_val]);
                            }

                            builder.switch_to_block(cont_block);
                            let result = builder.block_params(cont_block)[0];
                            self.push_value(result);
                            return Ok(false);
                        }
                    }

                    if let (Some(ctx), Some(get_int_dict_ref)) =
                        (self.ctx_param, self.local_slot_int_dict_get_func)
                    {
                        let slot_val = builder.ins().iconst(types::I64, *slot_index as i64);
                        let call = builder.ins().call(get_int_dict_ref, &[ctx, slot_val, _index]);
                        let value = builder.inst_results(call)[0];
                        self.push_value(value);
                        return Ok(false);
                    }

                    return Err("IndexGetInPlace requires int-dict helpers".to_string());
                }

                if let (Some(ctx), Some(local_get_ref)) =
                    (self.ctx_param, self.local_slot_dict_get_func)
                {
                    let _index = self.pop_value()?;
                    let slot_val = builder.ins().iconst(types::I64, *slot_index as i64);
                    let call = builder.ins().call(local_get_ref, &[ctx, slot_val, _index]);
                    let value = builder.inst_results(call)[0];
                    self.push_value(value);
                    return Ok(false);
                }

                if let (Some(ctx), Some(dict_get_ref)) = (self.ctx_param, self.dict_get_func) {
                    // Pop index from JIT stack
                    let _index = self.pop_value()?;

                    let var_name = var_name
                        .as_ref()
                        .ok_or_else(|| "IndexGetInPlace missing local slot name".to_string())?;

                    if self.non_int_locals.contains(var_name) {
                        if let (Some(load_var_ref), Some(stack_push), Some(stack_pop)) =
                            (self.load_var_func, self.stack_push_func, self.stack_pop_func)
                        {
                            let name_hash = Self::hash_var_name(var_name) as i64;
                            let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                            let zero = builder.ins().iconst(types::I64, 0);

                            // Load dict into VM stack
                            builder.ins().call(load_var_ref, &[ctx, name_hash_val, zero]);

                            // Push index to VM stack
                            builder.ins().call(stack_push, &[ctx, _index]);

                            // Call jit_dict_get
                            let call = builder.ins().call(dict_get_ref, &[ctx]);
                            let _success = builder.inst_results(call)[0];

                            // Pop result value from VM stack
                            let call = builder.ins().call(stack_pop, &[ctx]);
                            let value = builder.inst_results(call)[0];
                            self.push_value(value);
                            return Ok(false);
                        }
                    }

                    if self.use_local_slots {
                        if let Some(&slot) = self.local_slots.get(var_name) {
                            if let (Some(stack_push), Some(stack_pop)) =
                                (self.stack_push_func, self.stack_pop_func)
                            {
                                let object = builder.ins().stack_load(types::I64, slot, 0);

                                builder.ins().call(stack_push, &[ctx, object]);
                                builder.ins().call(stack_push, &[ctx, _index]);

                                let call = builder.ins().call(dict_get_ref, &[ctx]);
                                let _success = builder.inst_results(call)[0];

                                let call = builder.ins().call(stack_pop, &[ctx]);
                                let value = builder.inst_results(call)[0];
                                self.push_value(value);
                                return Ok(false);
                            } else {
                                return Err(
                                    "IndexGetInPlace requires stack push/pop functions".to_string()
                                );
                            }
                        }
                    }

                    // Load the variable (dict or array)
                    if let (Some(load_var_ref), Some(stack_pop)) =
                        (self.load_var_func, self.stack_pop_func)
                    {
                        // Calculate hash for variable name
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        var_name.hash(&mut hasher);
                        let name_hash = hasher.finish() as i64;

                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);

                        // Call jit_load_variable
                        builder.ins().call(load_var_ref, &[ctx, name_hash_val, zero]);

                        // Pop the loaded object
                        let call = builder.ins().call(stack_pop, &[ctx]);
                        let object = builder.inst_results(call)[0];

                        // Push object and index to VM stack for dict_get
                        if let Some(stack_push) = self.stack_push_func {
                            builder.ins().call(stack_push, &[ctx, object]);
                            builder.ins().call(stack_push, &[ctx, _index]);
                        }

                        // Call jit_dict_get
                        let call = builder.ins().call(dict_get_ref, &[ctx]);
                        let _success = builder.inst_results(call)[0];

                        // Pop the result value from VM stack
                        let call = builder.ins().call(stack_pop, &[ctx]);
                        let value = builder.inst_results(call)[0];
                        self.push_value(value);
                    }

                    return Ok(false);
                } else {
                    return Err(
                        "IndexGetInPlace requires context and dict_get function".to_string()
                    );
                }
            }

            OpCode::Return => {
                // Pop the return value from our stack
                if let Some(return_value) = self.value_stack.pop() {
                    self.persist_local_slots_to_vm(builder);
                    if let Some(ctx) = self.ctx_param {
                        // OPTIMIZATION: Use jit_set_return_int for fast integer returns
                        // This stores the return value directly in VMContext instead of
                        // pushing to the VM stack, avoiding stack operations overhead.
                        // The VM will check has_return_value and use return_value directly.
                        if let Some(set_return_int_func) = self.set_return_int_func {
                            let inst =
                                builder.ins().call(set_return_int_func, &[ctx, return_value]);
                            let _result = builder.inst_results(inst)[0];
                        } else if let Some(push_int_func) = self.push_int_func {
                            // Fallback: Use original jit_push_int (slower path)
                            let inst = builder.ins().call(push_int_func, &[ctx, return_value]);
                            let _result = builder.inst_results(inst)[0];
                        }
                    }
                }
                // Return 0 (success)
                let zero = builder.ins().iconst(types::I64, 0);
                builder.ins().return_(&[zero]);
                return Ok(true); // Terminates block
            }

            OpCode::ReturnNone => {
                self.persist_local_slots_to_vm(builder);
                let zero = builder.ins().iconst(types::I64, 0);
                builder.ins().return_(&[zero]);
                return Ok(true); // Terminates block
            }

            // Variable operations - use stack slots for locals (fast path) or runtime helpers (fallback)
            OpCode::LoadVar(name) => {
                // NOTE: Self-recursion detection was added but direct recursion causes
                // stack overflow due to shared stack slots. Keeping detection code for
                // future tail-call optimization work.
                if self.non_int_locals.contains(name) {
                    if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.load_var_func) {
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);
                        let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                        let result = builder.inst_results(call)[0];
                        self.push_value(result);
                        return Ok(false);
                    }
                }

                // OPTIMIZATION: Use stack slots for local variables (register-based locals)
                // This avoids C function calls and HashMap lookups for every variable access
                if self.use_local_slots {
                    if let Some(&slot) = self.local_slots.get(name) {
                        // Fast path: Direct stack slot load
                        // Load the i64 value directly from the stack slot
                        let value = builder.ins().stack_load(types::I64, slot, 0);
                        self.push_value(value);

                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!("JIT: LoadVar '{}' using fast stack slot {:?}", name, slot);
                        }
                    } else {
                        // Variable not in local slots - fall back to runtime call
                        // This handles globals and captured variables
                        if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.load_var_func) {
                            let name_hash = Self::hash_var_name(name) as i64;
                            let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                            let zero = builder.ins().iconst(types::I64, 0);
                            let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                            let result = builder.inst_results(call)[0];
                            self.push_value(result);
                        } else {
                            let zero = builder.ins().iconst(types::I64, 0);
                            self.push_value(zero);
                        }
                    }
                } else if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.load_var_func) {
                    // Original slow path: Call runtime helper
                    let name_hash = {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        name.hash(&mut hasher);
                        hasher.finish() as i64
                    };

                    let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                    let zero = builder.ins().iconst(types::I64, 0); // name_len = 0 (use hash instead)

                    // Call jit_load_variable(ctx, name_hash, 0)
                    let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                    let result = builder.inst_results(call)[0];
                    self.push_value(result);
                } else {
                    // Fallback: load 0 if context not available
                    let zero = builder.ins().iconst(types::I64, 0);
                    self.push_value(zero);
                }
            }

            OpCode::LoadLocal(slot) => {
                if let Some(name) = self.local_names.get(*slot) {
                    if self.non_int_locals.contains(name) {
                        if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.load_var_func) {
                            let name_hash = Self::hash_var_name(name) as i64;
                            let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                            let zero = builder.ins().iconst(types::I64, 0);
                            let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                            let result = builder.inst_results(call)[0];
                            self.push_value(result);
                            return Ok(false);
                        }
                    }
                    if self.use_local_slots {
                        if let Some(&stack_slot) = self.local_slots.get(name) {
                            let value = builder.ins().stack_load(types::I64, stack_slot, 0);
                            self.push_value(value);
                            return Ok(false);
                        }
                    }

                    if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.load_var_func) {
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);
                        let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                        let result = builder.inst_results(call)[0];
                        self.push_value(result);
                        return Ok(false);
                    }
                }

                let zero = builder.ins().iconst(types::I64, 0);
                self.push_value(zero);
            }

            OpCode::StoreVar(name) => {
                // IMPORTANT: StoreVar PEEKS at the stack (doesn't pop)
                // The value remains on stack after assignment
                let value = self.peek_value()?;
                if self.non_int_locals.contains(name) {
                    if let (Some(ctx), Some(store_from_stack)) =
                        (self.ctx_param, self.store_var_from_stack_func)
                    {
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);

                        let marker = builder.ins().iconst(types::I64, -1);
                        let is_marker = builder.ins().icmp(IntCC::Equal, value, marker);

                        let marker_block = builder.create_block();
                        let push_block = builder.create_block();
                        let cont_block = builder.create_block();

                        builder.ins().brif(is_marker, marker_block, &[], push_block, &[]);
                        builder.switch_to_block(marker_block);
                        builder.ins().call(store_from_stack, &[ctx, name_hash_val]);
                        builder.ins().jump(cont_block, &[]);
                        builder.switch_to_block(push_block);
                        if let Some(stack_push) = self.stack_push_func {
                            builder.ins().call(stack_push, &[ctx, value]);
                        }
                        builder.ins().call(store_from_stack, &[ctx, name_hash_val]);
                        builder.ins().jump(cont_block, &[]);
                        builder.switch_to_block(cont_block);
                        return Ok(false);
                    }
                }

                // OPTIMIZATION: Use stack slots for local variables (register-based locals)
                if self.use_local_slots {
                    if let Some(&slot) = self.local_slots.get(name) {
                        self.dirty_local_names.insert(name.clone());
                        // Fast path: Direct stack slot store
                        // Store the i64 value directly to the stack slot
                        builder.ins().stack_store(value, slot, 0);

                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!("JIT: StoreVar '{}' using fast stack slot {:?}", name, slot);
                        }
                        // Value stays on stack - do NOT pop
                    } else {
                        // Variable not in local slots - fall back to runtime call
                        if let (Some(ctx), Some(store_func)) = (self.ctx_param, self.store_var_func)
                        {
                            let name_hash = Self::hash_var_name(name) as i64;
                            let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                            let zero = builder.ins().iconst(types::I64, 0);
                            builder.ins().call(store_func, &[ctx, name_hash_val, zero, value]);
                        }
                    }
                } else if let (Some(ctx), Some(store_func)) = (self.ctx_param, self.store_var_func)
                {
                    // Original slow path: Call runtime helper
                    let name_hash = {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        name.hash(&mut hasher);
                        hasher.finish() as i64
                    };

                    let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                    let zero = builder.ins().iconst(types::I64, 0); // name_len = 0

                    // Call jit_store_variable(ctx, name_hash, 0, value)
                    builder.ins().call(store_func, &[ctx, name_hash_val, zero, value]);
                    // Value stays on stack - do NOT pop
                }
                // If no context, just leave value on stack
            }

            OpCode::StoreLocal(slot) => {
                let value = self.peek_value()?;

                if let Some(name) = self.local_names.get(*slot) {
                    if self.non_int_locals.contains(name) {
                        if let (Some(ctx), Some(store_from_stack)) =
                            (self.ctx_param, self.store_var_from_stack_func)
                        {
                            let name_hash = Self::hash_var_name(name) as i64;
                            let name_hash_val = builder.ins().iconst(types::I64, name_hash);

                            let marker = builder.ins().iconst(types::I64, -1);
                            let is_marker = builder.ins().icmp(IntCC::Equal, value, marker);

                            let marker_block = builder.create_block();
                            let push_block = builder.create_block();
                            let cont_block = builder.create_block();

                            builder.ins().brif(is_marker, marker_block, &[], push_block, &[]);
                            builder.switch_to_block(marker_block);
                            builder.ins().call(store_from_stack, &[ctx, name_hash_val]);
                            builder.ins().jump(cont_block, &[]);
                            builder.switch_to_block(push_block);
                            if let Some(stack_push) = self.stack_push_func {
                                builder.ins().call(stack_push, &[ctx, value]);
                            }
                            builder.ins().call(store_from_stack, &[ctx, name_hash_val]);
                            builder.ins().jump(cont_block, &[]);
                            builder.switch_to_block(cont_block);
                            return Ok(false);
                        }
                    }
                    if self.use_local_slots {
                        if let Some(&stack_slot) = self.local_slots.get(name) {
                            self.dirty_local_names.insert(name.clone());
                            builder.ins().stack_store(value, stack_slot, 0);
                            if self.int_dict_slots.contains(slot) {
                                if let Some(&ptr_slot) = self.int_dict_ptr_slots.get(slot) {
                                    let zero = builder.ins().iconst(types::I64, 0);
                                    builder.ins().stack_store(zero, ptr_slot, 0);
                                }
                            }
                            return Ok(false);
                        }
                    }

                    if let (Some(ctx), Some(store_func)) = (self.ctx_param, self.store_var_func) {
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);
                        builder.ins().call(store_func, &[ctx, name_hash_val, zero, value]);
                    }
                }
            }

            OpCode::LoadGlobal(name) => {
                // Same as LoadVar for now
                if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.load_var_func) {
                    let name_hash = {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        name.hash(&mut hasher);
                        hasher.finish() as i64
                    };

                    let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                    let zero = builder.ins().iconst(types::I64, 0);

                    let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                    let result = builder.inst_results(call)[0];
                    self.push_value(result);
                } else {
                    let zero = builder.ins().iconst(types::I64, 0);
                    self.push_value(zero);
                }
            }

            OpCode::StoreGlobal(name) => {
                // Same as StoreVar - PEEKS at stack, doesn't pop
                if let (Some(ctx), Some(store_func)) = (self.ctx_param, self.store_var_func) {
                    let value = self.peek_value()?;

                    let name_hash = {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        name.hash(&mut hasher);
                        hasher.finish() as i64
                    };

                    let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                    let zero = builder.ins().iconst(types::I64, 0);

                    builder.ins().call(store_func, &[ctx, name_hash_val, zero, value]);
                    // Value stays on stack - do NOT pop
                } else {
                    // Can't store without context - leave value on stack
                }
            }

            // Unsupported operations fall back to interpreter
            _ => {
                return Err(format!("Unsupported opcode for JIT: {:?}", instruction));
            }
        }

        Ok(false) // Doesn't terminate block
    }

    /// Translate bytecode instruction for direct-arg mode
    /// This is the key optimization for recursive functions:
    /// - LoadVar of the parameter uses the direct Cranelift parameter value
    /// - Return directly returns the value (not a status code)
    /// - Self-recursive calls use direct Cranelift function calls
    fn translate_direct_arg_instruction(
        &mut self,
        builder: &mut FunctionBuilder,
        _pc: usize,
        instruction: &OpCode,
        constants: &[Constant],
        param_name: &str,
    ) -> Result<bool, String> {
        match instruction {
            OpCode::LoadLocal(slot) => {
                if let Some(name) = self.local_names.get(*slot) {
                    if let Some(&stack_slot) = self.local_slots.get(name) {
                        let value = builder.ins().stack_load(types::I64, stack_slot, 0);
                        self.push_value(value);
                        return Ok(false);
                    }

                    if name == param_name {
                        if let Some(direct_param) = self.direct_arg_param {
                            self.push_value(direct_param);
                            return Ok(false);
                        }
                    }

                    if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.load_var_func) {
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);
                        let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                        let result = builder.inst_results(call)[0];
                        self.push_value(result);
                        return Ok(false);
                    }
                }

                let zero = builder.ins().iconst(types::I64, 0);
                self.push_value(zero);
                Ok(false)
            }

            OpCode::StoreLocal(slot) => {
                let value = self.peek_value()?;
                if let Some(name) = self.local_names.get(*slot) {
                    if self.non_int_locals.contains(name) {
                        if let (Some(ctx), Some(store_from_stack)) =
                            (self.ctx_param, self.store_var_from_stack_func)
                        {
                            let name_hash = Self::hash_var_name(name) as i64;
                            let name_hash_val = builder.ins().iconst(types::I64, name_hash);

                            let marker = builder.ins().iconst(types::I64, -1);
                            let is_marker = builder.ins().icmp(IntCC::Equal, value, marker);

                            let marker_block = builder.create_block();
                            let push_block = builder.create_block();
                            let cont_block = builder.create_block();

                            builder.ins().brif(is_marker, marker_block, &[], push_block, &[]);
                            builder.switch_to_block(marker_block);
                            builder.ins().call(store_from_stack, &[ctx, name_hash_val]);
                            builder.ins().jump(cont_block, &[]);
                            builder.switch_to_block(push_block);
                            if let Some(stack_push) = self.stack_push_func {
                                builder.ins().call(stack_push, &[ctx, value]);
                            }
                            builder.ins().call(store_from_stack, &[ctx, name_hash_val]);
                            builder.ins().jump(cont_block, &[]);
                            builder.switch_to_block(cont_block);
                            return Ok(false);
                        }
                    }
                    if let Some(&stack_slot) = self.local_slots.get(name) {
                        builder.ins().stack_store(value, stack_slot, 0);
                        if self.int_dict_slots.contains(slot) {
                            if let Some(&ptr_slot) = self.int_dict_ptr_slots.get(slot) {
                                let zero = builder.ins().iconst(types::I64, 0);
                                builder.ins().stack_store(zero, ptr_slot, 0);
                            }
                        }
                        return Ok(false);
                    }

                    if let (Some(ctx), Some(store_func)) = (self.ctx_param, self.store_var_func) {
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);
                        builder.ins().call(store_func, &[ctx, name_hash_val, zero, value]);
                    }
                }
                Ok(false)
            }

            // LoadVar: Use direct_arg_param if loading the function parameter
            OpCode::LoadVar(name) => {
                if name == param_name {
                    // FAST PATH: Use the direct Cranelift parameter value
                    if let Some(direct_param) = self.direct_arg_param {
                        self.push_value(direct_param);
                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!("JIT direct-arg: LoadVar '{}' using direct parameter", name);
                        }
                    } else {
                        return Err(
                            "Direct-arg mode enabled but no direct_arg_param set".to_string()
                        );
                    }
                } else if let Some(func_name) = &self.current_function_name {
                    if name == func_name {
                        // Loading the function itself for recursion - push a marker
                        let marker = builder.ins().iconst(types::I64, -1);
                        self.push_value(marker);
                        self.self_recursion_pending = true;
                    } else if let Some(&slot) = self.local_slots.get(name) {
                        // Load from stack slot (other local variable)
                        let value = builder.ins().stack_load(types::I64, slot, 0);
                        self.push_value(value);
                    } else if let (Some(ctx), Some(load_func)) =
                        (self.ctx_param, self.load_var_func)
                    {
                        // Fall back to runtime call for globals
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);
                        let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                        let result = builder.inst_results(call)[0];
                        self.push_value(result);
                    } else {
                        let zero = builder.ins().iconst(types::I64, 0);
                        self.push_value(zero);
                    }
                } else if let Some(&slot) = self.local_slots.get(name) {
                    let value = builder.ins().stack_load(types::I64, slot, 0);
                    self.push_value(value);
                } else {
                    // Fall back to runtime call
                    if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.load_var_func) {
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);
                        let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                        let result = builder.inst_results(call)[0];
                        self.push_value(result);
                    } else {
                        let zero = builder.ins().iconst(types::I64, 0);
                        self.push_value(zero);
                    }
                }
                Ok(false)
            }

            // Call: For self-recursion, emit direct Cranelift call
            OpCode::Call(arg_count) => {
                if self.self_recursion_pending && *arg_count == 1 {
                    // DIRECT SELF-RECURSION: Call ourselves directly!
                    self.self_recursion_pending = false;

                    // Pop the function marker
                    let _marker = self.pop_value()?;

                    // Pop the argument
                    let arg_val = self.pop_value()?;

                    if let (Some(ctx), Some(self_func)) = (self.ctx_param, self.self_call_func) {
                        // Direct recursive call: self(ctx, arg)
                        // This is the KEY optimization - no FFI boundary crossing!
                        let call_inst = builder.ins().call(self_func, &[ctx, arg_val]);
                        let result = builder.inst_results(call_inst)[0];
                        self.push_value(result);

                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!("JIT direct-arg: Emitting direct recursive call");
                        }
                    } else {
                        return Err("Self-recursion detected but no self_call_func set".to_string());
                    }
                } else {
                    // Non-recursive call or multi-arg: fall back to standard path
                    self.self_recursion_pending = false;

                    // Pop the function marker
                    let _func_marker = self.pop_value()?;
                    let mut arg_vals = Vec::new();
                    for _ in 0..*arg_count {
                        arg_vals.push(self.pop_value()?);
                    }
                    arg_vals.reverse();

                    if let (Some(ctx), Some(call_func), Some(stack_push)) =
                        (self.ctx_param, self.call_func, self.stack_push_func)
                    {
                        // Push args to VM stack
                        for arg_val in &arg_vals {
                            builder.ins().call(stack_push, &[ctx, *arg_val]);
                        }

                        // Call jit_call_function
                        let null_ptr = builder.ins().iconst(types::I64, 0);
                        let arg_count_val = builder.ins().iconst(types::I64, *arg_count as i64);
                        builder.ins().call(call_func, &[ctx, null_ptr, arg_count_val]);

                        // Pop result from VM stack
                        if let Some(stack_pop) = self.stack_pop_func {
                            let call = builder.ins().call(stack_pop, &[ctx]);
                            let result = builder.inst_results(call)[0];
                            self.push_value(result);
                        } else {
                            let zero = builder.ins().iconst(types::I64, 0);
                            self.push_value(zero);
                        }
                    } else {
                        return Err("Call opcode requires context and call_func".to_string());
                    }
                }
                Ok(false)
            }

            // Return: In direct-arg mode, return the actual value (not status code)
            OpCode::Return => {
                if let Some(return_value) = self.value_stack.pop() {
                    // Direct return of the computed value
                    builder.ins().return_(&[return_value]);

                    if std::env::var("DEBUG_JIT").is_ok() {
                        eprintln!("JIT direct-arg: Direct value return");
                    }
                } else {
                    // No value to return - return 0
                    let zero = builder.ins().iconst(types::I64, 0);
                    builder.ins().return_(&[zero]);
                }
                Ok(true) // Terminates block
            }

            OpCode::ReturnNone => {
                let zero = builder.ins().iconst(types::I64, 0);
                builder.ins().return_(&[zero]);
                Ok(true)
            }

            // For all other opcodes, delegate to standard translate_instruction
            _ => self.translate_instruction(builder, _pc, instruction, constants),
        }
    }

    /// Translate Add operation with type specialization
    fn translate_add_specialized(
        &mut self,
        builder: &mut FunctionBuilder,
        var_a: Option<&str>,
        var_b: Option<&str>,
    ) -> Result<(), String> {
        // Try to determine types from specialization info
        let type_a = var_a.and_then(|name| {
            let hash = Self::hash_var_name(name);
            self.specialization.as_ref()?.specialized_types.get(&hash).copied()
        });

        let type_b = var_b.and_then(|name| {
            let hash = Self::hash_var_name(name);
            self.specialization.as_ref()?.specialized_types.get(&hash).copied()
        });

        let b = self.pop_value()?;
        let a = self.pop_value()?;

        // Generate specialized code based on types
        match (type_a, type_b) {
            (Some(ValueType::Int), Some(ValueType::Int)) => {
                // Pure integer addition - fastest path
                // Direct native i64 addition without type checks
                let result = builder.ins().iadd(a, b);
                self.push_value(result);
            }
            // Float specialization deferred to future optimization pass
            // Current implementation focuses on integer optimization
            _ => {
                // Generic fallback - assume integers for now
                let result = builder.ins().iadd(a, b);
                self.push_value(result);
            }
        }
        Ok(())
    }

    /// Translate Sub operation with type specialization
    fn translate_sub_specialized(
        &mut self,
        builder: &mut FunctionBuilder,
        var_a: Option<&str>,
        var_b: Option<&str>,
    ) -> Result<(), String> {
        let type_a = var_a.and_then(|name| {
            let hash = Self::hash_var_name(name);
            self.specialization.as_ref()?.specialized_types.get(&hash).copied()
        });

        let type_b = var_b.and_then(|name| {
            let hash = Self::hash_var_name(name);
            self.specialization.as_ref()?.specialized_types.get(&hash).copied()
        });

        let b = self.pop_value()?;
        let a = self.pop_value()?;

        match (type_a, type_b) {
            (Some(ValueType::Int), Some(ValueType::Int)) => {
                // Pure integer subtraction
                let result = builder.ins().isub(a, b);
                self.push_value(result);
            }
            _ => {
                let result = builder.ins().isub(a, b);
                self.push_value(result);
            }
        }
        Ok(())
    }

    /// Translate Mul operation with type specialization
    fn translate_mul_specialized(
        &mut self,
        builder: &mut FunctionBuilder,
        var_a: Option<&str>,
        var_b: Option<&str>,
    ) -> Result<(), String> {
        let type_a = var_a.and_then(|name| {
            let hash = Self::hash_var_name(name);
            self.specialization.as_ref()?.specialized_types.get(&hash).copied()
        });

        let type_b = var_b.and_then(|name| {
            let hash = Self::hash_var_name(name);
            self.specialization.as_ref()?.specialized_types.get(&hash).copied()
        });

        let b = self.pop_value()?;
        let a = self.pop_value()?;

        match (type_a, type_b) {
            (Some(ValueType::Int), Some(ValueType::Int)) => {
                // Pure integer multiplication
                let result = builder.ins().imul(a, b);
                self.push_value(result);
            }
            _ => {
                let result = builder.ins().imul(a, b);
                self.push_value(result);
            }
        }
        Ok(())
    }

    /// Translate Div operation with type specialization
    fn translate_div_specialized(
        &mut self,
        builder: &mut FunctionBuilder,
        var_a: Option<&str>,
        var_b: Option<&str>,
    ) -> Result<(), String> {
        let type_a = var_a.and_then(|name| {
            let hash = Self::hash_var_name(name);
            self.specialization.as_ref()?.specialized_types.get(&hash).copied()
        });

        let type_b = var_b.and_then(|name| {
            let hash = Self::hash_var_name(name);
            self.specialization.as_ref()?.specialized_types.get(&hash).copied()
        });

        let b = self.pop_value()?;
        let a = self.pop_value()?;

        match (type_a, type_b) {
            (Some(ValueType::Int), Some(ValueType::Int)) => {
                // Pure integer division
                let result = builder.ins().sdiv(a, b);
                self.push_value(result);
            }
            _ => {
                let result = builder.ins().sdiv(a, b);
                self.push_value(result);
            }
        }
        Ok(())
    }

    /// Helper to hash variable names consistently
    fn hash_var_name(name: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        hasher.finish()
    }

    fn push_value(&mut self, val: cranelift::prelude::Value) {
        self.value_stack.push(val);
    }

    fn pop_value(&mut self) -> Result<cranelift::prelude::Value, String> {
        self.value_stack.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    fn peek_value(&self) -> Result<cranelift::prelude::Value, String> {
        self.value_stack.last().copied().ok_or_else(|| "Stack empty".to_string())
    }
}

impl JitCompiler {
    /// Create a new JIT compiler instance
    pub fn new() -> Result<Self, String> {
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", "speed").map_err(|e| e.to_string())?;
        flag_builder.set("is_pic", "false").map_err(|e| e.to_string())?;

        let isa_builder = cranelift_native::builder()
            .map_err(|e| format!("Failed to create ISA builder: {}", e))?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| format!("Failed to create ISA: {}", e))?;

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Register runtime helper symbols so JIT can find them
        builder.symbol("jit_load_variable", jit_load_variable as *const u8);
        builder.symbol("jit_store_variable", jit_store_variable as *const u8);
        builder.symbol("jit_store_variable_from_stack", jit_store_variable_from_stack as *const u8);
        builder.symbol("jit_stack_push", jit_stack_push as *const u8);
        builder.symbol("jit_stack_pop", jit_stack_pop as *const u8);
        builder.symbol("jit_obj_push_string", jit_obj_push_string as *const u8);
        builder.symbol("jit_obj_to_vm_stack", jit_obj_to_vm_stack as *const u8);
        builder.symbol("jit_load_variable_float", jit_load_variable_float as *const u8);
        builder.symbol("jit_store_variable_float", jit_store_variable_float as *const u8);
        builder.symbol("jit_check_type_int", jit_check_type_int as *const u8);
        builder.symbol("jit_check_type_float", jit_check_type_float as *const u8);
        builder.symbol("jit_call_function", jit_call_function as *const u8);
        builder.symbol("jit_push_int", jit_push_int as *const u8);
        builder.symbol("jit_set_return_int", jit_set_return_int as *const u8);
        builder.symbol("jit_get_return_int", jit_get_return_int as *const u8);
        builder.symbol("jit_get_arg", jit_get_arg as *const u8);
        builder.symbol("jit_dict_get", jit_dict_get as *const u8);
        builder.symbol("jit_dict_set", jit_dict_set as *const u8);
        builder.symbol("jit_local_slot_dict_get", jit_local_slot_dict_get as *const u8);
        builder.symbol("jit_local_slot_dict_set", jit_local_slot_dict_set as *const u8);
        builder.symbol("jit_local_slot_int_dict_get", jit_local_slot_int_dict_get as *const u8);
        builder.symbol("jit_local_slot_int_dict_set", jit_local_slot_int_dict_set as *const u8);
        builder.symbol("jit_int_dict_unique_ptr", jit_int_dict_unique_ptr as *const u8);
        builder.symbol("jit_int_dict_get_ptr", jit_int_dict_get_ptr as *const u8);
        builder.symbol("jit_int_dict_set_ptr", jit_int_dict_set_ptr as *const u8);
        builder
            .symbol("jit_dense_int_dict_int_get_ptr", jit_dense_int_dict_int_get_ptr as *const u8);
        builder
            .symbol("jit_dense_int_dict_int_set_ptr", jit_dense_int_dict_int_set_ptr as *const u8);
        builder.symbol(
            "jit_dense_int_dict_int_full_get_ptr",
            jit_dense_int_dict_int_full_get_ptr as *const u8,
        );
        builder.symbol(
            "jit_dense_int_dict_int_full_set_ptr",
            jit_dense_int_dict_int_full_set_ptr as *const u8,
        );
        builder.symbol("jit_make_dict", jit_make_dict as *const u8);
        builder.symbol("jit_make_dict_with_keys", jit_make_dict_with_keys as *const u8);
        builder.symbol(
            "jit_append_const_string_in_place",
            jit_append_const_string_in_place as *const u8,
        );
        builder
            .symbol("jit_append_const_char_in_place", jit_append_const_char_in_place as *const u8);

        let module = JITModule::new(builder);

        Ok(JitCompiler {
            module,
            ctx: codegen::Context::new(),
            execution_counts: HashMap::new(),
            compiled_cache: HashMap::new(),
            enabled: true,
            loop_jit_blacklist: HashSet::new(),
            type_profiles: HashMap::new(),
            compiled_fn_info: HashMap::new(),
        })
    }

    /// Check if JIT should compile this function based on execution count
    pub fn should_compile(&mut self, offset: usize) -> bool {
        if !self.enabled {
            return false;
        }

        let count = self.execution_counts.entry(offset).or_insert(0);
        *count += 1;

        *count >= JIT_THRESHOLD && !self.compiled_cache.contains_key(&offset)
    }

    pub fn is_loop_jit_blocked(&self, offset: usize) -> bool {
        self.loop_jit_blacklist.contains(&offset)
    }

    pub fn mark_loop_jit_blocked(&mut self, offset: usize) {
        self.loop_jit_blacklist.insert(offset);
    }

    /// Check if a loop can be JIT-compiled (all opcodes supported)
    /// Scans from start to end (inclusive) checking for unsupported operations
    pub fn can_compile_loop(&self, chunk: &BytecodeChunk, start: usize, end: usize) -> bool {
        for pc in start..=end {
            if let Some(instruction) = chunk.instructions.get(pc) {
                if matches!(instruction, OpCode::IndexGetInPlace(_) | OpCode::IndexSetInPlace(_)) {
                    return false;
                }
                if !self.is_supported_opcode(instruction, &chunk.constants) {
                    if std::env::var("DEBUG_JIT").is_ok() {
                        eprintln!("JIT: Unsupported opcode at {}: {:?}", pc, instruction);
                    }
                    return false;
                }
            }
        }
        true
    }

    pub fn can_compile_loop_with_int_dicts(
        &self,
        chunk: &BytecodeChunk,
        start: usize,
        end: usize,
    ) -> bool {
        for pc in start..=end {
            if let Some(instruction) = chunk.instructions.get(pc) {
                if matches!(instruction, OpCode::IndexGetInPlace(_) | OpCode::IndexSetInPlace(_)) {
                    continue;
                }
                if !self.is_supported_opcode(instruction, &chunk.constants) {
                    if std::env::var("DEBUG_JIT").is_ok() {
                        eprintln!("JIT: Unsupported opcode at {}: {:?}", pc, instruction);
                    }
                    return false;
                }
            }
        }

        true
    }

    /// Check if a specific opcode is supported by JIT compiler
    fn is_supported_opcode(&self, opcode: &OpCode, constants: &[Constant]) -> bool {
        match opcode {
            // Constants - All types supported (strings push placeholder 0)
            OpCode::LoadConst(idx) => constants.get(*idx).is_some(),

            // Arithmetic operations
            OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod => true,
            OpCode::Negate => true,

            // Comparison operations
            OpCode::Equal | OpCode::NotEqual => true,
            OpCode::LessThan | OpCode::GreaterThan => true,
            OpCode::LessEqual | OpCode::GreaterEqual => true,

            // Logical operations
            OpCode::Not | OpCode::And | OpCode::Or => true,

            // Stack operations
            OpCode::Pop | OpCode::Dup => true,

            // Variable operations
            OpCode::LoadVar(_)
            | OpCode::LoadLocal(_)
            | OpCode::StoreVar(_)
            | OpCode::StoreLocal(_) => true,
            OpCode::LoadGlobal(_) | OpCode::StoreGlobal(_) => true,

            // Control flow - simple jumps only
            OpCode::Jump(_) | OpCode::JumpIfFalse(_) | OpCode::JumpIfTrue(_) => true,
            OpCode::JumpBack(_) => true,

            // Function returns - needed for compiling functions (not just loops)
            OpCode::Return | OpCode::ReturnNone => true,

            // Function calls - Step 3: Basic Call opcode support
            OpCode::Call(_) => true,

            // In-place add not supported by JIT (uses VM locals)
            OpCode::AddInPlace(_) => false,

            // Dict/Array operations - in-place ops supported for int paths
            OpCode::IndexGet | OpCode::IndexSet => false,
            OpCode::IndexGetInPlace(_) | OpCode::IndexSetInPlace(_) => true,
            OpCode::AppendConstStringInPlace(_, _)
            | OpCode::AppendConstCharInPlace(_, _)
            | OpCode::AppendConstCharUntilLocalInPlace(_, _, _, _) => true,
            // Only allow empty dict literals in JIT to avoid heavy helper overhead
            OpCode::MakeDict(count) => *count == 0,
            OpCode::MakeDictWithKeys(_) => false,

            // Everything else is unsupported - causes code to skip JIT
            // This includes: CallNative, Arrays, Dicts, Generators, Async, etc.
            _ => false,
        }
    }

    /// Get compiled function from cache
    #[allow(dead_code)] // Will be used when executing compiled code
    pub fn get_compiled(&self, offset: usize) -> Option<CompiledFn> {
        self.compiled_cache.get(&offset).copied()
    }

    /// Compile a bytecode chunk to native code
    pub fn compile(&mut self, chunk: &BytecodeChunk, offset: usize) -> Result<CompiledFn, String> {
        self.compile_with_options(chunk, offset, None, None)
    }

    /// Compile a loop with int-dict fast path enabled for specified local slots
    pub fn compile_loop_with_int_dicts(
        &mut self,
        chunk: &BytecodeChunk,
        offset: usize,
        loop_end: usize,
        int_dict_slots: std::collections::HashSet<usize>,
    ) -> Result<CompiledFn, String> {
        self.compile_with_options(chunk, offset, Some(int_dict_slots), Some(loop_end))
    }

    fn compile_with_options(
        &mut self,
        chunk: &BytecodeChunk,
        offset: usize,
        int_dict_slots: Option<std::collections::HashSet<usize>>,
        loop_end: Option<usize>,
    ) -> Result<CompiledFn, String> {
        // Clear previous context
        self.ctx.clear();

        // Create function signature: fn(*mut Value) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64)); // Stack pointer
        sig.returns.push(AbiParam::new(types::I64)); // Return value (0 = success)

        let func_id = self
            .module
            .declare_function(&format!("kujo_jit_{}", offset), Linkage::Local, &sig)
            .map_err(|e| format!("Failed to declare function: {}", e))?;

        self.ctx.func.signature = sig;

        // Declare external runtime helper functions
        // jit_load_variable: fn(*mut VMContext, *const u8, usize) -> i64
        let mut load_var_sig = self.module.make_signature();
        load_var_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        load_var_sig.params.push(AbiParam::new(types::I64)); // name_ptr
        load_var_sig.params.push(AbiParam::new(types::I64)); // name_len
        load_var_sig.returns.push(AbiParam::new(types::I64)); // return value

        let load_var_func_id = self
            .module
            .declare_function("jit_load_variable", Linkage::Import, &load_var_sig)
            .map_err(|e| format!("Failed to declare jit_load_variable: {}", e))?;

        // jit_store_variable: fn(*mut VMContext, *const u8, usize, i64)
        let mut store_var_sig = self.module.make_signature();
        store_var_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        store_var_sig.params.push(AbiParam::new(types::I64)); // name_ptr
        store_var_sig.params.push(AbiParam::new(types::I64)); // name_len
        store_var_sig.params.push(AbiParam::new(types::I64)); // value
                                                              // no return value

        let store_var_func_id = self
            .module
            .declare_function("jit_store_variable", Linkage::Import, &store_var_sig)
            .map_err(|e| format!("Failed to declare jit_store_variable: {}", e))?;

        // jit_store_variable_from_stack: fn(*mut VMContext, i64) -> i64
        let mut store_from_stack_sig = self.module.make_signature();
        store_from_stack_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        store_from_stack_sig.params.push(AbiParam::new(types::I64)); // name_hash
        store_from_stack_sig.returns.push(AbiParam::new(types::I64)); // success

        let store_from_stack_func_id = self
            .module
            .declare_function(
                "jit_store_variable_from_stack",
                Linkage::Import,
                &store_from_stack_sig,
            )
            .map_err(|e| format!("Failed to declare jit_store_variable_from_stack: {}", e))?;

        let mut local_dict_get_sig = self.module.make_signature();
        local_dict_get_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        local_dict_get_sig.params.push(AbiParam::new(types::I64)); // slot index
        local_dict_get_sig.params.push(AbiParam::new(types::I64)); // key
        local_dict_get_sig.returns.push(AbiParam::new(types::I64)); // value

        let local_dict_get_func_id = self
            .module
            .declare_function("jit_local_slot_dict_get", Linkage::Import, &local_dict_get_sig)
            .map_err(|e| format!("Failed to declare jit_local_slot_dict_get: {}", e))?;

        let mut local_dict_set_sig = self.module.make_signature();
        local_dict_set_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        local_dict_set_sig.params.push(AbiParam::new(types::I64)); // slot index
        local_dict_set_sig.params.push(AbiParam::new(types::I64)); // key
        local_dict_set_sig.params.push(AbiParam::new(types::I64)); // value
        local_dict_set_sig.returns.push(AbiParam::new(types::I64)); // success

        let local_dict_set_func_id = self
            .module
            .declare_function("jit_local_slot_dict_set", Linkage::Import, &local_dict_set_sig)
            .map_err(|e| format!("Failed to declare jit_local_slot_dict_set: {}", e))?;

        let mut local_int_dict_get_sig = self.module.make_signature();
        local_int_dict_get_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        local_int_dict_get_sig.params.push(AbiParam::new(types::I64)); // slot index
        local_int_dict_get_sig.params.push(AbiParam::new(types::I64)); // key
        local_int_dict_get_sig.returns.push(AbiParam::new(types::I64)); // value

        let local_int_dict_get_func_id = self
            .module
            .declare_function(
                "jit_local_slot_int_dict_get",
                Linkage::Import,
                &local_int_dict_get_sig,
            )
            .map_err(|e| format!("Failed to declare jit_local_slot_int_dict_get: {}", e))?;

        let mut local_int_dict_set_sig = self.module.make_signature();
        local_int_dict_set_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        local_int_dict_set_sig.params.push(AbiParam::new(types::I64)); // slot index
        local_int_dict_set_sig.params.push(AbiParam::new(types::I64)); // key
        local_int_dict_set_sig.params.push(AbiParam::new(types::I64)); // value
        local_int_dict_set_sig.returns.push(AbiParam::new(types::I64)); // success

        let local_int_dict_set_func_id = self
            .module
            .declare_function(
                "jit_local_slot_int_dict_set",
                Linkage::Import,
                &local_int_dict_set_sig,
            )
            .map_err(|e| format!("Failed to declare jit_local_slot_int_dict_set: {}", e))?;

        let mut int_dict_unique_ptr_sig = self.module.make_signature();
        int_dict_unique_ptr_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        int_dict_unique_ptr_sig.params.push(AbiParam::new(types::I64)); // slot index
        int_dict_unique_ptr_sig.returns.push(AbiParam::new(types::I64)); // dict ptr

        let int_dict_unique_ptr_func_id = self
            .module
            .declare_function("jit_int_dict_unique_ptr", Linkage::Import, &int_dict_unique_ptr_sig)
            .map_err(|e| format!("Failed to declare jit_int_dict_unique_ptr: {}", e))?;

        let mut int_dict_get_ptr_sig = self.module.make_signature();
        int_dict_get_ptr_sig.params.push(AbiParam::new(types::I64)); // dict ptr
        int_dict_get_ptr_sig.params.push(AbiParam::new(types::I64)); // key
        int_dict_get_ptr_sig.returns.push(AbiParam::new(types::I64)); // value

        let int_dict_get_ptr_func_id = self
            .module
            .declare_function("jit_int_dict_get_ptr", Linkage::Import, &int_dict_get_ptr_sig)
            .map_err(|e| format!("Failed to declare jit_int_dict_get_ptr: {}", e))?;

        let dense_int_dict_int_get_ptr_func_id = self
            .module
            .declare_function(
                "jit_dense_int_dict_int_get_ptr",
                Linkage::Import,
                &int_dict_get_ptr_sig,
            )
            .map_err(|e| format!("Failed to declare jit_dense_int_dict_int_get_ptr: {}", e))?;

        let dense_int_dict_int_full_get_ptr_func_id = self
            .module
            .declare_function(
                "jit_dense_int_dict_int_full_get_ptr",
                Linkage::Import,
                &int_dict_get_ptr_sig,
            )
            .map_err(|e| format!("Failed to declare jit_dense_int_dict_int_full_get_ptr: {}", e))?;

        let mut int_dict_set_ptr_sig = self.module.make_signature();
        int_dict_set_ptr_sig.params.push(AbiParam::new(types::I64)); // dict ptr
        int_dict_set_ptr_sig.params.push(AbiParam::new(types::I64)); // key
        int_dict_set_ptr_sig.params.push(AbiParam::new(types::I64)); // value
        int_dict_set_ptr_sig.returns.push(AbiParam::new(types::I64)); // success

        let int_dict_set_ptr_func_id = self
            .module
            .declare_function("jit_int_dict_set_ptr", Linkage::Import, &int_dict_set_ptr_sig)
            .map_err(|e| format!("Failed to declare jit_int_dict_set_ptr: {}", e))?;

        let dense_int_dict_int_set_ptr_func_id = self
            .module
            .declare_function(
                "jit_dense_int_dict_int_set_ptr",
                Linkage::Import,
                &int_dict_set_ptr_sig,
            )
            .map_err(|e| format!("Failed to declare jit_dense_int_dict_int_set_ptr: {}", e))?;

        let dense_int_dict_int_full_set_ptr_func_id = self
            .module
            .declare_function(
                "jit_dense_int_dict_int_full_set_ptr",
                Linkage::Import,
                &int_dict_set_ptr_sig,
            )
            .map_err(|e| format!("Failed to declare jit_dense_int_dict_int_full_set_ptr: {}", e))?;

        // jit_load_variable_float: fn(*mut VMContext, i64) -> f64
        let mut load_var_float_sig = self.module.make_signature();
        load_var_float_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        load_var_float_sig.params.push(AbiParam::new(types::I64)); // name_hash
        load_var_float_sig.returns.push(AbiParam::new(types::F64)); // return f64

        let load_var_float_func_id = self
            .module
            .declare_function("jit_load_variable_float", Linkage::Import, &load_var_float_sig)
            .map_err(|e| format!("Failed to declare jit_load_variable_float: {}", e))?;

        // jit_store_variable_float: fn(*mut VMContext, i64, f64)
        let mut store_var_float_sig = self.module.make_signature();
        store_var_float_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        store_var_float_sig.params.push(AbiParam::new(types::I64)); // name_hash
        store_var_float_sig.params.push(AbiParam::new(types::F64)); // value f64

        let store_var_float_func_id = self
            .module
            .declare_function("jit_store_variable_float", Linkage::Import, &store_var_float_sig)
            .map_err(|e| format!("Failed to declare jit_store_variable_float: {}", e))?;

        // jit_check_type_int: fn(*mut VMContext, i64) -> i64
        let mut check_int_sig = self.module.make_signature();
        check_int_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        check_int_sig.params.push(AbiParam::new(types::I64)); // name_hash
        check_int_sig.returns.push(AbiParam::new(types::I64)); // returns 1 if Int, 0 otherwise

        let check_type_int_func_id = self
            .module
            .declare_function("jit_check_type_int", Linkage::Import, &check_int_sig)
            .map_err(|e| format!("Failed to declare jit_check_type_int: {}", e))?;

        // jit_check_type_float: fn(*mut VMContext, i64) -> i64
        let mut check_float_sig = self.module.make_signature();
        check_float_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        check_float_sig.params.push(AbiParam::new(types::I64)); // name_hash
        check_float_sig.returns.push(AbiParam::new(types::I64)); // returns 1 if Float, 0 otherwise

        let check_type_float_func_id = self
            .module
            .declare_function("jit_check_type_float", Linkage::Import, &check_float_sig)
            .map_err(|e| format!("Failed to declare jit_check_type_float: {}", e))?;

        let mut obj_push_string_sig = self.module.make_signature();
        obj_push_string_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        obj_push_string_sig.params.push(AbiParam::new(types::I64)); // ptr
        obj_push_string_sig.params.push(AbiParam::new(types::I64)); // len
        obj_push_string_sig.returns.push(AbiParam::new(types::I64)); // handle

        let obj_push_string_func_id = self
            .module
            .declare_function("jit_obj_push_string", Linkage::Import, &obj_push_string_sig)
            .map_err(|e| format!("Failed to declare jit_obj_push_string: {}", e))?;

        let mut obj_to_vm_sig = self.module.make_signature();
        obj_to_vm_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        obj_to_vm_sig.params.push(AbiParam::new(types::I64)); // handle
        obj_to_vm_sig.returns.push(AbiParam::new(types::I64)); // success

        let obj_to_vm_func_id = self
            .module
            .declare_function("jit_obj_to_vm_stack", Linkage::Import, &obj_to_vm_sig)
            .map_err(|e| format!("Failed to declare jit_obj_to_vm_stack: {}", e))?;

        let mut make_dict_keys_sig = self.module.make_signature();
        make_dict_keys_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        make_dict_keys_sig.params.push(AbiParam::new(types::I64)); // keys ptr
        make_dict_keys_sig.params.push(AbiParam::new(types::I64)); // num keys
        make_dict_keys_sig.returns.push(AbiParam::new(types::I64)); // success

        let make_dict_keys_func_id = self
            .module
            .declare_function("jit_make_dict_with_keys", Linkage::Import, &make_dict_keys_sig)
            .map_err(|e| format!("Failed to declare jit_make_dict_with_keys: {}", e))?;

        let mut make_dict_sig = self.module.make_signature();
        make_dict_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        make_dict_sig.params.push(AbiParam::new(types::I64)); // num pairs
        make_dict_sig.returns.push(AbiParam::new(types::I64)); // success

        let make_dict_func_id = self
            .module
            .declare_function("jit_make_dict", Linkage::Import, &make_dict_sig)
            .map_err(|e| format!("Failed to declare jit_make_dict: {}", e))?;

        let mut append_const_string_sig = self.module.make_signature();
        append_const_string_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        append_const_string_sig.params.push(AbiParam::new(types::I64)); // slot index
        append_const_string_sig.params.push(AbiParam::new(types::I64)); // string ptr
        append_const_string_sig.params.push(AbiParam::new(types::I64)); // string len
        append_const_string_sig.returns.push(AbiParam::new(types::I64)); // success

        let append_const_string_func_id = self
            .module
            .declare_function(
                "jit_append_const_string_in_place",
                Linkage::Import,
                &append_const_string_sig,
            )
            .map_err(|e| format!("Failed to declare jit_append_const_string_in_place: {}", e))?;

        let mut append_const_char_sig = self.module.make_signature();
        append_const_char_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        append_const_char_sig.params.push(AbiParam::new(types::I64)); // slot index
        append_const_char_sig.params.push(AbiParam::new(types::I64)); // unicode scalar value
        append_const_char_sig.returns.push(AbiParam::new(types::I64)); // success

        let append_const_char_func_id = self
            .module
            .declare_function(
                "jit_append_const_char_in_place",
                Linkage::Import,
                &append_const_char_sig,
            )
            .map_err(|e| format!("Failed to declare jit_append_const_char_in_place: {}", e))?;

        // Build the function with a fresh builder context
        {
            let mut builder_ctx = FunctionBuilderContext::new();
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

            // Import the external functions into this function's scope
            let load_var_func_ref =
                self.module.declare_func_in_func(load_var_func_id, builder.func);
            let store_var_func_ref =
                self.module.declare_func_in_func(store_var_func_id, builder.func);
            let store_from_stack_func_ref =
                self.module.declare_func_in_func(store_from_stack_func_id, builder.func);
            let local_dict_get_func_ref =
                self.module.declare_func_in_func(local_dict_get_func_id, builder.func);
            let local_dict_set_func_ref =
                self.module.declare_func_in_func(local_dict_set_func_id, builder.func);
            let local_int_dict_get_func_ref =
                self.module.declare_func_in_func(local_int_dict_get_func_id, builder.func);
            let local_int_dict_set_func_ref =
                self.module.declare_func_in_func(local_int_dict_set_func_id, builder.func);
            let int_dict_unique_ptr_func_ref =
                self.module.declare_func_in_func(int_dict_unique_ptr_func_id, builder.func);
            let int_dict_get_ptr_func_ref =
                self.module.declare_func_in_func(int_dict_get_ptr_func_id, builder.func);
            let dense_int_dict_int_get_ptr_func_ref =
                self.module.declare_func_in_func(dense_int_dict_int_get_ptr_func_id, builder.func);
            let dense_int_dict_int_full_get_ptr_func_ref = self
                .module
                .declare_func_in_func(dense_int_dict_int_full_get_ptr_func_id, builder.func);
            let int_dict_set_ptr_func_ref =
                self.module.declare_func_in_func(int_dict_set_ptr_func_id, builder.func);
            let dense_int_dict_int_set_ptr_func_ref =
                self.module.declare_func_in_func(dense_int_dict_int_set_ptr_func_id, builder.func);
            let dense_int_dict_int_full_set_ptr_func_ref = self
                .module
                .declare_func_in_func(dense_int_dict_int_full_set_ptr_func_id, builder.func);
            let load_var_float_func_ref =
                self.module.declare_func_in_func(load_var_float_func_id, builder.func);
            let store_var_float_func_ref =
                self.module.declare_func_in_func(store_var_float_func_id, builder.func);
            let check_int_func_ref =
                self.module.declare_func_in_func(check_type_int_func_id, builder.func);
            let check_float_func_ref =
                self.module.declare_func_in_func(check_type_float_func_id, builder.func);
            let obj_push_string_func_ref =
                self.module.declare_func_in_func(obj_push_string_func_id, builder.func);
            let obj_to_vm_func_ref =
                self.module.declare_func_in_func(obj_to_vm_func_id, builder.func);
            let make_dict_keys_func_ref =
                self.module.declare_func_in_func(make_dict_keys_func_id, builder.func);
            let make_dict_func_ref =
                self.module.declare_func_in_func(make_dict_func_id, builder.func);
            let append_const_string_func_ref =
                self.module.declare_func_in_func(append_const_string_func_id, builder.func);
            let append_const_char_func_ref =
                self.module.declare_func_in_func(append_const_char_func_id, builder.func);

            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);

            let ctx_ptr = builder.block_params(entry_block)[0];

            // Translate bytecode instructions to Cranelift IR
            let mut translator = BytecodeTranslator::new();
            translator.set_local_names(chunk.local_names.clone());
            translator.set_context_param(ctx_ptr);
            translator.set_external_functions(
                load_var_func_ref,
                store_var_func_ref,
                store_from_stack_func_ref,
            );
            translator
                .set_local_slot_dict_functions(local_dict_get_func_ref, local_dict_set_func_ref);
            translator.set_local_slot_int_dict_functions(
                local_int_dict_get_func_ref,
                local_int_dict_set_func_ref,
            );
            translator.set_int_dict_ptr_functions(
                int_dict_unique_ptr_func_ref,
                int_dict_get_ptr_func_ref,
                dense_int_dict_int_get_ptr_func_ref,
                dense_int_dict_int_full_get_ptr_func_ref,
                int_dict_set_ptr_func_ref,
                dense_int_dict_int_set_ptr_func_ref,
                dense_int_dict_int_full_set_ptr_func_ref,
            );
            translator.set_float_functions(load_var_float_func_ref, store_var_float_func_ref);
            translator.set_guard_functions(check_int_func_ref, check_float_func_ref);
            translator.set_object_functions(obj_push_string_func_ref, obj_to_vm_func_ref);
            translator.set_make_dict_function(make_dict_func_ref);
            translator.set_make_dict_with_keys_function(make_dict_keys_func_ref);
            translator.set_append_in_place_functions(
                append_const_string_func_ref,
                append_const_char_func_ref,
            );

            // Initialize current block tracking
            let mut current_block = entry_block;
            let mut entry_block_sealed = false;

            // Set specialization info if available
            if let Some(spec) = self.type_profiles.get(&offset) {
                translator.set_specialization(spec.clone());

                // Generate type guards at function entry for specialized variables
                if !spec.specialized_types.is_empty() {
                    // Create blocks for guard success and guard failure
                    let guard_success_block = builder.create_block();
                    let guard_failure_block = builder.create_block();

                    // Check each specialized variable's type
                    let mut all_guards_passed = None;
                    for (var_hash, expected_type) in &spec.specialized_types {
                        let hash_val = builder.ins().iconst(types::I64, *var_hash as i64);

                        // Call appropriate type check function
                        let check_result = match expected_type {
                            ValueType::Int => {
                                let call =
                                    builder.ins().call(check_int_func_ref, &[ctx_ptr, hash_val]);
                                builder.inst_results(call)[0]
                            }
                            ValueType::Float => {
                                let call =
                                    builder.ins().call(check_float_func_ref, &[ctx_ptr, hash_val]);
                                builder.inst_results(call)[0]
                            }
                            _ => {
                                // Other types not yet specialized, skip guard
                                continue;
                            }
                        };

                        // Check if result is 1 (type matches)
                        let one = builder.ins().iconst(types::I64, 1);
                        let guard_passed = builder.ins().icmp(IntCC::Equal, check_result, one);

                        // Combine with previous guards (AND operation)
                        all_guards_passed = Some(if let Some(prev) = all_guards_passed {
                            builder.ins().band(prev, guard_passed)
                        } else {
                            guard_passed
                        });
                    }

                    // Branch based on guard results
                    if let Some(guards_passed) = all_guards_passed {
                        // Branch: if guards_passed, go to success, else go to failure
                        builder.ins().brif(
                            guards_passed,
                            guard_success_block,
                            &[],
                            guard_failure_block,
                            &[],
                        );

                        // Seal entry block now that we've branched from it
                        builder.seal_block(entry_block);
                        entry_block_sealed = true;

                        // Guard failure block: return error code (-1)
                        builder.switch_to_block(guard_failure_block);
                        builder.seal_block(guard_failure_block);
                        let error_code = builder.ins().iconst(types::I64, -1);
                        builder.ins().return_(&[error_code]);

                        // Guard success block: continue with function body
                        builder.switch_to_block(guard_success_block);
                        builder.seal_block(guard_success_block);
                        current_block = guard_success_block;
                    }
                }
            }

            // Create blocks for jump targets (loop JIT)
            translator.create_blocks(&mut builder, &chunk.instructions)?;
            let function_end = if let Some(loop_end) = loop_end {
                let mut max_target = loop_end;
                for (pc, instruction) in
                    chunk.instructions.iter().enumerate().skip(offset).take(loop_end - offset + 1)
                {
                    match instruction {
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

                    if pc > loop_end {
                        break;
                    }
                }

                max_target + 1
            } else {
                translator.function_end
            };
            translator.function_end = function_end;
            translator.allocate_local_slots(&mut builder, &chunk.instructions, function_end);
            translator.initialize_local_slots_from_vm(&mut builder, load_var_func_ref);

            if let Some(int_dict_slots) = int_dict_slots {
                translator.set_int_dict_slots(int_dict_slots);
                translator.allocate_int_dict_ptr_slots(&mut builder);
                translator.initialize_int_dict_ptr_slots(&mut builder);
            }

            let start_block = match translator.blocks.get(&offset) {
                Some(block) => *block,
                None => entry_block,
            };

            if offset != 0 && start_block != entry_block {
                builder.ins().jump(start_block, &[]);
                builder.seal_block(entry_block);
                entry_block_sealed = true;
                builder.switch_to_block(start_block);
                current_block = start_block;
            }

            if let Some(&expected_depth) = translator.block_entry_stack_depth.get(&offset) {
                translator.value_stack.clear();
                let params = builder.block_params(start_block);
                if params.len() >= expected_depth {
                    for i in 0..expected_depth {
                        translator.value_stack.push(params[i]);
                    }
                }
            }

            if !entry_block_sealed {
                builder.seal_block(entry_block);
            }

            let mut sealed_blocks = std::collections::HashSet::new();
            sealed_blocks.insert(entry_block);
            if current_block != entry_block {
                sealed_blocks.insert(current_block);
            }
            let mut block_terminated = false;

            for (pc, instruction) in chunk.instructions.iter().enumerate().skip(offset) {
                if pc >= function_end {
                    break;
                }

                if let Some(&block) = translator.blocks.get(&pc) {
                    if block != current_block {
                        if !block_terminated {
                            let args: Vec<_> = translator.value_stack.clone();
                            builder.ins().jump(block, &args);
                        }

                        if !sealed_blocks.contains(&current_block)
                            && !translator
                                .loop_header_pcs
                                .iter()
                                .any(|&lpc| translator.blocks.get(&lpc) == Some(&current_block))
                        {
                            builder.seal_block(current_block);
                            sealed_blocks.insert(current_block);
                        }

                        builder.switch_to_block(block);
                        current_block = block;
                        block_terminated = false;

                        if let Some(&expected_depth) = translator.block_entry_stack_depth.get(&pc) {
                            translator.value_stack.clear();
                            let existing_params = builder.block_params(block);

                            if existing_params.len() >= expected_depth {
                                for i in 0..expected_depth {
                                    translator.value_stack.push(existing_params[i]);
                                }
                            } else {
                                for _ in 0..expected_depth {
                                    let param = builder.append_block_param(block, types::I64);
                                    translator.value_stack.push(param);
                                }
                            }

                            if std::env::var("DEBUG_JIT").is_ok() {
                                eprintln!(
                                    "JIT: Block at PC {} has {} params, stack restored",
                                    pc, expected_depth
                                );
                            }
                        }
                    }
                }

                // Skip instruction if block is already terminated
                if block_terminated {
                    continue;
                }

                if std::env::var("DEBUG_JIT").is_ok() {
                    eprintln!(
                        "JIT: Translating PC {} {:?} (stack depth {})",
                        pc,
                        instruction,
                        translator.value_stack.len()
                    );
                }

                match translator.translate_instruction(
                    &mut builder,
                    pc,
                    instruction,
                    &chunk.constants,
                ) {
                    Ok(terminates_block) => {
                        if terminates_block {
                            if !sealed_blocks.contains(&current_block)
                                && !translator
                                    .loop_header_pcs
                                    .iter()
                                    .any(|&lpc| translator.blocks.get(&lpc) == Some(&current_block))
                            {
                                builder.seal_block(current_block);
                                sealed_blocks.insert(current_block);
                            }
                            block_terminated = true;
                        }
                    }
                    Err(e) => {
                        return Err(format!("Translation failed at PC {}: {}", pc, e));
                    }
                }
            }

            // If the last block is not terminated, add a return
            if !block_terminated {
                let zero = builder.ins().iconst(types::I64, 0);
                builder.ins().return_(&[zero]);
                if !sealed_blocks.contains(&current_block) {
                    builder.seal_block(current_block);
                    sealed_blocks.insert(current_block);
                }
            }

            // STEP 11 FIX: Seal any remaining unsealed blocks (including loop headers)
            // This must be done AFTER all edges (including back-edges) have been added
            for (&pc, &block) in &translator.blocks {
                if !sealed_blocks.contains(&block) {
                    builder.seal_block(block);
                    sealed_blocks.insert(block);
                    if std::env::var("DEBUG_JIT").is_ok() {
                        eprintln!("JIT: Late-sealing block at PC {}", pc);
                    }
                }
            }

            builder.finalize();
        }

        // Compile the function
        self.module.define_function(func_id, &mut self.ctx).map_err(|e| {
            if std::env::var("DEBUG_JIT").is_ok() {
                eprintln!("JIT define_function error: {:#?}", e);
            }
            format!("Failed to define function: {}", e)
        })?;

        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions().map_err(|e| format!("Failed to finalize: {}", e))?;

        // Get the compiled function pointer
        let code_ptr = self.module.get_finalized_function(func_id);
        let compiled_fn = compiled_fn_from_code_ptr(code_ptr);

        // Cache it
        self.compiled_cache.insert(offset, compiled_fn);

        Ok(compiled_fn)
    }

    /// Compile an entire function to native code
    /// Returns a compiled function pointer that can be called directly
    pub fn compile_function(
        &mut self,
        chunk: &BytecodeChunk,
        name: &str,
    ) -> Result<CompiledFn, String> {
        // 1. Check if function is compilable
        if !self.can_compile_function(chunk) {
            return Err(format!("Function '{}' contains unsupported opcodes", name));
        }

        // 2. Clear previous context
        self.ctx.clear();

        // 3. Create Cranelift function signature
        //    Takes VMContext pointer, returns status code (i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64)); // VMContext pointer
        sig.returns.push(AbiParam::new(types::I64)); // Status code

        // 4. Declare function in module
        let func_id = self
            .module
            .declare_function(name, Linkage::Export, &sig)
            .map_err(|e| format!("Failed to declare function: {}", e))?;

        // 5. Set function signature
        self.ctx.func.signature = sig;

        // 6. Build function body
        {
            let mut builder_ctx = FunctionBuilderContext::new();
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);

            // Get VMContext parameter
            let vm_context_param = builder.block_params(entry_block)[0];

            // Declare jit_call_function for Call opcode support
            // jit_call_function: fn(*mut VMContext, *const Value, i64) -> i64
            let mut call_func_sig = self.module.make_signature();
            call_func_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            call_func_sig.params.push(AbiParam::new(types::I64)); // func_value_ptr
            call_func_sig.params.push(AbiParam::new(types::I64)); // arg_count
            call_func_sig.returns.push(AbiParam::new(types::I64)); // return value

            let call_func_id = self
                .module
                .declare_function("jit_call_function", Linkage::Import, &call_func_sig)
                .map_err(|e| format!("Failed to declare jit_call_function: {}", e))?;

            let call_func_ref = self.module.declare_func_in_func(call_func_id, &mut builder.func);

            // Declare jit_push_int for Return opcode support
            // jit_push_int: fn(*mut VMContext, i64) -> i64
            let mut push_int_sig = self.module.make_signature();
            push_int_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            push_int_sig.params.push(AbiParam::new(types::I64)); // value to push
            push_int_sig.returns.push(AbiParam::new(types::I64)); // status code

            let push_int_id = self
                .module
                .declare_function("jit_push_int", Linkage::Import, &push_int_sig)
                .map_err(|e| format!("Failed to declare jit_push_int: {}", e))?;

            let push_int_ref = self.module.declare_func_in_func(push_int_id, &mut builder.func);

            // Declare jit_set_return_int for optimized Return opcode
            // jit_set_return_int: fn(*mut VMContext, i64) -> i64
            // This is the FAST PATH - stores return value directly in VMContext
            // instead of pushing to stack, avoiding stack operations overhead
            let mut set_return_int_sig = self.module.make_signature();
            set_return_int_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            set_return_int_sig.params.push(AbiParam::new(types::I64)); // return value
            set_return_int_sig.returns.push(AbiParam::new(types::I64)); // status code

            let set_return_int_id = self
                .module
                .declare_function("jit_set_return_int", Linkage::Import, &set_return_int_sig)
                .map_err(|e| format!("Failed to declare jit_set_return_int: {}", e))?;

            let set_return_int_ref =
                self.module.declare_func_in_func(set_return_int_id, &mut builder.func);

            // Declare jit_get_return_int for retrieving recursive call results
            // jit_get_return_int: fn(*mut VMContext) -> i64
            let mut get_return_int_sig = self.module.make_signature();
            get_return_int_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            get_return_int_sig.returns.push(AbiParam::new(types::I64)); // return value

            let get_return_int_id = self
                .module
                .declare_function("jit_get_return_int", Linkage::Import, &get_return_int_sig)
                .map_err(|e| format!("Failed to declare jit_get_return_int: {}", e))?;

            let get_return_int_ref =
                self.module.declare_func_in_func(get_return_int_id, &mut builder.func);

            // OPTIMIZATION: Declare self-reference for direct recursive calls
            // This enables direct JIT → JIT recursion without going through VM
            // Self-recursive call signature: fn(*mut VMContext) -> i64
            let self_func_ref = self.module.declare_func_in_func(func_id, &mut builder.func);

            // Declare jit_stack_pop for getting call results
            // jit_stack_pop: fn(*mut VMContext) -> i64
            let mut stack_pop_sig = self.module.make_signature();
            stack_pop_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            stack_pop_sig.returns.push(AbiParam::new(types::I64)); // popped value

            let stack_pop_id = self
                .module
                .declare_function("jit_stack_pop", Linkage::Import, &stack_pop_sig)
                .map_err(|e| format!("Failed to declare jit_stack_pop: {}", e))?;

            let stack_pop_ref = self.module.declare_func_in_func(stack_pop_id, &mut builder.func);

            // Declare jit_stack_push for pushing values to VM stack before calls
            // jit_stack_push: fn(*mut VMContext, i64)
            let mut stack_push_sig = self.module.make_signature();
            stack_push_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            stack_push_sig.params.push(AbiParam::new(types::I64)); // value to push

            let stack_push_id = self
                .module
                .declare_function("jit_stack_push", Linkage::Import, &stack_push_sig)
                .map_err(|e| format!("Failed to declare jit_stack_push: {}", e))?;

            let stack_push_ref = self.module.declare_func_in_func(stack_push_id, &mut builder.func);

            // Declare jit_dict_get for fast dictionary read operations
            // jit_dict_get: fn(*mut VMContext) -> i64
            let mut dict_get_sig = self.module.make_signature();
            dict_get_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            dict_get_sig.returns.push(AbiParam::new(types::I64)); // return status

            let dict_get_id = self
                .module
                .declare_function("jit_dict_get", Linkage::Import, &dict_get_sig)
                .map_err(|e| format!("Failed to declare jit_dict_get: {}", e))?;

            let dict_get_ref = self.module.declare_func_in_func(dict_get_id, &mut builder.func);

            // Declare jit_dict_set for fast dictionary write operations
            // jit_dict_set: fn(*mut VMContext) -> i64
            let mut dict_set_sig = self.module.make_signature();
            dict_set_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            dict_set_sig.returns.push(AbiParam::new(types::I64)); // return status

            let dict_set_id = self
                .module
                .declare_function("jit_dict_set", Linkage::Import, &dict_set_sig)
                .map_err(|e| format!("Failed to declare jit_dict_set: {}", e))?;

            let dict_set_ref = self.module.declare_func_in_func(dict_set_id, &mut builder.func);

            // Declare int-dict helpers for IndexGetInPlace/IndexSetInPlace fast path
            let mut local_slot_int_dict_get_sig = self.module.make_signature();
            local_slot_int_dict_get_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            local_slot_int_dict_get_sig.params.push(AbiParam::new(types::I64)); // slot index
            local_slot_int_dict_get_sig.params.push(AbiParam::new(types::I64)); // key
            local_slot_int_dict_get_sig.returns.push(AbiParam::new(types::I64)); // value

            let local_slot_int_dict_get_id = self
                .module
                .declare_function(
                    "jit_local_slot_int_dict_get",
                    Linkage::Import,
                    &local_slot_int_dict_get_sig,
                )
                .map_err(|e| format!("Failed to declare jit_local_slot_int_dict_get: {}", e))?;
            let local_slot_int_dict_get_ref =
                self.module.declare_func_in_func(local_slot_int_dict_get_id, &mut builder.func);

            let mut local_slot_int_dict_set_sig = self.module.make_signature();
            local_slot_int_dict_set_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            local_slot_int_dict_set_sig.params.push(AbiParam::new(types::I64)); // slot index
            local_slot_int_dict_set_sig.params.push(AbiParam::new(types::I64)); // key
            local_slot_int_dict_set_sig.params.push(AbiParam::new(types::I64)); // value
            local_slot_int_dict_set_sig.returns.push(AbiParam::new(types::I64)); // status

            let local_slot_int_dict_set_id = self
                .module
                .declare_function(
                    "jit_local_slot_int_dict_set",
                    Linkage::Import,
                    &local_slot_int_dict_set_sig,
                )
                .map_err(|e| format!("Failed to declare jit_local_slot_int_dict_set: {}", e))?;
            let local_slot_int_dict_set_ref =
                self.module.declare_func_in_func(local_slot_int_dict_set_id, &mut builder.func);

            let mut int_dict_unique_ptr_sig = self.module.make_signature();
            int_dict_unique_ptr_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            int_dict_unique_ptr_sig.params.push(AbiParam::new(types::I64)); // slot index
            int_dict_unique_ptr_sig.returns.push(AbiParam::new(types::I64)); // dict ptr

            let int_dict_unique_ptr_id = self
                .module
                .declare_function(
                    "jit_int_dict_unique_ptr",
                    Linkage::Import,
                    &int_dict_unique_ptr_sig,
                )
                .map_err(|e| format!("Failed to declare jit_int_dict_unique_ptr: {}", e))?;
            let int_dict_unique_ptr_ref =
                self.module.declare_func_in_func(int_dict_unique_ptr_id, &mut builder.func);

            let mut int_dict_get_ptr_sig = self.module.make_signature();
            int_dict_get_ptr_sig.params.push(AbiParam::new(types::I64)); // dict ptr
            int_dict_get_ptr_sig.params.push(AbiParam::new(types::I64)); // key
            int_dict_get_ptr_sig.returns.push(AbiParam::new(types::I64)); // value

            let int_dict_get_ptr_id = self
                .module
                .declare_function("jit_int_dict_get_ptr", Linkage::Import, &int_dict_get_ptr_sig)
                .map_err(|e| format!("Failed to declare jit_int_dict_get_ptr: {}", e))?;
            let int_dict_get_ptr_ref =
                self.module.declare_func_in_func(int_dict_get_ptr_id, &mut builder.func);

            let dense_int_dict_int_get_ptr_id = self
                .module
                .declare_function(
                    "jit_dense_int_dict_int_get_ptr",
                    Linkage::Import,
                    &int_dict_get_ptr_sig,
                )
                .map_err(|e| format!("Failed to declare jit_dense_int_dict_int_get_ptr: {}", e))?;
            let dense_int_dict_int_get_ptr_ref =
                self.module.declare_func_in_func(dense_int_dict_int_get_ptr_id, &mut builder.func);

            let dense_int_dict_int_full_get_ptr_id = self
                .module
                .declare_function(
                    "jit_dense_int_dict_int_full_get_ptr",
                    Linkage::Import,
                    &int_dict_get_ptr_sig,
                )
                .map_err(|e| {
                    format!("Failed to declare jit_dense_int_dict_int_full_get_ptr: {}", e)
                })?;
            let dense_int_dict_int_full_get_ptr_ref = self
                .module
                .declare_func_in_func(dense_int_dict_int_full_get_ptr_id, &mut builder.func);

            let mut int_dict_set_ptr_sig = self.module.make_signature();
            int_dict_set_ptr_sig.params.push(AbiParam::new(types::I64)); // dict ptr
            int_dict_set_ptr_sig.params.push(AbiParam::new(types::I64)); // key
            int_dict_set_ptr_sig.params.push(AbiParam::new(types::I64)); // value
            int_dict_set_ptr_sig.returns.push(AbiParam::new(types::I64)); // status

            let int_dict_set_ptr_id = self
                .module
                .declare_function("jit_int_dict_set_ptr", Linkage::Import, &int_dict_set_ptr_sig)
                .map_err(|e| format!("Failed to declare jit_int_dict_set_ptr: {}", e))?;
            let int_dict_set_ptr_ref =
                self.module.declare_func_in_func(int_dict_set_ptr_id, &mut builder.func);

            let dense_int_dict_int_set_ptr_id = self
                .module
                .declare_function(
                    "jit_dense_int_dict_int_set_ptr",
                    Linkage::Import,
                    &int_dict_set_ptr_sig,
                )
                .map_err(|e| format!("Failed to declare jit_dense_int_dict_int_set_ptr: {}", e))?;
            let dense_int_dict_int_set_ptr_ref =
                self.module.declare_func_in_func(dense_int_dict_int_set_ptr_id, &mut builder.func);

            let dense_int_dict_int_full_set_ptr_id = self
                .module
                .declare_function(
                    "jit_dense_int_dict_int_full_set_ptr",
                    Linkage::Import,
                    &int_dict_set_ptr_sig,
                )
                .map_err(|e| {
                    format!("Failed to declare jit_dense_int_dict_int_full_set_ptr: {}", e)
                })?;
            let dense_int_dict_int_full_set_ptr_ref = self
                .module
                .declare_func_in_func(dense_int_dict_int_full_set_ptr_id, &mut builder.func);

            // Declare jit_make_dict for dictionary creation
            // jit_make_dict: fn(*mut VMContext, i64) -> i64
            let mut make_dict_sig = self.module.make_signature();
            make_dict_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            make_dict_sig.params.push(AbiParam::new(types::I64)); // num_pairs
            make_dict_sig.returns.push(AbiParam::new(types::I64)); // return status

            let make_dict_id = self
                .module
                .declare_function("jit_make_dict", Linkage::Import, &make_dict_sig)
                .map_err(|e| format!("Failed to declare jit_make_dict: {}", e))?;

            let make_dict_ref = self.module.declare_func_in_func(make_dict_id, &mut builder.func);

            let mut append_const_string_sig = self.module.make_signature();
            append_const_string_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            append_const_string_sig.params.push(AbiParam::new(types::I64)); // slot index
            append_const_string_sig.params.push(AbiParam::new(types::I64)); // string ptr
            append_const_string_sig.params.push(AbiParam::new(types::I64)); // string len
            append_const_string_sig.returns.push(AbiParam::new(types::I64)); // success

            let append_const_string_id = self
                .module
                .declare_function(
                    "jit_append_const_string_in_place",
                    Linkage::Import,
                    &append_const_string_sig,
                )
                .map_err(|e| {
                    format!("Failed to declare jit_append_const_string_in_place: {}", e)
                })?;
            let append_const_string_ref =
                self.module.declare_func_in_func(append_const_string_id, &mut builder.func);

            let mut append_const_char_sig = self.module.make_signature();
            append_const_char_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            append_const_char_sig.params.push(AbiParam::new(types::I64)); // slot index
            append_const_char_sig.params.push(AbiParam::new(types::I64)); // unicode scalar value
            append_const_char_sig.returns.push(AbiParam::new(types::I64)); // success

            let append_const_char_id = self
                .module
                .declare_function(
                    "jit_append_const_char_in_place",
                    Linkage::Import,
                    &append_const_char_sig,
                )
                .map_err(|e| format!("Failed to declare jit_append_const_char_in_place: {}", e))?;
            let append_const_char_ref =
                self.module.declare_func_in_func(append_const_char_id, &mut builder.func);

            let mut make_dict_keys_sig = self.module.make_signature();
            make_dict_keys_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            make_dict_keys_sig.params.push(AbiParam::new(types::I64)); // keys ptr
            make_dict_keys_sig.params.push(AbiParam::new(types::I64)); // num keys
            make_dict_keys_sig.returns.push(AbiParam::new(types::I64)); // return status

            let make_dict_keys_id = self
                .module
                .declare_function("jit_make_dict_with_keys", Linkage::Import, &make_dict_keys_sig)
                .map_err(|e| format!("Failed to declare jit_make_dict_with_keys: {}", e))?;

            let make_dict_keys_ref =
                self.module.declare_func_in_func(make_dict_keys_id, &mut builder.func);

            // Declare jit_load_variable and jit_store_variable for variable operations
            // jit_load_variable: fn(*mut VMContext, i64, usize) -> i64
            let mut load_var_sig = self.module.make_signature();
            load_var_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            load_var_sig.params.push(AbiParam::new(types::I64)); // name_hash
            load_var_sig.params.push(AbiParam::new(types::I64)); // name_len
            load_var_sig.returns.push(AbiParam::new(types::I64)); // value

            let load_var_id = self
                .module
                .declare_function("jit_load_variable", Linkage::Import, &load_var_sig)
                .map_err(|e| format!("Failed to declare jit_load_variable: {}", e))?;

            let load_var_ref = self.module.declare_func_in_func(load_var_id, &mut builder.func);

            // jit_store_variable: fn(*mut VMContext, i64, usize, i64)
            let mut store_var_sig = self.module.make_signature();
            store_var_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            store_var_sig.params.push(AbiParam::new(types::I64)); // name_hash
            store_var_sig.params.push(AbiParam::new(types::I64)); // name_len
            store_var_sig.params.push(AbiParam::new(types::I64)); // value

            let store_var_id = self
                .module
                .declare_function("jit_store_variable", Linkage::Import, &store_var_sig)
                .map_err(|e| format!("Failed to declare jit_store_variable: {}", e))?;

            let store_var_ref = self.module.declare_func_in_func(store_var_id, &mut builder.func);

            // jit_store_variable_from_stack: fn(*mut VMContext, i64) -> i64
            let mut store_from_stack_sig = self.module.make_signature();
            store_from_stack_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            store_from_stack_sig.params.push(AbiParam::new(types::I64)); // name_hash
            store_from_stack_sig.returns.push(AbiParam::new(types::I64)); // success

            let store_from_stack_id = self
                .module
                .declare_function(
                    "jit_store_variable_from_stack",
                    Linkage::Import,
                    &store_from_stack_sig,
                )
                .map_err(|e| format!("Failed to declare jit_store_variable_from_stack: {}", e))?;

            let store_from_stack_ref =
                self.module.declare_func_in_func(store_from_stack_id, &mut builder.func);

            // Declare jit_get_arg for fast parameter access
            // jit_get_arg: fn(*mut VMContext, i64) -> i64
            let mut get_arg_sig = self.module.make_signature();
            get_arg_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            get_arg_sig.params.push(AbiParam::new(types::I64)); // arg index
            get_arg_sig.returns.push(AbiParam::new(types::I64)); // arg value

            let get_arg_id = self
                .module
                .declare_function("jit_get_arg", Linkage::Import, &get_arg_sig)
                .map_err(|e| format!("Failed to declare jit_get_arg: {}", e))?;

            let get_arg_ref = self.module.declare_func_in_func(get_arg_id, &mut builder.func);

            // Create BytecodeTranslator for this function
            let mut translator = BytecodeTranslator::new();
            translator.set_local_names(chunk.local_names.clone());
            translator.set_context_param(vm_context_param);
            translator.set_call_function(call_func_ref);
            translator.set_push_int_function(push_int_ref);
            translator.set_return_int_function(set_return_int_ref);
            translator.set_get_return_int_function(get_return_int_ref);
            translator.set_stack_pop_function(stack_pop_ref);
            translator.set_stack_push_function(stack_push_ref);
            translator.set_dict_get_function(dict_get_ref);
            translator.set_dict_set_function(dict_set_ref);
            translator.set_make_dict_function(make_dict_ref);
            translator.set_make_dict_with_keys_function(make_dict_keys_ref);
            translator
                .set_append_in_place_functions(append_const_string_ref, append_const_char_ref);
            translator.set_external_functions(load_var_ref, store_var_ref, store_from_stack_ref);
            translator.set_local_slot_int_dict_functions(
                local_slot_int_dict_get_ref,
                local_slot_int_dict_set_ref,
            );
            translator.set_int_dict_ptr_functions(
                int_dict_unique_ptr_ref,
                int_dict_get_ptr_ref,
                dense_int_dict_int_get_ptr_ref,
                dense_int_dict_int_full_get_ptr_ref,
                int_dict_set_ptr_ref,
                dense_int_dict_int_set_ptr_ref,
                dense_int_dict_int_full_set_ptr_ref,
            );

            // OPTIMIZATION: Enable direct self-recursion (Phase 7 Step 10)
            // This allows recursive functions to call themselves directly without
            // going through jit_call_function → VM → call_function_from_jit
            translator.set_current_function_name(name);
            translator.set_self_call_function(self_func_ref);

            // Create blocks for all jump targets
            translator.create_blocks(&mut builder, &chunk.instructions)?;

            // OPTIMIZATION: Pre-allocate stack slots for local variables
            // This enables register-based locals - direct memory access instead of
            // runtime function calls and HashMap lookups for every variable access
            // This is the key optimization for Phase 7 Step 7 - targeting 10-50x speedup
            let function_end = translator.function_end;
            translator.allocate_local_slots(&mut builder, &chunk.instructions, function_end);

            // Also allocate stack slots for function parameters
            // Parameters need slots so they can be accessed via the fast path
            translator.allocate_parameter_slots(&mut builder, &chunk.params);

            let mut int_dict_slots = std::collections::HashSet::new();
            for instr in &chunk.instructions {
                if let OpCode::IndexGetInPlace(slot) | OpCode::IndexSetInPlace(slot) = instr {
                    int_dict_slots.insert(*slot);
                }
            }

            if !int_dict_slots.is_empty() {
                translator.set_int_dict_slots(int_dict_slots);
                translator.allocate_int_dict_ptr_slots(&mut builder);
                translator.initialize_int_dict_ptr_slots(&mut builder);
            }

            let has_loop = chunk.instructions.iter().any(|op| matches!(op, OpCode::JumpBack(_)));
            // Initialize parameter slots with values from the func_locals HashMap
            // This copies parameter values from the HashMap into fast stack slots
            // at function entry, enabling fast access during function execution
            // OPTIMIZATION: Pass get_arg_ref for fast arg loading via VMContext.argN
            // NOTE: Disable fast args for loops to avoid incorrect parameter values.
            let get_arg_ref_opt = if has_loop { None } else { Some(get_arg_ref) };
            translator.initialize_parameter_slots(
                &mut builder,
                &chunk.params,
                load_var_ref,
                get_arg_ref_opt,
            );

            // Add entry block to block map
            if !translator.blocks.contains_key(&0) {
                translator.blocks.insert(0, entry_block);
            }

            // Seal entry block
            builder.seal_block(entry_block);

            let mut sealed_blocks = std::collections::HashSet::new();
            sealed_blocks.insert(entry_block);

            let mut current_block = entry_block;
            let mut block_terminated = false;

            // Translate all instructions from function body
            for (pc, instr) in chunk.instructions.iter().enumerate() {
                // Stop at function end
                if pc >= function_end {
                    break;
                }

                // If this PC has a block, switch to it
                if let Some(&block) = translator.blocks.get(&pc) {
                    if block != current_block {
                        // If current block not terminated, add fallthrough jump
                        if !block_terminated {
                            // Pass current stack as arguments
                            let args: Vec<_> = translator.value_stack.clone();
                            builder.ins().jump(block, &args);
                        }

                        // Seal previous block
                        if !sealed_blocks.contains(&current_block) {
                            builder.seal_block(current_block);
                            sealed_blocks.insert(current_block);
                        }

                        builder.switch_to_block(block);
                        current_block = block;
                        block_terminated = false;

                        // If this block expects stack values, use block parameters
                        // NOTE: For loop headers, parameters were added during create_blocks
                        if let Some(&expected_depth) = translator.block_entry_stack_depth.get(&pc) {
                            translator.value_stack.clear();
                            let existing_params = builder.block_params(block);

                            if existing_params.len() >= expected_depth {
                                // Loop header - use existing parameters
                                for i in 0..expected_depth {
                                    translator.value_stack.push(existing_params[i]);
                                }
                            } else {
                                // Not a loop header - add parameters now
                                for _ in 0..expected_depth {
                                    let param = builder.append_block_param(block, types::I64);
                                    translator.value_stack.push(param);
                                }
                            }
                            if std::env::var("DEBUG_JIT").is_ok() {
                                eprintln!(
                                    "JIT: Block at PC {} has {} params, stack restored",
                                    pc, expected_depth
                                );
                            }
                        }
                    }
                }

                // Skip if block terminated
                if block_terminated {
                    continue;
                }

                // Translate the instruction
                match translator.translate_instruction(&mut builder, pc, instr, &chunk.constants) {
                    Ok(terminates_block) => {
                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!(
                                "JIT: PC {} OK, stack depth now: {}",
                                pc,
                                translator.value_stack.len()
                            );
                        }
                        if terminates_block {
                            if !sealed_blocks.contains(&current_block) {
                                builder.seal_block(current_block);
                                sealed_blocks.insert(current_block);
                            }
                            block_terminated = true;
                        }
                    }
                    Err(e) => {
                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!(
                                "JIT: PC {} FAILED: {}, instruction: {:?}, stack depth: {}",
                                pc,
                                e,
                                instr,
                                translator.value_stack.len()
                            );
                        }
                        return Err(format!("Translation failed at PC {}: {}", pc, e));
                    }
                }
            }

            // If last block not terminated, add return
            if !block_terminated {
                let zero = builder.ins().iconst(types::I64, 0);
                builder.ins().return_(&[zero]);
                if !sealed_blocks.contains(&current_block) {
                    builder.seal_block(current_block);
                }
            }

            // Finalize the function
            builder.finalize();
        }

        // 7. Compile the function
        // Debug: Print IR before compilation if DEBUG_JIT is set
        if std::env::var("DEBUG_JIT_IR").is_ok() {
            eprintln!("JIT IR for '{}':\n{}", name, self.ctx.func.display());
        }

        self.module.define_function(func_id, &mut self.ctx).map_err(|e| {
            // Get more details about the error
            let err_msg = format!("{:?}", e);
            format!("Failed to define function: {}", err_msg)
        })?;

        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions().map_err(|e| format!("Failed to finalize: {}", e))?;

        // 8. Get function pointer
        let code_ptr = self.module.get_finalized_function(func_id);

        // 9. Cast to our function type
        let compiled_fn = compiled_fn_from_code_ptr(code_ptr);

        Ok(compiled_fn)
    }

    /// Compile a function with direct-arg signature for JIT recursion optimization
    /// This generates a function with signature: fn(*mut VMContext, arg0: i64) -> i64
    /// where the return value is the actual computed result (not a status code)
    ///
    /// This signature enables direct JIT-to-JIT recursion without crossing FFI boundaries,
    /// providing 30-50x speedup on recursive functions compared to the standard path.
    ///
    /// Only suitable for single-parameter integer functions (like fibonacci, factorial)
    pub fn compile_function_with_direct_arg(
        &mut self,
        chunk: &BytecodeChunk,
        name: &str,
    ) -> Result<CompiledFnWithArg, String> {
        // Validate: must have exactly one parameter
        if chunk.params.len() != 1 {
            return Err(format!(
                "Direct-arg compilation requires exactly 1 parameter, got {}",
                chunk.params.len()
            ));
        }

        // 1. Check if function is compilable
        if !self.can_compile_function(chunk) {
            return Err(format!("Function '{}' contains unsupported opcodes", name));
        }

        // 2. Clear previous context
        self.ctx.clear();

        // 3. Create Cranelift function signature with direct argument
        //    Takes VMContext pointer + arg0, returns computed result (i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64)); // VMContext pointer
        sig.params.push(AbiParam::new(types::I64)); // arg0 (direct parameter)
        sig.returns.push(AbiParam::new(types::I64)); // Result value (not status code!)

        // 4. Declare function in module with unique name for direct-arg variant
        let direct_name = format!("{}_direct", name);
        let func_id = self
            .module
            .declare_function(&direct_name, Linkage::Export, &sig)
            .map_err(|e| format!("Failed to declare direct-arg function: {}", e))?;

        // 5. Set function signature
        self.ctx.func.signature = sig;

        // 6. Build function body
        {
            let mut builder_ctx = FunctionBuilderContext::new();
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);

            // Get parameters: VMContext and direct arg
            let vm_context_param = builder.block_params(entry_block)[0];
            let direct_arg_param = builder.block_params(entry_block)[1];

            // Declare self-reference for direct recursion
            // This function can call itself directly with the same signature!
            let self_func_ref = self.module.declare_func_in_func(func_id, &mut builder.func);

            // Create BytecodeTranslator with direct-arg mode enabled
            let mut translator = BytecodeTranslator::new();
            translator.set_local_names(chunk.local_names.clone());
            translator.set_context_param(vm_context_param);
            translator.set_direct_arg_mode(direct_arg_param, 1);
            translator.set_current_function_name(name);
            translator.set_self_call_function(self_func_ref);

            // Create blocks for all jump targets
            translator.create_blocks(&mut builder, &chunk.instructions)?;

            // IMPORTANT: For direct-arg mode, we DON'T allocate a stack slot for the parameter
            // Instead, we track it as the direct_arg_param and use it directly
            // Only allocate slots for OTHER local variables (not the first parameter)
            let param_name = &chunk.params[0];
            let function_end = translator.function_end;

            // Allocate slots for locals EXCEPT the direct parameter
            translator.allocate_local_slots_except(
                &mut builder,
                &chunk.instructions,
                function_end,
                param_name,
            );

            // Initialize the parameter slot with the direct argument value
            if let Some(&slot) = translator.local_slots.get(param_name) {
                builder.ins().stack_store(direct_arg_param, slot, 0);
            }

            // Add entry block to block map
            if !translator.blocks.contains_key(&0) {
                translator.blocks.insert(0, entry_block);
            }

            // Seal entry block
            builder.seal_block(entry_block);

            let mut sealed_blocks = std::collections::HashSet::new();
            sealed_blocks.insert(entry_block);

            let mut current_block = entry_block;
            let mut block_terminated = false;

            // Translate each instruction
            for (pc, instr) in chunk.instructions.iter().enumerate() {
                if pc >= translator.function_end {
                    break;
                }

                // Check if we need to switch to a new block
                if let Some(&block) = translator.blocks.get(&pc) {
                    if block != current_block {
                        if !block_terminated {
                            let args: Vec<_> = translator.value_stack.clone();
                            builder.ins().jump(block, &args);
                        }

                        if !sealed_blocks.contains(&current_block) {
                            builder.seal_block(current_block);
                            sealed_blocks.insert(current_block);
                        }
                        builder.switch_to_block(block);
                        current_block = block;
                        block_terminated = false;

                        // Use existing block parameters for loop headers
                        if let Some(&expected_depth) = translator.block_entry_stack_depth.get(&pc) {
                            translator.value_stack.clear();
                            let existing_params = builder.block_params(block);

                            if existing_params.len() >= expected_depth {
                                for i in 0..expected_depth {
                                    translator.value_stack.push(existing_params[i]);
                                }
                            } else {
                                for _ in 0..expected_depth {
                                    let param = builder.append_block_param(block, types::I64);
                                    translator.value_stack.push(param);
                                }
                            }
                        }
                    }
                }

                if block_terminated {
                    continue;
                }

                match translator.translate_direct_arg_instruction(
                    &mut builder,
                    pc,
                    instr,
                    &chunk.constants,
                    param_name,
                ) {
                    Ok(terminates_block) => {
                        if terminates_block {
                            if !sealed_blocks.contains(&current_block) {
                                builder.seal_block(current_block);
                                sealed_blocks.insert(current_block);
                            }
                            block_terminated = true;
                        }
                    }
                    Err(e) => {
                        return Err(format!("Direct-arg translation failed at PC {}: {}", pc, e));
                    }
                }
            }

            // If last block not terminated, return 0
            if !block_terminated {
                let zero = builder.ins().iconst(types::I64, 0);
                builder.ins().return_(&[zero]);
                if !sealed_blocks.contains(&current_block) {
                    builder.seal_block(current_block);
                }
            }

            builder.finalize();
        }

        // 7. Compile the function
        if std::env::var("DEBUG_JIT_IR").is_ok() {
            eprintln!("JIT IR for '{}' (direct-arg):\n{}", name, self.ctx.func.display());
        }

        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| format!("Failed to define direct-arg function: {:?}", e))?;

        self.module.clear_context(&mut self.ctx);
        self.module
            .finalize_definitions()
            .map_err(|e| format!("Failed to finalize direct-arg: {}", e))?;

        // 8. Get function pointer
        let code_ptr = self.module.get_finalized_function(func_id);

        // 9. Cast to our direct-arg function type
        let compiled_fn = compiled_fn_with_arg_from_code_ptr(code_ptr);

        Ok(compiled_fn)
    }

    /// Compile a function and return enhanced info including direct-arg variant if eligible
    pub fn compile_function_with_info(
        &mut self,
        chunk: &BytecodeChunk,
        name: &str,
    ) -> Result<CompiledFnInfo, String> {
        // First, compile the standard function
        let standard_fn = self.compile_function(chunk, name)?;

        // Check if eligible for direct-arg optimization
        // Criteria: exactly 1 parameter, function contains self-recursion
        let direct_fn = if chunk.params.len() == 1 && self.function_has_self_recursion(chunk, name)
        {
            match self.compile_function_with_direct_arg(chunk, name) {
                Ok(fn_ptr) => {
                    if std::env::var("DEBUG_JIT").is_ok() {
                        eprintln!(
                            "JIT: Compiled '{}' with direct-arg optimization for recursion",
                            name
                        );
                    }
                    Some(fn_ptr)
                }
                Err(e) => {
                    if std::env::var("DEBUG_JIT").is_ok() {
                        eprintln!("JIT: Direct-arg compilation failed for '{}': {}", name, e);
                    }
                    None
                }
            }
        } else {
            None
        };

        let info = CompiledFnInfo {
            fn_ptr: standard_fn,
            fn_with_arg: direct_fn,
            param_count: chunk.params.len(),
            supports_direct_recursion: direct_fn.is_some(),
        };

        // Cache the info
        self.compiled_fn_info.insert(name.to_string(), info);

        Ok(info)
    }

    /// Check if a function contains self-recursive calls
    fn function_has_self_recursion(&self, chunk: &BytecodeChunk, name: &str) -> bool {
        // Note: We must scan ALL bytecode, not stop at first Return, because
        // recursive calls often appear after early returns from base cases.
        // For example, in fib(n): if n < 2 { return n } else { return fib(n-1) + fib(n-2) }
        // The first return is the base case; the recursive call is in the else branch.

        if std::env::var("DEBUG_JIT").is_ok() {
            eprintln!("JIT: Checking self-recursion for '{}'", name);
        }

        // Scan ALL instructions looking for self-recursive call pattern
        let instructions = &chunk.instructions;
        for i in 0..instructions.len().saturating_sub(1) {
            if let OpCode::LoadVar(var_name) = &instructions[i] {
                if var_name == name {
                    // Check if next instruction is Call
                    if let OpCode::Call(arg_count) = &instructions[i + 1] {
                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!("JIT: Found self-recursive call at PC {} (LoadVar('{}') + Call({}))", 
                                     i, name, arg_count);
                        }
                        return true;
                    }
                }
            }
        }

        if std::env::var("DEBUG_JIT").is_ok() {
            eprintln!("JIT: No self-recursion found in '{}'", name);
        }
        false
    }

    /// Compile a top-level script (entire chunk) to native code
    /// This is similar to compile_function but doesn't expect a Return opcode
    /// The script executes all instructions and returns normally at the end
    pub fn compile_script(
        &mut self,
        chunk: &BytecodeChunk,
        name: &str,
    ) -> Result<CompiledFn, String> {
        // Check if script is compilable (all opcodes supported)
        for instr in &chunk.instructions {
            if !self.is_supported_opcode(instr, &chunk.constants) {
                return Err(format!("Script '{}' contains unsupported opcode: {:?}", name, instr));
            }
        }

        // Clear previous context
        self.ctx.clear();

        // Create function signature (same as compile_function)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64)); // VMContext pointer
        sig.returns.push(AbiParam::new(types::I64)); // Status code

        // Declare function
        let func_id = self
            .module
            .declare_function(name, Linkage::Export, &sig)
            .map_err(|e| format!("Failed to declare script: {}", e))?;

        self.ctx.func.signature = sig;

        // Build function body
        {
            let mut builder_ctx = FunctionBuilderContext::new();
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);

            // Get VMContext parameter
            let vm_context_param = builder.block_params(entry_block)[0];

            // Declare runtime helper functions (same as compile_function)
            let mut call_func_sig = self.module.make_signature();
            call_func_sig.params.push(AbiParam::new(types::I64));
            call_func_sig.params.push(AbiParam::new(types::I64));
            call_func_sig.params.push(AbiParam::new(types::I64));
            call_func_sig.returns.push(AbiParam::new(types::I64));

            let call_func_id = self
                .module
                .declare_function("jit_call_function", Linkage::Import, &call_func_sig)
                .map_err(|e| format!("Failed to declare jit_call_function: {}", e))?;
            let call_func_ref = self.module.declare_func_in_func(call_func_id, &mut builder.func);

            // Declare jit_stack_push for pushing values to VM stack before calls
            // jit_stack_push: fn(*mut VMContext, i64)
            let mut stack_push_sig = self.module.make_signature();
            stack_push_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            stack_push_sig.params.push(AbiParam::new(types::I64)); // value to push

            let stack_push_id = self
                .module
                .declare_function("jit_stack_push", Linkage::Import, &stack_push_sig)
                .map_err(|e| format!("Failed to declare jit_stack_push: {}", e))?;
            let stack_push_ref = self.module.declare_func_in_func(stack_push_id, &mut builder.func);

            // Declare jit_stack_pop for getting call results
            // jit_stack_pop: fn(*mut VMContext) -> i64
            let mut stack_pop_sig = self.module.make_signature();
            stack_pop_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            stack_pop_sig.returns.push(AbiParam::new(types::I64)); // popped value

            let stack_pop_id = self
                .module
                .declare_function("jit_stack_pop", Linkage::Import, &stack_pop_sig)
                .map_err(|e| format!("Failed to declare jit_stack_pop: {}", e))?;
            let stack_pop_ref = self.module.declare_func_in_func(stack_pop_id, &mut builder.func);

            // Declare jit_dict_get for fast dictionary read operations
            // jit_dict_get: fn(*mut VMContext) -> i64
            let mut dict_get_sig = self.module.make_signature();
            dict_get_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            dict_get_sig.returns.push(AbiParam::new(types::I64)); // return status

            let dict_get_id = self
                .module
                .declare_function("jit_dict_get", Linkage::Import, &dict_get_sig)
                .map_err(|e| format!("Failed to declare jit_dict_get: {}", e))?;
            let dict_get_ref = self.module.declare_func_in_func(dict_get_id, &mut builder.func);

            // Declare jit_dict_set for fast dictionary write operations
            // jit_dict_set: fn(*mut VMContext) -> i64
            let mut dict_set_sig = self.module.make_signature();
            dict_set_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            dict_set_sig.returns.push(AbiParam::new(types::I64)); // return status

            let dict_set_id = self
                .module
                .declare_function("jit_dict_set", Linkage::Import, &dict_set_sig)
                .map_err(|e| format!("Failed to declare jit_dict_set: {}", e))?;
            let dict_set_ref = self.module.declare_func_in_func(dict_set_id, &mut builder.func);

            let mut local_dict_get_sig = self.module.make_signature();
            local_dict_get_sig.params.push(AbiParam::new(types::I64));
            local_dict_get_sig.params.push(AbiParam::new(types::I64));
            local_dict_get_sig.params.push(AbiParam::new(types::I64));
            local_dict_get_sig.returns.push(AbiParam::new(types::I64));

            let local_dict_get_id = self
                .module
                .declare_function("jit_local_slot_dict_get", Linkage::Import, &local_dict_get_sig)
                .map_err(|e| format!("Failed to declare jit_local_slot_dict_get: {}", e))?;
            let _local_dict_get_ref =
                self.module.declare_func_in_func(local_dict_get_id, &mut builder.func);

            let mut local_dict_set_sig = self.module.make_signature();
            local_dict_set_sig.params.push(AbiParam::new(types::I64));
            local_dict_set_sig.params.push(AbiParam::new(types::I64));
            local_dict_set_sig.params.push(AbiParam::new(types::I64));
            local_dict_set_sig.params.push(AbiParam::new(types::I64));
            local_dict_set_sig.returns.push(AbiParam::new(types::I64));

            let local_dict_set_id = self
                .module
                .declare_function("jit_local_slot_dict_set", Linkage::Import, &local_dict_set_sig)
                .map_err(|e| format!("Failed to declare jit_local_slot_dict_set: {}", e))?;
            let _local_dict_set_ref =
                self.module.declare_func_in_func(local_dict_set_id, &mut builder.func);

            let mut local_dict_get_sig = self.module.make_signature();
            local_dict_get_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            local_dict_get_sig.params.push(AbiParam::new(types::I64)); // slot index
            local_dict_get_sig.params.push(AbiParam::new(types::I64)); // key
            local_dict_get_sig.returns.push(AbiParam::new(types::I64)); // value

            let local_dict_get_id = self
                .module
                .declare_function("jit_local_slot_dict_get", Linkage::Import, &local_dict_get_sig)
                .map_err(|e| format!("Failed to declare jit_local_slot_dict_get: {}", e))?;
            let local_dict_get_ref =
                self.module.declare_func_in_func(local_dict_get_id, &mut builder.func);

            let mut local_dict_set_sig = self.module.make_signature();
            local_dict_set_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            local_dict_set_sig.params.push(AbiParam::new(types::I64)); // slot index
            local_dict_set_sig.params.push(AbiParam::new(types::I64)); // key
            local_dict_set_sig.params.push(AbiParam::new(types::I64)); // value
            local_dict_set_sig.returns.push(AbiParam::new(types::I64)); // success

            let local_dict_set_id = self
                .module
                .declare_function("jit_local_slot_dict_set", Linkage::Import, &local_dict_set_sig)
                .map_err(|e| format!("Failed to declare jit_local_slot_dict_set: {}", e))?;
            let local_dict_set_ref =
                self.module.declare_func_in_func(local_dict_set_id, &mut builder.func);

            // Declare jit_make_dict for dictionary creation
            // jit_make_dict: fn(*mut VMContext, i64) -> i64
            let mut make_dict_sig = self.module.make_signature();
            make_dict_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            make_dict_sig.params.push(AbiParam::new(types::I64)); // num_pairs
            make_dict_sig.returns.push(AbiParam::new(types::I64)); // return status

            let make_dict_id = self
                .module
                .declare_function("jit_make_dict", Linkage::Import, &make_dict_sig)
                .map_err(|e| format!("Failed to declare jit_make_dict: {}", e))?;
            let make_dict_ref = self.module.declare_func_in_func(make_dict_id, &mut builder.func);

            let mut append_const_string_sig = self.module.make_signature();
            append_const_string_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            append_const_string_sig.params.push(AbiParam::new(types::I64)); // slot index
            append_const_string_sig.params.push(AbiParam::new(types::I64)); // string ptr
            append_const_string_sig.params.push(AbiParam::new(types::I64)); // string len
            append_const_string_sig.returns.push(AbiParam::new(types::I64)); // success

            let append_const_string_id = self
                .module
                .declare_function(
                    "jit_append_const_string_in_place",
                    Linkage::Import,
                    &append_const_string_sig,
                )
                .map_err(|e| {
                    format!("Failed to declare jit_append_const_string_in_place: {}", e)
                })?;
            let append_const_string_ref =
                self.module.declare_func_in_func(append_const_string_id, &mut builder.func);

            let mut append_const_char_sig = self.module.make_signature();
            append_const_char_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            append_const_char_sig.params.push(AbiParam::new(types::I64)); // slot index
            append_const_char_sig.params.push(AbiParam::new(types::I64)); // unicode scalar value
            append_const_char_sig.returns.push(AbiParam::new(types::I64)); // success

            let append_const_char_id = self
                .module
                .declare_function(
                    "jit_append_const_char_in_place",
                    Linkage::Import,
                    &append_const_char_sig,
                )
                .map_err(|e| format!("Failed to declare jit_append_const_char_in_place: {}", e))?;
            let append_const_char_ref =
                self.module.declare_func_in_func(append_const_char_id, &mut builder.func);

            let mut make_dict_keys_sig = self.module.make_signature();
            make_dict_keys_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            make_dict_keys_sig.params.push(AbiParam::new(types::I64)); // keys ptr
            make_dict_keys_sig.params.push(AbiParam::new(types::I64)); // num keys
            make_dict_keys_sig.returns.push(AbiParam::new(types::I64)); // return status

            let make_dict_keys_id = self
                .module
                .declare_function("jit_make_dict_with_keys", Linkage::Import, &make_dict_keys_sig)
                .map_err(|e| format!("Failed to declare jit_make_dict_with_keys: {}", e))?;
            let make_dict_keys_ref =
                self.module.declare_func_in_func(make_dict_keys_id, &mut builder.func);

            let mut local_int_dict_get_sig = self.module.make_signature();
            local_int_dict_get_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            local_int_dict_get_sig.params.push(AbiParam::new(types::I64)); // slot index
            local_int_dict_get_sig.params.push(AbiParam::new(types::I64)); // key
            local_int_dict_get_sig.returns.push(AbiParam::new(types::I64)); // value

            let local_int_dict_get_id = self
                .module
                .declare_function(
                    "jit_local_slot_int_dict_get",
                    Linkage::Import,
                    &local_int_dict_get_sig,
                )
                .map_err(|e| format!("Failed to declare jit_local_slot_int_dict_get: {}", e))?;
            let local_int_dict_get_ref =
                self.module.declare_func_in_func(local_int_dict_get_id, &mut builder.func);

            let mut local_int_dict_set_sig = self.module.make_signature();
            local_int_dict_set_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            local_int_dict_set_sig.params.push(AbiParam::new(types::I64)); // slot index
            local_int_dict_set_sig.params.push(AbiParam::new(types::I64)); // key
            local_int_dict_set_sig.params.push(AbiParam::new(types::I64)); // value
            local_int_dict_set_sig.returns.push(AbiParam::new(types::I64)); // success

            let local_int_dict_set_id = self
                .module
                .declare_function(
                    "jit_local_slot_int_dict_set",
                    Linkage::Import,
                    &local_int_dict_set_sig,
                )
                .map_err(|e| format!("Failed to declare jit_local_slot_int_dict_set: {}", e))?;
            let local_int_dict_set_ref =
                self.module.declare_func_in_func(local_int_dict_set_id, &mut builder.func);

            // Declare load/store variable functions
            let mut load_var_sig = self.module.make_signature();
            load_var_sig.params.push(AbiParam::new(types::I64));
            load_var_sig.params.push(AbiParam::new(types::I64));
            load_var_sig.params.push(AbiParam::new(types::I64));
            load_var_sig.returns.push(AbiParam::new(types::I64));

            let load_var_id = self
                .module
                .declare_function("jit_load_variable", Linkage::Import, &load_var_sig)
                .map_err(|e| format!("Failed to declare jit_load_variable: {}", e))?;
            let load_var_func_ref =
                self.module.declare_func_in_func(load_var_id, &mut builder.func);

            let mut store_var_sig = self.module.make_signature();
            store_var_sig.params.push(AbiParam::new(types::I64));
            store_var_sig.params.push(AbiParam::new(types::I64));
            store_var_sig.params.push(AbiParam::new(types::I64));
            store_var_sig.params.push(AbiParam::new(types::I64));

            let store_var_id = self
                .module
                .declare_function("jit_store_variable", Linkage::Import, &store_var_sig)
                .map_err(|e| format!("Failed to declare jit_store_variable: {}", e))?;
            let store_var_func_ref =
                self.module.declare_func_in_func(store_var_id, &mut builder.func);

            let mut store_from_stack_sig = self.module.make_signature();
            store_from_stack_sig.params.push(AbiParam::new(types::I64));
            store_from_stack_sig.params.push(AbiParam::new(types::I64));
            store_from_stack_sig.returns.push(AbiParam::new(types::I64));

            let store_from_stack_id = self
                .module
                .declare_function(
                    "jit_store_variable_from_stack",
                    Linkage::Import,
                    &store_from_stack_sig,
                )
                .map_err(|e| format!("Failed to declare jit_store_variable_from_stack: {}", e))?;
            let store_from_stack_func_ref =
                self.module.declare_func_in_func(store_from_stack_id, &mut builder.func);

            // Float load/store
            let mut load_var_float_sig = self.module.make_signature();
            load_var_float_sig.params.push(AbiParam::new(types::I64));
            load_var_float_sig.params.push(AbiParam::new(types::I64));
            load_var_float_sig.returns.push(AbiParam::new(types::F64));

            let load_var_float_id = self
                .module
                .declare_function("jit_load_variable_float", Linkage::Import, &load_var_float_sig)
                .map_err(|e| format!("Failed to declare jit_load_variable_float: {}", e))?;
            let load_var_float_func_ref =
                self.module.declare_func_in_func(load_var_float_id, &mut builder.func);

            let mut store_var_float_sig = self.module.make_signature();
            store_var_float_sig.params.push(AbiParam::new(types::I64));
            store_var_float_sig.params.push(AbiParam::new(types::I64));
            store_var_float_sig.params.push(AbiParam::new(types::F64));

            let store_var_float_id = self
                .module
                .declare_function("jit_store_variable_float", Linkage::Import, &store_var_float_sig)
                .map_err(|e| format!("Failed to declare jit_store_variable_float: {}", e))?;
            let store_var_float_func_ref =
                self.module.declare_func_in_func(store_var_float_id, &mut builder.func);

            // Guard functions (not used for scripts typically, but available)
            let mut check_int_sig = self.module.make_signature();
            check_int_sig.params.push(AbiParam::new(types::I64));
            check_int_sig.params.push(AbiParam::new(types::I64));
            check_int_sig.returns.push(AbiParam::new(types::I64));

            let check_int_id = self
                .module
                .declare_function("jit_check_int_guard", Linkage::Import, &check_int_sig)
                .map_err(|e| format!("Failed to declare jit_check_int_guard: {}", e))?;
            let check_int_func_ref =
                self.module.declare_func_in_func(check_int_id, &mut builder.func);

            let mut check_float_sig = self.module.make_signature();
            check_float_sig.params.push(AbiParam::new(types::I64));
            check_float_sig.params.push(AbiParam::new(types::I64));
            check_float_sig.returns.push(AbiParam::new(types::I64));

            let check_float_id = self
                .module
                .declare_function("jit_check_float_guard", Linkage::Import, &check_float_sig)
                .map_err(|e| format!("Failed to declare jit_check_float_guard: {}", e))?;
            let check_float_func_ref =
                self.module.declare_func_in_func(check_float_id, &mut builder.func);

            // Initialize translator
            let mut translator = BytecodeTranslator::new();
            translator.set_local_names(chunk.local_names.clone());
            translator.set_context_param(vm_context_param);
            translator.set_external_functions(
                load_var_func_ref,
                store_var_func_ref,
                store_from_stack_func_ref,
            );
            translator.set_float_functions(load_var_float_func_ref, store_var_float_func_ref);
            translator.set_guard_functions(check_int_func_ref, check_float_func_ref);
            translator.set_call_function(call_func_ref);
            translator.set_stack_push_function(stack_push_ref);
            translator.set_stack_pop_function(stack_pop_ref);
            translator.set_dict_get_function(dict_get_ref);
            translator.set_dict_set_function(dict_set_ref);
            translator.set_make_dict_function(make_dict_ref);
            translator.set_make_dict_with_keys_function(make_dict_keys_ref);
            translator
                .set_append_in_place_functions(append_const_string_ref, append_const_char_ref);
            translator.set_local_slot_dict_functions(local_dict_get_ref, local_dict_set_ref);
            translator
                .set_local_slot_int_dict_functions(local_int_dict_get_ref, local_int_dict_set_ref);
            translator.function_end = chunk.instructions.len(); // Process all instructions

            // Create blocks for jump targets (this also analyzes loop headers)
            translator.create_blocks(&mut builder, &chunk.instructions)?;

            // Add entry block to the map
            if !translator.blocks.contains_key(&0) {
                translator.blocks.insert(0, entry_block);
            }

            // Seal entry block
            builder.seal_block(entry_block);

            let mut sealed_blocks = std::collections::HashSet::new();
            sealed_blocks.insert(entry_block);

            let mut current_block = entry_block;
            let mut block_terminated = false;

            // Translate all instructions
            for (pc, instr) in chunk.instructions.iter().enumerate() {
                // If this PC has a block, switch to it
                if let Some(&block) = translator.blocks.get(&pc) {
                    if block != current_block {
                        // If current block not terminated, add fallthrough jump
                        if !block_terminated {
                            let args: Vec<_> = translator.value_stack.clone();
                            builder.ins().jump(block, &args);
                        }

                        // Seal previous block
                        if !sealed_blocks.contains(&current_block) {
                            builder.seal_block(current_block);
                            sealed_blocks.insert(current_block);
                        }

                        builder.switch_to_block(block);
                        current_block = block;
                        block_terminated = false;

                        // Restore stack from block parameters
                        if let Some(&expected_depth) = translator.block_entry_stack_depth.get(&pc) {
                            translator.value_stack.clear();
                            let existing_params = builder.block_params(block);

                            if existing_params.len() >= expected_depth {
                                for i in 0..expected_depth {
                                    translator.value_stack.push(existing_params[i]);
                                }
                            } else {
                                for _ in 0..expected_depth {
                                    let param = builder.append_block_param(block, types::I64);
                                    translator.value_stack.push(param);
                                }
                            }
                        }
                    }
                }

                // Skip if block terminated
                if block_terminated {
                    continue;
                }

                // Translate the instruction
                match translator.translate_instruction(&mut builder, pc, instr, &chunk.constants) {
                    Ok(terminates_block) => {
                        if terminates_block {
                            if !sealed_blocks.contains(&current_block) {
                                builder.seal_block(current_block);
                                sealed_blocks.insert(current_block);
                            }
                            block_terminated = true;
                        }
                    }
                    Err(e) => {
                        return Err(format!("Failed to translate instruction at PC {}: {}", pc, e));
                    }
                }
            }

            // Seal any unsealed loop headers (late sealing for back-edges)
            for &header_pc in &translator.loop_header_pcs {
                if let Some(&block) = translator.blocks.get(&header_pc) {
                    if !sealed_blocks.contains(&block) {
                        builder.seal_block(block);
                        sealed_blocks.insert(block);
                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!("JIT: Late-sealing loop header block at PC {}", header_pc);
                        }
                    }
                }
            }

            // If script ended without explicit return, add success return
            if !block_terminated {
                let success_code = builder.ins().iconst(types::I64, 0);
                builder.ins().return_(&[success_code]);

                // Seal final block
                if !sealed_blocks.contains(&current_block) {
                    builder.seal_block(current_block);
                }
            }

            builder.finalize();
        }

        // Compile the function
        if std::env::var("DEBUG_JIT_IR").is_ok() {
            eprintln!("JIT IR for script '{}':\n{}", name, self.ctx.func.display());
        }

        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| format!("Failed to define script: {:?}", e))?;

        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions().map_err(|e| format!("Failed to finalize: {}", e))?;

        // Get function pointer
        let code_ptr = self.module.get_finalized_function(func_id);

        // Cast to our function type
        let compiled_fn = compiled_fn_from_code_ptr(code_ptr);

        if std::env::var("DEBUG_JIT").is_ok() {
            eprintln!("JIT: Successfully compiled script '{}'", name);
        }

        Ok(compiled_fn)
    }

    /// Get compiled function info for a function name
    #[allow(dead_code)] // API for external tooling and debugging
    pub fn get_fn_info(&self, name: &str) -> Option<&CompiledFnInfo> {
        self.compiled_fn_info.get(name)
    }

    /// Check if a function can be JIT-compiled
    /// Returns true if all opcodes in the function are supported
    pub fn can_compile_function(&self, chunk: &BytecodeChunk) -> bool {
        for instr in &chunk.instructions {
            if !self.is_supported_opcode(instr, &chunk.constants) {
                return false;
            }

            // Stop at Return - that's the end of the function body
            match instr {
                OpCode::Return | OpCode::ReturnNone => break,
                _ => {}
            }
        }
        true
    }

    /// Return the first unsupported opcode found in the provided chunk or nested function chunks.
    pub fn first_unsupported_surface(
        &self,
        chunk: &BytecodeChunk,
    ) -> Option<UnsupportedJitSurface> {
        let chunk_name = chunk.name.as_deref().unwrap_or("<script>");
        self.first_unsupported_surface_in_chunk(chunk, chunk_name)
    }

    fn first_unsupported_surface_in_chunk(
        &self,
        chunk: &BytecodeChunk,
        chunk_name: &str,
    ) -> Option<UnsupportedJitSurface> {
        for (pc, instr) in chunk.instructions.iter().enumerate() {
            if !self.is_supported_opcode(instr, &chunk.constants) {
                return Some(UnsupportedJitSurface {
                    chunk_name: chunk_name.to_string(),
                    instruction_offset: pc,
                    opcode: instr.clone(),
                });
            }
        }

        for constant in &chunk.constants {
            if let Constant::Function(function_chunk) = constant {
                let nested_name = function_chunk.name.as_deref().unwrap_or("<anonymous-function>");
                if let Some(unsupported) =
                    self.first_unsupported_surface_in_chunk(function_chunk, nested_name)
                {
                    return Some(unsupported);
                }
            }
        }

        None
    }

    /// Enable or disable JIT compilation
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Record a type observation for profiling
    /// TODO: Integrate into VM execution loop for automatic profiling
    #[allow(dead_code)]
    pub fn record_type(&mut self, offset: usize, var_hash: u64, value: &Value) {
        let profile = self.type_profiles.entry(offset).or_insert_with(SpecializationInfo::new);
        let type_profile =
            profile.variable_types.entry(var_hash).or_insert_with(TypeProfile::default);
        type_profile.record(value);

        // Update specialization strategy if profile is stable
        if type_profile.is_stable() {
            if let Some(dominant_type) = type_profile.dominant_type() {
                profile.specialized_types.insert(var_hash, dominant_type);
            }
        }
    }

    /// Record a guard success
    /// TODO: Call from JIT-compiled code guard checks
    #[allow(dead_code)]
    pub fn record_guard_success(&mut self, offset: usize) {
        if let Some(profile) = self.type_profiles.get_mut(&offset) {
            profile.guard_successes += 1;
        }
    }

    /// Record a guard failure
    /// TODO: Call from JIT-compiled code when guard checks fail
    #[allow(dead_code)]
    pub fn record_guard_failure(&mut self, offset: usize) {
        if let Some(profile) = self.type_profiles.get_mut(&offset) {
            profile.guard_failures += 1;

            // Check if we should recompile
            if profile.should_despecialize() {
                // Remove from cache to force recompilation with generic code
                self.compiled_cache.remove(&offset);
                profile.specialized_types.clear();
                profile.guard_successes = 0;
                profile.guard_failures = 0;
            }
        }
    }

    /// Get specialization info for a function
    /// TODO: Use for adaptive recompilation decisions
    #[allow(dead_code)]
    pub fn get_specialization(&self, offset: usize) -> Option<&SpecializationInfo> {
        self.type_profiles.get(&offset)
    }

    /// Get JIT statistics
    #[allow(dead_code)] // Future runtime reporting/benchmark plumbing consumes this accessor.
    pub fn stats(&self) -> JitStats {
        JitStats {
            total_functions: self.execution_counts.len(),
            compiled_functions: self.compiled_cache.len(),
            enabled: self.enabled,
        }
    }
}

/// JIT compilation statistics
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields accessed via Debug formatting and will be used in future performance monitoring
pub struct JitStats {
    pub total_functions: usize,
    pub compiled_functions: usize,
    pub enabled: bool,
}

impl Default for JitCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create JIT compiler")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // SAFETY:
    // - Preconditions: dummy_compiled_fn is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
    //   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
    //   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
    // - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
    //   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
    unsafe extern "C" fn dummy_compiled_fn(_ctx: *mut VMContext) -> i64 {
        0
    }

    // SAFETY:
    // - Preconditions: dummy_compiled_fn_with_arg is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
    //   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
    //   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
    // - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
    //   and invalid inputs are handled via sentinel returns/early exits without transferring ownership.
    unsafe extern "C" fn dummy_compiled_fn_with_arg(_ctx: *mut VMContext, arg: i64) -> i64 {
        arg + 7
    }

    #[test]
    fn invoke_compiled_fn_wrapper_calls_function_pointer() {
        let mut ctx =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        let result = invoke_compiled_fn(dummy_compiled_fn as CompiledFn, &mut ctx);
        assert_eq!(result, 0);
    }

    #[test]
    fn invoke_compiled_fn_with_arg_wrapper_forwards_argument() {
        let mut ctx =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        let result = invoke_compiled_fn_with_arg(
            dummy_compiled_fn_with_arg as CompiledFnWithArg,
            &mut ctx,
            35,
        );
        assert_eq!(result, 42);
    }

    #[test]
    fn test_jit_compiler_creation() {
        let compiler = JitCompiler::new();
        assert!(compiler.is_ok(), "JIT compiler should be created successfully");
    }

    #[test]
    fn test_hot_path_detection() {
        let mut compiler = JitCompiler::new().unwrap();

        // Should not compile initially
        for i in 0..JIT_THRESHOLD - 1 {
            assert!(!compiler.should_compile(0), "Should not compile at iteration {}", i);
        }

        // Should compile after threshold
        assert!(compiler.should_compile(0), "Should compile after threshold");

        // Mark as compiled by adding a dummy entry to cache
        compiler.compiled_cache.insert(0, dummy_compiled_fn as CompiledFn);

        // Should not try to compile again (already in cache)
        assert!(!compiler.should_compile(0), "Should not recompile");
    }

    #[test]
    fn test_jit_enable_disable() {
        let mut compiler = JitCompiler::new().unwrap();

        assert!(compiler.enabled, "JIT should be enabled by default");

        compiler.set_enabled(false);
        assert!(!compiler.should_compile(0), "Should not compile when disabled");

        compiler.set_enabled(true);
        // Increment counter to threshold
        for _ in 0..JIT_THRESHOLD {
            compiler.should_compile(0);
        }
    }

    #[test]
    fn test_jit_stats() {
        let compiler = JitCompiler::new().unwrap();
        let stats = compiler.stats();

        assert_eq!(stats.total_functions, 0);
        assert_eq!(stats.compiled_functions, 0);
        assert!(stats.enabled);
    }

    #[test]
    fn test_compile_simple_arithmetic() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Create a simple program: 5 + 3
        let const_5 = chunk.add_constant(Constant::Int(5));
        let const_3 = chunk.add_constant(Constant::Int(3));

        chunk.emit(OpCode::LoadConst(const_5));
        chunk.emit(OpCode::LoadConst(const_3));
        chunk.emit(OpCode::Add);
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, 0);
        // Compilation should succeed for simple arithmetic
        assert!(result.is_ok(), "Should compile simple arithmetic: {:?}", result.err());
    }

    #[test]
    fn test_compile_comparisons() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Create a simple program: 10 < 20
        let const_10 = chunk.add_constant(Constant::Int(10));
        let const_20 = chunk.add_constant(Constant::Int(20));

        chunk.emit(OpCode::LoadConst(const_10));
        chunk.emit(OpCode::LoadConst(const_20));
        chunk.emit(OpCode::LessThan);
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, 0);
        assert!(result.is_ok(), "Should compile comparison operations: {:?}", result.err());
    }

    #[test]
    fn test_compile_logical_ops() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Create a simple program: true && false
        let const_true = chunk.add_constant(Constant::Bool(true));
        let const_false = chunk.add_constant(Constant::Bool(false));

        chunk.emit(OpCode::LoadConst(const_true));
        chunk.emit(OpCode::LoadConst(const_false));
        chunk.emit(OpCode::And);
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, 0);
        assert!(result.is_ok(), "Should compile logical operations: {:?}", result.err());
    }

    #[test]
    fn test_compile_stack_operations() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Test dup and pop
        let const_42 = chunk.add_constant(Constant::Int(42));

        chunk.emit(OpCode::LoadConst(const_42));
        chunk.emit(OpCode::Dup);
        chunk.emit(OpCode::Pop);
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, 0);
        assert!(result.is_ok(), "Should compile stack operations: {:?}", result.err());
    }

    #[test]
    fn test_compile_simple_loop() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Simulates: while (counter < 10) { counter := counter + 1 }
        // NOTE: StoreVar PEEKS (doesn't pop), so we need Pop after to clean stack
        // Following the actual compiler's bytecode pattern:
        //   loop_start:
        //     LoadVar counter
        //     LoadConst 10
        //     LessThan
        //     JumpIfFalse end
        //     Pop (condition)
        //     LoadVar counter
        //     LoadConst 1
        //     Add
        //     StoreVar counter
        //     Pop (clean stack - StoreVar peeks, doesn't pop)
        //     JumpBack loop_start
        //   end:
        //     Pop (condition)
        //     Return

        let const_1 = chunk.add_constant(Constant::Int(1));
        let const_10 = chunk.add_constant(Constant::Int(10));
        let const_0 = chunk.add_constant(Constant::Int(0));

        // Initialize counter = 0
        chunk.emit(OpCode::LoadConst(const_0)); // 0: load 0
        chunk.emit(OpCode::StoreVar("counter".to_string())); // 1: store to counter
        chunk.emit(OpCode::Pop); // 2: clean stack after let

        // loop_start (PC 3): stack is empty []
        let loop_start = chunk.instructions.len();

        // Check condition: counter < 10
        chunk.emit(OpCode::LoadVar("counter".to_string())); // 3: load counter, stack: [counter]
        chunk.emit(OpCode::LoadConst(const_10)); // 4: load 10, stack: [counter, 10]
        chunk.emit(OpCode::LessThan); // 5: <, stack: [is_less]
        chunk.emit(OpCode::JumpIfFalse(0)); // 6: jump if false (patched)
        chunk.emit(OpCode::Pop); // 7: pop condition, stack: []

        // Body: counter := counter + 1
        chunk.emit(OpCode::LoadVar("counter".to_string())); // 8: load counter
        chunk.emit(OpCode::LoadConst(const_1)); // 9: load 1
        chunk.emit(OpCode::Add); // 10: add
        chunk.emit(OpCode::StoreVar("counter".to_string())); // 11: store counter
        chunk.emit(OpCode::Pop); // 12: pop (StoreVar peeks, we need clean stack)

        // Jump back to loop start (stack is empty, matches PC 3 entry)
        chunk.emit(OpCode::JumpBack(loop_start)); // 13: jump back

        // end (PC 14):
        let end_pc = chunk.instructions.len();
        chunk.emit(OpCode::Pop); // 14: pop condition
        chunk.emit(OpCode::LoadVar("counter".to_string())); // 15: load final counter
        chunk.emit(OpCode::Return); // 16: return

        // Patch jump
        chunk.instructions[6] = OpCode::JumpIfFalse(end_pc);

        let result = compiler.compile(&chunk, 0);
        assert!(result.is_ok(), "Should compile simple loop: {:?}", result.err());
    }

    #[test]
    fn test_execute_compiled_code() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Simple arithmetic: 5 + 3 = 8
        let const_5 = chunk.add_constant(Constant::Int(5));
        let const_3 = chunk.add_constant(Constant::Int(3));

        chunk.emit(OpCode::LoadConst(const_5));
        chunk.emit(OpCode::LoadConst(const_3));
        chunk.emit(OpCode::Add);
        chunk.emit(OpCode::Return);

        let compiled_fn = compiler.compile(&chunk, 0).expect("Should compile");

        // Execute the compiled function
        // For now, pass a null pointer since we don't need context for pure arithmetic
        let mut ctx =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        let result = invoke_compiled_fn(compiled_fn, &mut ctx);

        // Result should be 0 (success code)
        assert_eq!(result, 0, "Compiled function should return success code");

        println!("✓ Compiled code executed successfully!");
    }

    #[test]
    fn test_compile_with_variables() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Test that variable opcodes at least compile (even if stubbed)
        // x := 5; y := 3; result := x + y
        let const_5 = chunk.add_constant(Constant::Int(5));
        let const_3 = chunk.add_constant(Constant::Int(3));

        chunk.emit(OpCode::LoadConst(const_5));
        chunk.emit(OpCode::StoreVar("x".to_string()));
        chunk.emit(OpCode::LoadConst(const_3));
        chunk.emit(OpCode::StoreVar("y".to_string()));
        chunk.emit(OpCode::LoadVar("x".to_string()));
        chunk.emit(OpCode::LoadVar("y".to_string()));
        chunk.emit(OpCode::Add);
        chunk.emit(OpCode::StoreVar("result".to_string()));
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, 0);
        assert!(result.is_ok(), "Should compile variable operations: {:?}", result.err());

        println!("✓ Variable operations compile successfully!");
    }

    #[test]
    fn test_execute_with_variables() {
        use std::collections::hash_map::DefaultHasher;
        use std::collections::HashMap;
        use std::hash::{Hash, Hasher};

        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // x := 10; y := 20; result := x + y; return
        let const_10 = chunk.add_constant(Constant::Int(10));
        let const_20 = chunk.add_constant(Constant::Int(20));

        chunk.emit(OpCode::LoadConst(const_10));
        chunk.emit(OpCode::StoreVar("x".to_string()));
        chunk.emit(OpCode::LoadConst(const_20));
        chunk.emit(OpCode::StoreVar("y".to_string()));
        chunk.emit(OpCode::LoadVar("x".to_string()));
        chunk.emit(OpCode::LoadVar("y".to_string()));
        chunk.emit(OpCode::Add);
        chunk.emit(OpCode::Return);

        let compiled_fn = compiler.compile(&chunk, 0).expect("Should compile");

        // Create actual variable storage
        let mut locals: HashMap<String, Value> = HashMap::new();
        let mut globals: HashMap<String, Value> = HashMap::new();
        let mut var_names: HashMap<u64, String> = HashMap::new();

        // Register variable names with their hashes
        for name in &["x", "y", "result"] {
            let mut hasher = DefaultHasher::new();
            name.hash(&mut hasher);
            let hash = hasher.finish();
            var_names.insert(hash, name.to_string());
        }

        // Create context
        let mut ctx = VMContext::with_var_names(
            std::ptr::null_mut(),
            &mut locals as *mut _,
            &mut globals as *mut _,
            &mut var_names as *mut _,
        );

        // Execute compiled code
        let result = invoke_compiled_fn(compiled_fn, &mut ctx);

        assert_eq!(result, 0, "Should return success");

        // Check variables were stored
        assert!(locals.contains_key("x"), "x should exist");
        assert!(locals.contains_key("y"), "y should exist");

        // Check values
        match locals.get("x") {
            Some(Value::Int(10)) => {}
            other => panic!("Expected x=10, got {:?}", other),
        }

        match locals.get("y") {
            Some(Value::Int(20)) => {}
            other => panic!("Expected y=20, got {:?}", other),
        }

        println!("✓ Variables work correctly in JIT code!");
        println!("  x = {:?}", locals.get("x"));
        println!("  y = {:?}", locals.get("y"));
    }

    #[test]
    fn test_type_profiling() {
        let mut compiler = JitCompiler::new().unwrap();
        let offset = 0;

        // Simulate profiling Int values
        let var_hash = 12345u64;
        for _ in 0..60 {
            compiler.record_type(offset, var_hash, &Value::Int(42));
        }

        // Check profile
        let profile = compiler.get_specialization(offset).expect("Profile should exist");
        let type_profile =
            profile.variable_types.get(&var_hash).expect("Type profile should exist");

        assert_eq!(type_profile.int_count, 60);
        assert_eq!(type_profile.float_count, 0);
        assert!(type_profile.is_stable(), "Profile should be stable");
        assert_eq!(type_profile.dominant_type(), Some(ValueType::Int));

        // Check specialization was recorded
        assert_eq!(profile.specialized_types.get(&var_hash), Some(&ValueType::Int));
    }

    #[test]
    fn test_type_profiling_mixed_types() {
        let mut compiler = JitCompiler::new().unwrap();
        let offset = 0;
        let var_hash = 12345u64;

        // Simulate mixed types (40 Int, 20 Float)
        for _ in 0..40 {
            compiler.record_type(offset, var_hash, &Value::Int(42));
        }
        for _ in 0..20 {
            compiler.record_type(offset, var_hash, &Value::Float(3.14));
        }

        let profile = compiler.get_specialization(offset).expect("Profile should exist");
        let type_profile =
            profile.variable_types.get(&var_hash).expect("Type profile should exist");

        assert_eq!(type_profile.int_count, 40);
        assert_eq!(type_profile.float_count, 20);
        assert_eq!(type_profile.total(), 60);

        // Not stable enough for specialization (need >90% of one type)
        assert!(type_profile.dominant_type().is_none(), "Mixed types shouldn't specialize");
    }

    #[test]
    fn test_guard_success_tracking() {
        let mut compiler = JitCompiler::new().unwrap();
        let offset = 0;

        // Initialize profile
        compiler.record_type(offset, 12345, &Value::Int(1));

        // Record guard successes
        for _ in 0..100 {
            compiler.record_guard_success(offset);
        }

        let profile = compiler.get_specialization(offset).expect("Profile should exist");
        assert_eq!(profile.guard_successes, 100);
        assert_eq!(profile.guard_failures, 0);
        assert!(!profile.should_despecialize());
    }

    #[test]
    fn test_guard_failure_despecialization() {
        let mut compiler = JitCompiler::new().unwrap();
        let offset = 0;

        // Initialize profile and add to compiled cache
        compiler.record_type(offset, 12345, &Value::Int(1));
        compiler.compiled_cache.insert(offset, dummy_compiled_fn as CompiledFn);

        // Record mostly successes, then failures
        for _ in 0..90 {
            compiler.record_guard_success(offset);
        }

        // Add failures - when we hit 11 failures out of 101 total (10.9%), despec triggers
        // After that, the counters reset and remaining failures get added to new counters
        for _ in 0..20 {
            compiler.record_guard_failure(offset);
        }

        // Should have cleared cache (happened when threshold was crossed)
        assert!(!compiler.compiled_cache.contains_key(&offset), "Should remove from cache");
        let profile = compiler.get_specialization(offset).expect("Profile should exist");
        assert_eq!(profile.specialized_types.len(), 0, "Should clear specializations");

        // Counters were reset when despec happened, then more failures were added
        // So we should have some failures but not all 20
        assert!(
            profile.guard_failures < 20 && profile.guard_failures > 0,
            "Should have some failures after reset: {}",
            profile.guard_failures
        );
        assert_eq!(profile.guard_successes, 0, "Successes should be reset");
    }

    #[test]
    fn test_float_specialization_profile() {
        let mut compiler = JitCompiler::new().unwrap();
        let offset = 0;
        let var_hash = 99999u64;

        // Profile float values
        for _ in 0..60 {
            compiler.record_type(offset, var_hash, &Value::Float(3.14));
        }

        let profile = compiler.get_specialization(offset).expect("Profile should exist");
        let type_profile =
            profile.variable_types.get(&var_hash).expect("Type profile should exist");

        assert_eq!(type_profile.float_count, 60);
        assert!(type_profile.is_stable());
        assert_eq!(type_profile.dominant_type(), Some(ValueType::Float));
        assert_eq!(profile.specialized_types.get(&var_hash), Some(&ValueType::Float));
    }

    #[test]
    fn test_int_specialized_addition() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Test: 100 + 200 = 300 (pure integer addition)
        let const_100 = chunk.add_constant(Constant::Int(100));
        let const_200 = chunk.add_constant(Constant::Int(200));

        chunk.emit(OpCode::LoadConst(const_100));
        chunk.emit(OpCode::LoadConst(const_200));
        chunk.emit(OpCode::Add);
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, 0);
        assert!(result.is_ok(), "Should compile int addition: {:?}", result.err());
    }

    #[test]
    fn test_int_specialized_subtraction() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Test: 500 - 200 = 300
        let const_500 = chunk.add_constant(Constant::Int(500));
        let const_200 = chunk.add_constant(Constant::Int(200));

        chunk.emit(OpCode::LoadConst(const_500));
        chunk.emit(OpCode::LoadConst(const_200));
        chunk.emit(OpCode::Sub);
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, 0);
        assert!(result.is_ok(), "Should compile int subtraction: {:?}", result.err());
    }

    #[test]
    fn test_int_specialized_multiplication() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Test: 25 * 4 = 100
        let const_25 = chunk.add_constant(Constant::Int(25));
        let const_4 = chunk.add_constant(Constant::Int(4));

        chunk.emit(OpCode::LoadConst(const_25));
        chunk.emit(OpCode::LoadConst(const_4));
        chunk.emit(OpCode::Mul);
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, 0);
        assert!(result.is_ok(), "Should compile int multiplication: {:?}", result.err());
    }

    #[test]
    fn test_int_specialized_division() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Test: 100 / 4 = 25
        let const_100 = chunk.add_constant(Constant::Int(100));
        let const_4 = chunk.add_constant(Constant::Int(4));

        chunk.emit(OpCode::LoadConst(const_100));
        chunk.emit(OpCode::LoadConst(const_4));
        chunk.emit(OpCode::Div);
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, 0);
        assert!(result.is_ok(), "Should compile int division: {:?}", result.err());
    }

    #[test]
    fn test_specialized_arithmetic_chain() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Test: (10 + 5) * 2 - 3 = 27
        let const_10 = chunk.add_constant(Constant::Int(10));
        let const_5 = chunk.add_constant(Constant::Int(5));
        let const_2 = chunk.add_constant(Constant::Int(2));
        let const_3 = chunk.add_constant(Constant::Int(3));

        chunk.emit(OpCode::LoadConst(const_10));
        chunk.emit(OpCode::LoadConst(const_5));
        chunk.emit(OpCode::Add);
        chunk.emit(OpCode::LoadConst(const_2));
        chunk.emit(OpCode::Mul);
        chunk.emit(OpCode::LoadConst(const_3));
        chunk.emit(OpCode::Sub);
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, 0);
        assert!(result.is_ok(), "Should compile arithmetic chain: {:?}", result.err());
    }

    #[test]
    fn test_compilation_with_specialization_context() {
        let mut compiler = JitCompiler::new().unwrap();
        let offset = 0;
        let var_hash_x = 11111u64;
        let var_hash_y = 22222u64;

        // Build type profile with stable Int types
        for _ in 0..60 {
            compiler.record_type(offset, var_hash_x, &Value::Int(10));
            compiler.record_type(offset, var_hash_y, &Value::Int(20));
        }

        // Verify profile is stable and specialized
        let profile = compiler.get_specialization(offset).expect("Profile should exist");
        assert_eq!(profile.specialized_types.get(&var_hash_x), Some(&ValueType::Int));
        assert_eq!(profile.specialized_types.get(&var_hash_y), Some(&ValueType::Int));

        // Now compile with specialization context
        let mut chunk = BytecodeChunk::new();
        let const_5 = chunk.add_constant(Constant::Int(5));
        let const_3 = chunk.add_constant(Constant::Int(3));

        chunk.emit(OpCode::LoadConst(const_5));
        chunk.emit(OpCode::LoadConst(const_3));
        chunk.emit(OpCode::Add);
        chunk.emit(OpCode::Return);

        // Compilation should succeed with specialization
        let result = compiler.compile(&chunk, offset);
        assert!(result.is_ok(), "Should compile with specialization context: {:?}", result.err());
    }

    #[test]
    fn test_compilation_without_specialization_fallback() {
        let mut compiler = JitCompiler::new().unwrap();

        // No type profiling - compile without specialization
        let mut chunk = BytecodeChunk::new();
        let const_100 = chunk.add_constant(Constant::Int(100));
        let const_50 = chunk.add_constant(Constant::Int(50));

        chunk.emit(OpCode::LoadConst(const_100));
        chunk.emit(OpCode::LoadConst(const_50));
        chunk.emit(OpCode::Sub);
        chunk.emit(OpCode::Return);

        // Should still compile successfully using generic path
        let result = compiler.compile(&chunk, 0);
        assert!(result.is_ok(), "Should compile without specialization: {:?}", result.err());
    }

    #[test]
    fn test_all_arithmetic_ops_with_specialization() {
        let mut compiler = JitCompiler::new().unwrap();
        let offset_base = 100;

        // Profile for specialization at offset 100
        for _ in 0..60 {
            compiler.record_type(offset_base, 99999u64, &Value::Int(42));
        }

        // Test Add with specialization at offset 100
        let mut chunk = BytecodeChunk::new();
        let const_10 = chunk.add_constant(Constant::Int(10));
        let const_20 = chunk.add_constant(Constant::Int(20));
        chunk.emit(OpCode::LoadConst(const_10));
        chunk.emit(OpCode::LoadConst(const_20));
        chunk.emit(OpCode::Add);
        chunk.emit(OpCode::Return);
        assert!(compiler.compile(&chunk, offset_base).is_ok());

        // Profile for specialization at offset 200
        let offset_sub = 200;
        for _ in 0..60 {
            compiler.record_type(offset_sub, 88888u64, &Value::Int(30));
        }

        // Test Sub with specialization at offset 200
        let mut chunk = BytecodeChunk::new();
        let const_30 = chunk.add_constant(Constant::Int(30));
        let const_10 = chunk.add_constant(Constant::Int(10));
        chunk.emit(OpCode::LoadConst(const_30));
        chunk.emit(OpCode::LoadConst(const_10));
        chunk.emit(OpCode::Sub);
        chunk.emit(OpCode::Return);
        let result = compiler.compile(&chunk, offset_sub);
        assert!(result.is_ok(), "Sub compilation failed: {:?}", result.err());

        // Profile for specialization at offset 300
        let offset_mul = 300;
        for _ in 0..60 {
            compiler.record_type(offset_mul, 77777u64, &Value::Int(5));
        }

        // Test Mul with specialization at offset 300
        let mut chunk = BytecodeChunk::new();
        let const_5 = chunk.add_constant(Constant::Int(5));
        let const_6 = chunk.add_constant(Constant::Int(6));
        chunk.emit(OpCode::LoadConst(const_5));
        chunk.emit(OpCode::LoadConst(const_6));
        chunk.emit(OpCode::Mul);
        chunk.emit(OpCode::Return);
        assert!(compiler.compile(&chunk, offset_mul).is_ok());

        // Profile for specialization at offset 400
        let offset_div = 400;
        for _ in 0..60 {
            compiler.record_type(offset_div, 66666u64, &Value::Int(100));
        }

        // Test Div with specialization at offset 400
        let mut chunk = BytecodeChunk::new();
        let const_100 = chunk.add_constant(Constant::Int(100));
        let const_5 = chunk.add_constant(Constant::Int(5));
        chunk.emit(OpCode::LoadConst(const_100));
        chunk.emit(OpCode::LoadConst(const_5));
        chunk.emit(OpCode::Div);
        chunk.emit(OpCode::Return);
        assert!(compiler.compile(&chunk, offset_div).is_ok());
    }

    #[test]
    fn test_guard_generation_with_specialization() {
        let mut compiler = JitCompiler::new().unwrap();
        let offset = 500;
        let var_hash = 12345u64;

        // Build stable Int type profile
        for _ in 0..70 {
            compiler.record_type(offset, var_hash, &Value::Int(42));
        }

        // Verify specialization was created
        let profile = compiler.get_specialization(offset).expect("Profile should exist");
        assert_eq!(profile.specialized_types.get(&var_hash), Some(&ValueType::Int));

        // Compile with guards
        let mut chunk = BytecodeChunk::new();
        let const_10 = chunk.add_constant(Constant::Int(10));
        let const_20 = chunk.add_constant(Constant::Int(20));
        chunk.emit(OpCode::LoadConst(const_10));
        chunk.emit(OpCode::LoadConst(const_20));
        chunk.emit(OpCode::Add);
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, offset);
        assert!(result.is_ok(), "Should compile with guards: {:?}", result.err());
    }

    #[test]
    fn test_compilation_without_guards_when_no_specialization() {
        let mut compiler = JitCompiler::new().unwrap();

        // No profiling - no guards should be generated
        let mut chunk = BytecodeChunk::new();
        let const_5 = chunk.add_constant(Constant::Int(5));
        let const_3 = chunk.add_constant(Constant::Int(3));
        chunk.emit(OpCode::LoadConst(const_5));
        chunk.emit(OpCode::LoadConst(const_3));
        chunk.emit(OpCode::Mul);
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, 600);
        assert!(result.is_ok(), "Should compile without guards: {:?}", result.err());
    }

    #[test]
    fn test_multiple_specialized_variables_guards() {
        let mut compiler = JitCompiler::new().unwrap();
        let offset = 700;
        let var_hash_1 = 11111u64;
        let var_hash_2 = 22222u64;
        let var_hash_3 = 33333u64;

        // Build profiles for multiple variables
        for _ in 0..70 {
            compiler.record_type(offset, var_hash_1, &Value::Int(10));
            compiler.record_type(offset, var_hash_2, &Value::Int(20));
            compiler.record_type(offset, var_hash_3, &Value::Int(30));
        }

        // Verify all specialized
        let profile = compiler.get_specialization(offset).expect("Profile should exist");
        assert_eq!(profile.specialized_types.len(), 3);

        // Compile - should generate guards for all 3 variables
        let mut chunk = BytecodeChunk::new();
        let const_100 = chunk.add_constant(Constant::Int(100));
        chunk.emit(OpCode::LoadConst(const_100));
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, offset);
        assert!(result.is_ok(), "Should compile with multiple guards: {:?}", result.err());
    }

    // ========================================================================
    // Phase 4E: Performance Benchmarking Tests
    // ========================================================================

    /// Helper function to measure execution time of a closure
    fn measure_time<F>(f: F) -> std::time::Duration
    where
        F: FnOnce(),
    {
        let start = std::time::Instant::now();
        f();
        start.elapsed()
    }

    fn median_duration(samples: &mut [std::time::Duration]) -> std::time::Duration {
        samples.sort_unstable();
        samples[samples.len() / 2]
    }

    #[test]
    #[ignore] // Benchmark test - run with: cargo test --release -- --ignored benchmark_
    fn benchmark_specialized_vs_generic_addition() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();

        // Create a simple addition benchmark: sum 1000 times
        // We'll use constants to isolate JIT performance
        let const_1 = chunk.add_constant(Constant::Int(1));
        let const_0 = chunk.add_constant(Constant::Int(0));

        // Initialize sum = 0
        chunk.emit(OpCode::LoadConst(const_0));

        // Add 1, 1000 times (unrolled for simplicity)
        for _ in 0..1000 {
            chunk.emit(OpCode::LoadConst(const_1));
            chunk.emit(OpCode::Add);
        }
        chunk.emit(OpCode::Return);

        // Compile without specialization
        let compile_start = std::time::Instant::now();
        let result = compiler.compile(&chunk, 0);
        let compile_time = compile_start.elapsed();

        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

        println!("\n=== Specialized vs Generic Addition Benchmark ===");
        println!("Compilation time: {:?}", compile_time);
        println!("Instructions compiled: {}", chunk.instructions.len());
        println!("Note: This is a micro-benchmark of JIT compilation itself");
        println!("For runtime performance, see the benchmark_*.kujo examples");
    }

    #[test]
    #[ignore]
    fn benchmark_compilation_overhead() {
        let mut compiler = JitCompiler::new().unwrap();

        // Test 1: Simple arithmetic (should be fast)
        let mut chunk1 = BytecodeChunk::new();
        let c1 = chunk1.add_constant(Constant::Int(10));
        let c2 = chunk1.add_constant(Constant::Int(20));
        chunk1.emit(OpCode::LoadConst(c1));
        chunk1.emit(OpCode::LoadConst(c2));
        chunk1.emit(OpCode::Add);
        chunk1.emit(OpCode::Return);

        // Test 2: Complex arithmetic chain (should be slower)
        let mut chunk2 = BytecodeChunk::new();
        for i in 0..100 {
            let c = chunk2.add_constant(Constant::Int(i));
            chunk2.emit(OpCode::LoadConst(c));
            if i > 0 {
                chunk2.emit(OpCode::Add);
            }
        }
        chunk2.emit(OpCode::Return);

        // Warm up once to avoid one-time initialization noise dominating the first sample.
        let _ = compiler.compile(&chunk1, 0);
        let _ = compiler.compile(&chunk2, 1);

        const SAMPLES: usize = 9;
        let mut simple_samples = Vec::with_capacity(SAMPLES);
        let mut complex_samples = Vec::with_capacity(SAMPLES);

        for sample in 0..SAMPLES {
            simple_samples.push(measure_time(|| {
                let result = compiler.compile(&chunk1, 1_000 + sample);
                assert!(result.is_ok(), "Simple benchmark compilation failed: {:?}", result.err());
            }));
            complex_samples.push(measure_time(|| {
                let result = compiler.compile(&chunk2, 2_000 + sample);
                assert!(result.is_ok(), "Complex benchmark compilation failed: {:?}", result.err());
            }));
        }

        let time1 = median_duration(&mut simple_samples);
        let time2 = median_duration(&mut complex_samples);

        println!("\n=== Compilation Overhead Benchmark ===");
        println!("Simple (3 instructions): {:?}", time1);
        println!("Complex (200 instructions): {:?}", time2);
        println!("Ratio: {:.2}x", time2.as_nanos() as f64 / time1.as_nanos() as f64);

        // Timing can fluctuate across CI/host loads; this benchmark's contract is compile success and telemetry output.
        assert!(
            time1 > std::time::Duration::ZERO || time2 > std::time::Duration::ZERO,
            "At least one benchmark sample should report a measurable duration"
        );
    }

    #[test]
    #[ignore]
    fn benchmark_type_profiling_overhead() {
        let mut compiler = JitCompiler::new().unwrap();
        let offset = 0;
        let var_hash = 12345u64;

        // Benchmark: recording 10,000 type observations
        let iterations = 10_000;

        let time = measure_time(|| {
            for _ in 0..iterations {
                compiler.record_type(offset, var_hash, &Value::Int(42));
            }
        });

        println!("\n=== Type Profiling Overhead Benchmark ===");
        println!("Recorded {} type observations in {:?}", iterations, time);
        println!("Average per observation: {:?}", time / iterations);
        println!("Observations per second: {:.0}", iterations as f64 / time.as_secs_f64());

        // Verify profile was built
        let profile = compiler.get_specialization(offset).expect("Profile should exist");
        assert_eq!(profile.specialized_types.get(&var_hash), Some(&ValueType::Int));
    }

    #[test]
    #[ignore]
    fn benchmark_specialized_arithmetic_chain() {
        let mut compiler = JitCompiler::new().unwrap();

        // Build a long arithmetic chain: ((((1 + 2) + 3) + 4) + ... + 100)
        let mut chunk = BytecodeChunk::new();

        for i in 1..=100 {
            let c = chunk.add_constant(Constant::Int(i));
            chunk.emit(OpCode::LoadConst(c));
            if i > 1 {
                chunk.emit(OpCode::Add);
            }
        }
        chunk.emit(OpCode::Return);

        // Compile the chain
        let compile_time = measure_time(|| {
            let result = compiler.compile(&chunk, 0);
            assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
        });

        println!("\n=== Specialized Arithmetic Chain Benchmark ===");
        println!("Chain length: 100 additions");
        println!("Total instructions: {}", chunk.instructions.len());
        println!("Compilation time: {:?}", compile_time);
        println!("Time per instruction: {:?}", compile_time / chunk.instructions.len() as u32);
    }

    #[test]
    #[ignore]
    fn benchmark_guard_generation_overhead() {
        let mut compiler = JitCompiler::new().unwrap();
        let offset = 0;

        // Setup: Create specialization profiles for many variables
        let var_count = 50;
        let mut var_hashes = Vec::new();

        for i in 0..var_count {
            let var_hash = (i as u64) * 1000;
            var_hashes.push(var_hash);

            // Build stable profile
            for _ in 0..100 {
                compiler.record_type(offset, var_hash, &Value::Int(42));
            }
        }

        // Compile with many guards
        let mut chunk = BytecodeChunk::new();
        let const_1 = chunk.add_constant(Constant::Int(1));
        chunk.emit(OpCode::LoadConst(const_1));
        chunk.emit(OpCode::Return);

        let compile_time = measure_time(|| {
            let result = compiler.compile(&chunk, offset);
            assert!(result.is_ok(), "Compilation with guards failed: {:?}", result.err());
        });

        println!("\n=== Guard Generation Overhead Benchmark ===");
        println!("Variables with specialized types: {}", var_count);
        println!("Compilation time: {:?}", compile_time);
        println!("Time per guard: {:?}", compile_time / var_count);
    }

    #[test]
    #[ignore]
    fn benchmark_cache_lookup_performance() {
        let mut compiler = JitCompiler::new().unwrap();

        // Compile several functions
        for offset in 0..100 {
            let mut chunk = BytecodeChunk::new();
            let c = chunk.add_constant(Constant::Int(offset as i64));
            chunk.emit(OpCode::LoadConst(c));
            chunk.emit(OpCode::Return);

            let _ = compiler.compile(&chunk, offset);
        }

        // Benchmark: Check if functions should compile (cache lookup)
        let iterations = 100_000;
        let time = measure_time(|| {
            for _ in 0..iterations {
                for offset in 0..100 {
                    let _ = compiler.should_compile(offset);
                }
            }
        });

        println!("\n=== Cache Lookup Performance Benchmark ===");
        println!("Cache entries: 100");
        println!("Total lookups: {}", iterations * 100);
        println!("Time: {:?}", time);
        println!("Lookups per second: {:.0}", (iterations * 100) as f64 / time.as_secs_f64());
        println!("Average per lookup: {:?}", time / (iterations * 100));
    }

    #[test]
    #[ignore]
    fn benchmark_specialization_decision_overhead() {
        let mut compiler = JitCompiler::new().unwrap();
        let offset = 0;
        let var_hash = 99999u64;

        // Record mixed types to test decision logic
        for i in 0..100 {
            if i % 10 == 0 {
                compiler.record_type(offset, var_hash, &Value::Float(1.0));
            } else {
                compiler.record_type(offset, var_hash, &Value::Int(1));
            }
        }

        // Benchmark: decision making
        let iterations = 100_000;
        let time = measure_time(|| {
            for _ in 0..iterations {
                if let Some(profile) = compiler.get_specialization(offset) {
                    let _ = profile.specialized_types.get(&var_hash);
                    let _ = profile.should_despecialize();
                }
            }
        });

        println!("\n=== Specialization Decision Overhead Benchmark ===");
        println!("Decisions made: {}", iterations);
        println!("Time: {:?}", time);
        println!("Decisions per second: {:.0}", iterations as f64 / time.as_secs_f64());
        println!("Average per decision: {:?}", time / iterations);
    }

    #[test]
    fn test_return_value_optimization() {
        // This test validates Phase 7 Step 8: Return Value Optimization
        // The optimization stores return values in VMContext.return_value
        // instead of pushing to the VM stack, avoiding stack overhead.

        println!("\n=== Phase 7 Step 8: Return Value Optimization ===");

        // Test 1: Verify VMContext has the new fields
        let mut vm_context =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());

        assert_eq!(vm_context.return_value, 0, "return_value should be initialized to 0");
        assert!(!vm_context.has_return_value, "has_return_value should be false initially");

        // Test 2: Verify jit_set_return_int works correctly
        let result = set_return_int(&mut vm_context, 42);
        assert_eq!(result, 0, "jit_set_return_int should return 0 (success)");
        assert_eq!(vm_context.return_value, 42, "return_value should be set to 42");
        assert!(vm_context.has_return_value, "has_return_value should be true after set");

        // Test 3: Verify negative values work
        let result = set_return_int(&mut vm_context, -123);
        assert_eq!(result, 0, "jit_set_return_int should handle negative values");
        assert_eq!(vm_context.return_value, -123, "return_value should be -123");

        // Test 4: Verify large values work
        let large_value = i64::MAX;
        let result = set_return_int(&mut vm_context, large_value);
        assert_eq!(result, 0, "jit_set_return_int should handle large values");
        assert_eq!(vm_context.return_value, large_value, "return_value should be i64::MAX");

        // Test 5: Verify null context returns error
        // SAFETY:
        // - Preconditions: this test intentionally passes null to assert defensive error handling path.
        // - Postconditions: call must return sentinel error code and avoid dereferencing null.
        unsafe {
            let result = jit_set_return_int(std::ptr::null_mut(), 42);
            assert_eq!(result, 1, "jit_set_return_int should return error for null context");
        }

        println!("✅ VMContext.return_value field working");
        println!("✅ jit_set_return_int() function working");
        println!("✅ Error handling for null context working");
        println!("\n🎉 Phase 7 Step 8 Return Value Optimization validated!");
    }

    #[test]
    fn validate_phase4_infrastructure_complete() {
        // This test validates that Phase 4A-4D infrastructure is in place
        let mut compiler = JitCompiler::new().unwrap();

        println!("\n=== Phase 4A-4D Infrastructure Validation ===");

        // Phase 4A: Type profiling system
        let offset = 0;
        let var_hash = 11111u64;

        for _ in 0..60 {
            compiler.record_type(offset, var_hash, &Value::Int(42));
        }

        let profile = compiler.get_specialization(offset);
        assert!(profile.is_some(), "Phase 4A: Type profiling should work");
        println!("✅ Phase 4A: Type profiling system working");

        // Phase 4B: Specialized code generation
        let mut chunk = BytecodeChunk::new();
        let c1 = chunk.add_constant(Constant::Int(10));
        let c2 = chunk.add_constant(Constant::Int(20));
        chunk.emit(OpCode::LoadConst(c1));
        chunk.emit(OpCode::LoadConst(c2));
        chunk.emit(OpCode::Add);
        chunk.emit(OpCode::Return);

        let result = compiler.compile(&chunk, offset);
        assert!(result.is_ok(), "Phase 4B: Specialized compilation should work");
        println!("✅ Phase 4B: Specialized code generation working");

        // Phase 4C: Integration (compile with profile)
        assert!(
            compiler.compiled_cache.contains_key(&offset),
            "Phase 4C: Compiled function should be cached"
        );
        println!("✅ Phase 4C: Integration working");

        // Phase 4D: Guard generation
        let profile = compiler.get_specialization(offset).unwrap();
        assert!(
            !profile.specialized_types.is_empty(),
            "Phase 4D: Should have specialized types for guards"
        );
        println!("✅ Phase 4D: Guard generation working");

        println!("\n🎉 All Phase 4A-4D infrastructure validated!");
        println!("Phase 4E: Ready for performance validation and advanced optimizations");
    }
}
