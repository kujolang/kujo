// macOS diagnostic: query mappings without invoking or dereferencing stale code.
// Link against a recorded accepted release library. This is not an RSS benchmark.
#[cfg(not(target_os = "macos"))]
compile_error!("This mapping diagnostic currently supports macOS only");

use kujo::bytecode::{BytecodeChunk, Constant, OpCode};
use kujo::interpreter::Value;
use kujo::vm::VM;
use std::collections::HashMap;
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

// Sum executable virtual mappings, not RSS. Query traversal performs no code reads.
fn executable_bytes() -> u64 {
    let mut address = 0u64;
    let mut total = 0u64;
    loop {
        let mut size = 0u64;
        let mut info = [0i32; 32];
        let mut count = info.len() as u32;
        let mut object = 0u32;
        // SAFETY: current task, initialized output buffers, BASIC_INFO_64 ABI.
        let result = unsafe {
            mach_vm_region(
                mach_task_self_,
                &mut address,
                &mut size,
                9,
                info.as_mut_ptr(),
                &mut count,
                &mut object,
            )
        };
        if object != 0 {
            // SAFETY: release the returned send right only.
            assert_eq!(unsafe { mach_port_deallocate(mach_task_self_, object) }, 0);
        }
        if result == 1 {
            break;
        }
        assert_eq!(result, 0);
        assert!(size > 0);
        if info[0] & 4 != 0 {
            total = total.checked_add(size).unwrap();
        }
        address = address.checked_add(size).unwrap();
    }
    total
}
fn main() {
    let count: usize = std::env::args().nth(1).expect("VM count").parse().unwrap();
    let jit = match std::env::args().nth(2).as_deref() {
        Some("jit") => true,
        Some("vm") => false,
        _ => panic!("mode jit|vm"),
    };
    assert!((1..=64).contains(&count));
    // SAFETY: ABI calls as in the compiler probe; owned anonymous mapping only.
    let page_size = unsafe { getpagesize() } as usize;
    assert!(page_size.is_power_of_two());
    let control = unsafe { mmap(std::ptr::null_mut(), page_size, 3, 0x1002, -1, 0) };
    assert_ne!(control as isize, -1);
    assert!(mapped(control as usize, page_size));
    assert_eq!(unsafe { munmap(control, page_size) }, 0);
    assert!(!mapped(control as usize, page_size));
    let baseline = executable_bytes();
    let mut observations = Vec::new();
    for iteration in 0..count {
        let mut chunk = BytecodeChunk::new();
        chunk.name = Some("lifetime_probe".to_owned());
        let constant = chunk.add_constant(Constant::Int(iteration as i64));
        chunk.emit(OpCode::LoadConst(constant));
        chunk.emit(OpCode::Return);
        let function = Value::BytecodeFunction {
            chunk,
            captured: HashMap::new(),
            captured_binding_kinds: HashMap::new(),
        };
        let mut vm = VM::new();
        vm.set_jit_enabled(jit);
        assert_eq!(vm.jit_compile_bytecode_function(&function).unwrap(), jit);
        let live = executable_bytes();
        drop(vm);
        let after = executable_bytes();
        observations.push(format!("{{\"iteration\":{iteration},\"live_executable_bytes\":{live},\"after_drop_executable_bytes\":{after}}}"));
    }
    println!("{{\"vms\":{count},\"jit\":{jit},\"control_mapped\":true,\"control_unmapped\":true,\"baseline_executable_bytes\":{baseline},\"observations\":[{}]}}", observations.join(","));
}
