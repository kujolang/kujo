// Compile-only ownership diagnostic, linked to a frozen Kujo library.
// Default checks movement; --cfg require_sync checks shared access separately.
#[cfg(not(any(require_sync, require_pool)))]
fn main() {
    fn require<T: Send>() {}
    require::<kujo::jit::JitCompiler>();
}

#[cfg(require_pool)]
fn main() {
    fn require<T: Send + Sync>() {}
    require::<std::sync::Arc<std::sync::Mutex<kujo::jit::JitCompiler>>>();
}

#[cfg(require_sync)]
fn main() {
    fn require<T: Sync>() {}
    require::<kujo::jit::JitCompiler>();
}
