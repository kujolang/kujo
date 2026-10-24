#![cfg(feature = "runtime-jit")]

use kujo::interpreter::Value;
use kujo::jit::{
    jit_duplicate_operand, jit_enter_function, jit_indexed_read, jit_leave_function,
    jit_load_literal, jit_release_operand, jit_try_append, VMContext,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::{collections::HashMap, sync::Arc};

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

#[test]
fn preallocated_native_scalar_appends_do_not_allocate_per_element() {
    for count in [8usize, 128, 1024] {
        // SAFETY: the context exclusively borrows live stack/global storage.
        // Literal pointers stay live through encoding; subsequent operations
        // use validated root handles inside one paired native activation.
        unsafe {
            let mut stack = Vec::new();
            let mut globals = HashMap::new();
            let mut ctx = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
            assert_eq!(jit_enter_function(&mut ctx), 0);
            let initial = Value::Array(Arc::new(Vec::with_capacity(count + 1)));
            let target = jit_load_literal(&mut ctx, &initial as *const Value as i64);
            let kind = ctx.return_value_kind;
            drop(initial);
            let function = Value::NativeFunction("push".into());
            let callable = jit_load_literal(&mut ctx, &function as *const Value as i64);
            let callable_kind = ctx.return_value_kind;
            // Warm root-slot capacity and depth machinery before accounting.
            let original = jit_duplicate_operand(&mut ctx, target, kind);
            assert_eq!(
                jit_try_append(
                    &mut ctx,
                    1,
                    target,
                    kind,
                    original,
                    kind,
                    0,
                    0,
                    1,
                    callable,
                    callable_kind
                ),
                1
            );
            assert_eq!(jit_release_operand(&mut ctx, original, kind), 0);
            ALLOCATIONS.with(|v| v.set(0));
            REQUESTED_BYTES.with(|v| v.set(0));
            TRACK.with(|v| v.set(true));
            for n in 0..count {
                let original = jit_duplicate_operand(&mut ctx, target, kind);
                assert_eq!(
                    jit_try_append(
                        &mut ctx,
                        1,
                        target,
                        kind,
                        original,
                        kind,
                        n as i64,
                        0,
                        1,
                        callable,
                        callable_kind
                    ),
                    1
                );
                assert_eq!(jit_release_operand(&mut ctx, original, kind), 0);
            }
            TRACK.with(|v| v.set(false));
            assert!(ctx.runtime_error.is_none());
            for n in 0..count {
                assert_eq!(jit_indexed_read(&mut ctx, target, kind, (n + 1) as i64, 0), n as i64);
                assert_eq!(ctx.return_value_kind, 0);
            }
            let allocations = ALLOCATIONS.with(Cell::get);
            let requested_bytes = REQUESTED_BYTES.with(Cell::get);
            ctx.has_return_value = false;
            jit_leave_function(&mut ctx);
            assert!(ctx.runtime_error.is_none());
            assert_eq!(allocations, 0, "preallocated scalar appends allocated for count={count}");
            assert_eq!(requested_bytes, 0);
        }
    }
}
