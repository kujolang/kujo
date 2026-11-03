// One linked library identity for three independent process-level diagnostics.
// Arguments: COUNT jit|vm live|vm-maps|compiler-maps. macOS mapping probes only.
mod live {
    include!("vm_live_allocations.rs");
    pub fn run() {
        main();
    }
}
mod vm_maps {
    include!("vm_code_lifetime.rs");
    pub fn run() {
        main();
    }
}
mod compiler_maps {
    include!("jit_code_lifetime.rs");
    pub fn run() {
        main();
    }
}

fn main() {
    match std::env::args().nth(3).as_deref() {
        Some("live") => live::run(),
        Some("vm-maps") => vm_maps::run(),
        Some("compiler-maps") => compiler_maps::run(),
        _ => panic!("usage: COUNT jit|vm live|vm-maps|compiler-maps"),
    }
}
