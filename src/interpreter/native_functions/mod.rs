// File: src/interpreter/native_functions/mod.rs
//
// Module organization for native (built-in) function implementations.
// This module is part of Phase 3 modularization to split the massive
// call_native_function_impl into manageable category-based modules.

pub mod async_ops;
pub mod collections;
pub mod concurrency;
pub mod crypto;
#[cfg(feature = "runtime-db")]
pub mod database;
pub mod dns;
#[cfg(not(feature = "runtime-db"))]
pub mod database {
    use super::super::Value;

    pub fn handle(name: &str, _arg_values: &[Value]) -> Option<Value> {
        match name {
            "db_connect" | "db_execute" | "db_query" | "db_close" | "db_pool"
            | "db_pool_acquire" | "db_pool_release" | "db_pool_stats" | "db_pool_close"
            | "db_begin" | "db_commit" | "db_rollback" | "db_last_insert_id" => Some(Value::Error(
                "Database native APIs are disabled in this build (enable the 'runtime-db' feature)"
                    .to_string(),
            )),
            _ => None,
        }
    }
}
pub mod filesystem;
pub mod http;
pub mod io;
pub mod json;
pub mod math;
pub mod network;
pub mod schema;
pub mod strings;
pub mod system;
pub mod tls;
pub mod token;
pub mod type_ops;
pub mod vector;

use super::{Interpreter, Value};

/// Main dispatcher that routes native function calls to appropriate category modules
pub fn call_native_function(interp: &mut Interpreter, name: &str, arg_values: &[Value]) -> Value {
    // Legacy aliases and compatibility helpers.
    match name {
        "__vm_for_iterable" => {
            if arg_values.len() != 1 {
                return Value::Error(format!(
                    "__vm_for_iterable expects 1 argument, got {}",
                    arg_values.len()
                ));
            }
            return match &arg_values[0] {
                Value::Int(n) => {
                    let upper = (*n).max(0);
                    let values: Vec<Value> = (0..upper).map(Value::Int).collect();
                    Value::Array(std::sync::Arc::new(values))
                }
                Value::Float(n) => {
                    let upper = (*n as i64).max(0);
                    let values: Vec<Value> = (0..upper).map(Value::Int).collect();
                    Value::Array(std::sync::Arc::new(values))
                }
                Value::Array(arr) => Value::Array(arr.clone()),
                Value::Dict(dict) => {
                    let keys: Vec<Value> = dict
                        .keys()
                        .map(|key| Value::Str(std::sync::Arc::new(key.to_string())))
                        .collect();
                    Value::Array(std::sync::Arc::new(keys))
                }
                Value::FixedDict { keys, .. } => {
                    let key_values: Vec<Value> = keys
                        .iter()
                        .map(|key| Value::Str(std::sync::Arc::new(key.to_string())))
                        .collect();
                    Value::Array(std::sync::Arc::new(key_values))
                }
                Value::Str(text) => {
                    let chars: Vec<Value> = text
                        .chars()
                        .map(|ch| Value::Str(std::sync::Arc::new(ch.to_string())))
                        .collect();
                    Value::Array(std::sync::Arc::new(chars))
                }
                other => other.clone(),
            };
        }
        "dict" => {
            if let Some(arity) = Interpreter::native_callable_arity("dict") {
                if let Err(message) = arity.validate(arg_values.len()) {
                    return Value::Error(message);
                }
            }
            return Value::dict(Default::default());
        }
        "array" => {
            return Value::Array(std::sync::Arc::new(arg_values.to_vec()));
        }
        "error" => {
            if let Some(arity) = Interpreter::native_callable_arity("error") {
                if let Err(message) = arity.validate(arg_values.len()) {
                    return Value::Error(message);
                }
            }

            let message = match &arg_values[0] {
                Value::Str(text) => text.as_ref().clone(),
                other => format!("{:?}", other),
            };

            return Value::ErrorObject { message, stack: Vec::new(), line: None, cause: None };
        }
        "collect" => {
            if let Some(arity) = Interpreter::native_callable_arity("collect") {
                if let Err(message) = arity.validate(arg_values.len()) {
                    return Value::Error(message);
                }
            }

            return match &arg_values[0] {
                Value::Array(arr) => Value::Array(arr.clone()),
                Value::Iterator { .. } => interp.collect_iterator(arg_values[0].clone()),
                _ => Value::Error("collect() expects an iterator or array".to_string()),
            };
        }
        _ => {}
    }

    let canonical_name = Interpreter::canonical_native_function_name(name);

    if let Some(capability) = Interpreter::native_function_capability(canonical_name) {
        if let Err(error) = interp.require_capability(capability, canonical_name) {
            return error;
        }
    }
    for capability in
        super::capabilities::additional_capabilities_for_native_function(canonical_name)
    {
        if let Err(error) = interp.require_capability(*capability, canonical_name) {
            return error;
        }
    }

    if let Some(arity) = Interpreter::native_function_arity(canonical_name) {
        if let Err(message) = arity.validate(arg_values.len()) {
            return Value::Error(message);
        }
    }

    // Try async operations first (high priority for async functions)
    if let Some(result) = async_ops::handle(interp, canonical_name, arg_values) {
        return result;
    }
    // Try each category in order
    if let Some(result) = io::handle(interp, canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = math::handle(canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = vector::handle(canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = token::handle(canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = strings::handle(canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = collections::handle(interp, canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = type_ops::handle(canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = filesystem::handle(interp, canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = http::handle_with_interpreter(interp, canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = json::handle(canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = schema::handle(canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = crypto::handle(canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = system::handle(canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = concurrency::handle(interp, canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = database::handle(canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = dns::handle(interp, canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = tls::handle(interp, canonical_name, arg_values) {
        return result;
    }
    if let Some(result) = network::handle(interp, canonical_name, arg_values) {
        return result;
    }

    // Unknown function
    Value::Error(format!("Unknown native function: {}", name))
}

#[cfg(test)]
mod tests {
    use super::call_native_function;
    use crate::interpreter::{AsyncRuntime, Interpreter, LeakyFunctionBody, Value};
    use std::sync::{Arc, Mutex, OnceLock};

    fn cwd_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn available_tcp_port() -> Option<i64> {
        match std::net::TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => Some(
                listener.local_addr().expect("ephemeral tcp listener should have local addr").port()
                    as i64,
            ),
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => None,
            Err(error) => panic!("ephemeral tcp listener should bind: {}", error),
        }
    }

    fn available_udp_port() -> Option<i64> {
        match std::net::UdpSocket::bind("127.0.0.1:0") {
            Ok(socket) => Some(
                socket.local_addr().expect("ephemeral udp socket should have local addr").port()
                    as i64,
            ),
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => None,
            Err(error) => panic!("ephemeral udp socket should bind: {}", error),
        }
    }

    fn tmp_test_path(file_name: &str) -> String {
        let mut path = std::env::current_dir().expect("current_dir should resolve");
        path.push("tmp");
        path.push("native_zip_dispatch_tests");
        std::fs::create_dir_all(&path).expect("zip test tmp dir should be created");
        path.push(file_name);
        path.to_string_lossy().to_string()
    }

    fn is_unknown_native_error(value: &Value) -> bool {
        if let Value::Error(message) = value {
            message.starts_with("Unknown native function:")
        } else {
            false
        }
    }

    fn await_native_promise(value: Value) -> Result<Value, String> {
        match value {
            Value::Promise { receiver, .. } => AsyncRuntime::block_on(async {
                let rx = {
                    let mut receiver_guard = receiver.lock().unwrap();
                    let (dummy_tx, dummy_rx) = tokio::sync::oneshot::channel();
                    drop(dummy_tx);
                    std::mem::replace(&mut *receiver_guard, dummy_rx)
                };

                match rx.await {
                    Ok(result) => result,
                    Err(_) => Err("Promise channel closed before resolution".to_string()),
                }
            }),
            other => panic!("Expected Promise value, got {:?}", other),
        }
    }

    fn noop_spawnable_function() -> Value {
        Value::Function(vec![], LeakyFunctionBody::new(Vec::<crate::ast::Stmt>::new()), None)
    }

    #[test]
    fn test_unknown_native_function_returns_explicit_error() {
        let mut interpreter = Interpreter::new();
        let result = call_native_function(&mut interpreter, "__unknown_native_test__", &[]);

        match result {
            Value::Error(message) => {
                assert_eq!(message, "Unknown native function: __unknown_native_test__");
            }
            other => panic!("Expected Value::Error, got {:?}", other),
        }
    }

    #[test]
    fn test_release_hardening_builtin_dispatch_coverage_for_recent_apis() {
        let mut interpreter = Interpreter::new();
        let critical_builtin_names = [
            "len",
            "type",
            "is_int",
            "is_float",
            "is_string",
            "is_bool",
            "is_array",
            "is_dict",
            "is_null",
            "is_function",
            "parse_int",
            "parse_float",
            "to_int",
            "to_float",
            "to_string",
            "secret",
            "reveal",
            "is_secret",
            "to_bool",
            "bytes",
            "Set",
            "ssg_render_pages",
            "ssg_build_output_paths",
            "ssg_render_and_write_pages",
            "ssg_read_render_and_write_pages",
            "upper",
            "lower",
            "replace",
            "append",
            "push",
            "pop",
            "insert",
            "remove",
            "remove_at",
            "clear",
            "slice",
            "concat",
            "map",
            "filter",
            "reduce",
            "find",
            "sort",
            "reverse",
            "unique",
            "sum",
            "any",
            "all",
            "chunk",
            "flatten",
            "zip",
            "enumerate",
            "take",
            "skip",
            "windows",
            "starts_with",
            "ends_with",
            "repeat",
            "char_at",
            "is_empty",
            "count_chars",
            "pad_left",
            "pad_right",
            "pad_start",
            "pad_end",
            "read_file_lossy",
            "lines",
            "words",
            "str_reverse",
            "slugify",
            "truncate",
            "to_camel_case",
            "to_snake_case",
            "to_kebab_case",
            "substring",
            "capitalize",
            "escape_xml",
            "render_markdown",
            "trim",
            "trim_start",
            "trim_end",
            "is_truthy",
            "split",
            "join",
            "Promise.all",
            "parallel_map",
            "promise_wait",
            "par_map",
            "channel",
            "async_sleep",
            "async_timeout",
            "async_http_get",
            "async_http_post",
            "async_read_file",
            "async_write_file",
            "spawn_task",
            "await_task",
            "cancel_task",
            "sleep",
            "execute",
            "contains",
            "index_of",
            "io_read_bytes",
            "io_write_bytes",
            "io_append_bytes",
            "io_read_at",
            "io_write_at",
            "io_seek_read",
            "io_file_metadata",
            "io_set_permissions",
            "io_write_private_file",
            "io_truncate",
            "io_copy_range",
            "http_get",
            "http_request",
            "http_post",
            "http_put",
            "http_delete",
            "http_get_binary",
            "http_download_file",
            "http_upload_file",
            "parallel_http",
            "jwt_encode",
            "jwt_decode",
            "jwt_verify",
            "oauth2_auth_url",
            "oauth2_get_token",
            "http_get_stream",
            "http_server",
            "http_listen",
            "http_response",
            "json_response",
            "html_response",
            "redirect_response",
            "set_header",
            "set_headers",
            "db_connect",
            "db_execute",
            "db_query",
            "db_close",
            "db_pool",
            "db_pool_acquire",
            "db_pool_release",
            "db_pool_stats",
            "db_pool_close",
            "db_begin",
            "db_commit",
            "db_rollback",
            "db_last_insert_id",
            "join_path",
            "path_join",
            "set_add",
            "set_has",
            "set_remove",
            "set_union",
            "set_intersect",
            "set_difference",
            "set_to_array",
            "Queue",
            "queue_enqueue",
            "queue_dequeue",
            "queue_peek",
            "queue_is_empty",
            "queue_to_array",
            "Stack",
            "stack_push",
            "stack_pop",
            "stack_peek",
            "stack_is_empty",
            "stack_to_array",
            "queue_size",
            "stack_size",
            "shared_set",
            "shared_get",
            "shared_has",
            "shared_delete",
            "shared_add_int",
            "async_read_files",
            "async_write_files",
            "promise_all",
            "await_all",
            "par_each",
            "set_task_pool_size",
            "get_task_pool_size",
            "spawn_process",
            "pipe_commands",
            "load_image",
            "gif_to_webp",
            "zip_create",
            "zip_add_file",
            "zip_add_dir",
            "zip_close",
            "unzip",
            "ip_classify",
            "tcp_bind_probe",
            "tcp_listen",
            "tcp_accept",
            "tcp_connect",
            "tcp_connect_bound",
            "tcp_send",
            "tcp_receive",
            "tcp_close",
            "tcp_set_nonblocking",
            "tcp_info",
            "tcp_set_timeouts",
            "udp_bind",
            "udp_send_to",
            "udp_receive_from",
            "udp_close",
            "dns_lookup_mx",
            "dns_lookup_txt",
            "dns_lookup_ptr",
            "dns_lookup_tlsa",
            "tls_connect",
            "tls_upgrade_client",
            "tls_acceptor",
            "tls_upgrade_server",
            "tls_send",
            "tls_receive",
            "tls_close",
            "tls_info",
            "sha256",
            "hmac_sha256",
            "hmac_sha256_verify",
            "md5",
            "md5_file",
            "hash_password",
            "verify_password",
            "aes_encrypt",
            "aes_decrypt",
            "aes_encrypt_bytes",
            "aes_decrypt_bytes",
            "aes_encrypt_file_stream",
            "aes_decrypt_file_stream",
            "rsa_generate_keypair",
            "rsa_encrypt",
            "rsa_decrypt",
            "rsa_sign",
            "rsa_verify",
            "random",
            "random_int",
            "random_choice",
            "uuid_v4",
            "random_id",
            "set_random_seed",
            "clear_random_seed",
            "now",
            "now_utc",
            "now_unix",
            "now_utc_seconds",
            "current_timestamp",
            "performance_now",
            "time_us",
            "time_ns",
            "format_duration",
            "elapsed",
            "format_date",
            "parse_date",
            "abs",
            "sqrt",
            "pow",
            "floor",
            "ceil",
            "round",
            "min",
            "max",
            "sin",
            "cos",
            "tan",
            "log",
            "exp",
            "range",
            "keys",
            "values",
            "items",
            "has_key",
            "get",
            "merge",
            "invert",
            "update",
            "get_default",
            "format",
            "parse_json",
            "to_json",
            "to_json_pretty",
            "parse_toml",
            "to_toml",
            "parse_yaml",
            "to_yaml",
            "parse_csv",
            "to_csv",
            "encode_base64",
            "decode_base64",
            "decode_base64_utf8",
            "encode_uri_component",
            "regex_match",
            "regex_find_all",
            "regex_replace",
            "regex_split",
            "assert",
            "debug",
            "assert_equal",
            "assert_true",
            "assert_false",
            "assert_contains",
            "read_file",
            "write_file",
            "write_file_atomic",
            "append_file",
            "file_exists",
            "read_lines",
            "jsonl_query",
            "list_dir",
            "create_dir",
            "file_size",
            "delete_file",
            "rename_file",
            "copy_file",
            "read_binary_file",
            "write_binary_file",
            "env",
            "env_or",
            "env_int",
            "env_float",
            "env_bool",
            "env_required",
            "kv_set",
            "kv_get",
            "env_set",
            "env_list",
            "args",
            "arg_parser",
            "os_getcwd",
            "os_chdir",
            "os_rmdir",
            "os_environ",
            "dirname",
            "basename",
            "path_exists",
            "path_absolute",
            "path_is_dir",
            "path_is_file",
            "path_extension",
        ];

        for builtin_name in critical_builtin_names {
            let result = call_native_function(&mut interpreter, builtin_name, &[]);
            assert!(
                !is_unknown_native_error(&result),
                "Builtin '{}' unexpectedly hit unknown-native fallback with result {:?}",
                builtin_name,
                result
            );
        }
    }

    #[test]
    fn test_release_hardening_builtin_dispatch_coverage_for_declared_builtins() {
        let mut interpreter = Interpreter::new();
        let mut unknown_builtin_names = Vec::new();
        let expected_known_legacy_dispatch_gaps: Vec<String> = vec![];

        for builtin_name in Interpreter::get_builtin_names() {
            let probe_args = match builtin_name {
                "input" => vec![Value::Int(1)],
                "exit" => vec![Value::Str(Arc::new("non-numeric".to_string()))],
                _ => vec![],
            };

            let result = call_native_function(&mut interpreter, builtin_name, &probe_args);
            if is_unknown_native_error(&result) {
                unknown_builtin_names.push(builtin_name.to_string());
            }
        }

        assert_eq!(
            unknown_builtin_names,
            expected_known_legacy_dispatch_gaps,
            "Declared builtin dispatch drift changed. If a gap was fixed, remove it from expected list; if a new gap appeared, investigate and either fix dispatch or explicitly acknowledge it here."
        );
    }

    #[test]
    fn test_release_hardening_async_sleep_timeout_contracts() {
        let mut interpreter = Interpreter::new();

        let sleep_missing_args = call_native_function(&mut interpreter, "async_sleep", &[]);
        assert!(
            matches!(sleep_missing_args, Value::Error(message) if message.contains("expects 1 argument"))
        );

        let sleep_negative =
            call_native_function(&mut interpreter, "async_sleep", &[Value::Int(-1)]);
        assert!(
            matches!(sleep_negative, Value::Error(message) if message.contains("requires non-negative milliseconds"))
        );

        let timeout_wrong_first_arg = call_native_function(
            &mut interpreter,
            "async_timeout",
            &[Value::Int(1), Value::Int(5)],
        );
        assert!(
            matches!(timeout_wrong_first_arg, Value::Error(message) if message.contains("requires a Promise as first argument"))
        );

        let timeout_non_positive_sleep =
            call_native_function(&mut interpreter, "async_sleep", &[Value::Int(1)]);
        let timeout_non_positive = call_native_function(
            &mut interpreter,
            "async_timeout",
            &[timeout_non_positive_sleep, Value::Int(0)],
        );
        assert!(
            matches!(timeout_non_positive, Value::Error(message) if message.contains("requires positive timeout_ms"))
        );

        let quick_sleep = call_native_function(&mut interpreter, "async_sleep", &[Value::Int(2)]);
        let timeout_success =
            call_native_function(&mut interpreter, "async_timeout", &[quick_sleep, Value::Int(50)]);
        let timeout_success_result = await_native_promise(timeout_success);
        assert!(matches!(timeout_success_result, Ok(Value::Null)));

        let slow_sleep = call_native_function(&mut interpreter, "async_sleep", &[Value::Int(30)]);
        let timeout_failure =
            call_native_function(&mut interpreter, "async_timeout", &[slow_sleep, Value::Int(1)]);
        let timeout_failure_result = await_native_promise(timeout_failure);
        assert!(
            matches!(timeout_failure_result, Err(message) if message.contains("Timeout after"))
        );
    }

    #[test]
    fn test_release_hardening_async_http_argument_contracts() {
        let mut interpreter = Interpreter::new();

        let http_get_wrong_arity = call_native_function(
            &mut interpreter,
            "async_http_get",
            &[Value::Str(Arc::new("https://example.com".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(http_get_wrong_arity, Value::Error(message) if message.contains("expects 1 argument"))
        );

        let http_get_wrong_type =
            call_native_function(&mut interpreter, "async_http_get", &[Value::Int(1)]);
        assert!(
            matches!(http_get_wrong_type, Value::Error(message) if message.contains("requires a string URL argument"))
        );

        let http_post_wrong_arity = call_native_function(
            &mut interpreter,
            "async_http_post",
            &[Value::Str(Arc::new("https://example.com".to_string()))],
        );
        assert!(
            matches!(http_post_wrong_arity, Value::Error(message) if message.contains("expects 2-3 arguments"))
        );

        let http_post_bad_headers = call_native_function(
            &mut interpreter,
            "async_http_post",
            &[
                Value::Str(Arc::new("https://example.com".to_string())),
                Value::Str(Arc::new("payload".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(http_post_bad_headers, Value::Error(message) if message.contains("headers must be a dictionary"))
        );
    }

    #[test]
    fn test_release_hardening_async_file_wrapper_contracts() {
        let mut interpreter = Interpreter::new();
        let file_path = tmp_test_path("release_hardening_async_file_wrapper.txt");

        let async_read_bad_arg =
            call_native_function(&mut interpreter, "async_read_file", &[Value::Int(1)]);
        assert!(
            matches!(async_read_bad_arg, Value::Error(message) if message.contains("requires a string path argument"))
        );

        let async_write_bad_arg = call_native_function(
            &mut interpreter,
            "async_write_file",
            &[Value::Str(Arc::new(file_path.clone())), Value::Int(1)],
        );
        assert!(
            matches!(async_write_bad_arg, Value::Error(message) if message.contains("requires a string content argument"))
        );

        let write_promise = call_native_function(
            &mut interpreter,
            "async_write_file",
            &[
                Value::Str(Arc::new(file_path.clone())),
                Value::Str(Arc::new("release-hardening-async".to_string())),
            ],
        );
        let write_result = await_native_promise(write_promise);
        assert!(matches!(write_result, Ok(Value::Bool(true))));

        let read_promise = call_native_function(
            &mut interpreter,
            "async_read_file",
            &[Value::Str(Arc::new(file_path.clone()))],
        );
        let read_result = await_native_promise(read_promise);
        assert!(
            matches!(read_result, Ok(Value::Str(content)) if content.as_ref() == "release-hardening-async")
        );

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_release_hardening_channel_and_task_handle_contracts() {
        let mut interpreter = Interpreter::new();

        let channel_with_args = call_native_function(&mut interpreter, "channel", &[Value::Int(1)]);
        assert!(
            matches!(channel_with_args, Value::Error(message) if message.contains("channel() expects 0 arguments"))
        );

        let channel_value = call_native_function(&mut interpreter, "channel", &[]);
        assert!(matches!(channel_value, Value::Channel(_)));

        let await_task_missing = call_native_function(&mut interpreter, "await_task", &[]);
        assert!(
            matches!(await_task_missing, Value::Error(message) if message.contains("await_task() expects 1 argument"))
        );

        let await_task_bad_type =
            call_native_function(&mut interpreter, "await_task", &[Value::Int(1)]);
        assert!(
            matches!(await_task_bad_type, Value::Error(message) if message.contains("await_task() requires a TaskHandle argument"))
        );

        let cancel_task_missing = call_native_function(&mut interpreter, "cancel_task", &[]);
        assert!(
            matches!(cancel_task_missing, Value::Error(message) if message.contains("cancel_task() expects 1 argument"))
        );

        let cancel_task_bad_type =
            call_native_function(&mut interpreter, "cancel_task", &[Value::Int(1)]);
        assert!(
            matches!(cancel_task_bad_type, Value::Error(message) if message.contains("cancel_task() requires a TaskHandle argument"))
        );

        let spawn_bad_arg = call_native_function(&mut interpreter, "spawn_task", &[Value::Int(1)]);
        assert!(
            matches!(spawn_bad_arg, Value::Error(message) if message.contains("requires an async function argument"))
        );

        let task_handle =
            call_native_function(&mut interpreter, "spawn_task", &[noop_spawnable_function()]);
        assert!(matches!(task_handle, Value::TaskHandle { .. }));

        let await_task_promise =
            call_native_function(&mut interpreter, "await_task", &[task_handle.clone()]);
        let await_task_result = await_native_promise(await_task_promise);
        assert!(matches!(await_task_result, Ok(Value::Null)));

        let await_task_consumed =
            call_native_function(&mut interpreter, "await_task", &[task_handle.clone()]);
        let await_task_consumed_result = await_native_promise(await_task_consumed);
        assert!(
            matches!(await_task_consumed_result, Err(message) if message.contains("already consumed"))
        );

        let cancel_target =
            call_native_function(&mut interpreter, "spawn_task", &[noop_spawnable_function()]);
        let cancel_result =
            call_native_function(&mut interpreter, "cancel_task", &[cancel_target.clone()]);
        assert!(matches!(cancel_result, Value::Bool(true)));

        let cancel_again_result =
            call_native_function(&mut interpreter, "cancel_task", &[cancel_target.clone()]);
        assert!(matches!(cancel_again_result, Value::Bool(false)));

        let await_cancelled =
            call_native_function(&mut interpreter, "await_task", &[cancel_target]);
        let await_cancelled_result = await_native_promise(await_cancelled);
        assert!(
            matches!(await_cancelled_result, Err(message) if message.contains("already consumed"))
        );
    }

    #[test]
    fn test_release_hardening_system_operation_contracts() {
        let mut interpreter = Interpreter::new();

        let sleep_missing_args = call_native_function(&mut interpreter, "sleep", &[]);
        assert!(
            matches!(sleep_missing_args, Value::Error(message) if message.contains("expects 1 argument"))
        );

        let sleep_wrong_type = call_native_function(
            &mut interpreter,
            "sleep",
            &[Value::Str(Arc::new("1".to_string()))],
        );
        assert!(
            matches!(sleep_wrong_type, Value::Error(message) if message.contains("requires a number argument"))
        );

        let sleep_negative = call_native_function(&mut interpreter, "sleep", &[Value::Int(-1)]);
        assert!(
            matches!(sleep_negative, Value::Error(message) if message.contains("non-negative milliseconds"))
        );

        let sleep_success = call_native_function(&mut interpreter, "sleep", &[Value::Int(0)]);
        assert!(matches!(sleep_success, Value::Int(0)));

        let execute_missing_args = call_native_function(&mut interpreter, "execute", &[]);
        assert!(
            matches!(execute_missing_args, Value::Error(message) if message.contains("expects 1-2 arguments"))
        );

        let execute_wrong_type =
            call_native_function(&mut interpreter, "execute", &[Value::Int(1)]);
        assert!(
            matches!(execute_wrong_type, Value::Error(message) if message.contains("requires a string command"))
        );

        let execute_extra_arg = call_native_function(
            &mut interpreter,
            "execute",
            &[
                Value::Str(Arc::new("echo release-hardening-system".to_string())),
                Value::Dict(Arc::new(Default::default())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(execute_extra_arg, Value::Error(message) if message.contains("expects 1-2 arguments"))
        );

        let execute_success = call_native_function(
            &mut interpreter,
            "execute",
            &[Value::Str(Arc::new("echo release-hardening-system".to_string()))],
        );
        assert!(
            matches!(execute_success, Value::Str(output) if output.as_ref().contains("release-hardening-system"))
        );

        let execute_status_success = call_native_function(
            &mut interpreter,
            "execute_status",
            &[Value::Str(Arc::new("echo release-hardening-system".to_string()))],
        );
        assert!(
            matches!(execute_status_success, Value::Struct { name, .. } if name == "ProcessResult")
        );

        let input_wrong_type = call_native_function(&mut interpreter, "input", &[Value::Int(1)]);
        assert!(
            matches!(input_wrong_type, Value::Error(message) if message.contains("requires a string prompt"))
        );

        let input_extra_arg = call_native_function(
            &mut interpreter,
            "input",
            &[
                Value::Str(Arc::new("prompt".to_string())),
                Value::Str(Arc::new("extra".to_string())),
            ],
        );
        assert!(
            matches!(input_extra_arg, Value::Error(message) if message.contains("input expects 0 to 1 arguments, got 2"))
        );

        let exit_extra_arg =
            call_native_function(&mut interpreter, "exit", &[Value::Int(1), Value::Int(2)]);
        assert!(
            matches!(exit_extra_arg, Value::Error(message) if message.contains("exit expects 0 to 1 arguments, got 2"))
        );

        let exit_wrong_type = call_native_function(
            &mut interpreter,
            "exit",
            &[Value::Str(Arc::new("bad".to_string()))],
        );
        assert!(
            matches!(exit_wrong_type, Value::Error(message) if message.contains("requires a numeric exit code"))
        );
    }

    #[test]
    fn test_release_hardening_contains_index_of_argument_and_polymorphic_contracts() {
        let mut interpreter = Interpreter::new();

        let contains_missing_args = call_native_function(&mut interpreter, "contains", &[]);
        assert!(
            matches!(contains_missing_args, Value::Error(message) if message.contains("contains expects 2 arguments, got 0"))
        );

        let index_of_missing_args = call_native_function(&mut interpreter, "index_of", &[]);
        assert!(
            matches!(index_of_missing_args, Value::Error(message) if message.contains("index_of expects 2 arguments, got 0"))
        );

        let contains_array = call_native_function(
            &mut interpreter,
            "contains",
            &[Value::Array(std::sync::Arc::new(vec![Value::Int(1), Value::Int(2)])), Value::Int(2)],
        );
        assert!(matches!(contains_array, Value::Bool(true)));

        let index_of_array = call_native_function(
            &mut interpreter,
            "index_of",
            &[
                Value::Array(std::sync::Arc::new(vec![Value::Int(10), Value::Int(20)])),
                Value::Int(20),
            ],
        );
        assert!(matches!(index_of_array, Value::Int(1)));

        let index_of_unicode = call_native_function(
            &mut interpreter,
            "index_of",
            &[Value::Str(Arc::new("a→b".to_string())), Value::Str(Arc::new("b".to_string()))],
        );
        assert!(matches!(index_of_unicode, Value::Int(2)));
    }

    #[test]
    fn test_release_hardening_len_polymorphic_contracts() {
        let mut interpreter = Interpreter::new();

        let string_len = call_native_function(
            &mut interpreter,
            "len",
            &[Value::Str(Arc::new("kujo".to_string()))],
        );
        assert!(matches!(string_len, Value::Int(4)));

        let unicode_len = call_native_function(
            &mut interpreter,
            "len",
            &[Value::Str(Arc::new("Free Tools → Plan".to_string()))],
        );
        assert!(matches!(unicode_len, Value::Int(17)));

        let array_len = call_native_function(
            &mut interpreter,
            "len",
            &[Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)]))],
        );
        assert!(matches!(array_len, Value::Int(2)));

        let mut dict = crate::interpreter::DictMap::default();
        dict.insert("a".into(), Value::Int(1));
        dict.insert("b".into(), Value::Int(2));
        let dict_len =
            call_native_function(&mut interpreter, "len", &[Value::Dict(Arc::new(dict))]);
        assert!(matches!(dict_len, Value::Int(2)));

        let bytes_len =
            call_native_function(&mut interpreter, "len", &[Value::Bytes(vec![1, 2, 3])]);
        assert!(matches!(bytes_len, Value::Int(3)));

        let set_len = call_native_function(
            &mut interpreter,
            "len",
            &[Value::Set(vec![Value::Int(1), Value::Int(2), Value::Int(3)])],
        );
        assert!(matches!(set_len, Value::Int(3)));

        let queue_len = call_native_function(
            &mut interpreter,
            "len",
            &[Value::Queue(std::collections::VecDeque::from(vec![Value::Int(1), Value::Int(2)]))],
        );
        assert!(matches!(queue_len, Value::Int(2)));

        let stack_len = call_native_function(
            &mut interpreter,
            "len",
            &[Value::Stack(vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)])],
        );
        assert!(matches!(stack_len, Value::Int(4)));

        let invalid_len = call_native_function(&mut interpreter, "len", &[Value::Null]);
        assert!(matches!(invalid_len, Value::Error(message) if message.contains("len() requires")));

        let missing_args_len = call_native_function(&mut interpreter, "len", &[]);
        assert!(matches!(
            missing_args_len,
            Value::Error(message) if message.contains("len expects 1 arguments, got 0")
        ));

        let extra_args_len =
            call_native_function(&mut interpreter, "len", &[Value::Int(1), Value::Int(2)]);
        assert!(matches!(
            extra_args_len,
            Value::Error(message) if message.contains("len expects 1 arguments, got 2")
        ));
    }

    #[test]
    fn test_release_hardening_type_and_is_introspection_contracts() {
        let mut interpreter = Interpreter::new();

        let type_string = call_native_function(
            &mut interpreter,
            "type",
            &[Value::Str(Arc::new("kujo".to_string()))],
        );
        assert!(matches!(type_string, Value::Str(name) if name.as_ref() == "string"));

        let type_array = call_native_function(
            &mut interpreter,
            "type",
            &[Value::Array(Arc::new(vec![Value::Int(1)]))],
        );
        assert!(matches!(type_array, Value::Str(name) if name.as_ref() == "array"));

        let type_null = call_native_function(&mut interpreter, "type", &[Value::Null]);
        assert!(matches!(type_null, Value::Str(name) if name.as_ref() == "null"));

        let type_missing = call_native_function(&mut interpreter, "type", &[]);
        assert!(
            matches!(type_missing, Value::Error(message) if message.contains("type expects 1 arguments, got 0"))
        );

        let type_extra =
            call_native_function(&mut interpreter, "type", &[Value::Int(1), Value::Int(2)]);
        assert!(
            matches!(type_extra, Value::Error(message) if message.contains("type expects 1 arguments, got 2"))
        );

        let is_int_true = call_native_function(&mut interpreter, "is_int", &[Value::Int(7)]);
        assert!(matches!(is_int_true, Value::Bool(true)));
        let is_int_false = call_native_function(&mut interpreter, "is_int", &[Value::Float(7.0)]);
        assert!(matches!(is_int_false, Value::Bool(false)));
        let is_int_missing = call_native_function(&mut interpreter, "is_int", &[]);
        assert!(matches!(is_int_missing, Value::Bool(false)));
        let is_int_extra =
            call_native_function(&mut interpreter, "is_int", &[Value::Int(7), Value::Int(8)]);
        assert!(
            matches!(is_int_extra, Value::Error(message) if message.contains("is_int() expects 1 argument"))
        );

        let is_float_true =
            call_native_function(&mut interpreter, "is_float", &[Value::Float(7.5)]);
        assert!(matches!(is_float_true, Value::Bool(true)));
        let is_float_false = call_native_function(&mut interpreter, "is_float", &[Value::Int(7)]);
        assert!(matches!(is_float_false, Value::Bool(false)));
        let is_float_extra =
            call_native_function(&mut interpreter, "is_float", &[Value::Float(7.5), Value::Int(8)]);
        assert!(
            matches!(is_float_extra, Value::Error(message) if message.contains("is_float() expects 1 argument"))
        );

        let is_string_true = call_native_function(
            &mut interpreter,
            "is_string",
            &[Value::Str(Arc::new("x".to_string()))],
        );
        assert!(matches!(is_string_true, Value::Bool(true)));
        let is_string_false =
            call_native_function(&mut interpreter, "is_string", &[Value::Bool(true)]);
        assert!(matches!(is_string_false, Value::Bool(false)));
        let is_string_extra = call_native_function(
            &mut interpreter,
            "is_string",
            &[Value::Str(Arc::new("x".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(is_string_extra, Value::Error(message) if message.contains("is_string() expects 1 argument"))
        );

        let is_bool_true = call_native_function(&mut interpreter, "is_bool", &[Value::Bool(true)]);
        assert!(matches!(is_bool_true, Value::Bool(true)));
        let is_bool_false = call_native_function(
            &mut interpreter,
            "is_bool",
            &[Value::Str(Arc::new("true".to_string()))],
        );
        assert!(matches!(is_bool_false, Value::Bool(false)));
        let is_bool_extra = call_native_function(
            &mut interpreter,
            "is_bool",
            &[Value::Bool(true), Value::Bool(false)],
        );
        assert!(
            matches!(is_bool_extra, Value::Error(message) if message.contains("is_bool() expects 1 argument"))
        );

        let is_array_true =
            call_native_function(&mut interpreter, "is_array", &[Value::Array(Arc::new(vec![]))]);
        assert!(matches!(is_array_true, Value::Bool(true)));
        let is_array_false = call_native_function(&mut interpreter, "is_array", &[Value::Null]);
        assert!(matches!(is_array_false, Value::Bool(false)));
        let is_array_extra = call_native_function(
            &mut interpreter,
            "is_array",
            &[Value::Array(Arc::new(vec![])), Value::Int(1)],
        );
        assert!(
            matches!(is_array_extra, Value::Error(message) if message.contains("is_array() expects 1 argument"))
        );

        let mut dict = crate::interpreter::DictMap::default();
        dict.insert("k".into(), Value::Int(1));
        let is_dict_true =
            call_native_function(&mut interpreter, "is_dict", &[Value::Dict(Arc::new(dict))]);
        assert!(matches!(is_dict_true, Value::Bool(true)));
        let is_dict_false =
            call_native_function(&mut interpreter, "is_dict", &[Value::Array(Arc::new(vec![]))]);
        assert!(matches!(is_dict_false, Value::Bool(false)));
        let is_dict_extra = call_native_function(
            &mut interpreter,
            "is_dict",
            &[Value::Dict(Arc::new(crate::interpreter::DictMap::default())), Value::Int(1)],
        );
        assert!(
            matches!(is_dict_extra, Value::Error(message) if message.contains("is_dict() expects 1 argument"))
        );

        let is_null_true = call_native_function(&mut interpreter, "is_null", &[Value::Null]);
        assert!(matches!(is_null_true, Value::Bool(true)));
        let is_null_false = call_native_function(&mut interpreter, "is_null", &[Value::Int(0)]);
        assert!(matches!(is_null_false, Value::Bool(false)));
        let is_null_extra =
            call_native_function(&mut interpreter, "is_null", &[Value::Null, Value::Int(0)]);
        assert!(
            matches!(is_null_extra, Value::Error(message) if message.contains("is_null() expects 1 argument"))
        );

        let is_function_true = call_native_function(
            &mut interpreter,
            "is_function",
            &[Value::NativeFunction("len".to_string())],
        );
        assert!(matches!(is_function_true, Value::Bool(true)));
        let is_function_false =
            call_native_function(&mut interpreter, "is_function", &[Value::Int(1)]);
        assert!(matches!(is_function_false, Value::Bool(false)));
        let is_function_missing = call_native_function(&mut interpreter, "is_function", &[]);
        assert!(matches!(is_function_missing, Value::Bool(false)));
        let is_function_extra = call_native_function(
            &mut interpreter,
            "is_function",
            &[Value::NativeFunction("len".to_string()), Value::Int(1)],
        );
        assert!(
            matches!(is_function_extra, Value::Error(message) if message.contains("is_function() expects 1 argument"))
        );
    }

    #[test]
    fn test_release_hardening_conversion_and_bytes_contracts() {
        let mut interpreter = Interpreter::new();

        let parse_int_ok = call_native_function(
            &mut interpreter,
            "parse_int",
            &[Value::Str(Arc::new("42".to_string()))],
        );
        assert!(matches!(parse_int_ok, Value::Int(42)));

        let parse_int_bad = call_native_function(
            &mut interpreter,
            "parse_int",
            &[Value::Str(Arc::new("abc".to_string()))],
        );
        assert!(
            matches!(parse_int_bad, Value::Error(message) if message.contains("Cannot parse 'abc' as integer"))
        );

        let parse_int_extra = call_native_function(
            &mut interpreter,
            "parse_int",
            &[Value::Str(Arc::new("42".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(parse_int_extra, Value::Error(message) if message.contains("parse_int requires a string argument"))
        );

        let parse_float_ok = call_native_function(
            &mut interpreter,
            "parse_float",
            &[Value::Str(Arc::new("3.25".to_string()))],
        );
        assert!(matches!(parse_float_ok, Value::Float(value) if (value - 3.25).abs() < 1e-12));

        let parse_float_bad = call_native_function(
            &mut interpreter,
            "parse_float",
            &[Value::Str(Arc::new("not-float".to_string()))],
        );
        assert!(
            matches!(parse_float_bad, Value::Error(message) if message.contains("Cannot parse 'not-float' as float"))
        );

        let parse_float_extra = call_native_function(
            &mut interpreter,
            "parse_float",
            &[Value::Str(Arc::new("3.25".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(parse_float_extra, Value::Error(message) if message.contains("parse_float requires a string argument"))
        );

        let to_int_ok = call_native_function(
            &mut interpreter,
            "to_int",
            &[Value::Str(Arc::new("7".to_string()))],
        );
        assert!(matches!(to_int_ok, Value::Int(7)));

        let to_int_bad = call_native_function(
            &mut interpreter,
            "to_int",
            &[Value::Str(Arc::new("bad".to_string()))],
        );
        assert!(
            matches!(to_int_bad, Value::Error(message) if message.contains("Cannot convert 'bad' to int"))
        );

        let to_int_extra =
            call_native_function(&mut interpreter, "to_int", &[Value::Int(7), Value::Int(8)]);
        assert!(
            matches!(to_int_extra, Value::Error(message) if message.contains("to_int() requires one argument"))
        );

        let to_float_ok = call_native_function(
            &mut interpreter,
            "to_float",
            &[Value::Str(Arc::new("2.5".to_string()))],
        );
        assert!(matches!(to_float_ok, Value::Float(value) if (value - 2.5).abs() < 1e-12));

        let to_float_bad = call_native_function(
            &mut interpreter,
            "to_float",
            &[Value::Str(Arc::new("bad".to_string()))],
        );
        assert!(
            matches!(to_float_bad, Value::Error(message) if message.contains("Cannot convert 'bad' to float"))
        );

        let to_float_extra =
            call_native_function(&mut interpreter, "to_float", &[Value::Float(2.5), Value::Int(1)]);
        assert!(
            matches!(to_float_extra, Value::Error(message) if message.contains("to_float() requires one argument"))
        );

        let to_string_ok = call_native_function(&mut interpreter, "to_string", &[Value::Int(99)]);
        assert!(matches!(to_string_ok, Value::Str(value) if value.as_ref() == "99"));

        let to_string_extra =
            call_native_function(&mut interpreter, "to_string", &[Value::Int(99), Value::Int(1)]);
        assert!(
            matches!(to_string_extra, Value::Error(message) if message.contains("to_string() requires one argument"))
        );

        let to_bool_false = call_native_function(
            &mut interpreter,
            "to_bool",
            &[Value::Str(Arc::new("false".to_string()))],
        );
        assert!(matches!(to_bool_false, Value::Bool(false)));

        let to_bool_true = call_native_function(
            &mut interpreter,
            "to_bool",
            &[Value::Str(Arc::new("yes".to_string()))],
        );
        assert!(matches!(to_bool_true, Value::Bool(true)));

        let to_bool_extra =
            call_native_function(&mut interpreter, "to_bool", &[Value::Bool(true), Value::Int(1)]);
        assert!(
            matches!(to_bool_extra, Value::Error(message) if message.contains("to_bool() requires one argument"))
        );

        let bytes_ok = call_native_function(
            &mut interpreter,
            "bytes",
            &[Value::Array(Arc::new(vec![Value::Int(65), Value::Int(66), Value::Int(67)]))],
        );
        assert!(matches!(bytes_ok, Value::Bytes(values) if values == vec![65, 66, 67]));

        let bytes_bad_range = call_native_function(
            &mut interpreter,
            "bytes",
            &[Value::Array(Arc::new(vec![Value::Int(256)]))],
        );
        assert!(
            matches!(bytes_bad_range, Value::Error(message) if message.contains("requires integers in range 0-255"))
        );

        let bytes_bad_shape = call_native_function(
            &mut interpreter,
            "bytes",
            &[Value::Array(Arc::new(vec![Value::Str(Arc::new("x".to_string()))]))],
        );
        assert!(
            matches!(bytes_bad_shape, Value::Error(message) if message.contains("requires an array of integers"))
        );

        let bytes_extra = call_native_function(
            &mut interpreter,
            "bytes",
            &[Value::Array(Arc::new(vec![Value::Int(65), Value::Int(66)])), Value::Int(67)],
        );
        assert!(
            matches!(bytes_extra, Value::Error(message) if message.contains("bytes() requires an array argument"))
        );
    }

    #[test]
    fn test_release_hardening_core_alias_behavior_parity_contracts() {
        let mut interpreter = Interpreter::new();

        let to_upper = call_native_function(
            &mut interpreter,
            "to_upper",
            &[Value::Str(Arc::new("Kujo Lang".to_string()))],
        );
        let upper = call_native_function(
            &mut interpreter,
            "upper",
            &[Value::Str(Arc::new("Kujo Lang".to_string()))],
        );
        assert!(matches!(to_upper, Value::Str(s) if s.as_ref() == "KUJO LANG"));
        assert!(matches!(upper, Value::Str(s) if s.as_ref() == "KUJO LANG"));

        let to_lower = call_native_function(
            &mut interpreter,
            "to_lower",
            &[Value::Str(Arc::new("Kujo Lang".to_string()))],
        );
        let lower = call_native_function(
            &mut interpreter,
            "lower",
            &[Value::Str(Arc::new("Kujo Lang".to_string()))],
        );
        assert!(matches!(to_lower, Value::Str(s) if s.as_ref() == "kujo lang"));
        assert!(matches!(lower, Value::Str(s) if s.as_ref() == "kujo lang"));

        let replace_str = call_native_function(
            &mut interpreter,
            "replace_str",
            &[
                Value::Str(Arc::new("kujo-lang-2026".to_string())),
                Value::Str(Arc::new("-".to_string())),
                Value::Str(Arc::new("_".to_string())),
            ],
        );
        let replace = call_native_function(
            &mut interpreter,
            "replace",
            &[
                Value::Str(Arc::new("kujo-lang-2026".to_string())),
                Value::Str(Arc::new("-".to_string())),
                Value::Str(Arc::new("_".to_string())),
            ],
        );
        assert!(matches!(replace_str, Value::Str(s) if s.as_ref() == "kujo_lang_2026"));
        assert!(matches!(replace, Value::Str(s) if s.as_ref() == "kujo_lang_2026"));

        let type_value = call_native_function(&mut interpreter, "type", &[Value::Int(42)]);
        let type_alias = call_native_function(&mut interpreter, "type_of", &[Value::Int(42)]);
        assert!(matches!(type_value, Value::Str(s) if s.as_ref() == "int"));
        assert!(matches!(type_alias, Value::Str(s) if s.as_ref() == "int"));

        let push = call_native_function(
            &mut interpreter,
            "push",
            &[Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)])), Value::Int(3)],
        );
        let append = call_native_function(
            &mut interpreter,
            "append",
            &[Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)])), Value::Int(3)],
        );
        assert!(
            matches!(push, Value::Array(values) if values.len() == 3 && matches!(&values[0], Value::Int(1)) && matches!(&values[1], Value::Int(2)) && matches!(&values[2], Value::Int(3)))
        );
        assert!(
            matches!(append, Value::Array(values) if values.len() == 3 && matches!(&values[0], Value::Int(1)) && matches!(&values[1], Value::Int(2)) && matches!(&values[2], Value::Int(3)))
        );
    }

    #[test]
    fn test_release_hardening_string_utility_behavior_and_error_contracts() {
        let mut interpreter = Interpreter::new();

        let starts_with_true = call_native_function(
            &mut interpreter,
            "starts_with",
            &[
                Value::Str(Arc::new("kujo-lang".to_string())),
                Value::Str(Arc::new("kujo".to_string())),
            ],
        );
        assert!(matches!(starts_with_true, Value::Bool(true)));

        let starts_with_false = call_native_function(
            &mut interpreter,
            "starts_with",
            &[
                Value::Str(Arc::new("kujo-lang".to_string())),
                Value::Str(Arc::new("lang".to_string())),
            ],
        );
        assert!(matches!(starts_with_false, Value::Bool(false)));

        let starts_with_invalid = call_native_function(
            &mut interpreter,
            "starts_with",
            &[Value::Str(Arc::new("kujo-lang".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(starts_with_invalid, Value::Error(message) if message.contains("starts_with() requires string and prefix string arguments"))
        );

        let ends_with_true = call_native_function(
            &mut interpreter,
            "ends_with",
            &[
                Value::Str(Arc::new("kujo-lang".to_string())),
                Value::Str(Arc::new("lang".to_string())),
            ],
        );
        assert!(matches!(ends_with_true, Value::Bool(true)));

        let ends_with_false = call_native_function(
            &mut interpreter,
            "ends_with",
            &[
                Value::Str(Arc::new("kujo-lang".to_string())),
                Value::Str(Arc::new("kujo".to_string())),
            ],
        );
        assert!(matches!(ends_with_false, Value::Bool(false)));

        let ends_with_invalid = call_native_function(
            &mut interpreter,
            "ends_with",
            &[Value::Str(Arc::new("kujo-lang".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(ends_with_invalid, Value::Error(message) if message.contains("ends_with() requires string and suffix string arguments"))
        );

        let repeat_ok = call_native_function(
            &mut interpreter,
            "repeat",
            &[Value::Str(Arc::new("ru".to_string())), Value::Int(3)],
        );
        assert!(matches!(repeat_ok, Value::Str(value) if value.as_ref() == "rururu"));

        let repeat_missing = call_native_function(
            &mut interpreter,
            "repeat",
            &[Value::Str(Arc::new("ru".to_string()))],
        );
        assert!(
            matches!(repeat_missing, Value::Error(message) if message.contains("repeat expects 2 arguments, got 1"))
        );

        let char_at_ok = call_native_function(
            &mut interpreter,
            "char_at",
            &[Value::Str(Arc::new("kujo".to_string())), Value::Int(2)],
        );
        assert!(matches!(char_at_ok, Value::Str(value) if value.as_ref() == "j"));

        let char_at_oob = call_native_function(
            &mut interpreter,
            "char_at",
            &[Value::Str(Arc::new("kujo".to_string())), Value::Int(99)],
        );
        assert!(matches!(char_at_oob, Value::Str(value) if value.as_ref().is_empty()));

        let char_at_missing = call_native_function(
            &mut interpreter,
            "char_at",
            &[Value::Str(Arc::new("kujo".to_string()))],
        );
        assert!(
            matches!(char_at_missing, Value::Error(message) if message.contains("char_at() index must be a number"))
        );

        let is_empty_true = call_native_function(
            &mut interpreter,
            "is_empty",
            &[Value::Str(Arc::new(String::new()))],
        );
        assert!(matches!(is_empty_true, Value::Bool(true)));

        let is_empty_false = call_native_function(
            &mut interpreter,
            "is_empty",
            &[Value::Str(Arc::new("kujo".to_string()))],
        );
        assert!(matches!(is_empty_false, Value::Bool(false)));

        let is_empty_missing = call_native_function(&mut interpreter, "is_empty", &[]);
        assert!(
            matches!(is_empty_missing, Value::Error(message) if message.contains("is_empty() requires a string argument"))
        );

        let count_chars_ok = call_native_function(
            &mut interpreter,
            "count_chars",
            &[Value::Str(Arc::new("kujo".to_string()))],
        );
        assert!(matches!(count_chars_ok, Value::Int(4)));

        let count_chars_unicode = call_native_function(
            &mut interpreter,
            "count_chars",
            &[Value::Str(Arc::new("🔥a".to_string()))],
        );
        assert!(matches!(count_chars_unicode, Value::Int(2)));

        let count_chars_missing = call_native_function(&mut interpreter, "count_chars", &[]);
        assert!(
            matches!(count_chars_missing, Value::Error(message) if message.contains("count_chars() requires a string argument"))
        );
    }

    #[test]
    fn test_release_hardening_string_transform_and_tokenization_contracts() {
        let mut interpreter = Interpreter::new();

        let substring_ok = call_native_function(
            &mut interpreter,
            "substring",
            &[Value::Str(Arc::new("kujo-language".to_string())), Value::Int(0), Value::Int(4)],
        );
        assert!(matches!(substring_ok, Value::Str(value) if value.as_ref() == "kujo"));

        let substring_missing = call_native_function(
            &mut interpreter,
            "substring",
            &[Value::Str(Arc::new("kujo-language".to_string()))],
        );
        assert!(
            matches!(substring_missing, Value::Error(message) if message.contains("substring expects 3 arguments, got 1"))
        );

        let capitalize_ok = call_native_function(
            &mut interpreter,
            "capitalize",
            &[Value::Str(Arc::new("kujo language".to_string()))],
        );
        assert!(matches!(capitalize_ok, Value::Str(value) if value.as_ref() == "Kujo language"));

        let capitalize_missing = call_native_function(&mut interpreter, "capitalize", &[]);
        assert!(
            matches!(capitalize_missing, Value::Error(message) if message.contains("capitalize expects 1 arguments, got 0"))
        );

        let trim_ok = call_native_function(
            &mut interpreter,
            "trim",
            &[Value::Str(Arc::new("  kujo  ".to_string()))],
        );
        assert!(matches!(trim_ok, Value::Str(value) if value.as_ref() == "kujo"));

        let trim_start_ok = call_native_function(
            &mut interpreter,
            "trim_start",
            &[Value::Str(Arc::new("  kujo".to_string()))],
        );
        assert!(matches!(trim_start_ok, Value::Str(value) if value.as_ref() == "kujo"));

        let trim_end_ok = call_native_function(
            &mut interpreter,
            "trim_end",
            &[Value::Str(Arc::new("kujo  ".to_string()))],
        );
        assert!(matches!(trim_end_ok, Value::Str(value) if value.as_ref() == "kujo"));

        let trim_missing = call_native_function(&mut interpreter, "trim", &[]);
        assert!(
            matches!(trim_missing, Value::Error(message) if message.contains("trim expects 1 arguments, got 0"))
        );

        let split_ok = call_native_function(
            &mut interpreter,
            "split",
            &[Value::Str(Arc::new("a,b,c".to_string())), Value::Str(Arc::new(",".to_string()))],
        );
        assert!(matches!(split_ok, Value::Array(parts) if parts.len() == 3
            && matches!(&parts[0], Value::Str(s) if s.as_ref() == "a")
            && matches!(&parts[1], Value::Str(s) if s.as_ref() == "b")
            && matches!(&parts[2], Value::Str(s) if s.as_ref() == "c")));

        let split_missing = call_native_function(
            &mut interpreter,
            "split",
            &[Value::Str(Arc::new("a,b,c".to_string()))],
        );
        assert!(
            matches!(split_missing, Value::Error(message) if message.contains("split expects 2 arguments, got 1"))
        );

        let join_ok = call_native_function(
            &mut interpreter,
            "join",
            &[
                Value::Array(Arc::new(vec![
                    Value::Str(Arc::new("kujo".to_string())),
                    Value::Int(2026),
                    Value::Bool(true),
                ])),
                Value::Str(Arc::new("-".to_string())),
            ],
        );
        assert!(matches!(join_ok, Value::Str(value) if value.as_ref() == "kujo-2026-true"));

        let join_missing = call_native_function(
            &mut interpreter,
            "join",
            &[Value::Array(Arc::new(vec![Value::Str(Arc::new("kujo".to_string()))]))],
        );
        assert!(
            matches!(join_missing, Value::Error(message) if message.contains("join expects 2 arguments, got 1"))
        );
    }

    #[test]
    fn test_release_hardening_advanced_string_methods_contracts() {
        let mut interpreter = Interpreter::new();

        let pad_left_ok = call_native_function(
            &mut interpreter,
            "pad_left",
            &[
                Value::Str(Arc::new("kujo".to_string())),
                Value::Int(6),
                Value::Str(Arc::new("0".to_string())),
            ],
        );
        assert!(matches!(pad_left_ok, Value::Str(value) if value.as_ref() == "00kujo"));

        let pad_left_width_short = call_native_function(
            &mut interpreter,
            "pad_left",
            &[
                Value::Str(Arc::new("kujo".to_string())),
                Value::Int(2),
                Value::Str(Arc::new("0".to_string())),
            ],
        );
        assert!(matches!(pad_left_width_short, Value::Str(value) if value.as_ref() == "kujo"));

        let pad_right_ok = call_native_function(
            &mut interpreter,
            "pad_right",
            &[
                Value::Str(Arc::new("kujo".to_string())),
                Value::Int(6),
                Value::Str(Arc::new(".".to_string())),
            ],
        );
        assert!(matches!(pad_right_ok, Value::Str(value) if value.as_ref() == "kujo.."));

        let pad_start_ok = call_native_function(
            &mut interpreter,
            "pad_start",
            &[
                Value::Str(Arc::new("kujo".to_string())),
                Value::Int(6),
                Value::Str(Arc::new("0".to_string())),
            ],
        );
        assert!(matches!(pad_start_ok, Value::Str(value) if value.as_ref() == "00kujo"));

        let pad_end_ok = call_native_function(
            &mut interpreter,
            "pad_end",
            &[
                Value::Str(Arc::new("kujo".to_string())),
                Value::Int(6),
                Value::Str(Arc::new(".".to_string())),
            ],
        );
        assert!(matches!(pad_end_ok, Value::Str(value) if value.as_ref() == "kujo.."));

        let pad_left_missing = call_native_function(
            &mut interpreter,
            "pad_left",
            &[Value::Str(Arc::new("kujo".to_string()))],
        );
        assert!(
            matches!(pad_left_missing, Value::Error(message) if message.contains("pad_left expects 3 arguments"))
        );

        let pad_right_missing = call_native_function(
            &mut interpreter,
            "pad_right",
            &[Value::Str(Arc::new("kujo".to_string()))],
        );
        assert!(
            matches!(pad_right_missing, Value::Error(message) if message.contains("pad_right expects 3 arguments"))
        );

        let pad_start_missing = call_native_function(
            &mut interpreter,
            "pad_start",
            &[Value::Str(Arc::new("kujo".to_string()))],
        );
        assert!(
            matches!(pad_start_missing, Value::Error(message) if message.contains("pad_left expects 3 arguments"))
        );

        let pad_end_missing = call_native_function(
            &mut interpreter,
            "pad_end",
            &[Value::Str(Arc::new("kujo".to_string()))],
        );
        assert!(
            matches!(pad_end_missing, Value::Error(message) if message.contains("pad_right expects 3 arguments"))
        );

        let lines_ok = call_native_function(
            &mut interpreter,
            "lines",
            &[Value::Str(Arc::new("first\nsecond\n".to_string()))],
        );
        assert!(matches!(lines_ok, Value::Array(parts) if parts.len() == 2
            && matches!(&parts[0], Value::Str(s) if s.as_ref() == "first")
            && matches!(&parts[1], Value::Str(s) if s.as_ref() == "second")));

        let lines_invalid = call_native_function(&mut interpreter, "lines", &[Value::Int(1)]);
        assert!(
            matches!(lines_invalid, Value::Error(message) if message.contains("lines() requires a string argument"))
        );

        let words_ok = call_native_function(
            &mut interpreter,
            "words",
            &[Value::Str(Arc::new("kujo   language\t2026".to_string()))],
        );
        assert!(matches!(words_ok, Value::Array(parts) if parts.len() == 3
            && matches!(&parts[0], Value::Str(s) if s.as_ref() == "kujo")
            && matches!(&parts[1], Value::Str(s) if s.as_ref() == "language")
            && matches!(&parts[2], Value::Str(s) if s.as_ref() == "2026")));

        let words_invalid = call_native_function(&mut interpreter, "words", &[Value::Bool(true)]);
        assert!(
            matches!(words_invalid, Value::Error(message) if message.contains("words() requires a string argument"))
        );

        let reverse_ok = call_native_function(
            &mut interpreter,
            "str_reverse",
            &[Value::Str(Arc::new("ab🔥".to_string()))],
        );
        assert!(matches!(reverse_ok, Value::Str(value) if value.as_ref() == "🔥ba"));

        let reverse_invalid = call_native_function(&mut interpreter, "str_reverse", &[Value::Null]);
        assert!(
            matches!(reverse_invalid, Value::Error(message) if message.contains("str_reverse() requires a string argument"))
        );

        let slugify_ok = call_native_function(
            &mut interpreter,
            "slugify",
            &[Value::Str(Arc::new("Kujo Lang__2026!".to_string()))],
        );
        assert!(matches!(slugify_ok, Value::Str(value) if value.as_ref() == "kujo-lang-2026"));

        let slugify_invalid =
            call_native_function(&mut interpreter, "slugify", &[Value::Int(2026)]);
        assert!(
            matches!(slugify_invalid, Value::Error(message) if message.contains("slugify() requires a string argument"))
        );

        let truncate_ok = call_native_function(
            &mut interpreter,
            "truncate",
            &[
                Value::Str(Arc::new("Kujo Language".to_string())),
                Value::Int(8),
                Value::Str(Arc::new("...".to_string())),
            ],
        );
        assert!(matches!(truncate_ok, Value::Str(value) if value.as_ref() == "Kujo ..."));

        let truncate_tiny_limit = call_native_function(
            &mut interpreter,
            "truncate",
            &[
                Value::Str(Arc::new("Kujo".to_string())),
                Value::Int(2),
                Value::Str(Arc::new("...".to_string())),
            ],
        );
        assert!(matches!(truncate_tiny_limit, Value::Str(value) if value.as_ref() == "..."));

        let truncate_missing = call_native_function(
            &mut interpreter,
            "truncate",
            &[Value::Str(Arc::new("Kujo".to_string())), Value::Int(2)],
        );
        assert!(
            matches!(truncate_missing, Value::Error(message) if message.contains("truncate() requires 3 arguments"))
        );

        let camel_ok = call_native_function(
            &mut interpreter,
            "to_camel_case",
            &[Value::Str(Arc::new("Kujo language_tools".to_string()))],
        );
        assert!(matches!(camel_ok, Value::Str(value) if value.as_ref() == "kujoLanguageTools"));

        let snake_ok = call_native_function(
            &mut interpreter,
            "to_snake_case",
            &[Value::Str(Arc::new("KujoLanguageTools".to_string()))],
        );
        assert!(matches!(snake_ok, Value::Str(value) if value.as_ref() == "kujo_language_tools"));

        let kebab_ok = call_native_function(
            &mut interpreter,
            "to_kebab_case",
            &[Value::Str(Arc::new("KujoLanguageTools".to_string()))],
        );
        assert!(matches!(kebab_ok, Value::Str(value) if value.as_ref() == "kujo-language-tools"));

        let camel_invalid = call_native_function(
            &mut interpreter,
            "to_camel_case",
            &[Value::Array(Arc::new(vec![]))],
        );
        assert!(
            matches!(camel_invalid, Value::Error(message) if message.contains("to_camel_case() requires a string argument"))
        );

        let snake_invalid =
            call_native_function(&mut interpreter, "to_snake_case", &[Value::Int(1)]);
        assert!(
            matches!(snake_invalid, Value::Error(message) if message.contains("to_snake_case() requires a string argument"))
        );

        let kebab_invalid =
            call_native_function(&mut interpreter, "to_kebab_case", &[Value::Bool(true)]);
        assert!(
            matches!(kebab_invalid, Value::Error(message) if message.contains("to_kebab_case() requires a string argument"))
        );
    }

    #[test]
    fn test_release_hardening_system_random_and_time_contracts() {
        let mut interpreter = Interpreter::new();

        let random_value = call_native_function(&mut interpreter, "random", &[]);
        assert!(matches!(random_value, Value::Float(value) if (0.0..=1.0).contains(&value)));

        let random_int_missing = call_native_function(&mut interpreter, "random_int", &[]);
        assert!(
            matches!(random_int_missing, Value::Error(message) if message.contains("random_int() expects 2 arguments"))
        );

        let random_extra = call_native_function(&mut interpreter, "random", &[Value::Int(1)]);
        assert!(
            matches!(random_extra, Value::Error(message) if message.contains("random() expects 0 arguments"))
        );

        let random_int_value =
            call_native_function(&mut interpreter, "random_int", &[Value::Int(3), Value::Int(7)]);
        assert!(matches!(random_int_value, Value::Int(value) if (3..=7).contains(&value)));

        let random_int_extra = call_native_function(
            &mut interpreter,
            "random_int",
            &[Value::Int(3), Value::Int(7), Value::Int(9)],
        );
        assert!(
            matches!(random_int_extra, Value::Error(message) if message.contains("random_int() expects 2 arguments"))
        );

        let random_choice_missing = call_native_function(&mut interpreter, "random_choice", &[]);
        assert!(
            matches!(random_choice_missing, Value::Error(message) if message.contains("random_choice() expects 1 argument"))
        );

        let random_choice_value = call_native_function(
            &mut interpreter,
            "random_choice",
            &[Value::Array(Arc::new(vec![Value::Int(10), Value::Int(20), Value::Int(30)]))],
        );
        assert!(matches!(random_choice_value, Value::Int(10) | Value::Int(20) | Value::Int(30)));

        let uuid_value = call_native_function(&mut interpreter, "uuid_v4", &[]);
        assert!(matches!(uuid_value, Value::Str(value) if value.len() == 36));

        let random_id_value =
            call_native_function(&mut interpreter, "random_id", &[Value::Int(10)]);
        assert!(matches!(random_id_value, Value::Str(value) if value.len() == 10));

        let now_utc_value = call_native_function(&mut interpreter, "now_utc", &[]);
        assert!(
            matches!(now_utc_value, Value::Str(value) if value.ends_with('Z') && value.contains('T'))
        );

        let now_unix_value = call_native_function(&mut interpreter, "now_unix", &[]);
        assert!(matches!(now_unix_value, Value::Int(value) if value > 0));

        let now_utc_seconds_value = call_native_function(&mut interpreter, "now_utc_seconds", &[]);
        assert!(matches!(now_utc_seconds_value, Value::Int(value) if value > 0));

        let set_seed_missing = call_native_function(&mut interpreter, "set_random_seed", &[]);
        assert!(
            matches!(set_seed_missing, Value::Error(message) if message.contains("set_random_seed() expects 1 argument"))
        );

        let clear_seed_extra =
            call_native_function(&mut interpreter, "clear_random_seed", &[Value::Int(1)]);
        assert!(
            matches!(clear_seed_extra, Value::Error(message) if message.contains("clear_random_seed() expects 0 arguments"))
        );

        let clear_seed_result = call_native_function(&mut interpreter, "clear_random_seed", &[]);
        assert!(matches!(clear_seed_result, Value::Null));

        let seed_result =
            call_native_function(&mut interpreter, "set_random_seed", &[Value::Int(12345)]);
        assert!(matches!(seed_result, Value::Null));

        let seeded_random_a = call_native_function(&mut interpreter, "random", &[]);
        let seeded_random_int_a =
            call_native_function(&mut interpreter, "random_int", &[Value::Int(1), Value::Int(100)]);
        let seeded_choice_a = call_native_function(
            &mut interpreter,
            "random_choice",
            &[Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]))],
        );

        let reseed_result =
            call_native_function(&mut interpreter, "set_random_seed", &[Value::Int(12345)]);
        assert!(matches!(reseed_result, Value::Null));

        let seeded_random_b = call_native_function(&mut interpreter, "random", &[]);
        let seeded_random_int_b =
            call_native_function(&mut interpreter, "random_int", &[Value::Int(1), Value::Int(100)]);
        let seeded_choice_b = call_native_function(
            &mut interpreter,
            "random_choice",
            &[Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]))],
        );

        match (seeded_random_a, seeded_random_b) {
            (Value::Float(a), Value::Float(b)) => assert!((a - b).abs() < f64::EPSILON),
            (left, right) => {
                panic!("Expected seeded random floats, got {:?} and {:?}", left, right)
            }
        }
        match (seeded_random_int_a, seeded_random_int_b) {
            (Value::Int(a), Value::Int(b)) => assert_eq!(a, b),
            (left, right) => {
                panic!("Expected seeded random_int ints, got {:?} and {:?}", left, right)
            }
        }
        match (seeded_choice_a, seeded_choice_b) {
            (Value::Int(a), Value::Int(b)) => assert_eq!(a, b),
            (left, right) => {
                panic!("Expected seeded random_choice ints, got {:?} and {:?}", left, right)
            }
        }

        let now_value = call_native_function(&mut interpreter, "now", &[]);
        assert!(matches!(now_value, Value::Float(value) if value > 0.0));

        let current_timestamp_value =
            call_native_function(&mut interpreter, "current_timestamp", &[]);
        assert!(matches!(current_timestamp_value, Value::Int(value) if value > 0));

        let performance_now_start = call_native_function(&mut interpreter, "performance_now", &[]);
        let performance_now_end = call_native_function(&mut interpreter, "performance_now", &[]);
        match (performance_now_start, performance_now_end) {
            (Value::Float(start), Value::Float(end)) => assert!(end >= start),
            (left, right) => {
                panic!("Expected performance_now floats, got {:?} and {:?}", left, right)
            }
        }

        let time_us_start = call_native_function(&mut interpreter, "time_us", &[]);
        let time_us_end = call_native_function(&mut interpreter, "time_us", &[]);
        match (time_us_start, time_us_end) {
            (Value::Float(start), Value::Float(end)) => assert!(end >= start),
            (left, right) => panic!("Expected time_us floats, got {:?} and {:?}", left, right),
        }

        let time_ns_start = call_native_function(&mut interpreter, "time_ns", &[]);
        let time_ns_end = call_native_function(&mut interpreter, "time_ns", &[]);
        match (time_ns_start, time_ns_end) {
            (Value::Float(start), Value::Float(end)) => assert!(end >= start),
            (left, right) => panic!("Expected time_ns floats, got {:?} and {:?}", left, right),
        }

        let format_duration_seconds =
            call_native_function(&mut interpreter, "format_duration", &[Value::Float(1500.0)]);
        assert!(matches!(format_duration_seconds, Value::Str(value) if value.as_ref() == "1.50s"));

        let format_duration_ms =
            call_native_function(&mut interpreter, "format_duration", &[Value::Float(2.5)]);
        assert!(matches!(format_duration_ms, Value::Str(value) if value.as_ref() == "2.50ms"));

        let format_duration_missing =
            call_native_function(&mut interpreter, "format_duration", &[]);
        assert!(
            matches!(format_duration_missing, Value::Error(message) if message.contains("format_duration() expects 1 argument"))
        );

        let elapsed_ok = call_native_function(
            &mut interpreter,
            "elapsed",
            &[Value::Float(10.0), Value::Float(15.5)],
        );
        assert!(matches!(elapsed_ok, Value::Float(value) if (value - 5.5).abs() < 1e-12));

        let elapsed_missing = call_native_function(&mut interpreter, "elapsed", &[]);
        assert!(
            matches!(elapsed_missing, Value::Error(message) if message.contains("elapsed() expects 2 arguments"))
        );

        let format_date_epoch = call_native_function(
            &mut interpreter,
            "format_date",
            &[Value::Float(0.0), Value::Str(Arc::new("YYYY-MM-DD".to_string()))],
        );
        assert!(matches!(format_date_epoch, Value::Str(value) if value.as_ref() == "1970-01-01"));

        let format_date_missing = call_native_function(&mut interpreter, "format_date", &[]);
        assert!(
            matches!(format_date_missing, Value::Error(message) if message.contains("format_date() expects 2 arguments"))
        );

        let parse_date_epoch = call_native_function(
            &mut interpreter,
            "parse_date",
            &[
                Value::Str(Arc::new("1970-01-01".to_string())),
                Value::Str(Arc::new("YYYY-MM-DD".to_string())),
            ],
        );
        assert!(matches!(parse_date_epoch, Value::Float(value) if value == 0.0));

        let parse_date_invalid = call_native_function(
            &mut interpreter,
            "parse_date",
            &[
                Value::Str(Arc::new("not-a-date".to_string())),
                Value::Str(Arc::new("YYYY-MM-DD".to_string())),
            ],
        );
        assert!(matches!(parse_date_invalid, Value::ErrorObject { .. }));

        let parse_date_missing = call_native_function(&mut interpreter, "parse_date", &[]);
        assert!(
            matches!(parse_date_missing, Value::Error(message) if message.contains("parse_date() expects 2 arguments"))
        );

        let parse_date_extra = call_native_function(
            &mut interpreter,
            "parse_date",
            &[
                Value::Str(Arc::new("1970-01-01".to_string())),
                Value::Str(Arc::new("YYYY-MM-DD".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(parse_date_extra, Value::Error(message) if message.contains("parse_date() expects 2 arguments"))
        );
    }

    #[test]
    fn test_release_hardening_math_behavior_and_error_contracts() {
        let mut interpreter = Interpreter::new();

        let abs_ok = call_native_function(&mut interpreter, "abs", &[Value::Int(-42)]);
        assert!(matches!(abs_ok, Value::Float(value) if (value - 42.0).abs() < 1e-12));

        let sqrt_ok = call_native_function(&mut interpreter, "sqrt", &[Value::Int(49)]);
        assert!(matches!(sqrt_ok, Value::Float(value) if (value - 7.0).abs() < 1e-12));

        let pow_ok = call_native_function(&mut interpreter, "pow", &[Value::Int(2), Value::Int(8)]);
        assert!(matches!(pow_ok, Value::Float(value) if (value - 256.0).abs() < 1e-12));

        let floor_ok = call_native_function(&mut interpreter, "floor", &[Value::Float(3.9)]);
        assert!(matches!(floor_ok, Value::Float(value) if (value - 3.0).abs() < 1e-12));

        let ceil_ok = call_native_function(&mut interpreter, "ceil", &[Value::Float(3.1)]);
        assert!(matches!(ceil_ok, Value::Float(value) if (value - 4.0).abs() < 1e-12));

        let round_ok = call_native_function(&mut interpreter, "round", &[Value::Float(3.5)]);
        assert!(matches!(round_ok, Value::Float(value) if (value - 4.0).abs() < 1e-12));

        let min_ok = call_native_function(&mut interpreter, "min", &[Value::Int(3), Value::Int(7)]);
        assert!(matches!(min_ok, Value::Float(value) if (value - 3.0).abs() < 1e-12));

        let max_ok = call_native_function(&mut interpreter, "max", &[Value::Int(3), Value::Int(7)]);
        assert!(matches!(max_ok, Value::Float(value) if (value - 7.0).abs() < 1e-12));

        let sin_ok = call_native_function(&mut interpreter, "sin", &[Value::Int(0)]);
        assert!(matches!(sin_ok, Value::Float(value) if value.abs() < 1e-12));

        let cos_ok = call_native_function(&mut interpreter, "cos", &[Value::Int(0)]);
        assert!(matches!(cos_ok, Value::Float(value) if (value - 1.0).abs() < 1e-12));

        let tan_ok = call_native_function(&mut interpreter, "tan", &[Value::Int(0)]);
        assert!(matches!(tan_ok, Value::Float(value) if value.abs() < 1e-12));

        let log_ok =
            call_native_function(&mut interpreter, "log", &[Value::Float(std::f64::consts::E)]);
        assert!(matches!(log_ok, Value::Float(value) if (value - 1.0).abs() < 1e-12));

        let exp_ok = call_native_function(&mut interpreter, "exp", &[Value::Int(1)]);
        assert!(
            matches!(exp_ok, Value::Float(value) if (value - std::f64::consts::E).abs() < 1e-12)
        );

        let abs_missing = call_native_function(&mut interpreter, "abs", &[]);
        assert!(
            matches!(abs_missing, Value::Error(message) if message.contains("abs() expects 1 argument"))
        );

        let abs_extra =
            call_native_function(&mut interpreter, "abs", &[Value::Int(-42), Value::Int(1)]);
        assert!(
            matches!(abs_extra, Value::Error(message) if message.contains("abs() expects 1 argument"))
        );

        let abs_invalid_type = call_native_function(
            &mut interpreter,
            "abs",
            &[Value::Str(Arc::new("invalid".to_string()))],
        );
        assert!(
            matches!(abs_invalid_type, Value::Error(message) if message.contains("abs() expects numeric argument"))
        );

        let pow_missing = call_native_function(&mut interpreter, "pow", &[Value::Int(2)]);
        assert!(
            matches!(pow_missing, Value::Error(message) if message.contains("pow() expects 2 arguments"))
        );

        let pow_extra = call_native_function(
            &mut interpreter,
            "pow",
            &[Value::Int(2), Value::Int(8), Value::Int(1)],
        );
        assert!(
            matches!(pow_extra, Value::Error(message) if message.contains("pow() expects 2 arguments"))
        );

        let pow_invalid_type = call_native_function(
            &mut interpreter,
            "pow",
            &[Value::Int(2), Value::Str(Arc::new("bad".to_string()))],
        );
        assert!(
            matches!(pow_invalid_type, Value::Error(message) if message.contains("pow() expects numeric arguments"))
        );

        let sqrt_domain_error = call_native_function(&mut interpreter, "sqrt", &[Value::Int(-1)]);
        assert!(
            matches!(sqrt_domain_error, Value::Error(message) if message.contains("sqrt() domain error"))
        );

        let log_domain_error = call_native_function(&mut interpreter, "log", &[Value::Int(0)]);
        assert!(
            matches!(log_domain_error, Value::Error(message) if message.contains("log() domain error"))
        );

        let min_extra = call_native_function(
            &mut interpreter,
            "min",
            &[Value::Int(1), Value::Int(2), Value::Int(3)],
        );
        assert!(
            matches!(min_extra, Value::Error(message) if message.contains("min() expects 2 arguments"))
        );

        let max_extra = call_native_function(
            &mut interpreter,
            "max",
            &[Value::Int(1), Value::Int(2), Value::Int(3)],
        );
        assert!(
            matches!(max_extra, Value::Error(message) if message.contains("max() expects 2 arguments"))
        );
    }

    #[test]
    fn test_release_hardening_collections_and_format_contracts() {
        let mut interpreter = Interpreter::new();

        let range_one_arg = call_native_function(&mut interpreter, "range", &[Value::Int(3)]);
        assert!(matches!(range_one_arg, Value::Array(values) if values.len() == 3
            && matches!(&values[0], Value::Int(0))
            && matches!(&values[1], Value::Int(1))
            && matches!(&values[2], Value::Int(2))));

        let range_two_arg =
            call_native_function(&mut interpreter, "range", &[Value::Int(2), Value::Int(5)]);
        assert!(matches!(range_two_arg, Value::Array(values) if values.len() == 3
            && matches!(&values[0], Value::Int(2))
            && matches!(&values[1], Value::Int(3))
            && matches!(&values[2], Value::Int(4))));

        let range_invalid = call_native_function(
            &mut interpreter,
            "range",
            &[Value::Str(Arc::new("bad".to_string()))],
        );
        assert!(
            matches!(range_invalid, Value::Error(message) if message.contains("range() requires numeric arguments"))
        );

        let mut left_dict = crate::interpreter::DictMap::default();
        left_dict.insert("b".into(), Value::Int(2));
        left_dict.insert("a".into(), Value::Int(1));
        let mut right_dict = crate::interpreter::DictMap::default();
        right_dict.insert("b".into(), Value::Int(20));
        right_dict.insert("c".into(), Value::Int(3));

        let dict_value = Value::Dict(Arc::new(left_dict.clone()));

        let keys_result = call_native_function(&mut interpreter, "keys", &[dict_value.clone()]);
        assert!(matches!(keys_result, Value::Array(keys) if keys.len() == 2
            && matches!(&keys[0], Value::Str(k) if k.as_ref() == "a")
            && matches!(&keys[1], Value::Str(k) if k.as_ref() == "b")));

        let values_result = call_native_function(&mut interpreter, "values", &[dict_value.clone()]);
        assert!(matches!(values_result, Value::Array(values) if values.len() == 2
            && matches!(&values[0], Value::Int(1))
            && matches!(&values[1], Value::Int(2))));

        let items_result = call_native_function(&mut interpreter, "items", &[dict_value.clone()]);
        assert!(matches!(items_result, Value::Array(items) if items.len() == 2
            && matches!(&items[0], Value::Array(pair) if pair.len() == 2
                && matches!(&pair[0], Value::Str(k) if k.as_ref() == "a")
                && matches!(&pair[1], Value::Int(1)))
            && matches!(&items[1], Value::Array(pair) if pair.len() == 2
                && matches!(&pair[0], Value::Str(k) if k.as_ref() == "b")
                && matches!(&pair[1], Value::Int(2)))));

        let has_key_true = call_native_function(
            &mut interpreter,
            "has_key",
            &[dict_value.clone(), Value::Str(Arc::new("a".to_string()))],
        );
        assert!(matches!(has_key_true, Value::Int(1)));

        let has_key_false = call_native_function(
            &mut interpreter,
            "has_key",
            &[dict_value.clone(), Value::Str(Arc::new("missing".to_string()))],
        );
        assert!(matches!(has_key_false, Value::Int(0)));

        let get_found = call_native_function(
            &mut interpreter,
            "get",
            &[dict_value.clone(), Value::Str(Arc::new("b".to_string()))],
        );
        assert!(matches!(get_found, Value::Int(2)));

        let get_missing_with_default = call_native_function(
            &mut interpreter,
            "get",
            &[dict_value.clone(), Value::Str(Arc::new("missing".to_string())), Value::Int(99)],
        );
        assert!(matches!(get_missing_with_default, Value::Int(99)));

        let get_default_found = call_native_function(
            &mut interpreter,
            "get_default",
            &[dict_value.clone(), Value::Str(Arc::new("a".to_string())), Value::Int(100)],
        );
        assert!(matches!(get_default_found, Value::Int(1)));

        let get_default_missing = call_native_function(
            &mut interpreter,
            "get_default",
            &[dict_value.clone(), Value::Str(Arc::new("missing".to_string())), Value::Int(100)],
        );
        assert!(matches!(get_default_missing, Value::Int(100)));

        let get_default_bad_shape =
            call_native_function(&mut interpreter, "get_default", &[dict_value.clone()]);
        assert!(
            matches!(get_default_bad_shape, Value::Error(message) if message.contains("get_default() requires 3 arguments"))
        );

        let merge_result = call_native_function(
            &mut interpreter,
            "merge",
            &[Value::Dict(Arc::new(left_dict.clone())), Value::Dict(Arc::new(right_dict.clone()))],
        );
        assert!(matches!(merge_result, Value::Dict(merged)
            if matches!(merged.get("a"), Some(Value::Int(1)))
            && matches!(merged.get("b"), Some(Value::Int(20)))
            && matches!(merged.get("c"), Some(Value::Int(3)) )));

        let update_result = call_native_function(
            &mut interpreter,
            "update",
            &[Value::Dict(Arc::new(left_dict.clone())), Value::Dict(Arc::new(right_dict.clone()))],
        );
        assert!(matches!(update_result, Value::Dict(updated)
            if matches!(updated.get("a"), Some(Value::Int(1)))
            && matches!(updated.get("b"), Some(Value::Int(20)))
            && matches!(updated.get("c"), Some(Value::Int(3)) )));

        let invert_result = call_native_function(
            &mut interpreter,
            "invert",
            &[Value::Dict(Arc::new(left_dict.clone()))],
        );
        assert!(matches!(invert_result, Value::Dict(inverted)
            if matches!(inverted.get("1"), Some(Value::Str(v)) if v.as_ref() == "a")
            && matches!(inverted.get("2"), Some(Value::Str(v)) if v.as_ref() == "b")));

        let invert_bad_shape = call_native_function(&mut interpreter, "invert", &[Value::Int(1)]);
        assert!(
            matches!(invert_bad_shape, Value::Error(message) if message.contains("invert() requires a dict argument"))
        );

        let format_ok = call_native_function(
            &mut interpreter,
            "format",
            &[
                Value::Str(Arc::new("Hello %s, you have %d tasks".to_string())),
                Value::Str(Arc::new("Kujo".to_string())),
                Value::Int(3),
            ],
        );
        assert!(
            matches!(format_ok, Value::Str(result) if result.as_ref() == "Hello Kujo, you have 3 tasks")
        );

        let format_missing_template = call_native_function(&mut interpreter, "format", &[]);
        assert!(
            matches!(format_missing_template, Value::Error(message) if message.contains("format() requires at least 1 argument"))
        );

        let format_bad_template =
            call_native_function(&mut interpreter, "format", &[Value::Int(1)]);
        assert!(
            matches!(format_bad_template, Value::Error(message) if message.contains("format() first argument must be a string"))
        );
    }

    #[test]
    fn test_release_hardening_array_and_higher_order_collection_contracts() {
        let mut interpreter = Interpreter::new();

        let base_array = Value::Array(Arc::new(vec![Value::Int(3), Value::Int(1), Value::Int(2)]));
        let other_array = Value::Array(Arc::new(vec![Value::Int(9), Value::Int(8)]));
        let empty_function = Value::Function(vec![], LeakyFunctionBody::new(Vec::new()), None);

        let strict_arity_cases: Vec<(&str, Vec<Value>, &str)> = vec![
            ("push", vec![base_array.clone()], "expects 2 arguments"),
            ("pop", vec![], "expects 1 argument"),
            ("slice", vec![base_array.clone(), Value::Int(0)], "expects 3 arguments"),
            (
                "concat",
                vec![base_array.clone(), other_array.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            (
                "insert",
                vec![base_array.clone(), Value::Int(1), Value::Int(99), Value::Int(7)],
                "expects 3 arguments",
            ),
            (
                "remove",
                vec![base_array.clone(), Value::Int(1), Value::Int(2)],
                "expects 2 arguments",
            ),
            (
                "remove_at",
                vec![base_array.clone(), Value::Int(0), Value::Int(1)],
                "expects 2 arguments",
            ),
            ("clear", vec![], "expects 1 argument"),
            (
                "map",
                vec![base_array.clone(), empty_function.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            (
                "filter",
                vec![base_array.clone(), empty_function.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            (
                "reduce",
                vec![base_array.clone(), Value::Int(0), empty_function.clone(), Value::Int(1)],
                "expects 3 arguments",
            ),
            (
                "find",
                vec![base_array.clone(), empty_function.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            (
                "any",
                vec![base_array.clone(), empty_function.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            (
                "all",
                vec![base_array.clone(), empty_function.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            ("sort", vec![base_array.clone(), Value::Int(1)], "expects 1 argument"),
            ("reverse", vec![base_array.clone(), Value::Int(1)], "expects 1 argument"),
            ("unique", vec![base_array.clone(), Value::Int(1)], "expects 1 argument"),
            ("sum", vec![base_array.clone(), Value::Int(1)], "expects 1 argument"),
            (
                "chunk",
                vec![base_array.clone(), Value::Int(2), Value::Int(1)],
                "expects 2 arguments",
            ),
            ("flatten", vec![base_array.clone(), Value::Int(1)], "expects 1 argument"),
            (
                "zip",
                vec![base_array.clone(), other_array.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            ("enumerate", vec![base_array.clone(), Value::Int(1)], "expects 1 argument"),
            ("take", vec![base_array.clone(), Value::Int(2), Value::Int(1)], "expects 2 arguments"),
            ("skip", vec![base_array.clone(), Value::Int(2), Value::Int(1)], "expects 2 arguments"),
        ];

        for (name, args, expected_message) in strict_arity_cases {
            let result = call_native_function(&mut interpreter, name, &args);
            assert!(
                matches!(result, Value::Error(ref message) if message.contains(expected_message)),
                "Expected strict-arity contract for {} to contain '{}', got {:?}",
                name,
                expected_message,
                result
            );
        }

        let windows_strict_arity = call_native_function(
            &mut interpreter,
            "windows",
            &[base_array.clone(), Value::Int(2), Value::Int(1)],
        );
        assert!(
            matches!(windows_strict_arity, Value::Error(message) if message.contains("expects 2 arguments"))
        );

        let push_success =
            call_native_function(&mut interpreter, "push", &[base_array.clone(), Value::Int(4)]);
        assert!(matches!(push_success, Value::Array(values) if values.len() == 4
            && matches!(&values[0], Value::Int(3))
            && matches!(&values[1], Value::Int(1))
            && matches!(&values[2], Value::Int(2))
            && matches!(&values[3], Value::Int(4))));

        let sort_success = call_native_function(&mut interpreter, "sort", &[base_array.clone()]);
        assert!(matches!(sort_success, Value::Array(values) if values.len() == 3
            && matches!(&values[0], Value::Int(1))
            && matches!(&values[1], Value::Int(2))
            && matches!(&values[2], Value::Int(3))));

        let reverse_success = call_native_function(
            &mut interpreter,
            "reverse",
            &[Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]))],
        );
        assert!(matches!(reverse_success, Value::Array(values) if values.len() == 3
            && matches!(&values[0], Value::Int(3))
            && matches!(&values[1], Value::Int(2))
            && matches!(&values[2], Value::Int(1))));

        let sum_success = call_native_function(
            &mut interpreter,
            "sum",
            &[Value::Array(Arc::new(vec![Value::Int(2), Value::Int(3), Value::Int(5)]))],
        );
        assert!(matches!(sum_success, Value::Int(10)));

        let chunk_success =
            call_native_function(&mut interpreter, "chunk", &[base_array.clone(), Value::Int(2)]);
        assert!(matches!(chunk_success, Value::Array(chunks) if chunks.len() == 2
            && matches!(&chunks[0], Value::Array(first) if first.len() == 2)
            && matches!(&chunks[1], Value::Array(second) if second.len() == 1)));

        let flatten_success = call_native_function(
            &mut interpreter,
            "flatten",
            &[Value::Array(Arc::new(vec![
                Value::Int(1),
                Value::Array(Arc::new(vec![Value::Int(2), Value::Int(3)])),
            ]))],
        );
        assert!(matches!(flatten_success, Value::Array(values) if values.len() == 3
            && matches!(&values[0], Value::Int(1))
            && matches!(&values[1], Value::Int(2))
            && matches!(&values[2], Value::Int(3))));

        let zip_success = call_native_function(
            &mut interpreter,
            "zip",
            &[base_array.clone(), Value::Array(Arc::new(vec![Value::Int(10), Value::Int(20)]))],
        );
        assert!(matches!(zip_success, Value::Array(pairs) if pairs.len() == 2
            && matches!(&pairs[0], Value::Array(pair) if pair.len() == 2
                && matches!(&pair[0], Value::Int(3))
                && matches!(&pair[1], Value::Int(10)))
            && matches!(&pairs[1], Value::Array(pair) if pair.len() == 2
                && matches!(&pair[0], Value::Int(1))
                && matches!(&pair[1], Value::Int(20)))));

        let enumerate_success = call_native_function(
            &mut interpreter,
            "enumerate",
            &[Value::Array(Arc::new(vec![
                Value::Str(Arc::new("a".to_string())),
                Value::Str(Arc::new("b".to_string())),
            ]))],
        );
        assert!(matches!(enumerate_success, Value::Array(entries) if entries.len() == 2
            && matches!(&entries[0], Value::Array(pair) if pair.len() == 2
                && matches!(&pair[0], Value::Int(0))
                && matches!(&pair[1], Value::Str(v) if v.as_ref() == "a"))
            && matches!(&entries[1], Value::Array(pair) if pair.len() == 2
                && matches!(&pair[0], Value::Int(1))
                && matches!(&pair[1], Value::Str(v) if v.as_ref() == "b"))));

        let take_success =
            call_native_function(&mut interpreter, "take", &[base_array.clone(), Value::Int(2)]);
        assert!(matches!(take_success, Value::Array(values) if values.len() == 2
            && matches!(&values[0], Value::Int(3))
            && matches!(&values[1], Value::Int(1))));

        let skip_success =
            call_native_function(&mut interpreter, "skip", &[base_array.clone(), Value::Int(1)]);
        assert!(matches!(skip_success, Value::Array(values) if values.len() == 2
            && matches!(&values[0], Value::Int(1))
            && matches!(&values[1], Value::Int(2))));

        let windows_success =
            call_native_function(&mut interpreter, "windows", &[base_array.clone(), Value::Int(2)]);
        assert!(matches!(windows_success, Value::Array(values) if values.len() == 2
            && matches!(&values[0], Value::Array(window) if window.len() == 2)
            && matches!(&values[1], Value::Array(window) if window.len() == 2)));

        let map_success = call_native_function(
            &mut interpreter,
            "map",
            &[base_array.clone(), empty_function.clone()],
        );
        assert!(matches!(map_success, Value::Array(values) if values.len() == 3));

        let filter_success = call_native_function(
            &mut interpreter,
            "filter",
            &[base_array.clone(), empty_function.clone()],
        );
        assert!(matches!(filter_success, Value::Array(values) if values.len() == 3));

        let find_success = call_native_function(
            &mut interpreter,
            "find",
            &[base_array.clone(), empty_function.clone()],
        );
        assert!(matches!(find_success, Value::Int(3)));

        let any_success = call_native_function(
            &mut interpreter,
            "any",
            &[base_array.clone(), empty_function.clone()],
        );
        assert!(matches!(any_success, Value::Bool(true)));

        let all_success =
            call_native_function(&mut interpreter, "all", &[base_array, empty_function]);
        assert!(matches!(all_success, Value::Bool(true)));
    }

    #[test]
    fn test_release_hardening_data_format_and_regex_contracts() {
        let mut interpreter = Interpreter::new();

        let parse_json_ok = call_native_function(
            &mut interpreter,
            "parse_json",
            &[Value::Str(Arc::new("{\"name\":\"kujo\",\"n\":2}".to_string()))],
        );
        match parse_json_ok {
            Value::Dict(map) => {
                assert!(map.contains_key("name"));
                assert!(map.contains_key("n"));
            }
            other => panic!("Expected parse_json dict result, got {:?}", other),
        }

        let mut sample_dict = crate::interpreter::DictMap::default();
        sample_dict.insert("ok".into(), Value::Bool(true));
        let to_json_ok = call_native_function(
            &mut interpreter,
            "to_json",
            &[Value::Dict(Arc::new(sample_dict))],
        );
        assert!(matches!(to_json_ok, Value::Str(result) if result.contains("\"ok\":true")));

        let mut pretty_dict = crate::interpreter::DictMap::default();
        pretty_dict.insert("ok".into(), Value::Bool(true));
        let to_json_pretty_ok = call_native_function(
            &mut interpreter,
            "to_json_pretty",
            &[Value::Dict(Arc::new(pretty_dict))],
        );
        assert!(
            matches!(to_json_pretty_ok, Value::Str(result) if result.contains("\"ok\": true") && result.contains('\n'))
        );

        let parse_toml_ok = call_native_function(
            &mut interpreter,
            "parse_toml",
            &[Value::Str(Arc::new("title = \"Kujo\"".to_string()))],
        );
        match parse_toml_ok {
            Value::Dict(map) => assert!(map.contains_key("title")),
            other => panic!("Expected parse_toml dict result, got {:?}", other),
        }

        let parse_yaml_ok = call_native_function(
            &mut interpreter,
            "parse_yaml",
            &[Value::Str(Arc::new("name: Kujo".to_string()))],
        );
        match parse_yaml_ok {
            Value::Dict(map) => assert!(map.contains_key("name")),
            other => panic!("Expected parse_yaml dict result, got {:?}", other),
        }

        let parse_csv_ok = call_native_function(
            &mut interpreter,
            "parse_csv",
            &[Value::Str(Arc::new("name,age\nKujo,2".to_string()))],
        );
        assert!(matches!(parse_csv_ok, Value::Array(rows) if rows.len() == 1));

        let mut csv_row = crate::interpreter::DictMap::default();
        csv_row.insert("name".into(), Value::Str(Arc::new("Kujo".to_string())));
        csv_row.insert("age".into(), Value::Int(2));
        let to_csv_ok = call_native_function(
            &mut interpreter,
            "to_csv",
            &[Value::Array(Arc::new(vec![Value::Dict(Arc::new(csv_row))]))],
        );
        assert!(
            matches!(to_csv_ok, Value::Str(csv) if csv.contains("name") && csv.contains("Kujo"))
        );

        let encode_base64_ok = call_native_function(
            &mut interpreter,
            "encode_base64",
            &[Value::Str(Arc::new("kujo".to_string()))],
        );
        let encoded = match encode_base64_ok {
            Value::Str(value) => value,
            other => panic!("Expected base64 string, got {:?}", other),
        };

        let decode_base64_ok =
            call_native_function(&mut interpreter, "decode_base64", &[Value::Str(encoded)]);
        assert!(matches!(decode_base64_ok, Value::Bytes(bytes) if bytes == b"kujo"));

        let parse_json_bad_shape =
            call_native_function(&mut interpreter, "parse_json", &[Value::Int(1)]);
        assert!(
            matches!(parse_json_bad_shape, Value::Error(message) if message.contains("parse_json requires a string argument"))
        );

        let parse_json_extra = call_native_function(
            &mut interpreter,
            "parse_json",
            &[Value::Str(Arc::new("{}".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(parse_json_extra, Value::Error(message) if message.contains("parse_json requires a string argument"))
        );

        let to_json_missing = call_native_function(&mut interpreter, "to_json", &[]);
        assert!(
            matches!(to_json_missing, Value::Error(message) if message.contains("to_json requires a value argument"))
        );

        let to_json_extra =
            call_native_function(&mut interpreter, "to_json", &[Value::Bool(true), Value::Int(1)]);
        assert!(
            matches!(to_json_extra, Value::Error(message) if message.contains("to_json requires a value argument"))
        );

        let to_json_pretty_extra = call_native_function(
            &mut interpreter,
            "to_json_pretty",
            &[Value::Bool(true), Value::Int(1)],
        );
        assert!(
            matches!(to_json_pretty_extra, Value::Error(message) if message.contains("to_json_pretty requires a value argument"))
        );

        let parse_toml_extra = call_native_function(
            &mut interpreter,
            "parse_toml",
            &[Value::Str(Arc::new("title = \"Kujo\"".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(parse_toml_extra, Value::Error(message) if message.contains("parse_toml requires a string argument"))
        );

        let to_toml_extra =
            call_native_function(&mut interpreter, "to_toml", &[Value::Bool(true), Value::Int(1)]);
        assert!(
            matches!(to_toml_extra, Value::Error(message) if message.contains("to_toml requires a value argument"))
        );

        let parse_yaml_extra = call_native_function(
            &mut interpreter,
            "parse_yaml",
            &[Value::Str(Arc::new("name: Kujo".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(parse_yaml_extra, Value::Error(message) if message.contains("parse_yaml requires a string argument"))
        );

        let to_yaml_extra =
            call_native_function(&mut interpreter, "to_yaml", &[Value::Bool(true), Value::Int(1)]);
        assert!(
            matches!(to_yaml_extra, Value::Error(message) if message.contains("to_yaml requires a value argument"))
        );

        let parse_csv_extra = call_native_function(
            &mut interpreter,
            "parse_csv",
            &[Value::Str(Arc::new("name,age\nKujo,2".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(parse_csv_extra, Value::Error(message) if message.contains("parse_csv requires a string argument"))
        );

        let to_csv_extra = call_native_function(
            &mut interpreter,
            "to_csv",
            &[Value::Array(Arc::new(vec![])), Value::Int(1)],
        );
        assert!(
            matches!(to_csv_extra, Value::Error(message) if message.contains("to_csv requires an array argument"))
        );

        let decode_base64_bad_shape =
            call_native_function(&mut interpreter, "decode_base64", &[Value::Int(1)]);
        assert!(
            matches!(decode_base64_bad_shape, Value::Error(message) if message.contains("decode_base64 requires a string argument"))
        );

        let decode_base64_extra = call_native_function(
            &mut interpreter,
            "decode_base64",
            &[Value::Str(Arc::new("cnVmZg==".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(decode_base64_extra, Value::Error(message) if message.contains("decode_base64 requires a string argument"))
        );

        let encode_base64_bad_shape =
            call_native_function(&mut interpreter, "encode_base64", &[Value::Int(1)]);
        assert!(
            matches!(encode_base64_bad_shape, Value::Error(message) if message.contains("encode_base64 requires a bytes or string argument"))
        );

        let encode_base64_extra = call_native_function(
            &mut interpreter,
            "encode_base64",
            &[Value::Str(Arc::new("kujo".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(encode_base64_extra, Value::Error(message) if message.contains("encode_base64 requires a bytes or string argument"))
        );

        let regex_match_ok = call_native_function(
            &mut interpreter,
            "regex_match",
            &[
                Value::Str(Arc::new("hello123".to_string())),
                Value::Str(Arc::new("^[a-z]+\\d+$".to_string())),
            ],
        );
        assert!(matches!(regex_match_ok, Value::Bool(true)));

        let regex_find_all_ok = call_native_function(
            &mut interpreter,
            "regex_find_all",
            &[
                Value::Str(Arc::new("a1 b22 c333".to_string())),
                Value::Str(Arc::new("\\d+".to_string())),
            ],
        );
        assert!(matches!(regex_find_all_ok, Value::Array(matches) if matches.len() == 3
            && matches!(&matches[0], Value::Str(value) if value.as_ref() == "1")
            && matches!(&matches[1], Value::Str(value) if value.as_ref() == "22")
            && matches!(&matches[2], Value::Str(value) if value.as_ref() == "333")));

        let regex_replace_ok = call_native_function(
            &mut interpreter,
            "regex_replace",
            &[
                Value::Str(Arc::new("a1 b22".to_string())),
                Value::Str(Arc::new("\\d+".to_string())),
                Value::Str(Arc::new("#".to_string())),
            ],
        );
        assert!(matches!(regex_replace_ok, Value::Str(value) if value.as_ref() == "a# b#"));

        let regex_split_ok = call_native_function(
            &mut interpreter,
            "regex_split",
            &[
                Value::Str(Arc::new("a, b; c".to_string())),
                Value::Str(Arc::new("[,;]\\s*".to_string())),
            ],
        );
        assert!(matches!(regex_split_ok, Value::Array(parts) if parts.len() == 3
            && matches!(&parts[0], Value::Str(value) if value.as_ref() == "a")
            && matches!(&parts[1], Value::Str(value) if value.as_ref() == "b")
            && matches!(&parts[2], Value::Str(value) if value.as_ref() == "c")));

        let regex_match_bad_shape =
            call_native_function(&mut interpreter, "regex_match", &[Value::Int(1)]);
        assert!(
            matches!(regex_match_bad_shape, Value::Error(message) if message.contains("regex_match requires two string arguments"))
        );

        let regex_replace_bad_shape = call_native_function(
            &mut interpreter,
            "regex_replace",
            &[Value::Str(Arc::new("a".to_string())), Value::Str(Arc::new("b".to_string()))],
        );
        assert!(
            matches!(regex_replace_bad_shape, Value::Error(message) if message.contains("regex_replace requires three string arguments"))
        );
    }

    #[test]
    fn test_release_hardening_env_os_path_and_assert_contracts() {
        let _cwd_guard = cwd_test_lock().lock().expect("cwd test lock should not be poisoned");
        let mut interpreter = Interpreter::new();

        let assert_true = call_native_function(&mut interpreter, "assert", &[Value::Bool(true)]);
        assert!(matches!(assert_true, Value::Bool(true)));

        let assert_false = call_native_function(&mut interpreter, "assert", &[Value::Bool(false)]);
        assert!(
            matches!(assert_false, Value::Error(message) if message.contains("Assertion failed"))
        );

        let assert_extra = call_native_function(
            &mut interpreter,
            "assert",
            &[Value::Bool(true), Value::Str(Arc::new("ok".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(assert_extra, Value::Error(message) if message.contains("expects at most 2 arguments"))
        );

        let assert_equal_ok =
            call_native_function(&mut interpreter, "assert_equal", &[Value::Int(7), Value::Int(7)]);
        assert!(matches!(assert_equal_ok, Value::Bool(true)));

        let assert_equal_bad_shape =
            call_native_function(&mut interpreter, "assert_equal", &[Value::Int(7)]);
        assert!(
            matches!(assert_equal_bad_shape, Value::Error(message) if message.contains("assert_equal requires 2 arguments"))
        );

        let assert_equal_extra = call_native_function(
            &mut interpreter,
            "assert_equal",
            &[Value::Int(7), Value::Int(7), Value::Int(7)],
        );
        assert!(
            matches!(assert_equal_extra, Value::Error(message) if message.contains("assert_equal requires 2 arguments"))
        );

        let assert_true_ok =
            call_native_function(&mut interpreter, "assert_true", &[Value::Bool(true)]);
        assert!(matches!(assert_true_ok, Value::Bool(true)));

        let assert_true_bad_type =
            call_native_function(&mut interpreter, "assert_true", &[Value::Int(1)]);
        assert!(
            matches!(assert_true_bad_type, Value::Error(message) if message.contains("assert_true requires a boolean argument"))
        );

        let assert_true_extra = call_native_function(
            &mut interpreter,
            "assert_true",
            &[Value::Bool(true), Value::Bool(true)],
        );
        assert!(
            matches!(assert_true_extra, Value::Error(message) if message.contains("assert_true requires exactly 1 argument"))
        );

        let assert_false_ok =
            call_native_function(&mut interpreter, "assert_false", &[Value::Bool(false)]);
        assert!(matches!(assert_false_ok, Value::Bool(true)));

        let assert_false_extra = call_native_function(
            &mut interpreter,
            "assert_false",
            &[Value::Bool(false), Value::Bool(false)],
        );
        assert!(
            matches!(assert_false_extra, Value::Error(message) if message.contains("assert_false requires exactly 1 argument"))
        );

        let assert_contains_array = call_native_function(
            &mut interpreter,
            "assert_contains",
            &[Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)])), Value::Int(2)],
        );
        assert!(matches!(assert_contains_array, Value::Bool(true)));

        let assert_contains_bad_shape = call_native_function(
            &mut interpreter,
            "assert_contains",
            &[Value::Str(Arc::new("x".to_string()))],
        );
        assert!(
            matches!(assert_contains_bad_shape, Value::Error(message) if message.contains("assert_contains requires 2 arguments"))
        );

        let assert_contains_extra = call_native_function(
            &mut interpreter,
            "assert_contains",
            &[
                Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)])),
                Value::Int(2),
                Value::Int(3),
            ],
        );
        assert!(
            matches!(assert_contains_extra, Value::Error(message) if message.contains("assert_contains requires 2 arguments"))
        );

        let debug_result = call_native_function(
            &mut interpreter,
            "debug",
            &[Value::Str(Arc::new("release-hardening".to_string())), Value::Int(1)],
        );
        assert!(matches!(debug_result, Value::Null));

        let env_key = "KUJO_RELEASE_HARDENING_ENV_TEST";
        let env_bool_invalid_key = "KUJO_RELEASE_HARDENING_ENV_BOOL_INVALID";
        let env_missing_key = "KUJO_RELEASE_HARDENING_ENV_MISSING";
        std::env::remove_var(env_missing_key);

        let env_set_result = call_native_function(
            &mut interpreter,
            "env_set",
            &[Value::Str(Arc::new(env_key.to_string())), Value::Str(Arc::new("42".to_string()))],
        );
        assert!(matches!(env_set_result, Value::Null));

        let env_bool_seed = call_native_function(
            &mut interpreter,
            "env_set",
            &[
                Value::Str(Arc::new(env_bool_invalid_key.to_string())),
                Value::Str(Arc::new("definitely-not-bool".to_string())),
            ],
        );
        assert!(matches!(env_bool_seed, Value::Null));

        let env_get_result = call_native_function(
            &mut interpreter,
            "env",
            &[Value::Str(Arc::new(env_key.to_string()))],
        );
        assert!(matches!(env_get_result, Value::Str(value) if value.as_ref() == "42"));

        let env_or_result = call_native_function(
            &mut interpreter,
            "env_or",
            &[
                Value::Str(Arc::new(env_missing_key.to_string())),
                Value::Str(Arc::new("fallback".to_string())),
            ],
        );
        assert!(matches!(env_or_result, Value::Str(value) if value.as_ref() == "fallback"));

        let env_extra = call_native_function(
            &mut interpreter,
            "env",
            &[Value::Str(Arc::new(env_key.to_string())), Value::Int(1)],
        );
        assert!(
            matches!(env_extra, Value::Error(message) if message.contains("env() expects 1 argument"))
        );

        let env_set_extra = call_native_function(
            &mut interpreter,
            "env_set",
            &[
                Value::Str(Arc::new(env_key.to_string())),
                Value::Str(Arc::new("42".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(env_set_extra, Value::Error(message) if message.contains("env_set() expects 2 arguments"))
        );

        let env_int_result = call_native_function(
            &mut interpreter,
            "env_int",
            &[Value::Str(Arc::new(env_key.to_string()))],
        );
        assert!(matches!(env_int_result, Value::Int(42)));

        let env_float_result = call_native_function(
            &mut interpreter,
            "env_float",
            &[Value::Str(Arc::new(env_key.to_string()))],
        );
        assert!(matches!(env_float_result, Value::Float(value) if (value - 42.0).abs() < 1e-12));

        let env_bool_bad_parse = call_native_function(
            &mut interpreter,
            "env_bool",
            &[Value::Str(Arc::new(env_bool_invalid_key.to_string()))],
        );
        assert!(matches!(env_bool_bad_parse, Value::ErrorObject { .. }));

        let env_required_missing = call_native_function(
            &mut interpreter,
            "env_required",
            &[Value::Str(Arc::new(env_missing_key.to_string()))],
        );
        assert!(matches!(env_required_missing, Value::ErrorObject { .. }));

        let env_list_result = call_native_function(&mut interpreter, "env_list", &[]);
        assert!(matches!(env_list_result, Value::Dict(map) if map.contains_key(env_key)));

        let env_list_extra = call_native_function(&mut interpreter, "env_list", &[Value::Int(1)]);
        assert!(
            matches!(env_list_extra, Value::Error(message) if message.contains("env_list() expects 0 arguments"))
        );

        let args_result = call_native_function(&mut interpreter, "args", &[]);
        assert!(matches!(args_result, Value::Array(_)));

        let args_extra = call_native_function(&mut interpreter, "args", &[Value::Int(1)]);
        assert!(
            matches!(args_extra, Value::Error(message) if message.contains("args() expects 0 arguments"))
        );

        let arg_parser_result = call_native_function(&mut interpreter, "arg_parser", &[]);
        assert!(matches!(arg_parser_result, Value::Struct { name, fields }
            if name == "ArgParser"
            && fields.contains_key("_args")
            && fields.contains_key("_app_name")
            && fields.contains_key("_description")));

        let arg_parser_extra =
            call_native_function(&mut interpreter, "arg_parser", &[Value::Int(1)]);
        assert!(
            matches!(arg_parser_extra, Value::Error(message) if message.contains("arg_parser() expects 0 arguments"))
        );

        let cwd_before = std::env::current_dir().expect("cwd should resolve");

        let mut temp_dir = cwd_before.clone();
        temp_dir.push("tmp");
        temp_dir.push(format!(
            "release_hardening_os_path_contract_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir)
                .expect("stale temp hardening dir should be removable");
        }
        std::fs::create_dir_all(&temp_dir).expect("temp hardening dir should be created");
        let temp_dir_string = temp_dir.to_string_lossy().to_string();

        let os_getcwd_before = call_native_function(&mut interpreter, "os_getcwd", &[]);
        assert!(matches!(os_getcwd_before, Value::Str(_)));

        let os_getcwd_extra = call_native_function(&mut interpreter, "os_getcwd", &[Value::Int(1)]);
        assert!(
            matches!(os_getcwd_extra, Value::Error(message) if message.contains("os_getcwd() expects 0 arguments"))
        );

        let os_chdir_result = call_native_function(
            &mut interpreter,
            "os_chdir",
            &[Value::Str(Arc::new(temp_dir_string.clone()))],
        );
        assert!(matches!(os_chdir_result, Value::Bool(true)));

        let os_chdir_extra = call_native_function(
            &mut interpreter,
            "os_chdir",
            &[Value::Str(Arc::new(temp_dir_string.clone())), Value::Int(1)],
        );
        assert!(
            matches!(os_chdir_extra, Value::Error(message) if message.contains("os_chdir() expects 1 argument"))
        );

        let os_getcwd_after = call_native_function(&mut interpreter, "os_getcwd", &[]);
        assert!(
            matches!(os_getcwd_after, Value::Str(path) if path.as_ref().as_str() == temp_dir_string)
        );

        std::env::set_current_dir(&cwd_before).expect("cwd should restore");

        let dirname_result = call_native_function(
            &mut interpreter,
            "dirname",
            &[Value::Str(Arc::new("a/b/file.kujo".to_string()))],
        );
        assert!(matches!(dirname_result, Value::Str(value) if value.as_ref().ends_with("a/b")));

        let dirname_extra = call_native_function(
            &mut interpreter,
            "dirname",
            &[Value::Str(Arc::new("a/b/file.kujo".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(dirname_extra, Value::Error(message) if message.contains("dirname() expects 1 argument"))
        );

        let basename_result = call_native_function(
            &mut interpreter,
            "basename",
            &[Value::Str(Arc::new("a/b/file.kujo".to_string()))],
        );
        assert!(matches!(basename_result, Value::Str(value) if value.as_ref() == "file.kujo"));

        let basename_extra = call_native_function(
            &mut interpreter,
            "basename",
            &[Value::Str(Arc::new("a/b/file.kujo".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(basename_extra, Value::Error(message) if message.contains("basename() expects 1 argument"))
        );

        let path_exists_true = call_native_function(
            &mut interpreter,
            "path_exists",
            &[Value::Str(Arc::new(temp_dir_string.clone()))],
        );
        assert!(matches!(path_exists_true, Value::Bool(true)));

        let path_exists_extra = call_native_function(
            &mut interpreter,
            "path_exists",
            &[Value::Str(Arc::new(temp_dir_string.clone())), Value::Int(1)],
        );
        assert!(
            matches!(path_exists_extra, Value::Error(message) if message.contains("path_exists() expects 1 argument"))
        );

        let path_is_dir_true = call_native_function(
            &mut interpreter,
            "path_is_dir",
            &[Value::Str(Arc::new(temp_dir_string.clone()))],
        );
        assert!(matches!(path_is_dir_true, Value::Bool(true)));

        let path_is_dir_extra = call_native_function(
            &mut interpreter,
            "path_is_dir",
            &[Value::Str(Arc::new(temp_dir_string.clone())), Value::Int(1)],
        );
        assert!(
            matches!(path_is_dir_extra, Value::Error(message) if message.contains("path_is_dir() expects 1 argument"))
        );

        let file_path = format!("{}/sample.kujo", temp_dir_string);
        std::fs::write(&file_path, "print(1)").expect("sample file should write");

        let path_is_file_true = call_native_function(
            &mut interpreter,
            "path_is_file",
            &[Value::Str(Arc::new(file_path.clone()))],
        );
        assert!(matches!(path_is_file_true, Value::Bool(true)));

        let path_is_file_extra = call_native_function(
            &mut interpreter,
            "path_is_file",
            &[Value::Str(Arc::new(file_path.clone())), Value::Int(1)],
        );
        assert!(
            matches!(path_is_file_extra, Value::Error(message) if message.contains("path_is_file() expects 1 argument"))
        );

        let path_is_symlink_true = call_native_function(
            &mut interpreter,
            "path_is_symlink",
            &[Value::Str(Arc::new(file_path.clone()))],
        );
        assert!(matches!(path_is_symlink_true, Value::Bool(false)));

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let symlink_path = format!("{}/sample-link.kujo", temp_dir_string);
            symlink(&file_path, &symlink_path).expect("symlink should be created");
            let path_is_symlink_link = call_native_function(
                &mut interpreter,
                "path_is_symlink",
                &[Value::Str(Arc::new(symlink_path.clone()))],
            );
            assert!(matches!(path_is_symlink_link, Value::Bool(true)));
            let _ = std::fs::remove_file(symlink_path);
        }

        let path_extension_result = call_native_function(
            &mut interpreter,
            "path_extension",
            &[Value::Str(Arc::new(file_path.clone()))],
        );
        assert!(matches!(path_extension_result, Value::Str(ext) if ext.as_ref() == "kujo"));

        let path_extension_extra = call_native_function(
            &mut interpreter,
            "path_extension",
            &[Value::Str(Arc::new(file_path.clone())), Value::Int(1)],
        );
        assert!(
            matches!(path_extension_extra, Value::Error(message) if message.contains("path_extension() expects 1 argument"))
        );

        let path_absolute_result = call_native_function(
            &mut interpreter,
            "path_absolute",
            &[Value::Str(Arc::new(file_path.clone()))],
        );
        assert!(
            matches!(path_absolute_result, Value::Str(path) if path.as_ref().ends_with("sample.kujo"))
        );

        let path_absolute_extra = call_native_function(
            &mut interpreter,
            "path_absolute",
            &[Value::Str(Arc::new(file_path.clone())), Value::Int(1)],
        );
        assert!(
            matches!(path_absolute_extra, Value::Error(message) if message.contains("path_absolute() expects 1 argument"))
        );

        std::fs::remove_file(&file_path).expect("sample file should remove");
        let rmdir_target = temp_dir.join("rmdir-target");
        std::fs::create_dir(&rmdir_target).expect("rmdir target should be created");
        let os_rmdir_result = call_native_function(
            &mut interpreter,
            "os_rmdir",
            &[Value::Str(Arc::new(rmdir_target.to_string_lossy().to_string()))],
        );
        assert!(matches!(os_rmdir_result, Value::Bool(true)));

        std::fs::remove_dir_all(&temp_dir).expect("temp hardening dir should remove");

        let os_environ_result = call_native_function(&mut interpreter, "os_environ", &[]);
        assert!(matches!(os_environ_result, Value::Dict(map) if map.contains_key(env_key)));

        let os_environ_extra =
            call_native_function(&mut interpreter, "os_environ", &[Value::Int(1)]);
        assert!(
            matches!(os_environ_extra, Value::Error(message) if message.contains("os_environ() expects 0 arguments"))
        );

        let os_rmdir_extra = call_native_function(
            &mut interpreter,
            "os_rmdir",
            &[Value::Str(Arc::new(temp_dir_string.clone())), Value::Int(1)],
        );
        assert!(
            matches!(os_rmdir_extra, Value::Error(message) if message.contains("os_rmdir() expects 1 argument"))
        );

        let os_chdir_bad_shape =
            call_native_function(&mut interpreter, "os_chdir", &[Value::Int(1)]);
        assert!(
            matches!(os_chdir_bad_shape, Value::Error(message) if message.contains("os_chdir requires a string argument"))
        );

        let dirname_bad_shape = call_native_function(&mut interpreter, "dirname", &[Value::Int(1)]);
        assert!(
            matches!(dirname_bad_shape, Value::Error(message) if message.contains("dirname requires a string argument"))
        );

        let path_exists_bad_shape =
            call_native_function(&mut interpreter, "path_exists", &[Value::Bool(true)]);
        assert!(
            matches!(path_exists_bad_shape, Value::Error(message) if message.contains("path_exists requires a string argument"))
        );
    }

    #[test]
    fn test_release_hardening_filesystem_core_contracts() {
        let mut interpreter = Interpreter::new();

        let read_missing = call_native_function(&mut interpreter, "read_file", &[]);
        assert!(
            matches!(read_missing, Value::Error(message) if message.contains("read_file_sync requires a string path argument"))
        );

        let read_extra = call_native_function(
            &mut interpreter,
            "read_file",
            &[Value::Str(Arc::new("/tmp/kujo-missing.txt".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(read_extra, Value::Error(message) if message.contains("read_file_sync requires a string path argument"))
        );

        let write_missing = call_native_function(&mut interpreter, "write_file", &[]);
        assert!(
            matches!(write_missing, Value::Error(message) if message.contains("write_file requires 2 or 3 arguments"))
        );

        let write_extra = call_native_function(
            &mut interpreter,
            "write_file",
            &[
                Value::Str(Arc::new("/tmp/kujo-write.txt".to_string())),
                Value::Str(Arc::new("content".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(write_extra, Value::Error(message) if message.contains("write_file optional overwrite flag must be a bool"))
        );

        let atomic_missing = call_native_function(&mut interpreter, "write_file_atomic", &[]);
        assert!(
            matches!(atomic_missing, Value::Error(message) if message.contains("write_file_atomic expects 2 to 3 arguments"))
        );

        let append_missing = call_native_function(&mut interpreter, "append_file", &[]);
        assert!(
            matches!(append_missing, Value::Error(message) if message.contains("append_file requires two arguments"))
        );

        let append_extra = call_native_function(
            &mut interpreter,
            "append_file",
            &[
                Value::Str(Arc::new("/tmp/kujo-append.txt".to_string())),
                Value::Str(Arc::new("content".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(append_extra, Value::Error(message) if message.contains("append_file requires two arguments"))
        );

        let exists_missing = call_native_function(&mut interpreter, "file_exists", &[]);
        assert!(
            matches!(exists_missing, Value::Error(message) if message.contains("file_exists requires a string path argument"))
        );

        let exists_extra = call_native_function(
            &mut interpreter,
            "file_exists",
            &[Value::Str(Arc::new("/tmp/kujo-file.txt".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(exists_extra, Value::Error(message) if message.contains("file_exists requires a string path argument"))
        );

        let lines_missing = call_native_function(&mut interpreter, "read_lines", &[]);
        assert!(
            matches!(lines_missing, Value::Error(message) if message.contains("read_lines requires a string path argument"))
        );

        let lines_extra = call_native_function(
            &mut interpreter,
            "read_lines",
            &[Value::Str(Arc::new("/tmp/kujo-lines.txt".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(lines_extra, Value::Error(message) if message.contains("read_lines requires a string path argument"))
        );

        let list_missing = call_native_function(&mut interpreter, "list_dir", &[]);
        assert!(
            matches!(list_missing, Value::Error(message) if message.contains("list_dir requires a string path argument"))
        );

        let list_extra = call_native_function(
            &mut interpreter,
            "list_dir",
            &[Value::Str(Arc::new("/tmp".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(list_extra, Value::Error(message) if message.contains("list_dir requires a string path argument"))
        );

        let create_missing = call_native_function(&mut interpreter, "create_dir", &[]);
        assert!(
            matches!(create_missing, Value::Error(message) if message.contains("create_dir requires a string path argument"))
        );

        let create_extra = call_native_function(
            &mut interpreter,
            "create_dir",
            &[Value::Str(Arc::new("/tmp/kujo-create-dir".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(create_extra, Value::Error(message) if message.contains("create_dir requires a string path argument"))
        );

        let file_size_missing = call_native_function(&mut interpreter, "file_size", &[]);
        assert!(
            matches!(file_size_missing, Value::Error(message) if message.contains("file_size requires a string path argument"))
        );

        let file_size_extra = call_native_function(
            &mut interpreter,
            "file_size",
            &[Value::Str(Arc::new("/tmp/kujo-size.txt".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(file_size_extra, Value::Error(message) if message.contains("file_size requires a string path argument"))
        );

        let delete_missing = call_native_function(&mut interpreter, "delete_file", &[]);
        assert!(
            matches!(delete_missing, Value::Error(message) if message.contains("delete_file requires a string path argument"))
        );

        let delete_extra = call_native_function(
            &mut interpreter,
            "delete_file",
            &[Value::Str(Arc::new("/tmp/kujo-delete.txt".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(delete_extra, Value::Error(message) if message.contains("delete_file requires a string path argument"))
        );

        let rename_missing = call_native_function(&mut interpreter, "rename_file", &[]);
        assert!(
            matches!(rename_missing, Value::Error(message) if message.contains("rename_file requires two arguments"))
        );

        let rename_extra = call_native_function(
            &mut interpreter,
            "rename_file",
            &[
                Value::Str(Arc::new("/tmp/kujo-old.txt".to_string())),
                Value::Str(Arc::new("/tmp/kujo-new.txt".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(rename_extra, Value::Error(message) if message.contains("rename_file requires two arguments"))
        );

        let copy_missing = call_native_function(&mut interpreter, "copy_file", &[]);
        assert!(
            matches!(copy_missing, Value::Error(message) if message.contains("copy_file requires two arguments"))
        );

        let copy_extra = call_native_function(
            &mut interpreter,
            "copy_file",
            &[
                Value::Str(Arc::new("/tmp/kujo-src.txt".to_string())),
                Value::Str(Arc::new("/tmp/kujo-dst.txt".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(copy_extra, Value::Error(message) if message.contains("copy_file requires two arguments"))
        );

        let read_binary_missing = call_native_function(&mut interpreter, "read_binary_file", &[]);
        assert!(
            matches!(read_binary_missing, Value::Error(message) if message.contains("read_binary_file requires a string path argument"))
        );

        let read_binary_extra = call_native_function(
            &mut interpreter,
            "read_binary_file",
            &[Value::Str(Arc::new("/tmp/kujo-bin.bin".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(read_binary_extra, Value::Error(message) if message.contains("read_binary_file requires a string path argument"))
        );

        let read_lossy_missing = call_native_function(&mut interpreter, "read_file_lossy", &[]);
        assert!(
            matches!(read_lossy_missing, Value::Error(message) if message.contains("read_file_lossy expects 1 argument"))
        );

        let read_lossy_extra = call_native_function(
            &mut interpreter,
            "read_file_lossy",
            &[Value::Str(Arc::new("/tmp/kujo-bin.txt".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(read_lossy_extra, Value::Error(message) if message.contains("read_file_lossy expects 1 argument"))
        );

        let write_binary_missing = call_native_function(&mut interpreter, "write_binary_file", &[]);
        assert!(
            matches!(write_binary_missing, Value::Error(message) if message.contains("write_binary_file requires 2 or 3 arguments"))
        );

        let write_binary_extra = call_native_function(
            &mut interpreter,
            "write_binary_file",
            &[
                Value::Str(Arc::new("/tmp/kujo-bin.bin".to_string())),
                Value::Bytes(vec![1, 2]),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(write_binary_extra, Value::Error(message) if message.contains("write_binary_file optional overwrite flag must be a bool"))
        );

        let base_dir = tmp_test_path("filesystem_hardening");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).expect("filesystem hardening dir should be created");

        let text_file = format!("{}/sample.txt", base_dir);
        let moved_file = format!("{}/sample.renamed.txt", base_dir);
        let copied_file = format!("{}/sample.copy.txt", base_dir);
        let nested_dir = format!("{}/nested", base_dir);
        let binary_file = format!("{}/payload.bin", base_dir);
        let atomic_text_file = format!("{}/atomic.txt", base_dir);
        let atomic_binary_file = format!("{}/atomic.bin", base_dir);

        let atomic_text_ok = call_native_function(
            &mut interpreter,
            "write_file_atomic",
            &[
                Value::Str(Arc::new(atomic_text_file.clone())),
                Value::Str(Arc::new("atomic-text".to_string())),
            ],
        );
        assert!(matches!(atomic_text_ok, Value::Bool(true)));
        assert_eq!(
            std::fs::read_to_string(&atomic_text_file).expect("atomic text should be readable"),
            "atomic-text"
        );

        let atomic_existing = call_native_function(
            &mut interpreter,
            "write_file_atomic",
            &[
                Value::Str(Arc::new(atomic_text_file.clone())),
                Value::Str(Arc::new("rejected".to_string())),
            ],
        );
        assert!(
            matches!(atomic_existing, Value::Error(message) if message.contains("file already exists"))
        );

        let atomic_replace = call_native_function(
            &mut interpreter,
            "write_file_atomic",
            &[
                Value::Str(Arc::new(atomic_text_file.clone())),
                Value::Str(Arc::new("replaced".to_string())),
                Value::Bool(true),
            ],
        );
        assert!(matches!(atomic_replace, Value::Bool(true)));
        assert_eq!(
            std::fs::read_to_string(&atomic_text_file)
                .expect("replaced atomic text should be readable"),
            "replaced"
        );

        let atomic_binary_ok = call_native_function(
            &mut interpreter,
            "write_file_atomic",
            &[Value::Str(Arc::new(atomic_binary_file.clone())), Value::Bytes(vec![0, 1, 2, 255])],
        );
        assert!(matches!(atomic_binary_ok, Value::Bool(true)));
        assert_eq!(
            std::fs::read(&atomic_binary_file).expect("atomic bytes should be readable"),
            vec![0, 1, 2, 255]
        );

        let write_ok = call_native_function(
            &mut interpreter,
            "write_file",
            &[
                Value::Str(Arc::new(text_file.clone())),
                Value::Str(Arc::new("line-1\nline-2".to_string())),
            ],
        );
        assert!(matches!(write_ok, Value::Bool(true)));

        let read_ok = call_native_function(
            &mut interpreter,
            "read_file",
            &[Value::Str(Arc::new(text_file.clone()))],
        );
        assert!(matches!(read_ok, Value::Str(content) if content.as_ref() == "line-1\nline-2"));

        let append_ok = call_native_function(
            &mut interpreter,
            "append_file",
            &[
                Value::Str(Arc::new(text_file.clone())),
                Value::Str(Arc::new("\nline-3".to_string())),
            ],
        );
        assert!(matches!(append_ok, Value::Bool(true)));

        let lines_ok = call_native_function(
            &mut interpreter,
            "read_lines",
            &[Value::Str(Arc::new(text_file.clone()))],
        );
        assert!(matches!(lines_ok, Value::Array(lines) if lines.len() == 3));

        let exists_true = call_native_function(
            &mut interpreter,
            "file_exists",
            &[Value::Str(Arc::new(text_file.clone()))],
        );
        assert!(matches!(exists_true, Value::Bool(true)));

        let file_size = call_native_function(
            &mut interpreter,
            "file_size",
            &[Value::Str(Arc::new(text_file.clone()))],
        );
        assert!(matches!(file_size, Value::Int(size) if size > 0));

        let rename_ok = call_native_function(
            &mut interpreter,
            "rename_file",
            &[Value::Str(Arc::new(text_file.clone())), Value::Str(Arc::new(moved_file.clone()))],
        );
        assert!(matches!(rename_ok, Value::Bool(true)));

        let copy_ok = call_native_function(
            &mut interpreter,
            "copy_file",
            &[Value::Str(Arc::new(moved_file.clone())), Value::Str(Arc::new(copied_file.clone()))],
        );
        assert!(matches!(copy_ok, Value::Bool(true)));

        let create_dir_ok = call_native_function(
            &mut interpreter,
            "create_dir",
            &[Value::Str(Arc::new(nested_dir.clone()))],
        );
        assert!(matches!(create_dir_ok, Value::Bool(true)));

        let list_dir_result = call_native_function(
            &mut interpreter,
            "list_dir",
            &[Value::Str(Arc::new(base_dir.clone()))],
        );
        assert!(matches!(list_dir_result, Value::Array(entries) if entries.len() >= 3));

        let write_binary_ok = call_native_function(
            &mut interpreter,
            "write_binary_file",
            &[Value::Str(Arc::new(binary_file.clone())), Value::Bytes(vec![0, 1, 2, 255])],
        );
        assert!(matches!(write_binary_ok, Value::Bool(true)));

        let read_binary_ok = call_native_function(
            &mut interpreter,
            "read_binary_file",
            &[Value::Str(Arc::new(binary_file.clone()))],
        );
        assert!(
            matches!(read_binary_ok, Value::Bytes(bytes) if bytes.len() == 4 && bytes[0] == 0 && bytes[3] == 255)
        );

        let lossy_file = format!("{}/lossy.txt", base_dir);
        std::fs::write(&lossy_file, [0x66_u8, 0x6f, 0x80, 0x6f])
            .expect("lossy file should be written");
        let read_lossy_ok = call_native_function(
            &mut interpreter,
            "read_file_lossy",
            &[Value::Str(Arc::new(lossy_file.clone()))],
        );
        let expected_lossy = String::from_utf8_lossy(&[0x66_u8, 0x6f, 0x80, 0x6f]).to_string();
        assert!(
            matches!(read_lossy_ok, Value::Str(text) if text.as_ref() == expected_lossy.as_str())
        );

        let delete_binary_ok = call_native_function(
            &mut interpreter,
            "delete_file",
            &[Value::Str(Arc::new(binary_file))],
        );
        assert!(matches!(delete_binary_ok, Value::Bool(true)));

        let delete_copy_ok = call_native_function(
            &mut interpreter,
            "delete_file",
            &[Value::Str(Arc::new(copied_file))],
        );
        assert!(matches!(delete_copy_ok, Value::Bool(true)));

        let delete_renamed_ok = call_native_function(
            &mut interpreter,
            "delete_file",
            &[Value::Str(Arc::new(moved_file.clone()))],
        );
        assert!(matches!(delete_renamed_ok, Value::Bool(true)));

        let exists_after_delete = call_native_function(
            &mut interpreter,
            "file_exists",
            &[Value::Str(Arc::new(moved_file))],
        );
        assert!(matches!(exists_after_delete, Value::Bool(false)));

        let _ = std::fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_release_hardening_io_module_dispatch_argument_contracts() {
        let mut interpreter = Interpreter::new();

        let read_bytes_missing = call_native_function(&mut interpreter, "io_read_bytes", &[]);
        assert!(
            matches!(read_bytes_missing, Value::Error(message) if message.contains("requires two arguments: path and count"))
        );

        let read_bytes_extra = call_native_function(
            &mut interpreter,
            "io_read_bytes",
            &[Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())), Value::Int(1), Value::Int(2)],
        );
        assert!(
            matches!(read_bytes_extra, Value::Error(message) if message.contains("requires two arguments: path and count"))
        );

        let write_bytes_extra = call_native_function(
            &mut interpreter,
            "io_write_bytes",
            &[
                Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())),
                Value::Bytes(vec![]),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(write_bytes_extra, Value::Error(message) if message.contains("requires two arguments: path and bytes"))
        );

        let append_bytes_extra = call_native_function(
            &mut interpreter,
            "io_append_bytes",
            &[
                Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())),
                Value::Bytes(vec![]),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(append_bytes_extra, Value::Error(message) if message.contains("requires two arguments: path and bytes"))
        );

        let read_at_missing = call_native_function(&mut interpreter, "io_read_at", &[]);
        assert!(
            matches!(read_at_missing, Value::Error(message) if message.contains("requires three arguments: path, offset, and count"))
        );

        let read_at_extra = call_native_function(
            &mut interpreter,
            "io_read_at",
            &[
                Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())),
                Value::Int(0),
                Value::Int(1),
                Value::Int(2),
            ],
        );
        assert!(
            matches!(read_at_extra, Value::Error(message) if message.contains("requires three arguments: path, offset, and count"))
        );

        let write_at_extra = call_native_function(
            &mut interpreter,
            "io_write_at",
            &[
                Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())),
                Value::Bytes(vec![]),
                Value::Int(0),
                Value::Int(2),
            ],
        );
        assert!(
            matches!(write_at_extra, Value::Error(message) if message.contains("requires three arguments: path, bytes, and offset"))
        );

        let seek_read_extra = call_native_function(
            &mut interpreter,
            "io_seek_read",
            &[Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())), Value::Int(0), Value::Int(2)],
        );
        assert!(
            matches!(seek_read_extra, Value::Error(message) if message.contains("requires two arguments: path and offset"))
        );

        let metadata_extra = call_native_function(
            &mut interpreter,
            "io_file_metadata",
            &[Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())), Value::Int(2)],
        );
        assert!(
            matches!(metadata_extra, Value::Error(message) if message.contains("requires a string path argument"))
        );

        let truncate_extra = call_native_function(
            &mut interpreter,
            "io_truncate",
            &[Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())), Value::Int(0), Value::Int(2)],
        );
        assert!(
            matches!(truncate_extra, Value::Error(message) if message.contains("requires two arguments: path and size"))
        );

        let copy_range_missing = call_native_function(&mut interpreter, "io_copy_range", &[]);
        assert!(
            matches!(copy_range_missing, Value::Error(message) if message.contains("requires four arguments: source, dest, offset, and count"))
        );

        let copy_range_extra = call_native_function(
            &mut interpreter,
            "io_copy_range",
            &[
                Value::Str(Arc::new("/tmp/source.bin".to_string())),
                Value::Str(Arc::new("/tmp/dest.bin".to_string())),
                Value::Int(0),
                Value::Int(1),
                Value::Int(2),
            ],
        );
        assert!(
            matches!(copy_range_extra, Value::Error(message) if message.contains("requires four arguments: source, dest, offset, and count"))
        );
    }

    #[test]
    fn test_release_hardening_http_module_dispatch_argument_contracts() {
        let mut interpreter = Interpreter::new();

        let get_extra = call_native_function(
            &mut interpreter,
            "http_get",
            &[
                Value::Str(Arc::new("https://example.com".to_string())),
                Value::Str(Arc::new("extra".to_string())),
            ],
        );
        assert!(
            matches!(get_extra, Value::Error(message) if message.contains("http_get() expects 1 argument"))
        );

        let get_missing = call_native_function(&mut interpreter, "http_get", &[]);
        assert!(
            matches!(get_missing, Value::Error(message) if message.contains("http_get() expects 1 argument"))
        );

        let post_extra = call_native_function(
            &mut interpreter,
            "http_post",
            &[
                Value::Str(Arc::new("https://example.com".to_string())),
                Value::Str(Arc::new("{}".to_string())),
                Value::Str(Arc::new("extra".to_string())),
            ],
        );
        assert!(
            matches!(post_extra, Value::Error(message) if message.contains("http_post() expects 2 arguments"))
        );

        let post_missing = call_native_function(&mut interpreter, "http_post", &[]);
        assert!(
            matches!(post_missing, Value::Error(message) if message.contains("http_post() expects 2 arguments"))
        );

        let put_extra = call_native_function(
            &mut interpreter,
            "http_put",
            &[
                Value::Str(Arc::new("https://example.com".to_string())),
                Value::Str(Arc::new("{}".to_string())),
                Value::Str(Arc::new("extra".to_string())),
            ],
        );
        assert!(
            matches!(put_extra, Value::Error(message) if message.contains("http_put() expects 2 arguments"))
        );

        let delete_extra = call_native_function(
            &mut interpreter,
            "http_delete",
            &[
                Value::Str(Arc::new("https://example.com".to_string())),
                Value::Str(Arc::new("extra".to_string())),
            ],
        );
        assert!(
            matches!(delete_extra, Value::Error(message) if message.contains("http_delete() expects 1 argument"))
        );

        let binary_extra = call_native_function(
            &mut interpreter,
            "http_get_binary",
            &[Value::Str(Arc::new("https://example.com".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(binary_extra, Value::Error(message) if message.contains("http_get_binary() expects 1 argument"))
        );

        let stream_extra = call_native_function(
            &mut interpreter,
            "http_get_stream",
            &[Value::Str(Arc::new("https://example.com".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(stream_extra, Value::Error(message) if message.contains("http_get_stream() expects 1 argument"))
        );

        let server_extra = call_native_function(
            &mut interpreter,
            "http_server",
            &[Value::Int(8080), Value::Int(1)],
        );
        assert!(
            matches!(server_extra, Value::Error(message) if message.contains("http_server() expects 1 argument"))
        );

        let response_extra = call_native_function(
            &mut interpreter,
            "http_response",
            &[
                Value::Int(200),
                Value::Str(std::sync::Arc::new("ok".to_string())),
                Value::Str(std::sync::Arc::new("extra".to_string())),
            ],
        );
        assert!(
            matches!(response_extra, Value::Error(message) if message.contains("http_response() expects 2 arguments"))
        );

        let response_shape = call_native_function(
            &mut interpreter,
            "http_response",
            &[Value::Int(200), Value::Str(std::sync::Arc::new("ok".to_string()))],
        );
        assert!(matches!(response_shape, Value::HttpResponse { status, .. } if status == 200));

        let json_response_extra = call_native_function(
            &mut interpreter,
            "json_response",
            &[Value::Int(200), Value::Null, Value::Int(1)],
        );
        assert!(
            matches!(json_response_extra, Value::Error(message) if message.contains("json_response() expects 2 arguments"))
        );

        let html_response_extra = call_native_function(
            &mut interpreter,
            "html_response",
            &[Value::Int(200), Value::Str(Arc::new("<h1>ok</h1>".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(html_response_extra, Value::Error(message) if message.contains("html_response() expects 2 arguments"))
        );

        let redirect_extra = call_native_function(
            &mut interpreter,
            "redirect_response",
            &[
                Value::Str(Arc::new("https://example.com".to_string())),
                Value::Dict(Arc::new(crate::interpreter::DictMap::default())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(redirect_extra, Value::Error(message) if message.contains("redirect_response() expects 1-2 arguments"))
        );

        let set_header_extra = call_native_function(
            &mut interpreter,
            "set_header",
            &[
                Value::Int(1),
                Value::Str(Arc::new("X-Test".to_string())),
                Value::Str(Arc::new("ok".to_string())),
                Value::Int(2),
            ],
        );
        assert!(
            matches!(set_header_extra, Value::Error(message) if message.contains("set_header() expects 3 arguments"))
        );

        let set_headers_extra = call_native_function(
            &mut interpreter,
            "set_headers",
            &[
                Value::Int(1),
                Value::Dict(Arc::new(crate::interpreter::DictMap::default())),
                Value::Int(2),
            ],
        );
        assert!(
            matches!(set_headers_extra, Value::Error(message) if message.contains("set_headers() expects 2 arguments"))
        );

        let parallel_http_missing = call_native_function(&mut interpreter, "parallel_http", &[]);
        assert!(
            matches!(parallel_http_missing, Value::Error(message) if message.contains("parallel_http() expects 1 argument"))
        );

        let parallel_http_extra = call_native_function(
            &mut interpreter,
            "parallel_http",
            &[Value::Array(Arc::new(vec![])), Value::Int(1)],
        );
        assert!(
            matches!(parallel_http_extra, Value::Error(message) if message.contains("parallel_http() expects 1 argument"))
        );

        let jwt_encode_missing = call_native_function(&mut interpreter, "jwt_encode", &[]);
        assert!(
            matches!(jwt_encode_missing, Value::Error(message) if message.contains("jwt_encode() expects 2 arguments"))
        );

        let jwt_encode_extra = call_native_function(
            &mut interpreter,
            "jwt_encode",
            &[
                Value::Dict(Arc::new(crate::interpreter::DictMap::default())),
                Value::Str(Arc::new("secret".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(jwt_encode_extra, Value::Error(message) if message.contains("jwt_encode() expects 2 arguments"))
        );

        let jwt_decode_missing = call_native_function(&mut interpreter, "jwt_decode", &[]);
        assert!(
            matches!(jwt_decode_missing, Value::Error(message) if message.contains("jwt_decode() expects 2 arguments"))
        );

        let jwt_decode_extra = call_native_function(
            &mut interpreter,
            "jwt_decode",
            &[
                Value::Str(Arc::new("token".to_string())),
                Value::Str(Arc::new("secret".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(jwt_decode_extra, Value::Error(message) if message.contains("jwt_decode() expects 2 arguments"))
        );

        let jwt_verify_missing = call_native_function(&mut interpreter, "jwt_verify", &[]);
        assert!(
            matches!(jwt_verify_missing, Value::Error(message) if message.contains("jwt_verify() expects 2 arguments"))
        );

        let jwt_verify_extra = call_native_function(
            &mut interpreter,
            "jwt_verify",
            &[
                Value::Str(Arc::new("token".to_string())),
                Value::Str(Arc::new("secret".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(jwt_verify_extra, Value::Error(message) if message.contains("jwt_verify() expects 2 arguments"))
        );

        let oauth2_auth_url_missing =
            call_native_function(&mut interpreter, "oauth2_auth_url", &[]);
        assert!(
            matches!(oauth2_auth_url_missing, Value::Error(message) if message.contains("oauth2_auth_url() expects 4 arguments"))
        );

        let oauth2_auth_url_extra = call_native_function(
            &mut interpreter,
            "oauth2_auth_url",
            &[
                Value::Str(Arc::new("client".to_string())),
                Value::Str(Arc::new("https://example.com/callback".to_string())),
                Value::Str(Arc::new("https://auth.example.com/authorize".to_string())),
                Value::Str(Arc::new("read".to_string())),
                Value::Str(Arc::new("extra".to_string())),
            ],
        );
        assert!(
            matches!(oauth2_auth_url_extra, Value::Error(message) if message.contains("oauth2_auth_url() expects 4 arguments"))
        );

        let oauth2_get_token_missing =
            call_native_function(&mut interpreter, "oauth2_get_token", &[]);
        assert!(
            matches!(oauth2_get_token_missing, Value::Error(message) if message.contains("oauth2_get_token() expects 5 arguments"))
        );

        let oauth2_get_token_extra = call_native_function(
            &mut interpreter,
            "oauth2_get_token",
            &[
                Value::Str(Arc::new("code".to_string())),
                Value::Str(Arc::new("client".to_string())),
                Value::Str(Arc::new("secret".to_string())),
                Value::Str(Arc::new("https://auth.example.com/token".to_string())),
                Value::Str(Arc::new("https://example.com/callback".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(oauth2_get_token_extra, Value::Error(message) if message.contains("oauth2_get_token() expects 5 arguments"))
        );
    }

    #[test]
    fn test_release_hardening_http_advanced_api_behavior_contracts() {
        let mut interpreter = Interpreter::new();

        let parallel_http_empty = call_native_function(
            &mut interpreter,
            "parallel_http",
            &[Value::Array(Arc::new(vec![]))],
        );
        assert!(matches!(parallel_http_empty, Value::Array(results) if results.is_empty()));

        let parallel_http_bad_element = call_native_function(
            &mut interpreter,
            "parallel_http",
            &[Value::Array(Arc::new(vec![Value::Int(1)]))],
        );
        assert!(
            matches!(parallel_http_bad_element, Value::Error(message) if message.contains("URL at index 0 must be a string"))
        );

        let oversized_urls = Value::Array(Arc::new(
            (0..=crate::runtime_limits::MAX_PARALLEL_HTTP_REQUESTS)
                .map(|_| Value::Str(Arc::new("https://example.com".to_string())))
                .collect(),
        ));
        let parallel_http_too_many =
            call_native_function(&mut interpreter, "parallel_http", &[oversized_urls]);
        assert!(
            matches!(parallel_http_too_many, Value::Error(message) if message.contains("request count exceeds maximum"))
        );

        let mut payload = crate::interpreter::DictMap::default();
        payload.insert("sub".into(), Value::Str(Arc::new("user-123".to_string())));
        payload.insert("role".into(), Value::Str(Arc::new("admin".to_string())));

        let secret = Value::Str(Arc::new("release-hardening-secret".to_string()));
        let encoded = call_native_function(
            &mut interpreter,
            "jwt_encode",
            &[Value::Dict(Arc::new(payload.clone())), secret.clone()],
        );

        let token = match encoded {
            Value::Str(token) => {
                assert!(!token.is_empty());
                token
            }
            other => panic!("Expected JWT token string from jwt_encode, got {:?}", other),
        };

        let decoded = call_native_function(
            &mut interpreter,
            "jwt_decode",
            &[Value::Str(token.clone()), secret.clone()],
        );
        match decoded {
            Value::Dict(decoded_payload) => {
                assert!(
                    matches!(decoded_payload.get("sub"), Some(Value::Str(sub)) if sub.as_ref() == "user-123")
                );
                assert!(
                    matches!(decoded_payload.get("role"), Some(Value::Str(role)) if role.as_ref() == "admin")
                );
            }
            other => panic!("Expected Dict payload from jwt_decode, got {:?}", other),
        }

        let verify_valid = call_native_function(
            &mut interpreter,
            "jwt_verify",
            &[Value::Str(token.clone()), secret.clone()],
        );
        assert!(matches!(verify_valid, Value::Bool(true)));

        let verify_invalid = call_native_function(
            &mut interpreter,
            "jwt_verify",
            &[Value::Str(token), Value::Str(Arc::new("wrong-secret".to_string()))],
        );
        assert!(matches!(verify_invalid, Value::Bool(false)));

        let auth_url = call_native_function(
            &mut interpreter,
            "oauth2_auth_url",
            &[
                Value::Str(Arc::new("client-abc".to_string())),
                Value::Str(Arc::new("https://example.com/callback".to_string())),
                Value::Str(Arc::new("https://auth.example.com/authorize".to_string())),
                Value::Str(Arc::new("read write".to_string())),
            ],
        );

        match auth_url {
            Value::Str(url) => {
                assert!(url.contains("client_id=client-abc"));
                assert!(url.contains("redirect_uri=https%3A%2F%2Fexample.com%2Fcallback"));
                assert!(url.contains("scope=read+write") || url.contains("scope=read%20write"));
            }
            other => panic!("Expected URL string from oauth2_auth_url, got {:?}", other),
        }
    }

    #[test]
    fn test_release_hardening_database_module_dispatch_argument_contracts() {
        let mut interpreter = Interpreter::new();

        if !cfg!(feature = "runtime-db") {
            let disabled = call_native_function(&mut interpreter, "db_connect", &[]);
            assert!(
                matches!(disabled, Value::Error(message) if message.contains("disabled in this build"))
            );
            return;
        }

        let connect_missing = call_native_function(&mut interpreter, "db_connect", &[]);
        assert!(
            matches!(connect_missing, Value::Error(message) if message.contains("db_connect requires database type"))
        );

        let execute_missing = call_native_function(&mut interpreter, "db_execute", &[]);
        assert!(
            matches!(execute_missing, Value::Error(message) if message.contains("db_execute requires a database connection"))
        );

        let pool_missing = call_native_function(&mut interpreter, "db_pool", &[]);
        assert!(
            matches!(pool_missing, Value::Error(message) if message.contains("db_pool requires database type and connection string"))
        );

        let connect_extra = call_native_function(
            &mut interpreter,
            "db_connect",
            &[
                Value::Str(Arc::new("sqlite".to_string())),
                Value::Str(Arc::new(":memory:".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(connect_extra, Value::Error(message) if message.contains("db_connect requires database type"))
        );

        let execute_extra = call_native_function(
            &mut interpreter,
            "db_execute",
            &[
                Value::Int(1),
                Value::Str(Arc::new("SELECT 1".to_string())),
                Value::Array(Arc::new(vec![])),
                Value::Int(99),
            ],
        );
        assert!(
            matches!(execute_extra, Value::Error(message) if message.contains("db_execute requires 2 or 3 arguments"))
        );

        let query_extra = call_native_function(
            &mut interpreter,
            "db_query",
            &[
                Value::Int(1),
                Value::Str(Arc::new("SELECT 1".to_string())),
                Value::Array(Arc::new(vec![])),
                Value::Int(99),
            ],
        );
        assert!(
            matches!(query_extra, Value::Error(message) if message.contains("db_query requires 2 or 3 arguments"))
        );

        let pool_extra = call_native_function(
            &mut interpreter,
            "db_pool",
            &[
                Value::Str(Arc::new("sqlite".to_string())),
                Value::Str(Arc::new(":memory:".to_string())),
                Value::Dict(Arc::new(crate::interpreter::DictMap::default())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(pool_extra, Value::Error(message) if message.contains("db_pool requires 2 or 3 arguments"))
        );

        for (name, args, expected_substring) in vec![
            (
                "db_close",
                vec![Value::Int(1), Value::Int(2)],
                "db_close requires a database connection",
            ),
            (
                "db_pool_acquire",
                vec![Value::Int(1), Value::Int(2)],
                "db_pool_acquire requires a database pool",
            ),
            (
                "db_pool_release",
                vec![Value::Int(1), Value::Int(2), Value::Int(3)],
                "db_pool_release requires two arguments",
            ),
            (
                "db_pool_stats",
                vec![Value::Int(1), Value::Int(2)],
                "db_pool_stats requires a database pool",
            ),
            (
                "db_pool_close",
                vec![Value::Int(1), Value::Int(2)],
                "db_pool_close requires a database pool",
            ),
            (
                "db_begin",
                vec![Value::Int(1), Value::Int(2)],
                "db_begin requires a database connection",
            ),
            (
                "db_commit",
                vec![Value::Int(1), Value::Int(2)],
                "db_commit requires a database connection",
            ),
            (
                "db_rollback",
                vec![Value::Int(1), Value::Int(2)],
                "db_rollback requires a database connection",
            ),
            (
                "db_last_insert_id",
                vec![Value::Int(1), Value::Int(2)],
                "db_last_insert_id requires a database connection",
            ),
        ] {
            let result = call_native_function(&mut interpreter, name, &args);
            assert!(
                matches!(result, Value::Error(ref message) if message.contains(expected_substring)),
                "Expected strict-arity rejection for {} with message containing '{}', got {:?}",
                name,
                expected_substring,
                result
            );
        }
    }

    #[test]
    fn test_release_hardening_process_module_dispatch_argument_contracts() {
        let mut interpreter = Interpreter::new();

        let spawn_missing = call_native_function(&mut interpreter, "spawn_process", &[]);
        assert!(
            matches!(spawn_missing, Value::Error(message) if message.contains("optional options"))
        );

        let pipe_missing = call_native_function(&mut interpreter, "pipe_commands", &[]);
        assert!(
            matches!(pipe_missing, Value::Error(message) if message.contains("optional options"))
        );

        let spawn_extra = call_native_function(
            &mut interpreter,
            "spawn_process",
            &[
                Value::Array(Arc::new(vec![Value::Str(Arc::new("echo".to_string()))])),
                Value::Dict(Arc::new(Default::default())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(spawn_extra, Value::Error(message) if message.contains("optional options"))
        );

        let pipe_extra = call_native_function(
            &mut interpreter,
            "pipe_commands",
            &[
                Value::Array(Arc::new(vec![Value::Array(Arc::new(vec![Value::Str(Arc::new(
                    "echo".to_string(),
                ))]))])),
                Value::Dict(Arc::new(Default::default())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(pipe_extra, Value::Error(message) if message.contains("optional options"))
        );
    }

    #[test]
    fn test_release_hardening_async_alias_dispatch_argument_contracts() {
        let mut interpreter = Interpreter::new();

        for alias in ["Promise.all", "promise_all", "await_all"] {
            let missing_args = call_native_function(&mut interpreter, alias, &[]);
            assert!(
                matches!(missing_args, Value::Error(ref message) if message.contains("Promise.all expects 1 to 2 arguments, got 0")),
                "Unexpected missing-args contract for {}: {:?}",
                alias,
                missing_args
            );

            let non_array = call_native_function(&mut interpreter, alias, &[Value::Int(1)]);
            assert!(
                matches!(non_array, Value::Error(ref message) if message.contains("Promise.all() requires an array of promises")),
                "Unexpected non-array contract for {}: {:?}",
                alias,
                non_array
            );

            let invalid_limit = call_native_function(
                &mut interpreter,
                alias,
                &[Value::Array(Arc::new(vec![])), Value::Int(0)],
            );
            assert!(
                matches!(invalid_limit, Value::Error(ref message) if message.contains("Promise.all() concurrency_limit must be > 0")),
                "Unexpected concurrency-limit contract for {}: {:?}",
                alias,
                invalid_limit
            );
        }

        for alias in ["parallel_map", "par_map", "par_each"] {
            let non_array = call_native_function(
                &mut interpreter,
                alias,
                &[Value::Int(1), Value::NativeFunction("len".to_string())],
            );
            assert!(
                matches!(non_array, Value::Error(ref message) if message.contains("parallel_map() first argument must be an array")),
                "Unexpected non-array contract for {}: {:?}",
                alias,
                non_array
            );

            let non_callable = call_native_function(
                &mut interpreter,
                alias,
                &[Value::Array(Arc::new(vec![Value::Int(1)])), Value::Int(1)],
            );
            assert!(
                matches!(non_callable, Value::Error(ref message) if message.contains("parallel_map() second argument must be a callable function")),
                "Unexpected callable-shape contract for {}: {:?}",
                alias,
                non_callable
            );

            let invalid_limit = call_native_function(
                &mut interpreter,
                alias,
                &[
                    Value::Array(Arc::new(vec![Value::Int(1)])),
                    Value::NativeFunction("len".to_string()),
                    Value::Int(0),
                ],
            );
            assert!(
                matches!(invalid_limit, Value::Error(ref message) if message.contains("parallel_map() concurrency_limit must be > 0")),
                "Unexpected concurrency-limit contract for {}: {:?}",
                alias,
                invalid_limit
            );
        }
    }

    #[test]
    fn test_release_hardening_async_batch_file_contracts() {
        let mut interpreter = Interpreter::new();

        let read_missing_args = call_native_function(&mut interpreter, "async_read_files", &[]);
        assert!(
            matches!(read_missing_args, Value::Error(message) if message.contains("expects 1 or 2 arguments"))
        );

        let read_non_array =
            call_native_function(&mut interpreter, "async_read_files", &[Value::Int(1)]);
        assert!(
            matches!(read_non_array, Value::Error(message) if message.contains("first argument must be an array of string paths"))
        );

        let read_non_positive_limit = call_native_function(
            &mut interpreter,
            "async_read_files",
            &[Value::Array(Arc::new(vec![])), Value::Int(0)],
        );
        assert!(
            matches!(read_non_positive_limit, Value::Error(message) if message.contains("concurrency_limit must be > 0"))
        );

        let read_non_int_limit = call_native_function(
            &mut interpreter,
            "async_read_files",
            &[Value::Array(Arc::new(vec![])), Value::Str(Arc::new("2".to_string()))],
        );
        assert!(
            matches!(read_non_int_limit, Value::Error(message) if message.contains("optional concurrency_limit must be an integer"))
        );

        let read_bad_path_element = call_native_function(
            &mut interpreter,
            "async_read_files",
            &[Value::Array(Arc::new(vec![Value::Int(1)]))],
        );
        assert!(
            matches!(read_bad_path_element, Value::Error(message) if message.contains("path at index 0 must be a string"))
        );

        let write_missing_args = call_native_function(&mut interpreter, "async_write_files", &[]);
        assert!(
            matches!(write_missing_args, Value::Error(message) if message.contains("expects 2 or 3 arguments"))
        );

        let write_non_array_paths = call_native_function(
            &mut interpreter,
            "async_write_files",
            &[Value::Int(1), Value::Array(Arc::new(vec![]))],
        );
        assert!(
            matches!(write_non_array_paths, Value::Error(message) if message.contains("first argument must be an array of string paths"))
        );

        let write_non_array_contents = call_native_function(
            &mut interpreter,
            "async_write_files",
            &[Value::Array(Arc::new(vec![])), Value::Int(1)],
        );
        assert!(
            matches!(write_non_array_contents, Value::Error(message) if message.contains("second argument must be an array of string contents"))
        );

        let write_mismatched_lengths = call_native_function(
            &mut interpreter,
            "async_write_files",
            &[
                Value::Array(Arc::new(vec![Value::Str(Arc::new("a.txt".to_string()))])),
                Value::Array(Arc::new(vec![])),
            ],
        );
        assert!(
            matches!(write_mismatched_lengths, Value::Error(message) if message.contains("paths and contents arrays must have the same length"))
        );

        let write_non_positive_limit = call_native_function(
            &mut interpreter,
            "async_write_files",
            &[Value::Array(Arc::new(vec![])), Value::Array(Arc::new(vec![])), Value::Int(0)],
        );
        assert!(
            matches!(write_non_positive_limit, Value::Error(message) if message.contains("concurrency_limit must be > 0"))
        );

        let write_non_int_limit = call_native_function(
            &mut interpreter,
            "async_write_files",
            &[
                Value::Array(Arc::new(vec![])),
                Value::Array(Arc::new(vec![])),
                Value::Str(Arc::new("2".to_string())),
            ],
        );
        assert!(
            matches!(write_non_int_limit, Value::Error(message) if message.contains("optional concurrency_limit must be an integer"))
        );

        let write_bad_path_element = call_native_function(
            &mut interpreter,
            "async_write_files",
            &[
                Value::Array(Arc::new(vec![Value::Int(1)])),
                Value::Array(Arc::new(vec![Value::Str(Arc::new("value".to_string()))])),
            ],
        );
        assert!(
            matches!(write_bad_path_element, Value::Error(message) if message.contains("path at index 0 must be a string"))
        );

        let write_bad_content_element = call_native_function(
            &mut interpreter,
            "async_write_files",
            &[
                Value::Array(Arc::new(vec![Value::Str(Arc::new("a.txt".to_string()))])),
                Value::Array(Arc::new(vec![Value::Int(1)])),
            ],
        );
        assert!(
            matches!(write_bad_content_element, Value::Error(message) if message.contains("content at index 0 must be a string"))
        );

        let base_dir = tmp_test_path("async_batch_file_contracts");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).expect("async batch hardening dir should be created");

        let first_path = format!("{}/first.txt", base_dir);
        let second_path = format!("{}/second.txt", base_dir);

        let write_success = call_native_function(
            &mut interpreter,
            "async_write_files",
            &[
                Value::Array(Arc::new(vec![
                    Value::Str(Arc::new(first_path.clone())),
                    Value::Str(Arc::new(second_path.clone())),
                ])),
                Value::Array(Arc::new(vec![
                    Value::Str(Arc::new("alpha".to_string())),
                    Value::Str(Arc::new("beta".to_string())),
                ])),
                Value::Int(1),
            ],
        );

        let write_results = await_native_promise(write_success);
        assert!(matches!(write_results, Ok(Value::Array(results)) if results.len() == 2
            && matches!(&results[0], Value::Bool(true))
            && matches!(&results[1], Value::Bool(true))));

        assert_eq!(
            std::fs::read_to_string(&first_path).expect("first async-written file should exist"),
            "alpha"
        );
        assert_eq!(
            std::fs::read_to_string(&second_path).expect("second async-written file should exist"),
            "beta"
        );

        let read_success = call_native_function(
            &mut interpreter,
            "async_read_files",
            &[
                Value::Array(Arc::new(vec![
                    Value::Str(Arc::new(first_path.clone())),
                    Value::Str(Arc::new(second_path.clone())),
                ])),
                Value::Int(1),
            ],
        );

        let read_results = await_native_promise(read_success);
        assert!(matches!(read_results, Ok(Value::Array(results)) if results.len() == 2
            && matches!(&results[0], Value::Str(content) if content.as_ref() == "alpha")
            && matches!(&results[1], Value::Str(content) if content.as_ref() == "beta")));

        let _ = std::fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_release_hardening_shared_state_and_task_pool_contracts() {
        let mut interpreter = Interpreter::new();

        let shared_key = format!(
            "KUJO_RELEASE_HARDENING_SHARED_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after UNIX_EPOCH")
                .as_nanos()
        );

        let shared_set_missing = call_native_function(&mut interpreter, "shared_set", &[]);
        assert!(
            matches!(shared_set_missing, Value::Error(message) if message.contains("shared_set requires (key, value) arguments"))
        );

        let shared_set_bad_key =
            call_native_function(&mut interpreter, "shared_set", &[Value::Int(1), Value::Int(2)]);
        assert!(
            matches!(shared_set_bad_key, Value::Error(message) if message.contains("shared_set key must be a string"))
        );

        let shared_get_missing = call_native_function(&mut interpreter, "shared_get", &[]);
        assert!(
            matches!(shared_get_missing, Value::Error(message) if message.contains("shared_get requires one key argument"))
        );

        let shared_get_missing_key = call_native_function(
            &mut interpreter,
            "shared_get",
            &[Value::Str(Arc::new("KUJO_RELEASE_HARDENING_SHARED_MISSING".to_string()))],
        );
        assert!(
            matches!(shared_get_missing_key, Value::Error(message) if message.contains("key 'KUJO_RELEASE_HARDENING_SHARED_MISSING' not found"))
        );

        let shared_set_ok = call_native_function(
            &mut interpreter,
            "shared_set",
            &[Value::Str(Arc::new(shared_key.clone())), Value::Int(10)],
        );
        assert!(matches!(shared_set_ok, Value::Bool(true)));

        let shared_has_true = call_native_function(
            &mut interpreter,
            "shared_has",
            &[Value::Str(Arc::new(shared_key.clone()))],
        );
        assert!(matches!(shared_has_true, Value::Bool(true)));

        let shared_get_ok = call_native_function(
            &mut interpreter,
            "shared_get",
            &[Value::Str(Arc::new(shared_key.clone()))],
        );
        assert!(matches!(shared_get_ok, Value::Int(10)));

        let shared_add_missing = call_native_function(&mut interpreter, "shared_add_int", &[]);
        assert!(
            matches!(shared_add_missing, Value::Error(message) if message.contains("shared_add_int requires (key, delta) arguments"))
        );

        let shared_add_bad_delta = call_native_function(
            &mut interpreter,
            "shared_add_int",
            &[Value::Str(Arc::new(shared_key.clone())), Value::Str(Arc::new("1".to_string()))],
        );
        assert!(
            matches!(shared_add_bad_delta, Value::Error(message) if message.contains("delta must be an int"))
        );

        let shared_add_missing_key = call_native_function(
            &mut interpreter,
            "shared_add_int",
            &[
                Value::Str(Arc::new("KUJO_RELEASE_HARDENING_SHARED_ADD_MISSING".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(shared_add_missing_key, Value::Error(message) if message.contains("key 'KUJO_RELEASE_HARDENING_SHARED_ADD_MISSING' not found"))
        );

        let shared_add_ok = call_native_function(
            &mut interpreter,
            "shared_add_int",
            &[Value::Str(Arc::new(shared_key.clone())), Value::Int(5)],
        );
        assert!(matches!(shared_add_ok, Value::Int(15)));

        let shared_get_after_add = call_native_function(
            &mut interpreter,
            "shared_get",
            &[Value::Str(Arc::new(shared_key.clone()))],
        );
        assert!(matches!(shared_get_after_add, Value::Int(15)));

        let shared_delete_missing = call_native_function(&mut interpreter, "shared_delete", &[]);
        assert!(
            matches!(shared_delete_missing, Value::Error(message) if message.contains("shared_delete requires one key argument"))
        );

        let shared_delete_ok = call_native_function(
            &mut interpreter,
            "shared_delete",
            &[Value::Str(Arc::new(shared_key.clone()))],
        );
        assert!(matches!(shared_delete_ok, Value::Bool(true)));

        let shared_has_false = call_native_function(
            &mut interpreter,
            "shared_has",
            &[Value::Str(Arc::new(shared_key.clone()))],
        );
        assert!(matches!(shared_has_false, Value::Bool(false)));

        let get_task_pool_with_args =
            call_native_function(&mut interpreter, "get_task_pool_size", &[Value::Int(1)]);
        assert!(
            matches!(get_task_pool_with_args, Value::Error(message) if message.contains("expects 0 arguments"))
        );

        let set_task_pool_missing =
            call_native_function(&mut interpreter, "set_task_pool_size", &[]);
        assert!(
            matches!(set_task_pool_missing, Value::Error(message) if message.contains("expects 1 argument"))
        );

        let set_task_pool_non_positive =
            call_native_function(&mut interpreter, "set_task_pool_size", &[Value::Int(0)]);
        assert!(
            matches!(set_task_pool_non_positive, Value::Error(message) if message.contains("size must be > 0"))
        );

        let set_task_pool_bad_type = call_native_function(
            &mut interpreter,
            "set_task_pool_size",
            &[Value::Str(Arc::new("4".to_string()))],
        );
        assert!(
            matches!(set_task_pool_bad_type, Value::Error(message) if message.contains("requires an integer size argument"))
        );

        let initial_pool_size = call_native_function(&mut interpreter, "get_task_pool_size", &[]);
        let initial_pool_size_value = match initial_pool_size {
            Value::Int(size) => {
                assert!(size > 0);
                size
            }
            other => panic!("Expected Int from get_task_pool_size(), got {:?}", other),
        };

        let set_task_pool_same = call_native_function(
            &mut interpreter,
            "set_task_pool_size",
            &[Value::Int(initial_pool_size_value)],
        );
        assert!(matches!(set_task_pool_same, Value::Int(previous) if previous > 0));

        let final_pool_size = call_native_function(&mut interpreter, "get_task_pool_size", &[]);
        assert!(matches!(final_pool_size, Value::Int(size) if size > 0));
    }

    #[test]
    fn test_release_hardening_ssg_render_pages_dispatch_contracts() {
        let mut interpreter = Interpreter::new();

        let missing_args = call_native_function(&mut interpreter, "ssg_render_pages", &[]);
        assert!(
            matches!(missing_args, Value::Error(message) if message.contains("ssg_render_pages() expects 1 argument"))
        );

        let non_array = call_native_function(
            &mut interpreter,
            "ssg_render_pages",
            &[Value::Str(Arc::new("not-an-array".to_string()))],
        );
        assert!(
            matches!(non_array, Value::Error(message) if message.contains("requires an array of source page strings"))
        );

        let non_string_element = call_native_function(
            &mut interpreter,
            "ssg_render_pages",
            &[Value::Array(Arc::new(vec![Value::Int(1)]))],
        );
        assert!(
            matches!(non_string_element, Value::Error(message) if message.contains("source page at index 0 must be a string"))
        );

        let success = call_native_function(
            &mut interpreter,
            "ssg_render_pages",
            &[Value::Array(Arc::new(vec![
                Value::Str(Arc::new("# Post A".to_string())),
                Value::Str(Arc::new("# Post B".to_string())),
            ]))],
        );

        match success {
            Value::Dict(result) => {
                assert!(
                    matches!(result.get("pages"), Some(Value::Array(pages)) if pages.len() == 2)
                );
                assert!(
                    matches!(result.get("checksum"), Some(Value::Int(checksum)) if *checksum > 0)
                );
            }
            other => panic!("Expected Dict from ssg_render_pages, got {:?}", other),
        }

        let paths_missing_args =
            call_native_function(&mut interpreter, "ssg_build_output_paths", &[]);
        assert!(
            matches!(paths_missing_args, Value::Error(message) if message.contains("expects 2 or 3 arguments"))
        );

        let paths_bad_dir = call_native_function(
            &mut interpreter,
            "ssg_build_output_paths",
            &[Value::Int(1), Value::Int(2)],
        );
        assert!(
            matches!(paths_bad_dir, Value::Error(message) if message.contains("output_dir must be a string"))
        );

        let paths_bad_count = call_native_function(
            &mut interpreter,
            "ssg_build_output_paths",
            &[Value::Str(Arc::new("tmp/out".to_string())), Value::Int(-1)],
        );
        assert!(
            matches!(paths_bad_count, Value::Error(message) if message.contains("file_count must be >= 0"))
        );

        let paths_success = call_native_function(
            &mut interpreter,
            "ssg_build_output_paths",
            &[
                Value::Str(Arc::new("tmp/out".to_string())),
                Value::Int(2),
                Value::Str(Arc::new(".html".to_string())),
            ],
        );

        match paths_success {
            Value::Array(paths) => {
                assert_eq!(paths.len(), 2);
                assert!(
                    matches!(&paths[0], Value::Str(path) if path.as_ref() == "tmp/out/post_0.html")
                );
                assert!(
                    matches!(&paths[1], Value::Str(path) if path.as_ref() == "tmp/out/post_1.html")
                );
            }
            other => panic!("Expected Array from ssg_build_output_paths, got {:?}", other),
        }

        let render_write_missing_args =
            call_native_function(&mut interpreter, "ssg_render_and_write_pages", &[]);
        assert!(
            matches!(render_write_missing_args, Value::Error(message) if message.contains("expects 2 or 3 arguments"))
        );

        let render_write_bad_first_arg = call_native_function(
            &mut interpreter,
            "ssg_render_and_write_pages",
            &[Value::Int(1), Value::Str(Arc::new("tmp/out".to_string()))],
        );
        assert!(
            matches!(render_write_bad_first_arg, Value::Error(message) if message.contains("first argument must be an array"))
        );

        let render_write_bad_second_arg = call_native_function(
            &mut interpreter,
            "ssg_render_and_write_pages",
            &[Value::Array(Arc::new(vec![])), Value::Int(1)],
        );
        assert!(
            matches!(render_write_bad_second_arg, Value::Error(message) if message.contains("second argument must be a string output_dir"))
        );

        let render_write_bad_limit = call_native_function(
            &mut interpreter,
            "ssg_render_and_write_pages",
            &[
                Value::Array(Arc::new(vec![Value::Str(Arc::new("# Post 0".to_string()))])),
                Value::Str(Arc::new("tmp/out".to_string())),
                Value::Int(0),
            ],
        );
        assert!(
            matches!(render_write_bad_limit, Value::Error(message) if message.contains("concurrency_limit must be > 0"))
        );

        let temp_dir = std::env::temp_dir().join(format!(
            "kujo_ssg_render_write_dispatch_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).expect("dispatch temp dir should be created");

        let render_write_success = call_native_function(
            &mut interpreter,
            "ssg_render_and_write_pages",
            &[
                Value::Array(Arc::new(vec![
                    Value::Str(Arc::new("# Post 0".to_string())),
                    Value::Str(Arc::new("# Post 1".to_string())),
                ])),
                Value::Str(Arc::new(temp_dir.to_string_lossy().to_string())),
                Value::Int(2),
            ],
        );

        let resolved_render_write = await_native_promise(render_write_success);
        match resolved_render_write {
            Ok(Value::Dict(result)) => {
                assert!(matches!(result.get("files"), Some(Value::Int(count)) if *count == 2));
                assert!(
                    matches!(result.get("checksum"), Some(Value::Int(checksum)) if *checksum > 0)
                );
            }
            other => panic!(
                "Expected Dict result from ssg_render_and_write_pages promise, got {:?}",
                other
            ),
        }

        let _ = std::fs::remove_file(temp_dir.join("post_0.html"));
        let _ = std::fs::remove_file(temp_dir.join("post_1.html"));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let read_render_missing_args =
            call_native_function(&mut interpreter, "ssg_read_render_and_write_pages", &[]);
        assert!(
            matches!(read_render_missing_args, Value::Error(message) if message.contains("expects 2 or 3 arguments"))
        );

        let read_render_bad_first_arg = call_native_function(
            &mut interpreter,
            "ssg_read_render_and_write_pages",
            &[Value::Int(1), Value::Str(Arc::new("tmp/out".to_string()))],
        );
        assert!(
            matches!(read_render_bad_first_arg, Value::Error(message) if message.contains("first argument must be an array"))
        );

        let read_render_bad_second_arg = call_native_function(
            &mut interpreter,
            "ssg_read_render_and_write_pages",
            &[Value::Array(Arc::new(vec![])), Value::Int(1)],
        );
        assert!(
            matches!(read_render_bad_second_arg, Value::Error(message) if message.contains("second argument must be a string output_dir"))
        );

        let read_render_bad_limit = call_native_function(
            &mut interpreter,
            "ssg_read_render_and_write_pages",
            &[
                Value::Array(Arc::new(vec![Value::Str(Arc::new("tmp/in/post_0.md".to_string()))])),
                Value::Str(Arc::new("tmp/out".to_string())),
                Value::Int(0),
            ],
        );
        assert!(
            matches!(read_render_bad_limit, Value::Error(message) if message.contains("concurrency_limit must be > 0"))
        );

        let read_render_input_dir = std::env::temp_dir().join(format!(
            "kujo_ssg_read_render_dispatch_input_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_millis()
        ));
        let read_render_output_dir = std::env::temp_dir().join(format!(
            "kujo_ssg_read_render_dispatch_output_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_millis()
        ));

        std::fs::create_dir_all(&read_render_input_dir)
            .expect("read/render dispatch input dir should be created");
        std::fs::create_dir_all(&read_render_output_dir)
            .expect("read/render dispatch output dir should be created");

        let source_path = read_render_input_dir.join("post_0.md");
        std::fs::write(&source_path, "# Post 0")
            .expect("read/render dispatch source file should be written");

        let read_render_success = call_native_function(
            &mut interpreter,
            "ssg_read_render_and_write_pages",
            &[
                Value::Array(Arc::new(vec![Value::Str(Arc::new(
                    source_path.to_string_lossy().to_string(),
                ))])),
                Value::Str(Arc::new(read_render_output_dir.to_string_lossy().to_string())),
                Value::Int(1),
            ],
        );

        let resolved_read_render = await_native_promise(read_render_success);
        match resolved_read_render {
            Ok(Value::Dict(result)) => {
                assert!(matches!(result.get("files"), Some(Value::Int(count)) if *count == 1));
                assert!(
                    matches!(result.get("checksum"), Some(Value::Int(checksum)) if *checksum > 0)
                );
                assert!(matches!(result.get("read_ms"), Some(Value::Float(ms)) if *ms >= 0.0));
                assert!(
                    matches!(result.get("render_write_ms"), Some(Value::Float(ms)) if *ms >= 0.0)
                );
            }
            other => panic!(
                "Expected Dict result from ssg_read_render_and_write_pages promise, got {:?}",
                other
            ),
        }

        let _ = std::fs::remove_file(source_path);
        let _ = std::fs::remove_file(read_render_output_dir.join("post_0.html"));
        let _ = std::fs::remove_dir_all(&read_render_input_dir);
        let _ = std::fs::remove_dir_all(&read_render_output_dir);
    }

    #[test]
    fn test_release_hardening_load_image_dispatch_contracts() {
        let mut interpreter = Interpreter::new();

        if !cfg!(feature = "runtime-image") {
            let disabled = call_native_function(&mut interpreter, "load_image", &[]);
            assert!(
                matches!(disabled, Value::Error(message) if message.contains("disabled in this build"))
            );
            return;
        }

        let load_image_missing_args = call_native_function(&mut interpreter, "load_image", &[]);
        assert!(
            matches!(load_image_missing_args, Value::Error(message) if message.contains("load_image requires a string path argument"))
        );

        let load_image_wrong_type =
            call_native_function(&mut interpreter, "load_image", &[Value::Int(1)]);
        assert!(
            matches!(load_image_wrong_type, Value::Error(message) if message.contains("load_image requires a string path argument"))
        );

        let load_image_extra_arg = call_native_function(
            &mut interpreter,
            "load_image",
            &[Value::Str(Arc::new("/tmp/kujo-image.png".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(load_image_extra_arg, Value::Error(message) if message.contains("load_image requires a string path argument"))
        );

        let missing_path = tmp_test_path("dispatch_missing_image.png");
        let load_image_missing_path = call_native_function(
            &mut interpreter,
            "load_image",
            &[Value::Str(Arc::new(missing_path.clone()))],
        );
        assert!(
            matches!(load_image_missing_path, Value::Error(message) if message.contains("Cannot load image") && message.contains(missing_path.as_str()))
        );
    }

    #[test]
    fn test_release_hardening_load_image_round_trip_behavior() {
        let mut interpreter = Interpreter::new();
        let image_path = tmp_test_path("dispatch_load_image.png");

        let mut image_buffer = image::RgbaImage::new(2, 1);
        image_buffer.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        image_buffer.put_pixel(1, 0, image::Rgba([0, 255, 0, 255]));
        image_buffer.save(&image_path).expect("seed image should be written");

        let load_result = call_native_function(
            &mut interpreter,
            "load_image",
            &[Value::Str(Arc::new(image_path.clone()))],
        );

        match load_result {
            Value::Image { data, format } => {
                assert_eq!(format, "png");
                let image_data = data.lock().expect("image mutex should lock");
                assert_eq!(image_data.width(), 2);
                assert_eq!(image_data.height(), 1);
            }
            other => panic!("Expected Value::Image, got {:?}", other),
        }

        let _ = std::fs::remove_file(image_path);
    }

    #[test]
    fn test_release_hardening_gif_to_webp_dispatch_contracts() {
        let mut interpreter = Interpreter::new();

        let missing_args = call_native_function(&mut interpreter, "gif_to_webp", &[]);
        assert!(matches!(
            missing_args,
            Value::Error(message) if message.contains("gif_to_webp requires 2 to 5 arguments")
        ));

        let wrong_input_type =
            call_native_function(&mut interpreter, "gif_to_webp", &[Value::Int(1), Value::Int(2)]);
        assert!(matches!(
            wrong_input_type,
            Value::Error(message) if message.contains("string input_path")
        ));

        let wrong_output_type = call_native_function(
            &mut interpreter,
            "gif_to_webp",
            &[Value::Str(Arc::new("in.gif".to_string())), Value::Int(2)],
        );
        assert!(matches!(
            wrong_output_type,
            Value::Error(message) if message.contains("string output_path")
        ));

        let wrong_quality_type = call_native_function(
            &mut interpreter,
            "gif_to_webp",
            &[
                Value::Str(Arc::new("in.gif".to_string())),
                Value::Str(Arc::new("out.webp".to_string())),
                Value::Bool(true),
            ],
        );
        assert!(matches!(
            wrong_quality_type,
            Value::Error(message) if message.contains("quality must be numeric")
        ));

        let out_of_range_quality = call_native_function(
            &mut interpreter,
            "gif_to_webp",
            &[
                Value::Str(Arc::new("in.gif".to_string())),
                Value::Str(Arc::new("out.webp".to_string())),
                Value::Int(999),
            ],
        );
        assert!(matches!(
            out_of_range_quality,
            Value::Error(message) if message.contains("quality must be in range 0-100")
        ));

        let out_of_range_method = call_native_function(
            &mut interpreter,
            "gif_to_webp",
            &[
                Value::Str(Arc::new("in.gif".to_string())),
                Value::Str(Arc::new("out.webp".to_string())),
                Value::Int(80),
                Value::Int(7),
            ],
        );
        assert!(matches!(
            out_of_range_method,
            Value::Error(message) if message.contains("method must be in range 0-6")
        ));
    }

    #[test]
    fn test_release_hardening_network_module_dispatch_argument_contracts() {
        let mut interpreter = Interpreter::new();

        let tcp_listen_missing = call_native_function(&mut interpreter, "tcp_listen", &[]);
        assert!(
            matches!(tcp_listen_missing, Value::Error(message) if message.contains("tcp_listen requires (string_host, int_port) arguments"))
        );

        let tcp_accept_missing = call_native_function(&mut interpreter, "tcp_accept", &[]);
        assert!(
            matches!(tcp_accept_missing, Value::Error(message) if message.contains("tcp_accept requires a TcpListener argument"))
        );

        let tcp_connect_missing = call_native_function(&mut interpreter, "tcp_connect", &[]);
        assert!(
            matches!(tcp_connect_missing, Value::Error(message) if message.contains("tcp_connect requires (string_host, int_port) arguments"))
        );

        let tcp_send_missing = call_native_function(&mut interpreter, "tcp_send", &[]);
        assert!(
            matches!(tcp_send_missing, Value::Error(message) if message.contains("tcp_send requires (TcpStream, string_or_bytes_data) arguments"))
        );

        let tcp_receive_missing = call_native_function(&mut interpreter, "tcp_receive", &[]);
        assert!(
            matches!(tcp_receive_missing, Value::Error(message) if message.contains("tcp_receive requires (TcpStream, int_size) arguments"))
        );

        let tcp_close_missing = call_native_function(&mut interpreter, "tcp_close", &[]);
        assert!(
            matches!(tcp_close_missing, Value::Error(message) if message.contains("tcp_close requires a TcpStream or TcpListener argument"))
        );

        let tcp_nonblocking_missing =
            call_native_function(&mut interpreter, "tcp_set_nonblocking", &[]);
        assert!(
            matches!(tcp_nonblocking_missing, Value::Error(message) if message.contains("tcp_set_nonblocking requires (TcpStream/TcpListener, bool) arguments"))
        );

        let tcp_info_missing = call_native_function(&mut interpreter, "tcp_info", &[]);
        assert!(
            matches!(tcp_info_missing, Value::Error(message) if message.contains("tcp_info requires a TcpStream argument"))
        );

        let tcp_timeouts_missing = call_native_function(&mut interpreter, "tcp_set_timeouts", &[]);
        assert!(
            matches!(tcp_timeouts_missing, Value::Error(message) if message.contains("tcp_set_timeouts requires (TcpStream, int_read_ms, int_write_ms) arguments"))
        );

        let udp_bind_missing = call_native_function(&mut interpreter, "udp_bind", &[]);
        assert!(
            matches!(udp_bind_missing, Value::Error(message) if message.contains("udp_bind requires (string_host, int_port) arguments"))
        );

        let udp_send_missing = call_native_function(&mut interpreter, "udp_send_to", &[]);
        assert!(
            matches!(udp_send_missing, Value::Error(message) if message.contains("udp_send_to requires (UdpSocket, string_or_bytes_data, string_host, int_port) arguments"))
        );

        let udp_receive_missing = call_native_function(&mut interpreter, "udp_receive_from", &[]);
        assert!(
            matches!(udp_receive_missing, Value::Error(message) if message.contains("udp_receive_from requires (UdpSocket, int_size) arguments"))
        );

        let udp_close_missing = call_native_function(&mut interpreter, "udp_close", &[]);
        assert!(
            matches!(udp_close_missing, Value::Error(message) if message.contains("udp_close requires a UdpSocket argument"))
        );
    }

    #[test]
    fn test_release_hardening_network_module_strict_arity_contracts() {
        let mut interpreter = Interpreter::new();

        let tcp_listen_extra = call_native_function(
            &mut interpreter,
            "tcp_listen",
            &[Value::Str(Arc::new("127.0.0.1".to_string())), Value::Int(80), Value::Int(1)],
        );
        assert!(
            matches!(tcp_listen_extra, Value::Error(message) if message.contains("tcp_listen requires (string_host, int_port) arguments"))
        );

        let tcp_accept_extra =
            call_native_function(&mut interpreter, "tcp_accept", &[Value::Int(1), Value::Int(2)]);
        assert!(
            matches!(tcp_accept_extra, Value::Error(message) if message.contains("tcp_accept requires a TcpListener argument"))
        );

        let tcp_connect_extra = call_native_function(
            &mut interpreter,
            "tcp_connect",
            &[Value::Str(Arc::new("127.0.0.1".to_string())), Value::Int(80), Value::Int(1)],
        );
        assert!(
            matches!(tcp_connect_extra, Value::Error(message) if message.contains("tcp_connect requires (string_host, int_port) arguments"))
        );

        let tcp_send_extra = call_native_function(
            &mut interpreter,
            "tcp_send",
            &[Value::Int(1), Value::Str(Arc::new("payload".to_string())), Value::Int(2)],
        );
        assert!(
            matches!(tcp_send_extra, Value::Error(message) if message.contains("tcp_send requires (TcpStream, string_or_bytes_data) arguments"))
        );

        let tcp_receive_extra = call_native_function(
            &mut interpreter,
            "tcp_receive",
            &[Value::Int(1), Value::Int(16), Value::Int(2)],
        );
        assert!(
            matches!(tcp_receive_extra, Value::Error(message) if message.contains("tcp_receive requires (TcpStream, int_size) arguments"))
        );

        let tcp_close_extra =
            call_native_function(&mut interpreter, "tcp_close", &[Value::Int(1), Value::Int(2)]);
        assert!(
            matches!(tcp_close_extra, Value::Error(message) if message.contains("tcp_close requires a TcpStream or TcpListener argument"))
        );

        let tcp_nonblocking_extra = call_native_function(
            &mut interpreter,
            "tcp_set_nonblocking",
            &[Value::Int(1), Value::Bool(true), Value::Int(2)],
        );
        assert!(
            matches!(tcp_nonblocking_extra, Value::Error(message) if message.contains("tcp_set_nonblocking requires (TcpStream/TcpListener, bool) arguments"))
        );

        let tcp_info_extra =
            call_native_function(&mut interpreter, "tcp_info", &[Value::Int(1), Value::Int(2)]);
        assert!(
            matches!(tcp_info_extra, Value::Error(message) if message.contains("tcp_info requires a TcpStream argument"))
        );

        let tcp_timeouts_extra = call_native_function(
            &mut interpreter,
            "tcp_set_timeouts",
            &[Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)],
        );
        assert!(
            matches!(tcp_timeouts_extra, Value::Error(message) if message.contains("tcp_set_timeouts requires (TcpStream, int_read_ms, int_write_ms) arguments"))
        );

        let udp_bind_extra = call_native_function(
            &mut interpreter,
            "udp_bind",
            &[Value::Str(Arc::new("127.0.0.1".to_string())), Value::Int(80), Value::Int(1)],
        );
        assert!(
            matches!(udp_bind_extra, Value::Error(message) if message.contains("udp_bind requires (string_host, int_port) arguments"))
        );

        let udp_send_extra = call_native_function(
            &mut interpreter,
            "udp_send_to",
            &[
                Value::Int(1),
                Value::Str(Arc::new("payload".to_string())),
                Value::Str(Arc::new("127.0.0.1".to_string())),
                Value::Int(80),
                Value::Int(2),
            ],
        );
        assert!(
            matches!(udp_send_extra, Value::Error(message) if message.contains("udp_send_to requires (UdpSocket, string_or_bytes_data, string_host, int_port) arguments"))
        );

        let udp_receive_extra = call_native_function(
            &mut interpreter,
            "udp_receive_from",
            &[Value::Int(1), Value::Int(16), Value::Int(2)],
        );
        assert!(
            matches!(udp_receive_extra, Value::Error(message) if message.contains("udp_receive_from requires (UdpSocket, int_size) arguments"))
        );

        let udp_close_extra =
            call_native_function(&mut interpreter, "udp_close", &[Value::Int(1), Value::Int(2)]);
        assert!(
            matches!(udp_close_extra, Value::Error(message) if message.contains("udp_close requires a UdpSocket argument"))
        );
    }

    #[test]
    fn test_release_hardening_network_module_size_limit_contracts() {
        let max_body_bytes = crate::network_policy::MAX_NETWORK_BODY_BYTES as i64;

        let Some(tcp_port) = available_tcp_port() else {
            eprintln!("Skipping TCP size-limit assertions: sandbox denied TCP bind permissions");
            return;
        };

        let mut server_interpreter = Interpreter::new();
        let listener = call_native_function(
            &mut server_interpreter,
            "tcp_listen",
            &[Value::Str(Arc::new("127.0.0.1".to_string())), Value::Int(tcp_port)],
        );
        let listener_value = match listener {
            Value::TcpListener { .. } => listener,
            other => panic!("Expected TcpListener from tcp_listen, got {:?}", other),
        };

        let client_thread = std::thread::spawn(move || {
            let mut client_interpreter = Interpreter::new();
            let stream = call_native_function(
                &mut client_interpreter,
                "tcp_connect",
                &[Value::Str(Arc::new("127.0.0.1".to_string())), Value::Int(tcp_port)],
            );
            let stream_value = match stream {
                Value::TcpStream { .. } => stream,
                other => panic!("Expected TcpStream from tcp_connect, got {:?}", other),
            };

            let close_result =
                call_native_function(&mut client_interpreter, "tcp_close", &[stream_value]);
            assert!(matches!(close_result, Value::Bool(true)));
        });

        let accepted =
            call_native_function(&mut server_interpreter, "tcp_accept", &[listener_value.clone()]);
        let accepted_stream = match accepted {
            Value::TcpStream { .. } => accepted,
            other => panic!("Expected TcpStream from tcp_accept, got {:?}", other),
        };

        let over_limit_receive = call_native_function(
            &mut server_interpreter,
            "tcp_receive",
            &[accepted_stream.clone(), Value::Int(max_body_bytes + 1)],
        );
        assert!(
            matches!(over_limit_receive, Value::Error(message) if message.contains("tcp_receive size exceeds maximum network body size"))
        );

        let close_stream =
            call_native_function(&mut server_interpreter, "tcp_close", &[accepted_stream]);
        assert!(matches!(close_stream, Value::Bool(true)));
        let close_listener =
            call_native_function(&mut server_interpreter, "tcp_close", &[listener_value]);
        assert!(matches!(close_listener, Value::Bool(true)));
        client_thread.join().expect("tcp client thread should complete");

        let Some(_) = available_udp_port() else {
            eprintln!("Skipping UDP size-limit assertions: sandbox denied UDP bind permissions");
            return;
        };
        let mut udp_interpreter = Interpreter::new();
        let receiver_socket = call_native_function(
            &mut udp_interpreter,
            "udp_bind",
            &[Value::Str(Arc::new("127.0.0.1".to_string())), Value::Int(0)],
        );
        let receiver_value = match receiver_socket {
            Value::UdpSocket { .. } => receiver_socket,
            other => panic!("Expected UdpSocket for receiver, got {:?}", other),
        };

        let over_limit_udp_receive = call_native_function(
            &mut udp_interpreter,
            "udp_receive_from",
            &[receiver_value.clone(), Value::Int(max_body_bytes + 1)],
        );
        assert!(
            matches!(over_limit_udp_receive, Value::Error(message) if message.contains("udp_receive_from size exceeds maximum network body size"))
        );

        let close_receiver =
            call_native_function(&mut udp_interpreter, "udp_close", &[receiver_value]);
        assert!(matches!(close_receiver, Value::Bool(true)));
    }

    #[test]
    fn test_release_hardening_network_module_round_trip_behaviors() {
        let Some(tcp_port) = available_tcp_port() else {
            eprintln!("Skipping network round-trip test: sandbox denied TCP bind permissions");
            return;
        };
        let mut server_interpreter = Interpreter::new();

        let listener = call_native_function(
            &mut server_interpreter,
            "tcp_listen",
            &[Value::Str(Arc::new("127.0.0.1".to_string())), Value::Int(tcp_port)],
        );
        let listener_value = match listener {
            Value::TcpListener { .. } => listener,
            other => panic!("Expected TcpListener from tcp_listen, got {:?}", other),
        };

        let listener_for_client = listener_value.clone();
        let client_thread = std::thread::spawn(move || {
            let mut client_interpreter = Interpreter::new();
            let stream = call_native_function(
                &mut client_interpreter,
                "tcp_connect",
                &[Value::Str(Arc::new("127.0.0.1".to_string())), Value::Int(tcp_port)],
            );
            let stream_value = match stream {
                Value::TcpStream { .. } => stream,
                other => panic!("Expected TcpStream from tcp_connect, got {:?}", other),
            };

            let sent = call_native_function(
                &mut client_interpreter,
                "tcp_send",
                &[stream_value.clone(), Value::Str(Arc::new("network-payload".to_string()))],
            );
            assert!(matches!(sent, Value::Int(15)));

            let close_result =
                call_native_function(&mut client_interpreter, "tcp_close", &[stream_value]);
            assert!(matches!(close_result, Value::Bool(true)));

            let close_listener_result =
                call_native_function(&mut client_interpreter, "tcp_close", &[listener_for_client]);
            assert!(matches!(close_listener_result, Value::Bool(true)));
        });

        let accepted =
            call_native_function(&mut server_interpreter, "tcp_accept", &[listener_value.clone()]);
        let accepted_stream = match accepted {
            Value::TcpStream { .. } => accepted,
            other => panic!("Expected TcpStream from tcp_accept, got {:?}", other),
        };

        let tcp_receive = call_native_function(
            &mut server_interpreter,
            "tcp_receive",
            &[accepted_stream.clone(), Value::Int(128)],
        );
        assert!(matches!(tcp_receive, Value::Str(data) if data.as_ref() == "network-payload"));

        let nonblocking_stream_result = call_native_function(
            &mut server_interpreter,
            "tcp_set_nonblocking",
            &[accepted_stream.clone(), Value::Bool(false)],
        );
        assert!(matches!(nonblocking_stream_result, Value::Bool(true)));

        let nonblocking_listener_result = call_native_function(
            &mut server_interpreter,
            "tcp_set_nonblocking",
            &[listener_value.clone(), Value::Bool(false)],
        );
        assert!(matches!(nonblocking_listener_result, Value::Bool(true)));

        let invalid_timeout_result = call_native_function(
            &mut server_interpreter,
            "tcp_set_timeouts",
            &[accepted_stream.clone(), Value::Int(0), Value::Int(100)],
        );
        assert!(
            matches!(invalid_timeout_result, Value::Error(message) if message.contains("between 1 and 600000 milliseconds"))
        );

        let timeout_result = call_native_function(
            &mut server_interpreter,
            "tcp_set_timeouts",
            &[accepted_stream.clone(), Value::Int(250), Value::Int(500)],
        );
        assert!(matches!(timeout_result, Value::Bool(true)));

        let info_result =
            call_native_function(&mut server_interpreter, "tcp_info", &[accepted_stream.clone()]);
        let Value::Dict(info) = info_result else {
            panic!("Expected deterministic tcp_info dictionary")
        };
        assert!(
            matches!(info.get("schema"), Some(Value::Str(value)) if value.as_ref() == "kujo.tcp.info.v1")
        );
        assert!(
            matches!(info.get("peer_address"), Some(Value::Str(value)) if value.starts_with("127.0.0.1:"))
        );
        assert!(
            matches!(info.get("local_address"), Some(Value::Str(value)) if value.starts_with("127.0.0.1:"))
        );
        assert!(matches!(info.get("read_timeout_ms"), Some(Value::Int(250))));
        assert!(matches!(info.get("write_timeout_ms"), Some(Value::Int(500))));

        let close_stream_result =
            call_native_function(&mut server_interpreter, "tcp_close", &[accepted_stream]);
        assert!(matches!(close_stream_result, Value::Bool(true)));

        let close_listener_result =
            call_native_function(&mut server_interpreter, "tcp_close", &[listener_value]);
        assert!(matches!(close_listener_result, Value::Bool(true)));

        client_thread.join().expect("tcp client thread should complete");

        if available_udp_port().is_none() {
            eprintln!("Skipping UDP round-trip assertions: sandbox denied UDP bind permissions");
            return;
        }
        let mut udp_interpreter = Interpreter::new();

        let receiver_socket = call_native_function(
            &mut udp_interpreter,
            "udp_bind",
            &[Value::Str(Arc::new("127.0.0.1".to_string())), Value::Int(0)],
        );
        let (receiver_value, udp_port) = match receiver_socket {
            Value::UdpSocket { ref socket, .. } => {
                let port = socket
                    .lock()
                    .expect("udp socket lock should not be poisoned")
                    .local_addr()
                    .expect("udp socket should expose local addr")
                    .port() as i64;
                (receiver_socket, port)
            }
            other => panic!("Expected UdpSocket for receiver, got {:?}", other),
        };

        let sender_socket = call_native_function(
            &mut udp_interpreter,
            "udp_bind",
            &[Value::Str(Arc::new("127.0.0.1".to_string())), Value::Int(0)],
        );
        let sender_value = match sender_socket {
            Value::UdpSocket { .. } => sender_socket,
            other => panic!("Expected UdpSocket for sender, got {:?}", other),
        };

        let udp_sent = call_native_function(
            &mut udp_interpreter,
            "udp_send_to",
            &[
                sender_value.clone(),
                Value::Str(Arc::new("udp-payload".to_string())),
                Value::Str(Arc::new("127.0.0.1".to_string())),
                Value::Int(udp_port),
            ],
        );
        assert!(matches!(udp_sent, Value::Int(11)));

        let udp_received = call_native_function(
            &mut udp_interpreter,
            "udp_receive_from",
            &[receiver_value.clone(), Value::Int(128)],
        );

        match udp_received {
            Value::Dict(result) => {
                assert!(
                    matches!(result.get("data"), Some(Value::Str(data)) if data.as_ref() == "udp-payload")
                );
                assert!(matches!(result.get("size"), Some(Value::Int(11))));
                assert!(
                    matches!(result.get("from"), Some(Value::Str(from)) if from.as_ref().contains("127.0.0.1:"))
                );
            }
            other => panic!("Expected udp_receive_from dictionary result, got {:?}", other),
        }

        let udp_close_sender =
            call_native_function(&mut udp_interpreter, "udp_close", &[sender_value]);
        assert!(matches!(udp_close_sender, Value::Bool(true)));

        let udp_close_receiver =
            call_native_function(&mut udp_interpreter, "udp_close", &[receiver_value]);
        assert!(matches!(udp_close_receiver, Value::Bool(true)));
    }

    #[test]
    fn test_release_hardening_zip_module_dispatch_argument_contracts() {
        let mut interpreter = Interpreter::new();

        if !cfg!(feature = "runtime-archive") {
            let disabled = call_native_function(&mut interpreter, "zip_create", &[]);
            assert!(
                matches!(disabled, Value::Error(message) if message.contains("disabled in this build"))
            );
            return;
        }

        let zip_create_missing = call_native_function(&mut interpreter, "zip_create", &[]);
        assert!(
            matches!(zip_create_missing, Value::Error(message) if message.contains("zip_create requires a string path argument"))
        );

        let zip_add_file_missing = call_native_function(&mut interpreter, "zip_add_file", &[]);
        assert!(
            matches!(zip_add_file_missing, Value::Error(message) if message.contains("zip_add_file requires (ZipArchive, string_path) arguments"))
        );

        let zip_add_dir_missing = call_native_function(&mut interpreter, "zip_add_dir", &[]);
        assert!(
            matches!(zip_add_dir_missing, Value::Error(message) if message.contains("zip_add_dir requires (ZipArchive, string_path) arguments"))
        );

        let zip_close_missing = call_native_function(&mut interpreter, "zip_close", &[]);
        assert!(
            matches!(zip_close_missing, Value::Error(message) if message.contains("zip_close requires a ZipArchive argument"))
        );

        let unzip_missing = call_native_function(&mut interpreter, "unzip", &[]);
        assert!(
            matches!(unzip_missing, Value::Error(message) if message.contains("unzip requires (string_zip_path, string_output_dir) arguments"))
        );

        let zip_create_extra = call_native_function(
            &mut interpreter,
            "zip_create",
            &[Value::Str(Arc::new("archive.zip".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(zip_create_extra, Value::Error(message) if message.contains("zip_create requires a string path argument"))
        );

        let zip_add_file_extra = call_native_function(
            &mut interpreter,
            "zip_add_file",
            &[Value::Int(1), Value::Str(Arc::new("a.txt".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(zip_add_file_extra, Value::Error(message) if message.contains("zip_add_file requires (ZipArchive, string_path) arguments"))
        );

        let zip_add_dir_extra = call_native_function(
            &mut interpreter,
            "zip_add_dir",
            &[Value::Int(1), Value::Str(Arc::new("dir".to_string())), Value::Int(1)],
        );
        assert!(
            matches!(zip_add_dir_extra, Value::Error(message) if message.contains("zip_add_dir requires (ZipArchive, string_path) arguments"))
        );

        let zip_close_extra =
            call_native_function(&mut interpreter, "zip_close", &[Value::Int(1), Value::Int(1)]);
        assert!(
            matches!(zip_close_extra, Value::Error(message) if message.contains("zip_close requires a ZipArchive argument"))
        );

        let unzip_extra = call_native_function(
            &mut interpreter,
            "unzip",
            &[
                Value::Str(Arc::new("archive.zip".to_string())),
                Value::Str(Arc::new("out".to_string())),
                Value::Int(1),
            ],
        );
        assert!(
            matches!(unzip_extra, Value::Error(message) if message.contains("unzip requires (string_zip_path, string_output_dir) arguments"))
        );
    }

    #[test]
    fn test_release_hardening_zip_module_round_trip_behaviors() {
        let mut interpreter = Interpreter::new();
        let archive_path = tmp_test_path("dispatch_zip_roundtrip.zip");
        let source_file_path = tmp_test_path("dispatch_zip_source.txt");
        let source_dir_path = tmp_test_path("dispatch_zip_source_dir");
        let nested_file_path = format!("{}/nested.txt", source_dir_path);
        let unzip_output_path = tmp_test_path("dispatch_zip_unzip_output");

        std::fs::write(&source_file_path, "zip dispatch content")
            .expect("seed zip file should be written");
        std::fs::create_dir_all(&source_dir_path).expect("seed zip directory should be created");
        std::fs::write(&nested_file_path, "zip nested content")
            .expect("nested file should be written");

        let zip_archive = call_native_function(
            &mut interpreter,
            "zip_create",
            &[Value::Str(Arc::new(archive_path.clone()))],
        );
        let archive_value = match zip_archive {
            Value::ZipArchive { .. } => zip_archive,
            other => panic!("Expected zip archive from zip_create, got {:?}", other),
        };

        let add_file_result = call_native_function(
            &mut interpreter,
            "zip_add_file",
            &[archive_value.clone(), Value::Str(Arc::new(source_file_path.clone()))],
        );
        assert!(matches!(add_file_result, Value::Bool(true)));

        let add_dir_result = call_native_function(
            &mut interpreter,
            "zip_add_dir",
            &[archive_value.clone(), Value::Str(Arc::new(source_dir_path.clone()))],
        );
        assert!(matches!(add_dir_result, Value::Bool(true)));

        let close_result =
            call_native_function(&mut interpreter, "zip_close", &[archive_value.clone()]);
        assert!(matches!(close_result, Value::Bool(true)));

        let close_again_result =
            call_native_function(&mut interpreter, "zip_close", &[archive_value.clone()]);
        assert!(
            matches!(close_again_result, Value::Error(message) if message.contains("already been closed"))
        );

        let unzip_result = call_native_function(
            &mut interpreter,
            "unzip",
            &[
                Value::Str(Arc::new(archive_path.clone())),
                Value::Str(Arc::new(unzip_output_path.clone())),
            ],
        );

        match unzip_result {
            Value::Array(extracted_paths) => {
                assert!(!extracted_paths.is_empty());
            }
            other => panic!("Expected extracted file list from unzip, got {:?}", other),
        }

        let extracted_file = format!("{}/dispatch_zip_source.txt", unzip_output_path);
        let extracted_nested_file = format!("{}/nested.txt", unzip_output_path);
        let extracted_file_content =
            std::fs::read_to_string(&extracted_file).expect("extracted source file should exist");
        let extracted_nested_content = std::fs::read_to_string(&extracted_nested_file)
            .expect("extracted nested file should exist");
        assert_eq!(extracted_file_content, "zip dispatch content");
        assert_eq!(extracted_nested_content, "zip nested content");

        let _ = std::fs::remove_file(archive_path);
        let _ = std::fs::remove_file(source_file_path);
        let _ = std::fs::remove_dir_all(source_dir_path);
        let _ = std::fs::remove_dir_all(unzip_output_path);
    }

    #[test]
    fn test_release_hardening_crypto_module_dispatch_argument_contracts() {
        let mut interpreter = Interpreter::new();

        let sha_missing = call_native_function(&mut interpreter, "sha256", &[]);
        assert!(
            matches!(sha_missing, Value::Error(message) if message.contains("sha256 requires a string or bytes argument"))
        );

        let sha_extra = call_native_function(
            &mut interpreter,
            "sha256",
            &[
                Value::Str(std::sync::Arc::new("data".to_string())),
                Value::Str(std::sync::Arc::new("extra".to_string())),
            ],
        );
        assert!(
            matches!(sha_extra, Value::Error(message) if message.contains("sha256 requires a string or bytes argument"))
        );

        let sha_bytes = call_native_function(
            &mut interpreter,
            "sha256",
            &[Value::Bytes(vec![0, 1, 2, 0x80, 0xFF, 0])],
        );
        assert!(
            matches!(sha_bytes, Value::Str(message) if message.as_ref() == "5b35354055af6a5442460fc80a36f4c47cf5fe7cade16773e1c474a2d37e9a3d")
        );

        let sha_file_path = std::env::temp_dir().join("kujo_sha256_file_dispatch.bin");
        std::fs::write(&sha_file_path, [0u8, 1, 2, 0x80, 0xFF, 0]).expect("seed sha file");
        let sha_file = call_native_function(
            &mut interpreter,
            "sha256_file",
            &[Value::Str(std::sync::Arc::new(sha_file_path.to_string_lossy().to_string()))],
        );
        assert!(
            matches!(sha_file, Value::Str(message) if message.as_ref() == "5b35354055af6a5442460fc80a36f4c47cf5fe7cade16773e1c474a2d37e9a3d")
        );
        let _ = std::fs::remove_file(sha_file_path);

        let verify_missing = call_native_function(
            &mut interpreter,
            "verify_password",
            &[Value::Str(std::sync::Arc::new("only_one".to_string()))],
        );
        assert!(
            matches!(verify_missing, Value::Error(message) if message.contains("verify_password requires"))
        );

        let verify_extra = call_native_function(
            &mut interpreter,
            "verify_password",
            &[
                Value::Str(std::sync::Arc::new("pw".to_string())),
                Value::Str(std::sync::Arc::new("hash".to_string())),
                Value::Str(std::sync::Arc::new("extra".to_string())),
            ],
        );
        assert!(
            matches!(verify_extra, Value::Error(message) if message.contains("verify_password requires"))
        );

        let aes_missing = call_native_function(
            &mut interpreter,
            "aes_encrypt",
            &[Value::Str(std::sync::Arc::new("plaintext".to_string()))],
        );
        assert!(
            matches!(aes_missing, Value::Error(message) if message.contains("aes_encrypt requires"))
        );

        let aes_extra = call_native_function(
            &mut interpreter,
            "aes_encrypt",
            &[
                Value::Str(std::sync::Arc::new("plaintext".to_string())),
                Value::Str(std::sync::Arc::new("key".to_string())),
                Value::Str(std::sync::Arc::new("extra".to_string())),
            ],
        );
        assert!(
            matches!(aes_extra, Value::Error(message) if message.contains("aes_encrypt requires"))
        );

        let rsa_bad_size =
            call_native_function(&mut interpreter, "rsa_generate_keypair", &[Value::Int(1024)]);
        assert!(
            matches!(rsa_bad_size, Value::Error(message) if message.contains("RSA key size must be 2048 or 4096 bits"))
        );

        let rsa_keypair_extra = call_native_function(
            &mut interpreter,
            "rsa_generate_keypair",
            &[Value::Int(2048), Value::Str(std::sync::Arc::new("extra".to_string()))],
        );
        assert!(
            matches!(rsa_keypair_extra, Value::Error(message) if message.contains("rsa_generate_keypair requires"))
        );

        let rsa_verify_extra = call_native_function(
            &mut interpreter,
            "rsa_verify",
            &[
                Value::Str(std::sync::Arc::new("msg".to_string())),
                Value::Str(std::sync::Arc::new("sig".to_string())),
                Value::Str(std::sync::Arc::new("pubkey".to_string())),
                Value::Str(std::sync::Arc::new("extra".to_string())),
            ],
        );
        assert!(
            matches!(rsa_verify_extra, Value::Error(message) if message.contains("rsa_verify requires"))
        );
    }

    #[test]
    fn test_release_hardening_set_queue_stack_method_contracts() {
        let mut interpreter = Interpreter::new();

        let set_add = call_native_function(
            &mut interpreter,
            "set_add",
            &[Value::Set(vec![Value::Int(1)]), Value::Int(2)],
        );
        assert!(matches!(set_add, Value::Set(values) if values.len() == 2));

        let set_add_duplicate = call_native_function(
            &mut interpreter,
            "set_add",
            &[Value::Set(vec![Value::Int(1), Value::Int(2)]), Value::Int(2)],
        );
        assert!(matches!(set_add_duplicate, Value::Set(values) if values.len() == 2));

        let set_has_true = call_native_function(
            &mut interpreter,
            "set_has",
            &[Value::Set(vec![Value::Int(3), Value::Int(5)]), Value::Int(5)],
        );
        assert!(matches!(set_has_true, Value::Bool(true)));

        let set_has_false = call_native_function(
            &mut interpreter,
            "set_has",
            &[Value::Set(vec![Value::Int(3), Value::Int(5)]), Value::Int(7)],
        );
        assert!(matches!(set_has_false, Value::Bool(false)));

        let set_remove = call_native_function(
            &mut interpreter,
            "set_remove",
            &[Value::Set(vec![Value::Int(1), Value::Int(2), Value::Int(3)]), Value::Int(2)],
        );
        assert!(matches!(set_remove, Value::Set(values) if values.len() == 2));

        let set_union = call_native_function(
            &mut interpreter,
            "set_union",
            &[
                Value::Set(vec![Value::Int(1), Value::Int(2)]),
                Value::Set(vec![Value::Int(2), Value::Int(3)]),
            ],
        );
        assert!(matches!(set_union, Value::Set(values) if values.len() == 3));

        let set_intersect = call_native_function(
            &mut interpreter,
            "set_intersect",
            &[
                Value::Set(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
                Value::Set(vec![Value::Int(2), Value::Int(3), Value::Int(4)]),
            ],
        );
        assert!(matches!(set_intersect, Value::Set(values) if values.len() == 2
            && values.iter().any(|value| matches!(value, Value::Int(2)))
            && values.iter().any(|value| matches!(value, Value::Int(3)))));

        let set_difference = call_native_function(
            &mut interpreter,
            "set_difference",
            &[
                Value::Set(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
                Value::Set(vec![Value::Int(2), Value::Int(4)]),
            ],
        );
        assert!(matches!(set_difference, Value::Set(values) if values.len() == 2
            && values.iter().any(|value| matches!(value, Value::Int(1)))
            && values.iter().any(|value| matches!(value, Value::Int(3)))));

        let set_to_array = call_native_function(
            &mut interpreter,
            "set_to_array",
            &[Value::Set(vec![Value::Int(8), Value::Int(9)])],
        );
        assert!(matches!(set_to_array, Value::Array(values) if values.len() == 2));

        let queue_constructor = call_native_function(
            &mut interpreter,
            "Queue",
            &[Value::Array(Arc::new(vec![Value::Int(10), Value::Int(20)]))],
        );
        assert!(matches!(queue_constructor, Value::Queue(queue) if queue.len() == 2));

        let queue_enqueue = call_native_function(
            &mut interpreter,
            "queue_enqueue",
            &[Value::Queue(std::collections::VecDeque::from(vec![Value::Int(1)])), Value::Int(2)],
        );
        assert!(matches!(queue_enqueue, Value::Queue(queue) if queue.len() == 2));

        let queue_dequeue = call_native_function(
            &mut interpreter,
            "queue_dequeue",
            &[Value::Queue(std::collections::VecDeque::from(vec![Value::Int(1), Value::Int(2)]))],
        );
        assert!(matches!(queue_dequeue, Value::Array(values) if values.len() == 2
            && matches!(&values[0], Value::Queue(queue) if queue.len() == 1)
            && matches!(&values[1], Value::Int(1))));

        let queue_peek = call_native_function(
            &mut interpreter,
            "queue_peek",
            &[Value::Queue(std::collections::VecDeque::from(vec![Value::Int(42), Value::Int(99)]))],
        );
        assert!(matches!(queue_peek, Value::Int(42)));

        let queue_is_empty_false = call_native_function(
            &mut interpreter,
            "queue_is_empty",
            &[Value::Queue(std::collections::VecDeque::from(vec![Value::Int(1)]))],
        );
        assert!(matches!(queue_is_empty_false, Value::Bool(false)));

        let queue_is_empty_true = call_native_function(
            &mut interpreter,
            "queue_is_empty",
            &[Value::Queue(std::collections::VecDeque::new())],
        );
        assert!(matches!(queue_is_empty_true, Value::Bool(true)));

        let queue_to_array = call_native_function(
            &mut interpreter,
            "queue_to_array",
            &[Value::Queue(std::collections::VecDeque::from(vec![Value::Int(7), Value::Int(8)]))],
        );
        assert!(matches!(queue_to_array, Value::Array(values) if values.len() == 2
            && matches!(&values[0], Value::Int(7))
            && matches!(&values[1], Value::Int(8))));

        let queue_size_ok = call_native_function(
            &mut interpreter,
            "queue_size",
            &[Value::Queue(std::collections::VecDeque::from(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
            ]))],
        );
        assert!(matches!(queue_size_ok, Value::Int(3)));

        let queue_size_bad_type =
            call_native_function(&mut interpreter, "queue_size", &[Value::Int(1)]);
        assert!(
            matches!(queue_size_bad_type, Value::Error(message) if message.contains("queue_size requires a Queue argument"))
        );

        let stack_constructor = call_native_function(
            &mut interpreter,
            "Stack",
            &[Value::Array(Arc::new(vec![Value::Int(3), Value::Int(4)]))],
        );
        assert!(matches!(stack_constructor, Value::Stack(values) if values.len() == 2));

        let stack_push = call_native_function(
            &mut interpreter,
            "stack_push",
            &[Value::Stack(vec![Value::Int(1)]), Value::Int(2)],
        );
        assert!(matches!(stack_push, Value::Stack(values) if values.len() == 2));

        let stack_pop = call_native_function(
            &mut interpreter,
            "stack_pop",
            &[Value::Stack(vec![Value::Int(1), Value::Int(2)])],
        );
        assert!(matches!(stack_pop, Value::Array(values) if values.len() == 2
            && matches!(&values[0], Value::Stack(stack) if stack.len() == 1)
            && matches!(&values[1], Value::Int(2))));

        let stack_peek = call_native_function(
            &mut interpreter,
            "stack_peek",
            &[Value::Stack(vec![Value::Int(5), Value::Int(6)])],
        );
        assert!(matches!(stack_peek, Value::Int(6)));

        let stack_is_empty_false = call_native_function(
            &mut interpreter,
            "stack_is_empty",
            &[Value::Stack(vec![Value::Int(1)])],
        );
        assert!(matches!(stack_is_empty_false, Value::Bool(false)));

        let stack_is_empty_true =
            call_native_function(&mut interpreter, "stack_is_empty", &[Value::Stack(vec![])]);
        assert!(matches!(stack_is_empty_true, Value::Bool(true)));

        let stack_to_array = call_native_function(
            &mut interpreter,
            "stack_to_array",
            &[Value::Stack(vec![Value::Int(11), Value::Int(12)])],
        );
        assert!(matches!(stack_to_array, Value::Array(values) if values.len() == 2
            && matches!(&values[0], Value::Int(11))
            && matches!(&values[1], Value::Int(12))));

        let stack_size_ok = call_native_function(
            &mut interpreter,
            "stack_size",
            &[Value::Stack(vec![Value::Int(1), Value::Int(2), Value::Int(3)])],
        );
        assert!(matches!(stack_size_ok, Value::Int(3)));

        let stack_size_bad_type =
            call_native_function(&mut interpreter, "stack_size", &[Value::Int(1)]);
        assert!(
            matches!(stack_size_bad_type, Value::Error(message) if message.contains("stack_size requires a Stack argument"))
        );
    }

    #[test]
    fn test_release_hardening_set_constructor_dispatch_contracts() {
        let mut interpreter = Interpreter::new();

        let set_empty = call_native_function(&mut interpreter, "Set", &[]);
        assert!(matches!(set_empty, Value::Set(items) if items.is_empty()));

        let set_from_array = call_native_function(
            &mut interpreter,
            "Set",
            &[Value::Array(std::sync::Arc::new(vec![Value::Int(1), Value::Int(1), Value::Int(2)]))],
        );
        assert!(matches!(set_from_array, Value::Set(items) if items.len() == 2));

        let set_invalid_type = call_native_function(&mut interpreter, "Set", &[Value::Int(42)]);
        assert!(
            matches!(set_invalid_type, Value::Error(message) if message.contains("Set constructor requires an array argument"))
        );

        let set_too_many = call_native_function(
            &mut interpreter,
            "Set",
            &[Value::Array(std::sync::Arc::new(vec![])), Value::Int(1)],
        );
        assert!(
            matches!(set_too_many, Value::Error(message) if message.contains("Set constructor takes at most 1 argument"))
        );
    }

    #[test]
    fn test_vm_for_iterable_normalization_contracts() {
        let mut interpreter = Interpreter::new();

        let int_iterable =
            call_native_function(&mut interpreter, "__vm_for_iterable", &[Value::Int(3)]);
        assert!(matches!(int_iterable, Value::Array(values) if values.len() == 3
            && matches!(&values[0], Value::Int(0))
            && matches!(&values[1], Value::Int(1))
            && matches!(&values[2], Value::Int(2))));

        let float_iterable =
            call_native_function(&mut interpreter, "__vm_for_iterable", &[Value::Float(2.9)]);
        assert!(matches!(float_iterable, Value::Array(values) if values.len() == 2
            && matches!(&values[0], Value::Int(0))
            && matches!(&values[1], Value::Int(1))));

        let mut dict = std::collections::HashMap::default();
        dict.insert(std::sync::Arc::<str>::from("a"), Value::Int(1));
        let dict_iterable = call_native_function(
            &mut interpreter,
            "__vm_for_iterable",
            &[Value::Dict(Arc::new(dict))],
        );
        assert!(matches!(dict_iterable, Value::Array(values) if values.len() == 1
            && matches!(&values[0], Value::Str(value) if value.as_ref() == "a")));

        let str_iterable = call_native_function(
            &mut interpreter,
            "__vm_for_iterable",
            &[Value::Str(Arc::new("ab".to_string()))],
        );
        assert!(matches!(str_iterable, Value::Array(values) if values.len() == 2
            && matches!(&values[0], Value::Str(ch) if ch.as_ref() == "a")
            && matches!(&values[1], Value::Str(ch) if ch.as_ref() == "b")));
    }
}
