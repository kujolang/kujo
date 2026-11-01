// macOS diagnostic: query mappings without invoking or dereferencing stale code.
// Link against a recorded accepted release library. This is not an RSS benchmark.
#[cfg(not(target_os = "macos"))]
compile_error!("This mapping diagnostic currently supports macOS only");

use kujo::bytecode::{BytecodeChunk, Constant, OpCode};
use kujo::jit::JitCompiler;
use std::collections::BTreeSet;
use std::ffi::{c_int, c_void};

extern "C" {
    fn getpagesize() -> c_int;
    static mach_task_self_: u32;
    fn mach_vm_region(
        task: u32,
        address: *mut u64,
        size: *mut u64,
        flavor: c_int,
        info: *mut c_int,
        count: *mut u32,
        object: *mut u32,
    ) -> c_int;
    fn mach_port_deallocate(task: u32, name: u32) -> c_int;
    fn mmap(
        address: *mut c_void,
        length: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: i64,
    ) -> *mut c_void;
    fn munmap(address: *mut c_void, length: usize) -> c_int;
}

fn mapped(address: usize, page_size: usize) -> bool {
    let page = (address / page_size * page_size) as u64;
    let mut base = page;
    let mut size = 0u64;
    let mut info = [0i32; 32];
    let mut count = info.len() as u32;
    let mut object = 0u32;
    // SAFETY: query this task's mappings; all output pointers refer to initialized
    // writable storage. Flavor 9 is VM_REGION_BASIC_INFO_64; the info buffer is
    // larger than its ABI payload. No code address is dereferenced.
    let result = unsafe {
        mach_vm_region(
            mach_task_self_,
            &mut base,
            &mut size,
            9,
            info.as_mut_ptr(),
            &mut count,
            &mut object,
        )
    };
    if object != 0 {
        // SAFETY: release only the send right returned by this region query.
        assert_eq!(unsafe { mach_port_deallocate(mach_task_self_, object) }, 0);
    }
    assert!(result == 0 || result == 1, "unexpected region query error {result}");
    // A successful query may return the NEXT region when this address is a hole.
    result == 0 && base <= page && page - base < size
}

fn main() {
    let count: usize = std::env::args().nth(1).expect("compiler count").parse().unwrap();
    assert!((1..=64).contains(&count));
    // SAFETY: getpagesize has no pointer arguments or preconditions.
    let page_size = unsafe { getpagesize() } as usize;
    assert!(page_size.is_power_of_two());
    // SAFETY: request one fresh anonymous private read/write page; no fixed
    // address or existing mapping is replaced. Constants are Darwin ABI values.
    let control = unsafe { mmap(std::ptr::null_mut(), page_size, 3, 0x1002, -1, 0) };
    assert_ne!(control as isize, -1, "mmap failed");
    assert!(mapped(control as usize, page_size));
    // SAFETY: this exact mapping is owned by the probe and has no references.
    assert_eq!(unsafe { munmap(control, page_size) }, 0);
    assert!(!mapped(control as usize, page_size));

    let mut pages = BTreeSet::new();
    let mut immediate_retained = 0;
    for iteration in 0..count {
        let mut chunk = BytecodeChunk::new();
        let constant = chunk.add_constant(Constant::Int(iteration as i64));
        chunk.emit(OpCode::LoadConst(constant));
        chunk.emit(OpCode::Return);
        let mut compiler = JitCompiler::new().unwrap();
        let compiled = compiler.compile(&chunk, 0).unwrap();
        let address = compiled as usize;
        assert!(mapped(address, page_size), "live compiled code must be mapped");
        drop(compiler);
        // No function call or dereference is made through the dropped owner.
        immediate_retained += usize::from(mapped(address, page_size));
        pages.insert(address / page_size * page_size);
    }
    let retained_pages = pages.iter().filter(|&&p| mapped(p, page_size)).count();
    println!(
        "{{\"compilers\":{count},\"page_size\":{page_size},\"control_mapped\":true,\"control_unmapped\":true,\"immediate_retained\":{immediate_retained},\"distinct_code_pages\":{},\"retained_code_pages\":{retained_pages},\"mapped_bytes_lower_bound\":{}}}",
        pages.len(), retained_pages * page_size
    );
}
