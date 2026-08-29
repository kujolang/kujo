// File: src/lib.rs
//
// Library interface for the Kujo interpreter.
// Exposes modules for integration testing and external use.
#![allow(clippy::all)]

pub mod agent_project;
pub mod ast;
pub mod benchmarks;
pub mod builtins;
pub mod bytecode;
pub mod cli_output;
pub mod compiler;
pub mod doc_generator;
pub mod docgen;
pub mod errors;
pub mod formatter;
pub mod http_request_utils;
pub mod interpreter;
#[cfg(feature = "runtime-jit")]
pub mod jit;
#[cfg(not(feature = "runtime-jit"))]
#[path = "jit_disabled.rs"]
pub mod jit;
pub mod lexer;
pub mod linter;
pub mod lsp_code_actions;
pub mod lsp_completion;
pub mod lsp_definition;
pub mod lsp_diagnostics;
pub mod lsp_hover;
pub mod lsp_references;
pub mod lsp_rename;
pub mod lsp_server;
pub mod module;
pub mod network_policy;
pub mod optimizer;
pub mod package_workflow;
pub mod parser;
pub mod path_security;
pub mod repl;
pub mod reserved_names;
pub mod runtime_limits;
pub mod serve_http;
pub mod type_checker;
pub mod vm;
pub mod workflow_pack;
