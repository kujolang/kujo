// SAFETY:
// - Preconditions: ABI caller passes a valid `ptr` for one read during this call.
// - Postconditions: function does not take ownership and returns the pointed value.
pub unsafe extern "C" fn jit_ffi(ptr: *mut i64) -> i64 {
    // SAFETY:
    // - Preconditions: `ptr` is non-null, uniquely borrowed for this call, and points to initialized memory.
    // - Postconditions: reads one i64 value without taking ownership or storing aliases.
    unsafe { *ptr }
}

pub fn wrapper(raw: *mut i64) -> i64 {
    // SAFETY:
    // - Preconditions: `raw` comes from trusted VM context and remains valid for this read.
    // - Postconditions: no mutation; returned value mirrors pointed memory at call time.
    unsafe { *raw }
}
