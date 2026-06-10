pub unsafe extern "C" fn jit_ffi(ptr: *mut i64) -> i64 {
    // SAFETY:
    // - Precondition: ptr is valid.
    // - Postcondition: one value is read.
    unsafe { *ptr }
}
