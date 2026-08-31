// File: src/interpreter/mod.rs
//
// Tree-walking interpreter for the Kujo programming language.
// Executes Kujo programs by traversing the Abstract Syntax Tree (AST).
//
// The interpreter maintains an environment (symbol table) for variables and
// functions, evaluates expressions to produce values, and executes statements
// to perform actions. It supports:
// - Variable binding and mutation
// - Function calls with lexical scoping
// - Enum variants and pattern matching
// - Error handling with try/except/throw
// - Control flow (if/else, loops, match)
// - Binary operations on numbers and strings
//
// Values in Kujo can be numbers, strings, tagged enum variants, functions,
// or error values for exception handling.

// Module structure
mod async_runtime;
mod capabilities;
mod control_flow;
mod environment;
pub(crate) mod native_functions;
mod test_runner;
mod value;

// Re-exports for backward compatibility
pub use async_runtime::AsyncRuntime;
pub use capabilities::{NativeCapability, RuntimeCapabilityPolicy};
pub use environment::{BindingKind, Environment};
// Test framework exports - used by CLI test command
#[allow(unused_imports)]
pub use test_runner::{TestCase, TestReport, TestResult, TestRunner};
// Database infrastructure - used by stub database.rs module
pub use value::{
    CallableArity, DenseIntDict, DenseIntDictInt, DenseIntDictIntFull, DictMap, IntDictMap,
    LeakyFunctionBody, Value,
};
#[cfg(feature = "runtime-db")]
#[allow(unused_imports)]
pub use value::{ConnectionPool, DatabaseConnection};

// Internal-only imports
use control_flow::ControlFlow;

use crate::ast::{Expr, Stmt};
use crate::builtins;
use crate::errors::{unsupported_struct_generator_method_message, KujoError};
use crate::http_request_utils;
use crate::module::ModuleLoader;
use crate::runtime_limits;

// Infrastructure imports for stub modules (crypto.rs, database.rs, network.rs)
// These will be used when stub modules are fully implemented
#[allow(unused_imports)]
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
#[allow(unused_imports)]
use md5::Md5;
#[cfg(feature = "runtime-db")]
#[allow(unused_imports)]
use mysql_async::{prelude::*, Conn as MysqlConn, Opts as MysqlOpts};
#[cfg(feature = "runtime-db")]
#[allow(unused_imports)]
use postgres::{Client as PostgresClient, NoTls};
#[cfg(feature = "runtime-db")]
#[allow(unused_imports)]
use rusqlite::Connection as SqliteConnection;
#[allow(unused_imports)]
use sha2::{Digest as Sha2Digest, Sha256};
#[allow(unused_imports)]
use std::collections::{HashMap, VecDeque};
#[allow(unused_imports)]
use std::fs::File;
#[allow(unused_imports)]
use std::io::Read;
use std::io::Write;
#[allow(unused_imports)]
use std::path::Path;
use std::sync::{Arc, Mutex};
#[cfg(feature = "runtime-archive")]
#[allow(unused_imports)]
use zip::{write::FileOptions, ZipArchive, ZipWriter};

pub const DEFAULT_ASYNC_TASK_POOL_SIZE: usize = 256;

fn lock_or_runtime_error<'a, T>(
    mutex: &'a Mutex<T>,
    context: &str,
) -> Result<std::sync::MutexGuard<'a, T>, Value> {
    mutex
        .lock()
        .map_err(|_| Value::Error(format!("{}: shared runtime state lock poisoned", context)))
}

#[derive(Clone, Debug)]
enum SpawnCapturedValue {
    Tagged { tag: String, fields: Vec<(String, SpawnCapturedValue)> },
    Int(i64),
    Float(f64),
    Str(String),
    Secret(String),
    Bool(bool),
    Null,
    Bytes(Vec<u8>),
    NativeFunction(String),
    Struct { name: String, fields: Vec<(String, SpawnCapturedValue)> },
    Array(Vec<SpawnCapturedValue>),
    Dict(Vec<(String, SpawnCapturedValue)>),
    FixedDict(Vec<(String, SpawnCapturedValue)>),
    IntDict(Vec<(i64, SpawnCapturedValue)>),
    DenseIntDict(Vec<SpawnCapturedValue>),
    DenseIntDictInt(Vec<Option<i64>>),
    DenseIntDictIntFull(Vec<i64>),
    Result { is_ok: bool, value: Box<SpawnCapturedValue> },
    Option { is_some: bool, value: Box<SpawnCapturedValue> },
}

#[cfg(test)]
mod runtime_limit_tests {
    use super::*;

    #[test]
    fn function_context_rejects_depth_at_limit() {
        let mut interpreter = Interpreter::new();
        interpreter.function_depth = runtime_limits::DEFAULT_MAX_INTERPRETER_CALL_DEPTH;

        let result = interpreter.with_function_context("limit_probe", |_interp| ());
        assert!(matches!(
            result,
            Err(Value::Error(message))
                if message.contains(&format!(
                    "Maximum call stack depth of {} exceeded",
                    runtime_limits::DEFAULT_MAX_INTERPRETER_CALL_DEPTH
                ))
        ));
    }

    #[test]
    fn function_context_rejects_call_stack_at_limit() {
        let mut interpreter = Interpreter::new();
        interpreter.call_stack =
            vec!["call".to_string(); runtime_limits::DEFAULT_MAX_INTERPRETER_CALL_DEPTH];

        let result = interpreter.with_function_context("stack_limit_probe", |_interp| ());
        assert!(matches!(
            result,
            Err(Value::Error(message))
                if message.contains(&format!(
                    "Maximum call stack depth of {} exceeded",
                    runtime_limits::DEFAULT_MAX_INTERPRETER_CALL_DEPTH
                ))
        ));
    }

    #[test]
    fn function_context_allows_boundary_minus_one() {
        let mut interpreter = Interpreter::new();
        interpreter.function_depth = runtime_limits::DEFAULT_MAX_INTERPRETER_CALL_DEPTH - 1;

        let result = interpreter.with_function_context("boundary_probe", |_interp| 42);
        assert!(matches!(result, Ok(42)));
        assert_eq!(
            interpreter.function_depth,
            runtime_limits::DEFAULT_MAX_INTERPRETER_CALL_DEPTH - 1
        );
    }
}

impl SpawnCapturedValue {
    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Tagged { tag, fields } => {
                let mut captured_fields = Vec::with_capacity(fields.len());
                for (field_name, field_value) in fields {
                    let captured_value = Self::from_value(field_value)?;
                    captured_fields.push((field_name.clone(), captured_value));
                }
                Some(SpawnCapturedValue::Tagged { tag: tag.clone(), fields: captured_fields })
            }
            Value::Int(number) => Some(SpawnCapturedValue::Int(*number)),
            Value::Float(number) => Some(SpawnCapturedValue::Float(*number)),
            Value::Str(text) => Some(SpawnCapturedValue::Str(text.as_ref().clone())),
            Value::Secret(text) => Some(SpawnCapturedValue::Secret(text.as_ref().clone())),
            Value::Bool(boolean) => Some(SpawnCapturedValue::Bool(*boolean)),
            Value::Null => Some(SpawnCapturedValue::Null),
            Value::Bytes(bytes) => Some(SpawnCapturedValue::Bytes(bytes.clone())),
            Value::NativeFunction(name) => Some(SpawnCapturedValue::NativeFunction(name.clone())),
            Value::Struct { name, fields } => {
                let mut captured_fields = Vec::with_capacity(fields.len());
                for (field_name, field_value) in fields {
                    let captured_value = Self::from_value(field_value)?;
                    captured_fields.push((field_name.clone(), captured_value));
                }
                Some(SpawnCapturedValue::Struct { name: name.clone(), fields: captured_fields })
            }
            Value::Array(elements) => {
                let mut captured_elements = Vec::with_capacity(elements.len());
                for element in elements.iter() {
                    captured_elements.push(Self::from_value(element)?);
                }
                Some(SpawnCapturedValue::Array(captured_elements))
            }
            Value::Dict(entries) => {
                let mut captured_entries = Vec::with_capacity(entries.len());
                for (key, dict_value) in entries.iter() {
                    captured_entries.push((key.to_string(), Self::from_value(dict_value)?));
                }
                Some(SpawnCapturedValue::Dict(captured_entries))
            }
            Value::FixedDict { keys, values } => {
                if keys.len() != values.len() {
                    return None;
                }

                let mut captured_entries = Vec::with_capacity(keys.len());
                for (key, dict_value) in keys.iter().zip(values.iter()) {
                    captured_entries.push((key.to_string(), Self::from_value(dict_value)?));
                }
                Some(SpawnCapturedValue::FixedDict(captured_entries))
            }
            Value::IntDict(entries) => {
                let mut captured_entries = Vec::with_capacity(entries.len());
                for (key, dict_value) in entries.iter() {
                    captured_entries.push((*key, Self::from_value(dict_value)?));
                }
                Some(SpawnCapturedValue::IntDict(captured_entries))
            }
            Value::DenseIntDict(values) => {
                let mut captured_values = Vec::with_capacity(values.len());
                for dict_value in values.iter() {
                    captured_values.push(Self::from_value(dict_value)?);
                }
                Some(SpawnCapturedValue::DenseIntDict(captured_values))
            }
            Value::DenseIntDictInt(values) => {
                Some(SpawnCapturedValue::DenseIntDictInt(values.as_ref().clone()))
            }
            Value::DenseIntDictIntFull(values) => {
                Some(SpawnCapturedValue::DenseIntDictIntFull(values.as_ref().clone()))
            }
            Value::Result { is_ok, value } => Some(SpawnCapturedValue::Result {
                is_ok: *is_ok,
                value: Box::new(Self::from_value(value)?),
            }),
            Value::Option { is_some, value } => Some(SpawnCapturedValue::Option {
                is_some: *is_some,
                value: Box::new(Self::from_value(value)?),
            }),
            _ => None,
        }
    }

    fn into_value(self) -> Value {
        match self {
            SpawnCapturedValue::Tagged { tag, fields } => {
                let mut value_fields = HashMap::with_capacity(fields.len());
                for (field_name, field_value) in fields {
                    value_fields.insert(field_name, field_value.into_value());
                }
                Value::Tagged { tag, fields: value_fields }
            }
            SpawnCapturedValue::Int(number) => Value::Int(number),
            SpawnCapturedValue::Float(number) => Value::Float(number),
            SpawnCapturedValue::Str(text) => Value::Str(Arc::new(text)),
            SpawnCapturedValue::Secret(text) => Value::Secret(Arc::new(text)),
            SpawnCapturedValue::Bool(boolean) => Value::Bool(boolean),
            SpawnCapturedValue::Null => Value::Null,
            SpawnCapturedValue::Bytes(bytes) => Value::Bytes(bytes),
            SpawnCapturedValue::NativeFunction(name) => Value::NativeFunction(name),
            SpawnCapturedValue::Struct { name, fields } => {
                let mut value_fields = HashMap::with_capacity(fields.len());
                for (field_name, field_value) in fields {
                    value_fields.insert(field_name, field_value.into_value());
                }
                Value::Struct { name, fields: value_fields }
            }
            SpawnCapturedValue::Array(elements) => {
                Value::Array(Arc::new(elements.into_iter().map(|v| v.into_value()).collect()))
            }
            SpawnCapturedValue::Dict(entries) => {
                let mut map = DictMap::default();
                for (key, dict_value) in entries {
                    map.insert(Arc::from(key), dict_value.into_value());
                }
                Value::Dict(Arc::new(map))
            }
            SpawnCapturedValue::FixedDict(entries) => {
                let mut keys = Vec::with_capacity(entries.len());
                let mut values = Vec::with_capacity(entries.len());
                for (key, dict_value) in entries {
                    keys.push(Arc::<str>::from(key));
                    values.push(dict_value.into_value());
                }
                Value::FixedDict { keys: Arc::new(keys), values }
            }
            SpawnCapturedValue::IntDict(entries) => {
                let mut map = IntDictMap::default();
                for (key, dict_value) in entries {
                    map.insert(key, dict_value.into_value());
                }
                Value::IntDict(Arc::new(map))
            }
            SpawnCapturedValue::DenseIntDict(values) => Value::DenseIntDict(Arc::new(
                values.into_iter().map(|dict_value| dict_value.into_value()).collect(),
            )),
            SpawnCapturedValue::DenseIntDictInt(values) => Value::DenseIntDictInt(Arc::new(values)),
            SpawnCapturedValue::DenseIntDictIntFull(values) => {
                Value::DenseIntDictIntFull(Arc::new(values))
            }
            SpawnCapturedValue::Result { is_ok, value } => {
                Value::Result { is_ok, value: Box::new(value.into_value()) }
            }
            SpawnCapturedValue::Option { is_some, value } => {
                Value::Option { is_some, value: Box::new(value.into_value()) }
            }
        }
    }
}

/// Main interpreter that executes Kujo programs
pub struct Interpreter {
    pub env: Environment,
    pub return_value: Option<Value>,
    control_flow: ControlFlow,
    function_depth: usize,
    loop_depth: usize,
    output: Option<Arc<Mutex<Vec<u8>>>>,
    pub source_file: Option<String>,
    pub source_lines: Vec<String>,
    pub module_loader: ModuleLoader,
    call_stack: Vec<String>, // Track function calls for stack traces
    async_task_pool_size: usize,
    capability_policy: RuntimeCapabilityPolicy,
}

impl Interpreter {
    pub fn canonical_native_function_name(name: &str) -> &str {
        match name {
            "println" => "print",
            "type_of" => "type",
            "str" => "to_string",
            "time" => "current_timestamp",
            "substr" => "substring",
            "pad_start" => "pad_left",
            "pad_end" => "pad_right",
            "now_utc_seconds" => "now_unix",
            other => other,
        }
    }

    pub fn native_function_capability(name: &str) -> Option<NativeCapability> {
        capabilities::capability_for_native_function(Self::canonical_native_function_name(name))
    }

    #[allow(dead_code)]
    pub fn native_function_capabilities(name: &str) -> Vec<NativeCapability> {
        let canonical_name = Self::canonical_native_function_name(name);
        let mut capabilities = Vec::new();
        if let Some(primary) = capabilities::capability_for_native_function(canonical_name) {
            capabilities.push(primary);
        }
        capabilities.extend_from_slice(capabilities::additional_capabilities_for_native_function(
            canonical_name,
        ));
        capabilities
    }

    pub fn native_function_arity(name: &str) -> Option<CallableArity> {
        Self::native_callable_arity(Self::canonical_native_function_name(name))
    }

    /// Creates a new interpreter with an empty environment
    pub fn new() -> Self {
        Self::with_capability_policy(RuntimeCapabilityPolicy::trusted())
    }

    pub fn with_capability_policy(capability_policy: RuntimeCapabilityPolicy) -> Self {
        let mut interpreter = Interpreter {
            env: Environment::default(),
            return_value: None,
            control_flow: ControlFlow::None,
            function_depth: 0,
            loop_depth: 0,
            output: None,
            source_file: None,
            source_lines: Vec::new(),
            module_loader: ModuleLoader::new(),
            call_stack: Vec::new(),
            async_task_pool_size: DEFAULT_ASYNC_TASK_POOL_SIZE,
            capability_policy,
        };

        // Register built-in functions and constants
        interpreter.register_builtins();

        interpreter
    }

    pub fn capability_policy(&self) -> &RuntimeCapabilityPolicy {
        &self.capability_policy
    }

    pub fn set_capability_policy(&mut self, capability_policy: RuntimeCapabilityPolicy) {
        self.capability_policy = capability_policy;
    }

    pub fn capability_error(capability: NativeCapability, surface: &str) -> Value {
        Value::Error(format!(
            "Capability denied: {} required for {}; rerun with {}",
            capability.as_str(),
            surface,
            capability.allow_flag()
        ))
    }

    pub fn require_capability(
        &self,
        capability: NativeCapability,
        surface: &str,
    ) -> Result<(), Value> {
        if self.capability_policy.allows(capability) {
            Ok(())
        } else {
            Err(Self::capability_error(capability, surface))
        }
    }

    /// Set the environment (used by VM to share environment)
    pub fn set_env(&mut self, env: Arc<Mutex<Environment>>) {
        // We need to extract the environment from the Mutex
        let locked_env = env.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        self.env = locked_env.clone();
    }

    /// Get the current call stack for error reporting
    pub fn get_call_stack(&self) -> Vec<String> {
        self.call_stack.clone()
    }

    pub fn get_async_task_pool_size(&self) -> usize {
        self.async_task_pool_size
    }

    pub fn set_async_task_pool_size(&mut self, size: usize) -> usize {
        let previous_size = self.async_task_pool_size;
        self.async_task_pool_size = size;
        previous_size
    }

    fn with_function_context<T>(
        &mut self,
        callable_name: &str,
        body: impl FnOnce(&mut Self) -> T,
    ) -> Result<T, Value> {
        let max_depth = runtime_limits::DEFAULT_MAX_INTERPRETER_CALL_DEPTH;
        if self.function_depth >= max_depth || self.call_stack.len() >= max_depth {
            return Err(Value::Error(format!(
                "Maximum call stack depth of {} exceeded while calling {}",
                max_depth, callable_name
            )));
        }

        self.function_depth += 1;
        let result = body(self);
        self.function_depth = self.function_depth.saturating_sub(1);
        Ok(result)
    }

    fn with_loop_context<T>(&mut self, body: impl FnOnce(&mut Self) -> T) -> T {
        self.loop_depth += 1;
        let result = body(self);
        self.loop_depth = self.loop_depth.saturating_sub(1);
        result
    }

    fn capture_spawn_bindings(&self) -> Vec<(String, SpawnCapturedValue)> {
        let mut merged_bindings: HashMap<String, SpawnCapturedValue> = HashMap::new();

        for scope in &self.env.scopes {
            for (name, value) in scope {
                if let Some(captured_value) = SpawnCapturedValue::from_value(value) {
                    merged_bindings.insert(name.clone(), captured_value);
                }
            }
        }

        merged_bindings.into_iter().collect()
    }

    /// Get all built-in function names (for VM initialization)
    /// This returns a list of all native functions that the interpreter supports
    pub fn get_builtin_names() -> Vec<&'static str> {
        vec![
            // I/O functions
            "print",
            "println",
            "eprint",
            // Math functions
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
            "bit_and",
            "bit_or",
            "bit_xor",
            "bit_not",
            "bit_shl",
            "bit_shr",
            "vec_dot",
            "vec_norm",
            "vec_normalize",
            "vec_cosine",
            "vec_top_k",
            "ai_count_tokens",
            "ai_fit_context",
            "ai_text",
            "ai_image_url",
            "ai_message",
            // String functions
            "len",
            "__vm_for_iterable",
            "substring",
            "substr",
            "to_upper",
            "upper",
            "to_lower",
            "lower",
            "capitalize",
            "escape_xml",
            "render_markdown",
            "render_listing_card",
            "render_layout_native",
            "trim",
            "trim_start",
            "trim_end",
            "contains",
            "replace_str",
            "replace",
            "split",
            "join",
            "ssg_render_pages",
            "ssg_build_output_paths",
            "ssg_render_and_write_pages",
            "ssg_read_render_and_write_pages",
            "starts_with",
            "ends_with",
            "pad_left",
            "pad_right",
            "pad_start",
            "pad_end",
            "lines",
            "words",
            "str_reverse",
            "slugify",
            "truncate",
            "to_camel_case",
            "to_snake_case",
            "to_kebab_case",
            "index_of",
            "repeat",
            "char_at",
            "is_empty",
            "count_chars",
            // Array functions
            "push",
            "append",
            "pop",
            "insert",
            "remove",
            "remove_at",
            "clear",
            "slice",
            "concat",
            // Array higher-order functions
            "map",
            "filter",
            "reduce",
            "find",
            // Array utility functions
            "sort",
            "reverse",
            "unique",
            "sum",
            "any",
            "all",
            // Advanced array methods
            "chunk",
            "flatten",
            "zip",
            "enumerate",
            "take",
            "skip",
            "windows",
            // Array generation functions
            "range",
            // String formatting functions
            "format",
            // Dict functions
            "keys",
            "values",
            "items",
            "has_key",
            "get",
            "merge",
            // Advanced dict methods
            "invert",
            "update",
            "get_default",
            // I/O functions
            "input",
            // Type conversion functions
            "parse_int",
            "parse_float",
            "to_int",
            "to_float",
            "to_string",
            "str",
            "secret",
            "reveal",
            "to_bool",
            "bytes",
            "dict",
            "array",
            "error",
            // Type checking functions
            "type",
            "type_of",
            "is_truthy",
            "is_int",
            "is_float",
            "is_string",
            "is_secret",
            "is_bool",
            "is_array",
            "is_dict",
            "is_null",
            "is_function",
            // Assert & Debug functions
            "assert",
            "debug",
            // File I/O functions
            "read_file",
            "read_file_lossy",
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
            // Binary file I/O functions
            "read_binary_file",
            "write_binary_file",
            // IO module functions (advanced binary operations)
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
            // JSON functions
            "parse_json",
            "to_json",
            "to_json_pretty",
            "json_schema_validate",
            // TOML functions
            "parse_toml",
            "to_toml",
            // YAML functions
            "parse_yaml",
            "to_yaml",
            // CSV functions
            "parse_csv",
            "to_csv",
            // Base64 encoding/decoding functions
            "encode_base64",
            "decode_base64",
            "encode_uri_component",
            // Random functions
            "random",
            "random_int",
            "random_choice",
            "uuid_v4",
            "random_id",
            "set_random_seed",
            "clear_random_seed",
            // Date/Time functions
            "now",
            "now_utc",
            "now_unix",
            "now_utc_seconds",
            "current_timestamp",
            "time",
            "performance_now",
            "time_us",
            "time_ns",
            "format_duration",
            "elapsed",
            "format_date",
            "parse_date",
            // System operation functions
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
            "exit",
            "sleep",
            "execute",
            "execute_status",
            // OS module functions
            "os_getcwd",
            "os_chdir",
            "os_rmdir",
            "os_environ",
            // Path operation functions
            "join_path",
            "dirname",
            "basename",
            "path_exists",
            "path_join",
            "path_absolute",
            "path_is_dir",
            "path_is_file",
            "path_is_symlink",
            "path_extension",
            // Regular expression functions
            "regex_match",
            "regex_find_all",
            "regex_replace",
            "regex_split",
            // HTTP client functions
            "http_get",
            "http_request",
            "http_post",
            "http_put",
            "http_delete",
            "http_get_binary",
            "http_download_file",
            "http_upload_file",
            "ai_request_hash",
            "ai_chat",
            "ai_stream_chat",
            "ai_embedding",
            "ai_tool_loop",
            // Concurrent HTTP functions
            "parallel_http",
            // JWT authentication functions
            "jwt_encode",
            "jwt_decode",
            "jwt_verify",
            // OAuth2 helper functions
            "oauth2_auth_url",
            "oauth2_get_token",
            // HTTP streaming functions
            "http_get_stream",
            // HTTP server functions
            "http_server",
            "http_listen",
            "http_response",
            "json_response",
            "html_response",
            "redirect_response",
            "set_header",
            "set_headers",
            // Database functions
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
            // Collection constructors and methods
            // Set
            "Set",
            "set_add",
            "set_has",
            "set_remove",
            "set_union",
            "set_intersect",
            "set_difference",
            "set_to_array",
            // Queue
            "Queue",
            "queue_enqueue",
            "queue_dequeue",
            "queue_peek",
            "queue_size",
            "queue_is_empty",
            "queue_to_array",
            // Stack
            "Stack",
            "stack_push",
            "stack_pop",
            "stack_peek",
            "stack_size",
            "stack_is_empty",
            "stack_to_array",
            // Concurrency functions
            "channel",
            "shared_set",
            "shared_get",
            "shared_has",
            "shared_delete",
            "shared_add_int",
            // Async operations
            "async_sleep",
            "async_timeout",
            "async_http_get",
            "async_http_post",
            "async_read_file",
            "async_read_files",
            "async_write_file",
            "async_write_files",
            "spawn_task",
            "await_task",
            "cancel_task",
            "Promise.all",
            "promise_all",
            "await_all",
            "promise_wait",
            "parallel_map",
            "par_map",
            "par_each",
            "set_task_pool_size",
            "get_task_pool_size",
            // Testing assertion functions
            "assert_equal",
            "assert_true",
            "assert_false",
            "assert_contains",
            // Image processing functions
            "load_image",
            "gif_to_webp",
            // Compression & Archive functions
            "zip_create",
            "zip_add_file",
            "zip_add_dir",
            "zip_close",
            "unzip",
            // Hashing & Cryptography functions
            "sha256",
            "hmac_sha256",
            "hmac_sha256_verify",
            "sha256_file",
            "md5",
            "md5_file",
            "hash_password",
            "verify_password",
            // Crypto functions
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
            // Process management functions
            "spawn_process",
            "pipe_commands",
            // Network functions
            "tcp_listen",
            "tcp_accept",
            "tcp_connect",
            "tcp_send",
            "tcp_receive",
            "tcp_close",
            "tcp_set_nonblocking",
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
        ]
    }

    /// Registers all built-in functions and constants
    fn register_builtins(&mut self) {
        // I/O functions
        self.env.define("print".to_string(), Value::NativeFunction("print".to_string()));
        self.env.define("println".to_string(), Value::NativeFunction("print".to_string()));
        self.env.define("eprint".to_string(), Value::NativeFunction("eprint".to_string()));

        // Legacy/null compatibility alias
        self.env.define("null".to_string(), Value::Null);

        // Math constants
        self.env.define("PI".to_string(), Value::Float(std::f64::consts::PI));
        self.env.define("E".to_string(), Value::Float(std::f64::consts::E));

        // Math functions
        self.env.define("abs".to_string(), Value::NativeFunction("abs".to_string()));
        self.env.define("sqrt".to_string(), Value::NativeFunction("sqrt".to_string()));
        self.env.define("pow".to_string(), Value::NativeFunction("pow".to_string()));
        self.env.define("floor".to_string(), Value::NativeFunction("floor".to_string()));
        self.env.define("ceil".to_string(), Value::NativeFunction("ceil".to_string()));
        self.env.define("round".to_string(), Value::NativeFunction("round".to_string()));
        self.env.define("min".to_string(), Value::NativeFunction("min".to_string()));
        self.env.define("max".to_string(), Value::NativeFunction("max".to_string()));
        self.env.define("sin".to_string(), Value::NativeFunction("sin".to_string()));
        self.env.define("cos".to_string(), Value::NativeFunction("cos".to_string()));
        self.env.define("tan".to_string(), Value::NativeFunction("tan".to_string()));
        self.env.define("log".to_string(), Value::NativeFunction("log".to_string()));
        self.env.define("exp".to_string(), Value::NativeFunction("exp".to_string()));
        self.env.define("bit_and".to_string(), Value::NativeFunction("bit_and".to_string()));
        self.env.define("bit_or".to_string(), Value::NativeFunction("bit_or".to_string()));
        self.env.define("bit_xor".to_string(), Value::NativeFunction("bit_xor".to_string()));
        self.env.define("bit_not".to_string(), Value::NativeFunction("bit_not".to_string()));
        self.env.define("bit_shl".to_string(), Value::NativeFunction("bit_shl".to_string()));
        self.env.define("bit_shr".to_string(), Value::NativeFunction("bit_shr".to_string()));
        self.env.define("vec_dot".to_string(), Value::NativeFunction("vec_dot".to_string()));
        self.env.define("vec_norm".to_string(), Value::NativeFunction("vec_norm".to_string()));
        self.env.define(
            "vec_normalize".to_string(),
            Value::NativeFunction("vec_normalize".to_string()),
        );
        self.env.define("vec_cosine".to_string(), Value::NativeFunction("vec_cosine".to_string()));
        self.env.define("vec_top_k".to_string(), Value::NativeFunction("vec_top_k".to_string()));
        self.env.define(
            "ai_count_tokens".to_string(),
            Value::NativeFunction("ai_count_tokens".to_string()),
        );
        self.env.define(
            "ai_fit_context".to_string(),
            Value::NativeFunction("ai_fit_context".to_string()),
        );
        self.env.define("ai_text".to_string(), Value::NativeFunction("ai_text".to_string()));
        self.env
            .define("ai_image_url".to_string(), Value::NativeFunction("ai_image_url".to_string()));
        self.env.define("ai_message".to_string(), Value::NativeFunction("ai_message".to_string()));

        // String functions
        self.env.define("len".to_string(), Value::NativeFunction("len".to_string()));
        self.env.define(
            "__vm_for_iterable".to_string(),
            Value::NativeFunction("__vm_for_iterable".to_string()),
        );
        self.env.define("substring".to_string(), Value::NativeFunction("substring".to_string()));
        self.env.define("substr".to_string(), Value::NativeFunction("substr".to_string()));
        self.env.define("to_upper".to_string(), Value::NativeFunction("to_upper".to_string()));
        self.env.define("upper".to_string(), Value::NativeFunction("upper".to_string())); // Alias
        self.env.define("to_lower".to_string(), Value::NativeFunction("to_lower".to_string()));
        self.env.define("lower".to_string(), Value::NativeFunction("lower".to_string())); // Alias
        self.env.define("capitalize".to_string(), Value::NativeFunction("capitalize".to_string()));
        self.env.define("escape_xml".to_string(), Value::NativeFunction("escape_xml".to_string()));
        self.env.define(
            "render_markdown".to_string(),
            Value::NativeFunction("render_markdown".to_string()),
        );
        self.env.define(
            "render_listing_card".to_string(),
            Value::NativeFunction("render_listing_card".to_string()),
        );
        self.env.define(
            "render_layout_native".to_string(),
            Value::NativeFunction("render_layout_native".to_string()),
        );
        self.env.define("trim".to_string(), Value::NativeFunction("trim".to_string()));
        self.env.define("trim_start".to_string(), Value::NativeFunction("trim_start".to_string()));
        self.env.define("trim_end".to_string(), Value::NativeFunction("trim_end".to_string()));
        self.env.define("contains".to_string(), Value::NativeFunction("contains".to_string()));
        self.env
            .define("replace_str".to_string(), Value::NativeFunction("replace_str".to_string()));
        self.env.define("replace".to_string(), Value::NativeFunction("replace".to_string())); // Alias
        self.env.define("split".to_string(), Value::NativeFunction("split".to_string()));
        self.env.define("join".to_string(), Value::NativeFunction("join".to_string()));
        self.env.define(
            "ssg_render_pages".to_string(),
            Value::NativeFunction("ssg_render_pages".to_string()),
        );
        self.env.define(
            "ssg_build_output_paths".to_string(),
            Value::NativeFunction("ssg_build_output_paths".to_string()),
        );
        self.env.define(
            "ssg_render_and_write_pages".to_string(),
            Value::NativeFunction("ssg_render_and_write_pages".to_string()),
        );
        self.env.define(
            "ssg_read_render_and_write_pages".to_string(),
            Value::NativeFunction("ssg_read_render_and_write_pages".to_string()),
        );
        self.env
            .define("starts_with".to_string(), Value::NativeFunction("starts_with".to_string()));
        self.env.define("ends_with".to_string(), Value::NativeFunction("ends_with".to_string()));
        self.env.define("index_of".to_string(), Value::NativeFunction("index_of".to_string()));
        self.env.define("repeat".to_string(), Value::NativeFunction("repeat".to_string()));
        self.env.define("char_at".to_string(), Value::NativeFunction("char_at".to_string()));
        self.env.define("is_empty".to_string(), Value::NativeFunction("is_empty".to_string()));
        self.env
            .define("count_chars".to_string(), Value::NativeFunction("count_chars".to_string()));

        // Advanced string methods
        self.env.define("pad_left".to_string(), Value::NativeFunction("pad_left".to_string()));
        self.env.define("pad_right".to_string(), Value::NativeFunction("pad_right".to_string()));
        self.env.define("pad_start".to_string(), Value::NativeFunction("pad_left".to_string()));
        self.env.define("pad_end".to_string(), Value::NativeFunction("pad_right".to_string()));
        self.env.define("lines".to_string(), Value::NativeFunction("lines".to_string()));
        self.env.define("words".to_string(), Value::NativeFunction("words".to_string()));
        self.env
            .define("str_reverse".to_string(), Value::NativeFunction("str_reverse".to_string()));
        self.env.define("slugify".to_string(), Value::NativeFunction("slugify".to_string()));
        self.env.define("truncate".to_string(), Value::NativeFunction("truncate".to_string()));
        self.env.define(
            "to_camel_case".to_string(),
            Value::NativeFunction("to_camel_case".to_string()),
        );
        self.env.define(
            "to_snake_case".to_string(),
            Value::NativeFunction("to_snake_case".to_string()),
        );
        self.env.define(
            "to_kebab_case".to_string(),
            Value::NativeFunction("to_kebab_case".to_string()),
        );

        // Array functions
        self.env.define("push".to_string(), Value::NativeFunction("push".to_string()));
        self.env.define("append".to_string(), Value::NativeFunction("append".to_string())); // Alias
        self.env.define("pop".to_string(), Value::NativeFunction("pop".to_string()));
        self.env.define("insert".to_string(), Value::NativeFunction("insert".to_string()));
        self.env.define("remove".to_string(), Value::NativeFunction("remove".to_string()));
        self.env.define("remove_at".to_string(), Value::NativeFunction("remove_at".to_string()));
        self.env.define("clear".to_string(), Value::NativeFunction("clear".to_string()));
        self.env.define("slice".to_string(), Value::NativeFunction("slice".to_string()));
        self.env.define("concat".to_string(), Value::NativeFunction("concat".to_string()));

        // Array higher-order functions
        self.env.define("map".to_string(), Value::NativeFunction("map".to_string()));
        self.env.define("filter".to_string(), Value::NativeFunction("filter".to_string()));
        self.env.define("reduce".to_string(), Value::NativeFunction("reduce".to_string()));
        self.env.define("find".to_string(), Value::NativeFunction("find".to_string()));

        // Array utility functions
        self.env.define("sort".to_string(), Value::NativeFunction("sort".to_string()));
        self.env.define("reverse".to_string(), Value::NativeFunction("reverse".to_string()));
        self.env.define("unique".to_string(), Value::NativeFunction("unique".to_string()));
        self.env.define("sum".to_string(), Value::NativeFunction("sum".to_string()));
        self.env.define("any".to_string(), Value::NativeFunction("any".to_string()));
        self.env.define("all".to_string(), Value::NativeFunction("all".to_string()));

        // Advanced array methods
        self.env.define("chunk".to_string(), Value::NativeFunction("chunk".to_string()));
        self.env.define("flatten".to_string(), Value::NativeFunction("flatten".to_string()));
        self.env.define("zip".to_string(), Value::NativeFunction("zip".to_string()));
        self.env.define("enumerate".to_string(), Value::NativeFunction("enumerate".to_string()));
        self.env.define("take".to_string(), Value::NativeFunction("take".to_string()));
        self.env.define("skip".to_string(), Value::NativeFunction("skip".to_string()));
        self.env.define("windows".to_string(), Value::NativeFunction("windows".to_string()));

        // Array generation functions
        self.env.define("range".to_string(), Value::NativeFunction("range".to_string()));

        // String formatting functions
        self.env.define("format".to_string(), Value::NativeFunction("format".to_string()));

        // Dict functions
        self.env.define("keys".to_string(), Value::NativeFunction("keys".to_string()));
        self.env.define("values".to_string(), Value::NativeFunction("values".to_string()));
        self.env.define("items".to_string(), Value::NativeFunction("items".to_string()));
        self.env.define("has_key".to_string(), Value::NativeFunction("has_key".to_string()));
        self.env.define("get".to_string(), Value::NativeFunction("get".to_string()));
        self.env.define("remove".to_string(), Value::NativeFunction("remove".to_string()));
        self.env.define("clear".to_string(), Value::NativeFunction("clear".to_string()));
        self.env.define("merge".to_string(), Value::NativeFunction("merge".to_string()));

        // Advanced dict methods
        self.env.define("invert".to_string(), Value::NativeFunction("invert".to_string()));
        self.env.define("update".to_string(), Value::NativeFunction("update".to_string()));
        self.env
            .define("get_default".to_string(), Value::NativeFunction("get_default".to_string()));

        // I/O functions
        self.env.define("input".to_string(), Value::NativeFunction("input".to_string()));

        // Type conversion functions
        self.env.define("parse_int".to_string(), Value::NativeFunction("parse_int".to_string()));
        self.env
            .define("parse_float".to_string(), Value::NativeFunction("parse_float".to_string()));
        self.env.define("to_int".to_string(), Value::NativeFunction("to_int".to_string()));
        self.env.define("to_float".to_string(), Value::NativeFunction("to_float".to_string()));
        self.env.define("to_string".to_string(), Value::NativeFunction("to_string".to_string()));
        self.env.define("str".to_string(), Value::NativeFunction("to_string".to_string()));
        self.env.define("secret".to_string(), Value::NativeFunction("secret".to_string()));
        self.env.define("reveal".to_string(), Value::NativeFunction("reveal".to_string()));
        self.env.define("to_bool".to_string(), Value::NativeFunction("to_bool".to_string()));
        self.env.define("is_truthy".to_string(), Value::NativeFunction("is_truthy".to_string()));
        self.env.define("bytes".to_string(), Value::NativeFunction("bytes".to_string()));
        self.env.define("dict".to_string(), Value::NativeFunction("dict".to_string()));
        self.env.define("array".to_string(), Value::NativeFunction("array".to_string()));
        self.env.define("error".to_string(), Value::NativeFunction("error".to_string()));

        // Type introspection functions
        self.env.define("type".to_string(), Value::NativeFunction("type".to_string()));
        self.env.define("type_of".to_string(), Value::NativeFunction("type".to_string()));
        self.env.define("is_int".to_string(), Value::NativeFunction("is_int".to_string()));
        self.env.define("is_float".to_string(), Value::NativeFunction("is_float".to_string()));
        self.env.define("is_string".to_string(), Value::NativeFunction("is_string".to_string()));
        self.env.define("is_secret".to_string(), Value::NativeFunction("is_secret".to_string()));
        self.env.define("is_array".to_string(), Value::NativeFunction("is_array".to_string()));
        self.env.define("is_dict".to_string(), Value::NativeFunction("is_dict".to_string()));
        self.env.define("is_bool".to_string(), Value::NativeFunction("is_bool".to_string()));
        self.env.define("is_null".to_string(), Value::NativeFunction("is_null".to_string()));
        self.env
            .define("is_function".to_string(), Value::NativeFunction("is_function".to_string()));

        // Assert & Debug functions
        self.env.define("assert".to_string(), Value::NativeFunction("assert".to_string()));
        self.env.define("debug".to_string(), Value::NativeFunction("debug".to_string()));

        // File I/O functions
        self.env.define("read_file".to_string(), Value::NativeFunction("read_file".to_string()));
        self.env.define(
            "read_file_lossy".to_string(),
            Value::NativeFunction("read_file_lossy".to_string()),
        );
        self.env.define("write_file".to_string(), Value::NativeFunction("write_file".to_string()));
        self.env.define(
            "write_file_atomic".to_string(),
            Value::NativeFunction("write_file_atomic".to_string()),
        );
        self.env
            .define("append_file".to_string(), Value::NativeFunction("append_file".to_string()));
        self.env
            .define("file_exists".to_string(), Value::NativeFunction("file_exists".to_string()));
        self.env.define("read_lines".to_string(), Value::NativeFunction("read_lines".to_string()));
        self.env
            .define("jsonl_query".to_string(), Value::NativeFunction("jsonl_query".to_string()));
        self.env.define("list_dir".to_string(), Value::NativeFunction("list_dir".to_string()));
        self.env.define("create_dir".to_string(), Value::NativeFunction("create_dir".to_string()));
        self.env.define("file_size".to_string(), Value::NativeFunction("file_size".to_string()));
        self.env
            .define("delete_file".to_string(), Value::NativeFunction("delete_file".to_string()));
        self.env
            .define("rename_file".to_string(), Value::NativeFunction("rename_file".to_string()));
        self.env.define("copy_file".to_string(), Value::NativeFunction("copy_file".to_string()));

        // Binary file I/O functions
        self.env.define(
            "read_binary_file".to_string(),
            Value::NativeFunction("read_binary_file".to_string()),
        );
        self.env.define(
            "write_binary_file".to_string(),
            Value::NativeFunction("write_binary_file".to_string()),
        );

        // IO module functions (advanced binary operations)
        self.env.define(
            "io_read_bytes".to_string(),
            Value::NativeFunction("io_read_bytes".to_string()),
        );
        self.env.define(
            "io_write_bytes".to_string(),
            Value::NativeFunction("io_write_bytes".to_string()),
        );
        self.env.define(
            "io_append_bytes".to_string(),
            Value::NativeFunction("io_append_bytes".to_string()),
        );
        self.env.define("io_read_at".to_string(), Value::NativeFunction("io_read_at".to_string()));
        self.env
            .define("io_write_at".to_string(), Value::NativeFunction("io_write_at".to_string()));
        self.env
            .define("io_seek_read".to_string(), Value::NativeFunction("io_seek_read".to_string()));
        self.env.define(
            "io_file_metadata".to_string(),
            Value::NativeFunction("io_file_metadata".to_string()),
        );
        self.env.define(
            "io_set_permissions".to_string(),
            Value::NativeFunction("io_set_permissions".to_string()),
        );
        self.env.define(
            "io_write_private_file".to_string(),
            Value::NativeFunction("io_write_private_file".to_string()),
        );
        self.env
            .define("io_truncate".to_string(), Value::NativeFunction("io_truncate".to_string()));
        self.env.define(
            "io_copy_range".to_string(),
            Value::NativeFunction("io_copy_range".to_string()),
        );

        // JSON functions
        self.env.define("parse_json".to_string(), Value::NativeFunction("parse_json".to_string()));
        self.env.define("to_json".to_string(), Value::NativeFunction("to_json".to_string()));
        self.env.define(
            "to_json_pretty".to_string(),
            Value::NativeFunction("to_json_pretty".to_string()),
        );
        self.env.define(
            "json_schema_validate".to_string(),
            Value::NativeFunction("json_schema_validate".to_string()),
        );

        // TOML functions
        self.env.define("parse_toml".to_string(), Value::NativeFunction("parse_toml".to_string()));
        self.env.define("to_toml".to_string(), Value::NativeFunction("to_toml".to_string()));

        // YAML functions
        self.env.define("parse_yaml".to_string(), Value::NativeFunction("parse_yaml".to_string()));
        self.env.define("to_yaml".to_string(), Value::NativeFunction("to_yaml".to_string()));

        // CSV functions
        self.env.define("parse_csv".to_string(), Value::NativeFunction("parse_csv".to_string()));
        self.env.define("to_csv".to_string(), Value::NativeFunction("to_csv".to_string()));

        // Base64 encoding/decoding functions
        self.env.define(
            "encode_base64".to_string(),
            Value::NativeFunction("encode_base64".to_string()),
        );
        self.env.define(
            "decode_base64".to_string(),
            Value::NativeFunction("decode_base64".to_string()),
        );
        self.env.define(
            "encode_uri_component".to_string(),
            Value::NativeFunction("encode_uri_component".to_string()),
        );

        // Random functions
        self.env.define("random".to_string(), Value::NativeFunction("random".to_string()));
        self.env.define("random_int".to_string(), Value::NativeFunction("random_int".to_string()));
        self.env.define(
            "random_choice".to_string(),
            Value::NativeFunction("random_choice".to_string()),
        );
        self.env.define("uuid_v4".to_string(), Value::NativeFunction("uuid_v4".to_string()));
        self.env.define("random_id".to_string(), Value::NativeFunction("random_id".to_string()));
        self.env.define(
            "set_random_seed".to_string(),
            Value::NativeFunction("set_random_seed".to_string()),
        );
        self.env.define(
            "clear_random_seed".to_string(),
            Value::NativeFunction("clear_random_seed".to_string()),
        );

        // Date/Time functions
        self.env.define("now".to_string(), Value::NativeFunction("now".to_string()));
        self.env.define("now_utc".to_string(), Value::NativeFunction("now_utc".to_string()));
        self.env.define("now_unix".to_string(), Value::NativeFunction("now_unix".to_string()));
        self.env
            .define("now_utc_seconds".to_string(), Value::NativeFunction("now_unix".to_string()));
        self.env.define(
            "current_timestamp".to_string(),
            Value::NativeFunction("current_timestamp".to_string()),
        );
        self.env.define("time".to_string(), Value::NativeFunction("current_timestamp".to_string()));
        self.env.define(
            "performance_now".to_string(),
            Value::NativeFunction("performance_now".to_string()),
        );
        self.env.define("time_us".to_string(), Value::NativeFunction("time_us".to_string()));
        self.env.define("time_ns".to_string(), Value::NativeFunction("time_ns".to_string()));
        self.env.define(
            "format_duration".to_string(),
            Value::NativeFunction("format_duration".to_string()),
        );
        self.env.define("elapsed".to_string(), Value::NativeFunction("elapsed".to_string()));
        self.env
            .define("format_date".to_string(), Value::NativeFunction("format_date".to_string()));
        self.env.define("parse_date".to_string(), Value::NativeFunction("parse_date".to_string()));

        // System operation functions
        self.env.define("env".to_string(), Value::NativeFunction("env".to_string()));
        self.env.define("env_or".to_string(), Value::NativeFunction("env_or".to_string()));
        self.env.define("env_int".to_string(), Value::NativeFunction("env_int".to_string()));
        self.env.define("env_float".to_string(), Value::NativeFunction("env_float".to_string()));
        self.env.define("env_bool".to_string(), Value::NativeFunction("env_bool".to_string()));
        self.env
            .define("env_required".to_string(), Value::NativeFunction("env_required".to_string()));
        self.env.define("kv_set".to_string(), Value::NativeFunction("kv_set".to_string()));
        self.env.define("kv_get".to_string(), Value::NativeFunction("kv_get".to_string()));
        self.env.define("env_set".to_string(), Value::NativeFunction("env_set".to_string()));
        self.env.define("env_list".to_string(), Value::NativeFunction("env_list".to_string()));
        self.env.define("args".to_string(), Value::NativeFunction("args".to_string()));
        self.env.define("arg_parser".to_string(), Value::NativeFunction("arg_parser".to_string()));
        self.env.define("exit".to_string(), Value::NativeFunction("exit".to_string()));
        self.env.define("sleep".to_string(), Value::NativeFunction("sleep".to_string()));
        self.env.define("execute".to_string(), Value::NativeFunction("execute".to_string()));
        self.env.define(
            "execute_status".to_string(),
            Value::NativeFunction("execute_status".to_string()),
        );

        // OS module functions
        self.env.define("os_getcwd".to_string(), Value::NativeFunction("os_getcwd".to_string()));
        self.env.define("os_chdir".to_string(), Value::NativeFunction("os_chdir".to_string()));
        self.env.define("os_rmdir".to_string(), Value::NativeFunction("os_rmdir".to_string()));
        self.env.define("os_environ".to_string(), Value::NativeFunction("os_environ".to_string()));

        // Path operation functions
        self.env.define("join_path".to_string(), Value::NativeFunction("join_path".to_string()));
        self.env.define("dirname".to_string(), Value::NativeFunction("dirname".to_string()));
        self.env.define("basename".to_string(), Value::NativeFunction("basename".to_string()));
        self.env
            .define("path_exists".to_string(), Value::NativeFunction("path_exists".to_string()));
        self.env.define("path_join".to_string(), Value::NativeFunction("path_join".to_string()));
        self.env.define(
            "path_absolute".to_string(),
            Value::NativeFunction("path_absolute".to_string()),
        );
        self.env
            .define("path_is_dir".to_string(), Value::NativeFunction("path_is_dir".to_string()));
        self.env
            .define("path_is_file".to_string(), Value::NativeFunction("path_is_file".to_string()));
        self.env.define(
            "path_is_symlink".to_string(),
            Value::NativeFunction("path_is_symlink".to_string()),
        );
        self.env.define(
            "path_extension".to_string(),
            Value::NativeFunction("path_extension".to_string()),
        );

        // Regular expression functions
        self.env
            .define("regex_match".to_string(), Value::NativeFunction("regex_match".to_string()));
        self.env.define(
            "regex_find_all".to_string(),
            Value::NativeFunction("regex_find_all".to_string()),
        );
        self.env.define(
            "regex_replace".to_string(),
            Value::NativeFunction("regex_replace".to_string()),
        );
        self.env
            .define("regex_split".to_string(), Value::NativeFunction("regex_split".to_string()));

        // HTTP client functions
        self.env.define("http_get".to_string(), Value::NativeFunction("http_get".to_string()));
        self.env
            .define("http_request".to_string(), Value::NativeFunction("http_request".to_string()));
        self.env.define("http_post".to_string(), Value::NativeFunction("http_post".to_string()));
        self.env.define("http_put".to_string(), Value::NativeFunction("http_put".to_string()));
        self.env
            .define("http_delete".to_string(), Value::NativeFunction("http_delete".to_string()));
        self.env.define(
            "http_get_binary".to_string(),
            Value::NativeFunction("http_get_binary".to_string()),
        );
        self.env.define(
            "http_download_file".to_string(),
            Value::NativeFunction("http_download_file".to_string()),
        );
        self.env.define(
            "http_upload_file".to_string(),
            Value::NativeFunction("http_upload_file".to_string()),
        );
        self.env.define(
            "ai_request_hash".to_string(),
            Value::NativeFunction("ai_request_hash".to_string()),
        );
        self.env.define("ai_chat".to_string(), Value::NativeFunction("ai_chat".to_string()));
        self.env.define(
            "ai_stream_chat".to_string(),
            Value::NativeFunction("ai_stream_chat".to_string()),
        );
        self.env
            .define("ai_embedding".to_string(), Value::NativeFunction("ai_embedding".to_string()));
        self.env
            .define("ai_tool_loop".to_string(), Value::NativeFunction("ai_tool_loop".to_string()));

        // Concurrent HTTP functions
        self.env.define(
            "parallel_http".to_string(),
            Value::NativeFunction("parallel_http".to_string()),
        );

        // JWT authentication functions
        self.env.define("jwt_encode".to_string(), Value::NativeFunction("jwt_encode".to_string()));
        self.env.define("jwt_decode".to_string(), Value::NativeFunction("jwt_decode".to_string()));
        self.env.define("jwt_verify".to_string(), Value::NativeFunction("jwt_verify".to_string()));

        // OAuth2 helper functions
        self.env.define(
            "oauth2_auth_url".to_string(),
            Value::NativeFunction("oauth2_auth_url".to_string()),
        );
        self.env.define(
            "oauth2_get_token".to_string(),
            Value::NativeFunction("oauth2_get_token".to_string()),
        );

        // HTTP streaming functions
        self.env.define(
            "http_get_stream".to_string(),
            Value::NativeFunction("http_get_stream".to_string()),
        );

        // HTTP server functions
        self.env
            .define("http_server".to_string(), Value::NativeFunction("http_server".to_string()));
        self.env
            .define("http_listen".to_string(), Value::NativeFunction("http_listen".to_string()));
        self.env.define(
            "http_response".to_string(),
            Value::NativeFunction("http_response".to_string()),
        );
        self.env.define(
            "json_response".to_string(),
            Value::NativeFunction("json_response".to_string()),
        );
        self.env.define(
            "html_response".to_string(),
            Value::NativeFunction("html_response".to_string()),
        );
        self.env.define(
            "redirect_response".to_string(),
            Value::NativeFunction("redirect_response".to_string()),
        );
        self.env.define("set_header".to_string(), Value::NativeFunction("set_header".to_string()));
        self.env
            .define("set_headers".to_string(), Value::NativeFunction("set_headers".to_string()));

        // Database functions
        self.env.define("db_connect".to_string(), Value::NativeFunction("db_connect".to_string()));
        self.env.define("db_execute".to_string(), Value::NativeFunction("db_execute".to_string()));
        self.env.define("db_query".to_string(), Value::NativeFunction("db_query".to_string()));
        self.env.define("db_close".to_string(), Value::NativeFunction("db_close".to_string()));
        self.env.define("db_pool".to_string(), Value::NativeFunction("db_pool".to_string()));
        self.env.define(
            "db_pool_acquire".to_string(),
            Value::NativeFunction("db_pool_acquire".to_string()),
        );
        self.env.define(
            "db_pool_release".to_string(),
            Value::NativeFunction("db_pool_release".to_string()),
        );
        self.env.define(
            "db_pool_stats".to_string(),
            Value::NativeFunction("db_pool_stats".to_string()),
        );
        self.env.define(
            "db_pool_close".to_string(),
            Value::NativeFunction("db_pool_close".to_string()),
        );
        self.env.define("db_begin".to_string(), Value::NativeFunction("db_begin".to_string()));
        self.env.define("db_commit".to_string(), Value::NativeFunction("db_commit".to_string()));
        self.env
            .define("db_rollback".to_string(), Value::NativeFunction("db_rollback".to_string()));
        self.env.define(
            "db_last_insert_id".to_string(),
            Value::NativeFunction("db_last_insert_id".to_string()),
        );

        // Collection constructors and methods
        // Set
        self.env.define("Set".to_string(), Value::NativeFunction("Set".to_string()));
        self.env.define("set_add".to_string(), Value::NativeFunction("set_add".to_string()));
        self.env.define("set_has".to_string(), Value::NativeFunction("set_has".to_string()));
        self.env.define("set_remove".to_string(), Value::NativeFunction("set_remove".to_string()));
        self.env.define("set_union".to_string(), Value::NativeFunction("set_union".to_string()));
        self.env.define(
            "set_intersect".to_string(),
            Value::NativeFunction("set_intersect".to_string()),
        );
        self.env.define(
            "set_difference".to_string(),
            Value::NativeFunction("set_difference".to_string()),
        );
        self.env
            .define("set_to_array".to_string(), Value::NativeFunction("set_to_array".to_string()));

        // Queue
        self.env.define("Queue".to_string(), Value::NativeFunction("Queue".to_string()));
        self.env.define(
            "queue_enqueue".to_string(),
            Value::NativeFunction("queue_enqueue".to_string()),
        );
        self.env.define(
            "queue_dequeue".to_string(),
            Value::NativeFunction("queue_dequeue".to_string()),
        );
        self.env.define("queue_peek".to_string(), Value::NativeFunction("queue_peek".to_string()));
        self.env.define(
            "queue_is_empty".to_string(),
            Value::NativeFunction("queue_is_empty".to_string()),
        );
        self.env.define("queue_size".to_string(), Value::NativeFunction("queue_size".to_string()));
        self.env.define(
            "queue_to_array".to_string(),
            Value::NativeFunction("queue_to_array".to_string()),
        );

        // Stack
        self.env.define("Stack".to_string(), Value::NativeFunction("Stack".to_string()));
        self.env.define("stack_push".to_string(), Value::NativeFunction("stack_push".to_string()));
        self.env.define("stack_pop".to_string(), Value::NativeFunction("stack_pop".to_string()));
        self.env.define("stack_peek".to_string(), Value::NativeFunction("stack_peek".to_string()));
        self.env.define(
            "stack_is_empty".to_string(),
            Value::NativeFunction("stack_is_empty".to_string()),
        );
        self.env.define("stack_size".to_string(), Value::NativeFunction("stack_size".to_string()));
        self.env.define(
            "stack_to_array".to_string(),
            Value::NativeFunction("stack_to_array".to_string()),
        );

        // Concurrency functions
        self.env.define("channel".to_string(), Value::NativeFunction("channel".to_string()));
        self.env.define("shared_set".to_string(), Value::NativeFunction("shared_set".to_string()));
        self.env.define("shared_get".to_string(), Value::NativeFunction("shared_get".to_string()));
        self.env.define("shared_has".to_string(), Value::NativeFunction("shared_has".to_string()));
        self.env.define(
            "shared_delete".to_string(),
            Value::NativeFunction("shared_delete".to_string()),
        );
        self.env.define(
            "shared_add_int".to_string(),
            Value::NativeFunction("shared_add_int".to_string()),
        );

        // Async operations
        self.env
            .define("async_sleep".to_string(), Value::NativeFunction("async_sleep".to_string()));
        self.env.define(
            "async_timeout".to_string(),
            Value::NativeFunction("async_timeout".to_string()),
        );
        self.env.define(
            "async_http_get".to_string(),
            Value::NativeFunction("async_http_get".to_string()),
        );
        self.env.define(
            "async_http_post".to_string(),
            Value::NativeFunction("async_http_post".to_string()),
        );
        self.env.define(
            "async_read_file".to_string(),
            Value::NativeFunction("async_read_file".to_string()),
        );
        self.env.define(
            "async_read_files".to_string(),
            Value::NativeFunction("async_read_files".to_string()),
        );
        self.env.define(
            "async_write_file".to_string(),
            Value::NativeFunction("async_write_file".to_string()),
        );
        self.env.define(
            "async_write_files".to_string(),
            Value::NativeFunction("async_write_files".to_string()),
        );
        self.env.define("spawn_task".to_string(), Value::NativeFunction("spawn_task".to_string()));
        self.env.define("await_task".to_string(), Value::NativeFunction("await_task".to_string()));
        self.env
            .define("cancel_task".to_string(), Value::NativeFunction("cancel_task".to_string()));
        self.env
            .define("Promise.all".to_string(), Value::NativeFunction("Promise.all".to_string()));
        self.env
            .define("promise_all".to_string(), Value::NativeFunction("promise_all".to_string())); // Alias
        self.env.define("await_all".to_string(), Value::NativeFunction("await_all".to_string())); // Alias
        self.env
            .define("promise_wait".to_string(), Value::NativeFunction("promise_wait".to_string()));
        self.env
            .define("parallel_map".to_string(), Value::NativeFunction("parallel_map".to_string()));
        self.env.define("par_map".to_string(), Value::NativeFunction("par_map".to_string()));
        self.env.define("par_each".to_string(), Value::NativeFunction("par_each".to_string()));
        self.env.define(
            "set_task_pool_size".to_string(),
            Value::NativeFunction("set_task_pool_size".to_string()),
        );
        self.env.define(
            "get_task_pool_size".to_string(),
            Value::NativeFunction("get_task_pool_size".to_string()),
        );

        // Testing assertion functions
        self.env
            .define("assert_equal".to_string(), Value::NativeFunction("assert_equal".to_string()));
        self.env
            .define("assert_true".to_string(), Value::NativeFunction("assert_true".to_string()));
        self.env
            .define("assert_false".to_string(), Value::NativeFunction("assert_false".to_string()));
        self.env.define(
            "assert_contains".to_string(),
            Value::NativeFunction("assert_contains".to_string()),
        );

        // Image processing functions
        self.env.define("load_image".to_string(), Value::NativeFunction("load_image".to_string()));
        self.env
            .define("gif_to_webp".to_string(), Value::NativeFunction("gif_to_webp".to_string()));

        // Compression & Archive functions
        self.env.define("zip_create".to_string(), Value::NativeFunction("zip_create".to_string()));
        self.env
            .define("zip_add_file".to_string(), Value::NativeFunction("zip_add_file".to_string()));
        self.env
            .define("zip_add_dir".to_string(), Value::NativeFunction("zip_add_dir".to_string()));
        self.env.define("zip_close".to_string(), Value::NativeFunction("zip_close".to_string()));
        self.env.define("unzip".to_string(), Value::NativeFunction("unzip".to_string()));

        // Hashing & Crypto functions
        self.env.define("sha256".to_string(), Value::NativeFunction("sha256".to_string()));
        self.env
            .define("hmac_sha256".to_string(), Value::NativeFunction("hmac_sha256".to_string()));
        self.env.define(
            "hmac_sha256_verify".to_string(),
            Value::NativeFunction("hmac_sha256_verify".to_string()),
        );
        self.env
            .define("sha256_file".to_string(), Value::NativeFunction("sha256_file".to_string()));
        self.env.define("md5".to_string(), Value::NativeFunction("md5".to_string()));
        self.env.define("md5_file".to_string(), Value::NativeFunction("md5_file".to_string()));
        self.env.define(
            "hash_password".to_string(),
            Value::NativeFunction("hash_password".to_string()),
        );
        self.env.define(
            "verify_password".to_string(),
            Value::NativeFunction("verify_password".to_string()),
        );

        // Crypto functions (AES/RSA encryption)
        self.env
            .define("aes_encrypt".to_string(), Value::NativeFunction("aes_encrypt".to_string()));
        self.env
            .define("aes_decrypt".to_string(), Value::NativeFunction("aes_decrypt".to_string()));
        self.env.define(
            "aes_encrypt_bytes".to_string(),
            Value::NativeFunction("aes_encrypt_bytes".to_string()),
        );
        self.env.define(
            "aes_decrypt_bytes".to_string(),
            Value::NativeFunction("aes_decrypt_bytes".to_string()),
        );
        self.env.define(
            "aes_encrypt_file_stream".to_string(),
            Value::NativeFunction("aes_encrypt_file_stream".to_string()),
        );
        self.env.define(
            "aes_decrypt_file_stream".to_string(),
            Value::NativeFunction("aes_decrypt_file_stream".to_string()),
        );
        self.env.define(
            "rsa_generate_keypair".to_string(),
            Value::NativeFunction("rsa_generate_keypair".to_string()),
        );
        self.env
            .define("rsa_encrypt".to_string(), Value::NativeFunction("rsa_encrypt".to_string()));
        self.env
            .define("rsa_decrypt".to_string(), Value::NativeFunction("rsa_decrypt".to_string()));
        self.env.define("rsa_sign".to_string(), Value::NativeFunction("rsa_sign".to_string()));
        self.env.define("rsa_verify".to_string(), Value::NativeFunction("rsa_verify".to_string()));

        // Process management functions
        self.env.define(
            "spawn_process".to_string(),
            Value::NativeFunction("spawn_process".to_string()),
        );
        self.env.define(
            "pipe_commands".to_string(),
            Value::NativeFunction("pipe_commands".to_string()),
        );

        // Network functions (TCP/UDP)
        self.env.define("tcp_listen".to_string(), Value::NativeFunction("tcp_listen".to_string()));
        self.env.define("tcp_accept".to_string(), Value::NativeFunction("tcp_accept".to_string()));
        self.env
            .define("tcp_connect".to_string(), Value::NativeFunction("tcp_connect".to_string()));
        self.env.define("tcp_send".to_string(), Value::NativeFunction("tcp_send".to_string()));
        self.env
            .define("tcp_receive".to_string(), Value::NativeFunction("tcp_receive".to_string()));
        self.env.define("tcp_close".to_string(), Value::NativeFunction("tcp_close".to_string()));
        self.env.define(
            "tcp_set_nonblocking".to_string(),
            Value::NativeFunction("tcp_set_nonblocking".to_string()),
        );
        self.env.define("udp_bind".to_string(), Value::NativeFunction("udp_bind".to_string()));
        self.env
            .define("udp_send_to".to_string(), Value::NativeFunction("udp_send_to".to_string()));
        self.env.define(
            "udp_receive_from".to_string(),
            Value::NativeFunction("udp_receive_from".to_string()),
        );
        self.env.define("udp_close".to_string(), Value::NativeFunction("udp_close".to_string()));
        self.env.define(
            "dns_lookup_mx".to_string(),
            Value::NativeFunction("dns_lookup_mx".to_string()),
        );
        self.env.define(
            "dns_lookup_txt".to_string(),
            Value::NativeFunction("dns_lookup_txt".to_string()),
        );
        self.env.define(
            "dns_lookup_ptr".to_string(),
            Value::NativeFunction("dns_lookup_ptr".to_string()),
        );
        self.env.define(
            "dns_lookup_tlsa".to_string(),
            Value::NativeFunction("dns_lookup_tlsa".to_string()),
        );
        self.env
            .define("tls_connect".to_string(), Value::NativeFunction("tls_connect".to_string()));
        self.env.define(
            "tls_upgrade_client".to_string(),
            Value::NativeFunction("tls_upgrade_client".to_string()),
        );
        self.env
            .define("tls_acceptor".to_string(), Value::NativeFunction("tls_acceptor".to_string()));
        self.env.define(
            "tls_upgrade_server".to_string(),
            Value::NativeFunction("tls_upgrade_server".to_string()),
        );
        self.env.define("tls_send".to_string(), Value::NativeFunction("tls_send".to_string()));
        self.env
            .define("tls_receive".to_string(), Value::NativeFunction("tls_receive".to_string()));
        self.env.define("tls_close".to_string(), Value::NativeFunction("tls_close".to_string()));
        self.env.define("tls_info".to_string(), Value::NativeFunction("tls_info".to_string()));
    }

    /// Sets the source file and content for error reporting
    pub fn set_source(&mut self, file: String, content: &str) {
        self.source_file = Some(file);
        self.source_lines = content.lines().map(|s| s.to_string()).collect();
    }

    /// Reports a runtime error with source location
    #[allow(dead_code)]
    fn report_error(&self, error: KujoError) {
        eprintln!("{}", error);
    }

    /// Gets the source line at a given line number (1-indexed)
    #[allow(dead_code)]
    fn get_source_line(&self, line: usize) -> Option<String> {
        if line > 0 && line <= self.source_lines.len() {
            Some(self.source_lines[line - 1].clone())
        } else {
            None
        }
    }

    /// Sets the output sink for print statements (used for testing)
    #[allow(dead_code)]
    pub fn set_output(&mut self, output: Arc<Mutex<Vec<u8>>>) {
        self.output = Some(output);
    }

    /// Helper function to call a user-defined function with given arguments
    /// Used by higher-order functions like map, filter, reduce
    pub(crate) fn call_user_function(&mut self, func: &Value, args: &[Value]) -> Value {
        match func {
            Value::GeneratorDef(params, body) => {
                let arity = Self::function_arity("<anonymous generator>", params);
                if let Some(error) = self.validate_callable_arity(&arity, args.len()) {
                    return error;
                }

                // Calling a generator function returns a Generator instance
                // Create a new environment for the generator
                let mut gen_env = self.env.clone();
                gen_env.push_scope();

                // Bind parameters to arguments
                for (i, param) in params.iter().enumerate() {
                    if let Some(arg) = args.get(i) {
                        gen_env.define(param.clone(), arg.clone());
                    }
                }

                // Return a Generator instance
                Value::Generator {
                    params: params.clone(),
                    body: body.clone(),
                    env: Arc::new(Mutex::new(gen_env)),
                    pc: 0,
                    is_exhausted: false,
                }
            }
            Value::Function(params, body, captured_env) => {
                let arity = Self::function_arity("<anonymous function>", params);
                if let Some(error) = self.validate_callable_arity(&arity, args.len()) {
                    return error;
                }

                // Push function name to call stack
                let func_name = format!("<function with {} params>", params.len());
                self.call_stack.push(func_name);

                // If this is a closure with captured environment, use it
                // Otherwise just create a new scope on top of current
                if let Some(closure_env_ref) = captured_env {
                    // Save current environment
                    let saved_env = self.env.clone();

                    // Use the captured environment (which is shared via Arc<Mutex<>>)
                    self.env = closure_env_ref
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .clone();
                    self.env.push_scope();

                    // Bind parameters to arguments
                    for (i, param) in params.iter().enumerate() {
                        if let Some(arg) = args.get(i) {
                            self.env.define(param.clone(), arg.clone());
                        }
                    }

                    // Execute function body
                    if let Err(error) = self
                        .with_function_context("<anonymous function>", |interp| {
                            interp.eval_stmts(&body.get())
                        })
                    {
                        self.env.pop_scope();
                        self.env = saved_env;
                        self.call_stack.pop();
                        return error;
                    }

                    // Get return value
                    let result = if let Some(Value::Return(val)) = self.return_value.clone() {
                        self.return_value = None; // Clear return value
                        *val
                    } else if let Some(Value::Error(msg)) = self.return_value.clone() {
                        // Propagate error - don't clear
                        Value::Error(msg)
                    } else if let Some(Value::ErrorObject { .. }) = self.return_value.clone() {
                        // Propagate error object - don't clear
                        self.return_value.clone().unwrap()
                    } else {
                        // No explicit return - function falls through to null
                        self.return_value = None;
                        Value::Null
                    };

                    // Pop the parameter scope
                    self.env.pop_scope();

                    // Update the captured environment with the modified state
                    *closure_env_ref.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) =
                        self.env.clone();

                    // Restore the saved environment
                    self.env = saved_env;

                    // Pop from call stack
                    self.call_stack.pop();

                    result
                } else {
                    // Non-closure: just create new scope on current environment
                    self.env.push_scope();

                    // Bind parameters to arguments
                    for (i, param) in params.iter().enumerate() {
                        if let Some(arg) = args.get(i) {
                            self.env.define(param.clone(), arg.clone());
                        }
                    }

                    // Execute function body
                    if let Err(error) = self
                        .with_function_context("<anonymous function>", |interp| {
                            interp.eval_stmts(&body.get())
                        })
                    {
                        self.env.pop_scope();
                        self.call_stack.pop();
                        return error;
                    }

                    // Get return value
                    let result = if let Some(Value::Return(val)) = self.return_value.clone() {
                        self.return_value = None; // Clear return value
                        *val
                    } else if let Some(Value::Error(msg)) = self.return_value.clone() {
                        // Propagate error - don't clear
                        Value::Error(msg)
                    } else if let Some(Value::ErrorObject { .. }) = self.return_value.clone() {
                        // Propagate error object - don't clear
                        self.return_value.clone().unwrap()
                    } else {
                        // No explicit return - function falls through to null
                        self.return_value = None;
                        Value::Null
                    };

                    // Restore parent environment
                    self.env.pop_scope();

                    // Pop from call stack
                    self.call_stack.pop();

                    result
                }
            }
            _ => Value::Int(0),
        }
    }

    /// Execute one async-function mapper invocation in an isolated interpreter.
    /// `parallel_map` uses this on bounded worker tasks so mapper state cannot
    /// race through a shared interpreter environment.
    pub(crate) fn execute_async_mapper_isolated(
        func: &Value,
        arg: Value,
        base_env: Environment,
        capability_policy: RuntimeCapabilityPolicy,
    ) -> Value {
        let Value::AsyncFunction(params, body, captured_env) = func else {
            return Value::Error("parallel_map async mapper is not an async function".to_string());
        };
        if params.len() != 1 {
            return Value::Error(format!(
                "parallel_map async mapper expects 1 parameter, got {}",
                params.len()
            ));
        }
        let mut interpreter = Interpreter::with_capability_policy(capability_policy);
        interpreter.env = base_env;
        if let Some(env_ref) = captured_env {
            let captured = env_ref.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone();
            for scope in captured.scopes {
                interpreter.env.push_scope();
                for (name, value) in scope {
                    interpreter.env.define(name, value);
                }
            }
        }
        interpreter.env.push_scope();
        interpreter.env.define(params[0].clone(), arg);
        if let Err(error) = interpreter.with_function_context("<parallel async mapper>", |interp| {
            interp.eval_stmts(&body.get())
        }) {
            return error;
        }
        match interpreter.return_value.take() {
            Some(Value::Return(value)) => *value,
            Some(value @ Value::Error(_)) | Some(value @ Value::ErrorObject { .. }) => value,
            _ => Value::Null,
        }
    }

    /// Attempts to call an operator method on a struct value
    /// Returns Some(result) if the struct has the operator method, None otherwise
    fn try_call_operator_method(
        &mut self,
        struct_val: &Value,
        method_name: &str,
        other: &Value,
    ) -> Option<Value> {
        if let Value::Struct { name, fields } = struct_val {
            // Look up the struct definition to find the operator method
            if let Some(Value::StructDef { name: _, field_names: _, methods }) = self.env.get(name)
            {
                if let Some(Value::Function(params, body, _captured_env)) = methods.get(method_name)
                {
                    let arity = Self::struct_method_arity(name, method_name, params);
                    if let Some(error) = self.validate_callable_arity(&arity, 1) {
                        return Some(error);
                    }

                    // Create new scope for operator method call
                    self.env.push_scope();

                    // Check if method has 'self' as first parameter
                    let has_self_param = params.first().map(|p| p == "self").unwrap_or(false);

                    if has_self_param {
                        // Bind self to the struct instance
                        self.env.define("self".to_string(), struct_val.clone());

                        // Bind the other operand as the second parameter (after self)
                        if let Some(param_name) = params.get(1) {
                            self.env.define(param_name.clone(), other.clone());
                        }
                    } else {
                        // Backward compatibility: bind fields directly into scope
                        for (field_name, field_value) in fields {
                            self.env.define(field_name.clone(), field_value.clone());
                        }

                        // Bind the other operand as the first parameter
                        if let Some(param_name) = params.first() {
                            self.env.define(param_name.clone(), other.clone());
                        }
                    }

                    // Execute method body
                    if let Err(error) = self
                        .with_function_context(&format!("{}.{}", name, method_name), |interp| {
                            interp.eval_stmts(&body.get())
                        })
                    {
                        self.env.pop_scope();
                        return Some(error);
                    }

                    let result = if let Some(Value::Return(val)) = self.return_value.clone() {
                        self.return_value = None;
                        *val
                    } else {
                        self.return_value = None;
                        Value::Null
                    };

                    // Restore parent environment
                    self.env.pop_scope();

                    return Some(result);
                }
            }
        }
        None
    }

    /// Attempts to call a unary operator method on a struct value
    /// Returns Some(result) if the struct has the operator method, None otherwise
    fn try_call_unary_operator_method(
        &mut self,
        struct_val: &Value,
        method_name: &str,
    ) -> Option<Value> {
        if let Value::Struct { name, fields } = struct_val {
            // Look up the struct definition to find the operator method
            if let Some(Value::StructDef { name: _, field_names: _, methods }) = self.env.get(name)
            {
                if let Some(Value::Function(params, body, _captured_env)) = methods.get(method_name)
                {
                    let arity = Self::struct_method_arity(name, method_name, params);
                    if let Some(error) = self.validate_callable_arity(&arity, 0) {
                        return Some(error);
                    }

                    // Create new scope for operator method call
                    self.env.push_scope();

                    // Check if method has 'self' as first (and only) parameter
                    let has_self_param = params.first().map(|p| p == "self").unwrap_or(false);

                    if has_self_param {
                        // Bind self to the struct instance
                        self.env.define("self".to_string(), struct_val.clone());
                    } else {
                        // Backward compatibility: bind fields directly into scope
                        for (field_name, field_value) in fields {
                            self.env.define(field_name.clone(), field_value.clone());
                        }
                    }

                    // Execute method body
                    if let Err(error) = self
                        .with_function_context(&format!("{}.{}", name, method_name), |interp| {
                            interp.eval_stmts(&body.get())
                        })
                    {
                        self.env.pop_scope();
                        return Some(error);
                    }

                    let result = if let Some(Value::Return(val)) = self.return_value.clone() {
                        self.return_value = None;
                        *val
                    } else {
                        self.return_value = None;
                        Value::Null
                    };

                    // Restore parent environment
                    self.env.pop_scope();

                    return Some(result);
                }
            }
        }
        None
    }

    /// Matches a route pattern against a URL path, extracting path parameters
    /// Returns Some(HashMap) with extracted params if matched, None if no match
    fn match_route_pattern(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        // Must have same number of segments
        if pattern_parts.len() != path_parts.len() {
            return None;
        }

        let mut params = HashMap::new();

        for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
            if let Some(param_name) = pattern_part.strip_prefix(':') {
                // This is a path parameter - extract it
                params.insert(param_name.to_string(), path_part.to_string());
            } else if *pattern_part != *path_part {
                // Static segment doesn't match
                return None;
            }
        }

        Some(params)
    }

    /// Starts an HTTP server with registered routes
    fn start_http_server(
        &mut self,
        host: &str,
        port: u16,
        routes: Vec<(String, String, Value)>,
    ) -> Value {
        use tiny_http::{Response, Server};
        if let Err(error) =
            self.require_capability(NativeCapability::NetworkServer, "http_server.listen")
        {
            return error;
        }

        println!("Starting HTTP server on {}:{}...", host, port);

        let server = match Server::http(format!("{}:{}", host, port)) {
            Ok(s) => s,
            Err(e) => return Value::Error(format!("Failed to start server: {}", e)),
        };

        println!("Server listening on http://{}:{}", host, port);
        println!("Press Ctrl+C to stop");

        // Main server loop
        for mut request in server.incoming_requests() {
            let method = request.method().to_string();
            let request_url = request.url().to_string();
            let (url_path, query_params, decoded_query_params, raw_query) =
                http_request_utils::split_http_path_and_query_with_decoded(&request_url);

            // Read body first (before any response handling)
            let body_content =
                match http_request_utils::read_bounded_http_request_body(request.as_reader()) {
                    Ok(body) => body,
                    Err(http_request_utils::HttpRequestBodyError::TooLarge) => {
                        let response =
                            Response::from_string("Payload Too Large").with_status_code(413);
                        let _ = request.respond(response);
                        continue;
                    }
                    Err(http_request_utils::HttpRequestBodyError::ReadFailed) => {
                        let response = Response::from_string("Bad Request").with_status_code(400);
                        let _ = request.respond(response);
                        continue;
                    }
                };

            // Find matching route (supports path parameters like /:code)
            // Exact matches take priority over parameterized routes
            let mut response_to_send: Option<Response<std::io::Cursor<Vec<u8>>>> = None;
            let mut matched_handler: Option<(&Value, HashMap<String, String>)> = None;

            // First pass: look for exact matches
            for (route_method, route_path, handler) in &routes {
                if method == *route_method && url_path == *route_path {
                    matched_handler = Some((handler, HashMap::new()));
                    break;
                }
            }

            // Second pass: if no exact match, try parameterized routes
            if matched_handler.is_none() {
                for (route_method, route_path, handler) in &routes {
                    if method != *route_method {
                        continue;
                    }
                    // Only try parameterized matching for routes with ':'
                    if route_path.contains(':') {
                        if let Some(path_params) = Self::match_route_pattern(route_path, &url_path)
                        {
                            matched_handler = Some((handler, path_params));
                            break;
                        }
                    }
                }
            }

            if let Some((handler, path_params)) = matched_handler {
                // Create params dict for request object
                let mut params_dict = DictMap::default();
                for (key, value) in &path_params {
                    params_dict
                        .insert(Arc::from(key.as_str()), Value::Str(Arc::new(value.clone())));
                }

                // Create request object as a Dict (not Struct) so has_key() and bracket access work
                let mut req_fields = DictMap::default();
                req_fields.insert("method".into(), Value::Str(Arc::new(method.clone())));
                req_fields.insert("path".into(), Value::Str(Arc::new(url_path.clone())));
                req_fields.insert("raw_path".into(), Value::Str(Arc::new(request_url.clone())));
                req_fields.insert("body".into(), Value::Str(Arc::new(body_content.clone())));
                req_fields.insert("params".into(), Value::Dict(Arc::new(params_dict)));

                // Extract query params from URL
                let mut query_dict = DictMap::default();
                for (key, value) in &query_params {
                    query_dict.insert(Arc::from(key.as_str()), Value::Str(Arc::new(value.clone())));
                }
                req_fields.insert("query".into(), Value::Dict(Arc::new(query_dict)));

                let mut decoded_query_dict = DictMap::default();
                for (key, value) in &decoded_query_params {
                    decoded_query_dict
                        .insert(Arc::from(key.as_str()), Value::Str(Arc::new(value.clone())));
                }
                req_fields
                    .insert("query_decoded".into(), Value::Dict(Arc::new(decoded_query_dict)));
                req_fields.insert("query_string".into(), Value::Str(Arc::new(raw_query.clone())));

                // Extract headers from request
                let mut headers_dict = DictMap::default();
                for header in request.headers() {
                    let header_name = header.field.as_str().to_string();
                    let header_value = header.value.as_str().to_string();
                    headers_dict.insert(header_name.into(), Value::Str(Arc::new(header_value)));
                }
                req_fields.insert("headers".into(), Value::Dict(Arc::new(headers_dict)));

                let req_obj = Value::Dict(Arc::new(req_fields));

                // Call handler function
                if let Value::Function(params, body, captured_env) = handler {
                    self.call_stack.push("<http route handler>".to_string());

                    let result = if let Some(closure_env_ref) = captured_env {
                        // Route handlers can be closures and must resolve symbols against
                        // their captured lexical environment instead of the request loop state.
                        let saved_env = self.env.clone();
                        self.env = closure_env_ref
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .clone();
                        self.env.push_scope();

                        if let Some(param) = params.first() {
                            self.env.define(param.clone(), req_obj.clone());
                        }

                        if let Err(error) = self
                            .with_function_context("<http route handler>", |interp| {
                                interp.eval_stmts(&body.get())
                            })
                        {
                            self.return_value = Some(error);
                        }

                        let result = if let Some(Value::Return(val)) = self.return_value.clone() {
                            self.return_value = None;
                            *val
                        } else if let Some(Value::Error(message)) = self.return_value.clone() {
                            self.return_value = None;
                            Value::Error(message)
                        } else if let Some(Value::ErrorObject { .. }) = self.return_value.clone() {
                            self.return_value.clone().unwrap()
                        } else {
                            self.return_value = None;
                            Value::HttpResponse {
                                status: 200,
                                body: "OK".to_string(),
                                headers: HashMap::new(),
                            }
                        };

                        self.env.pop_scope();
                        *closure_env_ref.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) =
                            self.env.clone();
                        self.env = saved_env;

                        result
                    } else {
                        self.env.push_scope();

                        // Bind request parameter
                        if let Some(param) = params.first() {
                            self.env.define(param.clone(), req_obj);
                        }

                        if let Err(error) = self
                            .with_function_context("<http route handler>", |interp| {
                                interp.eval_stmts(&body.get())
                            })
                        {
                            self.return_value = Some(error);
                        }

                        // Get result
                        let result = if let Some(Value::Return(val)) = self.return_value.clone() {
                            self.return_value = None;
                            *val
                        } else if let Some(Value::Error(message)) = self.return_value.clone() {
                            self.return_value = None;
                            Value::Error(message)
                        } else if let Some(Value::ErrorObject { .. }) = self.return_value.clone() {
                            self.return_value.clone().unwrap()
                        } else {
                            self.return_value = None;
                            Value::HttpResponse {
                                status: 200,
                                body: "OK".to_string(),
                                headers: HashMap::new(),
                            }
                        };

                        self.env.pop_scope();
                        result
                    };

                    self.call_stack.pop();

                    // Build response
                    if let Value::HttpResponse { status, body, headers } = result {
                        let mut response = Response::from_string(body);
                        response = response.with_status_code(status);

                        for (key, value) in headers {
                            if let Ok(header) =
                                tiny_http::Header::from_bytes(key.as_bytes(), value.as_bytes())
                            {
                                response = response.with_header(header);
                            }
                        }

                        response_to_send = Some(response);
                    } else if let Value::Error(_) | Value::ErrorObject { .. } = result {
                        response_to_send = Some(
                            Response::from_string("Internal Server Error").with_status_code(500),
                        );
                    } else {
                        // Handler didn't return HttpResponse
                        response_to_send = Some(
                            Response::from_string("Internal Server Error").with_status_code(500),
                        );
                    }
                }
            }

            // Send response
            if let Some(response) = response_to_send {
                let _ = request.respond(response);
            } else {
                // 404 Not Found
                let _ = request.respond(Response::from_string("Not Found").with_status_code(404));
            }
        }

        Value::Int(0)
    }
    /// Calls a native built-in function
    fn call_native_function(&mut self, name: &str, args: &[Expr]) -> Value {
        // Evaluate all arguments
        let arg_values: Vec<Value> = args.iter().map(|arg| self.eval_expr(arg)).collect();
        if let Some(error) = arg_values.iter().find(|value| Self::is_error_value(value)) {
            return error.clone();
        }

        // Delegate to the implementation that works with Values
        self.call_native_function_impl(name, &arg_values)
    }

    /// Implementation of native function calling with pre-evaluated Value arguments
    /// This is used both by call_native_function (after evaluating Expr args)
    /// and by the VM (which already has Value args)
    pub fn call_native_function_impl(&mut self, name: &str, arg_values: &[Value]) -> Value {
        // Delegate to the native_functions module dispatcher
        native_functions::call_native_function(self, name, arg_values)
    }

    /// Helper method to check if two values are equal (for Set operations)
    fn values_equal(a: &Value, b: &Value) -> bool {
        Value::equals(a, b)
    }

    fn undefined_variable(name: &str) -> Value {
        Value::Error(format!("Undefined variable: {}", name))
    }

    fn is_error_value(value: &Value) -> bool {
        matches!(value, Value::Error(_) | Value::ErrorObject { .. })
    }

    fn set_return_if_error(&mut self, value: &Value) -> bool {
        if Self::is_error_value(value) {
            self.return_value = Some(value.clone());
            true
        } else {
            false
        }
    }

    fn value_type_name(value: &Value) -> &'static str {
        match value {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Secret(_) => "secret",
            Value::Array(_) => "array",
            Value::Dict(_) => "dict",
            Value::Struct { .. } => "struct",
            Value::Function(..) => "function",
            Value::NativeFunction(_) => "native_function",
            Value::Null => "null",
            Value::Error(_) | Value::ErrorObject { .. } => "error",
            _ => "value",
        }
    }

    fn index_key_description(index: &Value) -> String {
        match index {
            Value::Str(key) => format!("{:?}", key.as_ref()),
            Value::Int(key) => key.to_string(),
            _ => Self::stringify_value(index),
        }
    }

    fn invalid_binary_operation(op: &str, left: &Value, right: &Value) -> Value {
        Value::Error(format!(
            "Invalid binary operation: {} {} {}",
            Self::value_type_name(left),
            op,
            Self::value_type_name(right)
        ))
    }

    fn invalid_unary_operation(op: &str, value: &Value) -> Value {
        Value::Error(format!("Invalid unary operation: {} {}", op, Self::value_type_name(value)))
    }

    fn index_value(object: &Value, index: &Value) -> Value {
        match (object, index) {
            (Value::Array(arr), Value::Int(i)) => {
                let idx = if *i < 0 { (arr.len() as i64) + *i } else { *i };
                if idx < 0 {
                    Value::Error(format!("Index out of bounds: {}", i))
                } else {
                    arr.get(idx as usize)
                        .cloned()
                        .unwrap_or_else(|| Value::Error(format!("Index out of bounds: {}", i)))
                }
            }
            (Value::Array(arr), Value::Float(i)) => {
                if !i.is_finite() {
                    return Value::Error("Invalid index operation".to_string());
                }
                let idx = *i as i64;
                let resolved = if idx < 0 { (arr.len() as i64) + idx } else { idx };
                if resolved < 0 {
                    Value::Error(format!("Index out of bounds: {}", i))
                } else {
                    arr.get(resolved as usize)
                        .cloned()
                        .unwrap_or_else(|| Value::Error(format!("Index out of bounds: {}", i)))
                }
            }
            (Value::Str(s), Value::Int(i)) => {
                let idx = if *i < 0 { (s.chars().count() as i64) + *i } else { *i };
                if idx < 0 {
                    Value::Error(format!("Index out of bounds: {}", i))
                } else {
                    s.chars()
                        .nth(idx as usize)
                        .map(|c| Value::Str(Arc::new(c.to_string())))
                        .unwrap_or_else(|| Value::Error(format!("Index out of bounds: {}", i)))
                }
            }
            (Value::Str(s), Value::Float(i)) => {
                if !i.is_finite() {
                    return Value::Error("Invalid index operation".to_string());
                }
                let idx = *i as i64;
                let resolved = if idx < 0 { (s.chars().count() as i64) + idx } else { idx };
                if resolved < 0 {
                    Value::Error(format!("Index out of bounds: {}", i))
                } else {
                    s.chars()
                        .nth(resolved as usize)
                        .map(|c| Value::Str(Arc::new(c.to_string())))
                        .unwrap_or_else(|| Value::Error(format!("Index out of bounds: {}", i)))
                }
            }
            (Value::Bytes(bytes), Value::Int(i)) => {
                let idx = if *i < 0 { (bytes.len() as i64) + *i } else { *i };
                if idx < 0 {
                    Value::Error(format!("Index out of bounds: {}", i))
                } else {
                    bytes
                        .get(idx as usize)
                        .copied()
                        .map(|byte| Value::Int(byte as i64))
                        .unwrap_or_else(|| Value::Error(format!("Index out of bounds: {}", i)))
                }
            }
            (Value::Bytes(bytes), Value::Float(i)) => {
                if !i.is_finite() {
                    return Value::Error("Invalid index operation".to_string());
                }
                let idx = *i as i64;
                let resolved = if idx < 0 { (bytes.len() as i64) + idx } else { idx };
                if resolved < 0 {
                    Value::Error(format!("Index out of bounds: {}", i))
                } else {
                    bytes
                        .get(resolved as usize)
                        .copied()
                        .map(|byte| Value::Int(byte as i64))
                        .unwrap_or_else(|| Value::Error(format!("Index out of bounds: {}", i)))
                }
            }
            (Value::Dict(map), Value::Str(key)) => map
                .get(key.as_str())
                .cloned()
                .unwrap_or_else(|| Value::Error(format!("Missing map key: {:?}", key.as_ref()))),
            (Value::Dict(map), Value::Int(key)) => map
                .get(key.to_string().as_str())
                .cloned()
                .unwrap_or_else(|| Value::Error(format!("Missing map key: {}", key))),
            _ => Value::Error("Invalid index operation".to_string()),
        }
    }

    fn assign_index(&mut self, object: &Expr, index: &Expr, value: &Value) -> Value {
        let index_value = self.eval_expr(index);
        if Self::is_error_value(&index_value) {
            return index_value;
        }

        let container_name = match object {
            Expr::Identifier(name) => name.as_str(),
            _ => return Value::Error("Complex index assignment not yet supported".to_string()),
        };

        let mut assignment_error: Option<String> = None;
        let index_clone = index_value.clone();
        let value_clone = value.clone();

        let mutate_result = self.env.mutate_checked(container_name, |container| match container {
            Value::Array(arr) => {
                let idx = match &index_clone {
                    Value::Int(i) => *i,
                    Value::Float(f) if f.is_finite() => *f as i64,
                    _ => {
                        assignment_error = Some(
                            "Invalid index assignment: unsupported array index type".to_string(),
                        );
                        return;
                    }
                };

                let arr_mut = Arc::make_mut(arr);
                let resolved = if idx < 0 { (arr_mut.len() as i64) + idx } else { idx };
                if resolved < 0 || resolved as usize >= arr_mut.len() {
                    assignment_error = Some(format!("Index out of bounds: {}", idx));
                    return;
                }

                arr_mut[resolved as usize] = value_clone.clone();
            }
            Value::Dict(dict) => {
                let key = Self::stringify_value(&index_clone);
                Arc::make_mut(dict).insert(key.into(), value_clone.clone());
            }
            _ => {
                assignment_error = Some("Invalid index assignment".to_string());
            }
        });

        if let Err(error) = mutate_result {
            return Value::Error(error);
        }

        if let Some(error) = assignment_error {
            return Value::Error(error);
        }

        Value::Null
    }

    fn assign_member(&mut self, object: &Expr, field: &str, value: &Value) -> Value {
        let field_name = field.to_string();
        let value_clone = value.clone();

        match object {
            Expr::Identifier(name) => {
                let mut assignment_error: Option<String> = None;
                let mutate_result = self.env.mutate_checked(name.as_str(), |obj_value| {
                    if let Value::Struct { name: _, fields } = obj_value {
                        fields.insert(field_name.clone(), value_clone.clone());
                    } else {
                        assignment_error = Some("Cannot set field on non-struct value".to_string());
                    }
                });

                if let Err(error) = mutate_result {
                    return Value::Error(error);
                }

                if let Some(error) = assignment_error {
                    return Value::Error(error);
                }

                Value::Null
            }
            Expr::IndexAccess { object: container_expr, index } => {
                let index_value = self.eval_expr(index);
                if Self::is_error_value(&index_value) {
                    return index_value;
                }

                let container_name = match container_expr.as_ref() {
                    Expr::Identifier(name) => name.as_str(),
                    _ => {
                        return Value::Error(
                            "Complex field assignment not yet supported".to_string(),
                        )
                    }
                };

                let mut assignment_error: Option<String> = None;
                let index_clone = index_value.clone();
                let mutate_result =
                    self.env.mutate_checked(container_name, |container| match container {
                        Value::Array(arr) => {
                            let idx = match &index_clone {
                                Value::Int(i) => *i,
                                Value::Float(f) if f.is_finite() => *f as i64,
                                _ => {
                                    assignment_error = Some(
                                        "Invalid index assignment: unsupported array index type"
                                            .to_string(),
                                    );
                                    return;
                                }
                            };

                            let arr_mut = Arc::make_mut(arr);
                            let resolved = if idx < 0 { (arr_mut.len() as i64) + idx } else { idx };
                            if resolved < 0 || resolved as usize >= arr_mut.len() {
                                assignment_error = Some(format!("Index out of bounds: {}", idx));
                                return;
                            }

                            if let Value::Struct { name: _, fields } =
                                &mut arr_mut[resolved as usize]
                            {
                                fields.insert(field_name.clone(), value_clone.clone());
                            } else {
                                assignment_error =
                                    Some("Array element is not a struct".to_string());
                            }
                        }
                        Value::Dict(dict) => {
                            let key = Self::stringify_value(&index_clone);
                            if let Some(Value::Struct { name: _, fields }) =
                                Arc::make_mut(dict).get_mut(key.as_str())
                            {
                                fields.insert(field_name.clone(), value_clone.clone());
                            } else {
                                assignment_error = Some(format!(
                                    "Missing map key: {}",
                                    Self::index_key_description(&index_clone)
                                ));
                            }
                        }
                        _ => {
                            assignment_error = Some("Invalid index assignment".to_string());
                        }
                    });

                if let Err(error) = mutate_result {
                    return Value::Error(error);
                }

                if let Some(error) = assignment_error {
                    return Value::Error(error);
                }

                Value::Null
            }
            _ => Value::Error("Complex field assignment not yet supported".to_string()),
        }
    }

    fn unary_op_value(&self, op: &str, value: &Value) -> Value {
        match (op, value) {
            ("-", Value::Int(n)) => match n.checked_neg() {
                Some(result) => Value::Int(result),
                None => Value::Error(format!("Integer overflow: -({})", n)),
            },
            ("-", Value::Float(n)) => Value::Float(-n),
            ("!", Value::Bool(b)) => Value::Bool(!b),
            _ => Self::invalid_unary_operation(op, value),
        }
    }

    fn binary_op_value(&self, left: &Value, op: &str, right: &Value) -> Value {
        if op == "&&" {
            return Value::Bool(left.is_truthy() && right.is_truthy());
        }
        if op == "||" {
            return Value::Bool(left.is_truthy() || right.is_truthy());
        }
        if op == "==" {
            return Value::Bool(Value::equals(left, right));
        }
        if op == "!=" {
            return Value::Bool(!Value::equals(left, right));
        }
        if matches!(op, "<" | ">" | "<=" | ">=") {
            return match Value::compare_order(left, op, right) {
                Ok(result) => Value::Bool(result),
                Err(error) => Value::Error(error),
            };
        }

        match (left, right) {
            (Value::Int(a), Value::Int(b)) => match op {
                "+" | "-" | "*" | "/" | "%" => match Value::checked_int_arithmetic(*a, op, *b) {
                    Ok(result) => Value::Int(result),
                    Err(error) => Value::Error(error),
                },
                _ => Self::invalid_binary_operation(op, left, right),
            },
            (Value::Float(a), Value::Float(b)) => match op {
                "+" | "-" | "*" | "/" | "%" => match Value::checked_float_arithmetic(*a, op, *b) {
                    Ok(result) => Value::Float(result),
                    Err(error) => Value::Error(error),
                },
                _ => Self::invalid_binary_operation(op, left, right),
            },
            (Value::Int(a), Value::Float(b)) => {
                let a_float = *a as f64;
                match op {
                    "+" | "-" | "*" | "/" | "%" => {
                        match Value::checked_float_arithmetic(a_float, op, *b) {
                            Ok(result) => Value::Float(result),
                            Err(error) => Value::Error(error),
                        }
                    }
                    _ => Self::invalid_binary_operation(op, left, right),
                }
            }
            (Value::Float(a), Value::Int(b)) => {
                let b_float = *b as f64;
                match op {
                    "+" | "-" | "*" | "/" | "%" => {
                        match Value::checked_float_arithmetic(*a, op, b_float) {
                            Ok(result) => Value::Float(result),
                            Err(error) => Value::Error(error),
                        }
                    }
                    _ => Self::invalid_binary_operation(op, left, right),
                }
            }
            (Value::Str(a), Value::Str(b)) => match op {
                "+" => {
                    let mut result = a.clone();
                    let result_str = Arc::make_mut(&mut result);
                    result_str.push_str(b.as_ref());
                    Value::Str(result)
                }
                _ => Self::invalid_binary_operation(op, left, right),
            },
            _ => Self::invalid_binary_operation(op, left, right),
        }
    }

    fn validate_callable_arity(
        &self,
        arity: &CallableArity,
        received_args: usize,
    ) -> Option<Value> {
        arity.validate(received_args).err().map(Value::Error)
    }

    fn function_arity(name: impl Into<String>, params: &[String]) -> CallableArity {
        CallableArity::exact(name, params.to_vec())
    }

    fn struct_method_arity(
        struct_name: &str,
        method_name: &str,
        params: &[String],
    ) -> CallableArity {
        let has_self = params.first().map(|p| p == "self").unwrap_or(false);
        let external_params: Vec<String> =
            if has_self { params.iter().skip(1).cloned().collect() } else { params.to_vec() };
        CallableArity::exact(format!("{}.{}", struct_name, method_name), external_params)
    }

    pub(crate) fn native_callable_arity(name: &str) -> Option<CallableArity> {
        let metadata = match name {
            "__vm_for_iterable" => {
                CallableArity::exact("__vm_for_iterable", vec!["value".to_string()])
            }
            "dict" => CallableArity::exact("dict", vec![]),
            "error" => CallableArity::exact("error", vec!["message".to_string()]),
            "secret" => CallableArity::exact("secret", vec!["value".to_string()]),
            "reveal" => CallableArity::exact("reveal", vec!["secret".to_string()]),
            "is_secret" => CallableArity::exact("is_secret", vec!["value".to_string()]),
            "collect" => CallableArity::exact("collect", vec!["iterable".to_string()]),
            "len" => CallableArity::exact("len", vec!["value".to_string()]),
            "bit_not" => CallableArity::exact("bit_not", vec!["value".to_string()]),
            "bit_and" | "bit_or" | "bit_xor" | "bit_shl" | "bit_shr" => {
                CallableArity::exact(name, vec!["left".to_string(), "right".to_string()])
            }
            "to_upper" | "upper" => CallableArity::exact(name, vec!["value".to_string()]),
            "to_lower" | "lower" => CallableArity::exact(name, vec!["value".to_string()]),
            "capitalize" => CallableArity::exact("capitalize", vec!["value".to_string()]),
            "trim" => CallableArity::exact("trim", vec!["value".to_string()]),
            "trim_start" => CallableArity::exact("trim_start", vec!["value".to_string()]),
            "trim_end" => CallableArity::exact("trim_end", vec!["value".to_string()]),
            "pad_left" | "pad_start" => CallableArity::exact(
                "pad_left",
                vec!["value".to_string(), "width".to_string(), "pad_char".to_string()],
            ),
            "pad_right" | "pad_end" => CallableArity::exact(
                "pad_right",
                vec!["value".to_string(), "width".to_string(), "pad_char".to_string()],
            ),
            "split" => {
                CallableArity::exact("split", vec!["value".to_string(), "delimiter".to_string()])
            }
            "join" => {
                CallableArity::exact("join", vec!["values".to_string(), "separator".to_string()])
            }
            "contains" => {
                CallableArity::exact("contains", vec!["value".to_string(), "needle".to_string()])
            }
            "starts_with" => {
                CallableArity::exact("starts_with", vec!["value".to_string(), "prefix".to_string()])
            }
            "ends_with" => {
                CallableArity::exact("ends_with", vec!["value".to_string(), "suffix".to_string()])
            }
            "index_of" => {
                CallableArity::exact("index_of", vec!["value".to_string(), "needle".to_string()])
            }
            "replace" => CallableArity::exact(
                "replace",
                vec!["value".to_string(), "from".to_string(), "to".to_string()],
            ),
            "substring" => CallableArity::exact(
                "substring",
                vec!["value".to_string(), "start".to_string(), "end".to_string()],
            ),
            "repeat" => {
                CallableArity::exact("repeat", vec!["value".to_string(), "count".to_string()])
            }
            "input" => CallableArity::range("input", 0, 1, vec!["prompt".to_string()]),
            "exit" => CallableArity::range("exit", 0, 1, vec!["code".to_string()]),
            "type" | "type_of" => CallableArity::exact("type", vec!["value".to_string()]),
            "is_truthy" => CallableArity::exact("is_truthy", vec!["value".to_string()]),
            "read_file_lossy" => CallableArity::exact("read_file_lossy", vec!["path".to_string()]),
            "jsonl_query" => {
                CallableArity::exact("jsonl_query", vec!["path".to_string(), "options".to_string()])
            }
            "write_file_atomic" => CallableArity::range(
                "write_file_atomic",
                2,
                3,
                vec!["path".to_string(), "content_or_bytes".to_string(), "overwrite".to_string()],
            ),
            "Promise.all" => CallableArity::range(
                "Promise.all",
                1,
                2,
                vec!["promises".to_string(), "concurrency".to_string()],
            ),
            "promise_all" | "await_all" => CallableArity::range(
                "Promise.all",
                1,
                2,
                vec!["promises".to_string(), "concurrency".to_string()],
            ),
            "promise_wait" => CallableArity::exact("promise_wait", vec!["promise".to_string()]),
            "parallel_map" | "par_map" => CallableArity::range(
                name,
                2,
                3,
                vec!["items".to_string(), "mapper".to_string(), "concurrency".to_string()],
            ),
            "par_each" => CallableArity::range(
                "par_each",
                2,
                3,
                vec!["items".to_string(), "mapper".to_string(), "concurrency".to_string()],
            ),
            "ai_request_hash" => CallableArity::exact(
                "ai_request_hash",
                vec!["prompt_or_messages".to_string(), "options".to_string()],
            ),
            "ai_chat" => CallableArity::exact(
                "ai_chat",
                vec!["prompt_or_messages".to_string(), "options".to_string()],
            ),
            "ai_stream_chat" => CallableArity::range(
                "ai_stream_chat",
                2,
                3,
                vec![
                    "prompt_or_messages".to_string(),
                    "options".to_string(),
                    "on_chunk".to_string(),
                ],
            ),
            "ai_embedding" => CallableArity::exact(
                "ai_embedding",
                vec!["input".to_string(), "options".to_string()],
            ),
            "ai_tool_loop" => CallableArity::exact(
                "ai_tool_loop",
                vec!["prompt_or_messages".to_string(), "options".to_string()],
            ),
            "vec_dot" | "vec_cosine" => {
                CallableArity::exact(name, vec!["a".to_string(), "b".to_string()])
            }
            "vec_norm" | "vec_normalize" => CallableArity::exact(name, vec!["a".to_string()]),
            "vec_top_k" => CallableArity::exact(
                "vec_top_k",
                vec!["query".to_string(), "matrix".to_string(), "k".to_string()],
            ),
            "ai_count_tokens" => CallableArity::range(
                "ai_count_tokens",
                1,
                2,
                vec!["text_or_messages".to_string(), "options".to_string()],
            ),
            "ai_fit_context" => CallableArity::range(
                "ai_fit_context",
                2,
                3,
                vec!["messages".to_string(), "max_tokens".to_string(), "options".to_string()],
            ),
            "ai_text" => CallableArity::exact("ai_text", vec!["content".to_string()]),
            "ai_image_url" => CallableArity::range(
                "ai_image_url",
                1,
                2,
                vec!["url".to_string(), "detail".to_string()],
            ),
            "ai_message" => CallableArity::exact(
                "ai_message",
                vec!["role".to_string(), "content_or_blocks".to_string()],
            ),
            "json_schema_validate" => CallableArity::exact(
                "json_schema_validate",
                vec!["value".to_string(), "schema".to_string()],
            ),
            "print" | "eprint" | "debug" | "array" => CallableArity::variadic(name, 0, vec![]),
            "hmac_sha256" => CallableArity::exact(
                "hmac_sha256",
                vec!["secret".to_string(), "message".to_string()],
            ),
            "hmac_sha256_verify" => CallableArity::exact(
                "hmac_sha256_verify",
                vec!["secret".to_string(), "message".to_string(), "expected_hex".to_string()],
            ),
            "sha256_file" => CallableArity::exact("sha256_file", vec!["path".to_string()]),
            "path_is_symlink" => CallableArity::exact("path_is_symlink", vec!["path".to_string()]),
            "io_set_permissions" => CallableArity::exact(
                "io_set_permissions",
                vec!["path".to_string(), "mode".to_string()],
            ),
            "io_write_private_file" => CallableArity::exact(
                "io_write_private_file",
                vec!["path".to_string(), "content".to_string(), "mode".to_string()],
            ),
            "http_download_file" => CallableArity::exact(
                "http_download_file",
                vec!["url".to_string(), "output_path".to_string(), "options".to_string()],
            ),
            "http_upload_file" => CallableArity::exact(
                "http_upload_file",
                vec!["url".to_string(), "input_path".to_string(), "options".to_string()],
            ),
            "aes_encrypt_file_stream" => CallableArity::exact(
                "aes_encrypt_file_stream",
                vec![
                    "input_path".to_string(),
                    "output_path".to_string(),
                    "key".to_string(),
                    "chunk_size".to_string(),
                ],
            ),
            "aes_decrypt_file_stream" => CallableArity::exact(
                "aes_decrypt_file_stream",
                vec!["input_path".to_string(), "output_path".to_string(), "key".to_string()],
            ),
            _ => return None,
        };

        Some(metadata)
    }

    /// Binds a pattern to a value, defining variables as needed
    fn bind_pattern(
        &mut self,
        pattern: &crate::ast::Pattern,
        value: Value,
        binding: BindingKind,
    ) -> Result<(), String> {
        use crate::ast::Pattern;

        match pattern {
            Pattern::Identifier(name) => {
                self.env.define_with_kind_checked(name.clone(), value, binding)?;
            }
            Pattern::Ignore => {
                // Do nothing - value is discarded
            }
            Pattern::Array { elements, rest } => {
                // Extract array elements
                if let Value::Array(arr) = value {
                    let mut i = 0;
                    let arr_len = arr.len();

                    // Bind each pattern element
                    for pattern_elem in elements {
                        if i < arr_len {
                            self.bind_pattern(pattern_elem, arr[i].clone(), binding)?;
                            i += 1;
                        } else {
                            // Not enough elements - bind to null
                            self.bind_pattern(pattern_elem, Value::Null, binding)?;
                        }
                    }

                    // Bind rest elements if present
                    if let Some(rest_name) = rest {
                        let rest_values: Vec<Value> =
                            if i < arr_len { arr[i..].to_vec() } else { vec![] };
                        self.env.define_with_kind_checked(
                            rest_name.clone(),
                            Value::Array(Arc::new(rest_values)),
                            binding,
                        )?;
                    }
                } else {
                    // Not an array - bind all patterns to null
                    for pattern_elem in elements {
                        self.bind_pattern(pattern_elem, Value::Null, binding)?;
                    }
                    if let Some(rest_name) = rest {
                        self.env.define_with_kind_checked(
                            rest_name.clone(),
                            Value::Array(Arc::new(vec![])),
                            binding,
                        )?;
                    }
                }
            }
            Pattern::Dict { keys, rest } => {
                // Extract dict values
                if let Value::Dict(dict) = value {
                    // Bind each key
                    for key in keys {
                        let val = dict.get(key.as_str()).cloned().unwrap_or(Value::Null);
                        self.env.define_with_kind_checked(key.clone(), val, binding)?;
                    }

                    // Bind rest elements if present
                    if let Some(rest_name) = rest {
                        let mut rest_dict = DictMap::default();
                        for (k, v) in dict.iter() {
                            if !keys.iter().any(|key| key.as_str() == k.as_ref()) {
                                rest_dict.insert(k.clone(), v.clone());
                            }
                        }
                        self.env.define_with_kind_checked(
                            rest_name.clone(),
                            Value::Dict(Arc::new(rest_dict)),
                            binding,
                        )?;
                    }
                } else if let Value::FixedDict { keys: dict_keys, values } = value {
                    for key in keys {
                        let idx = dict_keys.iter().position(|k| k.as_ref() == key.as_str());
                        let val = idx.and_then(|i| values.get(i).cloned()).unwrap_or(Value::Null);
                        self.env.define_with_kind_checked(key.clone(), val, binding)?;
                    }

                    if let Some(rest_name) = rest {
                        let mut rest_dict = DictMap::default();
                        for (k, v) in dict_keys.iter().cloned().zip(values.iter().cloned()) {
                            if !keys.iter().any(|key| key.as_str() == k.as_ref()) {
                                rest_dict.insert(k, v);
                            }
                        }
                        self.env.define_with_kind_checked(
                            rest_name.clone(),
                            Value::Dict(Arc::new(rest_dict)),
                            binding,
                        )?;
                    }
                } else if let Value::DenseIntDict(values) = value {
                    for key in keys {
                        let val = match key.parse::<i64>() {
                            Ok(int_key) => {
                                if int_key < 0 {
                                    Value::Null
                                } else {
                                    values.get(int_key as usize).cloned().unwrap_or(Value::Null)
                                }
                            }
                            Err(_) => Value::Null,
                        };
                        self.env.define_with_kind_checked(key.clone(), val, binding)?;
                    }

                    if let Some(rest_name) = rest {
                        let mut rest_dict = DictMap::default();
                        for (index, value) in values.iter().enumerate() {
                            let key = index.to_string();
                            if !keys.iter().any(|existing| existing.as_str() == key.as_str()) {
                                rest_dict.insert(Arc::from(key.as_str()), value.clone());
                            }
                        }
                        self.env.define_with_kind_checked(
                            rest_name.clone(),
                            Value::Dict(Arc::new(rest_dict)),
                            binding,
                        )?;
                    }
                } else if let Value::DenseIntDictInt(values) = value {
                    for key in keys {
                        let val = match key.parse::<i64>() {
                            Ok(int_key) => {
                                if int_key < 0 {
                                    Value::Null
                                } else {
                                    match values.get(int_key as usize) {
                                        Some(value) => {
                                            (*value).map(Value::Int).unwrap_or(Value::Null)
                                        }
                                        None => Value::Null,
                                    }
                                }
                            }
                            Err(_) => Value::Null,
                        };
                        self.env.define_with_kind_checked(key.clone(), val, binding)?;
                    }

                    if let Some(rest_name) = rest {
                        let mut rest_dict = DictMap::default();
                        for (index, value) in values.iter().enumerate() {
                            let key = index.to_string();
                            if !keys.iter().any(|existing| existing.as_str() == key.as_str()) {
                                rest_dict.insert(
                                    Arc::from(key.as_str()),
                                    (*value).map(Value::Int).unwrap_or(Value::Null),
                                );
                            }
                        }
                        self.env.define_with_kind_checked(
                            rest_name.clone(),
                            Value::Dict(Arc::new(rest_dict)),
                            binding,
                        )?;
                    }
                } else if let Value::DenseIntDictIntFull(values) = value {
                    for key in keys {
                        let val = match key.parse::<i64>() {
                            Ok(int_key) => {
                                if int_key < 0 {
                                    Value::Null
                                } else {
                                    values
                                        .get(int_key as usize)
                                        .map(|value| Value::Int(*value))
                                        .unwrap_or(Value::Null)
                                }
                            }
                            Err(_) => Value::Null,
                        };
                        self.env.define_with_kind_checked(key.clone(), val, binding)?;
                    }

                    if let Some(rest_name) = rest {
                        let mut rest_dict = DictMap::default();
                        for (index, value) in values.iter().enumerate() {
                            let key = index.to_string();
                            if !keys.iter().any(|existing| existing.as_str() == key.as_str()) {
                                rest_dict.insert(Arc::from(key.as_str()), Value::Int(*value));
                            }
                        }
                        self.env.define_with_kind_checked(
                            rest_name.clone(),
                            Value::Dict(Arc::new(rest_dict)),
                            binding,
                        )?;
                    }
                } else {
                    // Not a dict - bind all to null
                    for key in keys {
                        self.env.define_with_kind_checked(key.clone(), Value::Null, binding)?;
                    }
                    if let Some(rest_name) = rest {
                        self.env.define_with_kind_checked(
                            rest_name.clone(),
                            Value::Dict(Arc::new(DictMap::default())),
                            binding,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Evaluates a list of statements sequentially, stopping on return/error
    pub fn eval_stmts(&mut self, stmts: &[Stmt]) {
        let is_hoistable = |stmt: &Stmt| match stmt {
            Stmt::FuncDef { .. } => true,
            Stmt::Export { stmt } => matches!(stmt.as_ref(), Stmt::FuncDef { .. }),
            _ => false,
        };

        // Only hoist at module scope. Nested-scope hoisting captures lexical state
        // too early (before local bindings exist) and breaks closure behavior.
        let should_hoist = self.env.scopes.len() == 1;

        if should_hoist {
            for stmt in stmts {
                if is_hoistable(stmt) {
                    self.eval_stmt(stmt);
                    if self.return_value.is_some() || self.control_flow != ControlFlow::None {
                        break;
                    }
                }
            }
        }

        for stmt in stmts {
            if should_hoist && is_hoistable(stmt) {
                continue;
            }
            self.eval_stmt(stmt);
            if self.return_value.is_some() || self.control_flow != ControlFlow::None {
                break;
            }
        }
    }

    fn eval_scoped_stmts(&mut self, stmts: &[Stmt]) {
        self.env.push_scope();
        self.eval_stmts(stmts);
        self.env.pop_scope();
    }

    /// Public wrapper for evaluating a single statement (for REPL use)
    /// Returns an error if execution fails
    pub fn eval_stmt_repl(&mut self, stmt: &Stmt) -> Result<(), Box<KujoError>> {
        self.eval_stmt(stmt);

        // Check if an error occurred during evaluation
        if let Some(ref val) = self.return_value {
            match val {
                Value::Error(msg) => {
                    let err = KujoError::runtime_error(
                        msg.clone(),
                        crate::errors::SourceLocation::unknown(),
                    )
                    .with_call_stack(self.call_stack.clone());
                    self.return_value = None; // Clear error for next input
                    return Err(Box::new(err));
                }
                Value::ErrorObject { message, .. } => {
                    let err = KujoError::runtime_error(
                        message.clone(),
                        crate::errors::SourceLocation::unknown(),
                    )
                    .with_call_stack(self.call_stack.clone());
                    self.return_value = None; // Clear error for next input
                    return Err(Box::new(err));
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Public wrapper for evaluating an expression (for REPL use)
    /// Returns the evaluated value or an error
    pub fn eval_expr_repl(&mut self, expr: &Expr) -> Result<Value, Box<KujoError>> {
        let value = self.eval_expr(expr);

        // Check if the value is an error
        match value {
            Value::Error(msg) => Err(Box::new(
                KujoError::runtime_error(msg, crate::errors::SourceLocation::unknown())
                    .with_call_stack(self.call_stack.clone()),
            )),
            Value::ErrorObject { message, .. } => Err(Box::new(
                KujoError::runtime_error(message, crate::errors::SourceLocation::unknown())
                    .with_call_stack(self.call_stack.clone()),
            )),
            _ => Ok(value),
        }
    }

    /// Helper to write output to either the output buffer or stdout
    fn write_output(&self, msg: &str) {
        if let Some(out) = &self.output {
            let mut buffer = out.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let _ = writeln!(buffer, "{}", msg); // already includes newline
        } else {
            println!("{}", msg);
        }
    }

    /// Evaluates a single statement
    fn eval_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::If { condition, then_branch, else_branch } => {
                let cond_val = self.eval_expr(condition);
                if self.set_return_if_error(&cond_val) {
                    return;
                }
                if cond_val.is_truthy() {
                    self.eval_scoped_stmts(then_branch);
                } else if let Some(else_branch) = else_branch {
                    self.eval_scoped_stmts(else_branch);
                }
            }
            Stmt::Block(stmts) => {
                // Create new scope for block
                // Push new scope
                self.env.push_scope();

                self.eval_stmts(stmts);

                // Restore parent environment
                self.env.pop_scope();
            }
            Stmt::Let { pattern, value, mutable, type_annotation: _ } => {
                let val = self.eval_expr(value);
                self.set_return_if_error(&val);
                let binding =
                    if *mutable { BindingKind::Mutable } else { BindingKind::LetImmutable };
                if let Err(error) = self.bind_pattern(pattern, val, binding) {
                    self.return_value = Some(Value::Error(error));
                    return;
                }
            }
            Stmt::Const { name, value, type_annotation: _ } => {
                let val = self.eval_expr(value);
                self.set_return_if_error(&val);
                if let Err(error) =
                    self.env.define_with_kind_checked(name.clone(), val, BindingKind::Const)
                {
                    self.return_value = Some(Value::Error(error));
                    return;
                }
            }
            Stmt::Assign { target, value } => {
                let val = self.eval_expr(value);
                self.set_return_if_error(&val);

                // Always perform the assignment, even for errors
                let assignment_result = match target {
                    Expr::Identifier(name) => {
                        match self.env.assign_checked(name.clone(), val.clone()) {
                            Ok(()) => Value::Null,
                            Err(error) => Value::Error(error),
                        }
                    }
                    Expr::IndexAccess { object, index } => {
                        self.assign_index(object.as_ref(), index.as_ref(), &val)
                    }
                    Expr::FieldAccess { object, field } => {
                        self.assign_member(object.as_ref(), field, &val)
                    }
                    _ => Value::Error("Invalid assignment target".to_string()),
                };

                if self.set_return_if_error(&assignment_result) {
                    return;
                }

                // If expression evaluation resulted in an error, propagate it
                self.set_return_if_error(&val);
            }
            Stmt::FuncDef {
                name,
                params,
                param_types: _,
                return_type: _,
                body,
                is_generator,
                is_async,
            } => {
                // Named functions defined in nested scopes should capture lexical state
                // so interpreter behavior matches compiler/VM closure semantics.
                let captured_env = if self.env.scopes.len() > 1 {
                    Some(Arc::new(Mutex::new(self.env.clone())))
                } else {
                    None
                };

                // If it's a generator, create a generator value instead
                if *is_generator {
                    let gen =
                        Value::GeneratorDef(params.clone(), LeakyFunctionBody::new(body.clone()));
                    self.env.define(name.clone(), gen);
                } else if *is_async {
                    // Async functions are marked with a flag
                    // When called, they return a Promise and execute in background
                    let func = Value::AsyncFunction(
                        params.clone(),
                        LeakyFunctionBody::new(body.clone()),
                        captured_env,
                    );
                    self.env.define(name.clone(), func);
                } else {
                    let func = Value::Function(
                        params.clone(),
                        LeakyFunctionBody::new(body.clone()),
                        captured_env,
                    );
                    self.env.define(name.clone(), func);
                }
            }
            Stmt::EnumDef { name, variants } => {
                for variant in variants {
                    let tag = format!("{}::{}", name, variant);
                    // Store constructor function in env
                    let func = Value::Function(
                        vec!["$0".to_string()],
                        LeakyFunctionBody::new(vec![Stmt::Return(Some(Expr::Tag(
                            tag.clone(),
                            vec![Expr::Identifier("$0".to_string())],
                        )))]),
                        None, // Enum constructors don't need closure
                    );
                    self.env.define(tag.clone(), func);
                }
            }
            Stmt::Import { module, symbols } => {
                // Load the module
                match symbols {
                    None => {
                        // Import entire module: import math
                        // Load all exports into the current namespace
                        match self.module_loader.get_all_exports(module) {
                            Ok(exports) => {
                                for (name, value) in exports {
                                    self.env.define(name, value);
                                }
                            }
                            Err(err) => {
                                self.return_value = Some(Value::Error(err.message));
                                return;
                            }
                        }
                    }
                    Some(symbol_list) => {
                        // Selective import: from math import add, sub
                        for symbol_name in symbol_list {
                            match self.module_loader.get_symbol(module, symbol_name) {
                                Ok(value) => {
                                    self.env.define(symbol_name.clone(), value);
                                }
                                Err(err) => {
                                    self.return_value = Some(Value::Error(err.message));
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            Stmt::Export { stmt } => {
                // Export is metadata for module system - execute the inner statement
                self.eval_stmt(stmt);
            }
            Stmt::Match { value, cases, default } => {
                let val = self.eval_expr(value);
                if self.set_return_if_error(&val) {
                    return;
                }

                // Clone cases and default to avoid borrow issues during evaluation
                let cases_clone = cases.clone();
                let default_clone = default.clone();

                // Handle Result and Option types specially by extracting data first
                let (is_result_or_option, short_tag, full_tag, extracted_value) = match &val {
                    Value::Result { is_ok, value } => {
                        let tag = if *is_ok { "Ok" } else { "Err" };
                        (true, tag.to_string(), format!("Result::{}", tag), Some((**value).clone()))
                    }
                    Value::Option { is_some, value } => {
                        let tag = if *is_some { "Some" } else { "None" };
                        let val = if *is_some { Some((**value).clone()) } else { None };
                        (true, tag.to_string(), format!("Option::{}", tag), val)
                    }
                    _ => (false, String::new(), String::new(), None),
                };

                if is_result_or_option {
                    // Match against Result or Option
                    for (pattern, body) in cases_clone.iter() {
                        if let Some(open_paren) = pattern.find('(') {
                            let (enum_tag, param_var) = pattern.split_at(open_paren);
                            let param_var = param_var.trim_matches(&['(', ')'][..]);
                            let enum_tag = enum_tag.trim();
                            if short_tag == enum_tag || full_tag == enum_tag {
                                if let Some(val) = extracted_value {
                                    self.env.define(param_var.to_string(), val);
                                }
                                self.eval_stmts(body);
                                return;
                            }
                        } else {
                            let pattern = pattern.as_str().trim();
                            if pattern == short_tag || pattern == full_tag {
                                self.eval_stmts(body);
                                return;
                            }
                        }
                    }

                    if let Some(default_body) = default_clone {
                        self.eval_stmts(&default_body);
                    }
                    return;
                }

                // Handle other value types (existing code)
                let empty_map = HashMap::new();
                let (tag, fields): (String, &HashMap<String, Value>) = match &val {
                    Value::Tagged { tag, fields } => (tag.clone(), fields),
                    Value::Enum(e) => (e.clone(), &empty_map),
                    Value::Str(s) => (s.as_ref().clone(), &empty_map),
                    Value::Float(n) => (n.to_string(), &empty_map),
                    _ => {
                        if let Some(default_body) = default_clone {
                            self.eval_stmts(&default_body);
                        }
                        return;
                    }
                };

                for (pattern, body) in &cases_clone {
                    if let Some(open_paren) = pattern.find('(') {
                        let (enum_tag, param_var) = pattern.split_at(open_paren);
                        let param_var = param_var.trim_matches(&['(', ')'][..]);
                        if tag == enum_tag.trim() {
                            for i in 0.. {
                                let key = format!("${}", i);
                                if let Some(val) = fields.get(&key) {
                                    let param_name = if i == 0 {
                                        param_var.to_string()
                                    } else {
                                        format!("{}_{}", param_var, i)
                                    };
                                    self.env.define(param_name, val.clone());
                                } else {
                                    break;
                                }
                            }

                            self.eval_stmts(body);
                            return;
                        }
                    } else if pattern.as_str() == tag {
                        self.eval_stmts(body);
                        return;
                    }
                }

                if let Some(default_body) = default_clone {
                    self.eval_stmts(&default_body);
                }
            }
            Stmt::Loop { condition, body } => {
                self.with_loop_context(|interp| {
                    loop {
                        if let Some(condition) = condition.as_ref() {
                            let condition_value = interp.eval_expr(condition);
                            if interp.set_return_if_error(&condition_value) {
                                return;
                            }
                            if !condition_value.is_truthy() {
                                break;
                            }
                        }

                        interp.eval_scoped_stmts(body);

                        // Handle control flow
                        if interp.control_flow == ControlFlow::Break {
                            interp.control_flow = ControlFlow::None;
                            break;
                        } else if interp.control_flow == ControlFlow::Continue {
                            interp.control_flow = ControlFlow::None;
                            continue;
                        }

                        if interp.return_value.is_some() {
                            break;
                        }
                    }
                });
            }
            Stmt::For { var, iterable, body } => {
                self.with_loop_context(|interp| {
                    let mut iterable_value = interp.eval_expr(iterable);
                    if interp.set_return_if_error(&iterable_value) {
                        return;
                    }

                    // If we got a GeneratorDef, call it to get a Generator instance
                    // This handles cases like: for x in generator_func() { ... }
                    if let Value::GeneratorDef(_, _) = &iterable_value {
                        iterable_value = interp.call_user_function(&iterable_value, &[]);
                    }

                    // Check if this is a generator and handle it separately (needs to be mut)
                    if matches!(&iterable_value, Value::Generator { .. }) {
                        let mut gen_value = iterable_value;
                        loop {
                            let next_option = interp.generator_next(&mut gen_value);
                            match next_option {
                                Value::Option { is_some: true, value } => {
                                    // Got a value from generator
                                    interp.env.push_scope();
                                    interp.env.define(var.clone(), *value);

                                    interp.eval_stmts(body);

                                    interp.env.pop_scope();

                                    // Handle control flow
                                    if interp.control_flow == ControlFlow::Break {
                                        interp.control_flow = ControlFlow::None;
                                        break;
                                    } else if interp.control_flow == ControlFlow::Continue {
                                        interp.control_flow = ControlFlow::None;
                                        continue;
                                    }

                                    if interp.return_value.is_some() {
                                        break;
                                    }
                                }
                                Value::Option { is_some: false, .. } => {
                                    // Generator exhausted
                                    break;
                                }
                                Value::Error(msg) => {
                                    interp.return_value = Some(Value::Error(msg));
                                    break;
                                }
                                _ => {
                                    eprintln!("Unexpected value from generator iteration");
                                    break;
                                }
                            }
                        }
                        return;
                    }

                    match &iterable_value {
                        Value::Int(n) => {
                            // Numeric range: for i in 5 { ... } iterates 0..5
                            for i in 0..*n {
                                // Create new scope for loop iteration
                                // Push new scope
                                interp.env.push_scope();
                                interp.env.define(var.clone(), Value::Int(i));

                                interp.eval_stmts(body);

                                // Restore parent environment
                                interp.env.pop_scope();

                                // Handle control flow
                                if interp.control_flow == ControlFlow::Break {
                                    interp.control_flow = ControlFlow::None;
                                    break;
                                } else if interp.control_flow == ControlFlow::Continue {
                                    interp.control_flow = ControlFlow::None;
                                    continue;
                                }

                                if interp.return_value.is_some() {
                                    break;
                                }
                            }
                        }
                        Value::Float(n) => {
                            // Numeric range: for i in 5.0 { ... } iterates 0..5
                            for i in 0..*n as i64 {
                                // Create new scope for loop iteration
                                // Push new scope
                                interp.env.push_scope();
                                interp.env.define(var.clone(), Value::Int(i));

                                interp.eval_stmts(body);

                                // Restore parent environment
                                interp.env.pop_scope();

                                // Handle control flow
                                if interp.control_flow == ControlFlow::Break {
                                    interp.control_flow = ControlFlow::None;
                                    break;
                                } else if interp.control_flow == ControlFlow::Continue {
                                    interp.control_flow = ControlFlow::None;
                                    continue;
                                }

                                if interp.return_value.is_some() {
                                    break;
                                }
                            }
                        }
                        Value::Array(arr) => {
                            // Array iteration: for item in [1, 2, 3] { ... }
                            let arr_clone = arr.as_ref().clone();
                            for item in arr_clone {
                                // Create new scope for loop iteration
                                // Push new scope
                                interp.env.push_scope();
                                interp.env.define(var.clone(), item);

                                interp.eval_stmts(body);

                                // Restore parent environment
                                interp.env.pop_scope();

                                // Handle control flow
                                if interp.control_flow == ControlFlow::Break {
                                    interp.control_flow = ControlFlow::None;
                                    break;
                                } else if interp.control_flow == ControlFlow::Continue {
                                    interp.control_flow = ControlFlow::None;
                                    continue;
                                }

                                if interp.return_value.is_some() {
                                    break;
                                }
                            }
                        }
                        Value::Dict(dict) => {
                            // Dictionary iteration: for key in {"a": 1, "b": 2} { ... }
                            // Iterate over keys
                            let keys: Vec<String> =
                                dict.keys().map(|key| key.to_string()).collect();
                            for key in keys {
                                // Create new scope for loop iteration
                                // Push new scope
                                interp.env.push_scope();
                                interp.env.define(var.clone(), Value::Str(Arc::new(key)));

                                interp.eval_stmts(body);

                                // Restore parent environment
                                interp.env.pop_scope();

                                // Handle control flow
                                if interp.control_flow == ControlFlow::Break {
                                    interp.control_flow = ControlFlow::None;
                                    break;
                                } else if interp.control_flow == ControlFlow::Continue {
                                    interp.control_flow = ControlFlow::None;
                                    continue;
                                }

                                if interp.return_value.is_some() {
                                    break;
                                }
                            }
                        }
                        Value::Str(s) => {
                            // String iteration: for char in "hello" { ... }
                            let chars: Vec<char> = s.chars().collect();
                            for ch in chars {
                                // Create new scope for loop iteration
                                // Push new scope
                                interp.env.push_scope();
                                interp
                                    .env
                                    .define(var.clone(), Value::Str(Arc::new(ch.to_string())));

                                interp.eval_stmts(body);

                                // Restore parent environment
                                interp.env.pop_scope();

                                // Handle control flow
                                if interp.control_flow == ControlFlow::Break {
                                    interp.control_flow = ControlFlow::None;
                                    break;
                                } else if interp.control_flow == ControlFlow::Continue {
                                    interp.control_flow = ControlFlow::None;
                                    continue;
                                }

                                if interp.return_value.is_some() {
                                    break;
                                }
                            }
                        }
                        _ => {
                            eprintln!("Cannot iterate over non-iterable type");
                        }
                    }
                });
            }
            Stmt::While { condition, body } => {
                self.with_loop_context(|interp| {
                    // While loop: execute body while condition is truthy
                    loop {
                        let cond_val = interp.eval_expr(condition);
                        if interp.set_return_if_error(&cond_val) {
                            return;
                        }
                        if !cond_val.is_truthy() {
                            break;
                        }

                        interp.eval_scoped_stmts(body);

                        // Handle control flow
                        if interp.control_flow == ControlFlow::Break {
                            interp.control_flow = ControlFlow::None;
                            break;
                        } else if interp.control_flow == ControlFlow::Continue {
                            interp.control_flow = ControlFlow::None;
                            continue;
                        }

                        if interp.return_value.is_some() {
                            break;
                        }
                    }
                });
            }
            Stmt::Break => {
                if self.loop_depth == 0 {
                    self.return_value =
                        Some(Value::Error("break can only be used inside a loop".to_string()));
                } else {
                    self.control_flow = ControlFlow::Break;
                }
            }
            Stmt::Continue => {
                if self.loop_depth == 0 {
                    self.return_value =
                        Some(Value::Error("continue can only be used inside a loop".to_string()));
                } else {
                    self.control_flow = ControlFlow::Continue;
                }
            }
            Stmt::Return(expr) => {
                let value = expr.as_ref().map(|e| self.eval_expr(e)).unwrap_or(Value::Null);
                if Self::is_error_value(&value) {
                    self.return_value = Some(value);
                } else {
                    self.return_value = Some(Value::Return(Box::new(value)));
                }
            }
            Stmt::TryExcept { try_block, except_var, except_block } => {
                // Save current environment and create child scope for try block
                // Push new scope
                self.env.push_scope();

                self.eval_stmts(try_block);

                // Check if an error occurred (support both old Error and new ErrorObject)
                let error_occurred = matches!(
                    self.return_value,
                    Some(Value::Error(_)) | Some(Value::ErrorObject { .. })
                );

                if error_occurred {
                    let error_value = self.return_value.clone().unwrap();

                    // Pop try scope and create new scope for except block
                    self.env.pop_scope();
                    self.env.push_scope();

                    // Create error object with properties accessible via field access
                    match error_value {
                        Value::Error(msg) => {
                            // Legacy simple error - convert to struct-like object
                            let mut fields = HashMap::new();
                            fields.insert("message".to_string(), Value::Str(Arc::new(msg)));
                            fields.insert("stack".to_string(), Value::Array(Arc::new(Vec::new())));
                            fields.insert("line".to_string(), Value::Int(0));

                            self.env.define(
                                except_var.clone(),
                                Value::Struct { name: "Error".to_string(), fields },
                            );
                        }
                        Value::ErrorObject { message, stack, line, cause } => {
                            // New error object with full info
                            let mut fields = HashMap::new();
                            fields.insert("message".to_string(), Value::Str(Arc::new(message)));
                            fields.insert(
                                "stack".to_string(),
                                Value::Array(Arc::new(
                                    stack.iter().map(|s| Value::Str(Arc::new(s.clone()))).collect(),
                                )),
                            );
                            fields.insert("line".to_string(), Value::Int(line.unwrap_or(0) as i64));
                            if let Some(cause_val) = cause {
                                fields.insert("cause".to_string(), *cause_val);
                            }

                            self.env.define(
                                except_var.clone(),
                                Value::Struct { name: "Error".to_string(), fields },
                            );
                        }
                        _ => unreachable!(),
                    }

                    // Clear error and execute except block
                    self.return_value = None;
                    self.eval_stmts(except_block);
                }

                // Restore parent environment
                self.env.pop_scope();
            }
            Stmt::ExprStmt(expr) => {
                match expr {
                    // built-in print
                    Expr::Tag(name, args) if name == "print" => {
                        let mut output_parts = Vec::new();
                        for arg in args {
                            let v = self.eval_expr(arg);
                            if self.set_return_if_error(&v) {
                                return;
                            }
                            output_parts.push(Interpreter::stringify_value(&v));
                        }
                        self.write_output(&output_parts.join(" "));
                    }

                    // built-in throw
                    Expr::Tag(name, args) if name == "throw" => {
                        if let Some(arg) = args.first() {
                            let val = self.eval_expr(arg);
                            match val {
                                Value::Str(s) => {
                                    // Simple string error - create ErrorObject
                                    self.return_value = Some(Value::ErrorObject {
                                        message: s.as_ref().clone(),
                                        stack: self.call_stack.clone(),
                                        line: None,
                                        cause: None,
                                    });
                                }
                                Value::Struct { name, fields } => {
                                    // Custom error struct - wrap it in ErrorObject
                                    let message =
                                        if let Some(Value::Str(msg)) = fields.get("message") {
                                            msg.as_ref().clone()
                                        } else {
                                            format!("{} error", name)
                                        };

                                    let cause = fields.get("cause").cloned();

                                    self.return_value = Some(Value::ErrorObject {
                                        message,
                                        stack: self.call_stack.clone(),
                                        line: None,
                                        cause: cause.map(Box::new),
                                    });
                                }
                                Value::ErrorObject { .. } => {
                                    // Already an error object, propagate it
                                    self.return_value = Some(val);
                                }
                                _ => {
                                    self.return_value = Some(Value::ErrorObject {
                                        message: "error".to_string(),
                                        stack: self.call_stack.clone(),
                                        line: None,
                                        cause: None,
                                    });
                                }
                            }
                        }
                    }

                    // enum constructors or user functions (tags)
                    Expr::Tag(_, _) => {
                        let result = self.eval_expr(expr);
                        self.set_return_if_error(&result);
                    }

                    // everything else (including Call expressions and yield)
                    _ => {
                        let result = self.eval_expr(expr);
                        // Check if this is a yield (signaled by Return value)
                        if matches!(result, Value::Return(_)) {
                            self.return_value = Some(result);
                        } else {
                            self.set_return_if_error(&result);
                        }
                    }
                }
            }
            Stmt::Spawn { body } => {
                // Clone the body for the spawned thread
                let body_clone = body.clone();
                let captured_bindings = self.capture_spawn_bindings();
                let capability_policy = self.capability_policy.clone();

                // Spawn a new thread to execute the body with a transferable snapshot
                // of parent bindings. Unsupported non-transferable values remain isolated.
                std::thread::spawn(move || {
                    let mut thread_interp = Interpreter::with_capability_policy(capability_policy);

                    for (name, captured_value) in captured_bindings {
                        thread_interp.env.define(name, captured_value.into_value());
                    }

                    thread_interp.eval_stmts(&body_clone);
                });
                // Don't wait for the thread to finish - it runs in the background
            }
            Stmt::Test { .. }
            | Stmt::TestSetup { .. }
            | Stmt::TestTeardown { .. }
            | Stmt::TestGroup { .. } => {
                // Test statements are collected and executed by the test runner
                // When running normally (not in test mode), they are no-ops
                // This allows test files to be syntax-checked without running tests
            }
            Stmt::StructDef { name, fields, methods } => {
                // Extract field names
                let field_names: Vec<String> =
                    fields.iter().map(|(name, _type)| name.clone()).collect();

                // Store methods
                let mut method_map = HashMap::new();
                for method_stmt in methods {
                    if let Stmt::FuncDef {
                        name: method_name,
                        params,
                        param_types: _,
                        return_type: _,
                        body,
                        is_generator,
                        is_async: _,
                    } = method_stmt
                    {
                        if *is_generator {
                            let error = Value::Error(unsupported_struct_generator_method_message(
                                name,
                                method_name,
                            ));
                            self.set_return_if_error(&error);
                            return;
                        } else {
                            let func = Value::Function(
                                params.clone(),
                                LeakyFunctionBody::new(body.clone()),
                                Some(Arc::new(Mutex::new(self.env.clone()))),
                            );
                            method_map.insert(method_name.clone(), func);
                        }
                    }
                }

                // Store struct definition
                let struct_def =
                    Value::StructDef { name: name.clone(), field_names, methods: method_map };
                self.env.define(name.clone(), struct_def);
            }
        }
    }

    /// Evaluates an expression to produce a value
    fn eval_expr(&mut self, expr: &Expr) -> Value {
        let result = match expr {
            Expr::Int(n) => Value::Int(*n),
            Expr::Float(n) => Value::Float(*n),
            Expr::String(s) => Value::Str(Arc::new(s.clone())),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::InterpolatedString(parts) => {
                use crate::ast::InterpolatedStringPart;
                let mut result = String::new();
                for part in parts {
                    match part {
                        InterpolatedStringPart::Text(text) => {
                            result.push_str(text);
                        }
                        InterpolatedStringPart::Expr(expr) => {
                            let val = self.eval_expr(expr);
                            if Self::is_error_value(&val) {
                                return val;
                            }
                            result.push_str(&Self::stringify_value(&val));
                        }
                    }
                }
                Value::Str(Arc::new(result))
            }
            Expr::Identifier(name) => {
                if name == "null" {
                    Value::Null
                } else {
                    self.env.get(name).unwrap_or_else(|| Self::undefined_variable(name))
                }
            }
            Expr::Function {
                params,
                param_types: _,
                return_type: _,
                body,
                is_generator,
                is_async,
            } => {
                // Anonymous function expression - return as a value with captured environment
                if *is_generator {
                    Value::GeneratorDef(params.clone(), LeakyFunctionBody::new(body.clone()))
                } else if *is_async {
                    Value::AsyncFunction(
                        params.clone(),
                        LeakyFunctionBody::new(body.clone()),
                        Some(Arc::new(Mutex::new(self.env.clone()))),
                    )
                } else {
                    Value::Function(
                        params.clone(),
                        LeakyFunctionBody::new(body.clone()),
                        Some(Arc::new(Mutex::new(self.env.clone()))),
                    )
                }
            }
            Expr::UnaryOp { op, operand } => {
                let val = self.eval_expr(operand);
                if Self::is_error_value(&val) {
                    return val;
                }

                // Check if the operand is a struct with an unary operator method
                if let Some(method_name) = crate::ast::operator_methods::unary_op_method(op) {
                    if let Some(result) = self.try_call_unary_operator_method(&val, method_name) {
                        return result;
                    }
                }

                self.unary_op_value(op.as_str(), &val)
            }
            Expr::BinaryOp { left, op, right } => {
                // Handle special operators that need custom evaluation
                match op.as_str() {
                    "&&" => {
                        let l = self.eval_expr(left);
                        if Self::is_error_value(&l) {
                            return l;
                        }
                        if !l.is_truthy() {
                            return Value::Bool(false);
                        }

                        let r = self.eval_expr(right);
                        if Self::is_error_value(&r) {
                            return r;
                        }
                        return Value::Bool(r.is_truthy());
                    }
                    "||" => {
                        let l = self.eval_expr(left);
                        if Self::is_error_value(&l) {
                            return l;
                        }
                        if l.is_truthy() {
                            return Value::Bool(true);
                        }

                        let r = self.eval_expr(right);
                        if Self::is_error_value(&r) {
                            return r;
                        }
                        return Value::Bool(r.is_truthy());
                    }
                    // Null coalescing: return left if not null, otherwise right
                    "??" => {
                        let l = self.eval_expr(left);
                        if Self::is_error_value(&l) {
                            return l;
                        }
                        if matches!(l, Value::Null) {
                            return self.eval_expr(right);
                        }
                        return l;
                    }
                    // Optional chaining: return null if left is null, otherwise access field
                    "?." => {
                        let l = self.eval_expr(left);
                        if Self::is_error_value(&l) {
                            return l;
                        }
                        if matches!(l, Value::Null) {
                            return Value::Null;
                        }
                        // Right side is a String containing the field name
                        if let Expr::String(field_name) = right.as_ref() {
                            // Handle different value types
                            match l {
                                Value::Struct { name: _, fields } => {
                                    return fields.get(field_name).cloned().unwrap_or(Value::Null);
                                }
                                Value::Dict(map) => {
                                    return map
                                        .get(field_name.as_str())
                                        .cloned()
                                        .unwrap_or(Value::Null);
                                }
                                _ => return Value::Null,
                            }
                        }
                        return Value::Null;
                    }
                    // Pipe operator: pass left value as first argument to right function
                    "|>" => {
                        let value = self.eval_expr(left);
                        if Self::is_error_value(&value) {
                            return value;
                        }
                        let func = self.eval_expr(right);
                        if Self::is_error_value(&func) {
                            return func;
                        }

                        // Call the function with the value as the first argument
                        if let Value::Function(params, body, captured_env) = func {
                            // Push new scope
                            self.env.push_scope();

                            // Restore captured environment if this is a closure
                            let restore_env = if let Some(ref closure_env) = captured_env {
                                // Store current environment
                                let current = self.env.clone();
                                // Set interpreter's environment to the closure's captured environment
                                self.env = closure_env
                                    .lock()
                                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                                    .clone();
                                Some(current)
                            } else {
                                None
                            };

                            // Bind the piped value as the first parameter
                            if let Some(param) = params.first() {
                                self.env.define(param.clone(), value);
                            }

                            // Execute function body
                            if let Err(error) = self
                                .with_function_context("<pipe function>", |interp| {
                                    interp.eval_stmts(&body.get())
                                })
                            {
                                self.return_value = Some(error);
                            }
                            let mut result = Value::Null;
                            if let Some(Value::Return(val)) = self.return_value.clone() {
                                self.return_value = None;
                                result = *val;
                            } else if let Some(Value::Error(message)) = self.return_value.clone() {
                                self.return_value = None;
                                result = Value::Error(message);
                            } else if let Some(Value::ErrorObject { .. }) =
                                self.return_value.clone()
                            {
                                result = self.return_value.clone().unwrap();
                            }

                            // Restore environment if we changed it
                            if let Some(env) = restore_env {
                                self.env = env;
                            }

                            // Pop scope
                            self.env.pop_scope();

                            return result;
                        } else if let Value::NativeFunction(ref name) = func {
                            // Handle built-in functions
                            // Create a simple expression for the value and call the native function
                            let arg_expr = match value {
                                Value::Int(n) => Expr::Int(n),
                                Value::Float(n) => Expr::Float(n),
                                Value::Str(s) => Expr::String(s.as_ref().clone()),
                                Value::Bool(b) => Expr::Bool(b),
                                _ => {
                                    return Value::Error(
                                        "Cannot pipe this value type to native function"
                                            .to_string(),
                                    )
                                }
                            };
                            return self.call_native_function(name, &[arg_expr]);
                        }

                        return Value::Error(
                            "Pipe operator requires a function on the right side".to_string(),
                        );
                    }
                    _ => {}
                }

                let l = self.eval_expr(left);
                if Self::is_error_value(&l) {
                    return l;
                }
                let r = self.eval_expr(right);
                if Self::is_error_value(&r) {
                    return r;
                }

                // Check if left operand is a struct with an operator method
                if let Some(method_name) = crate::ast::operator_methods::binary_op_method(op) {
                    if let Some(result) = self.try_call_operator_method(&l, method_name, &r) {
                        return result;
                    }
                }

                self.binary_op_value(&l, op.as_str(), &r)
            }
            Expr::Call { function, args } => {
                // Special handling for method calls: obj.method(args)
                if let Expr::FieldAccess { object, field } = function.as_ref() {
                    let obj_val = self.eval_expr(object);
                    if Self::is_error_value(&obj_val) {
                        return obj_val;
                    }

                    // Handle HttpServer methods
                    if let Value::HttpServer { host, port, routes } = &obj_val {
                        match field.as_str() {
                            "route" => {
                                // server.route(method, path, handler)
                                if args.len() == 3 {
                                    let method_val = self.eval_expr(&args[0]);
                                    let path_val = self.eval_expr(&args[1]);
                                    let handler_val = self.eval_expr(&args[2]);

                                    if let (
                                        Value::Str(method),
                                        Value::Str(path),
                                        Value::Function(_, _, _),
                                    ) = (&method_val, &path_val, &handler_val)
                                    {
                                        let mut new_routes = routes.clone();
                                        new_routes.push((
                                            method.as_ref().clone(),
                                            path.as_ref().clone(),
                                            handler_val,
                                        ));
                                        return Value::HttpServer {
                                            host: host.clone(),
                                            port: *port,
                                            routes: new_routes,
                                        };
                                    }
                                }
                                return Value::Error(
                                    "route() requires (method, path, handler_function)".to_string(),
                                );
                            }
                            "listen" | "start" => {
                                if !args.is_empty() {
                                    return Value::Error(format!(
                                        "HttpServer.{} expects 0 arguments, got {}",
                                        field,
                                        args.len()
                                    ));
                                }
                                if let Err(error) = self.require_capability(
                                    NativeCapability::NetworkServer,
                                    "http_server.listen",
                                ) {
                                    return error;
                                }
                                // server.listen() / server.start() - start the HTTP server
                                return self.start_http_server(host, *port, routes.clone());
                            }
                            _ => {}
                        }
                    }

                    #[cfg(feature = "runtime-image")]
                    if let Value::Image { .. } = &obj_val {
                        if field == "save" {
                            if let Err(error) =
                                self.require_capability(NativeCapability::FilesystemWrite, "save")
                            {
                                return error;
                            }
                        }
                        let arg_values: Vec<Value> =
                            args.iter().map(|arg| self.eval_expr(arg)).collect();
                        if let Some(result) =
                            Self::call_image_method_impl(&obj_val, field, &arg_values)
                        {
                            return result;
                        }
                    }

                    // Handle Channel methods
                    if let Value::Channel(chan) = &obj_val {
                        match field.as_str() {
                            "send" => {
                                // chan.send(value) - send value to channel
                                if args.len() != 1 {
                                    return Value::Error(format!(
                                        "Channel.send expects 1 arguments, got {}",
                                        args.len()
                                    ));
                                }

                                let value = self.eval_expr(&args[0]);
                                return self.channel_send(chan, value);
                            }
                            "receive" => {
                                if !args.is_empty() {
                                    return Value::Error(format!(
                                        "Channel.receive expects 0 arguments, got {}",
                                        args.len()
                                    ));
                                }
                                return self.channel_receive_blocking(chan);
                            }
                            _ => return Value::Error(format!("Channel has no method '{}'", field)),
                        }
                    }

                    // Handle ArgParser methods
                    if let Value::Struct { name, fields } = &obj_val {
                        if name == "ArgParser" {
                            match field.as_str() {
                                "add_argument" => {
                                    // parser.add_argument(long, short, type, required, help, default)
                                    // Extract arguments
                                    let mut long_name = String::new();
                                    let mut short_name: Option<String> = None;
                                    let mut arg_type = String::from("string");
                                    let mut required = false;
                                    let mut help = String::new();
                                    let mut default: Option<String> = None;

                                    // First argument is always the long name
                                    if !args.is_empty() {
                                        if let Value::Str(s) = self.eval_expr(&args[0]) {
                                            long_name = s.as_ref().clone();
                                        }
                                    }

                                    // Process remaining keyword-style arguments
                                    // In Kujo, these come as alternating key-value pairs
                                    let mut i = 1;
                                    while i < args.len() {
                                        if let Value::Str(key) = self.eval_expr(&args[i]) {
                                            if i + 1 < args.len() {
                                                let value = self.eval_expr(&args[i + 1]);
                                                match key.as_str() {
                                                    "short" => {
                                                        if let Value::Str(s) = value {
                                                            short_name = Some(s.as_ref().clone());
                                                        }
                                                    }
                                                    "type" => {
                                                        if let Value::Str(s) = value {
                                                            arg_type = s.as_ref().clone();
                                                        }
                                                    }
                                                    "required" => {
                                                        if let Value::Bool(b) = value {
                                                            required = b;
                                                        }
                                                    }
                                                    "help" => {
                                                        if let Value::Str(s) = value {
                                                            help = s.as_ref().clone();
                                                        }
                                                    }
                                                    "default" => {
                                                        if let Value::Str(s) = value {
                                                            default = Some(s.as_ref().clone());
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                                i += 2;
                                            } else {
                                                i += 1;
                                            }
                                        } else {
                                            i += 1;
                                        }
                                    }

                                    // Create argument definition
                                    let mut arg_def = DictMap::default();
                                    arg_def.insert("long".into(), Value::Str(Arc::new(long_name)));
                                    if let Some(short) = short_name {
                                        arg_def.insert("short".into(), Value::Str(Arc::new(short)));
                                    }
                                    arg_def.insert("type".into(), Value::Str(Arc::new(arg_type)));
                                    arg_def.insert("required".into(), Value::Bool(required));
                                    arg_def.insert("help".into(), Value::Str(Arc::new(help)));
                                    if let Some(def) = default {
                                        arg_def.insert("default".into(), Value::Str(Arc::new(def)));
                                    }

                                    // Add to the parser's argument list
                                    let mut new_fields = fields.clone();
                                    if let Some(Value::Array(arg_list)) =
                                        new_fields.get("_args").cloned()
                                    {
                                        let mut arg_list_vec = Arc::try_unwrap(arg_list)
                                            .unwrap_or_else(|arc| (*arc).clone());
                                        arg_list_vec.push(Value::Dict(Arc::new(arg_def)));
                                        new_fields.insert(
                                            "_args".to_string(),
                                            Value::Array(Arc::new(arg_list_vec)),
                                        );
                                    }

                                    return Value::Struct {
                                        name: "ArgParser".to_string(),
                                        fields: new_fields,
                                    };
                                }
                                "parse" => {
                                    // parser.parse() - parse command-line arguments
                                    // Convert stored argument definitions to ArgumentDef structs
                                    let mut arg_defs = Vec::new();

                                    if let Some(Value::Array(arg_list)) = fields.get("_args") {
                                        for arg_val in arg_list.iter() {
                                            if let Value::Dict(arg_dict) = arg_val {
                                                let long_name = match arg_dict.get("long") {
                                                    Some(Value::Str(s)) => s.as_ref().clone(),
                                                    _ => continue,
                                                };

                                                let short_name = match arg_dict.get("short") {
                                                    Some(Value::Str(s)) => Some(s.as_ref().clone()),
                                                    _ => None,
                                                };

                                                let arg_type = match arg_dict.get("type") {
                                                    Some(Value::Str(s)) => s.as_ref().clone(),
                                                    _ => "string".to_string(),
                                                };

                                                let required = match arg_dict.get("required") {
                                                    Some(Value::Bool(b)) => *b,
                                                    _ => false,
                                                };

                                                let help = match arg_dict.get("help") {
                                                    Some(Value::Str(s)) => s.as_ref().clone(),
                                                    _ => String::new(),
                                                };

                                                let default = match arg_dict.get("default") {
                                                    Some(Value::Str(s)) => Some(s.as_ref().clone()),
                                                    _ => None,
                                                };

                                                arg_defs.push(builtins::ArgumentDef {
                                                    long_name,
                                                    short_name,
                                                    arg_type,
                                                    required,
                                                    help,
                                                    default,
                                                });
                                            }
                                        }
                                    }

                                    // Get command-line arguments
                                    let cli_args = builtins::get_args();

                                    // Parse arguments
                                    match builtins::parse_arguments(&arg_defs, &cli_args) {
                                        Ok(parsed) => return Value::Dict(Arc::new(parsed)),
                                        Err(msg) => {
                                            return Value::ErrorObject {
                                                message: msg,
                                                stack: Vec::new(),
                                                line: None,
                                                cause: None,
                                            }
                                        }
                                    }
                                }
                                "help" => {
                                    // parser.help() - generate help text
                                    let mut arg_defs = Vec::new();

                                    if let Some(Value::Array(arg_list)) = fields.get("_args") {
                                        for arg_val in arg_list.iter() {
                                            if let Value::Dict(arg_dict) = arg_val {
                                                let long_name = match arg_dict.get("long") {
                                                    Some(Value::Str(s)) => s.as_ref().clone(),
                                                    _ => continue,
                                                };

                                                let short_name = match arg_dict.get("short") {
                                                    Some(Value::Str(s)) => Some(s.as_ref().clone()),
                                                    _ => None,
                                                };

                                                let arg_type = match arg_dict.get("type") {
                                                    Some(Value::Str(s)) => s.as_ref().clone(),
                                                    _ => "string".to_string(),
                                                };

                                                let required = match arg_dict.get("required") {
                                                    Some(Value::Bool(b)) => *b,
                                                    _ => false,
                                                };

                                                let help = match arg_dict.get("help") {
                                                    Some(Value::Str(s)) => s.as_ref().clone(),
                                                    _ => String::new(),
                                                };

                                                let default = match arg_dict.get("default") {
                                                    Some(Value::Str(s)) => Some(s.as_ref().clone()),
                                                    _ => None,
                                                };

                                                arg_defs.push(builtins::ArgumentDef {
                                                    long_name,
                                                    short_name,
                                                    arg_type,
                                                    required,
                                                    help,
                                                    default,
                                                });
                                            }
                                        }
                                    }

                                    let app_name = match fields.get("_app_name") {
                                        Some(Value::Str(s)) => s.as_ref().clone(),
                                        _ => "program".to_string(),
                                    };

                                    let description = match fields.get("_description") {
                                        Some(Value::Str(s)) => s.as_ref().clone(),
                                        _ => String::new(),
                                    };

                                    let help_text =
                                        builtins::generate_help(&arg_defs, &app_name, &description);
                                    return Value::Str(Arc::new(help_text));
                                }
                                _ => {
                                    return Value::Error(format!(
                                        "ArgParser has no method '{}'",
                                        field
                                    ))
                                }
                            }
                        }
                    }

                    if let Value::Struct { name, fields } = &obj_val {
                        // Look up the struct definition to find the method
                        if let Some(Value::StructDef { name: _, field_names: _, methods }) =
                            self.env.get(name)
                        {
                            if let Some(Value::Function(params, body, _captured_env)) =
                                methods.get(field)
                            {
                                let evaluated_args: Vec<Value> =
                                    args.iter().map(|arg| self.eval_expr(arg)).collect();
                                if let Some(error) =
                                    evaluated_args.iter().find(|value| Self::is_error_value(value))
                                {
                                    return error.clone();
                                }

                                let arity = Self::struct_method_arity(name, field, params);
                                if let Some(error) =
                                    self.validate_callable_arity(&arity, evaluated_args.len())
                                {
                                    return error;
                                }

                                // Create new scope for method call
                                // Push new scope
                                self.env.push_scope();

                                // Check if method has 'self' as first parameter
                                let has_self_param =
                                    params.first().map(|p| p == "self").unwrap_or(false);

                                if has_self_param {
                                    // Bind self to the struct instance
                                    self.env.define("self".to_string(), obj_val.clone());

                                    // Bind remaining method parameters (skip first 'self' param)
                                    for (i, param) in params.iter().skip(1).enumerate() {
                                        if let Some(arg) = evaluated_args.get(i) {
                                            self.env.define(param.clone(), arg.clone());
                                        }
                                    }
                                } else {
                                    // Backward compatibility: bind fields directly into scope
                                    for (field_name, field_value) in fields {
                                        self.env.define(field_name.clone(), field_value.clone());
                                    }

                                    // Bind method parameters
                                    for (i, param) in params.iter().enumerate() {
                                        if let Some(arg) = evaluated_args.get(i) {
                                            self.env.define(param.clone(), arg.clone());
                                        }
                                    }
                                }

                                // Execute method body
                                if let Err(error) = self.with_function_context(
                                    &format!("{}.{}", name, field),
                                    |interp| interp.eval_stmts(&body.get()),
                                ) {
                                    self.env.pop_scope();
                                    return error;
                                }
                                let result = if let Some(Value::Return(val)) =
                                    self.return_value.clone()
                                {
                                    self.return_value = None; // Clear return value
                                    *val
                                } else if let Some(Value::Error(msg)) = self.return_value.clone() {
                                    // Propagate error - don't clear
                                    Value::Error(msg)
                                } else {
                                    // No explicit return - clear any lingering state
                                    self.return_value = None;
                                    Value::Null
                                };

                                // Restore parent environment
                                self.env.pop_scope();

                                return result;
                            }
                        }
                    }
                }
                // Regular function call
                let func_val = self.eval_expr(function);
                if Self::is_error_value(&func_val) {
                    return func_val;
                }
                let callable_name = match function.as_ref() {
                    Expr::Identifier(name) => name.clone(),
                    _ => "<anonymous function>".to_string(),
                };
                let call_result = match func_val {
                    Value::NativeFunction(name) => {
                        // Handle native function calls
                        let res = self.call_native_function(&name, args);
                        // Check if result is an error and set return_value to trigger try/except handling
                        match res {
                            Value::ErrorObject { .. } => {
                                self.return_value = Some(res.clone());
                                res
                            }
                            Value::Error(_) => {
                                self.return_value = Some(res.clone());
                                res
                            }
                            _ => res,
                        }
                    }
                    Value::Function(params, body, captured_env) => {
                        // Push to call stack
                        self.call_stack.push(callable_name.clone());

                        // Evaluate call arguments in the caller scope before any environment switch.
                        let evaluated_args: Vec<Value> =
                            args.iter().map(|arg| self.eval_expr(arg)).collect();
                        if let Some(error) =
                            evaluated_args.iter().find(|value| Self::is_error_value(value))
                        {
                            self.call_stack.pop();
                            return error.clone();
                        }

                        let arity = Self::function_arity(callable_name.clone(), &params);
                        if let Some(error) =
                            self.validate_callable_arity(&arity, evaluated_args.len())
                        {
                            self.call_stack.pop();
                            return error;
                        }

                        // Handle closure with captured environment
                        if let Some(closure_env_ref) = captured_env {
                            // Save current environment
                            let saved_env = self.env.clone();

                            // Use the captured environment
                            self.env = closure_env_ref
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner())
                                .clone();
                            self.env.push_scope();

                            for (i, param) in params.iter().enumerate() {
                                if let Some(arg) = evaluated_args.get(i) {
                                    self.env.define(param.clone(), arg.clone());
                                }
                            }

                            if let Err(error) = self
                                .with_function_context(callable_name.as_str(), |interp| {
                                    interp.eval_stmts(&body.get())
                                })
                            {
                                self.env.pop_scope();
                                self.env = saved_env;
                                self.call_stack.pop();
                                return error;
                            }

                            let result = if let Some(Value::Return(val)) = self.return_value.clone()
                            {
                                self.return_value = None;
                                *val
                            } else if let Some(Value::Error(msg)) = self.return_value.clone() {
                                Value::Error(msg)
                            } else if let Some(Value::ErrorObject { .. }) =
                                self.return_value.clone()
                            {
                                self.return_value.clone().unwrap()
                            } else {
                                self.return_value = None;
                                Value::Null
                            };

                            self.env.pop_scope();
                            // Update the captured environment
                            *closure_env_ref
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                                self.env.clone();
                            self.env = saved_env;
                            self.call_stack.pop();

                            result
                        } else {
                            // Non-closure: just create new scope
                            self.env.push_scope();

                            for (i, param) in params.iter().enumerate() {
                                if let Some(arg) = evaluated_args.get(i) {
                                    self.env.define(param.clone(), arg.clone());
                                }
                            }

                            if let Err(error) = self
                                .with_function_context(callable_name.as_str(), |interp| {
                                    interp.eval_stmts(&body.get())
                                })
                            {
                                self.env.pop_scope();
                                self.call_stack.pop();
                                return error;
                            }

                            let result = if let Some(Value::Return(val)) = self.return_value.clone()
                            {
                                self.return_value = None;
                                *val
                            } else if let Some(Value::Error(msg)) = self.return_value.clone() {
                                Value::Error(msg)
                            } else if let Some(Value::ErrorObject { .. }) =
                                self.return_value.clone()
                            {
                                self.return_value.clone().unwrap()
                            } else {
                                self.return_value = None;
                                Value::Null
                            };

                            self.env.pop_scope();
                            self.call_stack.pop();

                            result
                        }
                    }
                    Value::AsyncFunction(params, body, captured_env) => {
                        // Evaluate arguments
                        let args_vec: Vec<Value> =
                            args.iter().map(|arg| self.eval_expr(arg)).collect();
                        if let Some(error) =
                            args_vec.iter().find(|value| Self::is_error_value(value))
                        {
                            return error.clone();
                        }

                        let arity = Self::function_arity(callable_name.clone(), &params);
                        if let Some(error) = self.validate_callable_arity(&arity, args_vec.len()) {
                            return error;
                        }

                        // Clone what we need for the thread
                        let params = params.clone();
                        let body = body.clone();
                        let base_env = if let Some(ref env_ref) = captured_env {
                            env_ref.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
                        } else {
                            self.env.clone()
                        };
                        let closure_env_for_update = captured_env.clone();
                        let capability_policy = self.capability_policy.clone();

                        // Create a tokio oneshot channel for the result
                        let (tx, rx) = tokio::sync::oneshot::channel();

                        // Spawn a tokio task to execute the async function
                        AsyncRuntime::spawn_task(async move {
                            let mut async_interpreter =
                                Interpreter::with_capability_policy(capability_policy);
                            async_interpreter.env = base_env;
                            async_interpreter.env.push_scope();

                            // Bind parameters
                            for (i, param) in params.iter().enumerate() {
                                if let Some(arg) = args_vec.get(i) {
                                    async_interpreter.env.define(param.clone(), arg.clone());
                                }
                            }

                            // Execute the async function body
                            if let Err(error) = async_interpreter
                                .with_function_context("<async function>", |interp| {
                                    interp.eval_stmts(&body.get())
                                })
                            {
                                async_interpreter.env.pop_scope();
                                let _ = tx.send(Ok(error));
                                return Value::Null;
                            }

                            // Get the return value
                            let result = match async_interpreter.return_value.clone() {
                                Some(Value::Return(val)) => *val,
                                Some(Value::Error(message)) => Value::Error(message),
                                Some(value @ Value::ErrorObject { .. }) => value,
                                _ => Value::Null,
                            };

                            async_interpreter.env.pop_scope();
                            if let Some(env_ref) = closure_env_for_update {
                                *env_ref.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) =
                                    async_interpreter.env.clone();
                            }

                            // Send the result back
                            let _ = tx.send(Ok(result));

                            // Return a dummy value (task result not used, only channel matters)
                            Value::Null
                        });

                        // Return a Promise containing the receiver
                        Value::Promise {
                            receiver: Arc::new(Mutex::new(rx)),
                            is_polled: Arc::new(Mutex::new(false)),
                            cached_result: Arc::new(Mutex::new(None)),
                            task_handle: None,
                        }
                    }
                    Value::GeneratorDef(ref params, ref body) => {
                        // Calling a generator function creates a Generator instance
                        let args_vec: Vec<Value> =
                            args.iter().map(|arg| self.eval_expr(arg)).collect();
                        if let Some(error) =
                            args_vec.iter().find(|value| Self::is_error_value(value))
                        {
                            return error.clone();
                        }

                        let arity = Self::function_arity(callable_name.clone(), params);
                        if let Some(error) = self.validate_callable_arity(&arity, args_vec.len()) {
                            return error;
                        }

                        // Create a new environment for the generator
                        let mut gen_env = self.env.clone();
                        gen_env.push_scope();

                        // Bind parameters to arguments
                        for (i, param) in params.iter().enumerate() {
                            if let Some(arg) = args_vec.get(i) {
                                gen_env.define(param.clone(), arg.clone());
                            }
                        }

                        // Return a Generator instance
                        Value::Generator {
                            params: params.clone(),
                            body: body.clone(),
                            env: Arc::new(Mutex::new(gen_env)),
                            pc: 0,
                            is_exhausted: false,
                        }
                    }
                    _ => Value::Int(0),
                };
                call_result
            }
            Expr::Tag(name, args) => {
                // Enum constructors like `Result::Ok(...)` are represented as tags.
                // Treat namespaced tags as direct tagged-value construction to avoid
                // recursively invoking the generated constructor function binding.
                if name.contains("::") {
                    let mut fields = HashMap::new();
                    for (i, arg) in args.iter().enumerate() {
                        let value = self.eval_expr(arg);
                        if Self::is_error_value(&value) {
                            return value;
                        }
                        fields.insert(format!("${}", i), value);
                    }
                    return Value::Tagged { tag: name.clone(), fields };
                }

                // First check if this is a native or user function
                if let Some(func_val) = self.env.get(name) {
                    match func_val {
                        Value::NativeFunction(_) => {
                            // Call native function
                            return self.call_native_function(name, args);
                        }
                        Value::Function(params, body, captured_env) => {
                            // Push function name to call stack
                            self.call_stack.push(name.clone());

                            let evaluated_args: Vec<Value> =
                                args.iter().map(|arg| self.eval_expr(arg)).collect();
                            if let Some(error) =
                                evaluated_args.iter().find(|value| Self::is_error_value(value))
                            {
                                self.call_stack.pop();
                                return error.clone();
                            }

                            let arity = Self::function_arity(name, &params);
                            if let Some(error) =
                                self.validate_callable_arity(&arity, evaluated_args.len())
                            {
                                self.call_stack.pop();
                                return error;
                            }

                            // Handle closure with captured environment
                            if let Some(closure_env_ref) = captured_env {
                                // Save current environment
                                let saved_env = self.env.clone();

                                // Use the captured environment
                                self.env = closure_env_ref
                                    .lock()
                                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                                    .clone();
                                self.env.push_scope();

                                for (i, param) in params.iter().enumerate() {
                                    if let Some(arg) = evaluated_args.get(i) {
                                        if Self::is_error_value(arg) {
                                            self.env.pop_scope();
                                            self.env = saved_env;
                                            self.call_stack.pop();
                                            return arg.clone();
                                        }
                                        self.env.define(param.clone(), arg.clone());
                                    }
                                }

                                if let Err(error) = self
                                    .with_function_context(name.as_str(), |interp| {
                                        interp.eval_stmts(&body.get())
                                    })
                                {
                                    self.env.pop_scope();
                                    self.env = saved_env;
                                    self.call_stack.pop();
                                    return error;
                                }

                                let result = if let Some(Value::Return(val)) =
                                    self.return_value.clone()
                                {
                                    self.return_value = None;
                                    *val
                                } else if let Some(Value::Error(msg)) = self.return_value.clone() {
                                    Value::Error(msg)
                                } else if let Some(Value::ErrorObject { .. }) =
                                    self.return_value.clone()
                                {
                                    self.return_value.clone().unwrap()
                                } else {
                                    self.return_value = None;
                                    Value::Null
                                };

                                self.env.pop_scope();
                                // Update the captured environment
                                *closure_env_ref
                                    .lock()
                                    .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                                    self.env.clone();
                                self.env = saved_env;
                                self.call_stack.pop();

                                return result;
                            } else {
                                // Non-closure: just create new scope
                                self.env.push_scope();

                                for (i, param) in params.iter().enumerate() {
                                    if let Some(arg) = evaluated_args.get(i) {
                                        if Self::is_error_value(arg) {
                                            self.env.pop_scope();
                                            self.call_stack.pop();
                                            return arg.clone();
                                        }
                                        self.env.define(param.clone(), arg.clone());
                                    }
                                }

                                if let Err(error) = self
                                    .with_function_context(name.as_str(), |interp| {
                                        interp.eval_stmts(&body.get())
                                    })
                                {
                                    self.env.pop_scope();
                                    self.call_stack.pop();
                                    return error;
                                }

                                let result = if let Some(Value::Return(val)) =
                                    self.return_value.clone()
                                {
                                    self.return_value = None;
                                    *val
                                } else if let Some(Value::Error(msg)) = self.return_value.clone() {
                                    Value::Error(msg)
                                } else if let Some(Value::ErrorObject { .. }) =
                                    self.return_value.clone()
                                {
                                    self.return_value.clone().unwrap()
                                } else {
                                    self.return_value = None;
                                    Value::Null
                                };

                                self.env.pop_scope();
                                self.call_stack.pop();

                                return result;
                            }
                        }
                        Value::GeneratorDef(ref params, ref body) => {
                            // Calling a generator function creates a Generator instance
                            let args_vec: Vec<Value> =
                                args.iter().map(|arg| self.eval_expr(arg)).collect();
                            if let Some(error) =
                                args_vec.iter().find(|value| Self::is_error_value(value))
                            {
                                return error.clone();
                            }

                            let arity = Self::function_arity(name, params);
                            if let Some(error) =
                                self.validate_callable_arity(&arity, args_vec.len())
                            {
                                return error;
                            }

                            // Create a new environment for the generator
                            let mut gen_env = self.env.clone();
                            gen_env.push_scope();

                            // Bind parameters to arguments
                            for (i, param) in params.iter().enumerate() {
                                if let Some(arg) = args_vec.get(i) {
                                    gen_env.define(param.clone(), arg.clone());
                                }
                            }

                            // Return a Generator instance
                            return Value::Generator {
                                params: params.clone(),
                                body: body.clone(),
                                env: Arc::new(Mutex::new(gen_env)),
                                pc: 0,
                                is_exhausted: false,
                            };
                        }
                        _ => {}
                    }
                }

                // Otherwise, treat as enum constructor
                let mut fields = HashMap::new();
                for (i, arg) in args.iter().enumerate() {
                    let value = self.eval_expr(arg);
                    if Self::is_error_value(&value) {
                        return value;
                    }
                    fields.insert(format!("${}", i), value);
                }
                Value::Tagged { tag: name.clone(), fields }
            }
            Expr::StructInstance { name, fields } => {
                // Create a struct instance
                let mut field_values = HashMap::new();
                for (field_name, field_expr) in fields {
                    let value = self.eval_expr(field_expr);
                    if Self::is_error_value(&value) {
                        return value;
                    }
                    field_values.insert(field_name.clone(), value);
                }
                Value::Struct { name: name.clone(), fields: field_values }
            }
            Expr::FieldAccess { object, field } => {
                let obj_val = self.eval_expr(object);
                match obj_val {
                    Value::Struct { name: _, fields } => {
                        // Access field from struct instance
                        fields
                            .get(field)
                            .cloned()
                            .unwrap_or_else(|| Value::Error(format!("Field not found: {}", field)))
                    }
                    #[cfg(feature = "runtime-image")]
                    Value::Image { data, format } => {
                        // Access image properties
                        match field.as_str() {
                            "width" => {
                                let img =
                                    data.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                                Value::Float(img.width() as f64)
                            }
                            "height" => {
                                let img =
                                    data.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                                Value::Float(img.height() as f64)
                            }
                            "format" => Value::Str(Arc::new(format)),
                            _ => Value::Error(format!("Image has no field '{}'", field)),
                        }
                    }
                    Value::Error(_) | Value::ErrorObject { .. } => obj_val,
                    _ => Value::Error(format!(
                        "Cannot access field or method '{}' on non-struct value",
                        field
                    )),
                }
            }
            Expr::ArrayLiteral(elements) => {
                use crate::ast::ArrayElement;
                let mut values = Vec::new();

                for elem in elements {
                    match elem {
                        ArrayElement::Single(expr) => {
                            let value = self.eval_expr(expr);
                            if Self::is_error_value(&value) {
                                return value;
                            }
                            values.push(value);
                        }
                        ArrayElement::Spread(expr) => {
                            // Evaluate spread expression and merge its elements
                            let spread_val = self.eval_expr(expr);
                            if Self::is_error_value(&spread_val) {
                                return spread_val;
                            }
                            if let Value::Array(arr) = spread_val {
                                values.extend(arr.iter().cloned());
                            }
                            // If not an array, ignore the spread
                        }
                    }
                }

                Value::Array(Arc::new(values))
            }
            Expr::DictLiteral(pairs) => {
                use crate::ast::DictElement;
                let mut map = DictMap::default();

                for elem in pairs {
                    match elem {
                        DictElement::Pair(key_expr, val_expr) => {
                            let key = match self.eval_expr(key_expr) {
                                error if Self::is_error_value(&error) => return error,
                                Value::Str(s) => s.as_ref().clone(),
                                Value::Int(n) => n.to_string(),
                                Value::Float(n) => n.to_string(),
                                _ => continue,
                            };
                            let value = self.eval_expr(val_expr);
                            if Self::is_error_value(&value) {
                                return value;
                            }
                            map.insert(Arc::from(key), value);
                        }
                        DictElement::Spread(expr) => {
                            // Evaluate spread expression and merge its entries
                            let spread_val = self.eval_expr(expr);
                            if Self::is_error_value(&spread_val) {
                                return spread_val;
                            }
                            if let Value::Dict(dict) = spread_val {
                                for (k, v) in dict.iter() {
                                    map.insert(k.clone(), v.clone());
                                }
                            }
                            // If not a dict, ignore the spread
                        }
                    }
                }

                Value::Dict(Arc::new(map))
            }
            Expr::IndexAccess { object, index } => {
                let obj_val = self.eval_expr(object);
                if Self::is_error_value(&obj_val) {
                    return obj_val;
                }
                let idx_val = self.eval_expr(index);
                if Self::is_error_value(&idx_val) {
                    return idx_val;
                }

                Self::index_value(&obj_val, &idx_val)
            }
            Expr::Ok(value_expr) => {
                let value = self.eval_expr(value_expr);
                Value::Result { is_ok: true, value: Box::new(value) }
            }
            Expr::Err(error_expr) => {
                let error = self.eval_expr(error_expr);
                Value::Result { is_ok: false, value: Box::new(error) }
            }
            Expr::Some(value_expr) => {
                let value = self.eval_expr(value_expr);
                Value::Option { is_some: true, value: Box::new(value) }
            }
            Expr::None => Value::Option { is_some: false, value: Box::new(Value::Null) },
            Expr::Try(expr) => {
                let value = self.eval_expr(expr);
                match value {
                    Value::Result { is_ok, value } => {
                        if is_ok {
                            // Return the Ok value
                            *value
                        } else {
                            // Early return with the Err value wrapped in Result
                            self.return_value = Some(Value::Return(Box::new(Value::Result {
                                is_ok: false,
                                value,
                            })));
                            Value::Null // This will be ignored due to early return
                        }
                    }
                    _ => {
                        // ? operator only works on Result values
                        Value::Error(
                            "Try operator (?) can only be used on Result values".to_string(),
                        )
                    }
                }
            }
            Expr::Yield(value_expr) => {
                // Yield expression - should only be used inside generators
                // For now, return the yielded value wrapped in a special marker
                // The generator execution logic will handle this properly
                let yielded =
                    if let Some(expr) = value_expr { self.eval_expr(expr) } else { Value::Null };
                // Use a Return value to signal yield - generators will intercept this
                Value::Return(Box::new(yielded))
            }
            Expr::Await(promise_expr) => {
                // Await expression - wait for a promise to resolve using tokio runtime
                let promise_value = self.eval_expr(promise_expr);

                // If it's a promise, wait for it to resolve
                match promise_value {
                    Value::Promise { receiver, is_polled, cached_result, .. } => {
                        // Check if we've already polled this promise
                        {
                            let polled = match lock_or_runtime_error(
                                is_polled.as_ref(),
                                "await.promise.is_polled",
                            ) {
                                Ok(guard) => guard,
                                Err(error) => return error,
                            };
                            let cached = match lock_or_runtime_error(
                                cached_result.as_ref(),
                                "await.promise.cached_result",
                            ) {
                                Ok(guard) => guard,
                                Err(error) => return error,
                            };

                            if *polled {
                                // Use cached result
                                return match cached.as_ref() {
                                    Some(Ok(val)) => val.clone(),
                                    Some(Err(err)) => {
                                        Value::Error(format!("Promise rejected: {}", err))
                                    }
                                    None => Value::Error(
                                        "Promise polled but no result cached".to_string(),
                                    ),
                                };
                            }
                            // Locks dropped here
                        }

                        // Poll the promise using tokio runtime
                        // We need to take ownership of the receiver to await it
                        let result = {
                            let mut recv_guard = match lock_or_runtime_error(
                                receiver.as_ref(),
                                "await.promise.receiver",
                            ) {
                                Ok(guard) => guard,
                                Err(error) => return error,
                            };
                            // Take ownership by replacing with a dummy closed channel
                            let (dummy_tx, dummy_rx) = tokio::sync::oneshot::channel();
                            drop(dummy_tx); // Close immediately
                            let actual_rx = std::mem::replace(&mut *recv_guard, dummy_rx);
                            drop(recv_guard); // Release lock before blocking

                            // Block on the receiver using tokio runtime
                            AsyncRuntime::block_on(actual_rx)
                        };

                        // Now update the cache with the result
                        let mut polled = match lock_or_runtime_error(
                            is_polled.as_ref(),
                            "await.promise.is_polled",
                        ) {
                            Ok(guard) => guard,
                            Err(error) => return error,
                        };
                        let mut cached = match lock_or_runtime_error(
                            cached_result.as_ref(),
                            "await.promise.cached_result",
                        ) {
                            Ok(guard) => guard,
                            Err(error) => return error,
                        };

                        match result {
                            Ok(Ok(value)) => {
                                // Cache the successful result
                                *cached = Some(Ok(value.clone()));
                                *polled = true;
                                value
                            }
                            Ok(Err(error)) => {
                                // Cache the error
                                *cached = Some(Err(error.clone()));
                                *polled = true;
                                Value::Error(format!("Promise rejected: {}", error))
                            }
                            Err(_) => {
                                // Channel closed without sending - this shouldn't happen
                                *cached = Some(Err("Promise never resolved".to_string()));
                                *polled = true;
                                Value::Error("Promise never resolved (channel closed)".to_string())
                            }
                        }
                    }
                    _ => {
                        // Not a promise - just return the value
                        promise_value
                    }
                }
            }
            Expr::MethodCall { object, method, args } => {
                // Method call on an expression - used for iterator chaining
                let obj_value = self.eval_expr(object);
                if Self::is_error_value(&obj_value) {
                    return obj_value;
                }
                let arg_values: Vec<Value> = args.iter().map(|arg| self.eval_expr(arg)).collect();
                if let Some(error) = arg_values.iter().find(|value| Self::is_error_value(value)) {
                    return error.clone();
                }

                // Call the method on the object
                self.call_method(obj_value, method, arg_values)
            }
            Expr::Spread(_) => {
                // Spread expressions should only appear inside array/dict literals
                // If we reach here, it's a syntax error, but we'll return an error value
                Value::Error(
                    "Spread operator (...) can only be used inside array or dict literals"
                        .to_string(),
                )
            }
        };
        result
    }

    #[cfg(feature = "runtime-image")]
    pub(crate) fn call_image_method_impl(
        obj: &Value,
        method: &str,
        args: &[Value],
    ) -> Option<Value> {
        let (data, format) = match obj {
            Value::Image { data, format } => (data, format),
            _ => return None,
        };

        let as_f32 = |value: &Value| -> Option<f32> {
            match value {
                Value::Int(n) => Some(*n as f32),
                Value::Float(n) => Some(*n as f32),
                _ => None,
            }
        };
        let as_f64 = |value: &Value| -> Option<f64> {
            match value {
                Value::Int(n) => Some(*n as f64),
                Value::Float(n) => Some(*n),
                _ => None,
            }
        };

        let coordinate = |method: &str, name: &str, value: &Value| -> Result<u32, Value> {
            match value {
                Value::Int(n) if *n >= 0 && *n <= u32::MAX as i64 => Ok(*n as u32),
                _ => Err(Value::Error(format!(
                    "{} requires {} to be a non-negative integer",
                    method, name
                ))),
            }
        };
        let channel = |value: &Value| -> Option<u8> {
            match value {
                Value::Int(n) if (0..=255).contains(n) => Some(*n as u8),
                _ => None,
            }
        };

        let result = match method {
            "width" => {
                if !args.is_empty() {
                    Value::Error("width expects no arguments".to_string())
                } else {
                    let img = match lock_or_runtime_error(data.as_ref(), "image.width") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    Value::Int(img.width() as i64)
                }
            }
            "height" => {
                if !args.is_empty() {
                    Value::Error("height expects no arguments".to_string())
                } else {
                    let img = match lock_or_runtime_error(data.as_ref(), "image.height") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    Value::Int(img.height() as i64)
                }
            }
            "format" => {
                if !args.is_empty() {
                    Value::Error("format expects no arguments".to_string())
                } else {
                    Value::Str(Arc::new(format.clone()))
                }
            }
            "get_pixel" => {
                if args.len() != 2 {
                    Value::Error("get_pixel requires x and y arguments".to_string())
                } else {
                    let x = match coordinate("get_pixel", "x", &args[0]) {
                        Ok(value) => value,
                        Err(error) => return Some(error),
                    };
                    let y = match coordinate("get_pixel", "y", &args[1]) {
                        Ok(value) => value,
                        Err(error) => return Some(error),
                    };
                    let img = match lock_or_runtime_error(data.as_ref(), "image.get_pixel") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    if x >= img.width() || y >= img.height() {
                        Value::Error(format!(
                            "get_pixel coordinates ({}, {}) are outside image bounds {}x{}",
                            x,
                            y,
                            img.width(),
                            img.height()
                        ))
                    } else {
                        let pixel = image::GenericImageView::get_pixel(&*img, x, y);
                        Value::Array(Arc::new(
                            pixel.0.into_iter().map(|value| Value::Int(value as i64)).collect(),
                        ))
                    }
                }
            }
            "set_pixel" => {
                if args.len() != 5 && args.len() != 6 {
                    Value::Error(
                        "set_pixel requires x, y, r, g, b, and optional a arguments".to_string(),
                    )
                } else {
                    let x = match coordinate("set_pixel", "x", &args[0]) {
                        Ok(value) => value,
                        Err(error) => return Some(error),
                    };
                    let y = match coordinate("set_pixel", "y", &args[1]) {
                        Ok(value) => value,
                        Err(error) => return Some(error),
                    };
                    let channels: Option<Vec<u8>> = args[2..].iter().map(channel).collect();
                    let channels = match channels {
                        Some(values) => values,
                        None => {
                            return Some(Value::Error(
                                "set_pixel color channels must be integers from 0 to 255"
                                    .to_string(),
                            ))
                        }
                    };
                    let mut img = match lock_or_runtime_error(data.as_ref(), "image.set_pixel") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    if x >= img.width() || y >= img.height() {
                        Value::Error(format!(
                            "set_pixel coordinates ({}, {}) are outside image bounds {}x{}",
                            x,
                            y,
                            img.width(),
                            img.height()
                        ))
                    } else {
                        let alpha = if channels.len() == 4 {
                            channels[3]
                        } else {
                            image::GenericImageView::get_pixel(&*img, x, y).0[3]
                        };
                        image::GenericImage::put_pixel(
                            &mut *img,
                            x,
                            y,
                            image::Rgba([channels[0], channels[1], channels[2], alpha]),
                        );
                        Value::Bool(true)
                    }
                }
            }
            "resize" => {
                if args.len() < 2 {
                    Value::Error("resize requires at least width and height arguments".to_string())
                } else if let (Some(w), Some(h)) = (as_f32(&args[0]), as_f32(&args[1])) {
                    let img = match lock_or_runtime_error(data.as_ref(), "image.resize") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    let resized = if args.len() >= 3 {
                        if let Value::Str(mode) = &args[2] {
                            if mode.as_ref() == "fit" {
                                img.resize(
                                    w as u32,
                                    h as u32,
                                    image::imageops::FilterType::Lanczos3,
                                )
                            } else {
                                img.resize_exact(
                                    w as u32,
                                    h as u32,
                                    image::imageops::FilterType::Lanczos3,
                                )
                            }
                        } else {
                            img.resize_exact(
                                w as u32,
                                h as u32,
                                image::imageops::FilterType::Lanczos3,
                            )
                        }
                    } else {
                        img.resize_exact(w as u32, h as u32, image::imageops::FilterType::Lanczos3)
                    };
                    Value::Image { data: Arc::new(Mutex::new(resized)), format: format.clone() }
                } else {
                    Value::Error("resize requires numeric width and height".to_string())
                }
            }
            "crop" => {
                if args.len() < 4 {
                    Value::Error("crop requires x, y, width, and height arguments".to_string())
                } else if let (Some(x), Some(y), Some(w), Some(h)) =
                    (as_f32(&args[0]), as_f32(&args[1]), as_f32(&args[2]), as_f32(&args[3]))
                {
                    let mut img = match lock_or_runtime_error(data.as_ref(), "image.crop") {
                        Ok(guard) => guard.clone(),
                        Err(error) => return Some(error),
                    };
                    let cropped = img.crop(x as u32, y as u32, w as u32, h as u32);
                    Value::Image { data: Arc::new(Mutex::new(cropped)), format: format.clone() }
                } else {
                    Value::Error("crop requires numeric x, y, width, and height".to_string())
                }
            }
            "rotate" => {
                if args.is_empty() {
                    Value::Error("rotate requires a degrees argument".to_string())
                } else if let Some(degrees) = as_f32(&args[0]) {
                    let img = match lock_or_runtime_error(data.as_ref(), "image.rotate") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    let rotated = match degrees as i32 {
                        90 => img.rotate90(),
                        180 => img.rotate180(),
                        270 => img.rotate270(),
                        _ => {
                            return Some(Value::Error(
                                "rotate only supports 90, 180, or 270 degrees".to_string(),
                            ))
                        }
                    };
                    Value::Image { data: Arc::new(Mutex::new(rotated)), format: format.clone() }
                } else {
                    Value::Error("rotate requires a numeric degrees argument".to_string())
                }
            }
            "flip" => {
                if args.is_empty() {
                    Value::Error(
                        "flip requires a direction argument ('horizontal' or 'vertical')"
                            .to_string(),
                    )
                } else if let Value::Str(direction) = &args[0] {
                    let img = match lock_or_runtime_error(data.as_ref(), "image.flip") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    let flipped = match direction.as_str() {
                        "horizontal" => img.fliph(),
                        "vertical" => img.flipv(),
                        _ => {
                            return Some(Value::Error(
                                "flip direction must be 'horizontal' or 'vertical'".to_string(),
                            ))
                        }
                    };
                    Value::Image { data: Arc::new(Mutex::new(flipped)), format: format.clone() }
                } else {
                    Value::Error("flip requires a string direction argument".to_string())
                }
            }
            "save" => {
                if args.is_empty() {
                    Value::Error("save requires a path argument".to_string())
                } else if let Value::Str(path) = &args[0] {
                    let img = match lock_or_runtime_error(data.as_ref(), "image.save") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    match img.save(path.as_ref()) {
                        Ok(_) => Value::Bool(true),
                        Err(e) => Value::Error(format!("Failed to save image: {}", e)),
                    }
                } else {
                    Value::Error("save requires a string path argument".to_string())
                }
            }
            "to_grayscale" => {
                let img = match lock_or_runtime_error(data.as_ref(), "image.to_grayscale") {
                    Ok(guard) => guard,
                    Err(error) => return Some(error),
                };
                let gray = img.grayscale();
                Value::Image { data: Arc::new(Mutex::new(gray)), format: format.clone() }
            }
            "blur" => {
                if args.is_empty() {
                    Value::Error("blur requires a sigma argument".to_string())
                } else if let Some(sigma) = as_f32(&args[0]) {
                    let img = match lock_or_runtime_error(data.as_ref(), "image.blur") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    let blurred = img.blur(sigma);
                    Value::Image { data: Arc::new(Mutex::new(blurred)), format: format.clone() }
                } else {
                    Value::Error("blur requires a numeric sigma argument".to_string())
                }
            }
            "adjust_brightness" => {
                if args.is_empty() {
                    Value::Error("adjust_brightness requires a factor argument".to_string())
                } else if let Some(factor) = as_f64(&args[0]) {
                    let img = match lock_or_runtime_error(data.as_ref(), "image.adjust_brightness")
                    {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    let adjusted = img.brighten((factor * 50.0) as i32);
                    Value::Image { data: Arc::new(Mutex::new(adjusted)), format: format.clone() }
                } else {
                    Value::Error("adjust_brightness requires a numeric factor argument".to_string())
                }
            }
            "adjust_contrast" => {
                if args.is_empty() {
                    Value::Error("adjust_contrast requires a factor argument".to_string())
                } else if let Some(factor) = as_f32(&args[0]) {
                    let img = match lock_or_runtime_error(data.as_ref(), "image.adjust_contrast") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    let adjusted = img.adjust_contrast(factor);
                    Value::Image { data: Arc::new(Mutex::new(adjusted)), format: format.clone() }
                } else {
                    Value::Error("adjust_contrast requires a numeric factor argument".to_string())
                }
            }
            _ => Value::Error(format!("Image has no method '{}'", method)),
        };

        Some(result)
    }

    #[cfg(not(feature = "runtime-image"))]
    pub(crate) fn call_image_method_impl(
        obj: &Value,
        method: &str,
        args: &[Value],
    ) -> Option<Value> {
        let _ = (obj, method, args);
        None
    }

    fn channel_send(
        &self,
        chan: &Arc<Mutex<(std::sync::mpsc::SyncSender<Value>, std::sync::mpsc::Receiver<Value>)>>,
        value: Value,
    ) -> Value {
        let chan_lock = match lock_or_runtime_error(chan.as_ref(), "channel.send") {
            Ok(guard) => guard,
            Err(error) => return error,
        };
        let (sender, _) = &*chan_lock;
        match sender.send(value) {
            Ok(_) => Value::Bool(true),
            Err(_) => Value::Error("Failed to send to channel".to_string()),
        }
    }

    fn channel_receive_blocking(
        &self,
        chan: &Arc<Mutex<(std::sync::mpsc::SyncSender<Value>, std::sync::mpsc::Receiver<Value>)>>,
    ) -> Value {
        loop {
            let receive_result = {
                let chan_lock = match lock_or_runtime_error(chan.as_ref(), "channel.receive") {
                    Ok(guard) => guard,
                    Err(error) => return error,
                };
                let (_, receiver) = &*chan_lock;
                receiver.try_recv()
            };

            match receive_result {
                Ok(value) => return value,
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return Value::Error("Channel disconnected".to_string());
                }
            }
        }
    }

    /// Call a method on a value (used for iterator chaining and other method calls)
    fn call_method(&mut self, obj: Value, method: &str, args: Vec<Value>) -> Value {
        if method == "save" {
            #[cfg(feature = "runtime-image")]
            if matches!(&obj, Value::Image { .. }) {
                if let Err(error) =
                    self.require_capability(NativeCapability::FilesystemWrite, "save")
                {
                    return error;
                }
            }
        }

        if let Some(result) = Self::call_image_method_impl(&obj, method, &args) {
            return result;
        }

        if let Value::HttpServer { host, port, routes } = &obj {
            return match method {
                "route" => {
                    if args.len() != 3 {
                        Value::Error(
                            "route() requires (method, path, handler_function)".to_string(),
                        )
                    } else if let (Value::Str(http_method), Value::Str(path), handler) =
                        (&args[0], &args[1], &args[2])
                    {
                        if matches!(handler, Value::Function(_, _, _) | Value::NativeFunction(_)) {
                            let mut new_routes = routes.clone();
                            new_routes.push((
                                http_method.as_ref().clone(),
                                path.as_ref().clone(),
                                handler.clone(),
                            ));
                            Value::HttpServer {
                                host: host.clone(),
                                port: *port,
                                routes: new_routes,
                            }
                        } else {
                            Value::Error(
                                "route() requires (method, path, handler_function)".to_string(),
                            )
                        }
                    } else {
                        Value::Error(
                            "route() requires (method, path, handler_function)".to_string(),
                        )
                    }
                }
                "listen" | "start" => {
                    if !args.is_empty() {
                        Value::Error(format!(
                            "HttpServer.{} expects 0 arguments, got {}",
                            method,
                            args.len()
                        ))
                    } else if let Err(error) = self
                        .require_capability(NativeCapability::NetworkServer, "http_server.listen")
                    {
                        error
                    } else {
                        self.start_http_server(host, *port, routes.clone())
                    }
                }
                _ => Value::Error(format!("Unknown method: {}", method)),
            };
        }

        if let Value::Channel(chan) = &obj {
            return match method {
                "send" => {
                    if args.len() != 1 {
                        return Value::Error(format!(
                            "Channel.send expects 1 arguments, got {}",
                            args.len()
                        ));
                    }

                    self.channel_send(chan, args[0].clone())
                }
                "receive" => {
                    if !args.is_empty() {
                        return Value::Error(format!(
                            "Channel.receive expects 0 arguments, got {}",
                            args.len()
                        ));
                    }

                    self.channel_receive_blocking(chan)
                }
                _ => Value::Error(format!("Channel has no method '{}'", method)),
            };
        }

        match method {
            // Iterator methods
            "filter" if args.len() == 1 => {
                // Create an iterator with a filter function
                match &obj {
                    Value::Iterator { .. } => Value::Iterator {
                        // Preserve evaluation order by layering a new iterator stage.
                        source: Box::new(obj.clone()),
                        index: 0,
                        transformer: None,
                        filter_fn: Some(Box::new(args[0].clone())),
                        take_count: None,
                    },
                    Value::Array(_) => {
                        // Convert array to iterator with filter
                        Value::Iterator {
                            source: Box::new(obj),
                            index: 0,
                            transformer: None,
                            filter_fn: Some(Box::new(args[0].clone())),
                            take_count: None,
                        }
                    }
                    _ => Value::Error(
                        "filter() can only be called on iterators or arrays".to_string(),
                    ),
                }
            }
            "map" if args.len() == 1 => {
                // Create an iterator with a transformer function
                match &obj {
                    Value::Iterator { .. } => Value::Iterator {
                        // Preserve evaluation order by layering a new iterator stage.
                        source: Box::new(obj.clone()),
                        index: 0,
                        transformer: Some(Box::new(args[0].clone())),
                        filter_fn: None,
                        take_count: None,
                    },
                    Value::Array(_) => {
                        // Convert array to iterator with map
                        Value::Iterator {
                            source: Box::new(obj),
                            index: 0,
                            transformer: Some(Box::new(args[0].clone())),
                            filter_fn: None,
                            take_count: None,
                        }
                    }
                    _ => {
                        Value::Error("map() can only be called on iterators or arrays".to_string())
                    }
                }
            }
            "take" if args.len() == 1 => {
                // Limit the number of items
                if let Value::Int(n) = args[0] {
                    match &obj {
                        Value::Iterator {
                            source,
                            index,
                            transformer,
                            filter_fn,
                            take_count: _,
                        } => Value::Iterator {
                            source: source.clone(),
                            index: *index,
                            transformer: transformer.clone(),
                            filter_fn: filter_fn.clone(),
                            take_count: Some(n as usize),
                        },
                        Value::Array(_) => Value::Iterator {
                            source: Box::new(obj),
                            index: 0,
                            transformer: None,
                            filter_fn: None,
                            take_count: Some(n as usize),
                        },
                        _ => Value::Error(
                            "take() can only be called on iterators or arrays".to_string(),
                        ),
                    }
                } else {
                    Value::Error("take() requires an integer argument".to_string())
                }
            }
            "collect" if args.is_empty() => {
                // Collect iterator into an array
                self.collect_iterator(obj)
            }
            "next" if args.is_empty() => {
                // Get next value from iterator
                self.iterator_next(obj)
            }
            _ => {
                // Check if it's a struct method
                match obj {
                    Value::Struct { name, fields } => {
                        if let Some(Value::StructDef { name: _, field_names: _, methods }) =
                            self.env.get(&name)
                        {
                            if let Some(Value::Function(params, body, _captured_env)) =
                                methods.get(method)
                            {
                                let arity = Self::struct_method_arity(&name, method, params);
                                if let Some(error) =
                                    self.validate_callable_arity(&arity, args.len())
                                {
                                    return error;
                                }

                                self.env.push_scope();

                                let has_self_param =
                                    params.first().map(|param| param == "self").unwrap_or(false);

                                if has_self_param {
                                    self.env.define(
                                        "self".to_string(),
                                        Value::Struct {
                                            name: name.clone(),
                                            fields: fields.clone(),
                                        },
                                    );

                                    for (index, param) in params.iter().skip(1).enumerate() {
                                        if let Some(arg) = args.get(index) {
                                            self.env.define(param.clone(), arg.clone());
                                        }
                                    }
                                } else {
                                    for (field_name, field_value) in &fields {
                                        self.env.define(field_name.clone(), field_value.clone());
                                    }

                                    for (index, param) in params.iter().enumerate() {
                                        if let Some(arg) = args.get(index) {
                                            self.env.define(param.clone(), arg.clone());
                                        }
                                    }
                                }

                                if let Err(error) = self.with_function_context(
                                    &format!("{}.{}", name, method),
                                    |interp| interp.eval_stmts(&body.get()),
                                ) {
                                    self.env.pop_scope();
                                    return error;
                                }

                                let result =
                                    if let Some(Value::Return(value)) = self.return_value.clone() {
                                        self.return_value = None;
                                        *value
                                    } else if let Some(Value::Error(message)) =
                                        self.return_value.clone()
                                    {
                                        Value::Error(message)
                                    } else {
                                        self.return_value = None;
                                        Value::Null
                                    };

                                self.env.pop_scope();

                                return result;
                            }
                        }

                        Value::Error(format!("Unknown method: {}", method))
                    }
                    _ => Value::Error(format!("Unknown method: {}", method)),
                }
            }
        }
    }

    /// Collect all values from an iterator into an array
    fn collect_iterator(&mut self, mut iterator: Value) -> Value {
        let mut result = Vec::new();

        loop {
            match &mut iterator {
                Value::Iterator { source, index, transformer, filter_fn, take_count } => {
                    // Check if we've reached the take limit
                    if let Some(limit) = take_count {
                        if result.len() >= *limit {
                            return Value::Array(Arc::new(result));
                        }
                    }

                    // Get next item from source
                    match source.as_mut() {
                        Value::Array(items) => {
                            // Find next item that passes filter
                            loop {
                                if *index >= items.len() {
                                    // No more items
                                    return Value::Array(Arc::new(result));
                                }

                                let mut item = items[*index].clone();
                                *index += 1;

                                // Apply filter if present
                                if let Some(filter) = filter_fn {
                                    let args_vec = vec![item.clone()];
                                    let filter_result =
                                        self.call_user_function(filter.as_ref(), &args_vec);
                                    match filter_result {
                                        Value::Bool(true) => {
                                            // Item passes filter - continue processing
                                        }
                                        _ => {
                                            // Item filtered out, try next
                                            continue;
                                        }
                                    }
                                }

                                // Apply transformer if present
                                if let Some(trans) = transformer {
                                    let args_vec = vec![item];
                                    item = self.call_user_function(trans.as_ref(), &args_vec);
                                }

                                result.push(item);

                                // Check take limit after adding
                                if let Some(limit) = take_count {
                                    if result.len() >= *limit {
                                        return Value::Array(Arc::new(result));
                                    }
                                }

                                break; // Found one item, continue outer loop
                            }
                        }
                        Value::Generator { .. } => {
                            // Get next value from generator
                            let next_option = self.generator_next(source);
                            match next_option {
                                Value::Option { is_some: true, value } => {
                                    let mut item = *value;

                                    // Apply filter if present
                                    if let Some(filter) = filter_fn {
                                        let args_vec = vec![item.clone()];
                                        let filter_result =
                                            self.call_user_function(filter.as_ref(), &args_vec);
                                        match filter_result {
                                            Value::Bool(false) => {
                                                // Item filtered out, try next iteration of outer loop
                                                continue;
                                            }
                                            _ => {}
                                        }
                                    }

                                    // Apply transformer if present
                                    if let Some(trans) = transformer {
                                        let args_vec = vec![item];
                                        item = self.call_user_function(trans.as_ref(), &args_vec);
                                    }

                                    result.push(item);

                                    // Check take limit after adding
                                    if let Some(limit) = take_count {
                                        if result.len() >= *limit {
                                            return Value::Array(Arc::new(result));
                                        }
                                    }
                                    // Continue to next iteration of outer loop
                                }
                                Value::Option { is_some: false, .. } => {
                                    // Generator exhausted
                                    return Value::Array(Arc::new(result));
                                }
                                Value::Error(msg) => {
                                    return Value::Error(msg);
                                }
                                _ => {
                                    return Value::Error(
                                        "Unexpected value from generator".to_string(),
                                    );
                                }
                            }
                        }
                        Value::Iterator { .. } => {
                            // Materialize nested iterator state once, then continue processing.
                            let materialized = self.collect_iterator((**source).clone());
                            match materialized {
                                Value::Array(items) => {
                                    *source = Box::new(Value::Array(items));
                                    *index = 0;
                                    continue;
                                }
                                Value::Error(msg) => return Value::Error(msg),
                                _ => {
                                    return Value::Error(
                                        "Invalid nested iterator source".to_string(),
                                    );
                                }
                            }
                        }
                        _ => {
                            return Value::Error("Invalid iterator source".to_string());
                        }
                    }
                }
                _ => {
                    return Value::Error("collect() can only be called on iterators".to_string());
                }
            }
        }
    }

    /// Execute a generator until it yields a value or completes
    /// Returns Some(value) if yielded, None if exhausted
    fn generator_next(&mut self, generator: &mut Value) -> Value {
        match generator {
            Value::Generator { params: _, body, env, pc, is_exhausted } => {
                if *is_exhausted {
                    return Value::Option { is_some: false, value: Box::new(Value::Null) };
                }

                // Save current interpreter state
                let saved_env = self.env.clone();
                let saved_return_value = self.return_value.take();

                // Use the generator's environment
                self.env = env.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone();

                let stmts = body.get();
                let mut yielded_value = None;

                // Execute statements starting from PC until yield or end
                while *pc < stmts.len() {
                    let current_pc = *pc;

                    self.eval_stmt(&stmts[current_pc]);

                    // Check if a yield occurred (signaled by Return value)
                    if let Some(ret_val) = &self.return_value {
                        match ret_val {
                            Value::Return(inner) => {
                                // This is a yield - extract the value and suspend
                                // Advance PC so next call continues from next statement
                                *pc += 1;
                                yielded_value = Some(inner.as_ref().clone());
                                self.return_value = None;
                                break;
                            }
                            _ => {
                                // Regular return - generator is done
                                *is_exhausted = true;
                                break;
                            }
                        }
                    } else {
                        // Statement completed without yield - advance to next statement
                        *pc += 1;
                    }
                }

                // Save the generator's environment state
                *env.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = self.env.clone();

                // If we finished all statements without explicit return/yield, generator is exhausted
                if *pc >= stmts.len() {
                    *is_exhausted = true;
                }

                // Restore interpreter state
                self.env = saved_env;
                self.return_value = saved_return_value;

                // Return the yielded value or None if exhausted
                if let Some(value) = yielded_value {
                    Value::Option { is_some: true, value: Box::new(value) }
                } else {
                    Value::Option { is_some: false, value: Box::new(Value::Null) }
                }
            }
            _ => Value::Error("generator_next() can only be called on generators".to_string()),
        }
    }

    /// Get the next value from an iterator
    fn iterator_next(&mut self, mut iterator: Value) -> Value {
        match &mut iterator {
            Value::Iterator { source, index, transformer, filter_fn, take_count } => {
                loop {
                    // Check if we've reached the take limit
                    if let Some(limit) = take_count {
                        if *index >= *limit {
                            return Value::Option { is_some: false, value: Box::new(Value::Null) };
                        }
                    }

                    // Get next item from source
                    match source.as_mut() {
                        Value::Array(items) => {
                            // Find next item that passes filter
                            while *index < items.len() {
                                let mut item = items[*index].clone();
                                *index += 1;

                                // Apply filter if present
                                if let Some(filter) = filter_fn {
                                    let args_vec = vec![item.clone()];
                                    let filter_result =
                                        self.call_user_function(filter.as_ref(), &args_vec);
                                    match filter_result {
                                        Value::Bool(true) => {}
                                        _ => continue,
                                    }
                                }

                                // Apply transformer if present
                                if let Some(trans) = transformer {
                                    let args_vec = vec![item];
                                    item = self.call_user_function(trans.as_ref(), &args_vec);
                                }

                                return Value::Option { is_some: true, value: Box::new(item) };
                            }
                            // No more items
                            return Value::Option { is_some: false, value: Box::new(Value::Null) };
                        }
                        Value::Generator { .. } => {
                            // Delegate to generator_next
                            let result = self.generator_next(source);

                            // Apply transformer if present and we got a value
                            match result {
                                Value::Option { is_some: true, value } => {
                                    let mut item = *value;

                                    // Apply filter if present
                                    if let Some(filter) = filter_fn {
                                        let args_vec = vec![item.clone()];
                                        let filter_result =
                                            self.call_user_function(filter.as_ref(), &args_vec);
                                        match filter_result {
                                            Value::Bool(false) => {
                                                // Filtered generator items should continue to the next yield.
                                                continue;
                                            }
                                            _ => {}
                                        }
                                    }

                                    // Apply transformer if present
                                    if let Some(trans) = transformer {
                                        let args_vec = vec![item];
                                        item = self.call_user_function(trans.as_ref(), &args_vec);
                                    }

                                    *index += 1;
                                    return Value::Option { is_some: true, value: Box::new(item) };
                                }
                                Value::Option { is_some: false, .. } => {
                                    return Value::Option {
                                        is_some: false,
                                        value: Box::new(Value::Null),
                                    };
                                }
                                other => return other,
                            }
                        }
                        Value::Iterator { .. } => {
                            // Materialize nested iterator state once, then continue processing.
                            let materialized = self.collect_iterator((**source).clone());
                            match materialized {
                                Value::Array(items) => {
                                    *source = Box::new(Value::Array(items));
                                    *index = 0;
                                    continue;
                                }
                                Value::Error(msg) => return Value::Error(msg),
                                _ => {
                                    return Value::Error(
                                        "Invalid nested iterator source".to_string(),
                                    )
                                }
                            }
                        }
                        _ => return Value::Error("Invalid iterator source".to_string()),
                    }
                }
            }
            _ => Value::Error("next() can only be called on iterators".to_string()),
        }
    }

    /// Converts a runtime value to a string for display
    fn stringify_value(value: &Value) -> String {
        match value {
            Value::Str(s) => s.as_ref().clone(),
            Value::Secret(_) => "***".to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Tagged { tag, fields } => {
                if fields.is_empty() {
                    tag.clone()
                } else {
                    let args: Vec<String> =
                        fields.values().map(Interpreter::stringify_value).collect();
                    format!("{}({})", tag, args.join(","))
                }
            }
            Value::Struct { name, fields } => {
                let mut keys: Vec<&String> = fields.keys().collect();
                keys.sort();
                let field_strs: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        format!("{}: {}", k, Interpreter::stringify_value(fields.get(*k).unwrap()))
                    })
                    .collect();
                format!("{} {{ {} }}", name, field_strs.join(", "))
            }
            Value::Array(elements) => {
                let elem_strs: Vec<String> =
                    elements.iter().map(Interpreter::stringify_value).collect();
                format!("[{}]", elem_strs.join(", "))
            }
            Value::Dict(map) => {
                let mut keys: Vec<&Arc<str>> = map.keys().collect();
                keys.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
                let pair_strs: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        format!(
                            "\"{}\": {}",
                            k,
                            Interpreter::stringify_value(map.get(k.as_ref()).unwrap())
                        )
                    })
                    .collect();
                format!("{{{}}}", pair_strs.join(", "))
            }
            Value::FixedDict { keys, values } => {
                let mut pairs: Vec<(&Arc<str>, &Value)> = keys.iter().zip(values.iter()).collect();
                pairs.sort_by(|(a, _), (b, _)| a.as_ref().cmp(b.as_ref()));
                let pair_strs: Vec<String> = pairs
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, Interpreter::stringify_value(v)))
                    .collect();
                format!("{{{}}}", pair_strs.join(", "))
            }
            Value::IntDict(dict) => {
                let mut keys: Vec<i64> = dict.keys().copied().collect();
                keys.sort();
                let pair_strs: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        format!("\"{}\": {}", k, Interpreter::stringify_value(dict.get(k).unwrap()))
                    })
                    .collect();
                format!("{{{}}}", pair_strs.join(", "))
            }
            Value::DenseIntDict(values) => {
                let pair_strs: Vec<String> = values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        format!("\"{}\": {}", index, Interpreter::stringify_value(value))
                    })
                    .collect();
                format!("{{{}}}", pair_strs.join(", "))
            }
            Value::DenseIntDictInt(values) => {
                let pair_strs: Vec<String> = values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| match value {
                        Some(value) => format!("\"{}\": {}", index, value),
                        None => format!("\"{}\": null", index),
                    })
                    .collect();
                format!("{{{}}}", pair_strs.join(", "))
            }
            Value::DenseIntDictIntFull(values) => {
                let pair_strs: Vec<String> = values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| format!("\"{}\": {}", index, value))
                    .collect();
                format!("{{{}}}", pair_strs.join(", "))
            }
            Value::Return(inner) => Interpreter::stringify_value(inner),
            Value::Error(msg) => format!("Error: {}", msg),
            Value::ErrorObject { message, .. } => format!("Error: {}", message),
            Value::NativeFunction(name) => format!("<native function: {}>", name),
            Value::Result { is_ok, value } => {
                if *is_ok {
                    format!("Ok({})", Interpreter::stringify_value(value))
                } else {
                    format!("Err({})", Interpreter::stringify_value(value))
                }
            }
            Value::Option { is_some, value } => {
                if *is_some {
                    format!("Some({})", Interpreter::stringify_value(value))
                } else {
                    "None".to_string()
                }
            }
            Value::GeneratorDef(params, _) => {
                format!("<generator function with {} params>", params.len())
            }
            Value::Generator { params, is_exhausted, .. } => {
                if *is_exhausted {
                    format!("<exhausted generator ({} params)>", params.len())
                } else {
                    format!("<generator ({} params)>", params.len())
                }
            }
            Value::Iterator { .. } => "<iterator>".to_string(),
            _ => "<unknown>".into(),
        }
    }

    /// Cleanup method to rollback any active transactions before interpreter is dropped
    /// This prevents hanging when SQLite connections are dropped while in transaction
    pub fn cleanup(&mut self) {
        #[cfg(feature = "runtime-db")]
        {
            // Get all variables from the environment
            let var_names: Vec<String> =
                self.env.scopes.iter().flat_map(|scope| scope.keys().cloned()).collect();

            for var_name in var_names {
                if let Some(Value::Database { connection, db_type, in_transaction, .. }) =
                    self.env.get(&var_name)
                {
                    // Check if in transaction
                    let is_in_trans = {
                        match in_transaction.lock() {
                            Ok(in_trans) => *in_trans,
                            Err(_) => false,
                        }
                    };

                    if is_in_trans {
                        // Rollback the transaction
                        match (connection, db_type.as_str()) {
                            (DatabaseConnection::Sqlite(conn_arc), "sqlite") => {
                                if let Ok(conn) = conn_arc.lock() {
                                    let _ = conn.execute("ROLLBACK", []);
                                }
                            }
                            (DatabaseConnection::Postgres(client_arc), "postgres") => {
                                if let Ok(mut client) = client_arc.lock() {
                                    let _ = client.execute("ROLLBACK", &[]);
                                }
                            }
                            (DatabaseConnection::Mysql(conn_arc), "mysql") => {
                                if let Ok(mut conn) = conn_arc.lock() {
                                    if let Ok(runtime) = tokio::runtime::Runtime::new() {
                                        let _ = runtime.block_on(async {
                                            conn.exec_drop("ROLLBACK", mysql_async::Params::Empty)
                                                .await
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }

                        // Update transaction flag
                        if let Ok(mut in_trans) = in_transaction.lock() {
                            *in_trans = false;
                        }
                    }
                }
            }
        }
    }
}
