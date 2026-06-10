pub unsafe extern "C" fn jit_ffi(ptr: *mut i64) -> i64 {
    unsafe { *ptr }
}
