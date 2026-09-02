// File: src/main.rs
//
// Main entry point for the Kujo programming language interpreter.
// Handles command-line argument parsing and dispatches to the appropriate
// subcommand (run, repl, or test).
#![allow(clippy::all)]

mod agent_project;
mod ast;
mod benchmarks;
mod builtins;
mod bytecode;
mod cli_output;
mod compiler;
mod doc_generator;
mod docgen;
mod ecosystem_golden_path;
mod errors;
mod formatter;
mod http_request_utils;
mod interpreter;
#[cfg(feature = "runtime-jit")]
mod jit;
#[cfg(not(feature = "runtime-jit"))]
#[path = "jit_disabled.rs"]
mod jit;
mod lexer;
mod linter;
mod lsp_code_actions;
mod lsp_completion;
mod lsp_definition;
mod lsp_diagnostics;
mod lsp_hover;
mod lsp_references;
mod lsp_rename;
mod lsp_server;
mod module;
mod network_policy;
mod optimizer;
mod package_workflow;
mod parser;
mod path_security;
mod repl;
mod reserved_names;
mod runtime_limits;
mod serve_http;
mod type_checker;
mod vm;

mod workflow_pack;

use crate::interpreter::RuntimeCapabilityPolicy;
use clap::{Args, Parser as ClapParser, Subcommand, ValueEnum};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::runtime::Builder;

const VM_EXECUTION_STACK_SIZE: usize = 64 * 1024 * 1024;

#[derive(ClapParser)]
#[command(
    name = "kujo",
    about = "Kujo: A modern programming language",
    version = env!("CARGO_PKG_VERSION"),
    long_about = None,
    allow_external_subcommands = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Args, Clone, Debug, Default)]
struct CapabilityArgs {
    /// Restrict host-effect native APIs by default; opt in with --allow-* flags.
    #[arg(long, default_value_t = false)]
    untrusted: bool,

    /// Allow all host-effect capabilities (trusted mode).
    #[arg(long, default_value_t = false)]
    allow_all: bool,

    /// Allow filesystem reads and metadata probes.
    #[arg(long, default_value_t = false)]
    allow_fs_read: bool,

    /// Allow filesystem writes and in-place file mutations.
    #[arg(long, default_value_t = false)]
    allow_fs_write: bool,

    /// Allow filesystem delete operations.
    #[arg(long, default_value_t = false)]
    allow_fs_delete: bool,

    /// Allow process execution APIs (non-shell argv execution).
    #[arg(long, default_value_t = false)]
    allow_process_exec: bool,

    /// Allow shell command execution APIs.
    #[arg(long, default_value_t = false)]
    allow_shell_exec: bool,

    /// Allow environment variable reads.
    #[arg(long, default_value_t = false)]
    allow_env_read: bool,

    /// Allow environment variable writes.
    #[arg(long, default_value_t = false)]
    allow_env_write: bool,

    /// Allow outbound network client APIs (HTTP/TCP/UDP client calls).
    #[arg(long, default_value_t = false)]
    allow_net_client: bool,

    /// Allow network listener/server APIs.
    #[arg(long, default_value_t = false)]
    allow_net_server: bool,

    /// Convenience alias to enable both --allow-net-client and --allow-net-server.
    #[arg(long, default_value_t = false)]
    allow_net: bool,

    /// Allow high-level AI provider helper APIs (ai_*).
    #[arg(long, default_value_t = false)]
    allow_ai: bool,

    /// Allow database connection and query APIs.
    #[arg(long, default_value_t = false)]
    allow_database: bool,

    /// Allow wall-clock/time APIs.
    #[arg(long, default_value_t = false)]
    allow_clock: bool,

    /// Allow random-number generation APIs.
    #[arg(long, default_value_t = false)]
    allow_random: bool,

    /// Force outbound HTTP/TCP/UDP clients to reject private, local, link-local, multicast, and unspecified destinations.
    #[arg(long, default_value_t = false)]
    deny_private_net: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum TestRuntimeMode {
    /// Execute test fixtures via `kujo run --interpreter`.
    Interpreter,
    /// Execute test fixtures via the default VM runtime.
    Vm,
    /// Run VM first and fall back to interpreter only when VM output does not match snapshot.
    Dual,
}

#[derive(Subcommand)]
enum Commands {
    /// Create, inspect, run, and evaluate repository-owned Kujo agents
    Agent {
        #[command(subcommand)]
        command: agent_project::AgentCommands,
    },
    /// Run a Kujo script file
    Run {
        /// Path to the .kujo file
        file: PathBuf,

        /// Use tree-walking interpreter instead of bytecode VM (default: VM)
        #[arg(long)]
        interpreter: bool,

        /// Opt in to experimental JIT compilation for JIT-compatible bytecode surfaces.
        #[arg(long, default_value_t = false)]
        jit: bool,

        /// Cooperative scheduler timeout in milliseconds (overrides env/default)
        #[arg(long)]
        scheduler_timeout_ms: Option<u64>,

        /// Emit runtime failures for `kujo run` as machine-readable JSON on stdout.
        #[arg(long, default_value_t = false)]
        json_runtime_diagnostics: bool,

        #[command(flatten)]
        capabilities: CapabilityArgs,

        /// Arguments to pass to the script
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        script_args: Vec<String>,
    },

    /// Validate Kujo source (lex/parse/compile) without executing the program
    Check {
        /// Path to the .kujo file
        file: PathBuf,

        /// Suppress success output (failures still print diagnostics)
        #[arg(short, long, default_value_t = false, conflicts_with = "verbose")]
        quiet: bool,

        /// Print expanded success summary
        #[arg(short, long, default_value_t = false, conflicts_with = "quiet")]
        verbose: bool,

        /// Print validation result as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Serve a directory over HTTP for local preview/testing
    Serve {
        /// Directory to serve (default: current directory)
        #[arg(default_value = ".")]
        dir: PathBuf,

        /// Port to bind
        #[arg(long, default_value_t = 8080)]
        port: u16,

        /// Host/interface to bind
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Default index file when requesting '/'
        #[arg(long, default_value = "index.html")]
        index: String,

        /// Enable hardened defaults (security headers + strict static behavior)
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        hardened: bool,

        /// Cache-Control max-age in seconds for successful file responses
        #[arg(long, default_value_t = 300)]
        cache_max_age: u32,

        /// Enable concise access logs for each request
        #[arg(long, default_value_t = false)]
        access_log: bool,

        /// TLS certificate path (PEM). Requires --tls-key.
        #[arg(long)]
        tls_cert: Option<PathBuf>,

        /// TLS private key path (PEM). Requires --tls-cert.
        #[arg(long)]
        tls_key: Option<PathBuf>,

        /// Maximum HTTP request line size in bytes.
        #[arg(long, default_value_t = 8192)]
        max_request_line_bytes: usize,

        /// Maximum combined HTTP header bytes.
        #[arg(long, default_value_t = 16384)]
        max_header_bytes: usize,

        /// Maximum HTTP header count.
        #[arg(long, default_value_t = 100)]
        max_header_count: usize,

        /// Maximum request body bytes accepted by the static server.
        #[arg(long, default_value_t = 1048576)]
        max_request_body_bytes: usize,

        /// Per-connection read timeout in milliseconds.
        #[arg(long, default_value_t = 5000)]
        read_timeout_ms: u64,

        /// Per-connection write timeout in milliseconds.
        #[arg(long, default_value_t = 5000)]
        write_timeout_ms: u64,

        /// Maximum number of concurrent request handlers.
        #[arg(long, default_value_t = 128)]
        max_connections: usize,
    },

    /// Launch interactive Kujo REPL
    Repl,

    /// Run all test scripts in the tests/ directory
    Test {
        /// Regenerate all .out files based on actual output
        #[arg(long)]
        update: bool,

        /// Runtime path strategy for fixture execution.
        #[arg(long, value_enum, default_value_t = TestRuntimeMode::Dual)]
        runtime: TestRuntimeMode,

        /// Print additional fixture-level diagnostics.
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },

    /// Run tests defined with the test framework
    TestRun {
        /// Path to the .kujo file containing tests
        file: PathBuf,

        /// Print detailed output for each test
        #[arg(short, long)]
        verbose: bool,

        #[command(flatten)]
        capabilities: CapabilityArgs,
    },

    /// Run performance benchmarks
    Bench {
        /// Path to benchmark file or directory
        path: Option<PathBuf>,

        /// Number of iterations per benchmark (default: 10)
        #[arg(short, long, default_value_t = 10)]
        iterations: usize,

        /// Number of warmup runs (default: 2)
        #[arg(short, long, default_value_t = 2)]
        warmup: usize,
    },

    /// Format a Kujo source file
    Format {
        /// Path to the .kujo file
        file: PathBuf,

        /// Indentation width (spaces)
        #[arg(long, default_value_t = 4)]
        indent: usize,

        /// Maximum preferred line length
        #[arg(long, default_value_t = 100)]
        line_length: usize,

        /// Disable import sorting
        #[arg(long, default_value_t = false)]
        no_sort_imports: bool,

        /// Check if formatting changes are required
        #[arg(long, default_value_t = false)]
        check: bool,

        /// Write formatted output back to file
        #[arg(long, default_value_t = false)]
        write: bool,

        /// Print formatter result as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Lint a Kujo source file
    Lint {
        /// Path to the .kujo file
        file: PathBuf,

        /// Apply safe autofixes
        #[arg(long, default_value_t = false)]
        fix: bool,

        /// Print lint issues as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Initialize a Kujo project with kujo.toml and src/main.kujo
    Init {
        /// Project directory (defaults to current directory)
        #[arg(long)]
        dir: Option<PathBuf>,

        /// Package name override (defaults to directory name)
        #[arg(long)]
        name: Option<String>,
    },

    /// Add a dependency to kujo.toml
    PackageAdd {
        /// Dependency name
        name: String,

        /// Dependency version requirement
        #[arg(long, default_value = "*")]
        version: String,

        /// Path to kujo.toml (defaults to ./kujo.toml)
        #[arg(long)]
        manifest: Option<PathBuf>,
    },

    /// Validate dependencies declared in kujo.toml
    PackageInstall {
        /// Path to kujo.toml (defaults to ./kujo.toml)
        #[arg(long)]
        manifest: Option<PathBuf>,

        /// Path to kujo.lock (defaults to sibling lockfile next to manifest)
        #[arg(long)]
        lockfile: Option<PathBuf>,

        /// Verify lockfile is up to date without writing changes
        #[arg(long, default_value_t = false)]
        frozen: bool,
    },

    /// Preview package publish metadata from kujo.toml
    PackagePublish {
        /// Path to kujo.toml (defaults to ./kujo.toml)
        #[arg(long)]
        manifest: Option<PathBuf>,

        /// Reserved for future registry transport; currently rejected
        #[arg(long, default_value_t = false)]
        publish: bool,
    },

    /// Execute workflow-pack commands through the canonical pack-local path
    Pack {
        #[command(subcommand)]
        command: PackCommands,
    },

    /// Run the first-party Kujo Doctor workflow extension
    Doctor {
        /// Render Doctor output as JSON
        #[arg(long, default_value_t = false)]
        json: bool,

        /// Enable deeper project checks
        #[arg(long, default_value_t = false)]
        deep: bool,

        /// List available doctor profiles
        #[arg(long, default_value_t = false)]
        list_profiles: bool,

        /// Explicit doctor profile name
        #[arg(long)]
        profile: Option<String>,

        /// Positional doctor profile name (`kujo doctor <profile>`)
        profile_name: Option<String>,
    },

    /// Run fixture-first evidence workflows across the Kujo ecosystem
    Ecosystem {
        #[command(subcommand)]
        command: EcosystemCommands,
    },

    /// Generate universal documentation from source code
    Docgen {
        /// Path to a file or directory to document
        path: PathBuf,

        /// Output directory for generated docs (defaults to docs/generated)
        #[arg(long)]
        out_dir: Option<PathBuf>,

        /// Output format: html, markdown, json, or all
        #[arg(long)]
        format: Option<String>,

        /// Restrict processing to a single language adapter
        #[arg(long)]
        language: Option<String>,

        /// Comma-separated language adapter list
        #[arg(long)]
        languages: Option<String>,

        /// Disable builtin/native API reference generation
        #[arg(long, default_value_t = false)]
        no_builtins: bool,

        /// Emit AI task files for documentation gaps
        #[arg(long, default_value_t = false)]
        emit_ai_tasks: bool,

        /// Generate search and symbol index JSON outputs
        #[arg(long, default_value_t = false)]
        search_index: bool,

        /// Include source links in rendered docs
        #[arg(long, default_value_t = false)]
        source_links: bool,

        /// Source-link URL template (supports {path} and {line})
        #[arg(long)]
        source_link_template: Option<String>,

        /// Include only public/exported symbols
        #[arg(long, default_value_t = false)]
        public_only: bool,

        /// Explicitly include private/internal symbols
        #[arg(long, default_value_t = false)]
        include_private: bool,

        /// Fail when undocumented symbols are detected
        #[arg(long, default_value_t = false)]
        fail_on_undocumented: bool,

        /// Fail when broken links are detected in docs
        #[arg(long, default_value_t = false)]
        fail_on_broken_links: bool,

        /// Validate local markdown/html anchors for local links
        #[arg(long, default_value_t = false)]
        validate_local_anchors: bool,

        /// Validate external http/https links for allowlisted hosts
        #[arg(long, default_value_t = false)]
        validate_external_links: bool,

        /// External-link validation timeout in milliseconds
        #[arg(long, default_value_t = 1500)]
        external_link_timeout_ms: u64,

        /// Comma-separated host allowlist for external-link validation
        #[arg(long)]
        external_link_allowlist: Option<String>,

        /// Allow validating external links that resolve to private/loopback/link-local/multicast IPs
        #[arg(long, default_value_t = false)]
        allow_private_network_links: bool,

        /// Maximum total local/external links to validate before truncating checks
        #[arg(long)]
        max_link_checks: Option<usize>,

        /// Maximum external links to validate before truncating external checks
        #[arg(long)]
        max_external_link_checks: Option<usize>,

        /// Maximum total wall-clock time budget (ms) for link validation before truncating checks
        #[arg(long)]
        max_total_validation_time_ms: Option<u64>,

        /// Fail when any warnings are emitted
        #[arg(long, default_value_t = false)]
        fail_on_warnings: bool,

        /// Max file size (bytes) for discovery before files are skipped
        #[arg(long)]
        max_discovery_file_size_bytes: Option<u64>,

        /// Max number of files discovery will scan
        #[arg(long)]
        max_discovery_files: Option<usize>,

        /// Max directory traversal depth for discovery
        #[arg(long)]
        max_discovery_depth: Option<usize>,

        /// Optional cache directory for incremental docgen extraction reuse
        #[arg(long)]
        cache_dir: Option<PathBuf>,

        /// Opt in to Kujo parser-assisted symbol extraction with regex fallback on diagnostics
        #[arg(long, default_value_t = false)]
        kujo_parser_assisted: bool,

        /// Print documentation generation result as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Compare Kujo parallel_map benchmark against Python ProcessPoolExecutor
    BenchCross {
        /// Path to Kujo benchmark script
        #[arg(long, default_value = "benchmarks/cross-language/bench_parallel_map.kujo")]
        kujo_script: PathBuf,

        /// Path to Python ProcessPool benchmark script
        #[arg(long, default_value = "benchmarks/cross-language/bench_process_pool.py")]
        python_script: PathBuf,

        /// Python executable to use
        #[arg(long, default_value = "python3")]
        python: String,
    },

    /// Run the async SSG benchmark and optionally compare with Python
    BenchSsg {
        /// Path to Kujo SSG benchmark script
        #[arg(long, default_value = "benchmarks/cross-language/bench_ssg.kujo")]
        kujo_script: PathBuf,

        /// Number of warmup runs excluded from measured summary
        #[arg(long, default_value_t = 0)]
        warmup_runs: usize,

        /// Number of repeated benchmark runs for noise reduction (median reporting)
        #[arg(long, default_value_t = 1)]
        runs: usize,

        /// Optional Kujo median build-time gate in milliseconds (fails command on miss)
        #[arg(long)]
        throughput_gate_ms: Option<f64>,

        /// Print per-stage timing breakdown and bottleneck summary when available
        #[arg(long, default_value_t = false)]
        profile_async: bool,

        /// Compare against the Python baseline benchmark
        #[arg(long, default_value_t = false)]
        compare_python: bool,

        /// Path to Python SSG benchmark script
        #[arg(long, default_value = "benchmarks/cross-language/bench_ssg.py")]
        python_script: PathBuf,

        /// Python executable to use for comparison
        #[arg(long, default_value = "python3")]
        python: String,

        /// Optional temp root for benchmark artifacts (overrides workspace tmp/)
        #[arg(long)]
        tmp_dir: Option<PathBuf>,

        /// CV threshold (%) for measurement variability warnings
        #[arg(long, default_value_t = benchmarks::ssg::SSG_VARIABILITY_WARNING_THRESHOLD_PERCENT)]
        variability_warning_threshold: f64,

        /// Percent threshold for first-to-last trend drift warnings
        #[arg(long, default_value_t = benchmarks::ssg::SSG_TREND_WARNING_THRESHOLD_PERCENT)]
        trend_warning_threshold: f64,

        /// Percent threshold for mean-vs-median drift warnings
        #[arg(long, default_value_t = benchmarks::ssg::SSG_MEAN_MEDIAN_DRIFT_WARNING_THRESHOLD_PERCENT)]
        mean_median_drift_warning_threshold: f64,

        /// Percent threshold for min/max range-spread warnings relative to median
        #[arg(long, default_value_t = benchmarks::ssg::SSG_RANGE_SPREAD_WARNING_THRESHOLD_PERCENT)]
        range_spread_warning_threshold: f64,
    },

    /// Profile a Kujo script (CPU, memory, JIT stats)
    Profile {
        /// Path to the .kujo file
        file: PathBuf,

        /// Enable CPU profiling
        #[arg(long, default_value_t = true)]
        cpu: bool,

        /// Enable memory profiling
        #[arg(long, default_value_t = true)]
        memory: bool,

        /// Enable JIT statistics
        #[arg(long, default_value_t = true)]
        jit: bool,

        /// Generate flamegraph output file
        #[arg(long)]
        flamegraph: Option<PathBuf>,
    },

    /// Return completion candidates for LSP/autocomplete integration
    LspComplete {
        /// Path to the .kujo file
        file: PathBuf,

        /// 1-based line number for the completion request
        #[arg(long)]
        line: usize,

        /// 1-based column number for the completion request
        #[arg(long)]
        column: usize,

        /// Print completion items as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Return go-to-definition location for an identifier under cursor
    LspDefinition {
        /// Path to the .kujo file
        file: PathBuf,

        /// 1-based line number for the definition request
        #[arg(long)]
        line: usize,

        /// 1-based column number for the definition request
        #[arg(long)]
        column: usize,

        /// Print definition result as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Return all references for the identifier under cursor
    LspReferences {
        /// Path to the .kujo file
        file: PathBuf,

        /// 1-based line number for the references request
        #[arg(long)]
        line: usize,

        /// 1-based column number for the references request
        #[arg(long)]
        column: usize,

        /// Include definition location in results
        #[arg(long, default_value_t = true)]
        include_definition: bool,

        /// Print references as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Return hover information for the identifier under cursor
    LspHover {
        /// Path to the .kujo file
        file: PathBuf,

        /// 1-based line number for the hover request
        #[arg(long)]
        line: usize,

        /// 1-based column number for the hover request
        #[arg(long)]
        column: usize,

        /// Print hover info as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Return diagnostics for source code at editor refresh time
    LspDiagnostics {
        /// Path to the .kujo file
        file: PathBuf,

        /// Print diagnostics as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Rename the symbol under cursor and return edits
    LspRename {
        /// Path to the .kujo file
        file: PathBuf,

        /// 1-based line number for the rename request
        #[arg(long)]
        line: usize,

        /// 1-based column number for the rename request
        #[arg(long)]
        column: usize,

        /// New symbol name
        #[arg(long)]
        new_name: String,

        /// Print rename result as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Return quick-fix code actions based on diagnostics
    LspCodeActions {
        /// Path to the .kujo file
        file: PathBuf,

        /// Print code actions as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Launch Kujo as a long-running Language Server Protocol server (JSON-RPC over stdio)
    Lsp {
        /// Emit deterministic request/response logs to stderr for debugging
        #[arg(long, default_value_t = false)]
        deterministic_logs: bool,

        /// Timeout budget in milliseconds for a single LSP request
        #[arg(long, default_value_t = 5000)]
        request_timeout_ms: u64,
    },
}

#[derive(Subcommand)]
enum PackCommands {
    /// Run a pack-local command: `kujo pack run <namespace> <command>`
    Run {
        /// Workflow pack namespace
        namespace: String,

        /// Command name inside the namespace
        command: String,

        /// Render command output as JSON
        #[arg(long, default_value_t = false)]
        json: bool,

        /// Save doctor output into the pack .reports directory when supported
        #[arg(long, default_value_t = false)]
        save: bool,

        /// Additional command arguments passed to the pack command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command_args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum EcosystemCommands {
    /// Execute the isolated Kujo ecosystem golden path
    GoldenPath {
        /// Repository collection root containing ai-sdk, agents-sdk, dispatch, and sibling tools
        #[arg(long, default_value = ".")]
        repo_root: PathBuf,

        /// New or empty directory receiving stage results and hashed artifacts
        #[arg(long)]
        output_root: Option<PathBuf>,

        /// Maximum wall-clock time per stage
        #[arg(long, default_value_t = 15)]
        timeout_seconds: u64,

        /// Allow explicit blocked stages while still failing on execution errors
        #[arg(long, default_value_t = false)]
        allow_blocked: bool,

        /// Emit the complete machine-readable report to stdout
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}

const DEFAULT_COOPERATIVE_SCHEDULER_TIMEOUT_MS: u64 = 120_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
enum CliExitCode {
    UsageError = 2,
    LexParseError = 3,
    RuntimeError = 4,
    IoError = 5,
    InternalError = 6,
}

impl CliExitCode {
    fn code(self) -> i32 {
        self as i32
    }
}

fn cooperative_scheduler_timeout(
    cli_timeout_ms: Option<u64>,
) -> Result<std::time::Duration, String> {
    if let Some(timeout_ms) = cli_timeout_ms {
        if timeout_ms == 0 {
            return Err("Scheduler timeout must be greater than 0ms".to_string());
        }

        return Ok(std::time::Duration::from_millis(timeout_ms));
    }

    match std::env::var("KUJO_SCHEDULER_TIMEOUT_MS") {
        Ok(raw_timeout_ms) => match raw_timeout_ms.parse::<u64>() {
            Ok(timeout_ms) if timeout_ms > 0 => Ok(std::time::Duration::from_millis(timeout_ms)),
            _ => Ok(std::time::Duration::from_millis(DEFAULT_COOPERATIVE_SCHEDULER_TIMEOUT_MS)),
        },
        Err(_) => Ok(std::time::Duration::from_millis(DEFAULT_COOPERATIVE_SCHEDULER_TIMEOUT_MS)),
    }
}

fn build_runtime_capability_policy(args: &CapabilityArgs) -> RuntimeCapabilityPolicy {
    if args.allow_all {
        return RuntimeCapabilityPolicy::trusted();
    }

    let has_explicit_allows = args.allow_fs_read
        || args.allow_fs_write
        || args.allow_fs_delete
        || args.allow_process_exec
        || args.allow_shell_exec
        || args.allow_env_read
        || args.allow_env_write
        || args.allow_net_client
        || args.allow_net_server
        || args.allow_net
        || args.allow_ai
        || args.allow_database
        || args.allow_clock
        || args.allow_random;

    if !args.untrusted && !has_explicit_allows {
        return RuntimeCapabilityPolicy::trusted();
    }

    let mut policy = RuntimeCapabilityPolicy::restricted();
    policy.filesystem_read = args.allow_fs_read;
    policy.filesystem_write = args.allow_fs_write;
    policy.filesystem_delete = args.allow_fs_delete;
    policy.process_exec = args.allow_process_exec;
    policy.shell_exec = args.allow_shell_exec;
    policy.env_read = args.allow_env_read;
    policy.env_write = args.allow_env_write;
    policy.network_client = args.allow_net || args.allow_net_client;
    policy.network_ai = args.allow_ai;
    policy.network_server = args.allow_net || args.allow_net_server;
    policy.database = args.allow_database;
    policy.clock = args.allow_clock;
    policy.random = args.allow_random;
    policy
}

fn apply_untrusted_network_destination_policy_defaults(args: &CapabilityArgs) {
    if args.deny_private_net {
        std::env::set_var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV, "deny_private");
        return;
    }

    if args.allow_all || !args.untrusted {
        return;
    }

    let network_client_enabled = args.allow_net || args.allow_net_client || args.allow_ai;
    if !network_client_enabled {
        return;
    }

    if std::env::var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV).is_err() {
        std::env::set_var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV, "deny_private");
    }
}

fn report_lexer_diagnostics_and_exit(
    _file_label: &str,
    diagnostics: &[lexer::LexerDiagnostic],
) -> ! {
    let converted: Vec<errors::Diagnostic> =
        diagnostics.iter().map(|diagnostic| diagnostic.to_diagnostic()).collect();
    report_diagnostics_and_exit(&converted, CliExitCode::LexParseError);
}

fn report_parser_diagnostics_and_exit(
    file_label: &str,
    diagnostics: &[parser::ParseDiagnostic],
) -> ! {
    let converted: Vec<errors::Diagnostic> =
        diagnostics.iter().map(|diagnostic| diagnostic.to_diagnostic(Some(file_label))).collect();
    report_diagnostics_and_exit(&converted, CliExitCode::LexParseError);
}

fn report_diagnostics_and_exit(diagnostics: &[errors::Diagnostic], code: CliExitCode) -> ! {
    for diagnostic in diagnostics {
        eprintln!("{}", diagnostic.render_human());
    }
    std::process::exit(code.code());
}

fn report_cli_error_and_exit(message: impl Into<String>, code: CliExitCode) -> ! {
    let diagnostic = errors::Diagnostic::new(
        errors::DIAGNOSTIC_CODE_CLI,
        errors::DiagnosticSeverity::Error,
        errors::DiagnosticSubsystem::Cli,
        message.into(),
    );
    report_diagnostics_and_exit(&[diagnostic], code);
}

fn report_json_internal_error_and_exit(error_message: String, code: CliExitCode) -> ! {
    let diagnostic = errors::Diagnostic::new(
        errors::DIAGNOSTIC_CODE_CLI,
        errors::DiagnosticSeverity::Error,
        errors::DiagnosticSubsystem::Cli,
        error_message,
    );
    report_diagnostics_and_exit(&[diagnostic], code);
}

fn emit_json_payload_and_exit(payload: &serde_json::Value, code: CliExitCode) -> ! {
    match cli_output::emit_pretty_json(payload) {
        Ok(()) => std::process::exit(code.code()),
        Err(error) => report_json_internal_error_and_exit(
            format!("Failed to serialize runtime diagnostics JSON: {}", error),
            CliExitCode::InternalError,
        ),
    }
}

fn report_json_command_error_and_exit(
    command: &str,
    kind: &str,
    message: impl Into<String>,
    code: CliExitCode,
) -> ! {
    let payload = serde_json::json!({
        "command": command,
        "status": "error",
        "kind": kind,
        "contract_version": "1.0.0",
        "exit_code": code.code(),
        "message": message.into(),
    });
    emit_json_payload_and_exit(&payload, code);
}

fn emit_json_or_internal_error<T: serde::Serialize>(value: &T, context: &str) {
    if let Err(error) = cli_output::emit_pretty_json(value) {
        report_json_internal_error_and_exit(
            format!("Failed to serialize {}: {}", context, error),
            CliExitCode::InternalError,
        );
    }
}

fn report_run_runtime_diagnostic_and_exit(
    diagnostic: &errors::Diagnostic,
    code: CliExitCode,
    json_runtime_diagnostics: bool,
) -> ! {
    if json_runtime_diagnostics {
        let payload = errors::run_runtime_diagnostic_envelope_json(diagnostic, code.code());
        emit_json_payload_and_exit(&payload, code);
    }

    report_diagnostics_and_exit(std::slice::from_ref(diagnostic), code);
}

fn report_run_runtime_error_and_exit(
    error: &errors::KujoError,
    code: CliExitCode,
    json_runtime_diagnostics: bool,
) -> ! {
    if json_runtime_diagnostics {
        let diagnostic = error.as_diagnostic();
        let mut payload = errors::run_runtime_diagnostic_envelope_json(&diagnostic, code.code());
        if let Some(object) = payload.as_object_mut() {
            object.insert("runtime_kind".to_string(), serde_json::json!(format!("{}", error.kind)));
            object.insert("call_stack".to_string(), serde_json::json!(error.call_stack));
        }
        emit_json_payload_and_exit(&payload, code);
    }

    eprintln!("{}", error);
    std::process::exit(code.code());
}

fn read_kujo_source_for_parse(file: &Path) -> String {
    let max_source_bytes = parser::DEFAULT_MAX_SOURCE_BYTES;
    let file_label = file.to_string_lossy().to_string();

    if let Ok(metadata) = fs::metadata(file) {
        if metadata.is_file() && metadata.len() > max_source_bytes as u64 {
            let diagnostic =
                parser::source_size_limit_diagnostic(metadata.len() as usize, max_source_bytes);
            report_parser_diagnostics_and_exit(&file_label, &[diagnostic]);
        }
    }

    let code = match fs::read_to_string(file) {
        Ok(content) => content,
        Err(err) => {
            report_cli_error_and_exit(
                format!("Failed to read .kujo file '{}': {}", file.display(), err),
                CliExitCode::IoError,
            );
        }
    };

    if let Err(diagnostic) = parser::validate_source_size(&code, max_source_bytes) {
        report_parser_diagnostics_and_exit(&file_label, &[diagnostic]);
    }

    code
}

fn parse_kujo_program(file: &Path) -> (String, String, Vec<ast::Stmt>) {
    let code = read_kujo_source_for_parse(file);
    let filename = file.to_string_lossy().to_string();
    let tokens = match lexer::tokenize_with_file(&code, Some(&filename)) {
        Ok(tokens) => tokens,
        Err(diagnostics) => report_lexer_diagnostics_and_exit(&filename, &diagnostics),
    };
    let mut parser = parser::Parser::new(tokens);
    let parse_output = parser.parse_with_diagnostics();
    if !parse_output.diagnostics.is_empty() {
        report_parser_diagnostics_and_exit(&filename, &parse_output.diagnostics);
    }
    (code, filename, parse_output.stmts)
}

fn entry_script_search_paths(entry_file: &Path) -> Vec<PathBuf> {
    let mut search_paths = Vec::new();

    if let Some(parent) = entry_file.parent() {
        let normalized_parent = if parent.as_os_str().is_empty() { Path::new(".") } else { parent };
        search_paths.push(normalized_parent.to_path_buf());

        if parent.file_name().and_then(|name| name.to_str()) == Some("src") {
            if let Some(project_root) = parent.parent() {
                let normalized_root =
                    if project_root.as_os_str().is_empty() { Path::new(".") } else { project_root };
                search_paths.push(normalized_root.to_path_buf());
            }
        }
    }

    search_paths
}

fn is_known_cli_subcommand(name: &str) -> bool {
    matches!(
        name,
        "run"
            | "check"
            | "serve"
            | "repl"
            | "test"
            | "test-run"
            | "bench"
            | "format"
            | "lint"
            | "init"
            | "package-add"
            | "package-install"
            | "package-publish"
            | "pack"
            | "ecosystem"
            | "doctor"
            | "docgen"
            | "bench-cross"
            | "bench-ssg"
            | "profile"
            | "lsp-complete"
            | "lsp-definition"
            | "lsp-references"
            | "lsp-hover"
            | "lsp-diagnostics"
            | "lsp-rename"
            | "lsp-code-actions"
            | "lsp"
    )
}

fn reserved_external_alias_error(name: &str) -> Option<String> {
    if reserved_names::reservation_for_top_level_command(name).is_some() {
        return Some(reserved_names::external_reserved_command_error(name));
    }

    if let Some(reservation) = reserved_names::reservation_for_namespace(name) {
        return Some(reserved_names::external_reserved_namespace_error(name, reservation));
    }

    None
}

fn main() {
    let runtime = Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()
        .unwrap_or_else(|error| {
            eprintln!("Error: failed to initialize tokio runtime: {}", error);
            std::process::exit(1);
        });

    runtime.block_on(async_main());
}

async fn async_main() {
    let cli = Cli::parse();

    // Handle workflow pack commands via external subcommand routing.
    // When the user runs `kujo acme doctor`, clap sees "acme" as an unknown
    // subcommand and leaves `command` as None (allow_external_subcommands = true).
    if cli.command.is_none() {
        let raw_args: Vec<String> = std::env::args().collect();
        // raw_args[0] is the binary name; skip it.
        if raw_args.len() > 1 {
            let external_args: Vec<String> = raw_args[1..].to_vec();
            if let Some(first_arg) = external_args.first().map(String::as_str) {
                if !first_arg.starts_with('-') && !is_known_cli_subcommand(first_arg) {
                    if let Some(message) = reserved_external_alias_error(first_arg) {
                        report_cli_error_and_exit(message, CliExitCode::UsageError);
                    }

                    let cwd =
                        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                    match workflow_pack::handle_workflow_command(&external_args, &cwd) {
                        Ok(()) => return,
                        Err(message) => {
                            eprintln!("Error: {}", message);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }

        // If we get here, either no args were given or it was a known command
        // that clap couldn't parse (e.g., missing required args). Print help.
        use clap::CommandFactory;
        let mut app = Cli::command();
        let _ = app.print_help();
        return;
    }

    // Unwrap the command (guaranteed Some here).
    let command = cli.command.unwrap();

    match command {
        Commands::Run {
            file,
            interpreter,
            jit,
            scheduler_timeout_ms,
            json_runtime_diagnostics,
            capabilities,
            script_args,
        } => {
            let scheduler_timeout = match cooperative_scheduler_timeout(scheduler_timeout_ms) {
                Ok(timeout) => timeout,
                Err(error_message) => {
                    report_cli_error_and_exit(error_message, CliExitCode::UsageError);
                }
            };
            apply_untrusted_network_destination_policy_defaults(&capabilities);
            let capability_policy = build_runtime_capability_policy(&capabilities);

            // Store script arguments in environment for args() function to retrieve
            // We need to prepend the script filename so the filtering logic works correctly
            if !script_args.is_empty() {
                // Create a modified args list: [script_name, ...script_args]
                let mut full_args: Vec<String> = std::env::args().take(3).collect(); // kujo, run, script_name
                full_args.extend(script_args);

                // Clear current args and set new ones
                // Note: This is a workaround since we can't directly modify env::args()
                // Instead, we'll pass these to the interpreter through the environment
                std::env::set_var("KUJO_SCRIPT_ARGS", full_args[3..].join("\x1f"));
                // Use unit separator
            }

            let (code, filename, stmts) = parse_kujo_program(&file);

            // Debug: print AST for inspection
            if !interpreter && std::env::var("DEBUG_AST").is_ok() {
                eprintln!("DEBUG AST: {:#?}", stmts);
            }

            if !interpreter {
                // Use bytecode compiler and VM
                use std::sync::{Arc, Mutex};

                let mut compiler = compiler::Compiler::new();
                match compiler.compile(&stmts) {
                    Ok(chunk) => {
                        // Run the VM on a dedicated large-stack thread so deep
                        // value operations do not inherit tokio's smaller default stack.
                        let result = std::thread::Builder::new()
                            .name("kujo-vm-runner".to_string())
                            .stack_size(VM_EXECUTION_STACK_SIZE)
                            .spawn(move || {
                                let mut vm = vm::VM::new();
                                for search_path in entry_script_search_paths(&file) {
                                    vm.add_module_search_path(search_path);
                                }
                                let jit_requested = jit && std::env::var("DISABLE_JIT").is_err();
                                vm.set_jit_enabled(jit_requested);
                                if jit_requested {
                                    if let Err(reason) = vm.validate_jit_supported_surfaces(&chunk) {
                                        eprintln!(
                                            "JIT opt-in requested, but this program is not JIT-compatible ({}). Falling back to VM bytecode execution without JIT.",
                                            reason
                                        );
                                        vm.set_jit_enabled(false);
                                    }
                                }
                                vm.set_capability_policy(capability_policy.clone());

                                // Set up global environment with built-in functions
                                // We need to populate it with NativeFunction values for all built-ins
                                let env = Arc::new(Mutex::new(interpreter::Environment::new()));

                                // Register all built-in functions as NativeFunction values
                                // Get the complete list from the interpreter
                                let builtins = interpreter::Interpreter::get_builtin_names();

                                for builtin_name in builtins {
                                    env.lock().unwrap().set(
                                        builtin_name.to_string(),
                                        interpreter::Value::NativeFunction(
                                            builtin_name.to_string(),
                                        ),
                                    );
                                }

                                // Register constant globals that are not callable native functions.
                                {
                                    let mut env_lock = env.lock().unwrap();
                                    for (name, value) in crate::builtins::get_builtins() {
                                        env_lock.set(name, value);
                                    }
                                    env_lock.set("null".to_string(), interpreter::Value::Null);
                                }

                                vm.set_globals(env);

                                // Execute using cooperative suspend/resume for true concurrency
                                // Initial execution
                                let exec_result = match vm.execute_until_suspend(chunk.clone()) {
                                    Ok(vm::VmExecutionResult::Completed) => Ok(()),
                                    Ok(vm::VmExecutionResult::Suspended { .. }) => {
                                        // Run scheduler until all contexts complete.
                                        // Use a timeout budget so long-running async workloads
                                        // can complete without relying on a fixed round count.
                                        vm.run_scheduler_until_complete_with_timeout(
                                            scheduler_timeout,
                                        )
                                    }
                                    Err(e) => Err(e),
                                };

                                (exec_result, vm.get_call_stack())
                            })
                            .unwrap_or_else(|error| {
                                eprintln!("Error: failed to start Kujo VM thread: {}", error);
                                std::process::exit(1);
                            })
                            .join();

                        match result {
                            Ok((Ok(_result), _)) => {
                                // Success - program executed cooperatively to completion
                            }
                            Ok((Err(e), call_stack)) => {
                                // Create a proper error with call stack
                                use crate::errors::{
                                    DiagnosticSubsystem, KujoError, SourceLocation,
                                    DIAGNOSTIC_CODE_VM,
                                };
                                let error = KujoError::runtime_error(e, SourceLocation::unknown())
                                    .with_diagnostic_code(DIAGNOSTIC_CODE_VM)
                                    .with_subsystem(DiagnosticSubsystem::Vm)
                                    .with_call_stack(call_stack);
                                report_run_runtime_error_and_exit(
                                    &error,
                                    CliExitCode::RuntimeError,
                                    json_runtime_diagnostics,
                                );
                            }
                            Err(_) => {
                                let diagnostic = errors::Diagnostic::new(
                                    errors::DIAGNOSTIC_CODE_VM,
                                    errors::DiagnosticSeverity::Error,
                                    errors::DiagnosticSubsystem::Vm,
                                    "VM execution panicked in dedicated VM thread".to_string(),
                                );
                                report_run_runtime_diagnostic_and_exit(
                                    &diagnostic,
                                    CliExitCode::InternalError,
                                    json_runtime_diagnostics,
                                );
                            }
                        }
                    }
                    Err(e) => {
                        let diagnostic = errors::Diagnostic::new(
                            errors::DIAGNOSTIC_CODE_VM,
                            errors::DiagnosticSeverity::Error,
                            errors::DiagnosticSubsystem::Vm,
                            format!("Compilation error: {}", e),
                        );
                        report_run_runtime_diagnostic_and_exit(
                            &diagnostic,
                            CliExitCode::RuntimeError,
                            json_runtime_diagnostics,
                        );
                    }
                }
            } else {
                // Use tree-walking interpreter (fallback mode)
                // Type checking phase (optional - won't stop execution even if errors found)
                let mut type_checker = type_checker::TypeChecker::new();
                for search_path in entry_script_search_paths(&file) {
                    type_checker.add_search_path(search_path);
                }
                if let Err(errors) = type_checker.check(&stmts) {
                    eprintln!("Type checking warnings:");
                    for error in &errors {
                        eprintln!("  {}", error);
                    }
                    eprintln!();
                }

                let mut interpreter =
                    interpreter::Interpreter::with_capability_policy(capability_policy);
                for search_path in entry_script_search_paths(&file) {
                    interpreter.module_loader.add_search_path(search_path);
                }
                interpreter.set_source(filename, &code);

                // Execute statements
                interpreter.eval_stmts(&stmts);

                // Check for errors in return_value and display with call stack
                if let Some(ref val) = interpreter.return_value {
                    use crate::errors::KujoError;
                    match val {
                        interpreter::Value::Error(msg) => {
                            let err = KujoError::runtime_error(
                                msg.clone(),
                                crate::errors::SourceLocation::unknown(),
                            )
                            .with_call_stack(interpreter.get_call_stack());
                            report_run_runtime_error_and_exit(
                                &err,
                                CliExitCode::RuntimeError,
                                json_runtime_diagnostics,
                            );
                        }
                        interpreter::Value::ErrorObject { message, .. } => {
                            let err = KujoError::runtime_error(
                                message.clone(),
                                crate::errors::SourceLocation::unknown(),
                            )
                            .with_call_stack(interpreter.get_call_stack());
                            report_run_runtime_error_and_exit(
                                &err,
                                CliExitCode::RuntimeError,
                                json_runtime_diagnostics,
                            );
                        }
                        _ => {}
                    }
                }

                interpreter.cleanup();
                drop(interpreter);
            }
        }

        Commands::Check { file, quiet, verbose, json } => {
            let (_code, filename, stmts) = parse_kujo_program(&file);
            let mut compiler = compiler::Compiler::new();
            let instruction_count = match compiler.compile(&stmts) {
                Ok(chunk) => chunk.instructions.len(),
                Err(error_message) => {
                    let diagnostic = errors::Diagnostic::new(
                        errors::DIAGNOSTIC_CODE_VM,
                        errors::DiagnosticSeverity::Error,
                        errors::DiagnosticSubsystem::Vm,
                        format!("Compilation error: {}", error_message),
                    );
                    report_diagnostics_and_exit(&[diagnostic], CliExitCode::RuntimeError);
                }
            };

            if json {
                let output = serde_json::json!({
                    "command": "check",
                    "file": filename,
                    "status": "ok",
                    "statement_count": stmts.len(),
                    "bytecode_instruction_count": instruction_count,
                });
                match serde_json::to_string_pretty(&output) {
                    Ok(serialized) => println!("{}", serialized),
                    Err(error) => {
                        report_cli_error_and_exit(
                            format!("Failed to serialize check result: {}", error),
                            CliExitCode::InternalError,
                        );
                    }
                }
                return;
            }

            if !quiet {
                if verbose {
                    println!(
                        "check passed: {} (statements={}, bytecode_instructions={})",
                        file.display(),
                        stmts.len(),
                        instruction_count
                    );
                } else {
                    println!("check passed: {}", file.display());
                }
            }
        }

        Commands::Serve {
            dir,
            port,
            host,
            index,
            hardened,
            cache_max_age,
            access_log,
            tls_cert,
            tls_key,
            max_request_line_bytes,
            max_header_bytes,
            max_header_count,
            max_request_body_bytes,
            read_timeout_ms,
            write_timeout_ms,
            max_connections,
        } => {
            let options = serve_http::ServeServerOptions {
                index,
                hardened,
                cache_max_age: Some(u64::from(cache_max_age)),
                access_log,
                tls_cert,
                tls_key,
                max_request_line_bytes,
                max_header_bytes,
                max_header_count,
                max_request_body_bytes,
                read_timeout: Duration::from_millis(read_timeout_ms),
                write_timeout: Duration::from_millis(write_timeout_ms),
                max_connections,
            };

            if let Err(message) = serve_http::run_static_server(dir, host, port, options) {
                eprintln!("{}", message);
                std::process::exit(CliExitCode::RuntimeError.code());
            }
        }

        Commands::Repl => match repl::Repl::new() {
            Ok(mut repl) => {
                if let Err(e) = repl.run() {
                    eprintln!("REPL error: {}", e);
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            }
            Err(e) => {
                eprintln!("Failed to start REPL: {}", e);
                std::process::exit(CliExitCode::RuntimeError.code());
            }
        },

        Commands::Test { update, runtime, verbose } => {
            use std::path::Path;
            let runtime_strategy = match runtime {
                TestRuntimeMode::Interpreter => parser::TestRuntimeStrategy::Interpreter,
                TestRuntimeMode::Vm => parser::TestRuntimeStrategy::Vm,
                TestRuntimeMode::Dual => parser::TestRuntimeStrategy::Dual,
            };
            let exit_code = parser::Parser::run_all_tests(
                Path::new("tests"),
                update,
                runtime_strategy,
                verbose,
            );
            std::process::exit(exit_code);
        }

        Commands::TestRun { file, verbose, capabilities } => {
            apply_untrusted_network_destination_policy_defaults(&capabilities);
            let (_code, _filename, stmts) = parse_kujo_program(&file);

            // Create base interpreter with standard library loaded
            let base_interp = interpreter::Interpreter::with_capability_policy(
                build_runtime_capability_policy(&capabilities),
            );

            // Create test runner and collect tests
            let mut runner = interpreter::TestRunner::new();
            runner.collect_tests(&stmts);

            if runner.tests.is_empty() {
                println!("No tests found in {}", file.display());
                std::process::exit(CliExitCode::RuntimeError.code());
            }

            // Run all tests
            let report = runner.run_all(&base_interp);

            // Print results
            report.print(verbose);

            // Exit with appropriate code
            std::process::exit(report.exit_code());
        }

        Commands::Bench { path, iterations, warmup } => {
            use benchmarks::{BenchmarkRunner, Reporter};

            let runner = BenchmarkRunner::new().with_iterations(iterations).with_warmup(warmup);

            Reporter::print_header("Kujo Performance Benchmarks");

            let results = if let Some(p) = path {
                if p.is_dir() {
                    runner.run_directory(p)
                } else if p.is_file() {
                    vec![(
                        p.file_stem().and_then(|s| s.to_str()).unwrap_or("benchmark").to_string(),
                        runner.run_file(p),
                    )]
                } else {
                    eprintln!("Error: Path does not exist: {}", p.display());
                    std::process::exit(CliExitCode::IoError.code());
                }
            } else {
                // Default to benchmarks directory
                let default_path = PathBuf::from("examples/benchmarks");
                if default_path.exists() {
                    runner.run_directory(default_path)
                } else {
                    eprintln!("Error: No benchmark directory found. Please specify a path.");
                    std::process::exit(CliExitCode::IoError.code());
                }
            };

            // Print individual results
            for (name, bench_results) in &results {
                println!("\n{}", name);
                for result in bench_results {
                    Reporter::print_benchmark_result(result);
                }
            }

            // Print comparison table
            Reporter::print_comparison_table(&results);

            // Print summary
            Reporter::print_summary(&results);
        }

        Commands::Format { file, indent, line_length, no_sort_imports, check, write, json } => {
            let source = match fs::read_to_string(&file) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!("Failed to read .kujo file '{}': {}", file.display(), err);
                    std::process::exit(CliExitCode::IoError.code());
                }
            };
            let options = formatter::FormatterOptions {
                indent_width: indent,
                line_length,
                sort_imports: !no_sort_imports,
            };
            let formatted = formatter::format_source(&source, &options);
            let changed = source != formatted;

            if write {
                if let Err(err) = fs::write(&file, &formatted) {
                    eprintln!("Failed to write formatted file '{}': {}", file.display(), err);
                    std::process::exit(CliExitCode::IoError.code());
                }
            }

            if json {
                let status = if write {
                    "written"
                } else if check {
                    if changed {
                        "needs_formatting"
                    } else {
                        "already_formatted"
                    }
                } else {
                    "preview"
                };

                let output = serde_json::json!({
                    "command": "format",
                    "file": file.display().to_string(),
                    "status": status,
                    "changed": changed,
                    "options": {
                        "indent": indent,
                        "line_length": line_length,
                        "sort_imports": !no_sort_imports,
                        "check": check,
                        "write": write,
                    },
                    "formatted_source": if write { serde_json::Value::Null } else { serde_json::Value::String(formatted.clone()) },
                });

                match serde_json::to_string_pretty(&output) {
                    Ok(serialized) => println!("{}", serialized),
                    Err(e) => {
                        eprintln!("Failed to serialize formatter result: {}", e);
                        std::process::exit(CliExitCode::InternalError.code());
                    }
                }

                if check && changed {
                    std::process::exit(CliExitCode::RuntimeError.code());
                }

                return;
            }

            if check {
                if !changed {
                    println!("already formatted");
                } else {
                    println!("needs formatting");
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            } else if write {
                println!("formatted {}", file.display());
            } else {
                println!("{}", formatted);
            }
        }

        Commands::Lint { file, fix, json } => {
            let source = match fs::read_to_string(&file) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!("Failed to read .kujo file '{}': {}", file.display(), err);
                    std::process::exit(CliExitCode::IoError.code());
                }
            };
            let issues = linter::lint_source(&source);

            if fix {
                let fixed = linter::apply_safe_fixes(&source, &issues);
                if fixed != source {
                    if let Err(err) = fs::write(&file, fixed) {
                        eprintln!("Failed to write lint fixes to '{}': {}", file.display(), err);
                        std::process::exit(CliExitCode::IoError.code());
                    }
                    println!("applied safe lint fixes to {}", file.display());
                } else {
                    println!("no safe lint fixes applied");
                }
            }

            if json {
                let json_items: Vec<serde_json::Value> = issues
                    .iter()
                    .map(|issue| {
                        serde_json::json!({
                            "rule_id": issue.rule_id,
                            "line": issue.line,
                            "column": issue.column,
                            "severity": issue.severity.as_str(),
                            "message": issue.message,
                            "fix": issue.fix.as_ref().map(|fix| serde_json::json!({
                                "replacement_line": fix.replacement_line,
                                "description": fix.description,
                            })),
                        })
                    })
                    .collect();

                match serde_json::to_string_pretty(&json_items) {
                    Ok(serialized) => println!("{}", serialized),
                    Err(e) => {
                        eprintln!("Failed to serialize lint results: {}", e);
                        std::process::exit(CliExitCode::InternalError.code());
                    }
                }
            } else {
                for issue in issues.iter() {
                    println!(
                        "{}\t{}\t{}:{}:{}\t{}",
                        issue.severity.as_str(),
                        issue.rule_id,
                        file.display(),
                        issue.line,
                        issue.column,
                        issue.message
                    );
                }
            }

            if issues.iter().any(|issue| matches!(issue.severity, linter::LintSeverity::Error)) {
                std::process::exit(CliExitCode::RuntimeError.code());
            }
        }

        Commands::Init { dir, name } => {
            let project_dir = dir.unwrap_or_else(|| PathBuf::from("."));
            let package_name = match name {
                Some(explicit_name) => explicit_name,
                None => project_dir
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("kujo_project")
                    .to_string(),
            };

            if let Some(reservation) = reserved_names::reservation_for_package_name(&package_name) {
                report_cli_error_and_exit(
                    format!(
                        "Package name '{}' is reserved by Kujo (category: {}). Choose a different project/package name.",
                        package_name,
                        reservation.category_label()
                    ),
                    CliExitCode::UsageError,
                );
            }

            let src_dir = project_dir.join("src");
            if let Err(err) = std::fs::create_dir_all(&src_dir) {
                eprintln!("Failed to create src directory '{}': {}", src_dir.display(), err);
                std::process::exit(CliExitCode::IoError.code());
            }

            let manifest_path = project_dir.join("kujo.toml");
            if !manifest_path.exists() {
                let manifest = package_workflow::default_manifest(&package_name);
                if let Err(err) = std::fs::write(&manifest_path, manifest) {
                    eprintln!("Failed to write '{}': {}", manifest_path.display(), err);
                    std::process::exit(CliExitCode::IoError.code());
                }
            }

            let main_source_path = src_dir.join("main.kujo");
            if !main_source_path.exists() {
                if let Err(err) = std::fs::write(&main_source_path, "print(\"Hello from Kujo\")\n")
                {
                    eprintln!("Failed to write '{}': {}", main_source_path.display(), err);
                    std::process::exit(CliExitCode::IoError.code());
                }
            }

            println!("initialized project at {}", project_dir.display());
        }

        Commands::PackageAdd { name, version, manifest } => {
            let manifest_path = manifest.unwrap_or_else(|| PathBuf::from("kujo.toml"));
            let content = match fs::read_to_string(&manifest_path) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!("Failed to read '{}': {}", manifest_path.display(), err);
                    std::process::exit(CliExitCode::IoError.code());
                }
            };
            let updated = match package_workflow::add_dependency(&content, &name, &version) {
                Ok(manifest_content) => manifest_content,
                Err(message) => {
                    eprintln!("{}", message);
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            };

            if let Err(err) = fs::write(&manifest_path, updated) {
                eprintln!("Failed to write '{}': {}", manifest_path.display(), err);
                std::process::exit(CliExitCode::IoError.code());
            }
            println!("added dependency {} {}", name, version);
        }

        Commands::PackageInstall { manifest, lockfile, frozen } => {
            let manifest_path = manifest.unwrap_or_else(|| PathBuf::from("kujo.toml"));
            let content = match fs::read_to_string(&manifest_path) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!("Failed to read '{}': {}", manifest_path.display(), err);
                    std::process::exit(CliExitCode::IoError.code());
                }
            };
            let parsed = match package_workflow::parse_manifest(&content) {
                Ok(manifest_data) => manifest_data,
                Err(message) => {
                    eprintln!("{}", message);
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            };

            let lockfile_path =
                lockfile.unwrap_or_else(|| package_workflow::default_lockfile_path(&manifest_path));
            let expected_lockfile = package_workflow::lockfile_from_manifest(&parsed);

            if frozen {
                let lockfile_content = match fs::read_to_string(&lockfile_path) {
                    Ok(content) => content,
                    Err(err) => {
                        eprintln!("Failed to read '{}': {}", lockfile_path.display(), err);
                        std::process::exit(CliExitCode::IoError.code());
                    }
                };

                let lockfile_data = match package_workflow::parse_lockfile(&lockfile_content) {
                    Ok(lockfile_data) => lockfile_data,
                    Err(message) => {
                        eprintln!("{}", message);
                        std::process::exit(CliExitCode::RuntimeError.code());
                    }
                };

                if let Err(message) =
                    package_workflow::verify_lockfile_matches_manifest(&parsed, &lockfile_data)
                {
                    eprintln!("{}", message);
                    std::process::exit(CliExitCode::RuntimeError.code());
                }

                println!("lockfile verified\t{}", lockfile_path.display());
            } else {
                let lockfile_text = match package_workflow::serialize_lockfile(&expected_lockfile) {
                    Ok(lockfile_text) => lockfile_text,
                    Err(message) => {
                        eprintln!("{}", message);
                        std::process::exit(CliExitCode::RuntimeError.code());
                    }
                };

                if let Err(err) = fs::write(&lockfile_path, lockfile_text) {
                    eprintln!("Failed to write '{}': {}", lockfile_path.display(), err);
                    std::process::exit(CliExitCode::IoError.code());
                }

                println!("lockfile written\t{}", lockfile_path.display());
            }

            if parsed.dependencies.is_empty() {
                println!("no dependencies declared");
            } else {
                for (dependency_name, dependency_version) in parsed.dependencies.iter() {
                    println!("install\t{}\t{}", dependency_name, dependency_version);
                }
            }
        }

        Commands::PackagePublish { manifest, publish } => {
            let manifest_path = manifest.unwrap_or_else(|| PathBuf::from("kujo.toml"));
            let content = match fs::read_to_string(&manifest_path) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!("Failed to read '{}': {}", manifest_path.display(), err);
                    std::process::exit(CliExitCode::IoError.code());
                }
            };
            let parsed = match package_workflow::parse_manifest(&content) {
                Ok(manifest_data) => manifest_data,
                Err(message) => {
                    eprintln!("{}", message);
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            };

            if publish {
                eprintln!(
                    "package publish transport is not available in Kujo v1.0; run without --publish for metadata preview"
                );
                std::process::exit(CliExitCode::UsageError.code());
            }

            println!(
                "publish preview\t{}\t{}\tdependencies={}",
                parsed.package.name,
                parsed.package.version,
                parsed.dependencies.len()
            );
        }

        Commands::Agent { command } => {
            if let Err(err) = agent_project::execute(command) {
                eprintln!("{}", err.message);
                std::process::exit(err.exit_code);
            }
        }

        Commands::Pack { command } => match command {
            PackCommands::Run { namespace, command, json, save, command_args } => {
                if let Some(reservation) = reserved_names::reservation_for_namespace(&namespace) {
                    report_cli_error_and_exit(
                        reserved_names::external_reserved_namespace_error(&namespace, reservation),
                        CliExitCode::UsageError,
                    );
                }

                let mut external_args = vec![namespace, command];
                external_args.extend(command_args);
                if json {
                    external_args.push("--json".to_string());
                }
                if save {
                    external_args.push("--save".to_string());
                }

                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                match workflow_pack::handle_workflow_command(&external_args, &cwd) {
                    Ok(()) => {}
                    Err(message) => {
                        report_cli_error_and_exit(message, CliExitCode::RuntimeError);
                    }
                }
            }
        },

        Commands::Doctor { json, deep, list_profiles, profile, profile_name } => {
            if let (Some(explicit), Some(positional)) = (&profile, &profile_name) {
                if explicit != positional {
                    report_cli_error_and_exit(
                        format!(
                            "Conflicting doctor profile values: --profile '{}' and positional '{}'.",
                            explicit, positional
                        ),
                        CliExitCode::UsageError,
                    );
                }
            }

            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let (registry, warnings) = workflow_pack::initialize_registry(&cwd);
            for warning in &warnings {
                eprintln!("Warning: {}", warning);
            }

            let mut available_profiles: Vec<String> = registry
                .list_doctor_profiles()
                .iter()
                .map(|profile| profile.profile_name.clone())
                .collect();
            available_profiles
                .push(workflow_pack::builtins::doctor::GENERIC_PROFILE_NAME.to_string());
            available_profiles.push("agent".to_string());
            available_profiles.sort();
            available_profiles.dedup();

            if list_profiles {
                if json {
                    let payload = serde_json::json!({
                        "command": "doctor",
                        "tool": "kujo-doctor",
                        "profiles": available_profiles,
                        "default_profile": workflow_pack::builtins::doctor::GENERIC_PROFILE_NAME,
                    });
                    emit_json_or_internal_error(&payload, "doctor profile list");
                } else {
                    println!("Available doctor profiles:");
                    for profile_name in &available_profiles {
                        if profile_name == workflow_pack::builtins::doctor::GENERIC_PROFILE_NAME {
                            println!("- {} (default)", profile_name);
                        } else {
                            println!("- {}", profile_name);
                        }
                    }
                }
                return;
            }

            let selected_profile = profile.or(profile_name).unwrap_or_else(|| {
                workflow_pack::builtins::doctor::GENERIC_PROFILE_NAME.to_string()
            });

            if selected_profile == "agent" {
                let report = agent_project::doctor_report(&cwd, deep);
                if json {
                    emit_json_or_internal_error(&report, "agent doctor report");
                } else {
                    workflow_pack::renderer::render_human(&report);
                }
                if report.status == "fail" {
                    std::process::exit(1);
                }
                return;
            }

            if !available_profiles.iter().any(|candidate| candidate == &selected_profile) {
                report_cli_error_and_exit(
                    format!(
                        "Unknown doctor profile '{}'. Use `kujo doctor --list-profiles` to inspect available profiles.",
                        selected_profile
                    ),
                    CliExitCode::UsageError,
                );
            }

            let mut ctx_args = Vec::new();
            if deep {
                ctx_args.push("--deep".to_string());
            }
            let ctx = workflow_pack::types::CommandContext {
                cwd: cwd.clone(),
                json_output: json,
                args: ctx_args,
                env_vars: std::collections::HashMap::new(),
            };

            let mut generic_report = match registry.execute(
                workflow_pack::builtins::doctor::DOCTOR_NAMESPACE,
                workflow_pack::builtins::doctor::DOCTOR_COMMAND,
                &ctx,
            ) {
                Ok(report) => report,
                Err(message) => report_cli_error_and_exit(message, CliExitCode::RuntimeError),
            };
            workflow_pack::builtins::doctor::decorate_report_metadata(
                &mut generic_report,
                workflow_pack::builtins::doctor::GENERIC_PROFILE_NAME,
            );

            let mut report = if selected_profile
                == workflow_pack::builtins::doctor::GENERIC_PROFILE_NAME
            {
                generic_report
            } else {
                let profile_report = match registry.execute_doctor_profile(&selected_profile, &ctx)
                {
                    Ok(report) => report,
                    Err(message) => report_cli_error_and_exit(message, CliExitCode::RuntimeError),
                };
                workflow_pack::builtins::doctor::merge_reports(
                    &generic_report,
                    &profile_report,
                    &selected_profile,
                )
            };

            report.cwd = Some(cwd.to_string_lossy().to_string());

            if json {
                emit_json_or_internal_error(&report, "doctor report");
            } else {
                workflow_pack::renderer::render_human(&report);
            }

            if report.status == "fail" {
                std::process::exit(1);
            }
        }

        Commands::Ecosystem { command } => match command {
            EcosystemCommands::GoldenPath {
                repo_root,
                output_root,
                timeout_seconds,
                allow_blocked,
                json,
            } => {
                if timeout_seconds == 0 {
                    report_cli_error_and_exit(
                        "golden-path timeout must be greater than zero seconds".to_string(),
                        CliExitCode::UsageError,
                    );
                }
                let exit_code = ecosystem_golden_path::run(
                    &repo_root,
                    output_root.as_deref(),
                    Duration::from_secs(timeout_seconds),
                    allow_blocked,
                    json,
                );
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
        },

        Commands::Docgen {
            path,
            out_dir,
            format,
            language,
            languages,
            no_builtins,
            emit_ai_tasks,
            search_index,
            source_links,
            source_link_template,
            public_only,
            include_private,
            fail_on_undocumented,
            fail_on_broken_links,
            validate_local_anchors,
            validate_external_links,
            external_link_timeout_ms,
            external_link_allowlist,
            allow_private_network_links,
            max_link_checks,
            max_external_link_checks,
            max_total_validation_time_ms,
            fail_on_warnings,
            max_discovery_file_size_bytes,
            max_discovery_files,
            max_discovery_depth,
            cache_dir,
            kujo_parser_assisted,
            json,
        } => {
            let output_dir = out_dir.unwrap_or_else(|| PathBuf::from("docs/generated"));
            let doc_format = match docgen::core::parse_output_format(format.as_deref()) {
                Ok(value) => value,
                Err(message) => {
                    eprintln!("{}", message);
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            };
            let external_link_allowlist: BTreeSet<String> = external_link_allowlist
                .as_deref()
                .map(|csv| {
                    csv.split(',')
                        .map(|entry| entry.trim().to_ascii_lowercase())
                        .filter(|entry| !entry.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            let (_project, summary) = match docgen::core::run_with_link_validation_and_options(
                &docgen::core::DocgenConfig {
                    input: path.clone(),
                    out_dir: output_dir,
                    format: doc_format,
                    include_builtins: !no_builtins,
                    language,
                    languages,
                    emit_ai_tasks,
                    search_index,
                    source_links,
                    source_link_template,
                    fail_on_undocumented,
                    fail_on_broken_links,
                    fail_on_warnings,
                    public_only,
                    include_private,
                    max_discovery_file_size_bytes,
                    max_discovery_files,
                    max_discovery_depth,
                    cache_dir,
                },
                docgen::gaps::LinkValidationOptions {
                    validate_local_anchors,
                    validate_external_links,
                    external_link_timeout_ms,
                    external_link_allowlist,
                    allow_private_network_links,
                    max_link_checks,
                    max_external_link_checks,
                    max_total_validation_time_ms,
                },
                docgen::core::DocgenExtractionOptions { kujo_parser_assisted },
            ) {
                Ok(result) => result,
                Err(message) => {
                    eprintln!("{}", message);
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            };

            if json {
                let output = docgen::core::build_cli_json_payload(&path, &summary);

                match serde_json::to_string_pretty(&output) {
                    Ok(serialized) => println!("{}", serialized),
                    Err(e) => {
                        eprintln!("Failed to serialize docgen result: {}", e);
                        std::process::exit(CliExitCode::InternalError.code());
                    }
                }

                return;
            }

            println!("generated docs in {}", summary.output_dir.display());
            println!("module docs: {}", summary.module_doc_path.display());
            if let Some(builtin_path) = summary.builtin_doc_path {
                println!("builtin docs: {}", builtin_path.display());
            }
            println!("documented items: {}", summary.item_count);
            println!("project symbols: {}", summary.project_symbol_count);
            println!("builtin symbols: {}", summary.builtin_symbol_count);
            if !summary.gate_failures.is_empty() {
                for failure in &summary.gate_failures {
                    eprintln!("docgen gate failed: {}", failure);
                }
                std::process::exit(1);
            }
        }

        Commands::BenchCross { kujo_script, python_script, python } => {
            use benchmarks::run_process_pool_comparison;

            let kujo_binary = match std::env::current_exe() {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("Failed to determine Kujo binary path: {}", e);
                    std::process::exit(CliExitCode::InternalError.code());
                }
            };

            if !kujo_script.exists() {
                eprintln!("Kujo benchmark script not found: {}", kujo_script.display());
                std::process::exit(CliExitCode::IoError.code());
            }

            if !python_script.exists() {
                eprintln!("Python benchmark script not found: {}", python_script.display());
                std::process::exit(CliExitCode::IoError.code());
            }

            match run_process_pool_comparison(
                kujo_binary.as_path(),
                kujo_script.as_path(),
                python.as_str(),
                python_script.as_path(),
            ) {
                Ok(comparison) => {
                    println!("Kujo parallel_map vs Python ProcessPoolExecutor");
                    println!("-----------------------------------------------");
                    println!("Kujo parallel_map: {:.3} ms", comparison.kujo_parallel_map_ms);
                    println!(
                        "Python ProcessPoolExecutor: {:.3} ms",
                        comparison.python_process_pool_ms
                    );
                    if let Some(serial_ms) = comparison.python_serial_ms {
                        println!("Python serial baseline: {:.3} ms", serial_ms);
                    }

                    println!(
                        "Kujo speedup vs Python ProcessPool: {:.2}x",
                        comparison.kujo_vs_process_pool_speedup()
                    );
                    if let Some(pool_speedup) = comparison.process_pool_vs_serial_speedup() {
                        println!("Python ProcessPool speedup vs serial: {:.2}x", pool_speedup);
                    }
                }
                Err(e) => {
                    eprintln!("Cross-language benchmark failed: {}", e);
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            }
        }

        Commands::BenchSsg {
            kujo_script,
            warmup_runs,
            runs,
            throughput_gate_ms,
            profile_async,
            compare_python,
            python_script,
            python,
            tmp_dir,
            variability_warning_threshold,
            trend_warning_threshold,
            mean_median_drift_warning_threshold,
            range_spread_warning_threshold,
        } => {
            use benchmarks::ssg::{
                analyze_ssg_benchmark_trends, evaluate_ssg_throughput_gate,
                render_ssg_benchmark_report, SsgWarningThresholds,
            };
            use benchmarks::{aggregate_ssg_results, run_ssg_benchmark_series};

            let kujo_binary = match std::env::current_exe() {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("Failed to determine Kujo binary path: {}", e);
                    std::process::exit(CliExitCode::InternalError.code());
                }
            };

            if !kujo_script.exists() {
                eprintln!("Kujo SSG benchmark script not found: {}", kujo_script.display());
                std::process::exit(CliExitCode::IoError.code());
            }

            let python_script_path = if compare_python {
                if !python_script.exists() {
                    eprintln!("Python SSG benchmark script not found: {}", python_script.display());
                    std::process::exit(CliExitCode::IoError.code());
                }
                Some(python_script.as_path())
            } else {
                None
            };

            let python_binary = if compare_python { Some(python.as_str()) } else { None };

            if runs == 0 {
                eprintln!("SSG benchmark runs must be >= 1");
                std::process::exit(CliExitCode::UsageError.code());
            }

            if variability_warning_threshold < 0.0
                || trend_warning_threshold < 0.0
                || mean_median_drift_warning_threshold < 0.0
                || range_spread_warning_threshold < 0.0
            {
                eprintln!(
                    "SSG warning thresholds must be >= 0.0 (got variability={}, trend={}, mean-median={}, range-spread={})",
                    variability_warning_threshold,
                    trend_warning_threshold,
                    mean_median_drift_warning_threshold,
                    range_spread_warning_threshold
                );
                std::process::exit(CliExitCode::UsageError.code());
            }

            let warning_thresholds = SsgWarningThresholds {
                variability_percent: variability_warning_threshold,
                trend_percent: trend_warning_threshold,
                mean_median_drift_percent: mean_median_drift_warning_threshold,
                range_spread_percent: range_spread_warning_threshold,
            };

            if warmup_runs > 0 {
                println!("Running SSG benchmark warmups ({})...", warmup_runs);
            }
            if runs > 1 {
                println!("Running SSG benchmark measured runs ({})...", runs);
            }

            let run_results = match run_ssg_benchmark_series(
                kujo_binary.as_path(),
                kujo_script.as_path(),
                python_binary,
                python_script_path,
                tmp_dir.as_deref(),
                warmup_runs,
                runs,
                profile_async,
            ) {
                Ok(results) => results,
                Err(e) => {
                    eprintln!("SSG benchmark failed: {}", e);
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            };

            let summary = match aggregate_ssg_results(&run_results) {
                Ok(summary) => summary,
                Err(e) => {
                    eprintln!("SSG benchmark aggregation failed: {}", e);
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            };

            let throughput_gate = if let Some(gate_threshold_ms) = throughput_gate_ms {
                Some(
                    match evaluate_ssg_throughput_gate(
                        summary.kujo_build_ms.median,
                        gate_threshold_ms,
                    ) {
                        Ok(gate) => gate,
                        Err(e) => {
                            eprintln!("SSG throughput gate validation failed: {}", e);
                            std::process::exit(CliExitCode::RuntimeError.code());
                        }
                    },
                )
            } else {
                None
            };

            let trend_report = match analyze_ssg_benchmark_trends(&run_results) {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("SSG benchmark trend analysis failed: {}", e);
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            };

            for line in render_ssg_benchmark_report(
                &summary,
                trend_report.as_ref(),
                warmup_runs,
                throughput_gate,
                profile_async,
                warning_thresholds,
            ) {
                println!("{}", line);
            }

            if let Some(gate) = throughput_gate {
                if !gate.passed {
                    eprintln!(
                        "SSG throughput gate failed: Kujo median build time {:.3} ms exceeded target {:.3} ms",
                        gate.observed_median_ms,
                        gate.threshold_ms
                    );
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            }
        }

        Commands::Profile { file, cpu, memory, jit, flamegraph } => {
            use benchmarks::{
                print_profile_report, profiler::generate_flamegraph_data, ProfileConfig, Profiler,
            };

            // Read and parse the file
            let code = read_kujo_source_for_parse(&file);
            let filename = file.to_string_lossy().to_string();

            // Create profile configuration
            let config = ProfileConfig {
                cpu_profiling: cpu,
                memory_profiling: memory,
                jit_stats: jit,
                ..Default::default()
            };

            let mut profiler = Profiler::new(config);
            profiler.start();

            // Execute the code
            let tokens = match lexer::tokenize_with_file(&code, Some(&filename)) {
                Ok(tokens) => tokens,
                Err(diagnostics) => report_lexer_diagnostics_and_exit(&filename, &diagnostics),
            };
            let mut parser = parser::Parser::new(tokens);
            let parse_output = parser.parse_with_diagnostics();
            if !parse_output.diagnostics.is_empty() {
                report_parser_diagnostics_and_exit(&filename, &parse_output.diagnostics);
            }
            let stmts = parse_output.stmts;

            let mut interp = interpreter::Interpreter::new();
            interp.set_source(filename, &code);

            // Run the program
            interp.eval_stmts(&stmts);

            // Stop profiling
            let elapsed = profiler.stop();

            // Get profile data
            let profile_data = profiler.into_data();

            // Print profile report
            print_profile_report(&profile_data);

            println!("\nTotal execution time: {:.3}s", elapsed.as_secs_f64());

            // Generate flamegraph if requested
            if let Some(fg_path) = flamegraph {
                let fg_data = generate_flamegraph_data(&profile_data.cpu);
                if let Err(err) = fs::write(&fg_path, fg_data) {
                    eprintln!("Failed to write flamegraph data '{}': {}", fg_path.display(), err);
                    std::process::exit(CliExitCode::IoError.code());
                }
                println!("\nFlamegraph data written to: {}", fg_path.display());
                println!("Generate SVG with: flamegraph.pl {} > flamegraph.svg", fg_path.display());
            }

            // Cleanup
            interp.cleanup();
        }

        Commands::LspComplete { file, line, column, json } => {
            let code = read_kujo_source_for_parse(&file);
            let completion_items = lsp_completion::complete(&code, line, column);

            if json {
                #[derive(serde::Serialize)]
                struct CompletionOutput {
                    label: String,
                    kind: String,
                }

                if let Err(error) =
                    cli_output::emit_json_array(&completion_items, |item| CompletionOutput {
                        label: item.label.clone(),
                        kind: item.kind.as_str().to_string(),
                    })
                {
                    report_json_internal_error_and_exit(
                        format!("Failed to serialize completion results: {}", error),
                        CliExitCode::InternalError,
                    );
                }
            } else {
                cli_output::emit_tsv_rows(
                    completion_items.iter().map(|item| [item.label.as_str(), item.kind.as_str()]),
                );
            }
        }

        Commands::LspDefinition { file, line, column, json } => {
            let code = read_kujo_source_for_parse(&file);
            let definition = lsp_definition::find_definition(&code, line, column);

            if json {
                #[derive(serde::Serialize)]
                struct DefinitionOutput {
                    name: String,
                    line: usize,
                    column: usize,
                    kind: String,
                }

                if let Err(error) =
                    cli_output::emit_optional_json_record(definition.as_ref(), |location| {
                        DefinitionOutput {
                            name: location.name.clone(),
                            line: location.line,
                            column: location.column,
                            kind: location.kind.as_str().to_string(),
                        }
                    })
                {
                    report_json_internal_error_and_exit(
                        format!("Failed to serialize definition result: {}", error),
                        CliExitCode::InternalError,
                    );
                }
            } else {
                match definition {
                    Some(location) => {
                        let position =
                            format!("{}:{}:{}", file.display(), location.line, location.column);
                        println!(
                            "{}",
                            cli_output::format_tsv_row([location.name.as_str(), position.as_str()])
                        );
                    }
                    None => {
                        println!("not found");
                    }
                }
            }
        }

        Commands::LspReferences { file, line, column, include_definition, json } => {
            let code = read_kujo_source_for_parse(&file);
            let references =
                lsp_references::find_references(&code, line, column, include_definition);

            if json {
                if let Err(error) = cli_output::emit_json_array(&references, |reference| {
                    serde_json::json!({
                        "line": reference.line,
                        "column": reference.column,
                        "is_definition": reference.is_definition,
                    })
                }) {
                    report_json_internal_error_and_exit(
                        format!("Failed to serialize references result: {}", error),
                        CliExitCode::InternalError,
                    );
                }
            } else {
                cli_output::emit_tsv_rows(references.iter().map(|reference| {
                    let role = if reference.is_definition { "definition" } else { "reference" };
                    let position =
                        format!("{}:{}:{}", file.display(), reference.line, reference.column);
                    [role.to_string(), position]
                }));
            }
        }

        Commands::LspHover { file, line, column, json } => {
            let code = read_kujo_source_for_parse(&file);
            let hover_info = lsp_hover::hover(&code, line, column);

            if json {
                if let Err(error) =
                    cli_output::emit_optional_json_record(hover_info.as_ref(), |info| {
                        serde_json::json!({
                            "symbol": info.symbol,
                            "kind": info.kind,
                            "detail": info.detail,
                            "line": info.line,
                            "column": info.column,
                        })
                    })
                {
                    report_json_internal_error_and_exit(
                        format!("Failed to serialize hover result: {}", error),
                        CliExitCode::InternalError,
                    );
                }
            } else {
                match hover_info {
                    Some(info) => {
                        let position = format!("{}:{}:{}", file.display(), info.line, info.column);
                        println!(
                            "{}",
                            cli_output::format_tsv_row([
                                info.symbol.as_str(),
                                info.detail.as_str(),
                                position.as_str(),
                            ])
                        );
                    }
                    None => {
                        println!("not found");
                    }
                }
            }
        }

        Commands::LspDiagnostics { file, json } => {
            let code = read_kujo_source_for_parse(&file);
            let diagnostics = lsp_diagnostics::diagnose(&code);

            if json {
                if let Err(error) = cli_output::emit_json_array(&diagnostics, |diagnostic| {
                    diagnostic.to_json_value()
                }) {
                    report_json_internal_error_and_exit(
                        format!("Failed to serialize diagnostics result: {}", error),
                        CliExitCode::InternalError,
                    );
                }
            } else {
                cli_output::emit_tsv_rows(diagnostics.iter().map(|diagnostic| {
                    let position =
                        format!("{}:{}:{}", file.display(), diagnostic.line, diagnostic.column);
                    let message = format!(
                        "[{}] [{}] {}",
                        diagnostic.code,
                        diagnostic.subsystem.as_str(),
                        diagnostic.message
                    );
                    [diagnostic.severity.as_str().to_string(), position, message]
                }));
            }
        }

        Commands::LspRename { file, line, column, new_name, json } => {
            let code = read_kujo_source_for_parse(&file);
            let rename_result = match lsp_rename::rename_symbol(&code, line, column, &new_name) {
                Ok(result) => result,
                Err(message) => {
                    if json {
                        report_json_command_error_and_exit(
                            "lsp-rename",
                            "runtime_error",
                            message,
                            CliExitCode::RuntimeError,
                        );
                    }
                    eprintln!("{}", message);
                    std::process::exit(CliExitCode::RuntimeError.code());
                }
            };

            if json {
                let json_edits: Vec<serde_json::Value> = rename_result
                    .edits
                    .iter()
                    .map(|edit| {
                        serde_json::json!({
                            "line": edit.line,
                            "column": edit.column,
                            "old_name": edit.old_name,
                            "new_name": edit.new_name,
                        })
                    })
                    .collect();

                let output = serde_json::json!({
                    "edit_count": json_edits.len(),
                    "edits": json_edits,
                    "updated_source": rename_result.updated_source,
                });

                emit_json_or_internal_error(&output, "rename result");
            } else {
                println!("renamed\t{} edits", rename_result.edits.len());
                for edit in rename_result.edits.iter() {
                    let position = format!("{}:{}:{}", file.display(), edit.line, edit.column);
                    let rename = format!("{} -> {}", edit.old_name, edit.new_name);
                    println!(
                        "{}",
                        cli_output::format_tsv_row([position.as_str(), rename.as_str()])
                    );
                }
            }
        }

        Commands::LspCodeActions { file, json } => {
            let code = read_kujo_source_for_parse(&file);
            let actions = lsp_code_actions::code_actions(&code);

            if json {
                if let Err(error) = cli_output::emit_json_array(&actions, |action| {
                    serde_json::json!({
                        "title": action.title,
                        "kind": action.kind,
                        "line": action.line,
                        "column": action.column,
                        "replacement": action.replacement,
                        "description": action.description,
                    })
                }) {
                    report_json_internal_error_and_exit(
                        format!("Failed to serialize code actions result: {}", error),
                        CliExitCode::InternalError,
                    );
                }
            } else {
                cli_output::emit_tsv_rows(actions.iter().map(|action| {
                    let position = format!("{}:{}:{}", file.display(), action.line, action.column);
                    [action.title.clone(), position, action.replacement.clone()]
                }));
            }
        }

        Commands::Lsp { deterministic_logs, request_timeout_ms } => {
            let exit_code = lsp_server::run_stdio_server(lsp_server::LspServerConfig {
                deterministic_logging: deterministic_logs,
                request_timeout_ms,
            });
            if 0 != exit_code {
                std::process::exit(exit_code);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_untrusted_network_destination_policy_defaults, cooperative_scheduler_timeout,
        is_known_cli_subcommand, reserved_external_alias_error, CapabilityArgs,
        DEFAULT_COOPERATIVE_SCHEDULER_TIMEOUT_MS,
    };
    use std::sync::Mutex;
    use std::time::Duration;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_scheduler_timeout_env<F>(value: Option<&str>, test_fn: F)
    where
        F: FnOnce(),
    {
        let _guard = ENV_LOCK.lock().expect("environment lock poisoned");

        match value {
            Some(raw) => std::env::set_var("KUJO_SCHEDULER_TIMEOUT_MS", raw),
            None => std::env::remove_var("KUJO_SCHEDULER_TIMEOUT_MS"),
        }

        test_fn();
        std::env::remove_var("KUJO_SCHEDULER_TIMEOUT_MS");
    }

    fn with_outbound_policy_env<F>(value: Option<&str>, test_fn: F)
    where
        F: FnOnce(),
    {
        let _guard = ENV_LOCK.lock().expect("environment lock poisoned");

        match value {
            Some(raw) => {
                std::env::set_var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV, raw)
            }
            None => std::env::remove_var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV),
        }

        test_fn();
        std::env::remove_var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV);
    }

    #[test]
    fn cooperative_scheduler_timeout_uses_default_when_unset() {
        with_scheduler_timeout_env(None, || {
            let timeout = cooperative_scheduler_timeout(None)
                .expect("default scheduler timeout should resolve successfully");
            assert_eq!(timeout, Duration::from_millis(DEFAULT_COOPERATIVE_SCHEDULER_TIMEOUT_MS));
        });
    }

    #[test]
    fn cooperative_scheduler_timeout_uses_env_when_cli_missing() {
        with_scheduler_timeout_env(Some("2345"), || {
            let timeout = cooperative_scheduler_timeout(None)
                .expect("env scheduler timeout should resolve successfully");
            assert_eq!(timeout, Duration::from_millis(2345));
        });
    }

    #[test]
    fn cooperative_scheduler_timeout_prefers_cli_over_env() {
        with_scheduler_timeout_env(Some("5000"), || {
            let timeout = cooperative_scheduler_timeout(Some(2500))
                .expect("cli scheduler timeout should resolve successfully");
            assert_eq!(timeout, Duration::from_millis(2500));
        });
    }

    #[test]
    fn cooperative_scheduler_timeout_rejects_cli_zero() {
        with_scheduler_timeout_env(Some("5000"), || {
            let error = cooperative_scheduler_timeout(Some(0))
                .expect_err("zero cli scheduler timeout should be rejected");
            assert_eq!(error, "Scheduler timeout must be greater than 0ms");
        });
    }

    #[test]
    fn cooperative_scheduler_timeout_falls_back_on_invalid_env() {
        with_scheduler_timeout_env(Some("invalid"), || {
            let timeout = cooperative_scheduler_timeout(None)
                .expect("invalid env value should fall back to default");
            assert_eq!(timeout, Duration::from_millis(DEFAULT_COOPERATIVE_SCHEDULER_TIMEOUT_MS));
        });
    }

    #[test]
    fn untrusted_network_defaults_set_deny_private_when_unset() {
        with_outbound_policy_env(None, || {
            let args = CapabilityArgs {
                untrusted: true,
                allow_net_client: true,
                ..CapabilityArgs::default()
            };
            apply_untrusted_network_destination_policy_defaults(&args);
            assert_eq!(
                std::env::var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV).ok(),
                Some("deny_private".to_string())
            );
        });
    }

    #[test]
    fn untrusted_network_defaults_do_not_override_explicit_policy() {
        with_outbound_policy_env(Some("allow_all"), || {
            let args = CapabilityArgs {
                untrusted: true,
                allow_net_client: true,
                ..CapabilityArgs::default()
            };
            apply_untrusted_network_destination_policy_defaults(&args);
            assert_eq!(
                std::env::var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV).ok(),
                Some("allow_all".to_string())
            );
        });
    }

    #[test]
    fn untrusted_ai_defaults_set_deny_private_when_unset() {
        with_outbound_policy_env(None, || {
            let args =
                CapabilityArgs { untrusted: true, allow_ai: true, ..CapabilityArgs::default() };
            apply_untrusted_network_destination_policy_defaults(&args);
            assert_eq!(
                std::env::var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV).ok(),
                Some("deny_private".to_string())
            );
        });
    }

    #[test]
    fn deny_private_net_forces_strict_destination_policy_in_trusted_mode() {
        with_outbound_policy_env(Some("allow_all"), || {
            let args = CapabilityArgs { deny_private_net: true, ..CapabilityArgs::default() };
            apply_untrusted_network_destination_policy_defaults(&args);
            assert_eq!(
                std::env::var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV).ok(),
                Some("deny_private".to_string())
            );
        });
    }

    #[test]
    fn untrusted_network_defaults_do_not_set_policy_for_trusted_mode() {
        with_outbound_policy_env(None, || {
            let args = CapabilityArgs {
                untrusted: false,
                allow_net_client: true,
                ..CapabilityArgs::default()
            };
            apply_untrusted_network_destination_policy_defaults(&args);
            assert!(std::env::var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV).is_err());
        });
    }

    #[test]
    fn known_cli_subcommand_list_includes_pack_and_run() {
        assert!(is_known_cli_subcommand("run"));
        assert!(is_known_cli_subcommand("pack"));
        assert!(is_known_cli_subcommand("ecosystem"));
        assert!(is_known_cli_subcommand("doctor"));
    }

    #[test]
    fn reserved_aliases_are_rejected_before_workflow_routing() {
        assert!(reserved_external_alias_error("run").is_some());
        assert!(reserved_external_alias_error("doctor").is_some());
        assert!(reserved_external_alias_error("kujo").is_some());
        assert!(reserved_external_alias_error("acme-tools").is_none());
    }
}
