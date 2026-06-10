// File: src/runtime_limits.rs
//
// Centralized default resource limits for parser/runtime/native operations.

pub const DEFAULT_MAX_SOURCE_BYTES: usize = 1_048_576;
pub const DEFAULT_MAX_STRING_LITERAL_LENGTH: usize = 8_192;
pub const DEFAULT_MAX_COLLECTION_LITERAL_ITEMS: usize = 4_096;

pub const DEFAULT_MAX_EXPRESSION_DEPTH: usize = 256;
pub const DEFAULT_MAX_BLOCK_DEPTH: usize = 128;
pub const DEFAULT_MAX_VALUE_NESTING_DEPTH: usize = 4096;

pub const DEFAULT_MAX_INTERPRETER_CALL_DEPTH: usize = 32;
pub const DEFAULT_MAX_VM_CALL_DEPTH: usize = 256;

pub const MAX_FILE_IO_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_NETWORK_BODY_BYTES: usize = 8 * 1024 * 1024;
