# Unsafe Inventory

Generated: 2026-10-09
Command: grep -RInE --include='*.rs' --exclude-dir=target --exclude='unsafe_inventory_contract.rs' '\<unsafe\>' src tests benches fuzz

## Summary

- Total matches: 124
- Executable matches: 99
- Non-executable matches: 25
- Unknown classifications: 0

## Rows

| Path | Line | Kind | Classification | Text |
| --- | ---: | --- | --- | --- |
| src/http_request_utils.rs | 38 | executable | src_executable_other |         unsafe { |
| src/interpreter/native_functions/compression.rs | 239 | non_executable | src_comment_or_string |         writer.write_all(b""unsafe"").unwrap(); |
| src/interpreter/native_functions/confined_write_windows.rs | 42 | executable | src_executable_other | unsafe extern ""system"" { |
| src/interpreter/native_functions/confined_write_windows.rs | 71 | executable | src_executable_other |         Err(io::Error::from_raw_os_error(unsafe { RtlNtStatusToDosError(status) } as i32)) |
| src/interpreter/native_functions/confined_write_windows.rs | 101 | executable | src_executable_other |     let result = unsafe { |
| src/interpreter/native_functions/confined_write_windows.rs | 117 | executable | src_executable_other |     Ok(unsafe { File::from_raw_handle(handle) }) |
| src/interpreter/native_functions/confined_write_windows.rs | 134 | executable | src_executable_other |     let result = unsafe { |
| src/interpreter/native_functions/confined_write_windows.rs | 153 | executable | src_executable_other |     status_result(unsafe { |
| src/interpreter/native_functions/crypto.rs | 2368 | non_executable | src_comment_or_string |         fs::write(&invalid_path, b""unsafe \r\n"").unwrap(); |
| src/interpreter/native_functions/filesystem.rs | 3006 | executable | src_executable_other |         assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0); |
| src/interpreter/native_functions/io.rs | 759 | non_executable | src_comment_or_string |                                     ""Refusing unsafe private spool directory '{}' (must be a directory without group/other write permission)"", |
| src/interpreter/native_functions/io.rs | 1571 | non_executable | src_comment_or_string |             matches!(refused, Value::Error(message) if message.contains(""unsafe private spool directory"")) |
| src/interpreter/native_functions/pdf.rs | 658 | non_executable | src_comment_or_string |                         ""unsupported or unsafe pdf HTML attribute '{key}' on '{name}'"" |
| src/interpreter/native_functions/pdf.rs | 1098 | non_executable | src_comment_or_string |             ""<p onclick=\""steal()\"">unsafe</p>"", |
| src/interpreter/native_functions/pdf.rs | 1099 | non_executable | src_comment_or_string |             ""<p style=\""background:url(https://attacker.test/x)\"">unsafe</p>"", |
| src/interpreter/native_functions/pdf.rs | 1102 | non_executable | src_comment_or_string |             let error = render_html(html).expect_err(""unsafe HTML must fail closed""); |
| src/interpreter/native_functions/pdf.rs | 1105 | non_executable | src_comment_or_string |                     \|\| error.contains(""unsafe"") |
| src/interpreter/native_functions/platform.rs | 54 | executable | src_executable_other |                     let uid = unsafe { libc::geteuid() }; |
| src/interpreter/native_functions/platform.rs | 73 | executable | src_executable_other |                     let uid = unsafe { libc::geteuid() }; |
| src/interpreter/native_functions/system.rs | 90 | executable | src_executable_other |         unsafe { |
| src/interpreter/native_functions/system.rs | 601 | executable | src_executable_other |     unsafe { |
| src/interpreter/native_functions/system.rs | 619 | executable | src_executable_other |         if unsafe { libc::kill(process_group, libc::SIGKILL) } == 0 { |
| src/interpreter/native_functions/system.rs | 651 | executable | src_executable_other |     let handle = unsafe { |
| src/interpreter/native_functions/system.rs | 664 | executable | src_executable_other |     Ok(unsafe { File::from_raw_handle(handle) }) |
| src/interpreter/native_functions/tls.rs | 691 | non_executable | src_comment_or_string |             &[plain, string_value(""unsafe"")], |
| src/interpreter/native_functions/web_data.rs | 372 | executable | src_executable_other |         let result = unsafe { libc::renamex_np(src.as_ptr(), dst.as_ptr(), libc::RENAME_EXCL) }; |
| src/interpreter/native_functions/web_data.rs | 374 | executable | src_executable_other |         let result = unsafe { |
| src/interpreter/native_functions/web_data.rs | 405 | executable | src_executable_other |         let result = unsafe { |
| src/interpreter/native_functions/web_data.rs | 1268 | executable | src_executable_other |         let mut usage: libc::rusage = unsafe { std::mem::zeroed() }; |
| src/interpreter/native_functions/web_data.rs | 1269 | executable | src_executable_other |         if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } != 0 { |
| src/jit.rs | 62 | executable | jit_executable |     let vm_ctx = unsafe { &mut *ctx }; |
| src/jit.rs | 79 | executable | jit_executable |         let stack = unsafe { &mut *vm_ctx.stack_ptr }; |
| src/jit.rs | 90 | executable | jit_executable |     unsafe { std::mem::transmute(code_ptr) } |
| src/jit.rs | 98 | executable | jit_executable |     unsafe { std::mem::transmute(code_ptr) } |
| src/jit.rs | 191 | executable | jit_executable |             let vm = unsafe { &mut *(ctx.vm_ptr as *mut crate::vm::VM) }; |
| src/jit.rs | 227 | executable | jit_executable | pub type CompiledFn = unsafe extern ""C"" fn(*mut VMContext) -> i64; |
| src/jit.rs | 232 | executable | jit_executable | pub type CompiledFnWithArg = unsafe extern ""C"" fn(*mut VMContext, i64) -> i64; |
| src/jit.rs | 234 | non_executable | jit_comment_or_doc | /// Invoke a compiled JIT function through one audited unsafe boundary. |
| src/jit.rs | 247 | executable | jit_executable |     unsafe { compiled_fn(ctx as *mut VMContext) } |
| src/jit.rs | 250 | non_executable | jit_comment_or_doc | /// Invoke a single-argument compiled JIT function through one audited unsafe boundary. |
| src/jit.rs | 266 | executable | jit_executable |     unsafe { compiled_fn(ctx as *mut VMContext, arg) } |
| src/jit.rs | 423 | executable | jit_executable | pub unsafe extern ""C"" fn jit_stack_push(ctx: *mut VMContext, value: i64) { |
| src/jit.rs | 442 | executable | jit_executable | pub unsafe extern ""C"" fn jit_stack_pop(ctx: *mut VMContext) -> i64 { |
| src/jit.rs | 465 | executable | jit_executable | pub unsafe extern ""C"" fn jit_obj_push_string(ctx: *mut VMContext, ptr: i64, len: i64) -> i64 { |
| src/jit.rs | 493 | executable | jit_executable | pub unsafe extern ""C"" fn jit_obj_to_vm_stack(ctx: *mut VMContext, handle: i64) -> i64 { |
| src/jit.rs | 526 | executable | jit_executable | pub unsafe extern ""C"" fn jit_load_variable( |
| src/jit.rs | 625 | executable | jit_executable | pub unsafe extern ""C"" fn jit_load_local_slot(ctx: *mut VMContext, slot: i64) -> i64 { |
| src/jit.rs | 647 | executable | jit_executable | pub unsafe extern ""C"" fn jit_store_local_slot(ctx: *mut VMContext, slot: i64, value: i64) -> i64 { |
| src/jit.rs | 680 | executable | jit_executable | pub unsafe extern ""C"" fn jit_store_variable( |
| src/jit.rs | 722 | executable | jit_executable | pub unsafe extern ""C"" fn jit_store_variable_from_stack(ctx: *mut VMContext, name_hash: i64) -> i64 { |
| src/jit.rs | 774 | executable | jit_executable | pub unsafe extern ""C"" fn jit_append_const_string_in_place( |
| src/jit.rs | 831 | executable | jit_executable | pub unsafe extern ""C"" fn jit_append_const_char_in_place( |
| src/jit.rs | 891 | executable | jit_executable | pub unsafe extern ""C"" fn jit_local_slot_dict_get( |
| src/jit.rs | 1054 | executable | jit_executable | pub unsafe extern ""C"" fn jit_local_slot_dict_set( |
| src/jit.rs | 1329 | executable | jit_executable | pub unsafe extern ""C"" fn jit_local_slot_int_dict_get( |
| src/jit.rs | 1412 | executable | jit_executable | pub unsafe extern ""C"" fn jit_local_slot_int_dict_set( |
| src/jit.rs | 1560 | executable | jit_executable | pub unsafe extern ""C"" fn jit_int_dict_unique_ptr(ctx: *mut VMContext, slot_index: i64) -> i64 { |
| src/jit.rs | 1641 | executable | jit_executable | pub unsafe extern ""C"" fn jit_int_dict_get_ptr(dict_ptr: i64, key: i64) -> i64 { |
| src/jit.rs | 1700 | executable | jit_executable | pub unsafe extern ""C"" fn jit_dense_int_dict_int_get_ptr(dict_ptr: i64, key: i64) -> i64 { |
| src/jit.rs | 1730 | executable | jit_executable | pub unsafe extern ""C"" fn jit_dense_int_dict_int_full_get_ptr(dict_ptr: i64, key: i64) -> i64 { |
| src/jit.rs | 1757 | executable | jit_executable | pub unsafe extern ""C"" fn jit_int_dict_set_ptr(dict_ptr: i64, key: i64, value: i64) -> i64 { |
| src/jit.rs | 1828 | executable | jit_executable | pub unsafe extern ""C"" fn jit_dense_int_dict_int_set_ptr( |
| src/jit.rs | 1864 | executable | jit_executable | pub unsafe extern ""C"" fn jit_dense_int_dict_int_full_set_ptr( |
| src/jit.rs | 1899 | executable | jit_executable | pub unsafe extern ""C"" fn jit_load_variable_float(ctx: *mut VMContext, name_hash: i64) -> f64 { |
| src/jit.rs | 1945 | executable | jit_executable | pub unsafe extern ""C"" fn jit_store_variable_float(ctx: *mut VMContext, name_hash: i64, value: f64) { |
| src/jit.rs | 1980 | executable | jit_executable | pub unsafe extern ""C"" fn jit_check_type_int(ctx: *mut VMContext, name_hash: i64) -> i64 { |
| src/jit.rs | 2026 | executable | jit_executable | pub unsafe extern ""C"" fn jit_check_type_float(ctx: *mut VMContext, name_hash: i64) -> i64 { |
| src/jit.rs | 2072 | executable | jit_executable | pub unsafe extern ""C"" fn jit_push_int(ctx: *mut VMContext, value: i64) -> i64 { |
| src/jit.rs | 2103 | executable | jit_executable | pub unsafe extern ""C"" fn jit_set_return_int(ctx: *mut VMContext, value: i64) -> i64 { |
| src/jit.rs | 2122 | executable | jit_executable |     unsafe { jit_set_return_int(ctx as *mut VMContext, value) } |
| src/jit.rs | 2135 | executable | jit_executable | pub unsafe extern ""C"" fn jit_get_return_int(ctx: *mut VMContext) -> i64 { |
| src/jit.rs | 2159 | executable | jit_executable | pub unsafe extern ""C"" fn jit_get_arg(ctx: *mut VMContext, index: i64) -> i64 { |
| src/jit.rs | 2193 | executable | jit_executable | pub unsafe extern ""C"" fn jit_call_function( |
| src/jit.rs | 2287 | executable | jit_executable | pub unsafe extern ""C"" fn jit_dict_get(ctx: *mut VMContext) -> i64 { |
| src/jit.rs | 2418 | executable | jit_executable | pub unsafe extern ""C"" fn jit_dict_set(ctx: *mut VMContext) -> i64 { |
| src/jit.rs | 2925 | executable | jit_executable |         let keys = unsafe { &*(keys_ptr as *const Arc<Vec<Arc<str>>>) }; |
| src/jit.rs | 8412 | executable | jit_executable |     unsafe extern ""C"" fn dummy_compiled_fn(_ctx: *mut VMContext) -> i64 { |
| src/jit.rs | 8422 | executable | jit_executable |     unsafe extern ""C"" fn dummy_compiled_fn_with_arg(_ctx: *mut VMContext, arg: i64) -> i64 { |
| src/jit.rs | 9518 | executable | jit_executable |         unsafe { |
| src/module.rs | 903 | non_executable | src_comment_or_string |             ""expected unsafe traversal error, got: {}"", |
| src/process_lifetime.rs | 29 | executable | src_executable_other |     let job = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) }; |
| src/process_lifetime.rs | 37 | executable | src_executable_other |     let configured = unsafe { |
| src/process_lifetime.rs | 47 | executable | src_executable_other |     if configured == 0 \|\| unsafe { AssignProcessToJobObject(job, GetCurrentProcess()) } == 0 { |
| src/process_lifetime.rs | 51 | executable | src_executable_other |         unsafe { CloseHandle(job) }; |
| src/process_lifetime.rs | 94 | executable | src_executable_other |             let raw = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) }; |
| src/process_lifetime.rs | 99 | executable | src_executable_other |             let job = Self(unsafe { OwnedHandle::from_raw_handle(raw) }); |
| src/process_lifetime.rs | 103 | executable | src_executable_other |             if unsafe { |
| src/process_lifetime.rs | 122 | executable | src_executable_other |                 unsafe { AssignProcessToJobObject(job.0.as_raw_handle(), child.as_raw_handle()) }; |
| src/process_lifetime.rs | 140 | executable | src_executable_other |             if unsafe { TerminateJobObject(self.0.as_raw_handle(), 1) } == 0 { |
| src/process_lifetime.rs | 150 | executable | src_executable_other |         let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }; |
| src/process_lifetime.rs | 155 | executable | src_executable_other |         let snapshot = unsafe { OwnedHandle::from_raw_handle(raw) }; |
| src/process_lifetime.rs | 159 | executable | src_executable_other |         let mut found = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) }; |
| src/process_lifetime.rs | 163 | executable | src_executable_other |                 let raw = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) }; |
| src/process_lifetime.rs | 168 | executable | src_executable_other |                 let thread = unsafe { OwnedHandle::from_raw_handle(raw) }; |
| src/process_lifetime.rs | 171 | executable | src_executable_other |                 if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX { |
| src/process_lifetime.rs | 178 | executable | src_executable_other |             found = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) }; |
| src/upgrade.rs | 266 | non_executable | src_comment_or_string |             return Err(""unsafe ZIP entry"".into()); |
| src/upgrade.rs | 285 | non_executable | src_comment_or_string |                 return Err(""unsafe, duplicate, or oversized TAR entry"".into()); |
| src/upgrade.rs | 501 | non_executable | src_comment_or_string |         return Err(""unsafe upgrade lock path"".into()); |
| src/upgrade/tests.rs | 317 | executable | src_executable_other |     if unsafe { libc::geteuid() } != 0 { |
| src/upgrade/tests.rs | 499 | non_executable | src_comment_or_string |     archive.extend(tar_bytes(&[(""../unsafe"", b""evil"", b'0')])); |
| tests/benchmark_publication_policy_contract.rs | 58 | non_executable | test_comment_or_string |             ""retired launch-unsafe artifact should stay removed: {path}"" |
| tests/fixtures/unsafe_safety_contracts/malformed_contract.rs | 1 | executable | test_executable | pub unsafe extern ""C"" fn jit_ffi(ptr: *mut i64) -> i64 { |
| tests/fixtures/unsafe_safety_contracts/malformed_contract.rs | 4 | executable | test_executable |     unsafe { *ptr } |
| tests/fixtures/unsafe_safety_contracts/missing_contract.rs | 1 | executable | test_executable | pub unsafe extern ""C"" fn jit_ffi(ptr: *mut i64) -> i64 { |
| tests/fixtures/unsafe_safety_contracts/missing_contract.rs | 2 | executable | test_executable |     unsafe { *ptr } |
| tests/fixtures/unsafe_safety_contracts/type_alias_only.rs | 1 | executable | test_executable | pub type CompiledFn = unsafe extern ""C"" fn(*mut i64) -> i64; |
| tests/fixtures/unsafe_safety_contracts/valid_jit_like.rs | 4 | executable | test_executable | pub unsafe extern ""C"" fn jit_ffi(ptr: *mut i64) -> i64 { |
| tests/fixtures/unsafe_safety_contracts/valid_jit_like.rs | 8 | executable | test_executable |     unsafe { *ptr } |
| tests/fixtures/unsafe_safety_contracts/valid_jit_like.rs | 15 | executable | test_executable |     unsafe { *raw } |
| tests/fixtures/unsafe_safety_contracts/wrong_headings.rs | 1 | executable | test_executable | pub unsafe extern ""C"" fn jit_ffi(ptr: *mut i64) -> i64 { |
| tests/fixtures/unsafe_safety_contracts/wrong_headings.rs | 5 | executable | test_executable |     unsafe { *ptr } |
| tests/generated_artifact_freshness_contract.rs | 150 | non_executable | test_comment_or_string |     let output_md = temp_dir.join(""unsafe.md""); |
| tests/generated_artifact_freshness_contract.rs | 151 | non_executable | test_comment_or_string |     let output_csv = temp_dir.join(""unsafe.csv""); |
| tests/http_route_concurrency.rs | 204 | executable | test_executable |         let signal_result = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) }; |
| tests/jit_safety_contract_checker.rs | 152 | non_executable | test_comment_or_string |     assert!(stdout.contains(""Checked 0 executable unsafe boundaries"")); |
| tests/process_lifetime_contracts.rs | 125 | executable | test_executable |                     unsafe { WaitForSingleObject(grandchild.0, 5000) }, |
| tests/process_lifetime_contracts.rs | 145 | executable | test_executable |             unsafe { |
| tests/process_lifetime_contracts.rs | 159 | executable | test_executable |                         unsafe { OpenProcess(PROCESS_SYNCHRONIZE \| PROCESS_TERMINATE, 0, pid) }; |
| tests/process_lifetime_contracts.rs | 215 | executable | test_executable |                     unsafe { WaitForSingleObject(process.0, 5000) }, |
| tests/runtime_security.rs | 323 | non_executable | test_comment_or_string |         ""expected unsafe traversal error, got: {}"", |
| tests/unsafe_safety_gate_contract.rs | 14 | non_executable | test_comment_or_string |         .expect(""failed to run unsafe safety gate help""); |
| tests/unsafe_safety_gate_contract.rs | 29 | non_executable | test_comment_or_string |         .expect(""failed to run unsafe safety gate dry-run""); |
| tests/unsafe_safety_gate_contract.rs | 56 | non_executable | test_comment_or_string |         .expect(""failed to run unsafe safety gate unknown-arg check""); |
