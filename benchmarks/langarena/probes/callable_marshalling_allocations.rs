// Isolate borrowed native operand marshalling. Link like quicksort_allocations.rs.
// This diagnostic preserves the borrowing contract; it does not change runtime code.
use kujo::interpreter::Value;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static LARGE: Cell<usize> = const { Cell::new(0) };
    static LIMIT: Cell<usize> = const { Cell::new(usize::MAX) };
    static REQUESTED_BYTES: Cell<usize> = const { Cell::new(0) };
    // Fixed TLS accounting does not allocate inside the allocator callback.
    // Bucket 2048 combines requests >= 2048 bytes; other buckets are exact sizes.
    static SIZES: [Cell<usize>; 2049] = const { [const { Cell::new(0) }; 2049] };
}

struct CountingAllocator;
fn record(bytes: usize) {
    let _ = TRACK.try_with(|enabled| {
        if enabled.get() {
            if bytes >= LIMIT.with(Cell::get) {
                LARGE.with(|n| n.set(n.get() + 1));
            }
            ALLOCATIONS.with(|count| count.set(count.get() + 1));
            REQUESTED_BYTES.with(|count| count.set(count.get() + bytes));
            SIZES.with(|sizes| {
                let count = &sizes[bytes.min(2048)];
                count.set(count.get() + 1);
            });
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

use kujo::jit::{
    jit_enter_function, jit_leave_function, jit_release_operand, jit_stack_pop,
    jit_stack_push_scalar, jit_stack_take_scalar, VMContext,
};

fn main() {
    for consuming in [false, true] {
        for calls in [1, 128, 8192] {
            let mut stack = Vec::with_capacity(2);
            let mut context =
                VMContext::new(&mut stack, std::ptr::null_mut(), std::ptr::null_mut());
            // SAFETY: independent live stack and context; calls are sequential.
            assert_eq!(unsafe { jit_enter_function(&mut context) }, 0);
            ALLOCATIONS.with(|v| v.set(0));
            REQUESTED_BYTES.with(|v| v.set(0));
            for _ in 0..calls {
                stack.push(Value::NativeFunction("to_int".into()));
                let operand = unsafe { jit_stack_pop(&mut context) };
                let kind = context.return_value_kind;
                assert!(context.runtime_error.is_none());
                TRACK.with(|v| v.set(true));
                if consuming {
                    assert_eq!(unsafe { jit_stack_take_scalar(&mut context, operand, kind) }, 0);
                } else {
                    unsafe { jit_stack_push_scalar(&mut context, operand, kind) };
                }
                TRACK.with(|v| v.set(false));
                assert!(
                    matches!(stack.pop(), Some(Value::NativeFunction(name)) if name == "to_int")
                );
                if !consuming {
                    // The borrowed API must remain reusable; the consuming API is separate.
                    unsafe { jit_stack_push_scalar(&mut context, operand, kind) };
                    assert!(
                        matches!(stack.pop(), Some(Value::NativeFunction(name)) if name == "to_int")
                    );
                    assert_eq!(unsafe { jit_release_operand(&mut context, operand, kind) }, 0);
                }
                assert!(context.runtime_error.is_none());
            }
            println!(
                "consuming={consuming} calls={calls} allocations={} requested_bytes={}",
                ALLOCATIONS.with(Cell::get),
                REQUESTED_BYTES.with(Cell::get)
            );
            if consuming {
                assert_eq!(ALLOCATIONS.with(Cell::get), 0);
                assert_eq!(REQUESTED_BYTES.with(Cell::get), 0);
            }
            unsafe { jit_leave_function(&mut context) };
            assert!(context.runtime_error.is_none());
            assert!(stack.is_empty());
        }
    }
}
