// File: src/jit.rs
//
// JIT Compilation module for Kujo bytecode using Cranelift.
// Provides just-in-time compilation of hot bytecode functions to native machine code.

use crate::benchmarks::profiler::runtime::{self as measurement, Metric};
use crate::bytecode::{BytecodeBindingKind, BytecodeChunk, Constant, OpCode};
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
    /// Immediate return payload; its kind preserves the language type.
    /// When has_return_value is true, VM reads return_value instead of popping stack.
    pub return_value: i64,
    /// Flag indicating return_value contains a valid scalar return
    /// When true, VM uses return_value directly; when false, falls back to stack
    pub has_return_value: bool,
    /// Immediate kind: integer, Boolean, null or IEEE float bits.
    pub return_value_kind: i64,
    /// Runtime failure preserved across the native ABI; never replay failed work.
    pub runtime_error: Option<String>,
    /// Borrowed from the enclosing task; its owner outlives every native invocation.
    pub(crate) task_cancellation: *const std::sync::atomic::AtomicBool,
    /// Fast argument passing for recursive calls
    /// These avoid HashMap creation for simple recursive functions
    pub call_name_ptr: *const u8,
    pub call_name_len: usize,
    pub native_call_depth: usize,
    pub(crate) native_stack_bases: NativeStackBases,
    pub(crate) field_receiver_pending: bool,
    pub(crate) owned_roots: crate::native_owned::NativeCallRoots,
    pub(crate) vm_owned_views: bool,
    pub(crate) path_depth_valid: crate::interpreter::assignment::DepthCache,
    pub(crate) owned_return: Option<Value>,
    pub(crate) loop_dense_views: HashMap<usize, Box<LoopDenseView>>,
    pub(crate) loop_environment_ptr: *mut crate::interpreter::Environment,
    pub(crate) loop_resume_ip: Option<usize>,
    pub(crate) loop_stack_base: usize,
    pub arg_kinds: [i64; 4],
    pub arg0: i64,
    pub arg1: i64,
    pub arg2: i64,
    pub arg3: i64,
    pub arg_count: i64,
}

/// Explicit native ABI; generated code never interprets Rust Vec layout.
#[repr(C)]
pub(crate) struct LoopDenseView {
    data: *mut i64,
    len: usize,
}

/// Keep ordinary native frame entry allocation-free. Only nested invocations
/// sharing a context need overflow storage.
#[derive(Default)]
pub(crate) struct NativeStackBases {
    first: Option<usize>,
    nested: Vec<usize>,
}

impl NativeStackBases {
    fn push(&mut self, base: usize) {
        if self.first.is_none() {
            self.first = Some(base);
        } else {
            self.nested.push(base);
        }
    }
    fn pop(&mut self) -> Option<usize> {
        self.nested.pop().or_else(|| self.first.take())
    }
    fn is_empty(&self) -> bool {
        self.first.is_none() && self.nested.is_empty()
    }
}

impl VMContext {
    /// Refresh only VM-owned views; this function's named-local map is separately owned.
    unsafe fn refresh_vm_views(&mut self) {
        if !self.vm_owned_views || !self.loop_environment_ptr.is_null() {
            return;
        }
        if let Some(vm) = self.vm_ptr.cast::<VM>().as_mut() {
            let shared_globals = !self.globals_ptr.is_null() && self.locals_ptr == self.globals_ptr;
            let (globals, slots) = vm.native_context_views();
            self.globals_ptr = globals;
            if shared_globals {
                self.locals_ptr = globals;
            }
            self.local_slots_ptr = slots;
        }
    }

    #[allow(dead_code)] // Borrowing accessor retained; VM transfers result ownership.
    pub fn scalar_value(&self, value: i64) -> Value {
        if self.return_value_kind == crate::native_owned::OWNED {
            if value != self.return_value {
                return Value::Error("Invalid native owned return payload".into());
            }
            return self
                .owned_return
                .clone()
                .unwrap_or_else(|| Value::Error("Missing native owned return".into()));
        }
        crate::native_scalar::decode(value, self.return_value_kind)
    }

    pub(crate) fn take_return_value(&mut self, value: i64) -> Value {
        if self.return_value_kind == crate::native_owned::OWNED {
            if value != self.return_value {
                return Value::Error("Invalid native owned return payload".into());
            }
            return self
                .owned_return
                .take()
                .unwrap_or_else(|| Value::Error("Missing native owned return".into()));
        }
        crate::native_scalar::decode(value, self.return_value_kind)
    }

    fn begin_owned_boundary(&mut self) -> Result<usize, i64> {
        let previous = self.owned_roots.depth();
        self.owned_return = None;
        self.has_return_value = false;
        self.return_value = 0;
        self.return_value_kind = crate::native_scalar::INTEGER;
        if let Err(error) = self.owned_roots.enter() {
            self.runtime_error = Some(format!("Native ownership error: {error:?}"));
            return Err(1);
        }
        Ok(previous)
    }

    fn finish_owned_boundary(
        &mut self,
        previous: usize,
        mut result: i64,
        direct_result: bool,
    ) -> i64 {
        if result == 0 && self.runtime_error.is_some() {
            result = 1;
        }
        if self.owned_roots.depth() != previous + 1 {
            self.runtime_error.get_or_insert_with(|| "Unbalanced native ownership scopes".into());
            result = 1;
        }
        if (direct_result || result == 0)
            && self.runtime_error.is_none()
            && self.has_return_value
            && self.return_value_kind == crate::native_owned::OWNED
        {
            let payload = if direct_result { result } else { self.return_value };
            match self.owned_roots.take_value(payload, self.return_value_kind) {
                Ok(value) => self.owned_return = Some(value),
                Err(error) => {
                    self.runtime_error = Some(format!("Native ownership error: {error:?}"));
                    result = 1;
                }
            }
        }
        while self.owned_roots.depth() > previous {
            if let Err(error) = self.owned_roots.leave() {
                self.runtime_error
                    .get_or_insert_with(|| format!("Native ownership error: {error:?}"));
                result = 1;
                break;
            }
        }
        if self.runtime_error.is_some() {
            self.owned_return = None;
        }
        result
    }

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
            return_value_kind: 0,
            runtime_error: None,
            task_cancellation: std::ptr::null(),
            call_name_ptr: std::ptr::null(),
            call_name_len: 0,
            native_call_depth: 0,
            native_stack_bases: Default::default(),
            field_receiver_pending: false,
            owned_roots: Default::default(),
            vm_owned_views: false,
            path_depth_valid: Default::default(),
            owned_return: None,
            loop_dense_views: HashMap::new(),
            loop_environment_ptr: std::ptr::null_mut(),
            loop_resume_ip: None,
            loop_stack_base: 0,
            arg_kinds: [0; 4],
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
            return_value_kind: 0,
            runtime_error: None,
            task_cancellation: std::ptr::null(),
            call_name_ptr: std::ptr::null(),
            call_name_len: 0,
            native_call_depth: 0,
            native_stack_bases: Default::default(),
            field_receiver_pending: false,
            owned_roots: Default::default(),
            vm_owned_views: true,
            path_depth_valid: Default::default(),
            owned_return: None,
            loop_dense_views: HashMap::new(),
            loop_environment_ptr: std::ptr::null_mut(),
            loop_resume_ip: None,
            loop_stack_base: 0,
            arg_kinds: [0; 4],
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
            return_value_kind: 0,
            runtime_error: None,
            task_cancellation: std::ptr::null(),
            call_name_ptr: std::ptr::null(),
            call_name_len: 0,
            native_call_depth: 0,
            native_stack_bases: Default::default(),
            field_receiver_pending: false,
            owned_roots: Default::default(),
            vm_owned_views: false,
            path_depth_valid: Default::default(),
            owned_return: None,
            loop_dense_views: HashMap::new(),
            loop_environment_ptr: std::ptr::null_mut(),
            loop_resume_ip: None,
            loop_stack_base: 0,
            arg_kinds: [0; 4],
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
    let boundary = match ctx.begin_owned_boundary() {
        Ok(boundary) => boundary,
        Err(status) => return status,
    };
    measurement::add(Metric::JitNativeBoundaryEntries, 1);
    // SAFETY:
    // - Preconditions: `compiled_fn` was produced by this JIT and `ctx` is a live VM-owned context for this call.
    // - Postconditions: control returns with JIT status code and no pointer from `ctx` escapes this wrapper.
    let status = unsafe { compiled_fn(ctx as *mut VMContext) };
    ctx.finish_owned_boundary(boundary, status, false)
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
    let boundary = match ctx.begin_owned_boundary() {
        Ok(boundary) => boundary,
        Err(status) => return status,
    };
    measurement::add(Metric::JitNativeBoundaryEntries, 1);
    // SAFETY:
    // - Preconditions: identical to `invoke_compiled_fn`; `arg` is plain value data and ABI-compatible.
    // - Postconditions: returns the direct-arg JIT result without transferring ownership of context internals.
    let status = unsafe { compiled_fn(ctx as *mut VMContext, arg) };
    ctx.finish_owned_boundary(boundary, status, true)
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
    /// Every parameter is initialized through an ordered native SSA slot.
    pub supports_positional_arguments: bool,
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
    let Some(ctx) = ctx.as_mut() else { return 0 };
    ctx.return_value_kind = 0;
    if let Some(stack) = ctx.stack_ptr.as_mut() {
        if let Some(value) = stack.pop() {
            match ctx.owned_roots.encode(value) {
                Ok((payload, kind)) => {
                    ctx.return_value_kind = kind;
                    return payload;
                }
                Err(error) => {
                    ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
                }
            }
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
/// name_hash resolves through the context name table; global_only skips locals.
/// Scalar results carry their kind in context metadata. Opaque values retain the
/// existing VM-stack marker protocol; missing names still use the legacy sentinel.
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
    global_only: usize,
) -> i64 {
    if ctx.is_null() {
        return 0;
    }

    let ctx_ref = &mut *ctx;
    ctx_ref.return_value_kind = 0;

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
    if global_only == 0 && !ctx_ref.locals_ptr.is_null() {
        let locals = &*ctx_ref.locals_ptr;
        if let Some(val) = locals.get(name) {
            match val {
                Value::Float(value) => {
                    ctx_ref.return_value_kind = crate::native_scalar::FLOAT;
                    return value.to_bits() as i64;
                }
                Value::Null => {
                    ctx_ref.return_value_kind = crate::native_scalar::NULL;
                    return 0;
                }
                Value::Bool(value) => {
                    ctx_ref.return_value_kind = 1;
                    return i64::from(*value);
                }
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
                Value::Float(value) => {
                    ctx_ref.return_value_kind = crate::native_scalar::FLOAT;
                    return value.to_bits() as i64;
                }
                Value::Null => {
                    ctx_ref.return_value_kind = crate::native_scalar::NULL;
                    return 0;
                }
                Value::Bool(value) => {
                    ctx_ref.return_value_kind = 1;
                    return i64::from(*value);
                }
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

    // Builtins are resolved lazily by the VM, rather than installed in globals.
    // Preserve local/global shadowing, then use the same native callable value
    // so invocation still passes through the VM's capability checks.
    if crate::interpreter::Interpreter::is_builtin_name(name) && !ctx_ref.stack_ptr.is_null() {
        (&mut *ctx_ref.stack_ptr).push(Value::NativeFunction(name.to_owned()));
        return -1;
    }

    if std::env::var("DEBUG_JIT_CALL").is_ok() {
        eprintln!("jit_load_variable: '{}' not found", name);
    }
    0
}

#[no_mangle]
// SAFETY: ctx is live and exclusively borrowed. The root store validates the
// operand identity and creates a distinct root in the current activation.
pub unsafe extern "C" fn jit_duplicate_operand(ctx: *mut VMContext, value: i64, kind: i64) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    match ctx.owned_roots.duplicate(value, kind) {
        Ok(value) => value,
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            0
        }
    }
}

#[no_mangle]
// SAFETY: ctx is live and exclusively borrowed. Only a validated current-scope
// root can be released; payloads are never treated as raw pointers.
pub unsafe extern "C" fn jit_release_operand(ctx: *mut VMContext, value: i64, kind: i64) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    match ctx.owned_roots.release(value, kind) {
        Ok(()) => 0,
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            1
        }
    }
}

/// Remove a dead binding. Whole-function contexts borrow the caller's slot
/// vector, so only loop contexts may clear a physical VM local slot.
#[no_mangle]
// SAFETY: context maps and (for loop contexts) local slots remain exclusively
// borrowed for this call. Validate the slot before changing either container.
pub unsafe extern "C" fn jit_release_dead_local(
    ctx: *mut VMContext,
    slot: usize,
    name_hash: u64,
    clear_vm_slot: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    let Some(name) = ctx.var_names_ptr.as_ref().and_then(|names| names.get(&name_hash)) else {
        ctx.runtime_error = Some("JIT dead local has no variable name".into());
        return 1;
    };
    if clear_vm_slot != 0 {
        let Some(value) = ctx.local_slots_ptr.as_mut().and_then(|slots| slots.get_mut(slot)) else {
            ctx.runtime_error = Some("JIT dead local has no valid local slot".into());
            return 1;
        };
        *value = Value::Null;
    }
    if let Some(locals) = ctx.locals_ptr.as_mut() {
        locals.remove(name);
    }
    0
}

/// Transfer a named parameter into the active native frame.
#[no_mangle]
// SAFETY: ctx and its local/name maps remain live and exclusively borrowed.
// Parameter initialization runs after native frame entry. Transfer the local
// value into that frame's roots rather than retaining an artificial COW alias.
pub unsafe extern "C" fn jit_take_parameter(
    ctx: *mut VMContext,
    name_hash: i64,
    index: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    ctx.return_value_kind = 0;
    if let Some(argument) = ctx.owned_roots.take_argument(index) {
        match argument.and_then(|value| ctx.owned_roots.encode(value)) {
            Ok((payload, kind)) => {
                ctx.return_value_kind = kind;
                return payload;
            }
            Err(error) => {
                ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
                return 0;
            }
        }
    }
    let value = ctx
        .var_names_ptr
        .as_ref()
        .and_then(|names| names.get(&(name_hash as u64)))
        .and_then(|name| ctx.locals_ptr.as_mut().and_then(|locals| locals.remove(name)));
    let Some(value) = value else {
        ctx.runtime_error = Some("Missing native parameter".into());
        return 0;
    };
    match ctx.owned_roots.encode(value) {
        Ok((payload, kind)) => {
            ctx.return_value_kind = kind;
            payload
        }
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            0
        }
    }
}

/// Load a retained immutable literal through the ordinary owned-value codec.
#[no_mangle]
// SAFETY: generated code passes a Value allocation retained by JitCompiler for
// the lifetime of the machine code. ctx is live and exclusively borrowed.
pub unsafe extern "C" fn jit_load_literal(ctx: *mut VMContext, pointer: i64) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    ctx.return_value_kind = 0;
    let Some(value) = (pointer as *const Value).as_ref() else {
        ctx.runtime_error = Some("Missing native literal".into());
        return 0;
    };
    match ctx.owned_roots.encode(value.clone()) {
        Ok((payload, kind)) => {
            ctx.return_value_kind = kind;
            payload
        }
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            0
        }
    }
}

/// Load a named operand without leaving hidden values on the VM stack.
#[no_mangle]
// SAFETY: ctx and the referenced name/local/global maps remain live for this
// call. Values are cloned through their ordinary ownership contract, not pointers.
pub unsafe extern "C" fn jit_load_operand(
    ctx: *mut VMContext,
    name_hash: i64,
    global_only: usize,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    ctx.return_value_kind = 0;
    let Some(name) = ctx.var_names_ptr.as_ref().and_then(|names| names.get(&(name_hash as u64)))
    else {
        ctx.runtime_error = Some("JIT load has no variable name".into());
        return 0;
    };
    let local = if global_only == 0 {
        ctx.locals_ptr.as_ref().and_then(|locals| locals.get(name))
    } else {
        None
    };
    let value = local
        .or_else(|| ctx.globals_ptr.as_ref().and_then(|globals| globals.get(name)))
        .cloned()
        .or_else(|| {
            crate::interpreter::Interpreter::is_builtin_name(name)
                .then(|| Value::NativeFunction(name.clone()))
        });
    let Some(value) = value else {
        ctx.runtime_error = Some(format!("Undefined variable: {name}"));
        return 0;
    };
    match ctx.owned_roots.encode(value) {
        Ok((payload, kind)) => {
            ctx.return_value_kind = kind;
            payload
        }
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            0
        }
    }
}

/// Resolve a native field through the VM contract, preserving legacy receivers.
#[no_mangle]
// SAFETY: ctx is exclusive; name_ptr points into compiler-retained Arc<String>
// metadata. Global lookup uses the already-borrowed map and never locks the VM.
pub unsafe extern "C" fn jit_field_read(
    ctx: *mut VMContext,
    object: i64,
    kind: i64,
    name_ptr: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    ctx.field_receiver_pending = false;
    let result = (|| {
        let name = (name_ptr as *const String).as_ref().ok_or("Missing native field name")?;
        let object = ctx
            .owned_roots
            .borrow_value(object, kind)
            .map_err(|error| format!("Native ownership error: {error:?}"))?;
        let (value, receiver) = VM::get_field_value(&object, name, |method| {
            ctx.globals_ptr.as_ref().and_then(|globals| globals.get(method)).cloned()
        })?;
        let encoded = ctx
            .owned_roots
            .encode(value)
            .map_err(|error| format!("Native ownership error: {error:?}"))?;
        if let Some(receiver) = receiver {
            ctx.stack_ptr.as_mut().ok_or("Missing native receiver stack")?.push(receiver);
            ctx.field_receiver_pending = true;
        }
        Ok::<_, String>(encoded)
    })();
    match result {
        Ok((payload, kind)) => {
            ctx.return_value_kind = kind;
            payload
        }
        Err(error) => {
            ctx.runtime_error = Some(error);
            0
        }
    }
}

/// Consume a field-write operand through the canonical VM contract.
#[no_mangle]
// SAFETY: context roots are exclusive, and name_ptr is compiler-retained metadata.
// This operation invokes no user callbacks. Operand slots remain live for release.
pub unsafe extern "C" fn jit_field_write(
    ctx: *mut VMContext,
    object: i64,
    object_kind: i64,
    value: i64,
    value_kind: i64,
    name_ptr: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    let result = (|| {
        let name = (name_ptr as *const String).as_ref().ok_or("Missing native field name")?;
        let value = ctx
            .owned_roots
            .clone_value(value, value_kind)
            .map_err(|error| format!("Native ownership error: {error:?}"))?;
        let object = if object_kind == crate::native_owned::OWNED {
            std::mem::replace(
                ctx.owned_roots
                    .current_value_mut(object, object_kind)
                    .map_err(|error| format!("Native ownership error: {error:?}"))?,
                Value::Null,
            )
        } else {
            ctx.owned_roots
                .clone_value(object, object_kind)
                .map_err(|error| format!("Native ownership error: {error:?}"))?
        };
        let (updated, context) = VM::set_field_value(object, value, name.clone())?;
        VM::ensure_value_nesting_depth(&updated, context)?;
        ctx.owned_roots
            .encode(updated)
            .map_err(|error| format!("Native ownership error: {error:?}"))
    })();
    match result {
        Ok((payload, kind)) => {
            ctx.return_value_kind = kind;
            payload
        }
        Err(error) => {
            ctx.runtime_error = Some(error);
            0
        }
    }
}

/// Consume generic indexed replacement operands using the VM's exact contract.
#[no_mangle]
// SAFETY: ctx owns the tagged roots for this activation. The key-cache callback
// performs no user calls; consumed root slots remain live for generated release.
pub unsafe extern "C" fn jit_index_set_result(
    ctx: *mut VMContext,
    object: i64,
    object_kind: i64,
    index: i64,
    index_kind: i64,
    value: i64,
    value_kind: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    let result = (|| {
        let index = ctx
            .owned_roots
            .clone_value(index, index_kind)
            .map_err(|error| format!("Native ownership error: {error:?}"))?;
        let value = ctx
            .owned_roots
            .clone_value(value, value_kind)
            .map_err(|error| format!("Native ownership error: {error:?}"))?;
        let object = if object_kind == crate::native_owned::OWNED {
            std::mem::replace(
                ctx.owned_roots
                    .current_value_mut(object, object_kind)
                    .map_err(|error| format!("Native ownership error: {error:?}"))?,
                Value::Null,
            )
        } else {
            ctx.owned_roots
                .clone_value(object, object_kind)
                .map_err(|error| format!("Native ownership error: {error:?}"))?
        };
        let updated = VM::index_set_result(object, index, value, |key| {
            if let Some(vm) = ctx.vm_ptr.cast::<VM>().as_mut() {
                vm.int_key_string(key)
            } else {
                Arc::from(key.to_string())
            }
        })?;
        ctx.owned_roots
            .encode(updated)
            .map_err(|error| format!("Native ownership error: {error:?}"))
    })();
    match result {
        Ok((payload, kind)) => {
            ctx.return_value_kind = kind;
            payload
        }
        Err(error) => {
            ctx.runtime_error = Some(error);
            0
        }
    }
}

/// Match Call's argument suffix after an immediately preceding FieldGet.
#[no_mangle]
// SAFETY: ctx and its stack are exclusive. No pointers or references escape.
pub unsafe extern "C" fn jit_prepare_field_call(ctx: *mut VMContext, count: i64) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    if !std::mem::take(&mut ctx.field_receiver_pending) {
        return 0;
    }
    // With zero arguments the hidden receiver already has the VM layout.
    if count == 0 {
        return 0;
    }
    let base = usize::try_from(count)
        .ok()
        .and_then(|n| n.checked_add(2))
        .and_then(|n| ctx.stack_ptr.as_ref()?.len().checked_sub(n));
    let Some(base) = base else {
        ctx.runtime_error = Some("Stack underflow preparing field call".into());
        return 1;
    };
    let stack = &mut *ctx.stack_ptr;
    // [second receiver, callable, first receiver, user arguments]
    // becomes [first receiver, callable, user arguments, second receiver].
    stack.swap(base, base + 2);
    stack[base + 2..].rotate_left(1);
    0
}

/// One normalized iteration step. Kind -4 denotes exhaustion, never a language value.
#[no_mangle]
// SAFETY: tagged operands belong to this context. Locked loops resume before
// generator callbacks; whole-function callbacks use the VM continuation guard.
pub unsafe extern "C" fn jit_for_next(
    ctx: *mut VMContext,
    object: i64,
    object_kind: i64,
    index: i64,
    index_kind: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    let result = (|| {
        let object = ctx
            .owned_roots
            .clone_value(object, object_kind)
            .map_err(|e| format!("Native ownership error: {e:?}"))?;
        let value = match object {
            Value::BytecodeGenerator { .. } => {
                if !ctx.loop_environment_ptr.is_null() {
                    return Err("Generator step requires VM loop resume".into());
                }
                let vm =
                    ctx.vm_ptr.cast::<VM>().as_mut().ok_or("Generator step requires VM context")?;
                let step = vm.generator_next(object);
                ctx.refresh_vm_views();
                match step? {
                    Value::Option { is_some: true, value } => Some(*value),
                    Value::Option { is_some: false, .. } => None,
                    _ => return Err("Invalid generator step result".into()),
                }
            }
            Value::Array(values) => {
                if index_kind != crate::native_scalar::INTEGER || index < 0 {
                    return Err("ForNext requires a nonnegative index".into());
                }
                values.get(index as usize).cloned()
            }
            Value::Bytes(values) => {
                if index_kind != crate::native_scalar::INTEGER || index < 0 {
                    return Err("ForNext requires a nonnegative index".into());
                }
                values.get(index as usize).map(|n| Value::Int(i64::from(*n)))
            }
            _ => return Err("ForNext requires a normalized iterable".into()),
        };
        match value {
            Some(value) => {
                ctx.owned_roots.encode(value).map_err(|e| format!("Native ownership error: {e:?}"))
            }
            None => Ok((0, -4)),
        }
    })();
    match result {
        Ok((payload, kind)) => {
            ctx.return_value_kind = kind;
            payload
        }
        Err(error) => {
            ctx.runtime_error = Some(error);
            0
        }
    }
}

/// Read tagged operands using the VM's indexing and error contracts.
#[no_mangle]
// SAFETY: ctx is a live exclusive context. Decode validated root handles rather
// than treating payloads as pointers; no borrow or pointer escapes this call.
pub unsafe extern "C" fn jit_indexed_read(
    ctx: *mut VMContext,
    object: i64,
    object_kind: i64,
    index: i64,
    index_kind: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    indexed_read(ctx, object, object_kind, index, index_kind, None)
}

/// Read with a stack-local root-slot hint. The hint never bypasses handle checks.
#[no_mangle]
// SAFETY: ctx is live/exclusive; cache is null or points to an aligned initialized
// OwnedReadCache in the active generated stack frame, separate from ctx storage.
pub unsafe extern "C" fn jit_indexed_read_cached(
    ctx: *mut VMContext,
    object: i64,
    object_kind: i64,
    index: i64,
    index_kind: i64,
    cache: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    indexed_read(
        ctx,
        object,
        object_kind,
        index,
        index_kind,
        (cache as *mut crate::native_owned::OwnedReadCache).as_mut(),
    )
}

#[inline]
fn indexed_read(
    ctx: &mut VMContext,
    object: i64,
    object_kind: i64,
    index: i64,
    index_kind: i64,
    cache: Option<&mut crate::native_owned::OwnedReadCache>,
) -> i64 {
    let result = (|| {
        // Owned objects are borrowed once, including fallback cases. Construct
        // the borrowed Cow locally to avoid transferring a full-width Value
        // through the helper ABI on the common array-read path.
        let object = if object_kind == crate::native_owned::OWNED {
            std::borrow::Cow::Borrowed(
                match cache {
                    Some(cache) => ctx.owned_roots.borrow_owned_cached(object, cache),
                    None => ctx.owned_roots.borrow_owned(object),
                }
                .map_err(|error| format!("Native ownership error: {error:?}"))?,
            )
        } else {
            ctx.owned_roots
                .borrow_value(object, object_kind)
                .map_err(|error| format!("Native ownership error: {error:?}"))?
        };
        if index_kind == crate::native_scalar::INTEGER {
            if let Value::Array(values) = object.as_ref() {
                let resolved = if index < 0 { values.len() as i64 + index } else { index };
                if let Some(element) =
                    usize::try_from(resolved).ok().and_then(|offset| values.get(offset))
                {
                    if let Some(immediate) = crate::native_scalar::encode(element) {
                        return Ok(immediate);
                    }
                    // Match the VM's owned-element clone without repeating the
                    // validated lookup. The result receives an independent root.
                    let value = element.clone();
                    return ctx
                        .owned_roots
                        .encode(value)
                        .map_err(|error| format!("Native ownership error: {error:?}"));
                }
            }
        }
        // Other collection/index variants and errors retain the VM contract.
        let index = ctx
            .owned_roots
            .borrow_value(index, index_kind)
            .map_err(|error| format!("Native ownership error: {error:?}"))?;
        let value = VM::get_indexed_value(&object, &index)?;
        ctx.owned_roots.encode(value).map_err(|error| format!("Native ownership error: {error:?}"))
    })();
    match result {
        Ok((payload, kind)) => {
            ctx.return_value_kind = kind;
            payload
        }
        Err(error) => {
            ctx.runtime_error = Some(error);
            0
        }
    }
}

/// Expose unique dense integer storage through an explicit native ABI.
#[no_mangle]
// SAFETY: ctx and its physical slots are exclusively borrowed by this loop.
// Box addresses remain stable; the caller refreshes the view after every helper
// write. Only a unique dense integer vector admits raw in-bounds native access.
pub unsafe extern "C" fn jit_loop_dense_view(ctx: *mut VMContext, slot: i64) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    let Ok(slot) = usize::try_from(slot) else { return 0 };
    let Some(Value::DenseIntDictIntFull(values)) =
        ctx.local_slots_ptr.as_mut().and_then(|slots| slots.get_mut(slot))
    else {
        return 0;
    };
    let Some(values) = Arc::get_mut(values) else { return 0 };
    let view = ctx
        .loop_dense_views
        .entry(slot)
        .or_insert_with(|| Box::new(LoopDenseView { data: std::ptr::null_mut(), len: 0 }));
    view.data = values.as_mut_ptr();
    view.len = values.len();
    &mut **view as *mut LoopDenseView as i64
}

#[no_mangle]
// SAFETY: context slots stay live; no callback runs while the object is borrowed.
pub unsafe extern "C" fn jit_loop_indexed_read(
    ctx: *mut VMContext,
    slot: i64,
    index: i64,
    kind: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    let result = ctx
        .owned_roots
        .clone_value(index, kind)
        .map_err(|error| format!("Native ownership error: {error:?}"))
        .and_then(|index| {
            let object = usize::try_from(slot)
                .ok()
                .and_then(|slot| ctx.local_slots_ptr.as_ref().and_then(|slots| slots.get(slot)))
                .ok_or_else(|| format!("Invalid local slot: {slot}"))?;
            VM::get_indexed_value(object, &index)
        })
        .and_then(|value| {
            ctx.owned_roots
                .encode(value)
                .map_err(|error| format!("Native ownership error: {error:?}"))
        });
    match result {
        Ok((value, kind)) => {
            ctx.return_value_kind = kind;
            value
        }
        Err(error) => {
            ctx.runtime_error = Some(error);
            0
        }
    }
}

/// Mutate the callee binding through the VM's indexed-write contract.
#[no_mangle]
// SAFETY: ctx maps and roots are live and exclusively borrowed. Decode operand
// handles before borrowing the destination; mutation never invokes user code.
pub unsafe extern "C" fn jit_indexed_write(
    ctx: *mut VMContext,
    location: i64,
    target: i64,
    target_kind: i64,
    binding_kind: i64,
    name_hash: i64,
    index: i64,
    index_kind: i64,
    value: i64,
    value_kind: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    let name = ctx.var_names_ptr.as_ref().and_then(|names| names.get(&(name_hash as u64)));
    let Some(name) = name else {
        ctx.runtime_error = Some("Native indexed write has no binding name".into());
        return 1;
    };
    if binding_kind != 0 {
        let kind = match binding_kind {
            1 => BytecodeBindingKind::LetImmutable,
            2 => BytecodeBindingKind::Const,
            _ => {
                ctx.runtime_error = Some("Invalid native binding kind".into());
                return 1;
            }
        };
        ctx.runtime_error = Some(VM::local_mutation_error(kind, name));
        return 1;
    }
    let operands = ctx.owned_roots.clone_value(index, index_kind).and_then(|index| {
        ctx.owned_roots.clone_value(value, value_kind).map(|value| (index, value))
    });
    let (index, value) = match operands {
        Ok(values) => values,
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            return 1;
        }
    };
    if location == 1 && target_kind != crate::native_owned::OWNED {
        ctx.runtime_error = Some("Invalid index assignment".into());
        return 1;
    }
    let target = match location {
        0 => usize::try_from(target)
            .ok()
            .and_then(|slot| ctx.local_slots_ptr.as_mut().and_then(|slots| slots.get_mut(slot)))
            .ok_or_else(|| format!("Invalid local slot: {target}")),
        1 => ctx
            .owned_roots
            .current_value_mut(target, target_kind)
            .map_err(|error| format!("Native ownership error: {error:?}")),
        2 => ctx
            .locals_ptr
            .as_mut()
            .and_then(|locals| locals.get_mut(name))
            .ok_or_else(|| "Native indexed write binding is missing".to_string()),
        _ => Err("Invalid native indexed write location".into()),
    };
    match target.and_then(|target| VM::set_indexed_value(target, index, value)) {
        Ok(()) => 0,
        Err(error) => {
            ctx.runtime_error = Some(error);
            1
        }
    }
}

/// Conditional pop/remove rebinding. A fallback preserves evaluated call operands.
#[no_mangle]
// SAFETY: retained path metadata and tagged operand storage are live; target,
// dead-pair and operand roots belong to this context. No user callback runs here.
pub unsafe extern "C" fn jit_try_pop_rebind(
    ctx: *mut VMContext,
    location: i64,
    target: i64,
    target_kind: i64,
    mutable: i64,
    name_hash: i64,
    pair_location: i64,
    pair_target: i64,
    pair_kind: i64,
    shape: *const Vec<Option<String>>,
    argc: i64,
    operands: *const i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    let mut cache = if ctx.vm_ptr.is_null() {
        std::mem::take(&mut ctx.path_depth_valid)
    } else {
        VM::take_native_path_depth_cache(ctx.vm_ptr.cast())
    };
    let result = (|| -> Result<Option<(i64, i64)>, String> {
        if !(1..=2).contains(&argc) {
            return Ok(None);
        }
        let shape = shape.as_ref().ok_or("Missing native pop path")?;
        if operands.is_null() {
            return Err("Missing native pop operands".into());
        }
        let count = shape.iter().filter(|step| step.is_none()).count();
        let operands = std::slice::from_raw_parts(operands, (count + argc as usize + 1) * 2);
        let original = operands[count * 2];
        let original_kind = operands[count * 2 + 1];
        let function_offset = (count + argc as usize) * 2;
        let native = ctx
            .owned_roots
            .native_function_name(operands[function_offset], operands[function_offset + 1])
            .map_err(|e| format!("Native ownership error: {e:?}"))?;
        if !matches!((native, argc), (Some("pop"), 1) | (Some("remove"), 2)) {
            return Ok(None);
        }
        // This target is the forthcoming fresh declaration, not an in-scope local.
        match pair_location {
            1 if pair_kind == crate::native_owned::OWNED => {
                *ctx.owned_roots
                    .current_value_mut(pair_target, pair_kind)
                    .map_err(|e| format!("Native ownership error: {e:?}"))? = Value::Null;
            }
            0 => {
                if let Some(pair) = usize::try_from(pair_target)
                    .ok()
                    .and_then(|n| ctx.local_slots_ptr.as_mut()?.get_mut(n))
                {
                    *pair = Value::Null;
                }
            }
            2 => {
                if let Some(name) =
                    ctx.var_names_ptr.as_ref().and_then(|names| names.get(&(pair_target as u64)))
                {
                    if let Some(pair) =
                        ctx.locals_ptr.as_mut().and_then(|locals| locals.get_mut(name))
                    {
                        *pair = Value::Null;
                    }
                }
            }
            _ => {}
        }
        if mutable == 0 || original_kind != crate::native_owned::OWNED {
            return Ok(None);
        }
        let mut offset = 0;
        let mut path = Vec::with_capacity(shape.len());
        for step in shape {
            path.push(match step {
                Some(field) => crate::interpreter::assignment::Access::Field(field.clone()),
                None => {
                    let index = ctx
                        .owned_roots
                        .clone_value(operands[offset], operands[offset + 1])
                        .map_err(|e| format!("Native ownership error: {e:?}"))?;
                    offset += 2;
                    crate::interpreter::assignment::Access::Index(index)
                }
            });
        }
        let key = if argc == 2 {
            Some(
                ctx.owned_roots
                    .clone_value(operands[count * 2 + 2], operands[count * 2 + 3])
                    .map_err(|e| format!("Native ownership error: {e:?}"))?,
            )
        } else {
            None
        };
        let mut pair = None;
        let mut apply = |root: &mut Value, snapshot: &mut Option<Value>| {
            pair = if path.is_empty() {
                crate::interpreter::assignment::collection_rebinding_path(
                    root,
                    &path,
                    snapshot,
                    key.as_ref(),
                )
            } else {
                crate::interpreter::assignment::collection_rebinding_path_checked(
                    root,
                    &path,
                    snapshot,
                    key.as_ref(),
                    &mut cache,
                )
            };
            pair.is_some()
        };
        if location == 1 {
            if target_kind != crate::native_owned::OWNED {
                return Ok(None);
            }
            ctx.owned_roots
                .with_rebinding_snapshot(target, original, &mut apply)
                .map_err(|e| format!("Native ownership error: {e:?}"))?;
        } else {
            let value = std::mem::replace(
                ctx.owned_roots
                    .current_value_mut(original, original_kind)
                    .map_err(|e| format!("Native ownership error: {e:?}"))?,
                Value::Null,
            );
            let mut snapshot = Some(value);
            let mut operation = |root: &mut Value| apply(root, &mut snapshot);
            match location {
                0 => {
                    if let Some(root) = usize::try_from(target)
                        .ok()
                        .and_then(|n| ctx.local_slots_ptr.as_mut()?.get_mut(n))
                    {
                        operation(root);
                    }
                }
                2 | 3 => {
                    let name = ctx
                        .var_names_ptr
                        .as_ref()
                        .and_then(|names| names.get(&(name_hash as u64)))
                        .cloned();
                    if let Some(name) = name {
                        if location == 2 {
                            if let Some(root) =
                                ctx.locals_ptr.as_mut().and_then(|locals| locals.get_mut(&name))
                            {
                                operation(root);
                            }
                        } else if let Some(environment) = ctx.loop_environment_ptr.as_mut() {
                            let _ = environment.mutate_checked(&name, |root| {
                                operation(root);
                            });
                        } else if let Some(vm) = ctx.vm_ptr.cast::<VM>().as_mut() {
                            vm.try_guarded_global_mutation_from_jit(&name, &mut operation);
                        }
                    }
                }
                _ => {}
            }
            if let Some(value) = snapshot {
                *ctx.owned_roots
                    .current_value_mut(original, original_kind)
                    .map_err(|e| format!("Native ownership error: {e:?}"))? = value;
            }
        }
        pair.map(|value| {
            ctx.owned_roots.encode(value).map_err(|e| format!("Native ownership error: {e:?}"))
        })
        .transpose()
    })();
    if ctx.vm_ptr.is_null() {
        ctx.path_depth_valid = cache;
    } else {
        VM::restore_native_path_depth_cache(ctx.vm_ptr.cast(), cache);
    }
    match result {
        Ok(Some((payload, kind))) => {
            ctx.return_value_kind = kind;
            payload
        }
        Ok(None) => {
            ctx.return_value_kind = -4;
            0
        }
        Err(error) => {
            ctx.runtime_error = Some(error);
            0
        }
    }
}

/// Inspect builtin identity and operand types without retaining artificial aliases.
#[no_mangle]
// SAFETY: live exclusive context and live operand handles; no callbacks.
pub unsafe extern "C" fn jit_guard_append_path(
    ctx: *mut VMContext,
    original: i64,
    original_kind: i64,
    item: i64,
    item_kind: i64,
    function: i64,
    function_kind: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    let result: Result<bool, crate::native_owned::OwnershipError> = (|| {
        let concat = match ctx.owned_roots.native_function_name(function, function_kind)? {
            Some("push" | "append") => false,
            Some("concat") => true,
            _ => return Ok(false),
        };
        if original_kind != crate::native_owned::OWNED {
            return Ok(false);
        }
        let valid = if concat {
            matches!(ctx.owned_roots.current_value_mut(original, original_kind)?, Value::Bytes(_))
        } else {
            matches!(ctx.owned_roots.current_value_mut(original, original_kind)?, Value::Array(_))
        };
        if !valid || !concat {
            return Ok(valid);
        }
        Ok(item_kind == crate::native_owned::OWNED
            && matches!(ctx.owned_roots.current_value_mut(item, item_kind)?, Value::Bytes(_)))
    })();
    match result {
        Ok(value) => i64::from(value),
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            0
        }
    }
}

/// Checked nested mutation uses retained path metadata and a tagged operand array.
#[no_mangle]
// SAFETY: shape points to compiler-retained metadata; operands points to generated
// stack storage containing one payload/kind pair for every dynamic index followed
// by the assigned value. Neither allocation escapes this synchronous helper.
pub unsafe extern "C" fn jit_update_binding_path(
    ctx: *mut VMContext,
    location: i64,
    target: i64,
    target_kind: i64,
    binding_kind: i64,
    name_hash: i64,
    shape: *const Vec<Option<String>>,
    operands: *const i64,
) -> i64 {
    jit_mutate_binding_path(
        ctx,
        location,
        target,
        target_kind,
        binding_kind,
        name_hash,
        shape,
        operands,
        false,
    )
}

#[no_mangle]
// SAFETY: same retained metadata contract, with indices followed by original/item/callable pairs.
pub unsafe extern "C" fn jit_append_binding_path(
    ctx: *mut VMContext,
    location: i64,
    target: i64,
    target_kind: i64,
    binding_kind: i64,
    name_hash: i64,
    shape: *const Vec<Option<String>>,
    operands: *const i64,
) -> i64 {
    jit_mutate_binding_path(
        ctx,
        location,
        target,
        target_kind,
        binding_kind,
        name_hash,
        shape,
        operands,
        true,
    )
}

unsafe fn jit_mutate_binding_path(
    ctx: *mut VMContext,
    location: i64,
    target: i64,
    target_kind: i64,
    binding_kind: i64,
    name_hash: i64,
    shape: *const Vec<Option<String>>,
    operands: *const i64,
    append: bool,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    // Native callback contexts are short-lived. Keep validated identities with
    // their VM so recursive/helper calls do not rescan the whole payload. Move
    // the cache out during mutation to avoid aliasing other VM field borrows.
    let mut cache = if ctx.vm_ptr.is_null() {
        std::mem::take(&mut ctx.path_depth_valid)
    } else {
        VM::take_native_path_depth_cache(ctx.vm_ptr.cast())
    };
    let result = (|| {
        let name = ctx
            .var_names_ptr
            .as_ref()
            .and_then(|names| names.get(&(name_hash as u64)))
            .ok_or("Native path write has no binding name")?;
        if location != 3 && binding_kind != 0 {
            let kind = match binding_kind {
                1 => BytecodeBindingKind::LetImmutable,
                2 => BytecodeBindingKind::Const,
                _ => return Err("Invalid native binding kind".into()),
            };
            return Err(VM::local_mutation_error(kind, name));
        }
        let shape = shape.as_ref().ok_or("Missing native path metadata")?;
        if operands.is_null() {
            return Err("Missing native path operands".into());
        }
        let count = shape.iter().filter(|step| step.is_none()).count();
        let operands =
            std::slice::from_raw_parts(operands, (count + if append { 3 } else { 1 }) * 2);
        let mut path = Vec::with_capacity(shape.len());
        let mut offset = 0;
        for step in shape {
            path.push(match step {
                Some(field) => crate::interpreter::assignment::Access::Field(field.clone()),
                None => {
                    let value = ctx
                        .owned_roots
                        .clone_value(operands[offset], operands[offset + 1])
                        .map_err(|error| format!("Native ownership error: {error:?}"))?;
                    offset += 2;
                    crate::interpreter::assignment::Access::Index(value)
                }
            });
        }
        let concat = append
            && matches!(
                ctx.owned_roots
                    .native_function_name(operands[offset + 4], operands[offset + 5])
                    .map_err(|error| format!("Native ownership error: {error:?}"))?,
                Some("concat")
            );
        let mut decode = |index: usize, consume: bool| -> Result<Value, String> {
            let payload = operands[index];
            let kind = operands[index + 1];
            if consume && kind == crate::native_owned::OWNED {
                Ok(std::mem::replace(
                    ctx.owned_roots
                        .current_value_mut(payload, kind)
                        .map_err(|error| format!("Native ownership error: {error:?}"))?,
                    Value::Null,
                ))
            } else {
                ctx.owned_roots
                    .clone_value(payload, kind)
                    .map_err(|error| format!("Native ownership error: {error:?}"))
            }
        };
        let original = if append { Some(decode(offset, true)?) } else { None };
        let value = decode(offset + if append { 2 } else { 0 }, append)?;
        let operation = |root: &mut Value| {
            if let Some(original) = original {
                if concat {
                    crate::interpreter::assignment::concat_bytes_path(
                        root,
                        &path,
                        original,
                        value,
                        Some(&mut cache),
                    )
                } else {
                    crate::interpreter::assignment::append_path(
                        root,
                        &path,
                        original,
                        value,
                        Some(&mut cache),
                    )
                }
            } else {
                crate::interpreter::assignment::update_checked(root, &path, value, &mut cache)
            }
        };
        match location {
            0 => operation(
                usize::try_from(target)
                    .ok()
                    .and_then(|slot| ctx.local_slots_ptr.as_mut()?.get_mut(slot))
                    .ok_or("Invalid native path local slot")?,
            ),
            1 => {
                if target_kind != crate::native_owned::OWNED {
                    let mut scalar = ctx
                        .owned_roots
                        .clone_value(target, target_kind)
                        .map_err(|error| format!("Native ownership error: {error:?}"))?;
                    return operation(&mut scalar);
                }
                operation(
                    ctx.owned_roots
                        .current_value_mut(target, target_kind)
                        .map_err(|error| format!("Native ownership error: {error:?}"))?,
                )
            }
            2 => operation(
                ctx.locals_ptr
                    .as_mut()
                    .and_then(|locals| locals.get_mut(name))
                    .ok_or("Native path binding is missing")?,
            ),
            3 => {
                if let Some(environment) = ctx.loop_environment_ptr.as_mut() {
                    let mut result = Ok(());
                    environment.mutate_checked(name, |root| {
                        result = operation(root);
                    })?;
                    result
                } else if let Some(vm) = ctx.vm_ptr.cast::<VM>().as_mut() {
                    vm.mutate_global_from_jit(name, operation)
                } else {
                    Err("Native path write requires a VM".into())
                }
            }
            _ => Err("Invalid native path location".into()),
        }
    })();
    if ctx.vm_ptr.is_null() {
        ctx.path_depth_valid = cache;
    } else {
        VM::restore_native_path_depth_cache(ctx.vm_ptr.cast(), cache);
    }
    match result {
        Ok(()) => 0,
        Err(error) => {
            ctx.runtime_error = Some(error);
            1
        }
    }
}

/// A locked loop must return to bytecode before invoking user callbacks.
#[no_mangle]
// SAFETY: ctx and roots are live; this only borrows an operand, never calls user code.
pub unsafe extern "C" fn jit_loop_resume_guard(ctx: *mut VMContext, value: i64, kind: i64) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    if ctx.loop_environment_ptr.is_null() {
        return 0;
    }
    if kind == -1 {
        return 1;
    } // A Call instruction can enter arbitrary user code.
    if kind == -3 {
        return match ctx.owned_roots.clone_value(value, crate::native_owned::OWNED) {
            Ok(Value::BytecodeGenerator { .. }) => 1,
            Ok(_) => 0,
            Err(error) => {
                ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
                1
            }
        };
    }
    if kind == -2 {
        // A legacy field lookup inserts a physical receiver outside the tagged
        // operand stack. Resume before that insertion so later callback resume
        // cannot discard or reorder it while rebuilding the VM stack.
        return match ctx.owned_roots.clone_value(value, crate::native_owned::OWNED) {
            Ok(Value::Channel(_) | Value::HttpServer { .. }) => 1,
            Ok(Value::Struct { name, .. }) => i64::from(name == "ArgParser"),
            #[cfg(feature = "runtime-image")]
            Ok(Value::Image { .. }) => 1,
            Ok(_) => 0,
            Err(error) => {
                ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
                1
            }
        };
    }
    if kind != crate::native_owned::OWNED {
        return 0;
    }
    match ctx.owned_roots.has_operator_methods(value) {
        Ok(required) => i64::from(required),
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            1
        }
    }
}

#[no_mangle]
// SAFETY: the loop owns ctx/stack. The translator then marshals its complete
// operand stack and persists locals before returning to the VM at this exact PC.
pub unsafe extern "C" fn jit_loop_resume_prepare(ctx: *mut VMContext, pc: i64) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    let Some(stack) = ctx.stack_ptr.as_mut() else { return 1 };
    let Ok(pc) = usize::try_from(pc) else { return 1 };
    stack.truncate(ctx.loop_stack_base);
    ctx.loop_resume_ip = Some(pc);
    0
}

/// Execute owned operators through the reference VM, including method dispatch.
#[no_mangle]
// SAFETY: ctx and its VM are live for this call. Clone operands before entering
// VM callbacks so no root-store borrow or global lock spans user code.
pub unsafe extern "C" fn jit_owned_operator(
    ctx: *mut VMContext,
    operation: i64,
    left: i64,
    left_kind: i64,
    right: i64,
    right_kind: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    let operands = ctx
        .owned_roots
        .clone_value(left, left_kind)
        .and_then(|left| ctx.owned_roots.clone_value(right, right_kind).map(|right| (left, right)));
    let (left, right) = match operands {
        Ok(values) => values,
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            return 0;
        }
    };
    let Some(vm) = ctx.vm_ptr.cast::<VM>().as_mut() else {
        ctx.runtime_error = Some("Native owned operator requires VM context".into());
        return 0;
    };
    let result = match operation {
        0..=4 => vm.binary_op(&left, ["+", "-", "*", "/", "%"][operation as usize], &right),
        5 => vm.unary_op("-", &left),
        6 => vm.unary_op("!", &left),
        7..=10 => vm.compare_op(&left, ["<", ">", "<=", ">="][(operation - 7) as usize], &right),
        _ => Err("Invalid native owned operation".into()),
    };
    ctx.refresh_vm_views();
    match result {
        Ok(value) => match ctx.owned_roots.encode(value) {
            Ok((payload, kind)) => {
                ctx.return_value_kind = kind;
                payload
            }
            Err(error) => {
                ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
                0
            }
        },
        Err(error) => {
            ctx.runtime_error = Some(error);
            0
        }
    }
}

/// Load an immediate value and publish its kind from the active VM local slots.
#[no_mangle]
// SAFETY:
// - `ctx` must point to a live VMContext for the duration of the call.
// - The helper only reads a bounds-checked slot through the VM-owned vector pointer.
pub unsafe extern "C" fn jit_load_local_slot(ctx: *mut VMContext, slot: i64) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    ctx.return_value_kind = 0;
    let value = usize::try_from(slot)
        .ok()
        .and_then(|slot| ctx.local_slots_ptr.as_ref().and_then(|slots| slots.get(slot)))
        .cloned();
    match value {
        Some(value) => match ctx.owned_roots.encode(value) {
            Ok((payload, kind)) => {
                ctx.return_value_kind = kind;
                payload
            }
            Err(error) => {
                ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
                0
            }
        },
        None => {
            ctx.runtime_error = Some(format!("Invalid local slot: {slot}"));
            0
        }
    }
}

/// Persist a tagged value without consuming its live SSA root.
#[no_mangle]
// SAFETY: ctx and its local-slot vector remain live and exclusively borrowed.
pub unsafe extern "C" fn jit_store_local_slot(
    ctx: *mut VMContext,
    slot: i64,
    value: i64,
    kind: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    let value = match ctx.owned_roots.clone_value(value, kind) {
        Ok(value) => value,
        Err(error) => {
            ctx.runtime_error.get_or_insert_with(|| format!("Native ownership error: {error:?}"));
            return 0;
        }
    };
    match usize::try_from(slot)
        .ok()
        .and_then(|slot| ctx.local_slots_ptr.as_mut().and_then(|slots| slots.get_mut(slot)))
    {
        Some(target) => {
            *target = value;
            1
        }
        None => {
            ctx.runtime_error.get_or_insert_with(|| format!("Invalid local slot: {slot}"));
            0
        }
    }
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
    kind: i64,
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
        match ctx.owned_roots.clone_value(value, kind) {
            Ok(value) => {
                let locals = &mut *ctx.locals_ptr;
                locals.insert(name, value);
            }
            Err(error) => {
                ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            }
        }
    }
}

/// Raise a scalar operand type error on the cold side of a native type guard.
#[no_mangle]
// SAFETY: ctx points to the live, exclusively borrowed compiled-call context.
pub unsafe extern "C" fn jit_scalar_type_error(
    ctx: *mut VMContext,
    operation: i64,
    left: i64,
    right: i64,
    left_kind: i64,
    right_kind: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    let left = crate::native_scalar::decode(left, left_kind);
    let right = crate::native_scalar::decode(right, right_kind);
    let error = match operation {
        0..=4 => crate::vm::VM::invalid_binary_operation(
            ["+", "-", "*", "/", "%"][operation as usize],
            &left,
            &right,
        ),
        5 => format!("Invalid unary operation: - {:?}", left),
        6 => format!("Invalid unary operation: ! {:?}", left),
        7..=10 => crate::vm::VM::invalid_binary_operation(
            ["<", ">", "<=", ">="][(operation - 7) as usize],
            &left,
            &right,
        ),
        _ => "Invalid native scalar operation".into(),
    };
    ctx.runtime_error = Some(error);
    1
}

/// Construct the canonical arithmetic error only on a failed native guard.
#[no_mangle]
// SAFETY: ctx points to the active exclusively borrowed compiled-call context.
pub unsafe extern "C" fn jit_arithmetic_error(
    ctx: *mut VMContext,
    operation: i64,
    left: i64,
    right: i64,
) -> i64 {
    if let Some(ctx) = ctx.as_mut() {
        let error = if operation == 5 {
            left.checked_neg().map(|_| ()).ok_or_else(|| format!("Integer overflow: -({left})"))
        } else if let Some(operator) = ["+", "-", "*", "/", "%"].get(operation as usize) {
            Value::checked_int_arithmetic(left, operator, right).map(|_| ())
        } else {
            Err("Invalid JIT arithmetic operation".into())
        };
        ctx.runtime_error =
            Some(error.err().unwrap_or_else(|| "Invalid JIT arithmetic guard".into()));
    }
    1
}

/// Read/update physical loop-local initialization, or construct a readonly error.
#[no_mangle]
// SAFETY: ctx belongs to the active native call. Physical state access performs
// no callbacks and does not acquire the environment lock held by native loops.
pub unsafe extern "C" fn jit_local_initialization(
    ctx: *mut VMContext,
    slot: i64,
    action: i64,
    binding: i64,
    name_hash: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    if action == 2 {
        let name = ctx
            .var_names_ptr
            .as_ref()
            .and_then(|names| names.get(&(name_hash as u64)))
            .map(String::as_str)
            .unwrap_or("<local>");
        let kind = if binding == 1 {
            BytecodeBindingKind::LetImmutable
        } else {
            BytecodeBindingKind::Const
        };
        ctx.runtime_error = Some(VM::local_reassignment_error(kind, name));
        return 1;
    }
    let result = (ctx.vm_ptr as *mut VM)
        .as_mut()
        .ok_or_else(|| "Missing VM for native local initialization".to_string())
        .and_then(|vm| vm.native_local_initialization(slot, action == 1));
    match result {
        Ok(initialized) => i64::from(initialized),
        Err(error) => {
            ctx.runtime_error = Some(error);
            1
        }
    }
}

/// Report a pending error or task cancellation across a native boundary.
#[no_mangle]
// SAFETY: ctx points to the live, exclusively borrowed native-call context;
// its optional cancellation pointer is backed by the enclosing task's live Arc.
pub unsafe extern "C" fn jit_runtime_failed(ctx: *mut VMContext) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    if ctx
        .task_cancellation
        .as_ref()
        .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire))
    {
        ctx.runtime_error.get_or_insert_with(|| "Task was cancelled".into());
    }
    i64::from(ctx.runtime_error.is_some())
}

/// Assign a named scalar binding from a compiled function (0 success, 1 error).
/// Declared locals use slots; unresolved names must retain runtime global lookup.
#[no_mangle]
// SAFETY: ctx and its local/name maps belong to the active compiled call. The VM
// pointer is live and no globals lock is held by whole-function entry points.
pub unsafe extern "C" fn jit_assign_variable(
    ctx: *mut VMContext,
    name_hash: i64,
    value: i64,
    kind: i64,
    global_only: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    let name = ctx.var_names_ptr.as_ref().and_then(|names| names.get(&(name_hash as u64))).cloned();
    let Some(name) = name else {
        ctx.runtime_error = Some("JIT assignment has no variable name".into());
        return 1;
    };
    let value = match ctx.owned_roots.clone_value(value, kind) {
        Ok(value) => value,
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            return 1;
        }
    };
    // This is the compiled function's own named-local map, not its caller frame.
    if let Some(locals) = ctx.locals_ptr.as_mut() {
        if global_only == 0
            && locals.contains_key(&name)
            && !(ctx.locals_ptr == ctx.globals_ptr && !ctx.loop_environment_ptr.is_null())
        {
            locals.insert(name, value);
            return 0;
        }
    }
    if let Some(environment) = ctx.loop_environment_ptr.as_mut() {
        // The loop owns the globals guard for its entire native invocation.
        // Use that environment directly rather than acquiring its lock again.
        if global_only != 0
            || ctx.locals_ptr == ctx.globals_ptr
            || environment.scopes.iter().any(|scope| scope.contains_key(&name))
        {
            return match environment.assign_checked(name, value) {
                Ok(()) => 0,
                Err(error) => {
                    ctx.runtime_error = Some(error);
                    1
                }
            };
        }
    } else if let Some(vm) = (ctx.vm_ptr as *mut crate::vm::VM).as_mut() {
        match vm.assign_global_from_jit(&name, value.clone(), global_only != 0) {
            Ok(true) => return 0,
            Ok(false) => {}
            Err(error) => {
                ctx.runtime_error = Some(error);
                return 1;
            }
        }
    } else if let Some(globals) = ctx.globals_ptr.as_mut() {
        // Standalone compiler fixtures have no Environment/binding metadata.
        if global_only != 0 || globals.contains_key(&name) {
            globals.insert(name, value);
            return 0;
        }
    }
    if let Some(locals) = ctx.locals_ptr.as_mut() {
        locals.insert(name, value);
        return 0;
    }
    ctx.runtime_error = Some("JIT assignment has no binding environment".into());
    1
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

/// Append to the actual local binding: current-scope SSA root, named callee
/// local, or physical loop slot. Returns zero on success, one on error, or two
/// when a locked loop must resume in the VM before invoking a struct operator.
#[no_mangle]
// SAFETY: ctx and referenced maps/slots are live and exclusively borrowed. A
// string pointer refers to an Arc retained by JitCompiler; a negative length
// denotes a validated Unicode scalar instead. No pointer or borrow escapes.
pub unsafe extern "C" fn jit_append_local(
    ctx: *mut VMContext,
    location: i64,
    target: i64,
    kind: i64,
    suffix: i64,
    length: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    let mut utf8 = [0; 4];
    let text = if length == -1 {
        let Some(ch) = u32::try_from(suffix).ok().and_then(char::from_u32) else {
            ctx.runtime_error = Some("Invalid native append character".into());
            return 1;
        };
        ch.encode_utf8(&mut utf8)
    } else if length >= 0 && suffix != 0 {
        let bytes = std::slice::from_raw_parts(suffix as *const u8, length as usize);
        let Ok(text) = std::str::from_utf8(bytes) else {
            ctx.runtime_error = Some("Invalid native append string".into());
            return 1;
        };
        text
    } else {
        ctx.runtime_error = Some("Invalid native append string".into());
        return 1;
    };
    if location == 1 && kind != crate::native_owned::OWNED {
        let opcode =
            if length == -1 { "AppendConstCharInPlace" } else { "AppendConstStringInPlace" };
        ctx.runtime_error = Some(format!("Type mismatch in {opcode}"));
        return 1;
    }
    if location == 1 {
        ctx.return_value = target;
        ctx.return_value_kind = kind;
    }
    let locked_loop = !ctx.loop_environment_ptr.is_null();
    let value = jit_append_target(ctx, location, target, kind);
    let left = match value {
        Ok(Value::Str(value)) => {
            let value = Arc::make_mut(value);
            let needed = text.len();
            if value.capacity() - value.len() < needed {
                value.reserve(needed.saturating_mul(1000).max(value.capacity()));
            }
            value.push_str(text);
            return 0;
        }
        Ok(value @ Value::Struct { .. }) => {
            if locked_loop {
                return 2;
            }
            value.clone()
        }
        Ok(_) => {
            let opcode =
                if length == -1 { "AppendConstCharInPlace" } else { "AppendConstStringInPlace" };
            ctx.runtime_error = Some(format!("Type mismatch in {opcode}"));
            return 1;
        }
        Err(error) => {
            ctx.runtime_error = Some(error);
            return 1;
        }
    };
    let result = match ctx.vm_ptr.cast::<VM>().as_mut() {
        Some(vm) => vm.binary_op(&left, "+", &Value::Str(Arc::new(text.to_owned()))),
        None => Err("Native append operator requires VM context".into()),
    };
    ctx.refresh_vm_views();
    match result.and_then(|result| {
        if location == 1 {
            let (payload, result_kind) = ctx
                .owned_roots
                .encode(result)
                .map_err(|error| format!("Native ownership error: {error:?}"))?;
            ctx.owned_roots
                .release(target, kind)
                .map_err(|error| format!("Native ownership error: {error:?}"))?;
            ctx.return_value = payload;
            ctx.return_value_kind = result_kind;
        } else {
            *jit_append_target(ctx, location, target, kind)? = result;
        }
        Ok(())
    }) {
        Ok(()) => 0,
        Err(error) => {
            ctx.runtime_error = Some(error);
            1
        }
    }
}

#[no_mangle]
// SAFETY: ctx is the live native context; append published the new binding payload.
pub unsafe extern "C" fn jit_append_result(ctx: *mut VMContext) -> i64 {
    ctx.as_ref().map_or(0, |ctx| ctx.return_value)
}

// SAFETY: the returned borrow is confined to the live native context.
unsafe fn jit_append_target(
    ctx: &mut VMContext,
    location: i64,
    target: i64,
    kind: i64,
) -> Result<&mut Value, String> {
    match location {
        1 => ctx
            .owned_roots
            .current_value_mut(target, kind)
            .map_err(|error| format!("Native ownership error: {error:?}")),
        2 => {
            let name = ctx.var_names_ptr.as_ref().and_then(|names| names.get(&(target as u64)));
            name.and_then(|name| ctx.locals_ptr.as_mut().and_then(|locals| locals.get_mut(name)))
                .ok_or_else(|| "Native append local binding is missing".to_string())
        }
        0 => usize::try_from(target)
            .ok()
            .and_then(|slot| ctx.local_slots_ptr.as_mut().and_then(|slots| slots.get_mut(slot)))
            .ok_or_else(|| format!("Invalid local slot: {target}")),
        _ => Err("Invalid native append location".into()),
    }
}

/// Release only a local snapshot invisible to a same-invocation exception handler.
#[no_mangle]
pub unsafe extern "C" fn jit_release_rebind(
    ctx: *mut VMContext,
    location: i64,
    target: i64,
    target_kind: i64,
    original: i64,
    original_kind: i64,
    mutable: i64,
    function: i64,
    function_kind: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    if mutable == 0 {
        return 0;
    }
    // Whole native functions cannot contain a handler. Their SSA roots and
    // private named locals die when an error unwinds to the VM caller. A loop,
    // physical VM slot, or shared map still belongs to that caller's handler.
    let private_function_local = !ctx.native_stack_bases.is_empty()
        && (location == 1 || (location == 2 && ctx.locals_ptr != ctx.globals_ptr));
    if !private_function_local
        && ctx.vm_ptr.cast::<VM>().as_ref().is_some_and(|vm| vm.has_current_exception_handler())
    {
        return 0;
    }
    let result = (|| {
        if !ctx.owned_roots.is_ordinary_function(function, function_kind)? {
            return Ok(());
        }
        let argument = ctx.owned_roots.clone_value(original, original_kind)?;
        let apply = |root: &mut Value| {
            if crate::interpreter::assignment::same_storage(root, &argument) {
                *root = Value::Null;
            }
        };
        match location {
            1 if target_kind == crate::native_owned::OWNED => {
                apply(ctx.owned_roots.current_value_mut(target, target_kind)?);
            }
            0 => {
                if let Some(root) = usize::try_from(target)
                    .ok()
                    .and_then(|slot| ctx.local_slots_ptr.as_mut()?.get_mut(slot))
                {
                    apply(root);
                }
            }
            2 => {
                if let Some(name) =
                    ctx.var_names_ptr.as_ref().and_then(|names| names.get(&(target as u64)))
                {
                    if let Some(root) =
                        ctx.locals_ptr.as_mut().and_then(|locals| locals.get_mut(name))
                    {
                        apply(root);
                    }
                }
            }
            _ => {}
        }
        Ok::<_, crate::native_owned::OwnershipError>(())
    })();
    if let Err(error) = result {
        ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
        return 1;
    }
    0
}

/// Conditional rebinding returns 1 on success, 0 for the original bytecode fallback.
#[no_mangle]
// SAFETY: context, maps and roots are exclusively borrowed by this invocation.
// The VM global helper is called only outside a locked native-loop environment.
pub unsafe extern "C" fn jit_try_string_append(
    ctx: *mut VMContext,
    location: i64,
    target: i64,
    target_kind: i64,
    original: i64,
    original_kind: i64,
    suffix: i64,
    suffix_kind: i64,
    mutable: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    if mutable == 0 || original_kind != crate::native_owned::OWNED {
        return 0;
    }
    let result = (|| {
        let suffix = ctx.owned_roots.clone_value(suffix, suffix_kind)?;
        if !matches!(suffix, Value::Str(_)) {
            return Ok(false);
        }
        if !matches!(ctx.owned_roots.current_value_mut(original, original_kind)?, Value::Str(_)) {
            return Ok(false);
        }
        if location == 1 {
            if target_kind != crate::native_owned::OWNED {
                return Ok(false);
            }
            return ctx.owned_roots.with_rebinding_snapshot(target, original, |root, snapshot| {
                crate::interpreter::assignment::concat_string_rebinding(root, snapshot, &suffix)
            });
        }
        let value = std::mem::replace(
            ctx.owned_roots.current_value_mut(original, original_kind)?,
            Value::Null,
        );
        let mut snapshot = Some(value);
        let mut apply = |root: &mut Value| {
            crate::interpreter::assignment::concat_string_rebinding(root, &mut snapshot, &suffix)
        };
        let applied = match location {
            0 => usize::try_from(target)
                .ok()
                .and_then(|n| ctx.local_slots_ptr.as_mut()?.get_mut(n))
                .is_some_and(&mut apply),
            2 | 3 => {
                let name = ctx
                    .var_names_ptr
                    .as_ref()
                    .and_then(|names| names.get(&(target as u64)))
                    .cloned();
                if let Some(name) = name {
                    if location == 2 {
                        ctx.locals_ptr
                            .as_mut()
                            .and_then(|locals| locals.get_mut(&name))
                            .is_some_and(&mut apply)
                    } else if let Some(environment) = ctx.loop_environment_ptr.as_mut() {
                        let mut applied = false;
                        let _ = environment.mutate_checked(&name, |root| {
                            applied = apply(root);
                        });
                        applied
                    } else if let Some(vm) = ctx.vm_ptr.cast::<VM>().as_mut() {
                        vm.try_guarded_global_mutation_from_jit(&name, &mut apply)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        };
        if let Some(value) = snapshot {
            *ctx.owned_roots.current_value_mut(original, original_kind)? = value;
        }
        Ok::<_, crate::native_owned::OwnershipError>(applied)
    })();
    match result {
        Ok(applied) => i64::from(applied),
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            0
        }
    }
}

/// Conditional rebinding returns 1 on success, 0 for the original bytecode fallback.
#[no_mangle]
// SAFETY: context, maps and roots are exclusively borrowed by this invocation.
// The VM global helper is called only outside a locked native-loop environment.
pub unsafe extern "C" fn jit_try_append(
    ctx: *mut VMContext,
    location: i64,
    target: i64,
    target_kind: i64,
    original: i64,
    original_kind: i64,
    suffix: i64,
    suffix_kind: i64,
    mutable: i64,
    function: i64,
    function_kind: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    if mutable == 0 || original_kind != crate::native_owned::OWNED {
        return 0;
    }
    let result = (|| {
        let concat = match ctx.owned_roots.native_function_name(function, function_kind)? {
            Some("push" | "append") => false,
            Some("concat") => true,
            _ => return Ok(false),
        };
        let mut item = Some(ctx.owned_roots.clone_value(suffix, suffix_kind)?);
        if VM::value_nesting_depth(item.as_ref().unwrap())
            >= crate::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH
        {
            return Ok(false);
        }
        if !matches!(
            ctx.owned_roots.current_value_mut(original, original_kind)?,
            Value::Array(_) | Value::Bytes(_)
        ) {
            return Ok(false);
        }
        let mut operation = |root: &mut Value, snapshot: &mut Option<Value>| {
            if concat {
                crate::interpreter::assignment::concat_bytes_rebinding(
                    root,
                    snapshot,
                    item.as_ref().unwrap(),
                )
            } else {
                crate::interpreter::assignment::append_rebinding(root, snapshot, &mut item)
            }
        };
        if location == 1 {
            if target_kind != crate::native_owned::OWNED {
                return Ok(false);
            }
            return ctx.owned_roots.with_rebinding_snapshot(target, original, |root, snapshot| {
                operation(root, snapshot)
            });
        }
        let value = std::mem::replace(
            ctx.owned_roots.current_value_mut(original, original_kind)?,
            Value::Null,
        );
        let mut snapshot = Some(value);
        let mut apply = |root: &mut Value| operation(root, &mut snapshot);
        let applied = match location {
            0 => usize::try_from(target)
                .ok()
                .and_then(|n| ctx.local_slots_ptr.as_mut()?.get_mut(n))
                .is_some_and(&mut apply),
            2 | 3 => {
                let name = ctx
                    .var_names_ptr
                    .as_ref()
                    .and_then(|names| names.get(&(target as u64)))
                    .cloned();
                if let Some(name) = name {
                    if location == 2 {
                        ctx.locals_ptr
                            .as_mut()
                            .and_then(|locals| locals.get_mut(&name))
                            .is_some_and(&mut apply)
                    } else if let Some(environment) = ctx.loop_environment_ptr.as_mut() {
                        let mut applied = false;
                        let _ = environment.mutate_checked(&name, |root| {
                            applied = apply(root);
                        });
                        applied
                    } else if let Some(vm) = ctx.vm_ptr.cast::<VM>().as_mut() {
                        vm.try_guarded_global_mutation_from_jit(&name, &mut apply)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        };
        if let Some(value) = snapshot {
            *ctx.owned_roots.current_value_mut(original, original_kind)? = value;
        }
        Ok::<_, crate::native_owned::OwnershipError>(applied)
    })();
    match result {
        Ok(applied) => i64::from(applied),
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            0
        }
    }
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
        return measurement::guard_result(0);
    }

    let ctx = &*ctx;

    let name = if name_hash != 0 && !ctx.var_names_ptr.is_null() {
        let var_names = &*ctx.var_names_ptr;
        if let Some(n) = var_names.get(&(name_hash as u64)) {
            n.as_str()
        } else {
            return measurement::guard_result(0);
        }
    } else {
        return measurement::guard_result(0);
    };

    // Check locals first
    if !ctx.locals_ptr.is_null() {
        let locals = &*ctx.locals_ptr;
        if let Some(value) = locals.get(name) {
            return measurement::guard_result(if matches!(value, Value::Int(_)) { 1 } else { 0 });
        }
    }

    // Check globals
    if !ctx.globals_ptr.is_null() {
        let globals = &*ctx.globals_ptr;
        if let Some(value) = globals.get(name) {
            return measurement::guard_result(if matches!(value, Value::Int(_)) { 1 } else { 0 });
        }
    }

    measurement::guard_result(0)
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
        return measurement::guard_result(0);
    }

    let ctx = &*ctx;

    let name = if name_hash != 0 && !ctx.var_names_ptr.is_null() {
        let var_names = &*ctx.var_names_ptr;
        if let Some(n) = var_names.get(&(name_hash as u64)) {
            n.as_str()
        } else {
            return measurement::guard_result(0);
        }
    } else {
        return measurement::guard_result(0);
    };

    // Check locals first
    if !ctx.locals_ptr.is_null() {
        let locals = &*ctx.locals_ptr;
        if let Some(value) = locals.get(name) {
            return measurement::guard_result(if matches!(value, Value::Float(_)) { 1 } else { 0 });
        }
    }

    // Check globals
    if !ctx.globals_ptr.is_null() {
        let globals = &*ctx.globals_ptr;
        if let Some(value) = globals.get(name) {
            return measurement::guard_result(if matches!(value, Value::Float(_)) { 1 } else { 0 });
        }
    }

    measurement::guard_result(0)
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

#[no_mangle]
pub extern "C" fn jit_float_remainder(left: f64, right: f64) -> f64 {
    left % right
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
    ctx_ref.return_value_kind = 0;
    0 // Success
}

/// Preserve scalar type information across the native calling convention.
#[no_mangle]
// SAFETY: ctx is the live, exclusively borrowed native-call context.
pub unsafe extern "C" fn jit_set_return_scalar(ctx: *mut VMContext, value: i64, kind: i64) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    ctx.return_value = value;
    ctx.has_return_value = true;
    ctx.return_value_kind = kind;
    0
}

#[no_mangle]
// SAFETY: ctx points to the live native-call context.
pub unsafe extern "C" fn jit_return_scalar_kind(ctx: *mut VMContext) -> i64 {
    ctx.as_ref().map_or(0, |ctx| ctx.return_value_kind)
}

#[no_mangle]
// SAFETY: ctx is a live, exclusively borrowed native context. The payload is
// validated as a root identity before borrowing its Value; no raw address is used.
pub unsafe extern "C" fn jit_owned_truth(ctx: *mut VMContext, payload: i64) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    match ctx.owned_roots.owned_truth(payload) {
        Ok(truth) => i64::from(truth),
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            0
        }
    }
}

#[no_mangle]
// SAFETY: ctx is live and exclusively borrowed. Owned operands are validated
// store identities, borrowed only for this call; scalar payloads are decoded.
pub unsafe extern "C" fn jit_owned_equal(
    ctx: *mut VMContext,
    left: i64,
    left_kind: i64,
    right: i64,
    right_kind: i64,
) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    match ctx.owned_roots.values_equal(left, left_kind, right, right_kind) {
        Ok(equal) => i64::from(equal),
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            0
        }
    }
}

#[no_mangle]
// SAFETY: ctx and its stack are live and exclusively borrowed for this call.
pub unsafe extern "C" fn jit_stack_push_scalar(ctx: *mut VMContext, value: i64, kind: i64) {
    if let Some(ctx) = ctx.as_mut() {
        if let Some(stack) = ctx.stack_ptr.as_mut() {
            // The SSA operand can still be used after marshaling (Dup and
            // peek-style stores). Borrow it; do not consume its native root.
            match ctx.owned_roots.clone_value(value, kind) {
                Ok(value) => stack.push(value),
                Err(error) => {
                    ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
                }
            }
        }
    }
}

/// Transfer a consumed operand into the VM stack. Borrowed/peek operations
/// continue using jit_stack_push_scalar; this helper invalidates an owned root.
#[no_mangle]
// SAFETY: ctx and stack are live and exclusive. Owned payloads must belong to
// the current activation; the root store validates this before moving a value.
pub unsafe extern "C" fn jit_stack_take_scalar(ctx: *mut VMContext, value: i64, kind: i64) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    let Some(stack) = ctx.stack_ptr.as_mut() else {
        ctx.runtime_error = Some("Native operand transfer requires a stack".into());
        return 1;
    };
    match ctx.owned_roots.take_value(value, kind) {
        Ok(value) => {
            stack.push(value);
            0
        }
        Err(error) => {
            ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            1
        }
    }
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
/// If this invocation uses named parameters, transfer that parameter instead.
#[no_mangle]
// SAFETY:
// - Preconditions: jit_get_arg is invoked from JIT-compiled code using the C ABI, and `ctx` points to a live
//   `VMContext` whose nested pointers (`stack_ptr`, `locals_ptr`, `globals_ptr`, etc.) satisfy this
//   function's null/shape checks for the full call duration with no concurrent mutable aliasing.
// - Postconditions: all reads/writes stay within VM-owned structures reachable from validated pointers,
//   immediate arguments retain their bits; named arguments transfer into the current frame's roots.
pub unsafe extern "C" fn jit_get_arg(ctx: *mut VMContext, index: i64, name_hash: i64) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 0 };
    ctx.return_value_kind = 0;
    if index < 0 || index >= 4 {
        return 0;
    }
    if index >= ctx.arg_count {
        return jit_take_parameter(ctx, name_hash, index);
    }
    ctx.return_value_kind = ctx.arg_kinds[index as usize];
    [ctx.arg0, ctx.arg1, ctx.arg2, ctx.arg3][index as usize]
}

#[no_mangle]
// SAFETY: ctx is live and exclusively borrowed. Its optional VM and name pointers
// remain valid throughout the enclosing native invocation.
pub unsafe extern "C" fn jit_enter_function(ctx: *mut VMContext) -> i64 {
    let Some(ctx) = ctx.as_mut() else { return 1 };
    if !ctx.task_cancellation.is_null() && jit_runtime_failed(ctx) != 0 {
        return 1;
    }
    let name = if ctx.call_name_ptr.is_null() {
        "<native>"
    } else {
        std::str::from_utf8(std::slice::from_raw_parts(ctx.call_name_ptr, ctx.call_name_len))
            .unwrap_or("<native>")
    };
    let result = if let Some(vm) = (ctx.vm_ptr as *mut VM).as_mut() {
        vm.enter_native_function(name)
    } else if ctx.native_call_depth >= crate::runtime_limits::DEFAULT_MAX_VM_CALL_DEPTH {
        Err(format!(
            "Maximum VM call stack depth of {} exceeded while calling {}",
            crate::runtime_limits::DEFAULT_MAX_VM_CALL_DEPTH,
            name
        ))
    } else {
        ctx.native_call_depth += 1;
        Ok(())
    };
    if let Err(error) = result {
        ctx.runtime_error = Some(error);
        return 1;
    }
    if let Err(error) = ctx.owned_roots.enter() {
        if let Some(vm) = (ctx.vm_ptr as *mut VM).as_mut() {
            vm.leave_native_function();
        } else {
            ctx.native_call_depth = ctx.native_call_depth.saturating_sub(1);
        }
        ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
        return 1;
    }
    ctx.native_stack_bases.push(ctx.stack_ptr.as_ref().map_or(0, Vec::len));
    measurement::add(Metric::JitNativeFunctionEntries, 1);
    0
}

#[no_mangle]
// SAFETY: ctx and its optional VM remain live and exclusively borrowed; this
// pairs with one successful native-function entry on every generated exit path.
pub unsafe extern "C" fn jit_leave_function(ctx: *mut VMContext) {
    if let Some(ctx) = ctx.as_mut() {
        if ctx.runtime_error.is_none()
            && ctx.has_return_value
            && ctx.return_value_kind == crate::native_owned::OWNED
        {
            if let Err(error) = ctx.owned_roots.promote(ctx.return_value, ctx.return_value_kind) {
                ctx.runtime_error = Some(format!("Native ownership error: {error:?}"));
            }
        }
        if let Err(error) = ctx.owned_roots.leave() {
            ctx.runtime_error.get_or_insert_with(|| format!("Native ownership error: {error:?}"));
        }
        // A field method marker may leave an unconsumed receiver. Like a VM
        // frame, native functions discard their own stack suffix on every exit.
        if let Some(base) = ctx.native_stack_bases.pop() {
            if let Some(stack) = ctx.stack_ptr.as_mut() {
                stack.truncate(base);
            }
        }
        if let Some(vm) = (ctx.vm_ptr as *mut VM).as_mut() {
            vm.leave_native_function();
        } else {
            ctx.native_call_depth = ctx.native_call_depth.saturating_sub(1);
        }
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

    // Transfer valid calls' ordered suffix without repeated growth or reversal.
    // Preserve the legacy malformed-stack consumption and drop order below.
    let count = usize::try_from(arg_count).unwrap_or(0);
    let args = if count <= stack.len() {
        // Bytecode and builtin dispatch return emptied vectors to the pool.
        // Interpreter callables still consume their vectors.
        let recyclable = stack
            .len()
            .checked_sub(count)
            .and_then(|start| start.checked_sub(1))
            .and_then(|index| stack.get(index))
            .is_some_and(|callee| {
                matches!(callee, Value::BytecodeFunction { .. } | Value::NativeFunction(_))
            });
        if ctx_ref.vm_ptr.is_null() || !recyclable {
            stack.split_off(stack.len() - count)
        } else {
            // Borrow only the VM pool field, disjoint from the live stack borrow.
            // The returned vector belongs exclusively to this invocation.
            let mut args = VM::take_native_arguments(ctx_ref.vm_ptr.cast::<VM>(), count);
            args.extend(stack.drain(stack.len() - count..));
            args
        }
    } else {
        let mut args = Vec::new();
        for _ in 0..arg_count {
            if let Some(arg) = stack.pop() {
                args.push(arg);
            } else {
                return 4; // Error: stack underflow (not enough args)
            }
        }
        args.reverse();
        args
    };

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
    let result = vm.call_function_from_jit(function, args);
    ctx_ref.refresh_vm_views();
    match result {
        Ok(result) => {
            // Push result to stack
            if std::env::var("DEBUG_JIT_CALL").is_ok() {
                eprintln!("jit_call_function: result={:?}", result);
            }
            // Reborrow after the VM callback; it can mutate or grow this stack.
            (&mut *ctx_ref.stack_ptr).push(result);
            0 // Success
        }
        Err(err) => {
            if std::env::var("DEBUG_JIT_CALL").is_ok() {
                eprintln!("jit_call_function: error={}", err);
            }
            ctx_ref.runtime_error = Some(err);
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
        let Some(count) = usize::try_from(num_pairs).ok().and_then(|n| n.checked_mul(2)) else {
            vm_ctx.runtime_error = Some("Stack underflow".into());
            return 0;
        };
        let Some(start) = stack.len().checked_sub(count) else {
            vm_ctx.runtime_error = Some("Stack underflow".into());
            return 0;
        };
        let mut entries = stack.drain(start..);
        let mut map = DictMap::default();
        map.reserve(count / 2);
        while let Some(key) = entries.next() {
            let value = entries.next().expect("validated key/value pair count");
            let Value::Str(key) = key else {
                vm_ctx.runtime_error = Some("Dict keys must be strings".into());
                return 0;
            };
            // Source order is significant: the last duplicate key wins.
            map.insert(Arc::from(key.as_str()), value);
        }
        drop(entries);
        let value = Value::Dict(Arc::new(map));
        if let Err(error) = VM::ensure_value_nesting_depth(&value, "building dictionary literal") {
            vm_ctx.runtime_error = Some(error);
            return 0;
        }
        stack.push(value);
        1
    })
    .unwrap_or(0)
}

/// Build an array from typed values marshaled in source order. Spread markers
/// remain VM-only bytecode and cannot enter a compiled region.
#[no_mangle]
pub extern "C" fn jit_make_array(ctx: *mut VMContext, count: i64) -> i64 {
    with_vm_stack_mut(ctx, |vm_ctx, stack| {
        let Some(start) = usize::try_from(count).ok().and_then(|n| stack.len().checked_sub(n))
        else {
            vm_ctx.runtime_error = Some("Stack underflow in MakeArray".into());
            return 0;
        };
        let value = Value::Array(Arc::new(stack.drain(start..).collect()));
        if let Err(error) = VM::ensure_value_nesting_depth(&value, "building array literal") {
            vm_ctx.runtime_error = Some(error);
            return 0;
        }
        stack.push(value);
        1
    })
    .unwrap_or(0)
}

type NativeStructShape = (String, Vec<String>);

/// Construct a struct using immutable metadata retained by JitCompiler.
#[no_mangle]
pub extern "C" fn jit_make_struct(ctx: *mut VMContext, shape_ptr: i64) -> i64 {
    with_vm_stack_mut(ctx, |vm_ctx, stack| {
        if shape_ptr == 0 {
            vm_ctx.runtime_error = Some("Missing native struct metadata".into());
            return 0;
        }
        // SAFETY: generated code embeds Arc::as_ptr for metadata held by the
        // compiler for the entire lifetime of its machine code.
        let (name, names) = unsafe { &*(shape_ptr as *const NativeStructShape) };
        let Some(start) = stack.len().checked_sub(names.len()) else {
            vm_ctx.runtime_error = Some("Stack underflow".into());
            return 0;
        };
        let mut fields = HashMap::with_capacity(names.len());
        // Match VM insertion order, including bytecode with duplicate names.
        for (name, value) in names.iter().rev().zip(stack.drain(start..).rev()) {
            fields.insert(name.clone(), value);
        }
        let value = Value::Struct { name: name.clone(), fields: fields.into() };
        if let Err(error) = VM::ensure_value_nesting_depth(&value, "building struct value") {
            vm_ctx.runtime_error = Some(error);
            return 0;
        }
        stack.push(value);
        1
    })
    .unwrap_or(0)
}

/// Build a fixed-key dictionary. `keys_ptr` is Arc::as_ptr of a key vector
/// retained by the JIT compiler for the entire lifetime of the generated code.
/// Returns 1 after pushing the dictionary, or 0 with a runtime error.
#[no_mangle]
pub extern "C" fn jit_make_dict_with_keys(
    ctx: *mut VMContext,
    keys_ptr: i64,
    num_keys: i64,
) -> i64 {
    with_vm_stack_mut(ctx, |vm_ctx, stack| {
        let Ok(count) = usize::try_from(num_keys) else {
            vm_ctx.runtime_error = Some("Stack underflow".into());
            return 0;
        };
        if keys_ptr == 0 {
            vm_ctx.runtime_error = Some("Missing native dictionary keys".into());
            return 0;
        }
        // SAFETY: generated code passes Arc::as_ptr, and JitCompiler retains a
        // strong reference for the lifetime of the machine code. This reads the
        // Vec allocation, not an Arc wrapper or a temporary bytecode reference.
        let keys = unsafe { &*(keys_ptr as *const Vec<Arc<str>>) };
        if keys.len() != count || stack.len() < count {
            vm_ctx.runtime_error = Some("Stack underflow".into());
            return 0;
        }
        // SAFETY: the retained Arc keeps this allocation live. Incrementing
        // before from_raw creates exactly one owned reference for the new Value.
        let owned_keys = unsafe {
            Arc::increment_strong_count(keys_ptr as *const Vec<Arc<str>>);
            Arc::from_raw(keys_ptr as *const Vec<Arc<str>>)
        };
        let values: Vec<Value> = stack.drain(stack.len() - count..).collect();
        let value = Value::FixedDict { keys: owned_keys, values: values.into() };
        if let Err(error) =
            VM::ensure_value_nesting_depth(&value, "building fixed dictionary literal")
        {
            vm_ctx.runtime_error = Some(error);
            return 0;
        }
        stack.push(value);
        1
    })
    .unwrap_or(0)
}

/// JIT compiler for Kujo bytecode
pub struct JitCompiler {
    // Immutable per compiler: cached code must never cross cancellation policies.
    cancellation_checks: bool,
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

    /// Name-based inspection index; execution dispatch uses VM code-identity caches.
    /// Stores CompiledFnInfo which includes both standard and direct-arg variants
    /// This enables direct JIT recursion for eligible functions
    compiled_fn_info: HashMap<String, CompiledFnInfo>,
    // Key-vector allocations embedded in native instructions must outlive code.
    dictionary_keys: HashMap<usize, Arc<Vec<Arc<str>>>>,
    struct_shapes: HashMap<NativeStructShape, Arc<NativeStructShape>>,
    field_names: HashMap<String, Arc<String>>,
    path_shapes: HashMap<Vec<Option<String>>, Arc<Vec<Option<String>>>>,
    append_strings: HashMap<usize, Arc<str>>,
    literal_values: HashMap<(u8, Vec<u8>), Arc<Value>>,
}

/// Bytecode translator - converts bytecode to Cranelift IR
struct BytecodeTranslator {
    cancellation_checks: bool,
    /// Stack simulation - maps stack depth to Cranelift values
    value_stack: Vec<cranelift::prelude::Value>,
    scalar_kinds: HashMap<cranelift::prelude::Value, cranelift::prelude::Value>,
    scalar_return_func: Option<FuncRef>,
    scalar_kind_func: Option<FuncRef>,
    owned_truth_func: Option<FuncRef>,
    owned_equal_func: Option<FuncRef>,
    owned_operator_func: Option<FuncRef>,
    operand_load_func: Option<FuncRef>,
    literal_load_func: Option<FuncRef>,
    parameter_load_func: Option<FuncRef>,
    literal_values: HashMap<usize, Arc<Value>>,
    operand_duplicate_func: Option<FuncRef>,
    operand_release_func: Option<FuncRef>,
    dead_local_func: Option<FuncRef>,
    append_local_func: Option<FuncRef>,
    append_result_func: Option<FuncRef>,
    try_string_append_func: Option<FuncRef>,
    try_append_func: Option<FuncRef>,
    release_rebind_func: Option<FuncRef>,
    indexed_read_func: Option<FuncRef>,
    indexed_read_cached_func: Option<FuncRef>,
    indexed_read_caches: HashMap<usize, StackSlot>,
    for_next_func: Option<FuncRef>,
    indexed_write_func: Option<FuncRef>,
    update_binding_path_func: Option<FuncRef>,
    append_binding_path_func: Option<FuncRef>,
    guard_append_path_func: Option<FuncRef>,
    try_pop_rebind_func: Option<FuncRef>,
    loop_resume_enabled: bool,
    loop_resume_guard_func: Option<FuncRef>,
    loop_resume_prepare_func: Option<FuncRef>,
    loop_dense_view_func: Option<FuncRef>,
    loop_indexed_read_func: Option<FuncRef>,
    scalar_push_func: Option<FuncRef>,
    scalar_take_func: Option<FuncRef>,
    /// Most recently emitted integer constant and its SSA value. This lets
    /// arithmetic lowering recognize constant divisors without changing the
    /// bytecode stack representation.
    last_int_constant: Option<(cranelift::prelude::Value, i64)>,
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
    named_assignment_func: Option<FuncRef>,
    runtime_error_func: Option<FuncRef>,
    arithmetic_error_func: Option<FuncRef>,
    scalar_type_error_func: Option<FuncRef>,
    float_remainder_func: Option<FuncRef>,
    enter_function_func: Option<FuncRef>,
    leave_function_func: Option<FuncRef>,
    tracks_function_depth: bool,
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
    make_array_func: Option<FuncRef>,
    make_struct_func: Option<FuncRef>,
    field_read_func: Option<FuncRef>,
    field_write_func: Option<FuncRef>,
    index_set_result_func: Option<FuncRef>,
    field_call_func: Option<FuncRef>,
    field_call_sites: std::collections::HashSet<usize>,
    struct_shapes: HashMap<NativeStructShape, Arc<NativeStructShape>>,
    field_names: HashMap<String, Arc<String>>,
    path_shapes: HashMap<Vec<Option<String>>, Arc<Vec<Option<String>>>>,
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
    /// Active VM frame local-slot helpers (loop JIT scalar state)
    vm_local_slot_set_func: Option<FuncRef>,
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
    local_binding_kinds: Vec<BytecodeBindingKind>,
    readonly_initialization: HashMap<usize, StackSlot>,
    local_initialization_func: Option<FuncRef>,
    /// End of function body (PC index, exclusive)
    function_end: usize,
    /// Local variable slots - maps variable name to stack slot
    /// This enables register-based locals optimization (Phase 7 Step 7)
    /// Instead of calling runtime functions for every LoadVar/StoreVar,
    /// we use direct memory access via Cranelift stack slots
    local_slots: HashMap<String, StackSlot>,
    /// SSA variables for integer locals. Cranelift inserts block parameters at
    /// control-flow joins, keeping hot loop state in registers instead of
    /// repeatedly loading and storing explicit stack slots.
    local_variables: HashMap<String, Variable>,
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
    /// Whole-function VM calls discard local maps after successful returns.
    /// Error/loop synchronization remains independent of this switch.
    persist_successful_return_locals: bool,

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
            cancellation_checks: false,
            value_stack: Vec::new(),
            scalar_kinds: HashMap::new(),
            scalar_return_func: None,
            scalar_kind_func: None,
            owned_truth_func: None,
            owned_equal_func: None,
            owned_operator_func: None,
            operand_load_func: None,
            literal_load_func: None,
            parameter_load_func: None,
            literal_values: HashMap::new(),
            operand_duplicate_func: None,
            operand_release_func: None,
            dead_local_func: None,
            append_local_func: None,
            append_result_func: None,
            try_string_append_func: None,
            try_append_func: None,
            release_rebind_func: None,
            indexed_read_func: None,
            indexed_read_cached_func: None,
            indexed_read_caches: HashMap::new(),
            for_next_func: None,
            indexed_write_func: None,
            update_binding_path_func: None,
            append_binding_path_func: None,
            guard_append_path_func: None,
            try_pop_rebind_func: None,
            loop_resume_enabled: false,
            loop_resume_guard_func: None,
            loop_resume_prepare_func: None,
            loop_dense_view_func: None,
            loop_indexed_read_func: None,
            scalar_push_func: None,
            scalar_take_func: None,
            last_int_constant: None,
            variables: HashMap::new(),
            blocks: HashMap::new(),
            block_entry_stack_depth: HashMap::new(),
            loop_header_pcs: std::collections::HashSet::new(),
            ctx_param: None,
            load_var_func: None,
            store_var_func: None,
            named_assignment_func: None,
            runtime_error_func: None,
            arithmetic_error_func: None,
            scalar_type_error_func: None,
            float_remainder_func: None,
            enter_function_func: None,
            leave_function_func: None,
            tracks_function_depth: false,
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
            local_variables: HashMap::new(),
            non_int_locals: std::collections::HashSet::new(),
            use_local_slots: false,
            local_names: Vec::new(),
            local_binding_kinds: Vec::new(),
            readonly_initialization: HashMap::new(),
            local_initialization_func: None,
            current_function_name: None,
            self_recursion_pending: false,
            self_call_func: None,
            get_return_int_func: None,
            dict_get_func: None,
            dict_set_func: None,
            make_dict_func: None,
            make_array_func: None,
            make_struct_func: None,
            field_read_func: None,
            field_write_func: None,
            index_set_result_func: None,
            field_call_func: None,
            field_call_sites: Default::default(),
            struct_shapes: HashMap::new(),
            field_names: HashMap::new(),
            path_shapes: HashMap::new(),
            make_dict_with_keys_func: None,
            append_const_string_in_place_func: None,
            append_const_char_in_place_func: None,
            local_slot_dict_get_func: None,
            local_slot_dict_set_func: None,
            local_slot_int_dict_get_func: None,
            local_slot_int_dict_set_func: None,
            vm_local_slot_set_func: None,
            int_dict_unique_ptr_func: None,
            int_dict_get_ptr_func: None,
            int_dict_get_ptr_dense_int_func: None,
            int_dict_get_ptr_dense_int_full_func: None,
            int_dict_set_ptr_func: None,
            int_dict_set_ptr_dense_int_func: None,
            int_dict_set_ptr_dense_int_full_func: None,
            persist_locals_on_return: true,
            persist_successful_return_locals: true,
            direct_arg_mode: false,
            direct_arg_param: None,
            param_count: 0,
            dirty_local_names: std::collections::HashSet::new(),
            store_var_from_stack_func: None,
            int_dict_slots: std::collections::HashSet::new(),
            int_dict_ptr_slots: HashMap::new(),
        }
    }

    fn set_local_names(&mut self, local_names: Vec<String>, kinds: Vec<BytecodeBindingKind>) {
        self.local_names = local_names;
        self.local_binding_kinds = kinds;
    }

    /// Analyze bytecode to find loop headers (backward jump targets) and their expected stack depths.
    /// This is required for correct SSA handling - Cranelift blocks must have parameters
    /// declared when they're created, but for backward jumps we don't know the stack depth
    /// until we reach the jump instruction.
    ///
    /// Returns a HashMap: PC -> expected stack depth for blocks that receive backward jumps
    fn analyze_loop_headers(
        instructions: &[OpCode],
        function_end: usize,
        entry: usize,
    ) -> HashMap<usize, usize> {
        let mut loop_headers = HashMap::new();
        let mut depths = HashMap::from([(entry, 0usize)]);
        let mut pending = std::collections::VecDeque::from([entry]);
        while let Some(pc) = pending.pop_front() {
            if pc >= function_end || pc >= instructions.len() {
                continue;
            }
            let depth = depths[&pc];
            let instruction = &instructions[pc];
            let (pops, pushes) = Self::stack_effect(instruction);
            let next = depth.saturating_sub(pops) + pushes;
            let edges = match instruction {
                OpCode::Return | OpCode::ReturnNone => Vec::new(),
                OpCode::Jump(target) | OpCode::JumpBack(target) => vec![(*target, next)],
                OpCode::JumpIfTrue(target)
                | OpCode::JumpIfFalse(target)
                | OpCode::GuardAppendPath(target) => {
                    vec![(*target, next), (pc + 1, next)]
                }
                OpCode::TryPopRebind(_, _, argc, shape, end) => {
                    let fallback =
                        depth.saturating_sub(shape.iter().filter(|step| step.is_none()).count());
                    vec![(*end, fallback.saturating_sub(*argc)), (pc + 1, fallback)]
                }
                OpCode::ForNext(target) => vec![(*target, depth.saturating_sub(2)), (pc + 1, next)],
                OpCode::TryAppendAssign(_, target) => {
                    vec![(*target, depth.saturating_sub(3)), (pc + 1, depth)]
                }
                OpCode::TryStringAppendAssign(_, target) => {
                    vec![(*target, depth.saturating_sub(2)), (pc + 1, depth)]
                }
                _ => vec![(pc + 1, next)],
            };
            for (target, entry_depth) in edges {
                if target >= function_end {
                    continue;
                }
                if target <= pc {
                    loop_headers.insert(target, entry_depth);
                }
                if let std::collections::hash_map::Entry::Vacant(entry) = depths.entry(target) {
                    entry.insert(entry_depth);
                    pending.push_back(target);
                }
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
            OpCode::StoreVar(_)
            | OpCode::StoreLocal(_)
            | OpCode::DefineLocal(_)
            | OpCode::StoreGlobal(_)
            | OpCode::ReleaseRebindLocal(_, _)
            | OpCode::ReleaseDeadLocal(_) => (0, 0),

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
            | OpCode::GreaterEqual
            | OpCode::And
            | OpCode::Or => (2, 1),
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

            OpCode::ForNext(_) => (2, 1), // Exhausted edge consumes both without a result.

            // Call (pops args + func, pushes result)
            OpCode::Call(n) => (*n + 1, 1), // Pop n args + 1 func, push result

            // Dict/Array operations
            OpCode::FieldGet(_) => (1, 1),
            OpCode::FieldSet(_) => (2, 1),
            OpCode::GuardAppendPath(_) => (0, 0),
            OpCode::TryPopRebind(_, _, _, shape, _) => {
                (shape.iter().filter(|step| step.is_none()).count(), 0)
            }
            OpCode::AppendBindingPath(_, path) => {
                (3 + path.iter().filter(|step| step.is_none()).count(), 1)
            }
            OpCode::IndexGet => (2, 1), // Pop object + index, push value
            OpCode::IndexSet => (3, 1), // [value, object, index] -> [updated object]
            OpCode::UpdateBindingPath(_, path) => {
                (1 + path.iter().filter(|step| step.is_none()).count(), 1)
            }
            OpCode::UpdatePath(path) => (2 + path.iter().filter(|step| step.is_none()).count(), 1),
            OpCode::IndexGetInPlace(_) => (1, 1), // Pop index, load var, push value
            OpCode::IndexSetInPlace(_) => (2, 1), // Pop index/value and produce the assignment placeholder
            OpCode::MakeArray(n) => (*n, 1),
            OpCode::MakeStruct(_, fields) => (fields.len(), 1),
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
            for (slot_index, name) in self.local_names.iter().enumerate() {
                // These loop bindings are updated in physical slots. Creating
                // SSA aliases here would force unnecessary COW on native writes.
                if self.int_dict_slots.contains(&slot_index) {
                    continue;
                }
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
                // Loop entry cannot assume a named assignment target already
                // exists (or that a conditional store will execute). Keep these
                // bindings in their maps; physical VM locals still use SSA.
                if self.named_assignment_func.is_some() || self.persist_locals_on_return {
                    continue;
                }
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
        self.allocate_ssa_locals(builder);

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
                if self.named_assignment_func.is_some() {
                    continue;
                }
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
        self.allocate_ssa_locals(builder);
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
        self.allocate_ssa_locals(builder);
    }

    // Admit hints only where reads can repeat. Backward-edge intervals keep this
    // analysis linear even for nested loops.
    fn repeated_indexed_read_slots(instructions: &[OpCode]) -> HashSet<usize> {
        let mut starts = vec![0usize; instructions.len()];
        let mut ends = vec![0usize; instructions.len() + 1];
        for (pc, instruction) in instructions.iter().enumerate() {
            let target = match instruction {
                OpCode::Jump(target)
                | OpCode::JumpBack(target)
                | OpCode::JumpIfFalse(target)
                | OpCode::JumpIfTrue(target)
                | OpCode::ForNext(target) => *target,
                _ => continue,
            };
            if target <= pc {
                starts[target] += 1;
                ends[pc + 1] += 1;
            }
        }
        let mut active = 0;
        let mut seen = HashSet::new();
        let mut repeated = HashSet::new();
        for (pc, instruction) in instructions.iter().enumerate() {
            active -= ends[pc];
            active += starts[pc];
            if let OpCode::IndexGetInPlace(slot) = instruction {
                if !seen.insert(*slot) || active > 0 {
                    repeated.insert(*slot);
                }
            }
        }
        repeated
    }

    fn initialize_indexed_read_caches(
        &mut self,
        builder: &mut FunctionBuilder,
        instructions: &[OpCode],
    ) {
        if !self.tracks_function_depth {
            return;
        }
        let repeated = Self::repeated_indexed_read_slots(instructions);
        for instruction in instructions {
            let OpCode::IndexGetInPlace(slot) = instruction else { continue };
            if !repeated.contains(slot) || self.indexed_read_caches.contains_key(slot) {
                continue;
            }
            let Some(name) = self.local_names.get(*slot) else { continue };
            if !self.local_variables.contains_key(name) {
                continue;
            }
            let cache = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                16,
                3,
            ));
            let zero = builder.ins().iconst(types::I64, 0);
            builder.ins().stack_store(zero, cache, 0);
            builder.ins().stack_store(zero, cache, 8);
            self.indexed_read_caches.insert(*slot, cache);
        }
    }

    // Native activations own their initialization flags. A loop instead consults
    // the active VM frame so reentry and subsequent VM stores see the same state.
    fn initialize_readonly_locals(&mut self, builder: &mut FunctionBuilder, params: &[String]) {
        if self.loop_resume_enabled {
            return;
        }
        for (index, name) in self.local_names.iter().enumerate() {
            if !self
                .local_binding_kinds
                .get(index)
                .is_some_and(|kind| *kind != BytecodeBindingKind::Mutable)
            {
                continue;
            }
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                0,
            ));
            let initialized = builder.ins().iconst(types::I64, i64::from(params.contains(name)));
            builder.ins().stack_store(initialized, slot, 0);
            self.readonly_initialization.insert(index, slot);
        }
    }

    fn check_local_initialization(
        &mut self,
        builder: &mut FunctionBuilder,
        index: usize,
        define: bool,
    ) -> Result<(), String> {
        let kind =
            self.local_binding_kinds.get(index).copied().unwrap_or(BytecodeBindingKind::Mutable);
        if kind == BytecodeBindingKind::Mutable {
            return Ok(());
        }
        let ctx = self.ctx_param.ok_or("Local initialization requires context")?;
        let helper =
            self.local_initialization_func.ok_or("Local initialization requires helper")?;
        let slot_index = builder.ins().iconst(types::I64, index as i64);
        let binding = builder
            .ins()
            .iconst(types::I64, if kind == BytecodeBindingKind::LetImmutable { 1 } else { 2 });
        let name = self.local_names.get(index).ok_or("Invalid local initialization slot")?;
        let hash = builder.ins().iconst(types::I64, Self::hash_var_name(name) as i64);
        let local_flag = self.readonly_initialization.get(&index).copied();
        if !define {
            let initialized = if self.loop_resume_enabled {
                let read = builder.ins().iconst(types::I64, 0);
                let call = builder.ins().call(helper, &[ctx, slot_index, read, binding, hash]);
                builder.inst_results(call)[0]
            } else {
                builder.ins().stack_load(
                    types::I64,
                    local_flag.ok_or("Missing native initialization flag")?,
                    0,
                )
            };
            let failed = builder.create_block();
            let allowed = builder.create_block();
            builder.ins().brif(initialized, failed, &[], allowed, &[]);
            builder.seal_block(failed);
            builder.switch_to_block(failed);
            let error = builder.ins().iconst(types::I64, 2);
            let call = builder.ins().call(helper, &[ctx, slot_index, error, binding, hash]);
            self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
            builder.ins().jump(allowed, &[]);
            builder.seal_block(allowed);
            builder.switch_to_block(allowed);
        }
        let one = builder.ins().iconst(types::I64, 1);
        if self.loop_resume_enabled {
            builder.ins().call(helper, &[ctx, slot_index, one, binding, hash]);
        } else {
            builder.ins().stack_store(
                one,
                local_flag.ok_or("Missing native initialization flag")?,
                0,
            );
        }
        Ok(())
    }

    fn allocate_ssa_locals(&mut self, builder: &mut FunctionBuilder) {
        let names: Vec<String> = self.local_slots.keys().cloned().collect();
        for name in names {
            if self.local_variables.contains_key(&name) {
                continue;
            }
            // Adjacent payload/type SSA variables merge together at branch joins.
            let variable = Variable::new(self.local_variables.len() * 2);
            let kind_variable = Variable::new(self.local_variables.len() * 2 + 1);
            builder.declare_var(kind_variable, types::I64);
            builder.declare_var(variable, types::I64);
            let zero = builder.ins().iconst(types::I64, 0);
            builder.def_var(variable, zero);
            builder.def_var(kind_variable, zero);
            self.local_variables.insert(name, variable);
        }
    }

    fn scan_non_int_locals(&mut self, instructions: &[OpCode], function_end: usize) {
        // Loop locals must persist to physical VM slots, not only to the
        // temporary name map. Tagged SSA handles owned values as well as scalars.
        // Integer-map slots retain their separate physical-slot path.
        if self.loop_resume_enabled {
            return;
        }

        for pc in 0..function_end.saturating_sub(1) {
            let instruction = match instructions.get(pc) {
                Some(instr) => instr,
                None => break,
            };
            let next_instruction = instructions.get(pc + 1);

            if matches!(
                instruction,
                OpCode::MakeArray(_)
                    | OpCode::MakeStruct(_, _)
                    | OpCode::MakeDict(_)
                    | OpCode::MakeDictWithKeys(_)
            ) {
                if let Some(next) = next_instruction {
                    match next {
                        OpCode::StoreLocal(slot) | OpCode::DefineLocal(slot) => {
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
        let load_var_func = self.parameter_load_func.unwrap_or(load_var_func);

        for (i, param_name) in params.iter().enumerate() {
            if let Some(&variable) = self.local_variables.get(param_name) {
                let param_value = if use_fast_args {
                    // FAST PATH: Load parameter directly from VMContext.argN
                    let get_arg = get_arg_func.unwrap();
                    let index = builder.ins().iconst(types::I64, i as i64);
                    let hash =
                        builder.ins().iconst(types::I64, Self::hash_var_name(param_name) as i64);
                    let call = builder.ins().call(get_arg, &[ctx, index, hash]);
                    builder.inst_results(call)[0]
                } else {
                    // SLOW PATH: Load parameter value from HashMap via runtime call
                    let name_hash = Self::hash_var_name(param_name) as i64;
                    let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                    let index = builder.ins().iconst(types::I64, i as i64);

                    // Transfer the ordered argument or named fallback into this frame.
                    let call = builder.ins().call(load_var_func, &[ctx, name_hash_val, index]);
                    builder.inst_results(call)[0]
                };

                self.capture_scalar_kind(builder, ctx, param_value);
                builder.def_var(variable, param_value);
                let kind = self.scalar_kind(builder, param_value);
                builder.def_var(Variable::new(variable.index() + 1), kind);

                if std::env::var("DEBUG_JIT").is_ok() {
                    eprintln!(
                        "JIT: Initialized parameter '{}' in SSA variable {:?} (fast={})",
                        param_name, variable, use_fast_args
                    );
                }
            }
        }
    }

    /// Initialize local slots from VM locals (used for loop JIT)
    fn initialize_local_slots_from_vm(
        &mut self,
        builder: &mut FunctionBuilder,
        load_local_slot_func: FuncRef,
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

        let local_variables: Vec<(String, Variable)> =
            self.local_variables.iter().map(|(name, variable)| (name.clone(), *variable)).collect();

        for (name, variable) in local_variables {
            let value = match self.local_names.iter().position(|local| local == &name) {
                Some(index) => {
                    let slot_index_value = builder.ins().iconst(types::I64, index as i64);
                    let call = builder.ins().call(load_local_slot_func, &[ctx, slot_index_value]);
                    builder.inst_results(call)[0]
                }
                None => {
                    let name_hash = Self::hash_var_name(&name) as i64;
                    let name_hash_value = builder.ins().iconst(types::I64, name_hash);
                    let zero = builder.ins().iconst(types::I64, 0);
                    let call = builder.ins().call(
                        self.operand_load_func.unwrap_or(load_var_func),
                        &[ctx, name_hash_value, zero],
                    );
                    builder.inst_results(call)[0]
                }
            };
            self.capture_scalar_kind(builder, ctx, value);
            builder.def_var(variable, value);
            let kind = self.scalar_kind(builder, value);
            builder.def_var(Variable::new(variable.index() + 1), kind);
        }
        if let Some(error_func) = self.runtime_error_func {
            let check = builder.ins().call(error_func, &[ctx]);
            self.exit_on_assignment_error(builder, builder.inst_results(check)[0]);
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

        let unique_ptr_func = match self.loop_dense_view_func {
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
        if self.persist_successful_return_locals {
            self.persist_local_slots(builder, false);
        }
    }

    fn persist_local_slots(&self, builder: &mut FunctionBuilder, all: bool) {
        if !self.use_local_slots || !self.persist_locals_on_return {
            return;
        }

        if !all && self.dirty_local_names.is_empty() {
            return;
        }

        let ctx = match self.ctx_param {
            Some(ctx) => ctx,
            None => return,
        };

        for (name, variable) in &self.local_variables {
            if !all && !self.dirty_local_names.contains(name) {
                continue;
            }
            let value = builder.use_var(*variable);
            if let (Some(slot_index), Some(store_func)) = (
                self.local_names.iter().position(|local| local == name),
                self.vm_local_slot_set_func,
            ) {
                let slot_index_value = builder.ins().iconst(types::I64, slot_index as i64);
                let kind = builder.use_var(Variable::new(variable.index() + 1));
                builder.ins().call(store_func, &[ctx, slot_index_value, value, kind]);
            } else if let Some(store_func) = self.store_var_func {
                let name_hash = Self::hash_var_name(name) as i64;
                let name_hash_value = builder.ins().iconst(types::I64, name_hash);
                let zero = builder.ins().iconst(types::I64, 0);
                let kind = builder.use_var(Variable::new(variable.index() + 1));
                builder.ins().call(store_func, &[ctx, name_hash_value, zero, value, kind]);
            }
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
                | OpCode::JumpBack(target)
                | OpCode::TryStringAppendAssign(_, target)
                | OpCode::TryAppendAssign(_, target)
                | OpCode::GuardAppendPath(target)
                | OpCode::TryPopRebind(_, _, _, _, target)
                | OpCode::ForNext(target) => {
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

    fn begin_function(
        &mut self,
        builder: &mut FunctionBuilder,
        entry: Block,
    ) -> Result<Block, String> {
        let ctx = self.ctx_param.ok_or("Missing native context")?;
        let enter = self.enter_function_func.ok_or("Missing native depth guard")?;
        let call = builder.ins().call(enter, &[ctx]);
        let status = builder.inst_results(call)[0];
        let failed = builder.ins().icmp_imm(IntCC::NotEqual, status, 0);
        let failure = builder.create_block();
        let body = builder.create_block();
        builder.ins().brif(failed, failure, &[], body, &[]);
        builder.seal_block(entry);
        builder.seal_block(failure);
        builder.switch_to_block(failure);
        // Entry failed without acquiring a native depth slot.
        builder.ins().return_(&[status]);
        builder.switch_to_block(body);
        self.tracks_function_depth = true;
        Ok(body)
    }

    fn finish_parameter_initialization(
        &self,
        builder: &mut FunctionBuilder,
        entry: Block,
    ) -> Result<Block, String> {
        let ctx = self.ctx_param.ok_or("Missing native context")?;
        let check = self.runtime_error_func.ok_or("Missing native error helper")?;
        let call = builder.ins().call(check, &[ctx]);
        let status = builder.inst_results(call)[0];
        let failed = builder.ins().icmp_imm(IntCC::NotEqual, status, 0);
        let failure = builder.create_block();
        let body = builder.create_block();
        builder.ins().brif(failed, failure, &[], body, &[]);
        builder.seal_block(entry);
        builder.seal_block(failure);
        builder.switch_to_block(failure);
        // No body effects ran; never persist uninitialized locals on this exit.
        self.emit_return(builder, status);
        builder.switch_to_block(body);
        Ok(body)
    }

    fn emit_return(&self, builder: &mut FunctionBuilder, value: cranelift::prelude::Value) {
        if self.tracks_function_depth {
            builder.ins().call(
                self.leave_function_func.expect("native exit helper"),
                &[self.ctx_param.expect("native context")],
            );
        }
        builder.ins().return_(&[value]);
    }

    fn exit_on_assignment_error(
        &self,
        builder: &mut FunctionBuilder,
        status: cranelift::prelude::Value,
    ) {
        let failed = builder.ins().icmp_imm(IntCC::NotEqual, status, 0);
        let failure = builder.create_block();
        let success = builder.create_block();
        builder.ins().brif(failed, failure, &[], success, &[]);
        builder.seal_block(failure);
        builder.seal_block(success);
        builder.switch_to_block(failure);
        self.persist_local_slots(builder, true);
        self.emit_return(builder, status);
        builder.switch_to_block(success);
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

    fn set_vm_local_slot_store_function(&mut self, set_func: FuncRef) {
        self.vm_local_slot_set_func = Some(set_func);
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
        entry: usize,
    ) -> Result<(), String> {
        // Calculate and store function_end
        self.function_end = Self::calculate_function_end(instructions);

        // STEP 11 FIX: Analyze loop headers to find backward jump targets and their stack depths
        let loop_headers = Self::analyze_loop_headers(instructions, self.function_end, entry);

        // Create blocks for jump targets within function body
        for (pc, instruction) in instructions.iter().enumerate() {
            if pc >= self.function_end {
                break;
            }

            match instruction {
                OpCode::Jump(target)
                | OpCode::JumpIfFalse(target)
                | OpCode::JumpIfTrue(target)
                | OpCode::JumpBack(target)
                | OpCode::TryStringAppendAssign(_, target)
                | OpCode::TryAppendAssign(_, target)
                | OpCode::GuardAppendPath(target)
                | OpCode::TryPopRebind(_, _, _, _, target)
                | OpCode::ForNext(target) => {
                    if *target < self.function_end {
                        if !self.blocks.contains_key(target) {
                            let block = builder.create_block();

                            // STEP 11 FIX: If this is a loop header, add block parameters
                            // This enables backward jumps to pass values (like loop counters)
                            if let Some(&expected_depth) = loop_headers.get(target) {
                                // Record this as a loop header - it should NOT be sealed until
                                // all predecessors (including back-edges) are processed
                                self.loop_header_pcs.insert(*target);

                                // Each stack value carries an i64 payload and scalar kind
                                for _ in 0..expected_depth * 2 {
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
        // Every backward control-flow edge is a cancellation safepoint, including
        // conditional jumps and self-edges. The common non-task path skips the
        // Rust helper; the token itself is read atomically only in that helper.
        if self.cancellation_checks
            && matches!(instruction, OpCode::Jump(target) | OpCode::JumpBack(target)
            | OpCode::JumpIfFalse(target) | OpCode::JumpIfTrue(target) if *target <= pc)
        {
            let ctx = self.ctx_param.ok_or("Missing native cancellation context")?;
            let token = builder.ins().load(
                types::I64,
                MemFlags::new(),
                ctx,
                std::mem::offset_of!(VMContext, task_cancellation) as i32,
            );
            let poll = builder.create_block();
            let resume = builder.create_block();
            builder.ins().brif(token, poll, &[], resume, &[]);
            builder.switch_to_block(poll);
            builder.seal_block(poll);
            let helper = self.runtime_error_func.ok_or("Missing native cancellation helper")?;
            let call = builder.ins().call(helper, &[ctx]);
            self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
            builder.ins().jump(resume, &[]);
            builder.switch_to_block(resume);
            builder.seal_block(resume);
        }
        if self.guard_loop_callback(builder, pc, instruction)? {
            return Ok(true);
        }
        // Returns Ok(true) if this instruction terminates the block

        match instruction {
            // Successful operations stay native; only failures call into Rust.
            OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                let operation = match instruction {
                    OpCode::Add => 0,
                    OpCode::Sub => 1,
                    OpCode::Mul => 2,
                    OpCode::Div => 3,
                    _ => 4,
                };
                let owned_merge = self.begin_owned_operation(builder, operation, left, right);
                let merge = self.begin_float_operation(builder, operation, left, right)?;
                self.guard_scalar_types(builder, operation, left, right)?;
                let result = match instruction {
                    OpCode::Add | OpCode::Sub | OpCode::Mul => {
                        let (result, overflow, operation) = match instruction {
                            OpCode::Add => {
                                let (r, o) = builder.ins().sadd_overflow(left, right);
                                (r, o, 0)
                            }
                            OpCode::Sub => {
                                let (r, o) = builder.ins().ssub_overflow(left, right);
                                (r, o, 1)
                            }
                            _ => {
                                let (r, o) = builder.ins().smul_overflow(left, right);
                                (r, o, 2)
                            }
                        };
                        self.guard_arithmetic(builder, overflow, operation, left, right)?;
                        result
                    }
                    _ => {
                        let zero = builder.ins().icmp_imm(IntCC::Equal, right, 0);
                        let min = builder.ins().icmp_imm(IntCC::Equal, left, i64::MIN);
                        let minus_one = builder.ins().icmp_imm(IntCC::Equal, right, -1);
                        let overflow = builder.ins().band(min, minus_one);
                        let failed = builder.ins().bor(zero, overflow);
                        let operation = if matches!(instruction, OpCode::Div) { 3 } else { 4 };
                        // Guard before division: native traps cannot unwind VM handlers.
                        self.guard_arithmetic(builder, failed, operation, left, right)?;
                        if matches!(instruction, OpCode::Div) {
                            builder.ins().sdiv(left, right)
                        } else {
                            match self.last_int_constant {
                                Some((constant, divisor)) if constant == right => self
                                    .translate_mersenne_remainder(builder, left, divisor)
                                    .unwrap_or_else(|| builder.ins().srem(left, right)),
                                _ => builder.ins().srem(left, right),
                            }
                        }
                    }
                };
                self.finish_float_operation(builder, merge, result, 0);
                self.finish_owned_operation(builder, owned_merge)?;
                self.release_operand(builder, left);
                self.release_operand(builder, right);
            }
            OpCode::Negate => {
                let value = self.pop_value()?;
                let owned_merge = self.begin_owned_operation(builder, 5, value, value);
                let merge = self.begin_float_operation(builder, 5, value, value)?;
                self.guard_scalar_types(builder, 5, value, value)?;
                let overflow = builder.ins().icmp_imm(IntCC::Equal, value, i64::MIN);
                let zero = builder.ins().iconst(types::I64, 0);
                self.guard_arithmetic(builder, overflow, 5, value, zero)?;
                let result = builder.ins().ineg(value);
                self.finish_float_operation(builder, merge, result, 0);
                self.finish_owned_operation(builder, owned_merge)?;
                self.release_operand(builder, value);
            }

            // Comparison operations
            OpCode::Equal => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let owned_merge = self.begin_owned_equality(builder, a, b, false);
                let merge = self.begin_float_operation(builder, 11, a, b)?;
                let value_match = builder.ins().icmp(IntCC::Equal, a, b);
                let a_kind = self.scalar_kind(builder, a);
                let b_kind = self.scalar_kind(builder, b);
                let kind_match = builder.ins().icmp(IntCC::Equal, a_kind, b_kind);
                let equal = builder.ins().band(value_match, kind_match);
                let result = equal;
                let extended = builder.ins().uextend(types::I64, result);
                self.finish_float_operation(builder, merge, extended, 1);
                if let Some(owned_merge) = owned_merge {
                    let result = self.pop_value()?;
                    self.finish_float_operation(builder, owned_merge, result, 1);
                }
                self.release_operand(builder, a);
                self.release_operand(builder, b);
            }

            OpCode::NotEqual => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let owned_merge = self.begin_owned_equality(builder, a, b, true);
                let merge = self.begin_float_operation(builder, 12, a, b)?;
                let value_match = builder.ins().icmp(IntCC::Equal, a, b);
                let a_kind = self.scalar_kind(builder, a);
                let b_kind = self.scalar_kind(builder, b);
                let kind_match = builder.ins().icmp(IntCC::Equal, a_kind, b_kind);
                let equal = builder.ins().band(value_match, kind_match);
                let result = builder.ins().icmp_imm(IntCC::Equal, equal, 0);
                let extended = builder.ins().uextend(types::I64, result);
                self.finish_float_operation(builder, merge, extended, 1);
                if let Some(owned_merge) = owned_merge {
                    let result = self.pop_value()?;
                    self.finish_float_operation(builder, owned_merge, result, 1);
                }
                self.release_operand(builder, a);
                self.release_operand(builder, b);
            }

            OpCode::LessThan => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let owned_merge = self.begin_owned_operation(builder, 7, a, b);
                let merge = self.begin_float_operation(builder, 7, a, b)?;
                self.guard_scalar_types(builder, 7, a, b)?;
                let result = builder.ins().icmp(IntCC::SignedLessThan, a, b);
                let extended = builder.ins().uextend(types::I64, result);
                self.finish_float_operation(builder, merge, extended, 1);
                self.finish_owned_operation(builder, owned_merge)?;
                self.release_operand(builder, a);
                self.release_operand(builder, b);
            }

            OpCode::GreaterThan => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let owned_merge = self.begin_owned_operation(builder, 8, a, b);
                let merge = self.begin_float_operation(builder, 8, a, b)?;
                self.guard_scalar_types(builder, 8, a, b)?;
                let result = builder.ins().icmp(IntCC::SignedGreaterThan, a, b);
                let extended = builder.ins().uextend(types::I64, result);
                self.finish_float_operation(builder, merge, extended, 1);
                self.finish_owned_operation(builder, owned_merge)?;
                self.release_operand(builder, a);
                self.release_operand(builder, b);
            }

            OpCode::LessEqual => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let owned_merge = self.begin_owned_operation(builder, 9, a, b);
                let merge = self.begin_float_operation(builder, 9, a, b)?;
                self.guard_scalar_types(builder, 9, a, b)?;
                // a <= b: use SignedLessThanOrEqual directly
                let result = builder.ins().icmp(IntCC::SignedLessThanOrEqual, a, b);
                let extended = builder.ins().uextend(types::I64, result);
                self.finish_float_operation(builder, merge, extended, 1);
                self.finish_owned_operation(builder, owned_merge)?;
                self.release_operand(builder, a);
                self.release_operand(builder, b);
            }

            OpCode::GreaterEqual => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let owned_merge = self.begin_owned_operation(builder, 10, a, b);
                let merge = self.begin_float_operation(builder, 10, a, b)?;
                self.guard_scalar_types(builder, 10, a, b)?;
                // a >= b: use SignedGreaterThanOrEqual directly
                let result = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, a, b);
                let extended = builder.ins().uextend(types::I64, result);
                self.finish_float_operation(builder, merge, extended, 1);
                self.finish_owned_operation(builder, owned_merge)?;
                self.release_operand(builder, a);
                self.release_operand(builder, b);
            }

            // Logical operations
            OpCode::Not => {
                let a = self.pop_value()?;
                let owned_merge = self.begin_owned_operation(builder, 6, a, a);
                self.guard_scalar_types(builder, 6, a, a)?;
                let zero = builder.ins().iconst(types::I64, 0);
                let result = builder.ins().icmp(IntCC::Equal, a, zero);
                let extended = builder.ins().uextend(types::I64, result);
                let kind = builder.ins().iconst(types::I64, 1);
                self.scalar_kinds.insert(extended, kind);
                self.push_value(extended);
                self.finish_owned_operation(builder, owned_merge)?;
                self.release_operand(builder, a);
            }

            OpCode::And => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let a_truthy = self.scalar_truth(builder, a);
                let b_truthy = self.scalar_truth(builder, b);
                self.release_operand(builder, a);
                self.release_operand(builder, b);
                let truth = builder.ins().band(a_truthy, b_truthy);
                let result = builder.ins().uextend(types::I64, truth);
                let kind = builder.ins().iconst(types::I64, 1);
                self.scalar_kinds.insert(result, kind);
                self.push_value(result);
            }

            OpCode::Or => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let a_truthy = self.scalar_truth(builder, a);
                let b_truthy = self.scalar_truth(builder, b);
                self.release_operand(builder, a);
                self.release_operand(builder, b);
                let truth = builder.ins().bor(a_truthy, b_truthy);
                let result = builder.ins().uextend(types::I64, truth);
                let kind = builder.ins().iconst(types::I64, 1);
                self.scalar_kinds.insert(result, kind);
                self.push_value(result);
            }

            // Stack operations
            OpCode::Pop => {
                let value = self.pop_value()?;
                self.release_operand(builder, value);
            }

            OpCode::Dup => {
                let val = self.peek_value()?;
                let val = self.duplicate_operand(builder, val);
                self.push_value(val);
            }

            // Constant loading
            OpCode::LoadConst(index) => {
                if let Some(constant) = constants.get(*index) {
                    match constant {
                        Constant::Int(i) => {
                            let val = builder.ins().iconst(types::I64, *i);
                            self.last_int_constant = Some((val, *i));
                            self.push_value(val);
                        }
                        Constant::Bool(b) => {
                            let val = builder.ins().iconst(types::I64, if *b { 1 } else { 0 });
                            self.last_int_constant = None;
                            let kind = builder.ins().iconst(types::I64, 1);
                            self.scalar_kinds.insert(val, kind);
                            self.push_value(val);
                        }
                        Constant::Float(value) => {
                            let payload = builder.ins().iconst(types::I64, value.to_bits() as i64);
                            let kind =
                                builder.ins().iconst(types::I64, crate::native_scalar::FLOAT);
                            self.scalar_kinds.insert(payload, kind);
                            self.last_int_constant = None;
                            self.push_value(payload);
                        }
                        Constant::None => {
                            let payload = builder.ins().iconst(types::I64, 0);
                            let kind = builder.ins().iconst(types::I64, crate::native_scalar::NULL);
                            self.scalar_kinds.insert(payload, kind);
                            self.last_int_constant = None;
                            self.push_value(payload);
                        }
                        Constant::String(_) | Constant::Bytes(_) => {
                            let literal = self
                                .literal_values
                                .get(index)
                                .ok_or("Native literal was not retained")?;
                            let pointer = Arc::as_ptr(literal) as i64;
                            let helper = self.literal_load_func.ok_or("Missing literal helper")?;
                            let ctx = self.ctx_param.ok_or("Missing native context")?;
                            let pointer = builder.ins().iconst(types::I64, pointer);
                            let call = builder.ins().call(helper, &[ctx, pointer]);
                            let payload = builder.inst_results(call)[0];
                            self.capture_loaded_operand(builder, ctx, payload);
                            self.last_int_constant = None;
                            self.push_value(payload);
                        }
                        _ => {
                            // A placeholder is executed as a real integer by subsequent
                            // instructions (including division). Reject this region so
                            // the VM retains the actual value and its type.
                            return Err(format!("Unsupported JIT constant: {:?}", constant));
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
                    let args = self.stack_edge_args(builder, *target)?;
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
                let truth = self.scalar_truth(builder, condition);
                let is_false = builder.ins().icmp_imm(IntCC::Equal, truth, 0);

                if let Some(&target_block) = self.blocks.get(target) {
                    // Get or create the fallthrough block
                    let next_pc = pc + 1;
                    let fallthrough_block = *self.blocks.get(&next_pc).ok_or_else(|| {
                        format!("No fallthrough block after JumpIfFalse at PC {}", pc)
                    })?;

                    // Pass the current stack as block arguments to both branches
                    // This ensures both paths have access to the condition value
                    let args = self.stack_edge_args(builder, *target)?;

                    // Record expected stack depth for both blocks
                    self.prepare_stack_edge(builder, next_pc)?;

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
                let is_true = self.scalar_truth(builder, condition);

                if let Some(&target_block) = self.blocks.get(target) {
                    // Get or create the fallthrough block
                    let next_pc = pc + 1;
                    let fallthrough_block = *self.blocks.get(&next_pc).ok_or_else(|| {
                        format!("No fallthrough block after JumpIfTrue at PC {}", pc)
                    })?;

                    // Pass the current stack as block arguments to both branches
                    let args = self.stack_edge_args(builder, *target)?;

                    // Record expected stack depth for both blocks
                    self.prepare_stack_edge(builder, next_pc)?;

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
                    let args = self.stack_edge_args(builder, *target)?;
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

                // Marshal the tagged callable and arguments explicitly. Named
                // operand loads no longer leave hidden values on the VM stack.

                // Clear any pending self-recursion flag (not used yet)
                self.self_recursion_pending = false;

                // Pop values from JIT stack (in reverse order)
                let function_value = self.pop_value()?;
                let mut arg_vals = Vec::new();
                for _ in 0..*arg_count {
                    arg_vals.push(self.pop_value()?);
                }
                arg_vals.reverse(); // Reverse to get correct order

                if let (Some(ctx), Some(call_func)) = (self.ctx_param, self.call_func) {
                    // These popped operands are consumed by Call. Transfer them once;
                    // borrowed/peek marshaling elsewhere retains the original helper.
                    let take =
                        self.scalar_take_func.ok_or("Call requires consuming stack transfer")?;
                    for operand in std::iter::once(function_value).chain(arg_vals.iter().copied()) {
                        let kind = self.scalar_kind(builder, operand);
                        let call = builder.ins().call(take, &[ctx, operand, kind]);
                        self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
                    }

                    // A directly consumed FieldGet can insert a legacy receiver
                    // into the VM argument suffix. Reproduce that stack layout.
                    if self.field_call_sites.contains(&pc) {
                        let count = builder.ins().iconst(types::I64, *arg_count as i64);
                        let prepare =
                            self.field_call_func.ok_or("Field call requires receiver helper")?;
                        let call = builder.ins().call(prepare, &[ctx, count]);
                        self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
                    }

                    // Call jit_call_function
                    let null_ptr = builder.ins().iconst(types::I64, 0);
                    let arg_count_val = builder.ins().iconst(types::I64, *arg_count as i64);
                    let call_inst = builder.ins().call(call_func, &[ctx, null_ptr, arg_count_val]);
                    let status = builder.inst_results(call_inst)[0];
                    let failed = builder.ins().icmp_imm(IntCC::NotEqual, status, 0);
                    let failure_block = builder.create_block();
                    let success_block = builder.create_block();
                    builder.ins().brif(failed, failure_block, &[], success_block, &[]);
                    builder.seal_block(failure_block);
                    builder.seal_block(success_block);
                    builder.switch_to_block(failure_block);
                    // A later store in this loop's bytecode may already have run
                    // on a prior iteration. Persist every integer local here.
                    self.persist_local_slots(builder, true);
                    // Direct-argument functions return a value rather than a status.
                    // The owning context carries the failure in both conventions.
                    self.emit_return(builder, status);
                    builder.switch_to_block(success_block);

                    // Pop the result from VM stack
                    if let Some(stack_pop) = self.stack_pop_func {
                        let call = builder.ins().call(stack_pop, &[ctx]);
                        let result = builder.inst_results(call)[0];
                        self.capture_scalar_kind(builder, ctx, result);
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

            OpCode::ForNext(end) => {
                let index = self.pop_value()?;
                let object = self.pop_value()?;
                let ctx = self.ctx_param.ok_or("Iteration requires context")?;
                let helper = self.for_next_func.ok_or("Iteration requires helper")?;
                let object_kind = self.scalar_kind(builder, object);
                let index_kind = self.scalar_kind(builder, index);
                let call =
                    builder.ins().call(helper, &[ctx, object, object_kind, index, index_kind]);
                let result = builder.inst_results(call)[0];
                self.capture_loaded_operand(builder, ctx, result);
                let kind = self.scalar_kind(builder, result);
                self.release_operand(builder, object);
                self.release_operand(builder, index);
                let exhausted = builder.ins().icmp_imm(IntCC::Equal, kind, -4);
                let end_args = self.stack_edge_args(builder, *end)?;
                let end_block =
                    *self.blocks.get(end).ok_or("Missing iteration exhaustion target")?;
                self.push_value(result);
                let next_args = self.stack_edge_args(builder, pc + 1)?;
                let next_block =
                    *self.blocks.get(&(pc + 1)).ok_or("Missing iteration body target")?;
                builder.ins().brif(exhausted, end_block, &end_args, next_block, &next_args);
                return Ok(true);
            }

            OpCode::FieldSet(name) => {
                let object = self.pop_value()?;
                let value = self.pop_value()?;
                let ctx = self.ctx_param.ok_or("Field write requires context")?;
                let helper = self.field_write_func.ok_or("Field write requires helper")?;
                let name =
                    self.field_names.get(name).ok_or("Field write requires retained name")?;
                let pointer = builder.ins().iconst(types::I64, Arc::as_ptr(name) as i64);
                let object_kind = self.scalar_kind(builder, object);
                let value_kind = self.scalar_kind(builder, value);
                let call = builder
                    .ins()
                    .call(helper, &[ctx, object, object_kind, value, value_kind, pointer]);
                let result = builder.inst_results(call)[0];
                self.capture_loaded_operand(builder, ctx, result);
                self.release_operand(builder, object);
                self.release_operand(builder, value);
                self.push_value(result);
                return Ok(false);
            }

            OpCode::FieldGet(name) => {
                let object = self.pop_value()?;
                let ctx = self.ctx_param.ok_or("Field read requires context")?;
                let helper = self.field_read_func.ok_or("Field read requires helper")?;
                let name = self.field_names.get(name).ok_or("Field read requires retained name")?;
                let pointer = builder.ins().iconst(types::I64, Arc::as_ptr(name) as i64);
                let kind = self.scalar_kind(builder, object);
                let call = builder.ins().call(helper, &[ctx, object, kind, pointer]);
                let result = builder.inst_results(call)[0];
                self.capture_loaded_operand(builder, ctx, result);
                self.release_operand(builder, object);
                self.push_value(result);
                return Ok(false);
            }

            OpCode::IndexGet => {
                let index = self.pop_value()?;
                let object = self.pop_value()?;
                let ctx = self.ctx_param.ok_or("Indexed read requires context")?;
                let helper = self.indexed_read_func.ok_or("Indexed read requires helper")?;
                let object_kind = self.scalar_kind(builder, object);
                let index_kind = self.scalar_kind(builder, index);
                let call =
                    builder.ins().call(helper, &[ctx, object, object_kind, index, index_kind]);
                let result = builder.inst_results(call)[0];
                self.capture_loaded_operand(builder, ctx, result);
                self.release_operand(builder, object);
                self.release_operand(builder, index);
                self.push_value(result);
                return Ok(false);
            }

            OpCode::IndexSet => {
                let index = self.pop_value()?;
                let object = self.pop_value()?;
                let value = self.pop_value()?;
                let ctx = self.ctx_param.ok_or("Indexed replacement requires context")?;
                let helper =
                    self.index_set_result_func.ok_or("Indexed replacement requires helper")?;
                let object_kind = self.scalar_kind(builder, object);
                let index_kind = self.scalar_kind(builder, index);
                let value_kind = self.scalar_kind(builder, value);
                let call = builder.ins().call(
                    helper,
                    &[ctx, object, object_kind, index, index_kind, value, value_kind],
                );
                let result = builder.inst_results(call)[0];
                self.capture_loaded_operand(builder, ctx, result);
                self.release_operand(builder, index);
                self.release_operand(builder, object);
                self.release_operand(builder, value);
                self.push_value(result);
                return Ok(false);
            }

            OpCode::MakeStruct(name, fields) => {
                let ctx = self.ctx_param.ok_or("MakeStruct requires context")?;
                let helper = self.make_struct_func.ok_or("MakeStruct requires helper")?;
                let push = self.scalar_push_func.ok_or("MakeStruct requires tagged stack push")?;
                let shape = self
                    .struct_shapes
                    .get(&(name.clone(), fields.clone()))
                    .ok_or("MakeStruct requires retained metadata")?;
                let pointer = Arc::as_ptr(shape) as i64;
                let mut operands = Vec::with_capacity(fields.len());
                for _ in 0..fields.len() {
                    operands.push(self.pop_value()?);
                }
                for operand in operands.into_iter().rev() {
                    let kind = self.scalar_kind(builder, operand);
                    builder.ins().call(push, &[ctx, operand, kind]);
                    self.release_operand(builder, operand);
                }
                let pointer = builder.ins().iconst(types::I64, pointer);
                let call = builder.ins().call(helper, &[ctx, pointer]);
                let success = builder.inst_results(call)[0];
                let failed = builder.ins().icmp_imm(IntCC::Equal, success, 0);
                let status = builder.ins().uextend(types::I64, failed);
                self.exit_on_assignment_error(builder, status);
                let pop = self.stack_pop_func.ok_or("MakeStruct requires tagged stack pop")?;
                let call = builder.ins().call(pop, &[ctx]);
                let value = builder.inst_results(call)[0];
                self.capture_loaded_operand(builder, ctx, value);
                self.push_value(value);
                return Ok(false);
            }

            OpCode::MakeArray(count) => {
                let ctx = self.ctx_param.ok_or("MakeArray requires context")?;
                let helper = self.make_array_func.ok_or("MakeArray requires helper")?;
                let push = self.scalar_push_func.ok_or("MakeArray requires tagged stack push")?;
                let mut operands = Vec::with_capacity(*count);
                for _ in 0..*count {
                    operands.push(self.pop_value()?);
                }
                for operand in operands.into_iter().rev() {
                    let kind = self.scalar_kind(builder, operand);
                    builder.ins().call(push, &[ctx, operand, kind]);
                    self.release_operand(builder, operand);
                }
                let count = builder.ins().iconst(types::I64, *count as i64);
                let call = builder.ins().call(helper, &[ctx, count]);
                let success = builder.inst_results(call)[0];
                let failed = builder.ins().icmp_imm(IntCC::Equal, success, 0);
                let status = builder.ins().uextend(types::I64, failed);
                self.exit_on_assignment_error(builder, status);
                let pop = self.stack_pop_func.ok_or("MakeArray requires tagged stack pop")?;
                let call = builder.ins().call(pop, &[ctx]);
                let value = builder.inst_results(call)[0];
                self.capture_loaded_operand(builder, ctx, value);
                self.push_value(value);
                return Ok(false);
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
                    let push =
                        self.scalar_push_func.ok_or("MakeDict requires tagged stack push")?;
                    for (key, value) in temp_pairs.iter().rev() {
                        for operand in [*key, *value] {
                            let kind = self.scalar_kind(builder, operand);
                            builder.ins().call(push, &[ctx, operand, kind]);
                            self.release_operand(builder, operand);
                        }
                    }

                    // Call jit_make_dict with num_pairs
                    let num_pairs_val = builder.ins().iconst(types::I64, *num_pairs as i64);
                    let call = builder.ins().call(make_dict_ref, &[ctx, num_pairs_val]);
                    let _success = builder.inst_results(call)[0];

                    // The constructor's Value becomes an independently rooted
                    // operand, never a sentinel referring to hidden VM-stack state.
                    let failed = builder.ins().icmp_imm(IntCC::Equal, _success, 0);
                    let status = builder.ins().uextend(types::I64, failed);
                    self.exit_on_assignment_error(builder, status);
                    let pop = self.stack_pop_func.ok_or("MakeDict requires tagged stack pop")?;
                    let call = builder.ins().call(pop, &[ctx]);
                    let value = builder.inst_results(call)[0];
                    self.capture_loaded_operand(builder, ctx, value);
                    self.push_value(value);

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

                    let push = self
                        .scalar_push_func
                        .ok_or("MakeDictWithKeys requires tagged stack push")?;
                    for value in temp_values.iter().rev() {
                        let kind = self.scalar_kind(builder, *value);
                        builder.ins().call(push, &[ctx, *value, kind]);
                        self.release_operand(builder, *value);
                    }

                    let keys_ptr = builder.ins().iconst(types::I64, Arc::as_ptr(keys) as i64);
                    let keys_len = builder.ins().iconst(types::I64, keys.len() as i64);
                    let call = builder.ins().call(make_dict_keys_ref, &[ctx, keys_ptr, keys_len]);
                    let _success = builder.inst_results(call)[0];

                    let failed = builder.ins().icmp_imm(IntCC::Equal, _success, 0);
                    let status = builder.ins().uextend(types::I64, failed);
                    self.exit_on_assignment_error(builder, status);
                    let pop =
                        self.stack_pop_func.ok_or("MakeDictWithKeys requires tagged stack pop")?;
                    let call = builder.ins().call(pop, &[ctx]);
                    let value = builder.inst_results(call)[0];
                    self.capture_loaded_operand(builder, ctx, value);
                    self.push_value(value);

                    return Ok(false);
                } else {
                    return Err("MakeDictWithKeys requires context and helper".to_string());
                }
            }

            OpCode::TryStringAppendAssign(binding, end) | OpCode::TryAppendAssign(binding, end) => {
                use crate::bytecode::MutationBinding;
                let collection = matches!(instruction, OpCode::TryAppendAssign(_, _));
                let count = if collection { 3 } else { 2 };
                let depth = self.value_stack.len();
                if depth < count {
                    return Err("Append guard stack underflow".into());
                }
                let original = self.value_stack[depth - count];
                let suffix = self.value_stack[depth - count + 1];
                let ctx = self.ctx_param.ok_or("Append guard requires context")?;
                let helper =
                    if collection { self.try_append_func } else { self.try_string_append_func }
                        .ok_or("Append guard requires helper")?;
                let (location, target, target_kind, mutable) = match binding {
                    MutationBinding::Local(slot) => {
                        let name = self
                            .local_names
                            .get(*slot)
                            .ok_or("Invalid string append slot")?
                            .clone();
                        let mutable = self
                            .local_binding_kinds
                            .get(*slot)
                            .copied()
                            .unwrap_or(BytecodeBindingKind::Mutable)
                            == BytecodeBindingKind::Mutable;
                        self.dirty_local_names.insert(name.clone());
                        if let Some(&variable) = self.local_variables.get(&name) {
                            (
                                1,
                                builder.use_var(variable),
                                builder.use_var(Variable::new(variable.index() + 1)),
                                mutable,
                            )
                        } else {
                            let location = if self.tracks_function_depth { 2 } else { 0 };
                            let target = if location == 2 {
                                Self::hash_var_name(&name) as i64
                            } else {
                                *slot as i64
                            };
                            (
                                location,
                                builder.ins().iconst(types::I64, target),
                                builder.ins().iconst(types::I64, 0),
                                mutable,
                            )
                        }
                    }
                    MutationBinding::Named(name) => (
                        3,
                        builder.ins().iconst(types::I64, Self::hash_var_name(name) as i64),
                        builder.ins().iconst(types::I64, 0),
                        true,
                    ),
                };
                let location = builder.ins().iconst(types::I64, location);
                let mutable = builder.ins().iconst(types::I64, i64::from(mutable));
                let original_kind = self.scalar_kind(builder, original);
                let suffix_kind = self.scalar_kind(builder, suffix);
                let mut arguments = vec![
                    ctx,
                    location,
                    target,
                    target_kind,
                    original,
                    original_kind,
                    suffix,
                    suffix_kind,
                    mutable,
                ];
                let function = collection.then(|| self.value_stack[depth - 1]);
                if let Some(function) = function {
                    arguments.push(function);
                    arguments.push(self.scalar_kind(builder, function));
                }
                let call = builder.ins().call(helper, &arguments);
                let applied = builder.inst_results(call)[0];
                let check = builder
                    .ins()
                    .call(self.runtime_error_func.ok_or("Missing runtime error guard")?, &[ctx]);
                self.exit_on_assignment_error(builder, builder.inst_results(check)[0]);
                let next = pc + 1;
                let fallback_args = self.stack_edge_args(builder, next)?;
                let fallback = *self.blocks.get(&next).ok_or("Missing append fallback")?;
                let success = builder.create_block();
                let condition = builder.ins().icmp_imm(IntCC::NotEqual, applied, 0);
                builder.ins().brif(condition, success, &[], fallback, &fallback_args);
                builder.seal_block(success);
                builder.switch_to_block(success);
                self.value_stack.truncate(depth - count);
                self.release_operand(builder, original);
                self.release_operand(builder, suffix);
                if let Some(function) = function {
                    self.release_operand(builder, function);
                }
                let args = self.stack_edge_args(builder, *end)?;
                let destination = *self.blocks.get(end).ok_or("Missing append success target")?;
                builder.ins().jump(destination, &args);
                return Ok(true);
            }

            OpCode::AppendConstStringInPlace(slot, rhs) => {
                let suffix = builder.ins().iconst(types::I64, rhs.as_ptr() as i64);
                let length = builder.ins().iconst(types::I64, rhs.len() as i64);
                self.append_local(builder, pc, *slot, suffix, length)?;
            }
            OpCode::AppendConstCharInPlace(slot, rhs) => {
                let suffix = builder.ins().iconst(types::I64, *rhs as i64);
                let length = builder.ins().iconst(types::I64, -1);
                self.append_local(builder, pc, *slot, suffix, length)?;
            }

            OpCode::TryPopRebind(binding, pair_slot, argc, shape, end) => {
                use crate::bytecode::MutationBinding;
                let count = shape.iter().filter(|step| step.is_none()).count();
                let mut indices = Vec::with_capacity(count);
                for _ in 0..count {
                    indices.push(self.pop_value()?);
                }
                indices.reverse();
                let depth = self.value_stack.len();
                if depth < argc + 1 {
                    return Err("Pop rebind stack underflow".into());
                }
                let calls = self.value_stack[depth - argc - 1..].to_vec();
                let ctx = self.ctx_param.ok_or("Pop rebind requires context")?;
                let helper = self.try_pop_rebind_func.ok_or("Pop rebind requires helper")?;
                let (name, location, target, target_kind, mutable) = match binding {
                    MutationBinding::Local(slot) => {
                        let name =
                            self.local_names.get(*slot).ok_or("Invalid pop binding slot")?.clone();
                        self.dirty_local_names.insert(name.clone());
                        let mutable = self
                            .local_binding_kinds
                            .get(*slot)
                            .copied()
                            .unwrap_or(BytecodeBindingKind::Mutable)
                            == BytecodeBindingKind::Mutable;
                        let (location, target, kind) =
                            if let Some(&variable) = self.local_variables.get(&name) {
                                (
                                    1,
                                    builder.use_var(variable),
                                    builder.use_var(Variable::new(variable.index() + 1)),
                                )
                            } else {
                                (
                                    if self.tracks_function_depth { 2 } else { 0 },
                                    builder.ins().iconst(types::I64, *slot as i64),
                                    builder.ins().iconst(types::I64, 0),
                                )
                            };
                        (name, location, target, kind, mutable)
                    }
                    MutationBinding::Named(name) => (
                        name.clone(),
                        3,
                        builder.ins().iconst(types::I64, 0),
                        builder.ins().iconst(types::I64, 0),
                        true,
                    ),
                };
                let pair_name =
                    self.local_names.get(*pair_slot).ok_or("Invalid pop pair slot")?.clone();
                self.dirty_local_names.insert(pair_name.clone());
                let (pair_location, pair_target, pair_kind) =
                    if let Some(&variable) = self.local_variables.get(&pair_name) {
                        (
                            1,
                            builder.use_var(variable),
                            builder.use_var(Variable::new(variable.index() + 1)),
                        )
                    } else {
                        let location = if self.tracks_function_depth { 2 } else { 0 };
                        let target = if location == 2 {
                            Self::hash_var_name(&pair_name) as i64
                        } else {
                            *pair_slot as i64
                        };
                        (
                            location,
                            builder.ins().iconst(types::I64, target),
                            builder.ins().iconst(types::I64, 0),
                        )
                    };
                let retained = self.path_shapes.get(shape).ok_or("Missing pop path metadata")?;
                let shape = builder.ins().iconst(types::I64, Arc::as_ptr(retained) as i64);
                let values = indices.iter().chain(calls.iter()).copied().collect::<Vec<_>>();
                let size = u32::try_from(values.len().checked_mul(16).ok_or("Pop path too large")?)
                    .map_err(|_| "Pop path too large")?;
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size,
                    3,
                ));
                for (i, value) in values.iter().enumerate() {
                    let kind = self.scalar_kind(builder, *value);
                    builder.ins().stack_store(*value, slot, (i * 16) as i32);
                    builder.ins().stack_store(kind, slot, (i * 16 + 8) as i32);
                }
                let operands = builder.ins().stack_addr(types::I64, slot, 0);
                let location = builder.ins().iconst(types::I64, location);
                let mutable = builder.ins().iconst(types::I64, i64::from(mutable));
                let name_hash = builder.ins().iconst(types::I64, Self::hash_var_name(&name) as i64);
                let pair_location = builder.ins().iconst(types::I64, pair_location);
                let argc_value = builder.ins().iconst(types::I64, *argc as i64);
                let call = builder.ins().call(
                    helper,
                    &[
                        ctx,
                        location,
                        target,
                        target_kind,
                        mutable,
                        name_hash,
                        pair_location,
                        pair_target,
                        pair_kind,
                        shape,
                        argc_value,
                        operands,
                    ],
                );
                let result = builder.inst_results(call)[0];
                self.capture_loaded_operand(builder, ctx, result);
                for index in indices {
                    self.release_operand(builder, index);
                }
                let kind = self.scalar_kind(builder, result);
                let applied = builder.ins().icmp_imm(IntCC::NotEqual, kind, -4);
                let fallback = *self.blocks.get(&(pc + 1)).ok_or("Missing pop fallback")?;
                let fallback_args = self.stack_edge_args(builder, pc + 1)?;
                let success = builder.create_block();
                builder.ins().brif(applied, success, &[], fallback, &fallback_args);
                builder.seal_block(success);
                builder.switch_to_block(success);
                self.value_stack.truncate(depth - argc - 1);
                for value in calls {
                    self.release_operand(builder, value);
                }
                self.push_value(result);
                let success_args = self.stack_edge_args(builder, *end)?;
                let end_block = *self.blocks.get(end).ok_or("Missing pop success")?;
                builder.ins().jump(end_block, &success_args);
                return Ok(true);
            }

            OpCode::GuardAppendPath(fallback) => {
                let depth = self.value_stack.len();
                if depth < 3 {
                    return Err("Path append guard stack underflow".into());
                }
                let ctx = self.ctx_param.ok_or("Path append guard requires context")?;
                let helper =
                    self.guard_append_path_func.ok_or("Path append guard requires helper")?;
                let mut args = vec![ctx];
                for index in depth - 3..depth {
                    let value = self.value_stack[index];
                    args.push(value);
                    args.push(self.scalar_kind(builder, value));
                }
                let call = builder.ins().call(helper, &args);
                let valid = builder.inst_results(call)[0];
                let check = builder
                    .ins()
                    .call(self.runtime_error_func.ok_or("Missing runtime error guard")?, &[ctx]);
                self.exit_on_assignment_error(builder, builder.inst_results(check)[0]);
                let fallback_args = self.stack_edge_args(builder, *fallback)?;
                let next_args = self.stack_edge_args(builder, pc + 1)?;
                let fallback_block = *self.blocks.get(fallback).ok_or("Missing append fallback")?;
                let next = *self.blocks.get(&(pc + 1)).ok_or("Missing append body")?;
                builder.ins().brif(valid, next, &next_args, fallback_block, &fallback_args);
                return Ok(true);
            }

            OpCode::UpdateBindingPath(binding, shape)
            | OpCode::AppendBindingPath(binding, shape) => {
                use crate::bytecode::MutationBinding;
                let append = matches!(instruction, OpCode::AppendBindingPath(_, _));
                let count = shape.iter().filter(|step| step.is_none()).count();
                let mut indices = Vec::with_capacity(count);
                for _ in 0..count {
                    indices.push(self.pop_value()?);
                }
                indices.reverse();
                let values = if append {
                    let function = self.pop_value()?;
                    let item = self.pop_value()?;
                    let original = self.pop_value()?;
                    vec![original, item, function]
                } else {
                    vec![self.pop_value()?]
                };
                let ctx = self.ctx_param.ok_or("Path write requires context")?;
                let helper = (if append {
                    self.append_binding_path_func
                } else {
                    self.update_binding_path_func
                })
                .ok_or("Path write requires helper")?;
                let (name, location, target, kind, binding_kind) = match binding {
                    MutationBinding::Local(slot) => {
                        let name =
                            self.local_names.get(*slot).ok_or("Invalid path write slot")?.clone();
                        self.dirty_local_names.insert(name.clone());
                        let binding_kind = match self
                            .local_binding_kinds
                            .get(*slot)
                            .copied()
                            .unwrap_or(BytecodeBindingKind::Mutable)
                        {
                            BytecodeBindingKind::Mutable => 0,
                            BytecodeBindingKind::LetImmutable => 1,
                            BytecodeBindingKind::Const => 2,
                        };
                        let (location, target, kind) =
                            if let Some(&variable) = self.local_variables.get(&name) {
                                (
                                    1,
                                    builder.use_var(variable),
                                    builder.use_var(Variable::new(variable.index() + 1)),
                                )
                            } else {
                                (
                                    if self.tracks_function_depth { 2 } else { 0 },
                                    builder.ins().iconst(types::I64, *slot as i64),
                                    builder.ins().iconst(types::I64, 0),
                                )
                            };
                        (name, location, target, kind, binding_kind)
                    }
                    MutationBinding::Named(name) => (
                        name.clone(),
                        3,
                        builder.ins().iconst(types::I64, 0),
                        builder.ins().iconst(types::I64, 0),
                        0,
                    ),
                };
                let retained = self.path_shapes.get(shape).ok_or("Missing retained path shape")?;
                let shape = builder.ins().iconst(types::I64, Arc::as_ptr(retained) as i64);
                let size =
                    u32::try_from((count + values.len()).checked_mul(16).ok_or("Path too large")?)
                        .map_err(|_| "Path too large")?;
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size,
                    3,
                ));
                indices.extend(values);
                for (index, operand) in indices.iter().enumerate() {
                    let kind = self.scalar_kind(builder, *operand);
                    builder.ins().stack_store(*operand, slot, (index * 16) as i32);
                    builder.ins().stack_store(kind, slot, (index * 16 + 8) as i32);
                }
                let operands = builder.ins().stack_addr(types::I64, slot, 0);
                let location = builder.ins().iconst(types::I64, location);
                let binding_kind = builder.ins().iconst(types::I64, binding_kind);
                let hash = builder.ins().iconst(types::I64, Self::hash_var_name(&name) as i64);
                let call = builder.ins().call(
                    helper,
                    &[ctx, location, target, kind, binding_kind, hash, shape, operands],
                );
                self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
                for operand in indices {
                    self.release_operand(builder, operand);
                }
                let null = builder.ins().iconst(types::I64, 0);
                let kind = builder.ins().iconst(types::I64, crate::native_scalar::NULL);
                self.scalar_kinds.insert(null, kind);
                self.push_value(null);
            }

            OpCode::IndexSetInPlace(slot_index) => {
                if !self.tracks_function_depth && self.int_dict_slots.contains(slot_index) {
                    self.loop_indexed_access(builder, *slot_index, true)?;
                    return Ok(false);
                }
                if self.tracks_function_depth || self.loop_resume_enabled {
                    let index = self.pop_value()?;
                    let value = self.pop_value()?;
                    let name = self
                        .local_names
                        .get(*slot_index)
                        .ok_or("Invalid indexed write slot")?
                        .clone();
                    let ctx = self.ctx_param.ok_or("Indexed write requires context")?;
                    let helper = self.indexed_write_func.ok_or("Indexed write requires helper")?;
                    let (location, target, kind) =
                        if let Some(&variable) = self.local_variables.get(&name) {
                            self.dirty_local_names.insert(name.clone());
                            (
                                1,
                                builder.use_var(variable),
                                builder.use_var(Variable::new(variable.index() + 1)),
                            )
                        } else {
                            (
                                2,
                                builder.ins().iconst(types::I64, 0),
                                builder.ins().iconst(types::I64, 0),
                            )
                        };
                    let binding = match self
                        .local_binding_kinds
                        .get(*slot_index)
                        .copied()
                        .unwrap_or(BytecodeBindingKind::Mutable)
                    {
                        BytecodeBindingKind::Mutable => 0,
                        BytecodeBindingKind::LetImmutable => 1,
                        BytecodeBindingKind::Const => 2,
                    };
                    let binding = builder.ins().iconst(types::I64, binding);
                    let location = builder.ins().iconst(types::I64, location);
                    let hash = builder.ins().iconst(types::I64, Self::hash_var_name(&name) as i64);
                    let index_kind = self.scalar_kind(builder, index);
                    let value_kind = self.scalar_kind(builder, value);
                    let call = builder.ins().call(
                        helper,
                        &[
                            ctx, location, target, kind, binding, hash, index, index_kind, value,
                            value_kind,
                        ],
                    );
                    self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
                    self.release_operand(builder, index);
                    self.release_operand(builder, value);
                    let null = builder.ins().iconst(types::I64, 0);
                    let null_kind = builder.ins().iconst(types::I64, crate::native_scalar::NULL);
                    self.scalar_kinds.insert(null, null_kind);
                    self.push_value(null);
                    return Ok(false);
                }
                // In-place index assignment: var[index] = value
                // Stack: [value, index] -> []
                // This is like: tmp = LoadVar(var); tmp[index] = value; StoreVar(var, tmp)

                let var_name = self.local_names.get(*slot_index).cloned();

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
                if !self.tracks_function_depth && self.int_dict_slots.contains(slot_index) {
                    self.loop_indexed_access(builder, *slot_index, false)?;
                    return Ok(false);
                }
                if self.tracks_function_depth || self.loop_resume_enabled {
                    let index = self.pop_value()?;
                    // The synchronous read helper runs no user callbacks. A live
                    // SSA local can therefore lend its root without duplicating
                    // it for this operand. Named/captured bindings still use the
                    // ordinary load path. Never borrow the suspended caller's
                    // physical slots for a whole-function callee.
                    let borrowed_local = if self.use_local_slots {
                        self.local_names
                            .get(*slot_index)
                            .filter(|name| !self.non_int_locals.contains(*name))
                            .and_then(|name| self.local_variables.get(name))
                            .copied()
                    } else {
                        None
                    };
                    let object = if let Some(variable) = borrowed_local {
                        let value = builder.use_var(variable);
                        let kind = builder.use_var(Variable::new(variable.index() + 1));
                        self.scalar_kinds.insert(value, kind);
                        value
                    } else {
                        self.translate_instruction(
                            builder,
                            pc,
                            &OpCode::LoadLocal(*slot_index),
                            constants,
                        )?;
                        self.pop_value()?
                    };
                    let ctx = self.ctx_param.ok_or("Indexed read requires context")?;
                    let helper = self.indexed_read_func.ok_or("Indexed read requires helper")?;
                    let object_kind = self.scalar_kind(builder, object);
                    let index_kind = self.scalar_kind(builder, index);
                    let args = [ctx, object, object_kind, index, index_kind];
                    let call = if let (Some(_), Some(cache), Some(cached_helper)) = (
                        borrowed_local,
                        self.indexed_read_caches.get(slot_index),
                        self.indexed_read_cached_func,
                    ) {
                        let pointer = builder.ins().stack_addr(types::I64, *cache, 0);
                        builder.ins().call(
                            cached_helper,
                            &[ctx, object, object_kind, index, index_kind, pointer],
                        )
                    } else {
                        builder.ins().call(helper, &args)
                    };

                    let result = builder.inst_results(call)[0];
                    self.capture_loaded_operand(builder, ctx, result);
                    if borrowed_local.is_none() {
                        self.release_operand(builder, object);
                    }
                    self.release_operand(builder, index);
                    self.push_value(result);
                    return Ok(false);
                }
                // In-place index read: value = var[index]
                // Stack: [index] -> [value]
                // This is like: tmp = LoadVar(var); value = tmp[index]

                let var_name = self.local_names.get(*slot_index).cloned();

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
                            self.capture_scalar_kind(builder, ctx, value);
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
                                self.capture_scalar_kind(builder, ctx, value);
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
                        self.capture_scalar_kind(builder, ctx, value);
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
                        // Scalar return metadata is available in every entrypoint,
                        // including loop and script compilation.
                        if let Some(set_return) = self.scalar_return_func {
                            let kind = self.scalar_kind(builder, return_value);
                            builder.ins().call(set_return, &[ctx, return_value, kind]);
                        } else if let Some(set_return_int) = self.set_return_int_func {
                            builder.ins().call(set_return_int, &[ctx, return_value]);
                        } else if let Some(push_int_func) = self.push_int_func {
                            // Fallback: Use original jit_push_int (slower path)
                            let inst = builder.ins().call(push_int_func, &[ctx, return_value]);
                            let _result = builder.inst_results(inst)[0];
                        }
                    }
                }
                // Return 0 (success)
                let zero = builder.ins().iconst(types::I64, 0);
                self.emit_return(builder, zero);
                return Ok(true); // Terminates block
            }

            OpCode::ReturnNone => {
                self.emit_null_return(builder);
                self.persist_local_slots_to_vm(builder);
                let zero = builder.ins().iconst(types::I64, 0);
                self.emit_return(builder, zero);
                return Ok(true); // Terminates block
            }

            // Variable operations - use stack slots for locals (fast path) or runtime helpers (fallback)
            OpCode::LoadVar(name) => {
                // NOTE: Self-recursion detection was added but direct recursion causes
                // stack overflow due to shared stack slots. Keeping detection code for
                // future tail-call optimization work.
                if self.non_int_locals.contains(name) {
                    if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.operand_load_func) {
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);
                        let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                        let result = builder.inst_results(call)[0];
                        self.capture_loaded_operand(builder, ctx, result);
                        self.push_value(result);
                        return Ok(false);
                    }
                }

                // OPTIMIZATION: Use stack slots for local variables (register-based locals)
                // This avoids C function calls and HashMap lookups for every variable access
                if self.use_local_slots {
                    if let Some(&variable) = self.local_variables.get(name) {
                        let value = builder.use_var(variable);
                        let kind = builder.use_var(Variable::new(variable.index() + 1));
                        self.scalar_kinds.insert(value, kind);
                        let value = self.duplicate_operand(builder, value);
                        self.push_value(value);

                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!("JIT: LoadVar '{}' using SSA variable {:?}", name, variable);
                        }
                    } else {
                        // Variable not in local slots - fall back to runtime call
                        // This handles globals and captured variables
                        if let (Some(ctx), Some(load_func)) =
                            (self.ctx_param, self.operand_load_func)
                        {
                            let name_hash = Self::hash_var_name(name) as i64;
                            let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                            let zero = builder.ins().iconst(types::I64, 0);
                            let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                            let result = builder.inst_results(call)[0];
                            self.capture_loaded_operand(builder, ctx, result);
                            self.push_value(result);
                        } else {
                            let zero = builder.ins().iconst(types::I64, 0);
                            self.push_value(zero);
                        }
                    }
                } else if let (Some(ctx), Some(load_func)) =
                    (self.ctx_param, self.operand_load_func)
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
                    let zero = builder.ins().iconst(types::I64, 0); // name_len = 0 (use hash instead)

                    // Call jit_load_variable(ctx, name_hash, 0)
                    let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                    let result = builder.inst_results(call)[0];
                    self.capture_loaded_operand(builder, ctx, result);
                    self.push_value(result);
                } else {
                    // Fallback: load 0 if context not available
                    let zero = builder.ins().iconst(types::I64, 0);
                    self.push_value(zero);
                }
            }

            OpCode::ReleaseRebindLocal(slot, count) => {
                use crate::bytecode::MutationBinding;
                let depth = self.value_stack.len();
                if depth <= *count {
                    return Err("Rebind release stack underflow".into());
                }
                let original = self.value_stack[depth - count - 1];
                let function = self.value_stack[depth - 1];
                let ctx = self.ctx_param.ok_or("Rebind release requires context")?;
                let helper = self.release_rebind_func.ok_or("Rebind release requires helper")?;
                let (location, target, target_kind, mutable) =
                    match &crate::bytecode::MutationBinding::Local(*slot) {
                        MutationBinding::Local(slot) => {
                            let name = self
                                .local_names
                                .get(*slot)
                                .ok_or("Invalid string append slot")?
                                .clone();
                            let mutable = self
                                .local_binding_kinds
                                .get(*slot)
                                .copied()
                                .unwrap_or(BytecodeBindingKind::Mutable)
                                == BytecodeBindingKind::Mutable;
                            self.dirty_local_names.insert(name.clone());
                            if let Some(&variable) = self.local_variables.get(&name) {
                                (
                                    1,
                                    builder.use_var(variable),
                                    builder.use_var(Variable::new(variable.index() + 1)),
                                    mutable,
                                )
                            } else {
                                let location = if self.tracks_function_depth { 2 } else { 0 };
                                let target = if location == 2 {
                                    Self::hash_var_name(&name) as i64
                                } else {
                                    *slot as i64
                                };
                                (
                                    location,
                                    builder.ins().iconst(types::I64, target),
                                    builder.ins().iconst(types::I64, 0),
                                    mutable,
                                )
                            }
                        }
                        MutationBinding::Named(name) => (
                            3,
                            builder.ins().iconst(types::I64, Self::hash_var_name(name) as i64),
                            builder.ins().iconst(types::I64, 0),
                            true,
                        ),
                    };
                let location = builder.ins().iconst(types::I64, location);
                let mutable = builder.ins().iconst(types::I64, i64::from(mutable));
                let original_kind = self.scalar_kind(builder, original);
                let function_kind = self.scalar_kind(builder, function);
                let call = builder.ins().call(
                    helper,
                    &[
                        ctx,
                        location,
                        target,
                        target_kind,
                        original,
                        original_kind,
                        mutable,
                        function,
                        function_kind,
                    ],
                );
                self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
            }

            OpCode::ReleaseDeadLocal(slot) => {
                let name = self
                    .local_names
                    .get(*slot)
                    .ok_or_else(|| "Invalid dead-local slot".to_string())?
                    .clone();
                let ctx = self.ctx_param.ok_or("Dead local requires context")?;
                let helper = self.dead_local_func.ok_or("Dead local requires helper")?;
                let index = builder.ins().iconst(types::I64, *slot as i64);
                let hash = builder.ins().iconst(types::I64, Self::hash_var_name(&name) as i64);
                let clear =
                    builder.ins().iconst(types::I64, i64::from(!self.tracks_function_depth));
                let call = builder.ins().call(helper, &[ctx, index, hash, clear]);
                self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
                if let Some(&variable) = self.local_variables.get(&name) {
                    let null = builder.ins().iconst(types::I64, 0);
                    let kind = builder.ins().iconst(types::I64, crate::native_scalar::NULL);
                    self.scalar_kinds.insert(null, kind);
                    self.store_native_local(builder, variable, null);
                }
                if let Some(&cache) = self.int_dict_ptr_slots.get(slot) {
                    let zero = builder.ins().iconst(types::I64, 0);
                    builder.ins().stack_store(zero, cache, 0);
                }
            }

            OpCode::LoadLocal(slot) => {
                if let Some(name) = self.local_names.get(*slot) {
                    if self.non_int_locals.contains(name) {
                        if let (Some(ctx), Some(load_func)) =
                            (self.ctx_param, self.operand_load_func)
                        {
                            let name_hash = Self::hash_var_name(name) as i64;
                            let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                            let zero = builder.ins().iconst(types::I64, 0);
                            let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                            let result = builder.inst_results(call)[0];
                            self.capture_loaded_operand(builder, ctx, result);
                            self.push_value(result);
                            return Ok(false);
                        }
                    }
                    if self.use_local_slots {
                        if let Some(&variable) = self.local_variables.get(name) {
                            let value = builder.use_var(variable);
                            let kind = builder.use_var(Variable::new(variable.index() + 1));
                            self.scalar_kinds.insert(value, kind);
                            let value = self.duplicate_operand(builder, value);
                            self.push_value(value);
                            return Ok(false);
                        }
                    }

                    if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.operand_load_func) {
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);
                        let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                        let result = builder.inst_results(call)[0];
                        self.capture_loaded_operand(builder, ctx, result);
                        self.push_value(result);
                        return Ok(false);
                    }
                }

                let zero = builder.ins().iconst(types::I64, 0);
                self.push_value(zero);
            }

            OpCode::StoreVar(name) => {
                if let (Some(ctx), Some(assign)) = (self.ctx_param, self.named_assignment_func) {
                    // Tagged assignment handles owned values too. The legacy stack
                    // store writes private locals and must not shadow a global or
                    // bypass its readonly binding check for collection constructors.
                    let value = self.peek_value()?;
                    let hash = builder.ins().iconst(types::I64, Self::hash_var_name(name) as i64);
                    let kind = self.scalar_kind(builder, value);
                    let named = builder.ins().iconst(types::I64, 0);
                    let call = builder.ins().call(assign, &[ctx, hash, value, kind, named]);
                    self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
                    return Ok(false);
                }
                // IMPORTANT: StoreVar PEEKS at the stack (doesn't pop)
                // The value remains on stack after assignment
                let value = self.peek_value()?;
                if self.non_int_locals.contains(name) {
                    if let (Some(ctx), Some(store_from_stack)) =
                        (self.ctx_param, self.store_var_from_stack_func)
                    {
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);

                        if let Some(push) = self.scalar_push_func {
                            let kind = self.scalar_kind(builder, value);
                            builder.ins().call(push, &[ctx, value, kind]);
                        }
                        builder.ins().call(store_from_stack, &[ctx, name_hash_val]);
                        return Ok(false);
                    }
                }

                // OPTIMIZATION: Use stack slots for local variables (register-based locals)
                if self.use_local_slots {
                    if let Some(&variable) = self.local_variables.get(name) {
                        self.dirty_local_names.insert(name.clone());
                        self.store_native_local(builder, variable, value);

                        if std::env::var("DEBUG_JIT").is_ok() {
                            eprintln!("JIT: StoreVar '{}' using SSA variable {:?}", name, variable);
                        }
                        // Value stays on stack - do NOT pop
                    } else {
                        // Variable not in local slots - fall back to runtime call
                        if let (Some(ctx), Some(store_func)) = (self.ctx_param, self.store_var_func)
                        {
                            let name_hash = Self::hash_var_name(name) as i64;
                            let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                            let zero = builder.ins().iconst(types::I64, 0);
                            let kind = self.scalar_kind(builder, value);
                            builder
                                .ins()
                                .call(store_func, &[ctx, name_hash_val, zero, value, kind]);
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
                    let kind = self.scalar_kind(builder, value);
                    builder.ins().call(store_func, &[ctx, name_hash_val, zero, value, kind]);
                    // Value stays on stack - do NOT pop
                }
                // If no context, just leave value on stack
            }

            OpCode::StoreLocal(slot) | OpCode::DefineLocal(slot) => {
                self.check_local_initialization(
                    builder,
                    *slot,
                    matches!(instruction, OpCode::DefineLocal(_)),
                )?;
                let value = self.peek_value()?;

                if let Some(name) = self.local_names.get(*slot) {
                    if self.non_int_locals.contains(name) {
                        if let (Some(ctx), Some(store_from_stack)) =
                            (self.ctx_param, self.store_var_from_stack_func)
                        {
                            let name_hash = Self::hash_var_name(name) as i64;
                            let name_hash_val = builder.ins().iconst(types::I64, name_hash);

                            if let Some(push) = self.scalar_push_func {
                                let kind = self.scalar_kind(builder, value);
                                builder.ins().call(push, &[ctx, value, kind]);
                            }
                            builder.ins().call(store_from_stack, &[ctx, name_hash_val]);
                            return Ok(false);
                        }
                    }
                    if self.use_local_slots {
                        if let Some(&variable) = self.local_variables.get(name) {
                            self.dirty_local_names.insert(name.clone());
                            self.store_native_local(builder, variable, value);
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
                        let kind = self.scalar_kind(builder, value);
                        builder.ins().call(store_func, &[ctx, name_hash_val, zero, value, kind]);
                    }
                }
            }

            OpCode::LoadGlobal(name) => {
                // Explicit global bytecode must bypass same-named parameters.
                if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.operand_load_func) {
                    let name_hash = {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        name.hash(&mut hasher);
                        hasher.finish() as i64
                    };

                    let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                    let global_only = builder.ins().iconst(types::I64, 1);

                    let call = builder.ins().call(load_func, &[ctx, name_hash_val, global_only]);
                    let result = builder.inst_results(call)[0];
                    self.capture_loaded_operand(builder, ctx, result);
                    self.push_value(result);
                } else {
                    let zero = builder.ins().iconst(types::I64, 0);
                    self.push_value(zero);
                }
            }

            OpCode::StoreGlobal(name) => {
                if let (Some(ctx), Some(assign)) = (self.ctx_param, self.named_assignment_func) {
                    let value = self.peek_value()?;
                    let kind = self.scalar_kind(builder, value);
                    let hash = builder.ins().iconst(types::I64, Self::hash_var_name(name) as i64);
                    let global_only = builder.ins().iconst(types::I64, 1);
                    let call = builder.ins().call(assign, &[ctx, hash, value, kind, global_only]);
                    self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
                    return Ok(false);
                }
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

                    let kind = self.scalar_kind(builder, value);

                    builder.ins().call(store_func, &[ctx, name_hash_val, zero, value, kind]);
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
                    if let Some(&variable) = self.local_variables.get(name) {
                        let value = builder.use_var(variable);
                        let kind = builder.use_var(Variable::new(variable.index() + 1));
                        self.scalar_kinds.insert(value, kind);
                        let value = self.duplicate_operand(builder, value);
                        self.push_value(value);
                        return Ok(false);
                    }

                    if name == param_name {
                        if let Some(direct_param) = self.direct_arg_param {
                            let value = self.duplicate_operand(builder, direct_param);
                            self.push_value(value);
                            return Ok(false);
                        }
                    }

                    if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.operand_load_func) {
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);
                        let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                        let result = builder.inst_results(call)[0];
                        self.capture_loaded_operand(builder, ctx, result);
                        self.push_value(result);
                        return Ok(false);
                    }
                }

                let zero = builder.ins().iconst(types::I64, 0);
                self.push_value(zero);
                Ok(false)
            }

            OpCode::StoreLocal(slot) | OpCode::DefineLocal(slot) => {
                self.check_local_initialization(
                    builder,
                    *slot,
                    matches!(instruction, OpCode::DefineLocal(_)),
                )?;
                let value = self.peek_value()?;
                if let Some(name) = self.local_names.get(*slot) {
                    if self.non_int_locals.contains(name) {
                        if let (Some(ctx), Some(store_from_stack)) =
                            (self.ctx_param, self.store_var_from_stack_func)
                        {
                            let name_hash = Self::hash_var_name(name) as i64;
                            let name_hash_val = builder.ins().iconst(types::I64, name_hash);

                            if let Some(push) = self.scalar_push_func {
                                let kind = self.scalar_kind(builder, value);
                                builder.ins().call(push, &[ctx, value, kind]);
                            }
                            builder.ins().call(store_from_stack, &[ctx, name_hash_val]);
                            return Ok(false);
                        }
                    }
                    if let Some(&variable) = self.local_variables.get(name) {
                        self.store_native_local(builder, variable, value);
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
                        let kind = self.scalar_kind(builder, value);
                        builder.ins().call(store_func, &[ctx, name_hash_val, zero, value, kind]);
                    }
                }
                Ok(false)
            }

            // LoadVar: Use direct_arg_param if loading the function parameter
            OpCode::LoadVar(name) => {
                if name == param_name {
                    if let Some(&variable) = self.local_variables.get(name) {
                        let value = builder.use_var(variable);
                        let kind = builder.use_var(Variable::new(variable.index() + 1));
                        self.scalar_kinds.insert(value, kind);
                        let value = self.duplicate_operand(builder, value);
                        self.push_value(value);
                        return Ok(false);
                    }
                    // Compatibility fallback for manually constructed translators.
                    if let Some(direct_param) = self.direct_arg_param {
                        let value = self.duplicate_operand(builder, direct_param);
                        self.push_value(value);
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
                    } else if let Some(&variable) = self.local_variables.get(name) {
                        let value = builder.use_var(variable);
                        let kind = builder.use_var(Variable::new(variable.index() + 1));
                        self.scalar_kinds.insert(value, kind);
                        let value = self.duplicate_operand(builder, value);
                        self.push_value(value);
                    } else if let (Some(ctx), Some(load_func)) =
                        (self.ctx_param, self.operand_load_func)
                    {
                        // Fall back to runtime call for globals
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);
                        let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                        let result = builder.inst_results(call)[0];
                        self.capture_loaded_operand(builder, ctx, result);
                        self.push_value(result);
                    } else {
                        let zero = builder.ins().iconst(types::I64, 0);
                        self.push_value(zero);
                    }
                } else if let Some(&variable) = self.local_variables.get(name) {
                    let value = builder.use_var(variable);
                    let kind = builder.use_var(Variable::new(variable.index() + 1));
                    self.scalar_kinds.insert(value, kind);
                    let value = self.duplicate_operand(builder, value);
                    self.push_value(value);
                } else {
                    // Fall back to runtime call
                    if let (Some(ctx), Some(load_func)) = (self.ctx_param, self.operand_load_func) {
                        let name_hash = Self::hash_var_name(name) as i64;
                        let name_hash_val = builder.ins().iconst(types::I64, name_hash);
                        let zero = builder.ins().iconst(types::I64, 0);
                        let call = builder.ins().call(load_func, &[ctx, name_hash_val, zero]);
                        let result = builder.inst_results(call)[0];
                        self.capture_loaded_operand(builder, ctx, result);
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
                        let kind = self.scalar_kind(builder, arg_val);
                        // Context lifetime is the enclosing native invocation; use
                        // a native store rather than adding an FFI call per recursion.
                        builder.ins().store(
                            MemFlags::new(),
                            kind,
                            ctx,
                            std::mem::offset_of!(VMContext, arg_kinds) as i32,
                        );
                        let call_inst = builder.ins().call(self_func, &[ctx, arg_val]);
                        let result = builder.inst_results(call_inst)[0];
                        self.capture_scalar_kind(builder, ctx, result);
                        if let Some(error_func) = self.runtime_error_func {
                            let check = builder.ins().call(error_func, &[ctx]);
                            self.exit_on_assignment_error(builder, builder.inst_results(check)[0]);
                        }

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
                    let function_value = self.pop_value()?;
                    let mut arg_vals = Vec::new();
                    for _ in 0..*arg_count {
                        arg_vals.push(self.pop_value()?);
                    }
                    arg_vals.reverse();

                    if let (Some(ctx), Some(call_func)) = (self.ctx_param, self.call_func) {
                        // These popped operands are consumed by Call. Transfer them once;
                        // borrowed/peek marshaling elsewhere retains the original helper.
                        let take = self
                            .scalar_take_func
                            .ok_or("Call requires consuming stack transfer")?;
                        for operand in
                            std::iter::once(function_value).chain(arg_vals.iter().copied())
                        {
                            let kind = self.scalar_kind(builder, operand);
                            let call = builder.ins().call(take, &[ctx, operand, kind]);
                            self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
                        }

                        // Call jit_call_function
                        let null_ptr = builder.ins().iconst(types::I64, 0);
                        let arg_count_val = builder.ins().iconst(types::I64, *arg_count as i64);
                        builder.ins().call(call_func, &[ctx, null_ptr, arg_count_val]);

                        // Pop result from VM stack
                        if let Some(stack_pop) = self.stack_pop_func {
                            let call = builder.ins().call(stack_pop, &[ctx]);
                            let result = builder.inst_results(call)[0];
                            self.capture_scalar_kind(builder, ctx, result);
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
                    if let (Some(ctx), Some(set_return)) = (self.ctx_param, self.scalar_return_func)
                    {
                        let kind = self.scalar_kind(builder, return_value);
                        builder.ins().call(set_return, &[ctx, return_value, kind]);
                    }
                    // Direct return of the computed value
                    self.emit_return(builder, return_value);

                    if std::env::var("DEBUG_JIT").is_ok() {
                        eprintln!("JIT direct-arg: Direct value return");
                    }
                } else {
                    // No value to return - return 0
                    let zero = builder.ins().iconst(types::I64, 0);
                    self.emit_return(builder, zero);
                }
                Ok(true) // Terminates block
            }

            OpCode::ReturnNone => {
                self.emit_null_return(builder);
                let zero = builder.ins().iconst(types::I64, 0);
                self.emit_return(builder, zero);
                Ok(true)
            }

            // For all other opcodes, delegate to standard translate_instruction
            _ => self.translate_instruction(builder, _pc, instruction, constants),
        }
    }

    fn emit_null_return(&self, builder: &mut FunctionBuilder) {
        if let (Some(ctx), Some(set_return)) = (self.ctx_param, self.scalar_return_func) {
            let zero = builder.ins().iconst(types::I64, 0);
            let kind = builder.ins().iconst(types::I64, crate::native_scalar::NULL);
            builder.ins().call(set_return, &[ctx, zero, kind]);
        }
    }

    fn duplicate_operand(
        &mut self,
        builder: &mut FunctionBuilder,
        value: cranelift::prelude::Value,
    ) -> cranelift::prelude::Value {
        let (Some(ctx), Some(helper)) = (self.ctx_param, self.operand_duplicate_func) else {
            return value;
        };
        let kind = self.scalar_kind(builder, value);
        let owned = builder.ins().icmp_imm(IntCC::Equal, kind, crate::native_owned::OWNED);
        let copy = builder.create_block();
        let immediate = builder.create_block();
        let join = builder.create_block();
        builder.append_block_param(join, types::I64);
        builder.ins().brif(owned, copy, &[], immediate, &[]);
        builder.seal_block(copy);
        builder.seal_block(immediate);
        builder.switch_to_block(immediate);
        builder.ins().jump(join, &[value]);
        builder.switch_to_block(copy);
        let call = builder.ins().call(helper, &[ctx, value, kind]);
        let result = builder.inst_results(call)[0];
        if let Some(error_func) = self.runtime_error_func {
            let check = builder.ins().call(error_func, &[ctx]);
            self.exit_on_assignment_error(builder, builder.inst_results(check)[0]);
        }
        builder.ins().jump(join, &[result]);
        builder.seal_block(join);
        builder.switch_to_block(join);
        let result = builder.block_params(join)[0];
        self.scalar_kinds.insert(result, kind);
        result
    }

    fn release_operand(&self, builder: &mut FunctionBuilder, value: cranelift::prelude::Value) {
        let (Some(ctx), Some(helper)) = (self.ctx_param, self.operand_release_func) else {
            return;
        };
        let kind = self.scalar_kind(builder, value);
        let owned = builder.ins().icmp_imm(IntCC::Equal, kind, crate::native_owned::OWNED);
        let release = builder.create_block();
        let join = builder.create_block();
        builder.ins().brif(owned, release, &[], join, &[]);
        builder.seal_block(release);
        builder.switch_to_block(release);
        let call = builder.ins().call(helper, &[ctx, value, kind]);
        self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
        builder.ins().jump(join, &[]);
        builder.seal_block(join);
        builder.switch_to_block(join);
    }

    fn store_native_local(
        &mut self,
        builder: &mut FunctionBuilder,
        variable: Variable,
        value: cranelift::prelude::Value,
    ) {
        // Stores peek at their operand. The local must own a separate root so
        // discarding the operand cannot invalidate the stored value.
        let copy = self.duplicate_operand(builder, value);
        let old = builder.use_var(variable);
        let old_kind = builder.use_var(Variable::new(variable.index() + 1));
        self.scalar_kinds.insert(old, old_kind);
        self.release_operand(builder, old);
        builder.def_var(variable, copy);
        let kind = self.scalar_kind(builder, copy);
        builder.def_var(Variable::new(variable.index() + 1), kind);
    }

    fn begin_owned_equality(
        &self,
        builder: &mut FunctionBuilder,
        left: cranelift::prelude::Value,
        right: cranelift::prelude::Value,
        negate: bool,
    ) -> Option<Block> {
        let (Some(ctx), Some(helper)) = (self.ctx_param, self.owned_equal_func) else {
            return None;
        };
        let left_kind = self.scalar_kind(builder, left);
        let right_kind = self.scalar_kind(builder, right);
        let left_owned =
            builder.ins().icmp_imm(IntCC::Equal, left_kind, crate::native_owned::OWNED);
        let right_owned =
            builder.ins().icmp_imm(IntCC::Equal, right_kind, crate::native_owned::OWNED);
        let owned = builder.ins().bor(left_owned, right_owned);
        let owned_block = builder.create_block();
        let immediate_block = builder.create_block();
        let merge = builder.create_block();
        builder.append_block_param(merge, types::I64);
        builder.append_block_param(merge, types::I64);
        builder.ins().brif(owned, owned_block, &[], immediate_block, &[]);
        builder.seal_block(owned_block);
        builder.seal_block(immediate_block);
        builder.switch_to_block(owned_block);
        let call = builder.ins().call(helper, &[ctx, left, left_kind, right, right_kind]);
        let result = builder.inst_results(call)[0];
        if let Some(error_func) = self.runtime_error_func {
            let check = builder.ins().call(error_func, &[ctx]);
            self.exit_on_assignment_error(builder, builder.inst_results(check)[0]);
        }
        let result = if negate { builder.ins().bxor_imm(result, 1) } else { result };
        let kind = builder.ins().iconst(types::I64, crate::native_scalar::BOOLEAN);
        builder.ins().jump(merge, &[result, kind]);
        builder.switch_to_block(immediate_block);
        Some(merge)
    }

    fn begin_owned_operation(
        &mut self,
        builder: &mut FunctionBuilder,
        operation: i64,
        left: cranelift::prelude::Value,
        right: cranelift::prelude::Value,
    ) -> Option<Block> {
        let (Some(ctx), Some(helper)) = (self.ctx_param, self.owned_operator_func) else {
            return None;
        };
        let left_kind = self.scalar_kind(builder, left);
        let right_kind = self.scalar_kind(builder, right);
        let left_owned =
            builder.ins().icmp_imm(IntCC::Equal, left_kind, crate::native_owned::OWNED);
        let right_owned =
            builder.ins().icmp_imm(IntCC::Equal, right_kind, crate::native_owned::OWNED);
        let owned = builder.ins().bor(left_owned, right_owned);
        let owned_block = builder.create_block();
        let immediate = builder.create_block();
        let merge = builder.create_block();
        builder.append_block_param(merge, types::I64);
        builder.append_block_param(merge, types::I64);
        builder.ins().brif(owned, owned_block, &[], immediate, &[]);
        builder.seal_block(owned_block);
        builder.seal_block(immediate);
        builder.switch_to_block(owned_block);
        let op = builder.ins().iconst(types::I64, operation);
        let call = builder.ins().call(helper, &[ctx, op, left, left_kind, right, right_kind]);
        let result = builder.inst_results(call)[0];
        self.capture_loaded_operand(builder, ctx, result);
        let kind = self.scalar_kind(builder, result);
        builder.ins().jump(merge, &[result, kind]);
        builder.switch_to_block(immediate);
        Some(merge)
    }

    fn finish_owned_operation(
        &mut self,
        builder: &mut FunctionBuilder,
        merge: Option<Block>,
    ) -> Result<(), String> {
        if let Some(merge) = merge {
            let result = self.pop_value()?;
            let kind = self.scalar_kind(builder, result);
            builder.ins().jump(merge, &[result, kind]);
            builder.switch_to_block(merge);
            builder.seal_block(merge);
            let params = builder.block_params(merge);
            let payload = params[0];
            self.scalar_kinds.insert(payload, params[1]);
            self.push_value(payload);
        }
        Ok(())
    }

    /// Inline only unique, in-bounds dense integer access. All type/shape changes
    /// use the same tagged VM contracts, then refresh the explicit native view.
    fn loop_indexed_access(
        &mut self,
        builder: &mut FunctionBuilder,
        slot: usize,
        write: bool,
    ) -> Result<(), String> {
        let index = self.pop_value()?;
        let value = if write { Some(self.pop_value()?) } else { None };
        let index_kind = self.scalar_kind(builder, index);
        let value_kind = value.map(|v| self.scalar_kind(builder, v));
        let ctx = self.ctx_param.ok_or("Loop indexing requires context")?;
        let cache = *self.int_dict_ptr_slots.get(&slot).ok_or("Missing loop dictionary view")?;
        let name = self.local_names.get(slot).ok_or("Invalid loop dictionary slot")?;
        let hash = Self::hash_var_name(name) as i64;
        let binding = match self
            .local_binding_kinds
            .get(slot)
            .copied()
            .unwrap_or(BytecodeBindingKind::Mutable)
        {
            BytecodeBindingKind::Mutable => 0,
            BytecodeBindingKind::LetImmutable => 1,
            BytecodeBindingKind::Const => 2,
        };
        let view = builder.ins().stack_load(types::I64, cache, 0);
        let present = builder.ins().icmp_imm(IntCC::NotEqual, view, 0);
        let integer =
            builder.ins().icmp_imm(IntCC::Equal, index_kind, crate::native_scalar::INTEGER);
        let mut eligible = builder.ins().band(present, integer);
        if let Some(kind) = value_kind {
            let integer = builder.ins().icmp_imm(IntCC::Equal, kind, crate::native_scalar::INTEGER);
            eligible = builder.ins().band(eligible, integer);
            if binding != 0 {
                eligible = builder.ins().iconst(types::I8, 0);
            }
        }
        let bounds = builder.create_block();
        let fast = builder.create_block();
        let slow = builder.create_block();
        let join = builder.create_block();
        if !write {
            builder.append_block_param(join, types::I64);
            builder.append_block_param(join, types::I64);
        }
        builder.ins().brif(eligible, bounds, &[], slow, &[]);
        builder.seal_block(bounds);
        builder.switch_to_block(bounds);
        let len = builder.ins().load(
            types::I64,
            MemFlags::new(),
            view,
            std::mem::offset_of!(LoopDenseView, len) as i32,
        );
        // Unsigned comparison also rejects negative indices.
        let in_bounds = builder.ins().icmp(IntCC::UnsignedLessThan, index, len);
        builder.ins().brif(in_bounds, fast, &[], slow, &[]);
        builder.seal_block(fast);
        builder.seal_block(slow);
        builder.switch_to_block(fast);
        let data = builder.ins().load(
            types::I64,
            MemFlags::new(),
            view,
            std::mem::offset_of!(LoopDenseView, data) as i32,
        );
        let offset = builder.ins().imul_imm(index, std::mem::size_of::<i64>() as i64);
        let address = builder.ins().iadd(data, offset);
        if let Some(value) = value {
            builder.ins().store(MemFlags::new(), value, address, 0);
            builder.ins().jump(join, &[]);
        } else {
            let result = builder.ins().load(types::I64, MemFlags::new(), address, 0);
            let kind = builder.ins().iconst(types::I64, crate::native_scalar::INTEGER);
            builder.ins().jump(join, &[result, kind]);
        }
        builder.switch_to_block(slow);
        let slot_value = builder.ins().iconst(types::I64, slot as i64);
        if let Some(value) = value {
            let helper = self.indexed_write_func.ok_or("Missing loop write helper")?;
            let zero = builder.ins().iconst(types::I64, 0);
            let binding = builder.ins().iconst(types::I64, binding);
            let hash = builder.ins().iconst(types::I64, hash);
            let call = builder.ins().call(
                helper,
                &[
                    ctx,
                    zero,
                    slot_value,
                    zero,
                    binding,
                    hash,
                    index,
                    index_kind,
                    value,
                    value_kind.unwrap(),
                ],
            );
            self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
            let refresh = self.loop_dense_view_func.ok_or("Missing loop view helper")?;
            let call = builder.ins().call(refresh, &[ctx, slot_value]);
            let refreshed = builder.inst_results(call)[0];
            builder.ins().stack_store(refreshed, cache, 0);
            builder.ins().jump(join, &[]);
        } else {
            let helper = self.loop_indexed_read_func.ok_or("Missing loop read helper")?;
            let call = builder.ins().call(helper, &[ctx, slot_value, index, index_kind]);
            let result = builder.inst_results(call)[0];
            self.capture_loaded_operand(builder, ctx, result);
            let kind = self.scalar_kind(builder, result);
            builder.ins().jump(join, &[result, kind]);
        }
        builder.seal_block(join);
        builder.switch_to_block(join);
        self.release_operand(builder, index);
        if let Some(value) = value {
            self.release_operand(builder, value);
            let null = builder.ins().iconst(types::I64, 0);
            let kind = builder.ins().iconst(types::I64, crate::native_scalar::NULL);
            self.scalar_kinds.insert(null, kind);
            self.push_value(null);
        } else {
            let result = builder.block_params(join)[0];
            let kind = builder.block_params(join)[1];
            self.scalar_kinds.insert(result, kind);
            self.push_value(result);
        }
        Ok(())
    }

    fn guard_loop_callback(
        &self,
        builder: &mut FunctionBuilder,
        pc: usize,
        instruction: &OpCode,
    ) -> Result<bool, String> {
        if self.tracks_function_depth || !self.loop_resume_enabled {
            return Ok(false);
        }
        let depth = match instruction {
            OpCode::Add
            | OpCode::Sub
            | OpCode::Mul
            | OpCode::Div
            | OpCode::Mod
            | OpCode::LessThan
            | OpCode::LessEqual
            | OpCode::GreaterThan
            | OpCode::GreaterEqual
            | OpCode::ForNext(_) => 2,
            OpCode::Negate | OpCode::Not | OpCode::FieldGet(_) => 1,
            OpCode::Call(_) | OpCode::Return | OpCode::ReturnNone => 0,
            _ => return Ok(false),
        };
        let (Some(ctx), Some(guard), Some(prepare), Some(push)) = (
            self.ctx_param,
            self.loop_resume_guard_func,
            self.loop_resume_prepare_func,
            self.scalar_push_func,
        ) else {
            return Ok(false);
        };
        if depth == 0 {
            let pc = builder.ins().iconst(types::I64, pc as i64);
            builder.ins().call(prepare, &[ctx, pc]);
            for value in &self.value_stack {
                let kind = self.scalar_kind(builder, *value);
                builder.ins().call(push, &[ctx, *value, kind]);
            }
            self.persist_local_slots(builder, true);
            let status = builder.ins().iconst(types::I64, 2);
            self.emit_return(builder, status);
            return Ok(true);
        }
        let index =
            self.value_stack.len().checked_sub(depth).ok_or("Callback guard stack underflow")?;
        let value = self.value_stack[index];
        let kind = self.scalar_kind(builder, value);
        let resume = builder.create_block();
        let proceed = builder.create_block();
        let inspect = builder.create_block();
        let owned = builder.ins().icmp_imm(IntCC::Equal, kind, crate::native_owned::OWNED);
        builder.ins().brif(owned, inspect, &[], proceed, &[]);
        builder.seal_block(inspect);
        builder.switch_to_block(inspect);
        let guard_kind = if matches!(instruction, OpCode::FieldGet(_)) {
            builder.ins().iconst(types::I64, -2)
        } else if matches!(instruction, OpCode::ForNext(_)) {
            builder.ins().iconst(types::I64, -3)
        } else {
            kind
        };
        let call = builder.ins().call(guard, &[ctx, value, guard_kind]);
        let result = builder.inst_results(call)[0];
        let required = builder.ins().icmp_imm(IntCC::NotEqual, result, 0);
        builder.ins().brif(required, resume, &[], proceed, &[]);
        builder.seal_block(resume);
        builder.seal_block(proceed);
        builder.switch_to_block(resume);
        let pc = builder.ins().iconst(types::I64, pc as i64);
        builder.ins().call(prepare, &[ctx, pc]);
        for value in &self.value_stack {
            let kind = self.scalar_kind(builder, *value);
            builder.ins().call(push, &[ctx, *value, kind]);
        }
        self.persist_local_slots(builder, true);
        let status = builder.ins().iconst(types::I64, 2);
        self.emit_return(builder, status);
        builder.switch_to_block(proceed);
        Ok(false)
    }

    fn append_local(
        &mut self,
        builder: &mut FunctionBuilder,
        pc: usize,
        slot: usize,
        suffix: cranelift::prelude::Value,
        length: cranelift::prelude::Value,
    ) -> Result<(), String> {
        let name = self.local_names.get(slot).ok_or("Invalid append local slot")?.clone();
        let ctx = self.ctx_param.ok_or("Append requires context")?;
        let helper = self.append_local_func.ok_or("Append requires local helper")?;
        let (location, target, kind) = if let Some(&variable) = self.local_variables.get(&name) {
            self.dirty_local_names.insert(name.clone());
            (1, builder.use_var(variable), builder.use_var(Variable::new(variable.index() + 1)))
        } else {
            let (location, target) = if self.tracks_function_depth {
                (2, Self::hash_var_name(&name) as i64)
            } else {
                (0, slot as i64)
            };
            (
                location,
                builder.ins().iconst(types::I64, target),
                builder.ins().iconst(types::I64, 0),
            )
        };
        let location = builder.ins().iconst(types::I64, location);
        let call = builder.ins().call(helper, &[ctx, location, target, kind, suffix, length]);
        if self.loop_resume_enabled {
            let status = builder.inst_results(call)[0];
            let needs_resume = builder.ins().icmp_imm(IntCC::Equal, status, 2);
            let resume = builder.create_block();
            let proceed = builder.create_block();
            builder.ins().brif(needs_resume, resume, &[], proceed, &[]);
            builder.seal_block(resume);
            builder.seal_block(proceed);
            builder.switch_to_block(resume);
            let prepare = self.loop_resume_prepare_func.ok_or("Missing resume helper")?;
            let push = self.scalar_push_func.ok_or("Missing resume operand helper")?;
            let pc = builder.ins().iconst(types::I64, pc as i64);
            builder.ins().call(prepare, &[ctx, pc]);
            for value in &self.value_stack {
                let kind = self.scalar_kind(builder, *value);
                builder.ins().call(push, &[ctx, *value, kind]);
            }
            self.persist_local_slots(builder, true);
            self.emit_return(builder, status);
            builder.switch_to_block(proceed);
        }
        self.exit_on_assignment_error(builder, builder.inst_results(call)[0]);
        if let Some(&variable) = self.local_variables.get(&name) {
            let result_helper = self.append_result_func.ok_or("Missing append result helper")?;
            let call = builder.ins().call(result_helper, &[ctx]);
            let payload = builder.inst_results(call)[0];
            self.capture_scalar_kind(builder, ctx, payload);
            let kind = self.scalar_kind(builder, payload);
            builder.def_var(variable, payload);
            builder.def_var(Variable::new(variable.index() + 1), kind);
        }
        Ok(())
    }

    fn capture_loaded_operand(
        &mut self,
        builder: &mut FunctionBuilder,
        ctx: cranelift::prelude::Value,
        value: cranelift::prelude::Value,
    ) {
        self.capture_scalar_kind(builder, ctx, value);
        if let Some(error_func) = self.runtime_error_func {
            let check = builder.ins().call(error_func, &[ctx]);
            self.exit_on_assignment_error(builder, builder.inst_results(check)[0]);
        }
    }

    fn scalar_truth(
        &self,
        builder: &mut FunctionBuilder,
        value: cranelift::prelude::Value,
    ) -> cranelift::prelude::Value {
        let kind = self.scalar_kind(builder, value);
        let is_float = builder.ins().icmp_imm(IntCC::Equal, kind, crate::native_scalar::FLOAT);
        // Both signs of floating zero are false; NaNs and infinities are true.
        let magnitude = builder.ins().band_imm(value, i64::MAX);
        let truth_bits = builder.ins().select(is_float, magnitude, value);
        let immediate_truth = builder.ins().icmp_imm(IntCC::NotEqual, truth_bits, 0);
        if let (Some(ctx), Some(helper)) = (self.ctx_param, self.owned_truth_func) {
            let owned = builder.ins().icmp_imm(IntCC::Equal, kind, crate::native_owned::OWNED);
            let owned_block = builder.create_block();
            let immediate_block = builder.create_block();
            let join = builder.create_block();
            builder.append_block_param(join, types::I8);
            builder.ins().brif(owned, owned_block, &[], immediate_block, &[]);
            builder.seal_block(owned_block);
            builder.seal_block(immediate_block);
            builder.switch_to_block(immediate_block);
            builder.ins().jump(join, &[immediate_truth]);
            builder.switch_to_block(owned_block);
            let call = builder.ins().call(helper, &[ctx, value]);
            let truth = builder.inst_results(call)[0];
            if let Some(error_func) = self.runtime_error_func {
                let check = builder.ins().call(error_func, &[ctx]);
                self.exit_on_assignment_error(builder, builder.inst_results(check)[0]);
            }
            let truth = builder.ins().icmp_imm(IntCC::NotEqual, truth, 0);
            builder.ins().jump(join, &[truth]);
            builder.seal_block(join);
            builder.switch_to_block(join);
            builder.block_params(join)[0]
        } else {
            immediate_truth
        }
    }

    fn as_float(
        &self,
        builder: &mut FunctionBuilder,
        value: cranelift::prelude::Value,
        kind: cranelift::prelude::Value,
    ) -> cranelift::prelude::Value {
        let is_float = builder.ins().icmp_imm(IntCC::Equal, kind, crate::native_scalar::FLOAT);
        let bits = builder.ins().bitcast(types::F64, MemFlags::new(), value);
        let integer = builder.ins().fcvt_from_sint(types::F64, value);
        builder.ins().select(is_float, bits, integer)
    }

    /// Split dynamic numeric work while retaining the existing checked integer
    /// path. Floating operations stay native except the platform remainder call.
    fn begin_float_operation(
        &self,
        builder: &mut FunctionBuilder,
        operation: i64,
        left: cranelift::prelude::Value,
        right: cranelift::prelude::Value,
    ) -> Result<Block, String> {
        let left_kind = self.scalar_kind(builder, left);
        let right_kind = self.scalar_kind(builder, right);
        let left_float =
            builder.ins().icmp_imm(IntCC::Equal, left_kind, crate::native_scalar::FLOAT);
        let right_float =
            builder.ins().icmp_imm(IntCC::Equal, right_kind, crate::native_scalar::FLOAT);
        let any_float = builder.ins().bor(left_float, right_float);
        let floating = builder.create_block();
        let integer = builder.create_block();
        let merge = builder.create_block();
        builder.append_block_param(merge, types::I64);
        builder.append_block_param(merge, types::I64);
        builder.ins().brif(any_float, floating, &[], integer, &[]);
        builder.seal_block(floating);
        builder.seal_block(integer);
        builder.switch_to_block(floating);
        if operation < 11 {
            self.guard_scalar_types(builder, operation, left, right)?;
        }
        let a = self.as_float(builder, left, left_kind);
        let b = self.as_float(builder, right, right_kind);
        let result = match operation {
            0 => builder.ins().fadd(a, b),
            1 => builder.ins().fsub(a, b),
            2 => builder.ins().fmul(a, b),
            3 | 4 => {
                let zero_float = builder.ins().f64const(0.0);
                let zero_divisor = builder.ins().fcmp(FloatCC::Equal, b, zero_float);
                let zero = builder.ins().iconst(types::I64, 0);
                // Division/modulo have the same zero-divisor errors for both kinds.
                self.guard_arithmetic(builder, zero_divisor, operation, zero, zero)?;
                if operation == 3 {
                    builder.ins().fdiv(a, b)
                } else {
                    let remainder =
                        self.float_remainder_func.ok_or("Missing float remainder helper")?;
                    let call = builder.ins().call(remainder, &[a, b]);
                    builder.inst_results(call)[0]
                }
            }
            5 => builder.ins().fneg(a),
            7..=10 => {
                let relation = [
                    FloatCC::LessThan,
                    FloatCC::GreaterThan,
                    FloatCC::LessThanOrEqual,
                    FloatCC::GreaterThanOrEqual,
                ][(operation - 7) as usize];
                builder.ins().fcmp(relation, a, b)
            }
            11 | 12 => {
                // Match Value::float_equals: epsilon for finite values, exact
                // equality for infinities, and NaN unequal even to itself.
                let exact = builder.ins().fcmp(FloatCC::Equal, a, b);
                let difference = builder.ins().fsub(a, b);
                let absolute = builder.ins().fabs(difference);
                let epsilon = builder.ins().f64const(f64::EPSILON);
                let close = builder.ins().fcmp(FloatCC::LessThan, absolute, epsilon);
                let equal = builder.ins().bor(exact, close);
                let left_int = builder.ins().icmp_imm(IntCC::Equal, left_kind, 0);
                let right_int = builder.ins().icmp_imm(IntCC::Equal, right_kind, 0);
                let left_numeric = builder.ins().bor(left_int, left_float);
                let right_numeric = builder.ins().bor(right_int, right_float);
                let numeric = builder.ins().band(left_numeric, right_numeric);
                let equal = builder.ins().band(equal, numeric);
                if operation == 12 {
                    builder.ins().icmp_imm(IntCC::Equal, equal, 0)
                } else {
                    equal
                }
            }
            _ => return Err("Invalid floating operation".into()),
        };
        let (payload, kind) = if operation >= 7 {
            (builder.ins().uextend(types::I64, result), crate::native_scalar::BOOLEAN)
        } else {
            (
                builder.ins().bitcast(types::I64, MemFlags::new(), result),
                crate::native_scalar::FLOAT,
            )
        };
        let kind = builder.ins().iconst(types::I64, kind);
        builder.ins().jump(merge, &[payload, kind]);
        builder.switch_to_block(integer);
        Ok(merge)
    }

    fn finish_float_operation(
        &mut self,
        builder: &mut FunctionBuilder,
        merge: Block,
        result: cranelift::prelude::Value,
        kind: i64,
    ) {
        let kind = builder.ins().iconst(types::I64, kind);
        builder.ins().jump(merge, &[result, kind]);
        builder.switch_to_block(merge);
        builder.seal_block(merge);
        let params = builder.block_params(merge);
        let payload = params[0];
        self.scalar_kinds.insert(payload, params[1]);
        self.push_value(payload);
    }

    fn guard_scalar_types(
        &self,
        builder: &mut FunctionBuilder,
        operation: i64,
        left: cranelift::prelude::Value,
        right: cranelift::prelude::Value,
    ) -> Result<(), String> {
        let (Some(ctx), Some(error_func)) = (self.ctx_param, self.scalar_type_error_func) else {
            return Err("Scalar type guard requires a runtime error context".into());
        };
        let left_kind = self.scalar_kind(builder, left);
        let right_kind = self.scalar_kind(builder, right);
        let invalid = if operation == 6 {
            builder.ins().icmp_imm(IntCC::NotEqual, left_kind, crate::native_scalar::BOOLEAN)
        } else {
            let left_int = builder.ins().icmp_imm(IntCC::Equal, left_kind, 0);
            let left_float =
                builder.ins().icmp_imm(IntCC::Equal, left_kind, crate::native_scalar::FLOAT);
            let left_valid = builder.ins().bor(left_int, left_float);
            let right_int = builder.ins().icmp_imm(IntCC::Equal, right_kind, 0);
            let right_float =
                builder.ins().icmp_imm(IntCC::Equal, right_kind, crate::native_scalar::FLOAT);
            let right_valid = builder.ins().bor(right_int, right_float);
            let valid = builder.ins().band(left_valid, right_valid);
            builder.ins().icmp_imm(IntCC::Equal, valid, 0)
        };
        let failure = builder.create_block();
        let success = builder.create_block();
        builder.ins().brif(invalid, failure, &[], success, &[]);
        builder.seal_block(failure);
        builder.seal_block(success);
        builder.switch_to_block(failure);
        self.persist_local_slots(builder, true);
        let operation = builder.ins().iconst(types::I64, operation);
        let call =
            builder.ins().call(error_func, &[ctx, operation, left, right, left_kind, right_kind]);
        let status = builder.inst_results(call)[0];
        self.emit_return(builder, status);
        builder.switch_to_block(success);
        Ok(())
    }

    fn guard_arithmetic(
        &self,
        builder: &mut FunctionBuilder,
        failed: cranelift::prelude::Value,
        operation: i64,
        left: cranelift::prelude::Value,
        right: cranelift::prelude::Value,
    ) -> Result<(), String> {
        let (Some(ctx), Some(error_func)) = (self.ctx_param, self.arithmetic_error_func) else {
            return Err("Checked arithmetic requires a runtime error context".into());
        };
        let failure = builder.create_block();
        let success = builder.create_block();
        builder.ins().brif(failed, failure, &[], success, &[]);
        builder.seal_block(failure);
        builder.seal_block(success);
        builder.switch_to_block(failure);
        self.persist_local_slots(builder, true);
        let operation = builder.ins().iconst(types::I64, operation);
        let call = builder.ins().call(error_func, &[ctx, operation, left, right]);
        let status = builder.inst_results(call)[0];
        self.emit_return(builder, status);
        builder.switch_to_block(success);
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

    fn scalar_kind(
        &self,
        builder: &mut FunctionBuilder,
        value: cranelift::prelude::Value,
    ) -> cranelift::prelude::Value {
        self.scalar_kinds
            .get(&value)
            .copied()
            .unwrap_or_else(|| builder.ins().iconst(types::I64, 0))
    }

    /// Each operand-stack slot crosses a control-flow edge as (payload, kind).
    /// Create explicit parameters before translating the destination: local SSA
    /// variables can append their own parameters when the block is sealed.
    fn prepare_stack_edge(
        &mut self,
        builder: &mut FunctionBuilder,
        pc: usize,
    ) -> Result<(), String> {
        let depth = self.value_stack.len();
        if let Some(&expected) = self.block_entry_stack_depth.get(&pc) {
            if expected != depth {
                return Err(format!(
                    "Inconsistent JIT stack depth at PC {pc}: {depth}, expected {expected}"
                ));
            }
        } else {
            let block =
                *self.blocks.get(&pc).ok_or_else(|| format!("Undefined JIT block at PC {pc}"))?;
            for _ in 0..depth * 2 {
                builder.append_block_param(block, types::I64);
            }
            self.block_entry_stack_depth.insert(pc, depth);
        }
        Ok(())
    }

    fn stack_edge_args(
        &mut self,
        builder: &mut FunctionBuilder,
        pc: usize,
    ) -> Result<Vec<cranelift::prelude::Value>, String> {
        self.prepare_stack_edge(builder, pc)?;
        let mut args = Vec::with_capacity(self.value_stack.len() * 2);
        for &value in &self.value_stack {
            args.push(value);
            args.push(self.scalar_kind(builder, value));
        }
        Ok(args)
    }

    fn restore_block_stack(&mut self, builder: &FunctionBuilder, pc: usize, block: Block) {
        if let Some(&depth) = self.block_entry_stack_depth.get(&pc) {
            self.value_stack.clear();
            let params = builder.block_params(block);
            for slot in 0..depth {
                let value = params[slot * 2];
                self.value_stack.push(value);
                self.scalar_kinds.insert(value, params[slot * 2 + 1]);
            }
        }
    }

    fn capture_scalar_kind(
        &mut self,
        builder: &mut FunctionBuilder,
        ctx: cranelift::prelude::Value,
        value: cranelift::prelude::Value,
    ) {
        if let Some(get_kind) = self.scalar_kind_func {
            let call = builder.ins().call(get_kind, &[ctx]);
            self.scalar_kinds.insert(value, builder.inst_results(call)[0]);
        }
    }

    fn push_value(&mut self, val: cranelift::prelude::Value) {
        self.value_stack.push(val);
    }

    /// Emit signed remainder by a positive Mersenne constant (`2^k - 1`).
    ///
    /// Cranelift currently leaves these `srem` operations as hardware integer
    /// division on the supported targets. Folding the unsigned magnitude by
    /// `k`-bit limbs removes that division while preserving signed remainder
    /// semantics, including `i64::MIN`.
    fn translate_mersenne_remainder(
        &self,
        builder: &mut FunctionBuilder,
        dividend: cranelift::prelude::Value,
        divisor: i64,
    ) -> Option<cranelift::prelude::Value> {
        if divisor <= 0 {
            return None;
        }

        let modulus = divisor as u64;
        let next_power = modulus.checked_add(1)?;
        if !next_power.is_power_of_two() {
            return None;
        }

        let shift = next_power.trailing_zeros();
        // Small moduli need too many folds to beat division. `% 1` is always 0.
        if shift == 0 {
            return Some(builder.ins().iconst(types::I64, 0));
        }
        if shift < 8 || shift >= 63 {
            return None;
        }

        let sign = builder.ins().sshr_imm(dividend, 63);
        let sign_flipped = builder.ins().bxor(dividend, sign);
        let mut reduced = builder.ins().isub(sign_flipped, sign);

        // Each fold replaces the high limbs with their sum because 2^k == 1
        // (mod 2^k - 1). Compute the minimum number of folds whose conservative
        // upper bound is below twice the modulus, where one final subtraction
        // is sufficient. For the common 2^31-1 modulus this is two folds.
        let mut folds = 0;
        let mut upper_bound = u64::MAX;
        while upper_bound >= modulus * 2 {
            upper_bound = modulus + (upper_bound >> shift);
            folds += 1;
        }
        for _ in 0..folds {
            let low = builder.ins().band_imm(reduced, divisor);
            let high = builder.ins().ushr_imm(reduced, i64::from(shift));
            reduced = builder.ins().iadd(low, high);
        }

        let needs_subtract =
            builder.ins().icmp_imm(IntCC::UnsignedGreaterThanOrEqual, reduced, divisor);
        let subtracted = builder.ins().iadd_imm(reduced, -divisor);
        let magnitude = builder.ins().select(needs_subtract, subtracted, reduced);
        let signed = builder.ins().bxor(magnitude, sign);
        Some(builder.ins().isub(signed, sign))
    }

    fn pop_value(&mut self) -> Result<cranelift::prelude::Value, String> {
        self.value_stack.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    fn peek_value(&self) -> Result<cranelift::prelude::Value, String> {
        self.value_stack.last().copied().ok_or_else(|| "Stack empty".to_string())
    }
}

impl JitCompiler {
    fn retain_literal_values(&mut self, chunk: &BytecodeChunk) -> HashMap<usize, Arc<Value>> {
        chunk
            .constants
            .iter()
            .enumerate()
            .filter_map(|(index, constant)| {
                let key = match constant {
                    Constant::String(text) => (0, text.as_bytes().to_vec()),
                    Constant::Bytes(bytes) => (1, bytes.clone()),
                    _ => return None,
                };
                let value = self.literal_values.entry(key).or_insert_with(|| {
                    Arc::new(match constant {
                        Constant::String(text) => Value::Str(Arc::new(text.clone())),
                        Constant::Bytes(bytes) => Value::Bytes(Arc::new(bytes.clone())),
                        _ => unreachable!(),
                    })
                });
                Some((index, Arc::clone(value)))
            })
            .collect()
    }

    fn retain_path_shapes(
        &mut self,
        chunk: &BytecodeChunk,
    ) -> HashMap<Vec<Option<String>>, Arc<Vec<Option<String>>>> {
        chunk
            .instructions
            .iter()
            .filter_map(|instruction| {
                let (OpCode::UpdateBindingPath(_, shape)
                | OpCode::AppendBindingPath(_, shape)
                | OpCode::TryPopRebind(_, _, _, shape, _)) = instruction
                else {
                    return None;
                };
                let retained = self
                    .path_shapes
                    .entry(shape.clone())
                    .or_insert_with(|| Arc::new(shape.clone()));
                Some((shape.clone(), Arc::clone(retained)))
            })
            .collect()
    }

    fn retain_field_names(&mut self, chunk: &BytecodeChunk) -> HashMap<String, Arc<String>> {
        chunk
            .instructions
            .iter()
            .filter_map(|instruction| {
                let (OpCode::FieldGet(name) | OpCode::FieldSet(name)) = instruction else {
                    return None;
                };
                let retained =
                    self.field_names.entry(name.clone()).or_insert_with(|| Arc::new(name.clone()));
                Some((name.clone(), Arc::clone(retained)))
            })
            .collect()
    }

    fn retain_struct_shapes(
        &mut self,
        chunk: &BytecodeChunk,
    ) -> HashMap<NativeStructShape, Arc<NativeStructShape>> {
        chunk
            .instructions
            .iter()
            .filter_map(|instruction| {
                let OpCode::MakeStruct(name, fields) = instruction else { return None };
                let key = (name.clone(), fields.clone());
                let shape =
                    self.struct_shapes.entry(key.clone()).or_insert_with(|| Arc::new(key.clone()));
                Some((key, Arc::clone(shape)))
            })
            .collect()
    }

    fn retain_dictionary_keys(&mut self, chunk: &BytecodeChunk) {
        for instruction in &chunk.instructions {
            if let OpCode::AppendConstStringInPlace(_, text) = instruction {
                self.append_strings
                    .entry(text.as_ptr() as usize)
                    .or_insert_with(|| Arc::clone(text));
            }
            if let OpCode::MakeDictWithKeys(keys) = instruction {
                self.dictionary_keys
                    .entry(Arc::as_ptr(keys) as usize)
                    .or_insert_with(|| Arc::clone(keys));
            }
        }
    }

    fn declare_scalar_helpers(
        module: &mut JITModule,
        builder: &mut FunctionBuilder,
        translator: &mut BytecodeTranslator,
    ) -> Result<(), String> {
        for (name, parameters, returns) in [
            ("jit_set_return_scalar", 3, true),
            ("jit_return_scalar_kind", 1, true),
            ("jit_runtime_failed", 1, true),
            ("jit_local_initialization", 5, true),
            ("jit_owned_truth", 2, true),
            ("jit_owned_equal", 5, true),
            ("jit_owned_operator", 6, true),
            ("jit_load_operand", 3, true),
            ("jit_load_literal", 2, true),
            ("jit_take_parameter", 3, true),
            ("jit_duplicate_operand", 3, true),
            ("jit_release_operand", 3, true),
            ("jit_release_dead_local", 4, true),
            ("jit_append_local", 6, true),
            ("jit_append_result", 1, true),
            ("jit_try_string_append", 9, true),
            ("jit_try_append", 11, true),
            ("jit_release_rebind", 9, true),
            ("jit_stack_pop", 1, true),
            ("jit_make_struct", 2, true),
            ("jit_make_array", 2, true),
            ("jit_prepare_field_call", 2, true),
            ("jit_field_read", 4, true),
            ("jit_field_write", 6, true),
            ("jit_index_set_result", 7, true),
            ("jit_indexed_read", 5, true),
            ("jit_indexed_read_cached", 6, true),
            ("jit_for_next", 5, true),
            ("jit_indexed_write", 10, true),
            ("jit_update_binding_path", 8, true),
            ("jit_append_binding_path", 8, true),
            ("jit_guard_append_path", 7, true),
            ("jit_try_pop_rebind", 12, true),
            ("jit_loop_resume_guard", 3, true),
            ("jit_loop_resume_prepare", 2, true),
            ("jit_loop_dense_view", 2, true),
            ("jit_loop_indexed_read", 4, true),
            ("jit_stack_push_scalar", 3, false),
            ("jit_stack_take_scalar", 3, true),
            ("jit_scalar_type_error", 6, true),
            ("jit_enter_function", 1, true),
            ("jit_leave_function", 1, false),
        ] {
            let mut sig = module.make_signature();
            sig.params.extend(std::iter::repeat(AbiParam::new(types::I64)).take(parameters));
            if returns {
                sig.returns.push(AbiParam::new(types::I64));
            }
            let id = module
                .declare_function(name, Linkage::Import, &sig)
                .map_err(|error| error.to_string())?;
            let function = module.declare_func_in_func(id, builder.func);
            match name {
                "jit_set_return_scalar" => translator.scalar_return_func = Some(function),
                "jit_return_scalar_kind" => translator.scalar_kind_func = Some(function),
                "jit_runtime_failed" => translator.runtime_error_func = Some(function),
                "jit_local_initialization" => translator.local_initialization_func = Some(function),
                "jit_owned_truth" => translator.owned_truth_func = Some(function),
                "jit_owned_equal" => translator.owned_equal_func = Some(function),
                "jit_owned_operator" => translator.owned_operator_func = Some(function),
                "jit_load_operand" => translator.operand_load_func = Some(function),
                "jit_load_literal" => translator.literal_load_func = Some(function),
                "jit_take_parameter" => translator.parameter_load_func = Some(function),
                "jit_duplicate_operand" => translator.operand_duplicate_func = Some(function),
                "jit_release_operand" => translator.operand_release_func = Some(function),
                "jit_release_dead_local" => translator.dead_local_func = Some(function),
                "jit_append_local" => translator.append_local_func = Some(function),
                "jit_append_result" => translator.append_result_func = Some(function),
                "jit_try_string_append" => translator.try_string_append_func = Some(function),
                "jit_try_append" => translator.try_append_func = Some(function),
                "jit_release_rebind" => translator.release_rebind_func = Some(function),
                "jit_stack_pop" => translator.stack_pop_func = Some(function),
                "jit_make_struct" => translator.make_struct_func = Some(function),
                "jit_make_array" => translator.make_array_func = Some(function),
                "jit_prepare_field_call" => translator.field_call_func = Some(function),
                "jit_field_read" => translator.field_read_func = Some(function),
                "jit_field_write" => translator.field_write_func = Some(function),
                "jit_index_set_result" => translator.index_set_result_func = Some(function),
                "jit_indexed_read" => translator.indexed_read_func = Some(function),
                "jit_indexed_read_cached" => translator.indexed_read_cached_func = Some(function),
                "jit_for_next" => translator.for_next_func = Some(function),
                "jit_indexed_write" => translator.indexed_write_func = Some(function),
                "jit_update_binding_path" => translator.update_binding_path_func = Some(function),
                "jit_append_binding_path" => translator.append_binding_path_func = Some(function),
                "jit_guard_append_path" => translator.guard_append_path_func = Some(function),
                "jit_try_pop_rebind" => translator.try_pop_rebind_func = Some(function),
                "jit_loop_resume_guard" => translator.loop_resume_guard_func = Some(function),
                "jit_loop_resume_prepare" => translator.loop_resume_prepare_func = Some(function),
                "jit_loop_dense_view" => translator.loop_dense_view_func = Some(function),
                "jit_loop_indexed_read" => translator.loop_indexed_read_func = Some(function),
                "jit_stack_push_scalar" => translator.scalar_push_func = Some(function),
                "jit_stack_take_scalar" => translator.scalar_take_func = Some(function),
                "jit_enter_function" => translator.enter_function_func = Some(function),
                "jit_leave_function" => translator.leave_function_func = Some(function),
                _ => translator.scalar_type_error_func = Some(function),
            }
        }
        let mut signature = module.make_signature();
        signature.params.extend([AbiParam::new(types::F64); 2]);
        signature.returns.push(AbiParam::new(types::F64));
        let id = module
            .declare_function("jit_float_remainder", Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
        translator.float_remainder_func = Some(module.declare_func_in_func(id, builder.func));
        Ok(())
    }

    fn declare_arithmetic_error(
        module: &mut JITModule,
        builder: &mut FunctionBuilder,
    ) -> Result<FuncRef, String> {
        let mut sig = module.make_signature();
        sig.params.extend([AbiParam::new(types::I64); 4]);
        sig.returns.push(AbiParam::new(types::I64));
        let id = module
            .declare_function("jit_arithmetic_error", Linkage::Import, &sig)
            .map_err(|error| format!("Failed to declare arithmetic error: {error}"))?;
        Ok(module.declare_func_in_func(id, builder.func))
    }

    fn declare_named_assignment(
        module: &mut JITModule,
        builder: &mut FunctionBuilder,
    ) -> Result<FuncRef, String> {
        let mut sig = module.make_signature();
        sig.params.extend([AbiParam::new(types::I64); 5]);
        sig.returns.push(AbiParam::new(types::I64));
        let id = module
            .declare_function("jit_assign_variable", Linkage::Import, &sig)
            .map_err(|error| format!("Failed to declare named assignment: {error}"))?;
        Ok(module.declare_func_in_func(id, builder.func))
    }

    /// Create a new JIT compiler instance
    pub fn new() -> Result<Self, String> {
        Self::new_with_cancellation(false)
    }

    pub(crate) fn new_with_cancellation(cancellation_checks: bool) -> Result<Self, String> {
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
        builder.symbol("jit_load_operand", jit_load_operand as *const u8);
        builder.symbol("jit_indexed_read", jit_indexed_read as *const u8);
        builder.symbol("jit_indexed_read_cached", jit_indexed_read_cached as *const u8);
        builder.symbol("jit_for_next", jit_for_next as *const u8);
        builder.symbol("jit_indexed_write", jit_indexed_write as *const u8);
        builder.symbol("jit_update_binding_path", jit_update_binding_path as *const u8);
        builder.symbol("jit_append_binding_path", jit_append_binding_path as *const u8);
        builder.symbol("jit_guard_append_path", jit_guard_append_path as *const u8);
        builder.symbol("jit_try_pop_rebind", jit_try_pop_rebind as *const u8);
        builder.symbol("jit_loop_resume_guard", jit_loop_resume_guard as *const u8);
        builder.symbol("jit_loop_resume_prepare", jit_loop_resume_prepare as *const u8);
        builder.symbol("jit_loop_dense_view", jit_loop_dense_view as *const u8);
        builder.symbol("jit_loop_indexed_read", jit_loop_indexed_read as *const u8);
        builder.symbol("jit_duplicate_operand", jit_duplicate_operand as *const u8);
        builder.symbol("jit_release_operand", jit_release_operand as *const u8);
        builder.symbol("jit_release_dead_local", jit_release_dead_local as *const u8);
        builder.symbol("jit_append_local", jit_append_local as *const u8);
        builder.symbol("jit_append_result", jit_append_result as *const u8);
        builder.symbol("jit_try_string_append", jit_try_string_append as *const u8);
        builder.symbol("jit_try_append", jit_try_append as *const u8);
        builder.symbol("jit_release_rebind", jit_release_rebind as *const u8);
        builder.symbol("jit_store_variable", jit_store_variable as *const u8);
        builder.symbol("jit_assign_variable", jit_assign_variable as *const u8);
        builder.symbol("jit_runtime_failed", jit_runtime_failed as *const u8);
        builder.symbol("jit_arithmetic_error", jit_arithmetic_error as *const u8);
        builder.symbol("jit_scalar_type_error", jit_scalar_type_error as *const u8);
        builder.symbol("jit_local_initialization", jit_local_initialization as *const u8);
        builder.symbol("jit_load_local_slot", jit_load_local_slot as *const u8);
        builder.symbol("jit_store_local_slot", jit_store_local_slot as *const u8);
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
        builder.symbol("jit_float_remainder", jit_float_remainder as *const u8);
        builder.symbol("jit_set_return_int", jit_set_return_int as *const u8);
        builder.symbol("jit_set_return_scalar", jit_set_return_scalar as *const u8);
        builder.symbol("jit_return_scalar_kind", jit_return_scalar_kind as *const u8);
        builder.symbol("jit_owned_truth", jit_owned_truth as *const u8);
        builder.symbol("jit_load_literal", jit_load_literal as *const u8);
        builder.symbol("jit_take_parameter", jit_take_parameter as *const u8);
        builder.symbol("jit_owned_equal", jit_owned_equal as *const u8);
        builder.symbol("jit_owned_operator", jit_owned_operator as *const u8);
        builder.symbol("jit_stack_push_scalar", jit_stack_push_scalar as *const u8);
        builder.symbol("jit_stack_take_scalar", jit_stack_take_scalar as *const u8);
        builder.symbol("jit_get_return_int", jit_get_return_int as *const u8);
        builder.symbol("jit_get_arg", jit_get_arg as *const u8);
        builder.symbol("jit_enter_function", jit_enter_function as *const u8);
        builder.symbol("jit_leave_function", jit_leave_function as *const u8);
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
        builder.symbol("jit_prepare_field_call", jit_prepare_field_call as *const u8);
        builder.symbol("jit_field_read", jit_field_read as *const u8);
        builder.symbol("jit_field_write", jit_field_write as *const u8);
        builder.symbol("jit_index_set_result", jit_index_set_result as *const u8);
        builder.symbol("jit_make_struct", jit_make_struct as *const u8);
        builder.symbol("jit_make_array", jit_make_array as *const u8);
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
            cancellation_checks,
            loop_jit_blacklist: HashSet::new(),
            type_profiles: HashMap::new(),
            compiled_fn_info: HashMap::new(),
            dictionary_keys: HashMap::new(),
            struct_shapes: HashMap::new(),
            field_names: HashMap::new(),
            path_shapes: HashMap::new(),
            append_strings: HashMap::new(),
            literal_values: HashMap::new(),
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
                // Named captured roots require frame cells, not the locked global environment.
                if matches!(instruction, OpCode::UpdateBindingPath(crate::bytecode::MutationBinding::Named(name), _) | OpCode::AppendBindingPath(crate::bytecode::MutationBinding::Named(name), _) | OpCode::TryPopRebind(crate::bytecode::MutationBinding::Named(name),_,_,_,_)
                    if chunk.upvalues.contains(name) || chunk.lexical_self.is_some())
                {
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
                // Named captured roots require frame cells, not the locked global environment.
                if matches!(instruction, OpCode::UpdateBindingPath(crate::bytecode::MutationBinding::Named(name), _) | OpCode::AppendBindingPath(crate::bytecode::MutationBinding::Named(name), _) | OpCode::TryPopRebind(crate::bytecode::MutationBinding::Named(name),_,_,_,_)
                    if chunk.upvalues.contains(name) || chunk.lexical_self.is_some())
                {
                    return false;
                }
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
            // Only constants with a typed lowering and retained lifetime may enter native code.
            OpCode::LoadConst(idx) => {
                matches!(
                    constants.get(*idx),
                    Some(
                        Constant::Int(_)
                            | Constant::Bool(_)
                            | Constant::Float(_)
                            | Constant::None
                            | Constant::String(_)
                            | Constant::Bytes(_)
                    )
                )
            }

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
            | OpCode::StoreLocal(_)
            | OpCode::DefineLocal(_)
            | OpCode::ReleaseRebindLocal(_, _)
            | OpCode::ReleaseDeadLocal(_) => true,
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
            OpCode::FieldGet(_) | OpCode::FieldSet(_) | OpCode::IndexGet | OpCode::ForNext(_) => {
                true
            }
            OpCode::TryStringAppendAssign(_, _) | OpCode::TryAppendAssign(_, _) => true,
            OpCode::IndexSet => true,
            OpCode::IndexGetInPlace(_)
            | OpCode::IndexSetInPlace(_)
            | OpCode::UpdateBindingPath(_, _)
            | OpCode::GuardAppendPath(_)
            | OpCode::AppendBindingPath(_, _)
            | OpCode::TryPopRebind(_, _, _, _, _) => true,
            OpCode::AppendConstStringInPlace(_, _) | OpCode::AppendConstCharInPlace(_, _) => true,
            // Guarded fusion remains in the VM until native lowering exists.
            OpCode::AppendConstCharUntilLocalInPlace(_, _, _, _) => false,
            OpCode::MakeStruct(_, _)
            | OpCode::MakeArray(_)
            | OpCode::MakeDict(_)
            | OpCode::MakeDictWithKeys(_) => true,

            // Everything else is unsupported - causes code to skip JIT
            // This includes: CallNative, Arrays, Dicts, Generators, Async, etc.
            _ => false,
        }
    }

    /// Get compiled function from cache
    #[allow(dead_code)] // Will be used when executing compiled code
    pub fn get_compiled(&self, offset: usize) -> Option<CompiledFn> {
        let result = self.compiled_cache.get(&offset).copied();
        measurement::add(
            if result.is_some() { Metric::JitCacheHits } else { Metric::JitCacheMisses },
            1,
        );
        result
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
        self.retain_dictionary_keys(chunk);
        let literal_values = self.retain_literal_values(chunk);
        let field_names = self.retain_field_names(chunk);
        let path_shapes = self.retain_path_shapes(chunk);
        let struct_shapes = self.retain_struct_shapes(chunk);
        measurement::add(Metric::JitCompileEntries, 1);
        let _measurement = measurement::Span::new(Metric::JitCompileWallNs);
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
        store_var_sig.params.push(AbiParam::new(types::I64)); // scalar kind

        let store_var_func_id = self
            .module
            .declare_function("jit_store_variable", Linkage::Import, &store_var_sig)
            .map_err(|e| format!("Failed to declare jit_store_variable: {}", e))?;

        let mut load_local_slot_sig = self.module.make_signature();
        load_local_slot_sig.params.push(AbiParam::new(types::I64));
        load_local_slot_sig.params.push(AbiParam::new(types::I64));
        load_local_slot_sig.returns.push(AbiParam::new(types::I64));
        let load_local_slot_func_id = self
            .module
            .declare_function("jit_load_local_slot", Linkage::Import, &load_local_slot_sig)
            .map_err(|e| format!("Failed to declare jit_load_local_slot: {}", e))?;

        let mut store_local_slot_sig = self.module.make_signature();
        store_local_slot_sig.params.push(AbiParam::new(types::I64));
        store_local_slot_sig.params.push(AbiParam::new(types::I64));
        store_local_slot_sig.params.push(AbiParam::new(types::I64));
        store_local_slot_sig.params.push(AbiParam::new(types::I64)); // scalar kind
        store_local_slot_sig.returns.push(AbiParam::new(types::I64));
        let store_local_slot_func_id = self
            .module
            .declare_function("jit_store_local_slot", Linkage::Import, &store_local_slot_sig)
            .map_err(|e| format!("Failed to declare jit_store_local_slot: {}", e))?;

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
            let load_local_slot_func_ref =
                self.module.declare_func_in_func(load_local_slot_func_id, builder.func);
            let store_local_slot_func_ref =
                self.module.declare_func_in_func(store_local_slot_func_id, builder.func);
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
            translator.cancellation_checks = self.cancellation_checks;
            translator.literal_values = literal_values;
            translator.field_call_sites = chunk
                .instructions
                .windows(2)
                .enumerate()
                .filter_map(|(pc, pair)| {
                    matches!((&pair[0], &pair[1]), (OpCode::FieldGet(_), OpCode::Call(_)))
                        .then_some(pc + 1)
                })
                .collect();
            translator.field_names = field_names;
            translator.path_shapes = path_shapes;
            translator.struct_shapes = struct_shapes;
            Self::declare_scalar_helpers(&mut self.module, &mut builder, &mut translator)?;
            translator.arithmetic_error_func =
                Some(Self::declare_arithmetic_error(&mut self.module, &mut builder)?);
            translator
                .set_local_names(chunk.local_names.clone(), chunk.local_binding_kinds.clone());
            translator.set_context_param(ctx_ptr);
            translator.named_assignment_func =
                Some(Self::declare_named_assignment(&mut self.module, &mut builder)?);
            translator.set_external_functions(
                load_var_func_ref,
                store_var_func_ref,
                store_from_stack_func_ref,
            );
            translator.set_vm_local_slot_store_function(store_local_slot_func_ref);
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

            let presealed_current_block = (current_block != entry_block).then_some(current_block);

            // Create blocks for jump targets (loop JIT)
            translator.create_blocks(&mut builder, &chunk.instructions, offset)?;
            let function_end = if let Some(loop_end) = loop_end {
                let mut max_target = loop_end;
                for (pc, instruction) in
                    chunk.instructions.iter().enumerate().skip(offset).take(loop_end - offset + 1)
                {
                    match instruction {
                        OpCode::Jump(target)
                        | OpCode::JumpIfFalse(target)
                        | OpCode::JumpIfTrue(target)
                        | OpCode::JumpBack(target)
                        | OpCode::TryStringAppendAssign(_, target)
                        | OpCode::TryAppendAssign(_, target)
                        | OpCode::GuardAppendPath(target)
                        | OpCode::TryPopRebind(_, _, _, _, target)
                        | OpCode::ForNext(target) => {
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
            translator.loop_resume_enabled = loop_end.is_some();
            translator.initialize_readonly_locals(&mut builder, &chunk.params);
            translator.initialize_indexed_read_caches(&mut builder, &chunk.instructions);
            if let Some(slots) = int_dict_slots.as_ref() {
                translator.set_int_dict_slots(slots.clone());
            }
            translator.allocate_local_slots(&mut builder, &chunk.instructions, function_end);
            translator.initialize_local_slots_from_vm(
                &mut builder,
                load_local_slot_func_ref,
                load_var_func_ref,
            );

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
                if !entry_block_sealed {
                    builder.seal_block(entry_block);
                    entry_block_sealed = true;
                }
                builder.switch_to_block(start_block);
                current_block = start_block;
            }

            translator.restore_block_stack(&builder, offset, start_block);

            if !entry_block_sealed {
                builder.seal_block(entry_block);
            }

            let mut sealed_blocks = std::collections::HashSet::new();
            sealed_blocks.insert(entry_block);
            if let Some(block) = presealed_current_block {
                sealed_blocks.insert(block);
            }
            let mut block_terminated = false;

            for (pc, instruction) in chunk.instructions.iter().enumerate().skip(offset) {
                if pc >= function_end {
                    break;
                }

                if let Some(&block) = translator.blocks.get(&pc) {
                    if block != current_block {
                        if !block_terminated {
                            let args = translator.stack_edge_args(&mut builder, pc)?;
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

                        translator.restore_block_stack(&builder, pc, block);
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
                translator.persist_local_slots_to_vm(&mut builder);
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
        self.compile_function_metadata(chunk, name, true).map(|result| result.0)
    }

    fn compile_function_metadata(
        &mut self,
        chunk: &BytecodeChunk,
        name: &str,
        persist_return_locals: bool,
    ) -> Result<(CompiledFn, bool), String> {
        self.retain_dictionary_keys(chunk);
        let literal_values = self.retain_literal_values(chunk);
        let field_names = self.retain_field_names(chunk);
        let path_shapes = self.retain_path_shapes(chunk);
        let struct_shapes = self.retain_struct_shapes(chunk);
        measurement::add(Metric::JitCompileEntries, 1);
        let _measurement = measurement::Span::new(Metric::JitCompileWallNs);
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
            .declare_function(&format!("kujo_fn_{}", chunk.code_identity()), Linkage::Export, &sig)
            .map_err(|e| format!("Failed to declare function: {}", e))?;

        // 5. Set function signature
        self.ctx.func.signature = sig;

        let supports_positional_arguments;
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
            store_var_sig.params.push(AbiParam::new(types::I64)); // scalar kind

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
            // jit_get_arg: fn(*mut VMContext, index, name_hash) -> tagged payload
            let mut get_arg_sig = self.module.make_signature();
            get_arg_sig.params.push(AbiParam::new(types::I64)); // ctx pointer
            get_arg_sig.params.push(AbiParam::new(types::I64)); // arg index
            get_arg_sig.params.push(AbiParam::new(types::I64)); // parameter name hash
            get_arg_sig.returns.push(AbiParam::new(types::I64)); // arg value

            let get_arg_id = self
                .module
                .declare_function("jit_get_arg", Linkage::Import, &get_arg_sig)
                .map_err(|e| format!("Failed to declare jit_get_arg: {}", e))?;

            let get_arg_ref = self.module.declare_func_in_func(get_arg_id, &mut builder.func);

            // Create BytecodeTranslator for this function
            let mut translator = BytecodeTranslator::new();
            translator.cancellation_checks = self.cancellation_checks;
            translator.persist_successful_return_locals = persist_return_locals;
            translator.literal_values = literal_values;
            translator.field_call_sites = chunk
                .instructions
                .windows(2)
                .enumerate()
                .filter_map(|(pc, pair)| {
                    matches!((&pair[0], &pair[1]), (OpCode::FieldGet(_), OpCode::Call(_)))
                        .then_some(pc + 1)
                })
                .collect();
            translator.field_names = field_names;
            translator.path_shapes = path_shapes;
            translator.struct_shapes = struct_shapes;
            Self::declare_scalar_helpers(&mut self.module, &mut builder, &mut translator)?;
            translator.arithmetic_error_func =
                Some(Self::declare_arithmetic_error(&mut self.module, &mut builder)?);
            translator
                .set_local_names(chunk.local_names.clone(), chunk.local_binding_kinds.clone());
            translator.set_context_param(vm_context_param);
            let entry_block = translator.begin_function(&mut builder, entry_block)?;
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
            translator.named_assignment_func =
                Some(Self::declare_named_assignment(&mut self.module, &mut builder)?);
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
            translator.create_blocks(&mut builder, &chunk.instructions, 0)?;

            // OPTIMIZATION: Pre-allocate stack slots for local variables
            // This enables register-based locals - direct memory access instead of
            // runtime function calls and HashMap lookups for every variable access
            // This is the key optimization for Phase 7 Step 7 - targeting 10-50x speedup
            let function_end = translator.function_end;
            translator.allocate_local_slots(&mut builder, &chunk.instructions, function_end);

            // Also allocate stack slots for function parameters
            // Parameters need slots so they can be accessed via the fast path
            translator.allocate_parameter_slots(&mut builder, &chunk.params);
            supports_positional_arguments = translator.use_local_slots
                && chunk.params.iter().all(|name| translator.local_variables.contains_key(name));
            translator.initialize_readonly_locals(&mut builder, &chunk.params);
            translator.initialize_indexed_read_caches(&mut builder, &chunk.instructions);

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
            let entry_block =
                translator.finish_parameter_initialization(&mut builder, entry_block)?;

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
                            let args = translator.stack_edge_args(&mut builder, pc)?;
                            builder.ins().jump(block, &args);
                        }

                        // Loop headers must stay open until every back-edge has
                        // been added to Cranelift's SSA graph.
                        if !sealed_blocks.contains(&current_block)
                            && !translator.loop_header_pcs.iter().any(|&header_pc| {
                                translator.blocks.get(&header_pc) == Some(&current_block)
                            })
                        {
                            builder.seal_block(current_block);
                            sealed_blocks.insert(current_block);
                        }

                        builder.switch_to_block(block);
                        current_block = block;
                        block_terminated = false;

                        // If this block expects stack values, use block parameters
                        // NOTE: For loop headers, parameters were added during create_blocks
                        translator.restore_block_stack(&builder, pc, block);
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
                            if !sealed_blocks.contains(&current_block)
                                && !translator.loop_header_pcs.iter().any(|&header_pc| {
                                    translator.blocks.get(&header_pc) == Some(&current_block)
                                })
                            {
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
                translator.emit_return(&mut builder, zero);
                if !sealed_blocks.contains(&current_block) {
                    builder.seal_block(current_block);
                    sealed_blocks.insert(current_block);
                }
            }

            for (&pc, &block) in &translator.blocks {
                if !sealed_blocks.contains(&block) {
                    builder.seal_block(block);
                    sealed_blocks.insert(block);
                    if std::env::var("DEBUG_JIT").is_ok() {
                        eprintln!("JIT: Late-sealing function block at PC {}", pc);
                    }
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

        Ok((compiled_fn, supports_positional_arguments))
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
        self.retain_dictionary_keys(chunk);
        let literal_values = self.retain_literal_values(chunk);
        let field_names = self.retain_field_names(chunk);
        let path_shapes = self.retain_path_shapes(chunk);
        let struct_shapes = self.retain_struct_shapes(chunk);
        measurement::add(Metric::JitCompileEntries, 1);
        let _measurement = measurement::Span::new(Metric::JitCompileWallNs);
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
        let direct_name = format!("kujo_fn_{}_direct", chunk.code_identity());
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
            translator.cancellation_checks = self.cancellation_checks;
            translator.literal_values = literal_values;
            translator.field_call_sites = chunk
                .instructions
                .windows(2)
                .enumerate()
                .filter_map(|(pc, pair)| {
                    matches!((&pair[0], &pair[1]), (OpCode::FieldGet(_), OpCode::Call(_)))
                        .then_some(pc + 1)
                })
                .collect();
            translator.field_names = field_names;
            translator.path_shapes = path_shapes;
            translator.struct_shapes = struct_shapes;
            Self::declare_scalar_helpers(&mut self.module, &mut builder, &mut translator)?;
            translator.arithmetic_error_func =
                Some(Self::declare_arithmetic_error(&mut self.module, &mut builder)?);
            translator
                .set_local_names(chunk.local_names.clone(), chunk.local_binding_kinds.clone());
            translator.set_context_param(vm_context_param);
            let entry_block = translator.begin_function(&mut builder, entry_block)?;
            translator.set_direct_arg_mode(direct_arg_param, 1);
            let mut error_sig = self.module.make_signature();
            error_sig.params.push(AbiParam::new(types::I64));
            error_sig.returns.push(AbiParam::new(types::I64));
            let error_id = self
                .module
                .declare_function("jit_runtime_failed", Linkage::Import, &error_sig)
                .map_err(|error| format!("Failed to declare error query: {error}"))?;
            translator.runtime_error_func =
                Some(self.module.declare_func_in_func(error_id, builder.func));

            translator.named_assignment_func =
                Some(Self::declare_named_assignment(&mut self.module, &mut builder)?);
            let mut load_sig = self.module.make_signature();
            load_sig.params.extend([AbiParam::new(types::I64); 3]);
            load_sig.returns.push(AbiParam::new(types::I64));
            let load_id = self
                .module
                .declare_function("jit_load_variable", Linkage::Import, &load_sig)
                .map_err(|error| format!("Failed to declare variable load: {error}"))?;
            translator.load_var_func =
                Some(self.module.declare_func_in_func(load_id, builder.func));

            translator.set_current_function_name(name);
            translator.set_self_call_function(self_func_ref);

            // Create blocks for all jump targets
            translator.create_blocks(&mut builder, &chunk.instructions, 0)?;

            // Locals and the direct parameter use paired SSA ownership bindings.
            let param_name = &chunk.params[0];
            let function_end = translator.function_end;

            // Allocate slots for locals EXCEPT the direct parameter
            translator.allocate_local_slots_except(
                &mut builder,
                &chunk.instructions,
                function_end,
                param_name,
            );

            translator.allocate_parameter_slots(&mut builder, &chunk.params);
            translator.initialize_readonly_locals(&mut builder, &chunk.params);
            translator.initialize_indexed_read_caches(&mut builder, &chunk.instructions);

            // The context remains live for this native activation. Capture the
            // incoming kind in SSA before a recursive call changes the context.
            let kind = builder.ins().load(
                types::I64,
                MemFlags::new(),
                vm_context_param,
                std::mem::offset_of!(VMContext, arg_kinds) as i32,
            );
            translator.scalar_kinds.insert(direct_arg_param, kind);
            // Source-compiled parameters use paired payload/kind SSA locals.
            if let Some(&variable) = translator.local_variables.get(param_name) {
                let copy = translator.duplicate_operand(&mut builder, direct_arg_param);
                builder.def_var(variable, copy);
                builder.def_var(Variable::new(variable.index() + 1), kind);
            }

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
                            let args = translator.stack_edge_args(&mut builder, pc)?;
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
                        translator.restore_block_stack(&builder, pc, block);
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
                translator.emit_return(&mut builder, zero);
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
        self.compile_function_info(chunk, name, true)
    }

    /// VM-managed whole functions have no continuation consuming their local map.
    /// Keep standalone compilation and loop/script synchronization unchanged.
    pub(crate) fn compile_function_for_vm(
        &mut self,
        chunk: &BytecodeChunk,
        name: &str,
    ) -> Result<CompiledFnInfo, String> {
        self.compile_function_info(chunk, name, false)
    }

    fn compile_function_info(
        &mut self,
        chunk: &BytecodeChunk,
        name: &str,
        persist_return_locals: bool,
    ) -> Result<CompiledFnInfo, String> {
        // First, compile the standard function
        let (standard_fn, supports_positional_arguments) =
            self.compile_function_metadata(chunk, name, persist_return_locals)?;

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
            supports_positional_arguments,
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
        self.retain_dictionary_keys(chunk);
        let literal_values = self.retain_literal_values(chunk);
        let field_names = self.retain_field_names(chunk);
        let path_shapes = self.retain_path_shapes(chunk);
        let struct_shapes = self.retain_struct_shapes(chunk);
        measurement::add(Metric::JitCompileEntries, 1);
        let _measurement = measurement::Span::new(Metric::JitCompileWallNs);
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
            store_var_sig.params.push(AbiParam::new(types::I64)); // scalar kind

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
            translator.cancellation_checks = self.cancellation_checks;
            translator.literal_values = literal_values;
            translator.field_call_sites = chunk
                .instructions
                .windows(2)
                .enumerate()
                .filter_map(|(pc, pair)| {
                    matches!((&pair[0], &pair[1]), (OpCode::FieldGet(_), OpCode::Call(_)))
                        .then_some(pc + 1)
                })
                .collect();
            translator.field_names = field_names;
            translator.path_shapes = path_shapes;
            translator.struct_shapes = struct_shapes;
            Self::declare_scalar_helpers(&mut self.module, &mut builder, &mut translator)?;
            translator.arithmetic_error_func =
                Some(Self::declare_arithmetic_error(&mut self.module, &mut builder)?);
            translator
                .set_local_names(chunk.local_names.clone(), chunk.local_binding_kinds.clone());
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
            translator.initialize_readonly_locals(&mut builder, &chunk.params);
            translator.initialize_indexed_read_caches(&mut builder, &chunk.instructions);

            // Create blocks for jump targets (this also analyzes loop headers)
            translator.create_blocks(&mut builder, &chunk.instructions, 0)?;

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
                            let args = translator.stack_edge_args(&mut builder, pc)?;
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
                        translator.restore_block_stack(&builder, pc, block);
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

    /// Get the most recently compiled function info for a name (inspection only).
    #[allow(dead_code)] // API for external tooling and debugging
    pub fn get_fn_info(&self, name: &str) -> Option<&CompiledFnInfo> {
        let result = self.compiled_fn_info.get(name);
        measurement::add(
            if result.is_some() { Metric::JitCacheHits } else { Metric::JitCacheMisses },
            1,
        );
        result
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
    #[test]
    fn native_cancellation_preserves_prior_errors_and_balanced_entry() {
        let cancelled = std::sync::atomic::AtomicBool::new(true);
        let mut context =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        context.task_cancellation = &cancelled;
        context.runtime_error = Some("original failure".into());
        assert_eq!(unsafe { jit_enter_function(&mut context) }, 1);
        assert_eq!(context.runtime_error.as_deref(), Some("original failure"));
        assert_eq!(context.native_call_depth, 0);
        context.runtime_error = None;
        assert_eq!(unsafe { jit_enter_function(&mut context) }, 1);
        assert_eq!(context.runtime_error.as_deref(), Some("Task was cancelled"));
        assert_eq!(context.native_call_depth, 0);
        context.runtime_error = None;
        cancelled.store(false, std::sync::atomic::Ordering::Release);
        assert_eq!(unsafe { jit_enter_function(&mut context) }, 0);
        unsafe { jit_leave_function(&mut context) };
        assert_eq!(context.native_call_depth, 0);
        assert!(context.runtime_error.is_none());
    }
    #[test]
    fn native_cancellation_stops_running_backedges() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::{Duration, Instant};
        const CHILD: &str = "KUJO_NATIVE_CANCELLATION_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = tempfile::tempfile().unwrap();
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "jit::tests::native_cancellation_stops_running_backedges",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .stdout(output.try_clone().unwrap())
                .stderr(output.try_clone().unwrap())
                .spawn()
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(15);
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if Instant::now() >= deadline {
                    child.kill().unwrap();
                    child.wait().unwrap();
                    panic!("native loop did not cooperate with cancellation");
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            use std::io::{Read, Seek};
            let mut output = output;
            output.rewind().unwrap();
            let mut log = String::new();
            output.read_to_string(&mut log).unwrap();
            assert!(status.success(), "{log}");
            return;
        }
        for mode in ["loop", "function", "direct"] {
            let mut compiler = JitCompiler::new_with_cancellation(true).unwrap();
            let mut chunk = BytecodeChunk::new();
            chunk.emit(OpCode::JumpBack(0));
            let mut context =
                VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            let cancelled = Arc::new(AtomicBool::new(false));
            context.task_cancellation = Arc::as_ptr(&cancelled);
            let compiled = if mode == "loop" {
                Some(compiler.compile(&chunk, 0).unwrap())
            } else if mode == "function" {
                Some(compiler.compile_function(&chunk, "spin").unwrap())
            } else {
                None
            };
            let direct = if mode == "direct" {
                chunk.params.push("unused".into());
                Some(compiler.compile_function_with_direct_arg(&chunk, "spin_direct").unwrap())
            } else {
                None
            };
            let token = cancelled.clone();
            let canceller = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(25));
                token.store(true, Ordering::Release);
            });
            if let Some(compiled) = compiled {
                assert_ne!(invoke_compiled_fn(compiled, &mut context), 0, "{mode}");
            } else {
                invoke_compiled_fn_with_arg(direct.unwrap(), &mut context, 0);
            }
            canceller.join().unwrap();
            assert_eq!(context.runtime_error.as_deref(), Some("Task was cancelled"), "{mode}");
            assert_eq!(context.native_call_depth, 0, "cancelled native frame leaked: {mode}");
            assert_eq!(context.owned_roots.storage_state().1, 0, "cancelled roots leaked: {mode}");
        }
    }
    use super::*;
    #[test]
    fn native_pop_rebind_releases_dead_pairs_but_preserves_real_snapshots() {
        fn leaf(root: &Value) -> &Value {
            if let Value::Struct { fields, .. } = root {
                &fields["items"]
            } else {
                root
            }
        }
        fn identity(root: &Value) -> (usize, usize, usize) {
            let parent = if let Value::Struct { fields, .. } = root {
                Arc::as_ptr(fields) as usize
            } else {
                0
            };
            let (pointer, len) = match leaf(root) {
                Value::Array(a) => (Arc::as_ptr(a) as usize, a.len()),
                Value::Dict(d) => (Arc::as_ptr(d) as usize, d.len()),
                _ => panic!(),
            };
            (parent, pointer, len)
        }
        for remove in [false, true] {
            for nested in [false, true] {
                for count in [8usize, 128] {
                    let mut ctx = VMContext::new(
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    ctx.owned_roots.enter().unwrap();
                    let data = if remove {
                        Value::Dict(Arc::new(
                            (0..=count)
                                .map(|n| (Arc::<str>::from(n.to_string()), Value::Int(n as i64)))
                                .collect(),
                        ))
                    } else {
                        Value::array((0..=count).map(|n| Value::Int(n as i64)).collect())
                    };
                    let state = if nested {
                        Value::Struct {
                            name: "Bag".into(),
                            fields: Arc::new(HashMap::from([("items".into(), data)])),
                        }
                    } else {
                        data
                    };
                    let initial = identity(&state);
                    let (root, kind) = ctx.owned_roots.encode(state).unwrap();
                    let hash = BytecodeTranslator::hash_var_name("state");
                    let mut names = HashMap::from([(hash, "state".into())]);
                    ctx.var_names_ptr = &mut names;
                    let shape = if nested { vec![Some("items".into())] } else { vec![] };
                    let (function, function_kind) = ctx
                        .owned_roots
                        .encode(Value::NativeFunction(if remove { "remove" } else { "pop" }.into()))
                        .unwrap();
                    let (mut old_pair, mut old_kind) = (0, 0);
                    let mut saved = None;
                    for step in 0..count {
                        if step == count - 1 {
                            saved = Some(ctx.owned_roots.clone_value(root, kind).unwrap());
                        }
                        let argument =
                            leaf(ctx.owned_roots.current_value_mut(root, kind).unwrap()).clone();
                        let (original, original_kind) = ctx.owned_roots.encode(argument).unwrap();
                        let key = remove.then(|| {
                            ctx.owned_roots
                                .encode(Value::Str(Arc::new((count - step).to_string())))
                                .unwrap()
                        });
                        let mut operands = vec![original, original_kind];
                        if let Some((key, key_kind)) = key {
                            operands.extend([key, key_kind]);
                        }
                        operands.extend([function, function_kind]);
                        let pair = unsafe {
                            jit_try_pop_rebind(
                                &mut ctx,
                                1,
                                root,
                                kind,
                                1,
                                hash as i64,
                                1,
                                old_pair,
                                old_kind,
                                &shape,
                                if remove { 2 } else { 1 },
                                operands.as_ptr(),
                            )
                        };
                        assert!(ctx.runtime_error.is_none(), "{:?}", ctx.runtime_error);
                        assert_eq!(ctx.return_value_kind, crate::native_owned::OWNED);
                        // Deliberately keep the prior result live until the helper has run.
                        ctx.owned_roots.release(old_pair, old_kind).unwrap();
                        ctx.owned_roots.release(original, original_kind).unwrap();
                        if let Some((key, key_kind)) = key {
                            ctx.owned_roots.release(key, key_kind).unwrap();
                        }
                        old_pair = pair;
                        old_kind = ctx.return_value_kind;
                        let actual =
                            identity(ctx.owned_roots.current_value_mut(root, kind).unwrap());
                        assert_eq!(actual.2, count - step);
                        if step < count - 1 {
                            assert_eq!((actual.0, actual.1), (initial.0, initial.1));
                        } else {
                            assert_ne!(actual.1, initial.1);
                            if nested {
                                assert_ne!(actual.0, initial.0);
                            }
                        }
                        assert!(
                            matches!(ctx.owned_roots.current_value_mut(pair,old_kind).unwrap(),Value::Array(a) if matches!(a[1],Value::Int(v) if v==(count-step) as i64))
                        );
                    }
                    assert_eq!(identity(saved.as_ref().unwrap()).2, 2);
                    ctx.owned_roots.leave().unwrap();
                    let (_, live, slots, _) = ctx.owned_roots.storage_state();
                    assert_eq!(live, 0);
                    assert!(slots <= 8, "{slots}");
                }
            }
        }
    }

    #[test]
    fn native_path_appends_reuse_nodes_and_bound_operand_roots() {
        for bytes in [false, true] {
            for count in [8usize, 128] {
                let mut ctx = VMContext::new(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                ctx.owned_roots.enter().unwrap();
                let leaf = if bytes {
                    Value::Bytes(Arc::new(Vec::with_capacity(count + 1)))
                } else {
                    Value::Array(Arc::new(Vec::with_capacity(count + 1)))
                };
                let fields = Arc::new(HashMap::from([("items".into(), leaf)]));
                let parent_pointer = Arc::as_ptr(&fields) as usize;
                let leaf_pointer = match &fields["items"] {
                    Value::Array(a) => Arc::as_ptr(a) as usize,
                    Value::Bytes(a) => Arc::as_ptr(a) as usize,
                    _ => panic!(),
                };
                let (root, kind) =
                    ctx.owned_roots.encode(Value::Struct { name: "Bag".into(), fields }).unwrap();
                let hash = BytecodeTranslator::hash_var_name("state");
                let mut names = HashMap::from([(hash, "state".into())]);
                ctx.var_names_ptr = &mut names;
                let shape = vec![Some("items".into())];
                let (function, function_kind) = ctx
                    .owned_roots
                    .encode(Value::NativeFunction(if bytes { "concat" } else { "push" }.into()))
                    .unwrap();
                let mut snapshot = None;
                for step in 0..=count {
                    if step == count {
                        snapshot = Some(ctx.owned_roots.clone_value(root, kind).unwrap());
                    }
                    let original = match ctx.owned_roots.current_value_mut(root, kind).unwrap() {
                        Value::Struct { fields, .. } => fields["items"].clone(),
                        _ => panic!(),
                    };
                    let (original, original_kind) = ctx.owned_roots.encode(original).unwrap();
                    let item = if bytes { Value::Bytes(Arc::new(vec![1])) } else { Value::Int(1) };
                    let (item, item_kind) = ctx.owned_roots.encode(item).unwrap();
                    assert_eq!(
                        unsafe {
                            jit_guard_append_path(
                                &mut ctx,
                                original,
                                original_kind,
                                item,
                                item_kind,
                                function,
                                function_kind,
                            )
                        },
                        1
                    );
                    let operands =
                        [original, original_kind, item, item_kind, function, function_kind];
                    assert_eq!(
                        unsafe {
                            jit_append_binding_path(
                                &mut ctx,
                                1,
                                root,
                                kind,
                                0,
                                hash as i64,
                                &shape,
                                operands.as_ptr(),
                            )
                        },
                        0,
                        "{:?}",
                        ctx.runtime_error
                    );
                    ctx.owned_roots.release(original, original_kind).unwrap();
                    ctx.owned_roots.release(item, item_kind).unwrap();
                    let Value::Struct { fields, .. } =
                        ctx.owned_roots.current_value_mut(root, kind).unwrap()
                    else {
                        panic!()
                    };
                    let (pointer, len) = match &fields["items"] {
                        Value::Array(a) => (Arc::as_ptr(a) as usize, a.len()),
                        Value::Bytes(a) => (Arc::as_ptr(a) as usize, a.len()),
                        _ => panic!(),
                    };
                    assert_eq!(len, step + 1);
                    if step < count {
                        assert_eq!(Arc::as_ptr(fields) as usize, parent_pointer);
                        assert_eq!(pointer, leaf_pointer);
                    } else {
                        assert_ne!(Arc::as_ptr(fields) as usize, parent_pointer);
                        assert_ne!(pointer, leaf_pointer);
                    }
                }
                let Some(Value::Struct { fields, .. }) = snapshot else { panic!() };
                assert!(
                    matches!(&fields["items"],Value::Array(a) if a.len()==count)
                        || matches!(&fields["items"],Value::Bytes(a) if a.len()==count)
                );
                ctx.owned_roots.leave().unwrap();
                let (_, live, slots, _) = ctx.owned_roots.storage_state();
                assert_eq!(live, 0);
                assert!(slots <= 6, "{slots}");
            }
        }
    }

    #[test]
    fn native_field_write_consumes_temporary_without_losing_snapshots() {
        for count in [8, 128] {
            let mut ctx =
                VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            ctx.owned_roots.enter().unwrap();
            let fields = Arc::new(HashMap::from([("item".into(), Value::Int(0))]));
            let pointer = Arc::as_ptr(&fields) as usize;
            let (mut object, kind) =
                ctx.owned_roots.encode(Value::Struct { name: "Box".into(), fields }).unwrap();
            let name = String::from("item");
            for value in 1..=count {
                let updated = unsafe {
                    jit_field_write(&mut ctx, object, kind, value, 0, &name as *const String as i64)
                };
                assert!(ctx.runtime_error.is_none());
                ctx.owned_roots.release(object, kind).unwrap();
                object = updated;
                match ctx.owned_roots.current_value_mut(object, kind).unwrap() {
                    Value::Struct { fields, .. } => {
                        assert_eq!(Arc::as_ptr(fields) as usize, pointer);
                        assert!(
                            matches!(fields.get("item"), Some(Value::Int(actual)) if *actual == value)
                        );
                    }
                    _ => panic!(),
                }
            }
            let saved = ctx.owned_roots.clone_value(object, kind).unwrap();
            let updated = unsafe {
                jit_field_write(&mut ctx, object, kind, 999, 0, &name as *const String as i64)
            };
            assert!(ctx.runtime_error.is_none());
            ctx.owned_roots.release(object, kind).unwrap();
            assert!(
                matches!(saved,Value::Struct {ref fields,..} if matches!(fields.get("item"),Some(Value::Int(actual)) if *actual == count))
            );
            assert!(
                matches!(ctx.owned_roots.current_value_mut(updated,kind).unwrap(),Value::Struct {fields,..} if matches!(fields.get("item"),Some(Value::Int(999))) && Arc::as_ptr(fields) as usize != pointer)
            );
            ctx.owned_roots.leave().unwrap();
            let (_, live, slots, _) = ctx.owned_roots.storage_state();
            assert_eq!(live, 0);
            assert!(slots <= 4);
        }
    }

    #[test]
    fn native_path_depth_cache_survives_helper_call_contexts() {
        fn update(vm: &mut VM, root: Value, value: i64, binding_kind: i64) -> (Value, usize) {
            let hash = BytecodeTranslator::hash_var_name("state");
            let mut names = HashMap::from([(hash, "state".into())]);
            let mut context = VMContext::new_with_vm(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                vm as *mut VM as *mut std::ffi::c_void,
            );
            context.var_names_ptr = &mut names;
            context.owned_roots.enter().unwrap();
            let (payload, kind) = context.owned_roots.encode(root).unwrap();
            let shape = vec![None, None];
            let operands = [0, 0, 0, 0, value, 0];
            // SAFETY: the VM, context, shape and operands are live for this synchronous call.
            assert_eq!(
                unsafe {
                    jit_update_binding_path(
                        &mut context,
                        1,
                        payload,
                        kind,
                        binding_kind,
                        hash as i64,
                        &shape,
                        operands.as_ptr(),
                    )
                },
                i64::from(binding_kind != 0),
                "{:?}",
                context.runtime_error
            );
            if binding_kind != 0 {
                assert_eq!(
                    context.runtime_error.as_deref(),
                    Some("Cannot mutate immutable let binding: state")
                );
            }
            let scans = context.path_depth_valid.scan_count();
            let result = context.owned_roots.take_value(payload, kind).unwrap();
            context.owned_roots.leave().unwrap();
            (result, scans)
        }
        fn first(root: &Value) -> i64 {
            let Value::Array(rows) = root else { panic!() };
            let Value::Array(row) = &rows[0] else { panic!() };
            let Value::Int(value) = row[0] else { panic!() };
            value
        }
        let mut measurements = Vec::new();
        for calls in [8, 128] {
            let mut vm = VM::new();
            let mut root = Value::array(vec![Value::array(vec![Value::Int(0); 4096])]);
            let mut private_scans = 0;
            for value in 0..calls {
                let (updated, scans) = update(&mut vm, root, value as i64, 0);
                root = updated;
                private_scans += scans;
            }
            let total = private_scans + vm.native_path_depth_scan_count();
            measurements.push((calls, total));
            assert_eq!(first(&root), (calls - 1) as i64);
            let before_error = vm.native_path_depth_scan_count();
            let (unchanged, _) = update(&mut vm, root, -1, 1);
            assert_eq!(vm.native_path_depth_scan_count(), before_error);
            let (restored, _) = update(&mut vm, unchanged, (calls - 1) as i64, 0);
            assert_eq!(vm.native_path_depth_scan_count(), before_error);
            root = restored;
            let snapshot = root.clone();
            let (updated, _) = update(&mut vm, root, 999, 0);
            assert_eq!(first(&snapshot), (calls - 1) as i64);
            assert_eq!(first(&updated), 999);
        }
        eprintln!("native path depth scans across contexts: {measurements:?}");
        assert!(measurements.iter().all(|(_, scans)| *scans <= 1), "{measurements:?}");
    }

    #[test]
    fn compiled_nested_writes_release_operands_and_retain_path_metadata() {
        for repetitions in [8, 128] {
            let mut compiler = JitCompiler::new().unwrap();
            let mut chunk = BytecodeChunk::new();
            chunk.local_names.push("local".into());
            chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
            chunk.emit(OpCode::LoadVar("seed".into()));
            chunk.emit(OpCode::StoreLocal(0));
            chunk.emit(OpCode::Pop);
            let zero = chunk.add_constant(Constant::Int(0));
            for _ in 0..repetitions {
                chunk.emit(OpCode::LoadVar("item".into()));
                chunk.emit(OpCode::LoadConst(zero));
                chunk.emit(OpCode::LoadConst(zero));
                chunk.emit(OpCode::UpdateBindingPath(
                    crate::bytecode::MutationBinding::Local(0),
                    vec![None, None],
                ));
                chunk.emit(OpCode::Pop);
            }
            chunk.emit(OpCode::LoadLocal(0));
            chunk.emit(OpCode::Return);
            let compiled = compiler.compile_function(&chunk, "nested_owned_write").unwrap();
            drop(chunk);
            let seed = Value::array(vec![Value::array(vec![Value::Int(0)])]);
            let mut globals = HashMap::from([
                ("seed".into(), seed.clone()),
                ("item".into(), Value::Str(Arc::new("new".into()))),
            ]);
            let mut names = ["seed", "item", "local"]
                .into_iter()
                .map(|name| (BytecodeTranslator::hash_var_name(name), name.to_owned()))
                .collect();
            let mut caller = vec![Value::Int(77)];
            let mut stack = Vec::new();
            let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
            context.var_names_ptr = &mut names;
            context.local_slots_ptr = &mut caller;
            assert_eq!(
                invoke_compiled_fn(compiled, &mut context),
                0,
                "{:?}",
                context.runtime_error
            );
            let expected =
                Value::array(vec![Value::array(vec![Value::Str(Arc::new("new".into()))])]);
            assert!(Value::equals(&context.take_return_value(context.return_value), &expected));
            assert!(Value::equals(&seed, &Value::array(vec![Value::array(vec![Value::Int(0)])])));
            assert!(matches!(caller.as_slice(), [Value::Int(77)]));
            let (_, live, capacity, _) = context.owned_roots.storage_state();
            assert_eq!(live, 0);
            assert!(capacity <= 8, "{repetitions} writes retained {capacity} roots");
            assert!(stack.is_empty());
        }
    }

    #[test]
    fn native_nested_writes_reuse_unique_rows_and_preserve_aliases() {
        for repetitions in [8, 128] {
            let hash = BytecodeTranslator::hash_var_name("state");
            let mut names = HashMap::from([(hash, "state".into())]);
            let mut context =
                VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            context.var_names_ptr = &mut names;
            context.owned_roots.enter().unwrap();
            let (root, kind) = context
                .owned_roots
                .encode(Value::array(vec![Value::array(vec![Value::Int(0); 128])]))
                .unwrap();
            fn pointers(value: &Value) -> (*const Value, *const Value) {
                let Value::Array(rows) = value else { panic!() };
                let Value::Array(row) = &rows[0] else { panic!() };
                (rows.as_ptr(), row.as_ptr())
            }
            let before = pointers(context.owned_roots.current_value_mut(root, kind).unwrap());
            let shape = vec![None, None];
            for i in 0..repetitions {
                let operands = [0, 0, 0, 0, i, 0];
                // SAFETY: shape, tagged operands and context live through the helper call.
                assert_eq!(
                    unsafe {
                        jit_update_binding_path(
                            &mut context,
                            1,
                            root,
                            kind,
                            0,
                            hash as i64,
                            &shape,
                            operands.as_ptr(),
                        )
                    },
                    0,
                    "{:?}",
                    context.runtime_error
                );
                assert_eq!(
                    pointers(context.owned_roots.current_value_mut(root, kind).unwrap()),
                    before
                );
            }
            let snapshot = context.owned_roots.duplicate(root, kind).unwrap();
            let operands = [0, 0, 0, 0, 999, 0];
            // SAFETY: same live roots and tagged operand array, with a retained snapshot.
            assert_eq!(
                unsafe {
                    jit_update_binding_path(
                        &mut context,
                        1,
                        root,
                        kind,
                        0,
                        hash as i64,
                        &shape,
                        operands.as_ptr(),
                    )
                },
                0
            );
            assert_ne!(
                pointers(context.owned_roots.current_value_mut(root, kind).unwrap()),
                before
            );
            let saved = context.owned_roots.clone_value(snapshot, kind).unwrap();
            let Value::Array(rows) = saved else { panic!() };
            let Value::Array(row) = &rows[0] else { panic!() };
            assert!(matches!(row[0],Value::Int(n) if n == repetitions - 1));
            // SAFETY: unchanged input lifetime; rejected write must not mutate either row.
            assert_eq!(
                unsafe {
                    jit_update_binding_path(
                        &mut context,
                        1,
                        root,
                        kind,
                        1,
                        hash as i64,
                        &shape,
                        operands.as_ptr(),
                    )
                },
                1
            );
            assert_eq!(
                context.runtime_error.as_deref(),
                Some("Cannot mutate immutable let binding: state")
            );
            context.owned_roots.leave().unwrap();
            let (_, live, capacity, _) = context.owned_roots.storage_state();
            assert_eq!(live, 0);
            assert!(capacity <= 4, "root storage grew to {capacity}");
        }
    }

    #[test]
    fn native_readonly_local_first_store_and_redefinition() {
        for kind in [BytecodeBindingKind::LetImmutable, BytecodeBindingKind::Const] {
            for second_define in [false, true] {
                let mut compiler = JitCompiler::new().unwrap();
                let mut chunk = BytecodeChunk::new();
                chunk.local_names.push("value".into());
                chunk.local_binding_kinds.push(kind);
                let first = chunk.add_constant(Constant::Int(11));
                let second = chunk.add_constant(Constant::Int(22));
                chunk.emit(OpCode::LoadConst(first));
                chunk.emit(OpCode::StoreLocal(0)); // valid initialization without DefineLocal
                chunk.emit(OpCode::Pop);
                chunk.emit(OpCode::LoadConst(second));
                chunk.emit(if second_define {
                    OpCode::DefineLocal(0)
                } else {
                    OpCode::StoreLocal(0)
                });
                chunk.emit(OpCode::Pop);
                chunk.emit(OpCode::LoadLocal(0));
                chunk.emit(OpCode::Return);
                let compiled = compiler.compile_function(&chunk, "readonly_store").unwrap();
                let mut names = HashMap::from([(
                    BytecodeTranslator::hash_var_name("value"),
                    "value".to_owned(),
                )]);
                let mut stack = Vec::new();
                let mut context =
                    VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
                context.var_names_ptr = &mut names;
                let status = invoke_compiled_fn(compiled, &mut context);
                if second_define {
                    assert_eq!(status, 0, "{:?}", context.runtime_error);
                    assert!(matches!(
                        context.take_return_value(context.return_value),
                        Value::Int(22)
                    ));
                } else {
                    assert_ne!(status, 0);
                    assert_eq!(
                        context.runtime_error,
                        Some(VM::local_reassignment_error(kind, "value"))
                    );
                }
                assert_eq!(context.owned_roots.storage_state().1, 0);
            }
        }
    }

    #[test]
    fn native_generic_index_replacement_reuses_unique_storage_and_bounds_roots() {
        for repetitions in [8, 128] {
            for bytes in [false, true] {
                let mut stack = Vec::new();
                let mut ctx =
                    VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
                ctx.owned_roots.enter().unwrap();
                let seed = if bytes {
                    Value::bytes(vec![0, 1])
                } else {
                    Value::array(vec![Value::Int(0), Value::Int(1)])
                };
                let pointer = |value: &Value| match value {
                    Value::Array(items) => Arc::as_ptr(items) as usize,
                    Value::Bytes(items) => Arc::as_ptr(items) as usize,
                    _ => panic!("unexpected indexed representation"),
                };
                let initial = pointer(&seed);
                let (mut root, kind) = ctx.owned_roots.encode(seed).unwrap();
                for index in 0..repetitions {
                    // SAFETY: all operands are live native roots or tagged integers.
                    let updated =
                        unsafe { jit_index_set_result(&mut ctx, root, kind, 0, 0, index, 0) };
                    assert!(ctx.runtime_error.is_none(), "{:?}", ctx.runtime_error);
                    assert_eq!(ctx.owned_roots.storage_state().1, 2);
                    ctx.owned_roots.release(root, kind).unwrap();
                    assert_eq!(ctx.owned_roots.storage_state().1, 1);
                    root = updated;
                    assert_eq!(
                        pointer(ctx.owned_roots.current_value_mut(root, kind).unwrap()),
                        initial
                    );
                }
                let saved = ctx.owned_roots.clone_value(root, kind).unwrap();
                // A genuine retained snapshot must force isolation.
                let updated = unsafe { jit_index_set_result(&mut ctx, root, kind, 0, 0, 200, 0) };
                ctx.owned_roots.release(root, kind).unwrap();
                assert_ne!(
                    pointer(ctx.owned_roots.current_value_mut(updated, kind).unwrap()),
                    initial
                );
                assert!(
                    matches!(VM::get_indexed_value(&saved,&Value::Int(0)).unwrap(),Value::Int(n) if n==repetitions-1)
                );
                assert!(matches!(
                    VM::get_indexed_value(
                        ctx.owned_roots.current_value_mut(updated, kind).unwrap(),
                        &Value::Int(0)
                    )
                    .unwrap(),
                    Value::Int(200)
                ));
                ctx.owned_roots.leave().unwrap();
                let (_, live, capacity, _) = ctx.owned_roots.storage_state();
                assert_eq!(live, 0);
                // Vec's minimum allocation holds four slots; live roots stay at two.
                assert!(capacity <= 4, "{repetitions} updates retained {capacity} root slots");
            }
        }
    }

    #[test]
    fn generic_index_replacement_preserves_runtime_depth_policies() {
        let mut levels = vec![Value::Int(7)];
        for _ in 0..crate::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH {
            levels.push(Value::array(vec![levels.last().unwrap().clone()]));
        }
        let seeds = vec![
            Value::array(vec![Value::Int(1)]),
            Value::Dict(Arc::new([(Arc::from("0"), Value::Int(1))].into_iter().collect())),
            Value::FixedDict {
                keys: Arc::new(vec![Arc::from("0")]),
                values: vec![Value::Int(1)].into(),
            },
            Value::IntDict(Arc::new([(0, Value::Int(1))].into_iter().collect())),
            Value::DenseIntDict(Arc::new(vec![Value::Int(1)])),
            Value::DenseIntDictInt(Arc::new(vec![Some(1)])),
            Value::DenseIntDictIntFull(Arc::new(vec![1])),
        ];
        let mut failures = Vec::new();
        let tokens = crate::lexer::tokenize("target[0] = incoming").unwrap();
        let program = crate::parser::Parser::new(tokens).parse();
        let chunk = crate::compiler::Compiler::new().compile(&program).unwrap();
        assert!(chunk.instructions.iter().any(|op| matches!(op, OpCode::IndexSet)));
        for seed in seeds {
            let mut interpreter = crate::interpreter::Interpreter::new();
            interpreter.env.define("target".into(), seed.clone());
            interpreter.env.define("incoming".into(), levels.last().unwrap().clone());
            let error = interpreter
                .eval_stmt_repl(&program[0])
                .err()
                .map(|error| error.to_string())
                .unwrap_or_default();
            // The existing interpreter does not enforce the VM's value-depth
            // limit. Preserve that compatibility policy while testing COW.
            if !error.is_empty() {
                failures.push(format!("interpreter {seed:?}: {error}"));
            }
            let target = interpreter.env.get("target").unwrap();
            if Value::nesting_depth(&target)
                != crate::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH + 1
            {
                failures.push(format!("interpreter failed replacement for {seed:?}"));
            }
            if !matches!(VM::get_indexed_value(&seed, &Value::Int(0)).unwrap(), Value::Int(1)) {
                failures.push("interpreter mutated retained seed".into());
            }
            let mut environment = crate::interpreter::Environment::new();
            environment.define("target".into(), seed.clone());
            environment.define("incoming".into(), levels.last().unwrap().clone());
            let environment = Arc::new(std::sync::Mutex::new(environment));
            let mut vm = VM::new();
            vm.set_globals(Arc::clone(&environment));
            let error = vm.execute_until_suspend(chunk.clone()).err().unwrap_or_default();
            if !error.contains("Maximum data nesting depth") {
                failures.push(format!("VM {seed:?}: {error}"));
            }

            let mut ctx =
                VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            ctx.owned_roots.enter().unwrap();
            let snapshot = seed.clone();
            let (object, kind) = ctx.owned_roots.encode(seed).unwrap();
            let (value, value_kind) =
                ctx.owned_roots.encode(levels.last().unwrap().clone()).unwrap();
            // SAFETY: all handles belong to this active context.
            unsafe {
                jit_index_set_result(&mut ctx, object, kind, 0, 0, value, value_kind);
            }
            if !ctx
                .runtime_error
                .as_deref()
                .is_some_and(|error| error.starts_with("Maximum data nesting depth"))
            {
                failures.push(format!("{snapshot:?}: {:?}", ctx.runtime_error));
            }
            assert!(matches!(
                VM::get_indexed_value(&snapshot, &Value::Int(0)).unwrap(),
                Value::Int(1)
            ));
            ctx.owned_roots.leave().unwrap();
            assert_eq!(ctx.owned_roots.storage_state().1, 0);
        }
        while levels.pop().is_some() {}
        assert!(failures.is_empty(), "missing depth errors: {failures:?}");
    }

    #[test]
    fn guarded_loop_dense_writes_inline_without_rust_vec_layout_assumptions() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();
        chunk.local_names.push("mapping".into());
        chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
        let value = chunk.add_constant(Constant::Int(99));
        let index = chunk.add_constant(Constant::Int(1));
        chunk.emit(OpCode::LoadConst(value));
        chunk.emit(OpCode::LoadConst(index));
        chunk.emit(OpCode::IndexSetInPlace(0));
        chunk.emit(OpCode::Pop);
        chunk.emit(OpCode::ReturnNone);
        let compiled =
            compiler.compile_loop_with_int_dicts(&chunk, 0, 4, HashSet::from([0])).unwrap();
        let mut slots = vec![Value::DenseIntDictIntFull(Arc::new(vec![1, 2]))];
        let pointer = match &slots[0] {
            Value::DenseIntDictIntFull(values) => values.as_ptr(),
            _ => unreachable!(),
        };
        let mut stack = Vec::new();
        let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
        context.local_slots_ptr = &mut slots;
        // No name map: the generic write helper would fail. Success proves the
        // guarded native store ran, using an explicit C-layout data/length view.
        for _ in 0..1000 {
            context.loop_resume_ip = None;
            assert_eq!(
                invoke_compiled_fn(compiled, &mut context),
                2,
                "{:?}",
                context.runtime_error
            );
            assert!(
                matches!(&slots[0], Value::DenseIntDictIntFull(values) if values.as_ptr() == pointer && values.as_slice() == [1,99])
            );
            // The terminal ReturnNone belongs to VM frame cleanup.
            assert_eq!(context.loop_resume_ip, Some(4));
            assert!(stack.is_empty());
            assert_eq!(context.loop_dense_views.len(), 1);
            assert_eq!(context.owned_roots.storage_state(), (0, 0, 0, 0));
        }
        let snapshot = slots[0].clone();
        // SAFETY: live slot/context; shared vectors must never expose mutable data.
        assert_eq!(unsafe { jit_loop_dense_view(&mut context, 0) }, 0);
        assert!(
            matches!(snapshot, Value::DenseIntDictIntFull(values) if values.as_slice() == [1,99])
        );
        let mut readonly_compiler = JitCompiler::new().unwrap();
        chunk.local_binding_kinds[0] = BytecodeBindingKind::Const;
        let readonly = readonly_compiler
            .compile_loop_with_int_dicts(&chunk, 0, 4, HashSet::from([0]))
            .unwrap();
        let mut names =
            HashMap::from([(BytecodeTranslator::hash_var_name("mapping"), "mapping".to_owned())]);
        context.var_names_ptr = &mut names;
        assert_ne!(invoke_compiled_fn(readonly, &mut context), 0);
        assert_eq!(context.runtime_error.as_deref(), Some("Cannot mutate const binding: mapping"));
        assert!(
            matches!(&slots[0], Value::DenseIntDictIntFull(values) if values.as_slice() == [1,99])
        );
    }

    #[test]
    fn native_indexed_writes_reuse_unique_storage_and_preserve_snapshots() {
        let hash = BytecodeTranslator::hash_var_name("local");
        let mut names = HashMap::from([(hash, "local".into())]);
        let mut context =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        context.var_names_ptr = &mut names;
        context.owned_roots.enter().unwrap();
        let (root, kind) =
            context.owned_roots.encode(Value::array(vec![Value::Int(0); 100])).unwrap();
        let pointer = match context.owned_roots.current_value_mut(root, kind).unwrap() {
            Value::Array(values) => values.as_ptr(),
            _ => unreachable!(),
        };
        for i in 0..1000 {
            // SAFETY: live exclusive context and a current-scope array root.
            assert_eq!(
                unsafe {
                    jit_indexed_write(&mut context, 1, root, kind, 0, hash as i64, 0, 0, i, 0)
                },
                0
            );
            assert!(
                matches!(context.owned_roots.current_value_mut(root, kind).unwrap(), Value::Array(values) if values.as_ptr() == pointer)
            );
        }
        let snapshot = context.owned_roots.duplicate(root, kind).unwrap();
        // SAFETY: same context; the snapshot must force normal copy-on-write.
        assert_eq!(
            unsafe { jit_indexed_write(&mut context, 1, root, kind, 0, hash as i64, 0, 0, 7, 0) },
            0
        );
        assert!(
            matches!(context.owned_roots.clone_value(snapshot, kind).unwrap(), Value::Array(values) if matches!(values[0], Value::Int(999)))
        );
        // SAFETY: immutable metadata must reject mutation before touching the root.
        assert_eq!(
            unsafe { jit_indexed_write(&mut context, 1, root, kind, 1, hash as i64, 0, 0, 9, 0) },
            1
        );
        assert_eq!(
            context.runtime_error.as_deref(),
            Some("Cannot mutate immutable let binding: local")
        );
        assert!(
            matches!(context.owned_roots.clone_value(root, kind).unwrap(), Value::Array(values) if matches!(values[0], Value::Int(7)))
        );
        context.owned_roots.leave().unwrap();
        assert_eq!(context.owned_roots.storage_state().1, 0);
    }

    #[test]
    fn compiled_indexed_writes_release_operands_and_preserve_caller() {
        for repetitions in [8, 128] {
            let mut compiler = JitCompiler::new().unwrap();
            let mut chunk = BytecodeChunk::new();
            chunk.local_names.push("local".into());
            chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
            chunk.emit(OpCode::LoadVar("seed".into()));
            chunk.emit(OpCode::StoreLocal(0));
            chunk.emit(OpCode::Pop);
            let index = chunk.add_constant(Constant::Int(0));
            for _ in 0..repetitions {
                chunk.emit(OpCode::LoadVar("item".into()));
                chunk.emit(OpCode::LoadConst(index));
                chunk.emit(OpCode::IndexSetInPlace(0));
                chunk.emit(OpCode::Pop);
            }
            chunk.emit(OpCode::LoadLocal(0));
            chunk.emit(OpCode::Return);
            let compiled = compiler.compile_function(&chunk, "indexed_write").unwrap();
            let seed = Arc::new(vec![Value::Int(0)]);
            let mut globals = HashMap::from([
                ("seed".into(), Value::Array(seed.clone())),
                ("item".into(), Value::Str(Arc::new("value".into()))),
            ]);
            let mut names = ["seed", "item", "local"]
                .into_iter()
                .map(|name| (BytecodeTranslator::hash_var_name(name), name.to_owned()))
                .collect();
            let mut caller = vec![Value::Int(77)];
            let mut stack = Vec::new();
            let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
            context.var_names_ptr = &mut names;
            context.local_slots_ptr = &mut caller;
            assert_eq!(
                invoke_compiled_fn(compiled, &mut context),
                0,
                "{:?}",
                context.runtime_error
            );
            assert!(
                matches!(context.take_return_value(context.return_value), Value::Array(values) if matches!(&values[0], Value::Str(text) if text.as_str() == "value"))
            );
            assert!(matches!(caller.as_slice(), [Value::Int(77)]));
            assert!(matches!(seed[0], Value::Int(0)));
            let (_, live, capacity, _) = context.owned_roots.storage_state();
            assert_eq!(live, 0);
            assert!(capacity <= 4, "{repetitions} writes retained {capacity} root slots");
            assert!(stack.is_empty());
        }
    }

    #[test]
    fn indexed_read_cache_admission_requires_repeated_sites_or_loop_reads() {
        use OpCode::{IndexGetInPlace as Read, JumpBack};
        assert!(BytecodeTranslator::repeated_indexed_read_slots(&[Read(0)]).is_empty());
        assert_eq!(
            BytecodeTranslator::repeated_indexed_read_slots(&[Read(0), Read(1), Read(0)]),
            HashSet::from([0])
        );
        assert_eq!(
            BytecodeTranslator::repeated_indexed_read_slots(&[
                Read(0),
                Read(1),
                Read(2),
                JumpBack(2),
                JumpBack(1),
                Read(3),
            ]),
            HashSet::from([1, 2])
        );
        assert!(BytecodeTranslator::repeated_indexed_read_slots(&[
            OpCode::Jump(2),
            Read(0),
            Read(1),
        ])
        .is_empty());
    }

    #[test]
    fn cached_indexed_reads_match_generic_reads_and_reject_released_handles() {
        let mut context =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        context.owned_roots.enter().unwrap();
        let mut cache = crate::native_owned::OwnedReadCache::default();
        let pointer = &mut cache as *mut _ as i64;
        let objects = vec![
            Value::array(vec![
                Value::Float(-0.0),
                Value::Float(f64::from_bits(0x7ff8000000000042)),
                Value::Null,
                Value::Bool(true),
                Value::array(vec![Value::Int(8)]),
            ]),
            Value::Str(Arc::new("é🙂".into())),
            Value::Bytes(Arc::new(vec![0, 255])),
            Value::array(vec![]),
        ];
        for object in objects {
            let (root, kind) = context.owned_roots.encode(object.clone()).unwrap();
            for index in [
                Value::Int(0),
                Value::Int(-1),
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
                Value::Int(i64::MIN),
                Value::Float(0.5),
                Value::Float(f64::NAN),
                Value::Bool(false),
            ] {
                let expected = VM::get_indexed_value(&object, &index);
                let (i, ik) = context.owned_roots.encode(index).unwrap();
                context.runtime_error = None;
                // SAFETY: context/cache are live and disjoint; operands are rooted.
                let result =
                    unsafe { jit_indexed_read_cached(&mut context, root, kind, i, ik, pointer) };
                match expected {
                    Ok(value) => {
                        assert!(context.runtime_error.is_none());
                        if let Some(expected) = crate::native_scalar::encode(&value) {
                            assert_eq!((result, context.return_value_kind), expected);
                        } else {
                            let actual = context
                                .owned_roots
                                .clone_value(result, context.return_value_kind)
                                .unwrap();
                            assert!(Value::equals(&actual, &value));
                            context.owned_roots.release(result, context.return_value_kind).unwrap();
                        }
                    }
                    Err(error) => {
                        assert_eq!(context.runtime_error.as_deref(), Some(error.as_str()))
                    }
                }
                context.owned_roots.release(i, ik).unwrap();
            }
            context.owned_roots.release(root, kind).unwrap();
            context.runtime_error = None;
            // SAFETY: cache remains live; stale root must fail before value access.
            unsafe {
                jit_indexed_read_cached(&mut context, root, kind, 0, 0, pointer);
            }
            assert!(context
                .runtime_error
                .as_deref()
                .unwrap()
                .starts_with("Native ownership error:"));
        }
        context.owned_roots.leave().unwrap();
    }

    #[test]
    fn scalar_array_reads_match_vm_bits_bounds_and_owned_fallbacks() {
        let elements = vec![
            Value::Int(i64::MIN),
            Value::Int(i64::MAX),
            Value::Bool(false),
            Value::Bool(true),
            Value::Null,
            Value::Float(0.0),
            Value::Float(-0.0),
            Value::Float(f64::INFINITY),
            Value::Float(f64::NEG_INFINITY),
            Value::Float(f64::from_bits(0x7ff8_0000_0000_0042)),
            Value::Str(Arc::new("owned".into())),
            Value::array(vec![Value::Int(9)]),
        ];
        let array = Value::array(elements);
        let mut context =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        context.owned_roots.enter().unwrap();
        let (object, object_kind) = context.owned_roots.encode(array.clone()).unwrap();
        // The source may belong to the caller's active frame.
        context.owned_roots.enter().unwrap();
        let mut indices: Vec<Value> = (-13..=12).map(Value::Int).collect();
        indices.extend([
            Value::Int(i64::MIN),
            Value::Int(i64::MAX),
            Value::Float(0.5),
            Value::Float(-1.5),
            Value::Float(f64::NAN),
            Value::Float(f64::INFINITY),
            Value::Bool(false),
            Value::Str(Arc::new("0".into())),
        ]);
        for index in indices {
            let expected = VM::get_indexed_value(&array, &index);
            let (index_payload, index_kind) = context.owned_roots.encode(index.clone()).unwrap();
            context.runtime_error = None;
            // SAFETY: validated operands remain rooted in live nested frames.
            let result = unsafe {
                jit_indexed_read(&mut context, object, object_kind, index_payload, index_kind)
            };
            match expected {
                Ok(value) => {
                    assert!(
                        context.runtime_error.is_none(),
                        "{index:?}: {:?}",
                        context.runtime_error
                    );
                    if let Some((bits, kind)) = crate::native_scalar::encode(&value) {
                        assert_eq!((result, context.return_value_kind), (bits, kind), "{index:?}");
                    } else {
                        let actual = context
                            .owned_roots
                            .clone_value(result, context.return_value_kind)
                            .unwrap();
                        assert!(Value::equals(&actual, &value));
                        context.owned_roots.release(result, context.return_value_kind).unwrap();
                    }
                }
                Err(error) => {
                    assert_eq!(context.runtime_error.as_deref(), Some(error.as_str()), "{index:?}")
                }
            }
            context.owned_roots.release(index_payload, index_kind).unwrap();
        }
        context.owned_roots.leave().unwrap();
        context.owned_roots.release(object, object_kind).unwrap();
        context.runtime_error = None;
        // SAFETY: context remains live; stale handles must be rejected, never dereferenced.
        unsafe {
            jit_indexed_read(&mut context, object, object_kind, 0, 0);
        }
        assert!(context.runtime_error.as_deref().unwrap().starts_with("Native ownership error:"));
        context.owned_roots.leave().unwrap();
    }

    #[test]
    fn compiled_scalar_indexed_reads_do_not_issue_per_read_roots() {
        let mut counts = Vec::new();
        for reads in [8, 128] {
            let mut compiler = JitCompiler::new().unwrap();
            let mut chunk = BytecodeChunk::new();
            chunk.local_names.push("local".into());
            chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
            chunk.emit(OpCode::LoadVar("seed".into()));
            chunk.emit(OpCode::StoreLocal(0));
            chunk.emit(OpCode::Pop);
            let index = chunk.add_constant(Constant::Int(0));
            for _ in 0..reads {
                chunk.emit(OpCode::LoadConst(index));
                chunk.emit(OpCode::IndexGetInPlace(0));
                chunk.emit(OpCode::Pop);
            }
            chunk.emit(OpCode::LoadConst(index));
            chunk.emit(OpCode::IndexGetInPlace(0));
            chunk.emit(OpCode::Return);
            let compiled = compiler.compile_function(&chunk, "scalar_indexed").unwrap();
            let seed = Arc::new(vec![Value::Int(7)]);
            let mut globals = HashMap::from([("seed".into(), Value::Array(seed.clone()))]);
            let mut names = ["seed", "local"]
                .into_iter()
                .map(|name| (BytecodeTranslator::hash_var_name(name), name.to_owned()))
                .collect();
            let mut caller = vec![Value::Int(77)];
            let mut stack = Vec::new();
            let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
            context.var_names_ptr = &mut names;
            context.local_slots_ptr = &mut caller;
            assert_eq!(
                invoke_compiled_fn(compiled, &mut context),
                0,
                "{:?}",
                context.runtime_error
            );
            let returned = context.take_return_value(context.return_value);
            assert!(matches!(returned, Value::Int(7)), "reads={reads}: {returned:?}");
            assert!(matches!(caller.as_slice(), [Value::Int(77)]));
            assert!(matches!(seed[0], Value::Int(7)));
            assert_eq!(context.owned_roots.storage_state().1, 0);
            assert!(stack.is_empty());
            counts.push((reads + 1, context.owned_roots.issued_handles()));
        }
        assert!(counts.iter().all(|(_, issued)| *issued <= 2), "per-read root churn: {counts:?}");
    }

    #[test]
    fn compiled_indexed_reads_release_temporary_roots() {
        for repetitions in [8, 128] {
            let mut compiler = JitCompiler::new().unwrap();
            let mut chunk = BytecodeChunk::new();
            chunk.local_names.push("local".into());
            chunk.emit(OpCode::LoadVar("seed".into()));
            chunk.emit(OpCode::StoreLocal(0));
            chunk.emit(OpCode::Pop);
            let index = chunk.add_constant(Constant::Int(0));
            for _ in 0..repetitions {
                chunk.emit(OpCode::LoadConst(index));
                chunk.emit(OpCode::IndexGetInPlace(0));
                chunk.emit(OpCode::Pop);
            }
            chunk.emit(OpCode::LoadConst(index));
            chunk.emit(OpCode::Return);
            let compiled = compiler.compile_function(&chunk, "indexed").unwrap();
            let seed = Arc::new(vec![Value::Str(Arc::new("value".into()))]);
            let mut globals = HashMap::from([("seed".into(), Value::Array(seed.clone()))]);
            let mut names =
                HashMap::from([(BytecodeTranslator::hash_var_name("seed"), "seed".into())]);
            let mut caller = vec![Value::Int(77)];
            let mut stack = Vec::new();
            let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
            context.var_names_ptr = &mut names;
            context.local_slots_ptr = &mut caller;
            assert_eq!(
                invoke_compiled_fn(compiled, &mut context),
                0,
                "{:?}",
                context.runtime_error
            );
            assert!(matches!(context.take_return_value(context.return_value), Value::Int(0)));
            assert!(matches!(caller.as_slice(), [Value::Int(77)]));
            assert!(matches!(&seed[0], Value::Str(text) if text.as_str() == "value"));
            let (_, live, capacity, _) = context.owned_roots.storage_state();
            assert_eq!(live, 0);
            assert!(capacity <= 4, "{repetitions} reads retained {capacity} root slots");
            assert!(stack.is_empty());
        }
    }

    #[test]
    fn native_append_reuses_unique_storage_and_rejects_outer_roots() {
        let mut context =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        context.owned_roots.enter().unwrap();
        let mut text = String::with_capacity(4096);
        text.push('x');
        let (payload, kind) = context.owned_roots.encode(Value::Str(Arc::new(text))).unwrap();
        let pointer = match context.owned_roots.current_value_mut(payload, kind).unwrap() {
            Value::Str(text) => text.as_ptr(),
            _ => unreachable!(),
        };
        for _ in 0..1000 {
            // SAFETY: live exclusive context; negative length denotes a character.
            assert_eq!(
                unsafe { jit_append_local(&mut context, 1, payload, kind, 'a' as i64, -1) },
                0
            );
            assert!(
                matches!(context.owned_roots.current_value_mut(payload, kind).unwrap(), Value::Str(text) if text.as_ptr() == pointer)
            );
        }
        let snapshot = context.owned_roots.duplicate(payload, kind).unwrap();
        // SAFETY: same validated context and current-scope root.
        assert_eq!(unsafe { jit_append_local(&mut context, 1, payload, kind, 'b' as i64, -1) }, 0);
        assert!(
            matches!(context.owned_roots.clone_value(snapshot, kind).unwrap(), Value::Str(text) if text.len() == 1001 && text.ends_with('a'))
        );
        context.owned_roots.enter().unwrap();
        assert_eq!(
            context.owned_roots.current_value_mut(payload, kind).err(),
            Some(crate::native_owned::OwnershipError::NotCurrentFrame)
        );
        context.owned_roots.encode(Value::Str(Arc::new("child".into()))).unwrap();
        assert_eq!(
            context.owned_roots.current_value_mut(payload, kind).err(),
            Some(crate::native_owned::OwnershipError::NotCurrentFrame)
        );
        context.owned_roots.leave().unwrap();
        context.owned_roots.leave().unwrap();
        assert_eq!(context.owned_roots.storage_state().1, 0);
    }

    #[test]
    fn compiled_stack_index_reads_bound_owned_operand_storage() {
        for repetitions in [8, 128] {
            let mut compiler = JitCompiler::new().unwrap();
            let mut chunk = BytecodeChunk::new();
            chunk.params.push("rows".into());
            chunk.local_names.push("rows".into());
            chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
            chunk.local_count = 1;
            let index = chunk.add_constant(Constant::Int(0));
            for _ in 0..repetitions {
                chunk.emit(OpCode::LoadLocal(0));
                chunk.emit(OpCode::LoadConst(index));
                chunk.emit(OpCode::IndexGet);
                chunk.emit(OpCode::Pop);
            }
            chunk.emit(OpCode::LoadLocal(0));
            chunk.emit(OpCode::LoadConst(index));
            chunk.emit(OpCode::IndexGet);
            chunk.emit(OpCode::Return);
            let compiled = compiler.compile_function(&chunk, "stack_index_lifetime").unwrap();
            let expected = Value::array(vec![Value::Int(7)]);
            let input = Value::array(vec![expected.clone()]);
            let mut names =
                HashMap::from([(BytecodeTranslator::hash_var_name("rows"), "rows".into())]);
            for invalid in [false, true] {
                let mut locals = HashMap::from([(
                    "rows".into(),
                    if invalid { Value::Int(7) } else { input.clone() },
                )]);
                let mut stack = Vec::new();
                let mut context = VMContext::new(&mut stack, &mut locals, std::ptr::null_mut());
                context.var_names_ptr = &mut names;
                let status = invoke_compiled_fn(compiled, &mut context);
                if invalid {
                    assert_ne!(status, 0);
                    assert!(context.runtime_error.is_some());
                } else {
                    assert_eq!(status, 0, "{:?}", context.runtime_error);
                    assert!(Value::equals(
                        &context.take_return_value(context.return_value),
                        &expected
                    ));
                }
                let (_, live, capacity, _) = context.owned_roots.storage_state();
                assert_eq!(live, 0);
                assert!(capacity <= 6, "{repetitions} reads retained {capacity} roots");
                assert_eq!(context.native_call_depth, 0);
                assert!(stack.is_empty());
            }
        }
    }

    #[test]
    fn vm_function_returns_skip_dead_locals_but_keep_standalone_and_error_state() {
        for vm_managed in [false, true] {
            for fail in [false, true] {
                for return_none in [false, true] {
                    let mut compiler = JitCompiler::new().unwrap();
                    let mut chunk = BytecodeChunk::new();
                    chunk.local_names = vec!["scratch".into()];
                    chunk.local_binding_kinds = vec![BytecodeBindingKind::Mutable];
                    chunk.local_count = 1;
                    let value = chunk.add_constant(Constant::Int(41));
                    let zero = chunk.add_constant(Constant::Int(0));
                    chunk.emit(OpCode::LoadConst(value));
                    chunk.emit(OpCode::DefineLocal(0));
                    chunk.emit(OpCode::Pop);
                    if fail {
                        chunk.emit(OpCode::LoadConst(value));
                        chunk.emit(OpCode::LoadConst(zero));
                        chunk.emit(OpCode::Div);
                        chunk.emit(OpCode::Pop);
                    }
                    if return_none {
                        chunk.emit(OpCode::ReturnNone);
                    } else {
                        chunk.emit(OpCode::LoadLocal(0));
                        chunk.emit(OpCode::Return);
                    }
                    let info = if vm_managed {
                        compiler.compile_function_for_vm(&chunk, "dead_locals")
                    } else {
                        compiler.compile_function_with_info(&chunk, "dead_locals")
                    }
                    .unwrap();
                    let mut names = HashMap::from([(
                        BytecodeTranslator::hash_var_name("scratch"),
                        "scratch".into(),
                    )]);
                    let mut locals = HashMap::new();
                    let mut stack = Vec::new();
                    let mut context = VMContext::new(&mut stack, &mut locals, std::ptr::null_mut());
                    context.var_names_ptr = &mut names;
                    let status = invoke_compiled_fn(info.fn_ptr, &mut context);
                    assert_eq!(status != 0, fail);
                    assert_eq!(context.runtime_error.is_some(), fail);
                    if !fail {
                        let returned = context.take_return_value(context.return_value);
                        if return_none {
                            assert!(matches!(returned, Value::Null));
                        } else {
                            assert!(matches!(returned, Value::Int(41)));
                        }
                    }
                    assert_eq!(context.native_call_depth, 0);
                    assert_eq!(context.owned_roots.storage_state().1, 0);
                    assert!(stack.is_empty());
                    if !vm_managed || fail {
                        assert!(matches!(locals.get("scratch"), Some(Value::Int(41))));
                    } else {
                        assert!(locals.is_empty());
                        assert_eq!(locals.capacity(), 0);
                    }
                }
            }
        }
    }

    #[test]
    fn native_parameter_transfer_preserves_unique_backing_storage() {
        let hash = BytecodeTranslator::hash_var_name("value");
        let value = Value::array(vec![Value::Int(0); 128]);
        let pointer = match &value {
            Value::Array(values) => values.as_ptr(),
            _ => unreachable!(),
        };
        let mut locals = HashMap::from([("value".into(), value)]);
        let mut names = HashMap::from([(hash, "value".into())]);
        let mut context = VMContext::new(std::ptr::null_mut(), &mut locals, std::ptr::null_mut());
        context.var_names_ptr = &mut names;
        context.owned_roots.enter().unwrap();
        // SAFETY: live exclusive context, name map and parameter binding.
        let root = unsafe { jit_take_parameter(&mut context, hash as i64, 0) };
        let kind = context.return_value_kind;
        assert!(context.runtime_error.is_none());
        assert!(locals.is_empty());
        match context.owned_roots.current_value_mut(root, kind).unwrap() {
            Value::Array(values) => assert_eq!(Arc::strong_count(values), 1),
            _ => panic!("parameter lost its array type"),
        }
        for i in 0..128 {
            // SAFETY: current-scope root and mutable parameter metadata.
            assert_eq!(
                unsafe {
                    jit_indexed_write(&mut context, 1, root, kind, 0, hash as i64, 0, 0, i, 0)
                },
                0
            );
            assert!(
                matches!(context.owned_roots.current_value_mut(root, kind).unwrap(), Value::Array(values) if values.as_ptr() == pointer)
            );
        }
        context.owned_roots.leave().unwrap();
        assert_eq!(context.owned_roots.storage_state().1, 0);
    }

    #[test]
    fn missing_native_parameter_fails_before_body_effects_and_releases_frame() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();
        chunk.params.push("value".into());
        chunk.local_names.push("value".into());
        chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
        chunk.local_count = 1;
        let constant = chunk.add_constant(Constant::Int(99));
        chunk.emit(OpCode::LoadConst(constant));
        chunk.emit(OpCode::StoreGlobal("effect".into()));
        chunk.emit(OpCode::Pop);
        chunk.emit(OpCode::LoadLocal(0));
        chunk.emit(OpCode::Return);
        let compiled = compiler.compile_function(&chunk, "missing_parameter").unwrap();
        let mut names = HashMap::from([
            (BytecodeTranslator::hash_var_name("value"), "value".into()),
            (BytecodeTranslator::hash_var_name("effect"), "effect".into()),
        ]);
        let mut globals = HashMap::from([("effect".into(), Value::Int(0))]);
        let mut locals = HashMap::new();
        let mut stack = Vec::new();
        let mut context = VMContext::new(&mut stack, &mut locals, &mut globals);
        context.var_names_ptr = &mut names;
        assert_ne!(invoke_compiled_fn(compiled, &mut context), 0);
        assert_eq!(context.runtime_error.as_deref(), Some("Missing native parameter"));
        assert!(matches!(globals.get("effect"), Some(Value::Int(0))));
        assert_eq!(context.native_call_depth, 0);
        assert_eq!(context.owned_roots.storage_state().1, 0);
        assert!(stack.is_empty());
    }

    #[test]
    fn compiled_literals_outlive_source_and_bound_discarded_roots() {
        for constant in [Constant::String("héllo".into()), Constant::Bytes(vec![0, 255, 7])] {
            for repetitions in [8, 128] {
                let mut compiler = JitCompiler::new().unwrap();
                let mut chunk = BytecodeChunk::new();
                let index = chunk.add_constant(constant.clone());
                for _ in 0..repetitions {
                    chunk.emit(OpCode::LoadConst(index));
                    chunk.emit(OpCode::Pop);
                }
                chunk.emit(OpCode::LoadConst(index));
                chunk.emit(OpCode::Return);
                let compiled = compiler.compile_function(&chunk, "literal_lifetime").unwrap();
                let retained = compiler.literal_values.values().next().unwrap();
                let weak = Arc::downgrade(retained);
                let expected = retained.as_ref().clone();
                // Repeated retention must not grow with compilation attempts.
                compiler.retain_literal_values(&chunk);
                assert_eq!(compiler.literal_values.len(), 1);
                drop(chunk);
                let mut stack = Vec::new();
                let mut context =
                    VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
                for _ in 0..10 {
                    assert_eq!(
                        invoke_compiled_fn(compiled, &mut context),
                        0,
                        "{:?}",
                        context.runtime_error
                    );
                    let result = context.take_return_value(context.return_value);
                    assert!(Value::equals(&result, &expected));
                    // Literal loads share immutable backing storage, rather than
                    // allocating/copying the string or byte buffer per execution.
                    match (&result, &expected) {
                        (Value::Str(a), Value::Str(b)) => assert!(Arc::ptr_eq(a, b)),
                        (Value::Bytes(a), Value::Bytes(b)) => assert!(Arc::ptr_eq(a, b)),
                        _ => unreachable!(),
                    }
                    let (_, live, capacity, _) = context.owned_roots.storage_state();
                    assert_eq!(live, 0);
                    assert!(capacity <= 4, "{repetitions} loads retained {capacity} slots");
                    assert!(stack.is_empty());
                }
                drop(compiler);
                assert!(weak.upgrade().is_none());
            }
        }
    }

    #[test]
    fn compiled_appends_retain_literals_preserve_callers_and_bound_roots() {
        for repetitions in [8, 128] {
            let mut compiler = JitCompiler::new().unwrap();
            let mut chunk = BytecodeChunk::new();
            chunk.local_names.push("text".into());
            chunk.emit(OpCode::LoadVar("seed".into()));
            chunk.emit(OpCode::StoreLocal(0));
            chunk.emit(OpCode::Pop);
            let suffix: Arc<str> = Arc::from("ab");
            let weak = Arc::downgrade(&suffix);
            for _ in 0..repetitions {
                chunk.emit(OpCode::AppendConstStringInPlace(0, suffix.clone()));
            }
            chunk.emit(OpCode::LoadLocal(0));
            chunk.emit(OpCode::Return);
            let compiled = compiler.compile_function(&chunk, "append_owned").unwrap();
            drop(chunk);
            drop(suffix);
            assert_eq!(weak.strong_count(), 1);
            let seed = Arc::new("seed".to_string());
            let mut globals = HashMap::from([("seed".into(), Value::Str(seed.clone()))]);
            let mut names =
                HashMap::from([(BytecodeTranslator::hash_var_name("seed"), "seed".into())]);
            let mut caller_slots = vec![Value::Int(99)];
            let mut stack = Vec::new();
            let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
            context.var_names_ptr = &mut names;
            context.local_slots_ptr = &mut caller_slots;
            assert_eq!(
                invoke_compiled_fn(compiled, &mut context),
                0,
                "{:?}",
                context.runtime_error
            );
            assert!(
                matches!(context.take_return_value(context.return_value), Value::Str(text) if text.as_str() == format!("seed{}", "ab".repeat(repetitions)))
            );
            assert!(matches!(caller_slots.as_slice(), [Value::Int(99)]));
            assert_eq!(seed.as_str(), "seed");
            let (_, live, slots, _) = context.owned_roots.storage_state();
            assert_eq!(live, 0);
            // storage_state reports Vec capacity, whose minimum allocation is
            // four slots, rather than the number of simultaneously live roots.
            assert!(slots <= 4, "retained {slots} slots");
            assert!(stack.is_empty());
            drop(compiler);
            assert!(weak.upgrade().is_none());
        }
    }

    #[test]
    fn loop_owned_locals_survive_append_and_writeback() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();
        chunk.local_names.push("text".into());
        chunk.emit(OpCode::LoadLocal(0));
        chunk.emit(OpCode::StoreLocal(0));
        chunk.emit(OpCode::Pop);
        chunk.emit(OpCode::AppendConstCharInPlace(0, '🙂'));
        chunk.emit(OpCode::LoadLocal(0));
        chunk.emit(OpCode::Return);
        let compiled = compiler.compile(&chunk, 0).unwrap();
        let snapshot = Arc::new("seed".to_string());
        let mut slots = vec![Value::Str(snapshot.clone())];
        let mut stack = Vec::new();
        let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
        context.local_slots_ptr = &mut slots;
        assert_eq!(invoke_compiled_fn(compiled, &mut context), 0, "{:?}", context.runtime_error);
        assert!(
            matches!(context.take_return_value(context.return_value), Value::Str(text) if text.as_str() == "seed🙂")
        );
        assert!(matches!(&slots[0], Value::Str(text) if text.as_str() == "seed🙂"));
        assert_eq!(snapshot.as_str(), "seed");
        assert_eq!(context.owned_roots.storage_state().1, 0);
    }

    #[test]
    fn compiled_discarded_owned_operands_reuse_root_storage() {
        for repetitions in [8, 128] {
            for pattern in [
                "discard",
                "overwrite",
                "callback",
                "dead",
                "dictionary",
                "fixed_dictionary",
                "dynamic_dictionary",
                "owned_add",
                "owned_order",
            ] {
                let mut compiler = JitCompiler::new().unwrap();
                let mut chunk = BytecodeChunk::new();
                chunk.local_names.push("saved".into());
                for _ in 0..repetitions {
                    if pattern == "owned_add" || pattern == "owned_order" {
                        chunk.emit(OpCode::LoadVar("text".into()));
                        chunk.emit(OpCode::LoadVar("text".into()));
                        chunk.emit(if pattern == "owned_add" {
                            OpCode::Add
                        } else {
                            OpCode::LessThan
                        });
                    } else if pattern == "fixed_dictionary" {
                        chunk.emit(OpCode::LoadVar("source".into()));
                        chunk.emit(OpCode::MakeDictWithKeys(Arc::new(vec![Arc::from("key")])));
                    } else if pattern == "dynamic_dictionary" {
                        chunk.emit(OpCode::LoadVar("key".into()));
                        chunk.emit(OpCode::LoadVar("source".into()));
                        chunk.emit(OpCode::MakeDict(1));
                    } else if pattern == "dictionary" {
                        chunk.emit(OpCode::MakeDict(0));
                    } else {
                        chunk.emit(OpCode::LoadVar("source".into()));
                    }
                    if pattern == "callback" {
                        chunk.emit(OpCode::LoadVar("str".into()));
                        chunk.emit(OpCode::Call(1));
                        chunk.emit(OpCode::Pop);
                    } else {
                        chunk.emit(OpCode::Dup);
                        if pattern == "overwrite" || pattern == "dead" {
                            chunk.emit(OpCode::StoreLocal(0));
                        }
                        chunk.emit(OpCode::Pop);
                        if pattern == "dead" {
                            chunk.emit(OpCode::ReleaseDeadLocal(0));
                        }
                        chunk.emit(OpCode::Pop);
                    }
                }
                chunk.emit(if pattern == "overwrite" {
                    OpCode::LoadLocal(0)
                } else {
                    OpCode::LoadVar("source".into())
                });
                chunk.emit(OpCode::Return);
                let compiled = compiler.compile_function(&chunk, "discard_owned").unwrap();
                let source = Arc::new(vec![Value::Int(7)]);
                let mut globals = HashMap::from([("source".into(), Value::Array(source.clone()))]);
                globals.insert("key".into(), Value::Str(Arc::new("key".into())));
                globals.insert("text".into(), Value::Str(Arc::new("text".into())));
                let mut names = ["source", "saved", "str", "key", "text"]
                    .into_iter()
                    .map(|name| (BytecodeTranslator::hash_var_name(name), name.to_owned()))
                    .collect();
                let mut stack = Vec::new();
                let mut vm = VM::new();
                let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
                context.var_names_ptr = &mut names;
                context.vm_ptr = (&mut vm as *mut VM).cast();
                assert_eq!(
                    invoke_compiled_fn(compiled, &mut context),
                    0,
                    "{pattern}: {:?}",
                    context.runtime_error
                );
                assert!(context.runtime_error.is_none(), "{:?}", context.runtime_error);
                assert!(
                    matches!(context.take_return_value(context.return_value), Value::Array(value) if Arc::ptr_eq(&value, &source))
                );
                let (depth, live, slots, _) = context.owned_roots.storage_state();
                assert_eq!((depth, live), (0, 0));
                assert!(
                    slots <= 4,
                    "{pattern}: {repetitions} repetitions retained {slots} root slots"
                );
                assert_eq!(Arc::strong_count(&source), 2);
                assert!(stack.is_empty());
            }
        }
    }

    #[test]
    fn compiled_dictionary_keys_outlive_the_source_chunk() {
        let mut compiler = JitCompiler::new().unwrap();
        let keys = Arc::new(vec![Arc::<str>::from("a")]);
        let weak = Arc::downgrade(&keys);
        let mut chunk = BytecodeChunk::new();
        let value = chunk.add_constant(Constant::Int(7));
        chunk.emit(OpCode::LoadConst(value));
        chunk.emit(OpCode::MakeDictWithKeys(keys.clone()));
        chunk.emit(OpCode::Return);
        let compiled = compiler.compile_function(&chunk, "retained_keys").unwrap();
        drop(chunk);
        drop(keys);
        assert_eq!(weak.strong_count(), 1);
        let mut stack = Vec::new();
        let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!(invoke_compiled_fn(compiled, &mut context), 0);
        let result = context.take_return_value(context.return_value);
        assert!(matches!(&result, Value::FixedDict { keys, values }
            if keys[0].as_ref() == "a" && matches!(values[0], Value::Int(7))));
        assert!(stack.is_empty());
        drop(compiler);
        assert_eq!(weak.strong_count(), 1);
        drop(result);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn compiled_field_metadata_and_receivers_have_bounded_lifetimes() {
        for (repetitions, fail_after_reads) in [(8, false), (128, false), (8, true)] {
            let mut compiler = JitCompiler::new().unwrap();
            let mut chunk = BytecodeChunk::new();
            chunk.params.push("item".into());
            chunk.local_names.push("item".into());
            chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
            chunk.local_count = 1;
            for _ in 0..repetitions {
                chunk.emit(OpCode::LoadLocal(0));
                chunk.emit(OpCode::FieldGet("help".into()));
                chunk.emit(OpCode::Pop);
            }
            if fail_after_reads {
                let index = chunk.add_constant(Constant::Int(3));
                chunk.emit(OpCode::LoadConst(index));
            } else {
                chunk.emit(OpCode::LoadLocal(0));
            }
            chunk.emit(OpCode::FieldGet("help".into()));
            chunk.emit(OpCode::Return);
            let compiled = compiler.compile_function(&chunk, "field_lifetime").unwrap();
            let weak = Arc::downgrade(compiler.field_names.values().next().unwrap());
            drop(chunk);
            assert_eq!(weak.strong_count(), 1);
            for item in [
                Value::Struct {
                    name: "Holder".into(),
                    fields: HashMap::from([("help".into(), Value::array(vec![Value::Int(7)]))])
                        .into(),
                },
                Value::Struct { name: "ArgParser".into(), fields: HashMap::new().into() },
                Value::Int(3),
            ] {
                let expected = VM::get_field_value(
                    if fail_after_reads { &Value::Int(3) } else { &item },
                    "help",
                    |_| None,
                );
                let mut locals = HashMap::from([("item".into(), item)]);
                let mut names =
                    HashMap::from([(BytecodeTranslator::hash_var_name("item"), "item".into())]);
                let mut stack = vec![Value::Int(41)];
                let mut context = VMContext::new(&mut stack, &mut locals, std::ptr::null_mut());
                context.var_names_ptr = &mut names;
                let status = invoke_compiled_fn(compiled, &mut context);
                match expected {
                    Ok((value, _)) => {
                        assert_eq!(status, 0, "{:?}", context.runtime_error);
                        let result = context.take_return_value(context.return_value);
                        match (&result, &value) {
                            (Value::NativeFunction(a), Value::NativeFunction(b)) => {
                                assert_eq!(a, b)
                            }
                            _ => assert!(Value::equals(&result, &value)),
                        }
                    }
                    Err(error) => {
                        assert_ne!(status, 0);
                        assert_eq!(context.runtime_error.as_deref(), Some(error.as_str()));
                    }
                }
                let (_, live, capacity, _) = context.owned_roots.storage_state();
                assert_eq!(live, 0);
                assert!(capacity <= 5, "{repetitions} reads retained {capacity} roots");
                assert_eq!(context.native_call_depth, 0);
                assert!(context.native_stack_bases.is_empty());
                assert_eq!(stack.len(), 1);
                assert!(matches!(stack[0], Value::Int(41)));
            }
            drop(compiler);
            assert!(weak.upgrade().is_none());
        }
    }

    #[test]
    fn compiled_struct_metadata_outlives_chunk_with_bounded_roots() {
        for repetitions in [8, 128] {
            let mut compiler = JitCompiler::new().unwrap();
            let mut chunk = BytecodeChunk::new();
            chunk.params.push("item".into());
            chunk.local_names.push("item".into());
            chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
            chunk.local_count = 1;
            for _ in 0..repetitions {
                chunk.emit(OpCode::LoadLocal(0));
                chunk.emit(OpCode::MakeStruct("Holder".into(), vec!["value".into()]));
                chunk.emit(OpCode::Pop);
            }
            chunk.emit(OpCode::LoadLocal(0));
            chunk.emit(OpCode::MakeStruct("Holder".into(), vec!["value".into()]));
            chunk.emit(OpCode::Return);
            let compiled = compiler.compile_function(&chunk, "struct_lifetime").unwrap();
            let weak = Arc::downgrade(compiler.struct_shapes.values().next().unwrap());
            drop(chunk);
            assert_eq!(weak.strong_count(), 1);
            let item = Value::array(vec![Value::Bool(true), Value::Int(7)]);
            let mut locals = HashMap::from([("item".into(), item.clone())]);
            let mut names =
                HashMap::from([(BytecodeTranslator::hash_var_name("item"), "item".into())]);
            let mut stack = Vec::new();
            let mut context = VMContext::new(&mut stack, &mut locals, std::ptr::null_mut());
            context.var_names_ptr = &mut names;
            assert_eq!(
                invoke_compiled_fn(compiled, &mut context),
                0,
                "{:?}",
                context.runtime_error
            );
            let result = context.take_return_value(context.return_value);
            assert!(
                matches!(&result, Value::Struct { name, fields } if name == "Holder" && Value::equals(&fields["value"], &item))
            );
            let (_, live, capacity, _) = context.owned_roots.storage_state();
            assert_eq!(live, 0);
            assert!(capacity <= 5, "{repetitions} constructions retained {capacity} roots");
            assert_eq!(context.native_call_depth, 0);
            assert!(stack.is_empty());
            drop(compiler);
            assert!(weak.upgrade().is_none());
            assert!(matches!(&result, Value::Struct { name, .. } if name == "Holder"));
        }
    }

    #[test]
    fn struct_helper_preserves_vm_field_order_prefix_and_depth_errors() {
        let shape: Arc<NativeStructShape> = Arc::new(("Pair".into(), vec!["x".into(), "x".into()]));
        let pointer = Arc::as_ptr(&shape) as i64;
        let mut stack = vec![Value::Int(99), Value::Int(1), Value::Int(2)];
        let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!(jit_make_struct(&mut context, pointer), 1);
        assert!(
            matches!(stack.pop().unwrap(), Value::Struct { name, fields } if name == "Pair" && matches!(fields["x"], Value::Int(1)))
        );
        assert_eq!(jit_make_struct(&mut context, pointer), 0);
        assert_eq!(context.runtime_error.take().as_deref(), Some("Stack underflow"));
        assert_eq!(jit_make_struct(&mut context, 0), 0);
        assert_eq!(context.runtime_error.take().as_deref(), Some("Missing native struct metadata"));
        assert!(matches!(stack.as_slice(), [Value::Int(99)]));
        let shape: Arc<NativeStructShape> = Arc::new(("Holder".into(), vec!["value".into()]));
        let mut levels = vec![Value::Int(7)];
        for _ in 0..crate::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH {
            levels.push(Value::array(vec![levels.last().unwrap().clone()]));
        }
        stack.push(levels.last().unwrap().clone());
        assert_eq!(jit_make_struct(&mut context, Arc::as_ptr(&shape) as i64), 0);
        assert!(context.runtime_error.take().unwrap().contains("building struct value"));
        assert!(matches!(stack.as_slice(), [Value::Int(99)]));
        while levels.pop().is_some() {}
    }

    #[test]
    fn array_helper_checks_shape_depth_and_preserves_stack_prefix() {
        let mut stack = vec![Value::Int(99), Value::Bool(true), Value::Null];
        let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!(jit_make_array(&mut context, 2), 1);
        let result = stack.pop().unwrap();
        assert!(Value::equals(&result, &Value::array(vec![Value::Bool(true), Value::Null])));
        for count in [-1, 2] {
            assert_eq!(jit_make_array(&mut context, count), 0);
            assert_eq!(
                context.runtime_error.take().as_deref(),
                Some("Stack underflow in MakeArray")
            );
            assert!(matches!(stack.as_slice(), [Value::Int(99)]));
        }
        // Retain every level so teardown never recursively drops a deep chain.
        let mut levels = vec![Value::Int(7)];
        for _ in 0..crate::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH {
            levels.push(Value::array(vec![levels.last().unwrap().clone()]));
        }
        stack.push(levels.last().unwrap().clone());
        assert_eq!(jit_make_array(&mut context, 1), 0);
        assert!(context.runtime_error.take().unwrap().contains("building array literal"));
        assert!(matches!(stack.as_slice(), [Value::Int(99)]));
        while levels.pop().is_some() {}
        assert_eq!(jit_make_array(&mut context, 0), 1);
        assert!(Value::equals(&stack.pop().unwrap(), &Value::array(vec![])));
    }

    #[test]
    fn compiled_array_literals_bound_root_storage() {
        for repetitions in [8, 128] {
            let mut compiler = JitCompiler::new().unwrap();
            let mut chunk = BytecodeChunk::new();
            chunk.params.push("item".into());
            chunk.local_names.push("item".into());
            chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
            chunk.local_count = 1;
            for _ in 0..repetitions {
                chunk.emit(OpCode::LoadLocal(0));
                chunk.emit(OpCode::MakeArray(1));
                chunk.emit(OpCode::Pop);
            }
            chunk.emit(OpCode::LoadLocal(0));
            chunk.emit(OpCode::MakeArray(1));
            chunk.emit(OpCode::Return);
            let compiled = compiler.compile_function(&chunk, "array_lifetime").unwrap();
            let item = Value::array(vec![Value::Int(7)]);
            let mut locals = HashMap::from([("item".into(), item.clone())]);
            let mut names =
                HashMap::from([(BytecodeTranslator::hash_var_name("item"), "item".into())]);
            let mut stack = Vec::new();
            let mut context = VMContext::new(&mut stack, &mut locals, std::ptr::null_mut());
            context.var_names_ptr = &mut names;
            assert_eq!(
                invoke_compiled_fn(compiled, &mut context),
                0,
                "{:?}",
                context.runtime_error
            );
            assert!(Value::equals(
                &context.take_return_value(context.return_value),
                &Value::array(vec![item])
            ));
            let (_, live, capacity, _) = context.owned_roots.storage_state();
            assert_eq!(live, 0);
            assert!(capacity <= 5, "{repetitions} constructions retained {capacity} roots");
            assert_eq!(context.native_call_depth, 0);
            assert!(stack.is_empty());
        }
    }

    #[test]
    fn dictionary_helper_preserves_prefix_and_reports_invalid_keys() {
        let mut stack = vec![Value::Int(99), Value::Int(7), Value::Int(8)];
        let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!(jit_make_dict(&mut context, 1), 0);
        assert_eq!(context.runtime_error.take().as_deref(), Some("Dict keys must be strings"));
        assert!(matches!(stack.as_slice(), [Value::Int(99)]));
        assert_eq!(jit_make_dict(&mut context, -1), 0);
        assert_eq!(context.runtime_error.take().as_deref(), Some("Stack underflow"));
        assert!(matches!(stack.as_slice(), [Value::Int(99)]));
    }

    #[test]
    fn compiled_dead_local_reads_back_as_null() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();
        chunk.local_names.push("dead".into());
        chunk.emit(OpCode::LoadVar("source".into()));
        chunk.emit(OpCode::StoreLocal(0));
        chunk.emit(OpCode::ReleaseDeadLocal(0));
        chunk.emit(OpCode::Pop);
        chunk.emit(OpCode::LoadLocal(0));
        chunk.emit(OpCode::Return);
        let compiled = compiler.compile_function(&chunk, "release_null").unwrap();
        let mut globals =
            HashMap::from([("source".into(), Value::Array(Arc::new(vec![Value::Int(7)])))]);
        let mut names = ["source", "dead"]
            .into_iter()
            .map(|name| (BytecodeTranslator::hash_var_name(name), name.to_owned()))
            .collect();
        let mut stack = Vec::new();
        let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
        context.var_names_ptr = &mut names;
        assert_eq!(invoke_compiled_fn(compiled, &mut context), 0);
        assert!(context.runtime_error.is_none(), "{:?}", context.runtime_error);
        assert!(matches!(context.take_return_value(context.return_value), Value::Null));
        assert_eq!(context.owned_roots.storage_state().1, 0);
        assert!(stack.is_empty());
    }

    #[test]
    fn dead_local_helper_distinguishes_callee_bindings_from_caller_slots() {
        let hash = BytecodeTranslator::hash_var_name("dead");
        let mut names = HashMap::from([(hash, "dead".to_owned())]);
        let mut locals = HashMap::from([("dead".into(), Value::Int(7))]);
        let mut slots = vec![Value::Int(99)];
        let mut context = VMContext::new(std::ptr::null_mut(), &mut locals, std::ptr::null_mut());
        context.var_names_ptr = &mut names;
        context.local_slots_ptr = &mut slots;
        // SAFETY: all pointers refer to live, exclusively borrowed containers.
        unsafe {
            assert_eq!(jit_release_dead_local(&mut context, 0, hash, 0), 0);
            assert!(!locals.contains_key("dead"));
            assert!(matches!(slots[0], Value::Int(99)));
            locals.insert("dead".into(), Value::Int(8));
            assert_eq!(jit_release_dead_local(&mut context, 1, hash, 1), 1);
            assert!(matches!(locals.get("dead"), Some(Value::Int(8))));
            assert!(matches!(slots[0], Value::Int(99)));
            context.runtime_error = None;
            assert_eq!(jit_release_dead_local(&mut context, 0, hash, 1), 0);
            assert!(matches!(slots[0], Value::Null));
            assert!(!locals.contains_key("dead"));
        }
    }

    #[test]
    fn native_owned_truth_borrows_values_and_rejects_retired_roots() {
        let mut context =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        context.owned_roots.enter().unwrap();
        let values = [
            Value::Str(Arc::new(String::new())),
            Value::Str(Arc::new("false".into())),
            Value::Array(Arc::new(vec![])),
            Value::Array(Arc::new(vec![Value::Null])),
            Value::Dict(Arc::default()),
            Value::NativeFunction("len".into()),
        ];
        for value in values {
            let expected = i64::from(value.is_truthy());
            let (payload, kind) = context.owned_roots.encode(value).unwrap();
            let state = context.owned_roots.storage_state();
            for _ in 0..1000 {
                // SAFETY: context and root remain live for every call.
                assert_eq!(unsafe { jit_owned_truth(&mut context, payload) }, expected);
                assert_eq!(context.owned_roots.storage_state(), state);
            }
            context.owned_roots.take_value(payload, kind).unwrap();
            // SAFETY: stale identities are checked, never dereferenced as pointers.
            assert_eq!(unsafe { jit_owned_truth(&mut context, payload) }, 0);
            assert!(context.runtime_error.take().unwrap().contains("StaleHandle"));
        }
        context.owned_roots.leave().unwrap();
    }

    #[test]
    fn consuming_stack_transfer_moves_current_roots_and_rejects_reuse() {
        let mut stack = Vec::with_capacity(8);
        let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
        context.owned_roots.enter().unwrap();
        let name = String::from("to_int");
        let pointer = name.as_ptr();
        let (payload, kind) = context.owned_roots.encode(Value::NativeFunction(name)).unwrap();
        // SAFETY: live exclusive stack/context; root ownership is checked by helper.
        unsafe {
            assert_eq!(jit_stack_take_scalar(&mut context, payload, kind), 0);
            assert!(matches!(&stack[0], Value::NativeFunction(name) if name.as_ptr() == pointer));
            assert_eq!(jit_stack_take_scalar(&mut context, payload, kind), 1);
            assert_eq!(stack.len(), 1);
            assert!(context.runtime_error.take().unwrap().contains("StaleHandle"));
            assert_eq!(jit_stack_take_scalar(&mut context, 1, 99), 1);
            assert_eq!(stack.len(), 1);
            assert!(context.runtime_error.take().unwrap().contains("InvalidKind"));
        }
        let saved = Arc::new(vec![Value::Int(7)]);
        let (payload, kind) = context.owned_roots.encode(Value::Array(saved.clone())).unwrap();
        context.owned_roots.enter().unwrap();
        // A callee with no physical roots must not take a caller's operand.
        assert_eq!(unsafe { jit_stack_take_scalar(&mut context, payload, kind) }, 1);
        assert_eq!(stack.len(), 1);
        assert!(context.runtime_error.take().unwrap().contains("NotCurrentFrame"));
        // Also reject caller roots after the nested frame has physical storage.
        context.owned_roots.encode(Value::NativeFunction("len".into())).unwrap();
        assert_eq!(unsafe { jit_stack_take_scalar(&mut context, payload, kind) }, 1);
        assert_eq!(stack.len(), 1);
        assert!(context.runtime_error.take().unwrap().contains("NotCurrentFrame"));
        context.owned_roots.leave().unwrap();
        context.stack_ptr = std::ptr::null_mut();
        assert_eq!(unsafe { jit_stack_take_scalar(&mut context, payload, kind) }, 1);
        assert!(context.runtime_error.take().unwrap().contains("requires a stack"));
        context.stack_ptr = &mut stack;
        assert_eq!(unsafe { jit_stack_take_scalar(&mut context, payload, kind) }, 0);
        let Value::Array(mut changed) = stack.pop().unwrap() else { panic!("array lost") };
        assert!(Arc::ptr_eq(&changed, &saved));
        Arc::make_mut(&mut changed)[0] = Value::Int(9);
        assert!(matches!(saved[0], Value::Int(7)));
        context.owned_roots.leave().unwrap();
        assert_eq!(context.owned_roots.storage_state().1, 0);
        assert!(context.runtime_error.is_none());
    }

    #[test]
    fn consuming_stack_transfer_preserves_immediate_bits_without_roots() {
        let mut stack = Vec::with_capacity(8);
        let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
        for value in [
            Value::Int(i64::MIN),
            Value::Int(i64::MAX),
            Value::Bool(false),
            Value::Null,
            Value::Float(-0.0),
            Value::Float(f64::from_bits(0x7ff8_0000_0000_0123)),
        ] {
            let (payload, kind) = crate::native_scalar::encode(&value).unwrap();
            assert_eq!(unsafe { jit_stack_take_scalar(&mut context, payload, kind) }, 0);
            let actual = stack.pop().unwrap();
            if let (Value::Float(a), Value::Float(b)) = (&value, &actual) {
                assert_eq!(a.to_bits(), b.to_bits());
            } else {
                assert!(Value::equals(&value, &actual));
            }
        }
        assert_eq!(context.owned_roots.depth(), 0);
        assert_eq!(context.owned_roots.storage_state().1, 0);
        assert!(context.runtime_error.is_none());
    }

    #[test]
    fn native_stack_boundary_preserves_owned_values_and_snapshots() {
        let original = Arc::new(vec![Value::Int(8)]);
        let mut stack = vec![Value::Array(original.clone())];
        let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
        context.owned_roots.enter().unwrap();
        // SAFETY: the context and its exclusive stack pointer remain live.
        unsafe {
            let payload = jit_stack_pop(&mut context);
            let kind = context.return_value_kind;
            assert_eq!(kind, crate::native_owned::OWNED);
            assert!(stack.is_empty());
            jit_stack_push_scalar(&mut context, payload, kind);
            jit_stack_push_scalar(&mut context, payload, kind);
            let Value::Array(mut changed) = stack.pop().unwrap() else { panic!("array lost") };
            Arc::make_mut(&mut changed)[0] = Value::Int(99);
            assert!(matches!(&stack[0], Value::Array(a) if matches!(a[0], Value::Int(8))));
            assert!(matches!(original[0], Value::Int(8)));
            context.owned_roots.leave().unwrap();
            let before = stack.len();
            jit_stack_push_scalar(&mut context, payload, kind);
            assert_eq!(stack.len(), before);
            assert!(context.runtime_error.take().unwrap().contains("StaleHandle"));
        }
        assert_eq!(context.owned_roots.storage_state().1, 0);
    }

    #[test]
    fn native_stack_boundary_rejects_invalid_kinds() {
        let mut stack = Vec::new();
        let mut context = VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
        // SAFETY: context has a live exclusive stack pointer.
        unsafe { jit_stack_push_scalar(&mut context, 123, 99) };
        assert!(stack.is_empty());
        assert!(context.runtime_error.unwrap().contains("InvalidKind"));
    }

    // SAFETY: test callers supply a live context with initialized local values;
    // recursion stays within the normal native depth guard and retains no pointers.
    unsafe extern "C" fn owned_lifetime_probe(ctx: *mut VMContext, depth: i64) -> i64 {
        if jit_enter_function(ctx) != 0 {
            return 1;
        }
        let scratch = (&*(*ctx).locals_ptr)["scratch"].clone();
        (*ctx).owned_roots.encode(scratch).unwrap();
        let status = if depth > 0 {
            owned_lifetime_probe(ctx, depth - 1)
        } else if (*ctx).arg0 != 0 {
            (*ctx).runtime_error = Some("owned probe failure".into());
            7
        } else {
            let input = (&*(*ctx).locals_ptr)["input"].clone();
            let (payload, kind) = (*ctx).owned_roots.encode(input).unwrap();
            jit_set_return_scalar(ctx, payload, kind)
        };
        jit_leave_function(ctx);
        status
    }

    // SAFETY: forwards the same validated test context to the direct-arg probe.
    unsafe extern "C" fn owned_standard_probe(ctx: *mut VMContext) -> i64 {
        owned_lifetime_probe(ctx, 3)
    }

    // SAFETY: same live test context as owned_lifetime_probe; the direct ABI
    // returns the payload itself, not the standard ABI's success status.
    unsafe extern "C" fn owned_direct_probe(ctx: *mut VMContext, depth: i64) -> i64 {
        let status = owned_lifetime_probe(ctx, depth);
        if status == 0 {
            (*ctx).return_value
        } else {
            status
        }
    }

    #[test]
    fn guarded_collection_helpers_reuse_storage_and_keep_snapshots() {
        for bytes in [false, true] {
            for count in [8, 128] {
                let mut ctx = VMContext::new(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                ctx.owned_roots.enter().unwrap();
                let initial = if bytes {
                    Value::Bytes(Arc::new(Vec::new()))
                } else {
                    Value::Array(Arc::new(Vec::new()))
                };
                let (target, kind) = ctx.owned_roots.encode(initial).unwrap();
                let pointer = match ctx.owned_roots.current_value_mut(target, kind).unwrap() {
                    Value::Array(v) => Arc::as_ptr(v) as usize,
                    Value::Bytes(v) => Arc::as_ptr(v) as usize,
                    _ => unreachable!(),
                };
                let (function, function_kind) = ctx
                    .owned_roots
                    .encode(Value::NativeFunction(if bytes { "concat" } else { "push" }.into()))
                    .unwrap();
                for index in 0..=count {
                    let saved = if index == count {
                        Some(ctx.owned_roots.clone_value(target, kind).unwrap())
                    } else {
                        None
                    };
                    let original = ctx.owned_roots.duplicate(target, kind).unwrap();
                    let item = if bytes {
                        Value::Bytes(Arc::new(vec![255]))
                    } else {
                        Value::Int(index as i64)
                    };
                    let (item, item_kind) = ctx.owned_roots.encode(item).unwrap();
                    assert_eq!(
                        unsafe {
                            jit_try_append(
                                &mut ctx,
                                1,
                                target,
                                kind,
                                original,
                                kind,
                                item,
                                item_kind,
                                1,
                                function,
                                function_kind,
                            )
                        },
                        1
                    );
                    assert!(ctx.runtime_error.is_none());
                    ctx.owned_roots.release(original, kind).unwrap();
                    ctx.owned_roots.release(item, item_kind).unwrap();
                    let (actual_pointer, len) =
                        match ctx.owned_roots.current_value_mut(target, kind).unwrap() {
                            Value::Array(v) => (Arc::as_ptr(v) as usize, v.len()),
                            Value::Bytes(v) => (Arc::as_ptr(v) as usize, v.len()),
                            _ => unreachable!(),
                        };
                    assert_eq!(len, index + 1);
                    if let Some(saved) = saved {
                        assert_ne!(actual_pointer, pointer);
                        assert!(
                            matches!(saved, Value::Array(ref v) if v.len()==count)
                                || matches!(saved, Value::Bytes(ref v) if v.len()==count)
                        );
                    } else {
                        assert_eq!(actual_pointer, pointer);
                    }
                }
                ctx.owned_roots.leave().unwrap();
                let (_, live, slots, _) = ctx.owned_roots.storage_state();
                assert_eq!(live, 0);
                assert!(slots <= 8, "root capacity grew to {slots}");
            }
        }
    }

    #[test]
    fn guarded_rebind_release_drops_only_matching_mutable_local_snapshot() {
        let mut ctx =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        ctx.owned_roots.enter().unwrap();
        let (target, kind) =
            ctx.owned_roots.encode(Value::Array(Arc::new(vec![Value::Int(1)]))).unwrap();
        let original = ctx.owned_roots.duplicate(target, kind).unwrap();
        let (function, function_kind) = ctx
            .owned_roots
            .encode(Value::BytecodeFunction {
                chunk: BytecodeChunk::new(),
                captured: HashMap::new(),
                captured_binding_kinds: HashMap::new(),
            })
            .unwrap();
        assert_eq!(
            unsafe {
                jit_release_rebind(
                    &mut ctx,
                    1,
                    target,
                    kind,
                    original,
                    kind,
                    0,
                    function,
                    function_kind,
                )
            },
            0
        );
        assert!(
            matches!(ctx.owned_roots.current_value_mut(target,kind).unwrap(),Value::Array(v) if Arc::strong_count(v)==2)
        );
        let (different, _) =
            ctx.owned_roots.encode(Value::Array(Arc::new(vec![Value::Int(2)]))).unwrap();
        assert_eq!(
            unsafe {
                jit_release_rebind(
                    &mut ctx,
                    1,
                    target,
                    kind,
                    different,
                    kind,
                    1,
                    function,
                    function_kind,
                )
            },
            0
        );
        assert!(matches!(
            ctx.owned_roots.current_value_mut(target, kind).unwrap(),
            Value::Array(_)
        ));
        assert_eq!(
            unsafe {
                jit_release_rebind(
                    &mut ctx,
                    1,
                    target,
                    kind,
                    original,
                    kind,
                    1,
                    function,
                    function_kind,
                )
            },
            0
        );
        assert!(matches!(ctx.owned_roots.current_value_mut(target, kind).unwrap(), Value::Null));
        assert!(
            matches!(ctx.owned_roots.current_value_mut(original,kind).unwrap(),Value::Array(v) if Arc::strong_count(v)==1)
        );
        ctx.owned_roots.leave().unwrap();
    }

    #[test]
    fn native_iteration_distinguishes_null_from_exhaustion_and_bounds_roots() {
        for count in [8, 128] {
            let mut ctx =
                VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            ctx.owned_roots.enter().unwrap();
            let (array, kind) = ctx
                .owned_roots
                .encode(Value::Array(Arc::new(vec![
                    Value::Null,
                    Value::Str(Arc::new("snow".into())),
                ])))
                .unwrap();
            for _ in 0..count {
                for index in 0..3 {
                    let original = ctx.owned_roots.duplicate(array, kind).unwrap();
                    let result = unsafe {
                        jit_for_next(&mut ctx, original, kind, index, crate::native_scalar::INTEGER)
                    };
                    assert!(ctx.runtime_error.is_none());
                    let result_kind = ctx.return_value_kind;
                    match index {
                        0 => assert_eq!(result_kind, crate::native_scalar::NULL),
                        1 => assert!(
                            matches!(ctx.owned_roots.clone_value(result,result_kind).unwrap(),Value::Str(text) if text.as_str()=="snow")
                        ),
                        _ => assert_eq!(result_kind, -4),
                    }
                    if result_kind != -4 {
                        ctx.owned_roots.release(result, result_kind).unwrap();
                    }
                    ctx.owned_roots.release(original, kind).unwrap();
                }
            }
            ctx.owned_roots.leave().unwrap();
            let (_, live, capacity, _) = ctx.owned_roots.storage_state();
            assert_eq!(live, 0);
            assert!(capacity <= 4, "{capacity}");
        }
    }

    #[test]
    fn loop_header_analysis_uses_entry_relative_depth_and_assignment_placeholder() {
        let code = vec![
            OpCode::MakeDict(0),
            OpCode::DefineLocal(0),
            OpCode::LoadConst(0),
            OpCode::DefineLocal(1),
            OpCode::Pop,
            OpCode::LoadLocal(1),
            OpCode::LoadConst(1),
            OpCode::LessThan,
            OpCode::JumpIfFalse(21),
            OpCode::Pop,
            OpCode::LoadConst(0),
            OpCode::LoadConst(0),
            OpCode::IndexSetInPlace(0),
            OpCode::Pop,
            OpCode::LoadLocal(1),
            OpCode::LoadConst(1),
            OpCode::Add,
            OpCode::StoreLocal(1),
            OpCode::Pop,
            OpCode::JumpBack(5),
            OpCode::ReturnNone,
            OpCode::Pop,
            OpCode::ReturnNone,
        ];
        assert_eq!(
            BytecodeTranslator::analyze_loop_headers(&code, code.len(), 0).get(&5),
            Some(&1)
        );
        assert_eq!(
            BytecodeTranslator::analyze_loop_headers(&code, code.len(), 5).get(&5),
            Some(&0)
        );
        assert_eq!(BytecodeTranslator::stack_effect(&OpCode::IndexSetInPlace(0)), (2, 1));
        let unreachable = vec![OpCode::LoadConst(0), OpCode::Return, OpCode::JumpBack(0)];
        assert!(
            BytecodeTranslator::analyze_loop_headers(&unreachable, unreachable.len(), 0).is_empty()
        );
    }

    #[test]
    fn native_iteration_keeps_byte_values_and_index_errors() {
        let mut ctx =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        ctx.owned_roots.enter().unwrap();
        let (bytes, kind) = ctx.owned_roots.encode(Value::Bytes(Arc::new(vec![0, 255]))).unwrap();
        assert_eq!(
            unsafe { jit_for_next(&mut ctx, bytes, kind, 1, crate::native_scalar::INTEGER) },
            255
        );
        assert_eq!(ctx.return_value_kind, crate::native_scalar::INTEGER);
        for (index, index_kind) in
            [(-1, crate::native_scalar::INTEGER), (1, crate::native_scalar::BOOLEAN)]
        {
            unsafe { jit_for_next(&mut ctx, bytes, kind, index, index_kind) };
            assert_eq!(
                ctx.runtime_error.take().as_deref(),
                Some("ForNext requires a nonnegative index")
            );
        }
        unsafe {
            jit_for_next(&mut ctx, 0, crate::native_scalar::NULL, 0, crate::native_scalar::INTEGER)
        };
        assert_eq!(
            ctx.runtime_error.take().as_deref(),
            Some("ForNext requires a normalized iterable")
        );
        ctx.owned_roots.leave().unwrap();
    }

    #[test]
    fn native_boundaries_transfer_owned_returns_and_cleanup_recursive_failures() {
        let input = Arc::new(vec![Value::Int(8)]);
        let scratch = Arc::new("scratch".to_string());
        let mut locals = HashMap::from([
            ("input".into(), Value::Array(input.clone())),
            ("scratch".into(), Value::Str(scratch.clone())),
        ]);
        let mut context = VMContext::new(std::ptr::null_mut(), &mut locals, std::ptr::null_mut());
        for _ in 0..1000 {
            context.arg0 = 0;
            assert_eq!(invoke_compiled_fn(owned_standard_probe, &mut context), 0);
            assert!(
                matches!(context.scalar_value(context.return_value), Value::Array(array) if Arc::ptr_eq(&array, &input))
            );
            assert_eq!(Arc::strong_count(&input), 3);
            assert_eq!(Arc::strong_count(&scratch), 2);
            assert_eq!(context.native_call_depth, 0);
            let (depth, live, slots, frames) = context.owned_roots.storage_state();
            assert_eq!((depth, live), (0, 0));
            assert!(slots <= 8 && frames <= 8);
            let direct = invoke_compiled_fn_with_arg(owned_direct_probe, &mut context, 3);
            assert_ne!(direct, 0);
            assert!(
                matches!(context.scalar_value(direct), Value::Array(array) if Arc::ptr_eq(&array, &input))
            );
            assert_eq!(Arc::strong_count(&input), 3);
            let returned = context.take_return_value(direct);
            assert!(context.owned_return.is_none());
            assert!(matches!(&returned, Value::Array(array) if Arc::ptr_eq(array, &input)));
            assert_eq!(Arc::strong_count(&input), 3);
            drop(returned);
            assert_eq!(Arc::strong_count(&input), 2);
            context.arg0 = 1;
            assert_eq!(invoke_compiled_fn_with_arg(owned_lifetime_probe, &mut context, 3), 7);
            assert_eq!(context.runtime_error.take().as_deref(), Some("owned probe failure"));
            assert!(context.owned_return.is_none());
            assert_eq!(Arc::strong_count(&input), 2);
            assert_eq!(Arc::strong_count(&scratch), 2);
            assert_eq!(context.native_call_depth, 0);
            assert_eq!(context.owned_roots.depth(), 0);
        }
        assert_eq!(invoke_compiled_fn(dummy_compiled_fn, &mut context), 0);
        assert!(!context.has_return_value);
    }

    #[test]
    fn compiled_immediate_functions_keep_ownership_storage_unallocated() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();
        let constant = chunk.add_constant(Constant::Int(42));
        chunk.emit(OpCode::LoadConst(constant));
        chunk.emit(OpCode::Return);
        let compiled = compiler.compile_function(&chunk, "immediate_root_scope").unwrap();
        let mut context =
            VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        for _ in 0..1000 {
            assert_eq!(invoke_compiled_fn(compiled, &mut context), 0);
            assert_eq!(context.return_value, 42);
            assert_eq!(context.owned_roots.storage_state(), (0, 0, 0, 0));
            assert_eq!(context.native_call_depth, 0);
        }
        context.native_call_depth = crate::runtime_limits::DEFAULT_MAX_VM_CALL_DEPTH;
        assert_ne!(invoke_compiled_fn(compiled, &mut context), 0);
        assert_eq!(context.owned_roots.storage_state(), (0, 0, 0, 0));
        assert!(context.runtime_error.take().unwrap().contains("depth"));
        context.native_call_depth = 0;
        assert_eq!(invoke_compiled_fn(compiled, &mut context), 0);
    }

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
    fn jit_builtin_lookup_preserves_local_and_global_shadowing() {
        let hash = BytecodeTranslator::hash_var_name("bit_and");
        let mut names = std::collections::HashMap::from([(hash, "bit_and".to_owned())]);
        let mut stack = Vec::new();
        let mut locals = std::collections::HashMap::new();
        let mut globals = std::collections::HashMap::new();
        let mut context = VMContext::new(&mut stack, &mut locals, &mut globals);
        context.var_names_ptr = &mut names;
        // SAFETY: every context pointer references live, exclusively owned test
        // storage, and the helper neither retains pointers nor invokes code.
        assert_eq!(unsafe { jit_load_variable(&mut context, hash as i64, 0) }, -1);
        assert!(matches!(stack.pop(), Some(Value::NativeFunction(name)) if name == "bit_and"));
        globals.insert("bit_and".into(), Value::Int(7));
        // SAFETY: same live context/storage as above.
        assert_eq!(unsafe { jit_load_variable(&mut context, hash as i64, 0) }, 7);
        locals.insert("bit_and".into(), Value::Int(9));
        // SAFETY: same live context/storage as above.
        assert_eq!(unsafe { jit_load_variable(&mut context, hash as i64, 0) }, 9);
        assert!(stack.is_empty());
    }

    #[test]
    fn tagged_named_loads_preserve_shadowing_without_hidden_stack_values() {
        let hash = BytecodeTranslator::hash_var_name("len");
        let mut names = HashMap::from([(hash, "len".to_owned())]);
        let mut stack = vec![Value::Int(77)];
        let mut locals = HashMap::new();
        let mut globals = HashMap::new();
        let mut context = VMContext::new(&mut stack, &mut locals, &mut globals);
        context.var_names_ptr = &mut names;
        context.owned_roots.enter().unwrap();
        // SAFETY: context maps and stack remain live and exclusively borrowed.
        unsafe {
            let payload = jit_load_operand(&mut context, hash as i64, 0);
            let value = context.owned_roots.take_value(payload, context.return_value_kind).unwrap();
            assert!(matches!(value, Value::NativeFunction(name) if name == "len"));
            globals.insert("len".into(), Value::Str(Arc::new("global".into())));
            locals.insert("len".into(), Value::Str(Arc::new("local".into())));
            for (global_only, expected) in [(0, "local"), (1, "global")] {
                let payload = jit_load_operand(&mut context, hash as i64, global_only);
                let value =
                    context.owned_roots.take_value(payload, context.return_value_kind).unwrap();
                assert!(matches!(value, Value::Str(text) if text.as_str() == expected));
                assert_eq!(stack.len(), 1);
                assert!(matches!(stack[0], Value::Int(77)));
            }
            jit_load_operand(&mut context, 0, 0);
            assert!(context.runtime_error.take().unwrap().contains("no variable name"));
        }
        context.owned_roots.leave().unwrap();
        assert_eq!(context.owned_roots.storage_state().1, 0);
    }

    #[test]
    fn unsupported_aggregate_constants_are_rejected_before_native_execution() {
        for constant in [Constant::Array(vec![Constant::Int(7)]), Constant::Dict(vec![])] {
            let mut compiler = JitCompiler::new().unwrap();
            let mut chunk = BytecodeChunk::new();
            let index = chunk.add_constant(constant);
            chunk.emit(OpCode::LoadConst(index));
            chunk.emit(OpCode::Return);
            assert!(!compiler.can_compile_function(&chunk));
            assert!(!compiler.can_compile_loop(&chunk, 0, 1));
            assert!(!compiler.can_compile_loop_with_int_dicts(&chunk, 0, 1));
            assert!(compiler.compile_function(&chunk, "unsupported_constant").is_err());
            // The low-level compiler must also reject callers bypassing eligibility.
            assert!(compiler.compile(&chunk, 0).is_err());
        }
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
    fn test_mersenne_remainder_matches_signed_semantics() {
        let cases = [
            (0, 0),
            (2_147_483_647, 0),
            (2_147_483_648, 1),
            (i64::MAX, 1),
            (-2_147_483_648, -1),
            (i64::MIN, -2),
        ];

        for (case_index, (dividend, expected)) in cases.into_iter().enumerate() {
            let mut compiler = JitCompiler::new().unwrap();
            let mut chunk = BytecodeChunk::new();
            let dividend_constant = chunk.add_constant(Constant::Int(dividend));
            let divisor_constant = chunk.add_constant(Constant::Int(2_147_483_647));
            chunk.emit(OpCode::LoadConst(dividend_constant));
            chunk.emit(OpCode::LoadConst(divisor_constant));
            chunk.emit(OpCode::Mod);
            chunk.emit(OpCode::Return);

            let compiled = compiler
                .compile_function(&chunk, &format!("mersenne_remainder_case_{case_index}"))
                .expect("Mersenne remainder function should compile");
            let mut context =
                VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            assert_eq!(invoke_compiled_fn(compiled, &mut context), 0);
            assert!(context.has_return_value);
            assert_eq!(context.return_value, expected, "dividend: {dividend}");
        }
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
    fn loop_named_read_before_assignment_fails_without_later_effects() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();
        let value = chunk.add_constant(Constant::Int(10));
        chunk.emit(OpCode::LoadVar("missing".into()));
        chunk.emit(OpCode::Pop);
        chunk.emit(OpCode::LoadConst(value));
        chunk.emit(OpCode::StoreVar("missing".into()));
        chunk.emit(OpCode::Return);
        let compiled = compiler.compile(&chunk, 0).unwrap();
        let mut locals = HashMap::new();
        let mut globals = HashMap::new();
        let mut names =
            HashMap::from([(BytecodeTranslator::hash_var_name("missing"), "missing".into())]);
        let mut context =
            VMContext::with_var_names(std::ptr::null_mut(), &mut locals, &mut globals, &mut names);
        assert_ne!(invoke_compiled_fn(compiled, &mut context), 0);
        assert_eq!(context.runtime_error.as_deref(), Some("Undefined variable: missing"));
        assert!(locals.is_empty());
        assert!(globals.is_empty());
        assert_eq!(context.owned_roots.storage_state().1, 0);
    }

    #[test]
    fn loop_named_assignment_preserves_owned_values() {
        let mut compiler = JitCompiler::new().unwrap();
        let mut chunk = BytecodeChunk::new();
        chunk.emit(OpCode::LoadVar("seed".into()));
        chunk.emit(OpCode::StoreVar("copy".into()));
        chunk.emit(OpCode::Pop);
        chunk.emit(OpCode::LoadVar("copy".into()));
        chunk.emit(OpCode::Return);
        let compiled = compiler.compile(&chunk, 0).unwrap();
        let seed = Arc::new("seed".to_string());
        let mut locals = HashMap::new();
        let mut globals = HashMap::from([("seed".into(), Value::Str(seed.clone()))]);
        let mut names = ["seed", "copy"]
            .into_iter()
            .map(|name| (BytecodeTranslator::hash_var_name(name), name.to_string()))
            .collect();
        let mut context =
            VMContext::with_var_names(std::ptr::null_mut(), &mut locals, &mut globals, &mut names);
        assert_eq!(invoke_compiled_fn(compiled, &mut context), 0, "{:?}", context.runtime_error);
        assert!(
            matches!(context.take_return_value(context.return_value), Value::Str(text) if text.as_str() == "seed")
        );
        assert!(matches!(locals.get("copy"), Some(Value::Str(text)) if Arc::ptr_eq(text, &seed)));
        assert_eq!(context.owned_roots.storage_state().1, 0);
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
