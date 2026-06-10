pub unsafe extern "C" fn jit_ffi(ptr: *mut i64) -> i64 {
    // SAFETY:
    // - Preconditions: ptr is valid.
    unsafe { *ptr }
}
