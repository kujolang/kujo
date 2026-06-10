use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use kujo::ast::Stmt;
use kujo::compiler::Compiler;
use kujo::interpreter::{Environment, Interpreter, Value};
use kujo::lexer::{tokenize, Token};
use kujo::module::ModuleLoader;
use kujo::parser::Parser;
use kujo::serve_http::{run_static_server, ServeServerOptions};
use kujo::vm::VM;
use reqwest::blocking::Client;
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let counter = UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.push(format!("kujo_{}_{}_{}_{}", prefix, std::process::id(), nanos, counter));
    path
}

fn parse_program(source: &str) -> Vec<Stmt> {
    let tokens = tokenize(source).expect("benchmark source should tokenize");
    let mut parser = Parser::new(tokens);
    parser.parse()
}

fn build_large_lexer_source(statement_count: usize) -> String {
    let mut source = String::with_capacity(statement_count * 32);
    source.push_str("func add(a, b) { return a + b }\n");
    for i in 0..statement_count {
        source.push_str(&format!("value_{} := add({}, {})\n", i, i, i + 1));
    }
    source
}

fn build_many_tokens_source(statement_count: usize) -> String {
    let mut source = String::with_capacity(statement_count * 48);
    for i in 0..statement_count {
        source.push_str(&format!(
            "item_{} := {{\"left\": {}, \"right\": {}, \"sum\": {} + {}}}\n",
            i,
            i,
            i + 1,
            i,
            i + 1
        ));
    }
    source
}

fn build_deep_expression_source(depth: usize) -> String {
    let mut expr = "1".to_string();
    for _ in 0..depth {
        expr = format!("({} + 1)", expr);
    }
    format!("value := {}\n", expr)
}

fn build_runtime_workload_source() -> String {
    let mut source = String::new();
    source.push_str(
        "func fib(n) {\n\
         if n <= 1 {\n\
             return n\n\
         }\n\
         return fib(n - 1) + fib(n - 2)\n\
         }\n",
    );

    source.push_str("numbers := [");
    for i in 0..32 {
        if i > 0 {
            source.push_str(", ");
        }
        source.push_str(&(i + 1).to_string());
    }
    source.push_str("]\n");

    source.push_str(
        "total := 0\n\
         for i in range(32) {\n\
             total := total + numbers[i]\n\
         }\n\
         text := \"kujo\"\n\
         for i in range(40) {\n\
             text := text + \"-x\"\n\
         }\n\
         metrics := {\"sum\": total, \"fib\": fib(10), \"label\": text}\n\
         final_value := metrics[\"sum\"] + metrics[\"fib\"] + total\n",
    );

    source
}

fn bench_lexer(c: &mut Criterion) {
    let large_source = build_large_lexer_source(6_000);
    let many_tokens_source = build_many_tokens_source(3_500);

    let mut group = c.benchmark_group("lexer");
    group.bench_function("large_source", |b| {
        b.iter(|| {
            let tokens = tokenize(black_box(&large_source)).expect("large lexer source should lex");
            black_box(tokens.len());
        });
    });
    group.bench_function("many_tokens", |b| {
        b.iter(|| {
            let tokens =
                tokenize(black_box(&many_tokens_source)).expect("token-heavy source should lex");
            black_box(tokens.len());
        });
    });
    group.finish();
}

fn bench_parser(c: &mut Criterion) {
    let large_tokens =
        tokenize(&build_large_lexer_source(4_000)).expect("large parser source should tokenize");
    let deep_tokens =
        tokenize(&build_deep_expression_source(200)).expect("deep parser source should tokenize");

    let mut group = c.benchmark_group("parser");
    group.bench_function("large_file", |b| {
        b.iter_batched(
            || large_tokens.clone(),
            |tokens: Vec<Token>| {
                let mut parser = Parser::new(tokens);
                let output = parser.parse_with_diagnostics();
                black_box(output.stmts.len());
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("deep_expression", |b| {
        b.iter_batched(
            || deep_tokens.clone(),
            |tokens: Vec<Token>| {
                let mut parser = Parser::new(tokens);
                let output = parser.parse_with_diagnostics();
                black_box(output.stmts.len());
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_interpreter(c: &mut Criterion) {
    let runtime_source = build_runtime_workload_source();
    let stmts = parse_program(&runtime_source);

    let mut group = c.benchmark_group("interpreter");
    group.bench_function("loops_calls_recursion_strings_collections", |b| {
        b.iter_batched(
            Interpreter::new,
            |mut interpreter| {
                interpreter.eval_stmts(&stmts);
                black_box(interpreter.return_value.clone());
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_vm(c: &mut Criterion) {
    let runtime_source = build_runtime_workload_source();
    let stmts = parse_program(&runtime_source);
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&stmts).expect("runtime workload should compile");

    let mut group = c.benchmark_group("vm");
    group.bench_function("loops_calls_recursion_strings_collections", |b| {
        b.iter_batched(
            VM::new,
            |mut vm| {
                vm.set_jit_enabled(false);
                configure_vm_globals(&mut vm);
                let value =
                    vm.execute(chunk.clone()).expect("vm benchmark workload should execute");
                black_box(value);
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn configure_vm_globals(vm: &mut VM) {
    let globals = Arc::new(Mutex::new(Environment::new()));
    for builtin in Interpreter::get_builtin_names() {
        globals
            .lock()
            .expect("globals lock should be available")
            .set(builtin.to_string(), Value::NativeFunction(builtin.to_string()));
    }
    vm.set_globals(globals);
}

struct ModuleBenchmarkFixture {
    root_dir: PathBuf,
    entry_module: String,
}

struct DeepModuleChainBenchmarkFixture {
    root_dir: PathBuf,
    terminal_module: String,
}

struct DeepDottedModuleChainBenchmarkFixture {
    root_dir: PathBuf,
    terminal_module: String,
}

struct ImportHeavyNestedStartupBenchmarkFixture {
    root_dir: PathBuf,
    entry_module: String,
}

fn module_benchmark_fixture() -> &'static ModuleBenchmarkFixture {
    static FIXTURE: OnceLock<ModuleBenchmarkFixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let root_dir = unique_temp_dir("v1_perf_module_bench");
        fs::create_dir_all(&root_dir).expect("module benchmark root should be created");

        let module_count = 120usize;
        for index in 0..module_count {
            let module_name = format!("mod_{:03}", index);
            let module_path = root_dir.join(format!("{}.kujo", module_name));
            let body = if index == 0 {
                format!("export value := {}\n", index)
            } else {
                format!("import mod_{:03}\nexport value := {}\n", index - 1, index)
            };
            fs::write(module_path, body).expect("module benchmark file should be written");
        }

        let entry_module = "entry_module".to_string();
        let mut entry_source = String::new();
        for index in 0..module_count {
            entry_source.push_str(&format!("import mod_{:03}\n", index));
        }
        entry_source.push_str("export done := 1\n");
        fs::write(root_dir.join(format!("{}.kujo", entry_module)), entry_source)
            .expect("module benchmark entry file should be written");

        ModuleBenchmarkFixture { root_dir, entry_module }
    })
}

fn deep_module_chain_benchmark_fixture() -> &'static DeepModuleChainBenchmarkFixture {
    static FIXTURE: OnceLock<DeepModuleChainBenchmarkFixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let root_dir = unique_temp_dir("v1_perf_module_chain_bench");
        fs::create_dir_all(&root_dir).expect("deep module chain benchmark root should be created");

        let module_count = 24usize;
        let mut module_names = Vec::with_capacity(module_count);
        for index in 0..module_count {
            module_names.push(format!("chain_{index:03}"));
        }

        for index in 0..module_count {
            let module_path = root_dir.join(format!("{}.kujo", module_names[index]));
            let body = if index == 0 {
                "export depth := 0\n".to_string()
            } else {
                format!(
                    "from {} import depth\nexport depth := depth + 1\n",
                    module_names[index - 1]
                )
            };
            fs::write(module_path, body)
                .expect("deep module chain benchmark file should be written");
        }

        let terminal_module = module_names
            .last()
            .cloned()
            .expect("deep module chain should include a terminal module");
        DeepModuleChainBenchmarkFixture { root_dir, terminal_module }
    })
}

fn deep_dotted_module_chain_benchmark_fixture() -> &'static DeepDottedModuleChainBenchmarkFixture {
    static FIXTURE: OnceLock<DeepDottedModuleChainBenchmarkFixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let root_dir = unique_temp_dir("v1_perf_module_dotted_chain_bench");
        let nested_root = root_dir.join("src");
        fs::create_dir_all(&nested_root)
            .expect("deep dotted module chain benchmark root should be created");

        let module_count = 24usize;
        let mut module_names = Vec::with_capacity(module_count);
        for index in 0..module_count {
            module_names.push(format!("src.chain_{index:03}"));
        }

        for index in 0..module_count {
            let module_path = nested_root.join(format!("chain_{index:03}.kujo"));
            let body = if index == 0 {
                "export depth := 0\n".to_string()
            } else {
                format!(
                    "from {} import depth\nexport depth := depth + 1\n",
                    module_names[index - 1]
                )
            };
            fs::write(module_path, body)
                .expect("deep dotted module chain benchmark file should be written");
        }

        let terminal_module = module_names
            .last()
            .cloned()
            .expect("deep dotted module chain should include a terminal module");
        DeepDottedModuleChainBenchmarkFixture { root_dir, terminal_module }
    })
}

fn import_heavy_nested_startup_benchmark_fixture(
) -> &'static ImportHeavyNestedStartupBenchmarkFixture {
    static FIXTURE: OnceLock<ImportHeavyNestedStartupBenchmarkFixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let root_dir = unique_temp_dir("v1_perf_import_heavy_nested_startup_bench");
        let nested_root = root_dir.join("src").join("core");
        fs::create_dir_all(&nested_root)
            .expect("import-heavy nested startup benchmark root should be created");

        let module_count = 64usize;
        let mut entry_source = String::new();
        for index in 0..module_count {
            let symbol_name = format!("value_{index:03}");
            let module_name = format!("src.core.mod_{index:03}");
            let module_path = nested_root.join(format!("mod_{index:03}.kujo"));
            fs::write(module_path, format!("export {symbol_name} := {index}\n"))
                .expect("import-heavy nested module benchmark file should be written");
            entry_source.push_str(&format!("from {module_name} import {symbol_name}\n"));
        }
        entry_source.push_str("export ready := 1\n");

        let entry_module = "entry_nested_import_startup".to_string();
        fs::write(root_dir.join(format!("{}.kujo", entry_module)), entry_source)
            .expect("import-heavy nested startup benchmark entry file should be written");

        ImportHeavyNestedStartupBenchmarkFixture { root_dir, entry_module }
    })
}

fn bench_module_resolution(c: &mut Criterion) {
    let fixture = module_benchmark_fixture();
    let deep_chain_fixture = deep_module_chain_benchmark_fixture();
    let deep_dotted_chain_fixture = deep_dotted_module_chain_benchmark_fixture();
    let import_heavy_startup_fixture = import_heavy_nested_startup_benchmark_fixture();

    let mut group = c.benchmark_group("module_resolution");
    group.bench_function("many_small_modules_cold_loader", |b| {
        b.iter_batched(
            || {
                let mut loader = ModuleLoader::new();
                loader.add_search_path(&fixture.root_dir);
                loader
            },
            |mut loader| {
                let module =
                    loader.load_module(&fixture.entry_module).expect("module workload should load");
                black_box(module.exports.len());
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("deep_import_chain_cold_loader", |b| {
        b.iter_batched(
            || {
                let mut loader = ModuleLoader::new();
                loader.add_search_path(&deep_chain_fixture.root_dir);
                loader
            },
            |mut loader| {
                let depth = loader
                    .get_symbol(&deep_chain_fixture.terminal_module, "depth")
                    .expect("deep import chain workload should load");
                black_box(depth);
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("deep_dotted_import_chain_cold_loader", |b| {
        b.iter_batched(
            || {
                let mut loader = ModuleLoader::new();
                loader.add_search_path(&deep_dotted_chain_fixture.root_dir);
                loader
            },
            |mut loader| {
                let depth = loader
                    .get_symbol(&deep_dotted_chain_fixture.terminal_module, "depth")
                    .expect("deep dotted import chain workload should load");
                black_box(depth);
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("import_heavy_nested_dotted_startup_cold_loader", |b| {
        b.iter_batched(
            || {
                let mut loader = ModuleLoader::new();
                loader.add_search_path(&import_heavy_startup_fixture.root_dir);
                loader
            },
            |mut loader| {
                let module = loader
                    .load_module(&import_heavy_startup_fixture.entry_module)
                    .expect("import-heavy nested startup workload should load");
                black_box(module.exports.len());
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("import_heavy_nested_dotted_cached_lookup_warm_loader", |b| {
        b.iter_batched(
            || {
                let mut loader = ModuleLoader::new();
                loader.add_search_path(&import_heavy_startup_fixture.root_dir);
                loader
                    .load_module(&import_heavy_startup_fixture.entry_module)
                    .expect("import-heavy nested startup warm workload should preload");
                loader
            },
            |mut loader| {
                let ready = loader
                    .get_symbol(&import_heavy_startup_fixture.entry_module, "ready")
                    .expect("import-heavy nested startup warm lookup should succeed");
                black_box(ready);
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

struct ServeBenchmarkFixture {
    base_url: String,
    small_path: String,
    large_path: String,
    _root_dir: PathBuf,
    client: Client,
}

fn available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("ephemeral port binding should succeed")
        .local_addr()
        .expect("ephemeral listener should have local addr")
        .port()
}

fn serve_benchmark_fixture() -> &'static ServeBenchmarkFixture {
    static FIXTURE: OnceLock<ServeBenchmarkFixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let root_dir = unique_temp_dir("v1_perf_serve_bench");
        fs::create_dir_all(&root_dir).expect("serve benchmark root should be created");

        let small_path = "small.txt".to_string();
        let large_path = "large.bin".to_string();
        fs::write(root_dir.join(&small_path), "kujo benchmark static server payload\n")
            .expect("small benchmark payload should be written");
        fs::write(root_dir.join(&large_path), vec![b'x'; 2 * 1024 * 1024])
            .expect("large benchmark payload should be written");

        let port = available_port();
        let server_dir = root_dir.clone();
        thread::spawn(move || {
            let options = ServeServerOptions {
                index: "index.html".to_string(),
                hardened: true,
                cache_max_age: Some(60),
                access_log: false,
                tls_cert: None,
                tls_key: None,
                max_request_line_bytes: 8192,
                max_header_bytes: 16384,
                max_header_count: 100,
                max_request_body_bytes: 1_048_576,
                read_timeout: Duration::from_millis(500),
                write_timeout: Duration::from_millis(500),
                max_connections: 128,
            };
            let _ = run_static_server(server_dir, "127.0.0.1".to_string(), port, options);
        });

        let base_url = format!("http://127.0.0.1:{}", port);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("benchmark client should build");

        let health_url = format!("{}/{}", base_url, small_path);
        let mut ready = false;
        for _ in 0..100 {
            if let Ok(response) = client.get(&health_url).send() {
                if response.status().is_success() {
                    ready = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert!(ready, "serve benchmark server failed to become ready");

        ServeBenchmarkFixture { base_url, small_path, large_path, _root_dir: root_dir, client }
    })
}

fn bench_static_server(c: &mut Criterion) {
    let fixture = serve_benchmark_fixture();
    let small_url = format!("{}/{}", fixture.base_url, fixture.small_path);
    let large_url = format!("{}/{}", fixture.base_url, fixture.large_path);

    let mut group = c.benchmark_group("serve_http");
    group.sample_size(20);
    group.bench_function("small_file_get", |b| {
        b.iter(|| {
            let response = fixture
                .client
                .get(black_box(&small_url))
                .send()
                .expect("small-file request should succeed");
            let status = response.status();
            let bytes = response.bytes().expect("small-file response body should be readable");
            black_box((status.as_u16(), bytes.len()));
        });
    });
    group.bench_function("large_file_get", |b| {
        b.iter(|| {
            let response = fixture
                .client
                .get(black_box(&large_url))
                .send()
                .expect("large-file request should succeed");
            let status = response.status();
            let bytes = response.bytes().expect("large-file response body should be readable");
            black_box((status.as_u16(), bytes.len()));
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_lexer,
    bench_parser,
    bench_interpreter,
    bench_vm,
    bench_module_resolution,
    bench_static_server
);
criterion_main!(benches);
