use crate::bytecode::{BytecodeChunk, OpCode};
use crate::interpreter::Value;
use std::collections::{HashMap, HashSet};

#[repr(C)]
pub struct VMContext {
    pub stack_ptr: *mut Vec<Value>,
    pub locals_ptr: *mut HashMap<String, Value>,
    pub globals_ptr: *mut HashMap<String, Value>,
    pub var_names_ptr: *mut HashMap<u64, String>,
    pub local_slots_ptr: *mut Vec<Value>,
    pub obj_stack_ptr: *mut Vec<Value>,
    pub vm_ptr: *mut std::ffi::c_void,
    pub return_value: i64,
    pub has_return_value: bool,
    pub arg0: i64,
    pub arg1: i64,
    pub arg2: i64,
    pub arg3: i64,
    pub arg_count: i64,
}

impl VMContext {
    pub fn new(
        stack: *mut Vec<Value>,
        locals: *mut HashMap<String, Value>,
        globals: *mut HashMap<String, Value>,
    ) -> Self {
        Self {
            stack_ptr: stack,
            locals_ptr: locals,
            globals_ptr: globals,
            var_names_ptr: std::ptr::null_mut(),
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

    pub fn new_with_vm(
        stack: *mut Vec<Value>,
        locals: *mut HashMap<String, Value>,
        globals: *mut HashMap<String, Value>,
        vm: *mut std::ffi::c_void,
    ) -> Self {
        let mut ctx = Self::new(stack, locals, globals);
        ctx.vm_ptr = vm;
        ctx
    }

    pub fn with_var_names(
        stack: *mut Vec<Value>,
        locals: *mut HashMap<String, Value>,
        globals: *mut HashMap<String, Value>,
        var_names: *mut HashMap<u64, String>,
    ) -> Self {
        let mut ctx = Self::new(stack, locals, globals);
        ctx.var_names_ptr = var_names;
        ctx
    }
}

pub type CompiledFn = extern "C" fn(*mut VMContext) -> i64;
pub type CompiledFnWithArg = extern "C" fn(*mut VMContext, i64) -> i64;

#[inline]
pub fn invoke_compiled_fn(compiled_fn: CompiledFn, ctx: &mut VMContext) -> i64 {
    compiled_fn(ctx as *mut VMContext)
}

#[inline]
pub fn invoke_compiled_fn_with_arg(
    compiled_fn: CompiledFnWithArg,
    ctx: &mut VMContext,
    arg: i64,
) -> i64 {
    compiled_fn(ctx as *mut VMContext, arg)
}

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

#[derive(Clone, Copy)]
pub struct CompiledFnInfo {
    pub fn_ptr: CompiledFn,
    pub fn_with_arg: Option<CompiledFnWithArg>,
    pub param_count: usize,
    pub supports_direct_recursion: bool,
}

#[derive(Debug, Clone)]
pub struct JitStats {
    pub total_functions: usize,
    pub compiled_functions: usize,
    pub enabled: bool,
}

pub struct JitCompiler {
    enabled: bool,
    execution_counts: HashMap<usize, usize>,
    loop_jit_blacklist: HashSet<usize>,
}

impl JitCompiler {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            enabled: false,
            execution_counts: HashMap::new(),
            loop_jit_blacklist: HashSet::new(),
        })
    }

    fn disabled_error() -> String {
        "JIT support is disabled in this build (enable the 'runtime-jit' feature)".to_string()
    }

    pub fn should_compile(&mut self, offset: usize) -> bool {
        let count = self.execution_counts.entry(offset).or_insert(0);
        *count += 1;
        false
    }

    pub fn is_loop_jit_blocked(&self, offset: usize) -> bool {
        self.loop_jit_blacklist.contains(&offset)
    }

    pub fn mark_loop_jit_blocked(&mut self, offset: usize) {
        self.loop_jit_blacklist.insert(offset);
    }

    pub fn can_compile_loop(&self, _chunk: &BytecodeChunk, _start: usize, _end: usize) -> bool {
        false
    }

    pub fn can_compile_loop_with_int_dicts(
        &self,
        _chunk: &BytecodeChunk,
        _start: usize,
        _end: usize,
    ) -> bool {
        false
    }

    pub fn compile(
        &mut self,
        _chunk: &BytecodeChunk,
        _offset: usize,
    ) -> Result<CompiledFn, String> {
        Err(Self::disabled_error())
    }

    pub fn compile_loop_with_int_dicts(
        &mut self,
        _chunk: &BytecodeChunk,
        _offset: usize,
        _end: usize,
        _int_dict_slots: std::collections::HashSet<usize>,
    ) -> Result<CompiledFn, String> {
        Err(Self::disabled_error())
    }

    pub fn compile_function_with_info(
        &mut self,
        _chunk: &BytecodeChunk,
        _function_name: &str,
    ) -> Result<CompiledFnInfo, String> {
        Err(Self::disabled_error())
    }

    pub fn compile_script(
        &mut self,
        _chunk: &BytecodeChunk,
        _script_name: &str,
    ) -> Result<CompiledFn, String> {
        Err(Self::disabled_error())
    }

    pub fn first_unsupported_surface(
        &self,
        _chunk: &BytecodeChunk,
    ) -> Option<UnsupportedJitSurface> {
        None
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn stats(&self) -> JitStats {
        JitStats {
            total_functions: self.execution_counts.len(),
            compiled_functions: 0,
            enabled: self.enabled,
        }
    }
}

impl Default for JitCompiler {
    fn default() -> Self {
        Self::new().expect("JIT disabled compiler should initialize")
    }
}
