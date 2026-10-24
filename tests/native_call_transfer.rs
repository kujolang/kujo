#![cfg(feature = "runtime-jit")]
use kujo::interpreter::Value;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::HashMap;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static REQUESTED_BYTES: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;
fn record(bytes: usize) {
    let _ = TRACK.try_with(|enabled| {
        if enabled.get() {
            ALLOCATIONS.with(|count| count.set(count.get() + 1));
            REQUESTED_BYTES.with(|count| count.set(count.get() + bytes));
        }
    });
}

// SAFETY: all allocation operations delegate unchanged to System. Accounting
// uses allocation-free thread-local Cells and never dereferences allocation data.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        // SAFETY: caller supplies a valid allocation layout.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        // SAFETY: caller supplies a valid allocation layout.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: ownership and layout requirements are inherited from the caller.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        // SAFETY: pointer, original layout and new size are supplied by the caller.
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

use kujo::bytecode::{BytecodeBindingKind, BytecodeChunk, OpCode};

#[test]
fn native_call_transfer_has_bounded_allocations() {
    for arity in [1usize, 3, 4, 8, 32] {
        let mut chunk = BytecodeChunk::new();
        chunk.name = Some(format!("callback_{arity}"));
        for n in 0..arity {
            let name = format!("arg{n}");
            chunk.params.push(name.clone());
            chunk.local_names.push(name);
            chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
        }
        chunk.local_count = arity;
        let index = chunk.add_constant(kujo::bytecode::Constant::Int(0));
        chunk.emit(OpCode::LoadConst(index));
        chunk.emit(OpCode::IndexGetInPlace(0));
        chunk.emit(OpCode::Return);
        let function = Value::BytecodeFunction {
            chunk,
            captured: HashMap::new(),
            captured_binding_kinds: HashMap::new(),
        };
        let mut vm = kujo::vm::VM::new();
        vm.set_jit_enabled(true);
        assert!(vm.jit_compile_bytecode_function(&function).unwrap());
        let arguments = || {
            let mut a = vec![Value::Int(0); arity];
            a[0] = Value::array(vec![Value::Int(7); 8]);
            a
        };
        assert!(matches!(
            vm.call_function_from_jit(function.clone(), arguments()).unwrap(),
            Value::Int(7)
        ));
        // Measure the warmed callee separately; the bridge may add only its
        // exact-sized argument vector, regardless of the callee's own costs.
        let direct_function = function.clone();
        let direct_arguments = arguments();
        ALLOCATIONS.with(|v| v.set(0));
        REQUESTED_BYTES.with(|v| v.set(0));
        TRACK.with(|v| v.set(true));
        let direct_result = vm.call_function_from_jit(direct_function, direct_arguments);
        TRACK.with(|v| v.set(false));
        assert!(matches!(direct_result.unwrap(), Value::Int(7)));
        let direct_allocations = ALLOCATIONS.with(Cell::get);
        let direct_bytes = REQUESTED_BYTES.with(Cell::get);
        for calls in [1usize, 128] {
            let inputs: Vec<_> = (0..calls)
                .map(|_| {
                    let mut stack = Vec::with_capacity(arity + 1);
                    stack.push(function.clone());
                    stack.extend(arguments());
                    stack
                })
                .collect();

            ALLOCATIONS.with(|v| v.set(0));
            REQUESTED_BYTES.with(|v| v.set(0));
            TRACK.with(|v| v.set(true));
            for mut stack in inputs {
                let mut globals = HashMap::new();
                let mut ctx =
                    kujo::jit::VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
                ctx.vm_ptr = &mut vm as *mut _ as *mut std::ffi::c_void;
                // Caller-owned views remain live; callback does not own this stack.
                unsafe {
                    assert_eq!(
                        kujo::jit::jit_call_function(&mut ctx, std::ptr::null(), arity as i64),
                        0
                    );
                }
                assert!(ctx.runtime_error.is_none());
                assert_eq!(stack.len(), 1);
                assert!(matches!(stack[0], Value::Int(7)));
            }

            TRACK.with(|v| v.set(false));
            assert!(
                ALLOCATIONS.with(Cell::get) <= (direct_allocations + 1) * calls,
                "arity={arity} allocations={}",
                ALLOCATIONS.with(Cell::get)
            );
            assert!(
                REQUESTED_BYTES.with(Cell::get)
                    <= (direct_bytes + arity * std::mem::size_of::<Value>()) * calls,
                "arity={arity} requested bytes={}",
                REQUESTED_BYTES.with(Cell::get)
            );
            println!(
                "arity={arity} calls={calls} allocations={} bytes={}",
                ALLOCATIONS.with(Cell::get),
                REQUESTED_BYTES.with(Cell::get)
            );
        }
    }
}

use kujo::jit::{jit_call_function, VMContext};
#[test]
fn malformed_native_calls_preserve_stack_contracts() {
    unsafe {
        assert_eq!(jit_call_function(std::ptr::null_mut(), std::ptr::null(), 0), 1);
    }
    for (values, count, code, remaining) in [
        (vec![], 0, 3, vec![]),
        (vec![1], 2, 4, vec![]),
        (vec![1, 2], 2, 3, vec![2, 1]),
        (vec![9], -1, 5, vec![0]),
        (vec![9, 1, 2], 2, 5, vec![0]),
        (vec![9, 1], i64::MAX, 4, vec![]),
    ] {
        let mut stack: Vec<_> = values.into_iter().map(Value::Int).collect();
        let mut globals = HashMap::new();
        let mut ctx = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
        unsafe {
            assert_eq!(jit_call_function(&mut ctx, std::ptr::null(), count), code);
        }
        assert_eq!(stack.len(), remaining.len());
        for (actual, want) in stack.iter().zip(remaining) {
            assert!(matches!(actual,Value::Int(n) if *n==want));
        }
        println!("PASS count={count} code={code}");
    }
    let mut ctx = VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
    unsafe {
        assert_eq!(jit_call_function(&mut ctx, std::ptr::null(), 0), 2);
    }
}

#[test]
fn argument_order_snapshots_and_failures_match_both_runtimes() {
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let output = command.arg("tests/fixtures/native_call_transfer.kujo").output().unwrap();
        assert!(output.status.success(), "{mode:?}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
    }
}
