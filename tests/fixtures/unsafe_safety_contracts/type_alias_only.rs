pub type CompiledFn = unsafe extern "C" fn(*mut i64) -> i64;

pub fn noop() {}
