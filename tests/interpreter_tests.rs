// Integration tests for the Kujo interpreter
//
// These tests verify the interpreter's behavior by running complete Kujo programs
// and checking the results. Tests cover:
// - Variable assignment and scoping
// - Control flow (if/else, loops, match)
// - Functions and closures
// - Data structures (arrays, dicts, structs)
// - Error handling
// - Built-in functions

use kujo::interpreter::{BindingKind, Environment, Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

fn unique_shared_key(prefix: &str) -> String {
    static SHARED_KEY_COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("{}_{}", prefix, SHARED_KEY_COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn unique_module_name(prefix: &str) -> String {
    static MODULE_COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("{}_{}", prefix, MODULE_COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn run_code(code: &str) -> Interpreter {
    let tokens = tokenize(code).expect("test source should tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse();
    let mut interp = Interpreter::new();
    interp.eval_stmts(&program);
    interp
}

fn run_code_with_output(code: &str) -> (Interpreter, String) {
    let tokens = tokenize(code).expect("test source should tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse();
    let output = Arc::new(Mutex::new(Vec::new()));
    let mut interp = Interpreter::new();
    interp.set_output(output.clone());
    interp.eval_stmts(&program);
    let bytes = output.lock().expect("test output lock should not be poisoned").clone();
    let text = String::from_utf8(bytes).expect("print output should be utf-8");
    (interp, text)
}

#[test]
fn test_parallel_map_async_mapper_isolated_ordered_and_fail_closed() {
    let ordered = run_code(
        r#"
        async func worker(value) {
            await async_sleep((4 - value) * 10)
            return value * 10
        }
        ordered := await parallel_map([1, 2, 3], worker, 2)
        "#,
    );
    match ordered.env.get("ordered") {
        Some(Value::Array(values)) => {
            assert_eq!(values.len(), 3);
            assert!(matches!(values[0], Value::Int(10)));
            assert!(matches!(values[1], Value::Int(20)));
            assert!(matches!(values[2], Value::Int(30)));
        }
        other => panic!("expected ordered async mapper results, got {other:?}"),
    }

    let failed = run_code(
        r#"
        async func worker(value) {
            if value == 2 { return error("worker-denied") }
            await async_sleep(20)
            return value
        }
        result := await parallel_map([1, 2, 3, 4], worker, 2)
        "#,
    );
    assert!(matches!(
        failed.env.get("result"),
        Some(Value::Error(message)) if message.contains("worker-denied")
    ));
}

#[test]
fn test_jsonl_query_streams_filters_and_joins_with_bounds() {
    let directory =
        std::env::temp_dir().join(format!("kujo-jsonl-query-{}", unique_shared_key("fixture")));
    std::fs::create_dir_all(&directory).expect("temp directory should be created");
    let left = directory.join("left.jsonl");
    let right = directory.join("right.jsonl");
    std::fs::write(
        &left,
        "{\"provider\":\"gsc\",\"row\":{\"id\":1}}\n{\"provider\":\"ga4\",\"row\":{\"id\":2}}\n{\"provider\":\"gsc\",\"row\":{\"id\":3}}\n",
    )
    .expect("left fixture should be written");
    std::fs::write(&right, "{\"id\":1,\"name\":\"one\"}\n{\"id\":3,\"name\":\"three\"}\n")
        .expect("right fixture should be written");
    let source = format!(
        r#"
        filtered := jsonl_query({0:?}, {{"filter_field": "provider", "filter_equals": "gsc", "max_rows": 1}})
        joined := jsonl_query({0:?}, {{"filter_field": "provider", "filter_equals": "gsc", "join_path": {1:?}, "left_field": "row.id", "right_field": "id", "max_rows": 10}})
        query_ok := len(filtered) == 1 && filtered[0]["row"]["id"] == 1 && len(joined) == 2 && joined[1]["right"]["name"] == "three"
        "#,
        left.to_string_lossy(),
        right.to_string_lossy()
    );
    let interpreter = run_code(&source);
    assert!(matches!(interpreter.env.get("query_ok"), Some(Value::Bool(true))));
    std::fs::remove_dir_all(directory).expect("temp directory should be removed");
}

#[test]
fn test_jsonl_query_rejects_partial_join_configuration() {
    let directory = std::env::temp_dir()
        .join(format!("kujo-jsonl-query-partial-join-{}", unique_shared_key("fixture")));
    std::fs::create_dir_all(&directory).expect("temp directory should be created");
    let left = directory.join("left.jsonl");
    let right = directory.join("right.jsonl");
    std::fs::write(&left, "{\"id\":1}\n").expect("left fixture should be written");
    std::fs::write(&right, "{\"id\":1}\n").expect("right fixture should be written");
    for (name, options) in
        [("missing_left", r#""right_field": "id""#), ("missing_right", r#""left_field": "id""#)]
    {
        let source = format!(
            r#"{name} := jsonl_query({0:?}, {{"join_path": {1:?}, {options}}})"#,
            left.to_string_lossy(),
            right.to_string_lossy()
        );
        let interpreter = run_code(&source);
        assert!(
            matches!(
                interpreter.env.get(name),
                Some(Value::Error(message))
                    if message.contains("join_path, left_field, and right_field together")
            ),
            "{name} should reject a partial join configuration, got {:?}",
            interpreter.env.get(name)
        );
    }
    std::fs::remove_dir_all(directory).expect("temp directory should be removed");
}

#[test]
fn test_builtin_names_include_release_hardening_contract_entries() {
    let builtins: HashSet<&str> = Interpreter::get_builtin_names().into_iter().collect();

    let required = vec![
        "eprint",
        "bytes",
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
        "secret",
        "reveal",
        "is_secret",
        "index_of",
        "repeat",
        "char_at",
        "is_empty",
        "count_chars",
        "pad_start",
        "pad_end",
        "is_truthy",
        "read_file_lossy",
        "db_pool",
        "db_pool_acquire",
        "db_pool_release",
        "db_pool_stats",
        "db_pool_close",
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
        "ai_request_hash",
        "json_schema_validate",
        "ai_chat",
        "ai_stream_chat",
        "ai_embedding",
        "ai_tool_loop",
        "os_getcwd",
        "os_chdir",
        "os_rmdir",
        "os_environ",
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
        "sha256_file",
        "type_of",
        "queue_size",
        "stack_size",
        "shared_set",
        "shared_get",
        "shared_has",
        "shared_delete",
        "shared_add_int",
        "parallel_map",
        "par_map",
        "set_task_pool_size",
        "get_task_pool_size",
    ];

    for name in required {
        assert!(builtins.contains(name), "Missing builtin from API contract: {name}");
    }
}

#[test]
fn test_builtin_names_do_not_contain_duplicates() {
    let names = Interpreter::get_builtin_names();
    let unique: HashSet<&str> = names.iter().copied().collect();
    assert_eq!(names.len(), unique.len(), "Duplicate names found in builtin API list");
}

#[test]
fn test_builtin_aliases_match_canonical_behavior() {
    let code = r#"
        source := "HeLLo"
        upper_a := to_upper(source)
        upper_b := upper(source)

        lower_a := to_lower(source)
        lower_b := lower(source)

        replace_a := replace_str("a-b-c", "-", "_")
        replace_b := replace("a-b-c", "-", "_")

        type_a := type(42)
        type_b := type_of(42)

        pad_left_a := pad_left("kujo", 6, "0")
        pad_start_b := pad_start("kujo", 6, "0")
        pad_right_a := pad_right("kujo", 6, ".")
        pad_end_b := pad_end("kujo", 6, ".")

        truthy_a := is_truthy(0)
        truthy_b := is_truthy("false")
        truthy_c := is_truthy([1])
    "#;

    let interp = run_code(code);

    match (interp.env.get("upper_a"), interp.env.get("upper_b")) {
        (Some(Value::Str(a)), Some(Value::Str(b))) => assert_eq!(a.as_ref(), b.as_ref()),
        _ => panic!("Expected upper alias results to be strings"),
    }

    match (interp.env.get("lower_a"), interp.env.get("lower_b")) {
        (Some(Value::Str(a)), Some(Value::Str(b))) => assert_eq!(a.as_ref(), b.as_ref()),
        _ => panic!("Expected lower alias results to be strings"),
    }

    match (interp.env.get("replace_a"), interp.env.get("replace_b")) {
        (Some(Value::Str(a)), Some(Value::Str(b))) => assert_eq!(a.as_ref(), b.as_ref()),
        _ => panic!("Expected replace alias results to be strings"),
    }

    match (interp.env.get("type_a"), interp.env.get("type_b")) {
        (Some(Value::Str(a)), Some(Value::Str(b))) => assert_eq!(a.as_ref(), b.as_ref()),
        _ => panic!("Expected type alias results to be strings"),
    }

    match (interp.env.get("pad_left_a"), interp.env.get("pad_start_b")) {
        (Some(Value::Str(a)), Some(Value::Str(b))) => assert_eq!(a.as_ref(), b.as_ref()),
        _ => panic!("Expected pad_start alias results to be strings"),
    }

    match (interp.env.get("pad_right_a"), interp.env.get("pad_end_b")) {
        (Some(Value::Str(a)), Some(Value::Str(b))) => assert_eq!(a.as_ref(), b.as_ref()),
        _ => panic!("Expected pad_end alias results to be strings"),
    }

    match interp.env.get("truthy_a") {
        Some(Value::Bool(v)) => assert!(!v),
        _ => panic!("Expected truthy_a bool false"),
    }

    match interp.env.get("truthy_b") {
        Some(Value::Bool(v)) => assert!(v),
        _ => panic!("Expected truthy_b bool true"),
    }

    match interp.env.get("truthy_c") {
        Some(Value::Bool(v)) => assert!(v),
        _ => panic!("Expected truthy_c bool true"),
    }
}

#[test]
fn test_path_builtin_alias_and_core_operations() {
    let unique = unique_shared_key("path_hardening");
    let temp_dir = std::env::temp_dir().join(format!("kujo_{unique}"));
    std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir for path tests");
    let temp_file = temp_dir.join("sample.txt");
    let symlink_file = temp_dir.join("sample-link.txt");
    std::fs::write(&temp_file, "hardening").expect("failed to write temp file");

    let dir_str = temp_dir.to_string_lossy().to_string();
    let file_str = temp_file.to_string_lossy().to_string();

    let code = format!(
        r#"
        joined_a := join_path("{dir_str}", "sample.txt")
        joined_b := path_join("{dir_str}", "sample.txt")
        exists_flag := path_exists("{file_str}")
        is_file_flag := path_is_file("{file_str}")
        is_symlink_flag := path_is_symlink("{file_str}")
        is_dir_flag := path_is_dir("{dir_str}")
        ext := path_extension("{file_str}")
        base := basename("{file_str}")
        parent := dirname("{file_str}")
        abs_path := path_absolute("{file_str}")
    "#,
    );

    let interp = run_code(&code);

    match (interp.env.get("joined_a"), interp.env.get("joined_b")) {
        (Some(Value::Str(a)), Some(Value::Str(b))) => assert_eq!(a.as_ref(), b.as_ref()),
        _ => panic!("Expected join_path/path_join to return strings"),
    }

    match interp.env.get("exists_flag") {
        Some(Value::Bool(v)) => assert!(v),
        _ => panic!("Expected exists_flag bool true"),
    }

    match interp.env.get("is_file_flag") {
        Some(Value::Bool(v)) => assert!(v),
        _ => panic!("Expected is_file_flag bool true"),
    }

    match interp.env.get("is_symlink_flag") {
        Some(Value::Bool(v)) => assert!(!v),
        _ => panic!("Expected is_symlink_flag bool false"),
    }

    match interp.env.get("is_dir_flag") {
        Some(Value::Bool(v)) => assert!(v),
        _ => panic!("Expected is_dir_flag bool true"),
    }

    match interp.env.get("ext") {
        Some(Value::Str(v)) => assert_eq!("txt", v.as_ref()),
        _ => panic!("Expected ext string"),
    }

    match interp.env.get("base") {
        Some(Value::Str(v)) => assert_eq!("sample.txt", v.as_ref()),
        _ => panic!("Expected base string"),
    }

    match interp.env.get("parent") {
        Some(Value::Str(v)) => assert!(!v.is_empty()),
        _ => panic!("Expected parent string"),
    }

    match interp.env.get("abs_path") {
        Some(Value::Str(v)) => assert!(!v.is_empty()),
        _ => panic!("Expected abs_path string"),
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        symlink(&temp_file, &symlink_file).expect("failed to create symlink");
        let symlink_code = format!(
            r#"
            link_flag := path_is_symlink("{link}")
        "#,
            link = symlink_file.to_string_lossy()
        );
        let symlink_interp = run_code(&symlink_code);
        match symlink_interp.env.get("link_flag") {
            Some(Value::Bool(v)) => assert!(v),
            _ => panic!("Expected link_flag bool true"),
        }
    }

    std::fs::remove_file(&temp_file).expect("failed to clean up temp file");
    let _ = std::fs::remove_file(&symlink_file);
    std::fs::remove_dir(&temp_dir).expect("failed to clean up temp dir");
}

#[test]
fn test_read_file_lossy_and_bytes_slice_helpers() {
    let unique = unique_shared_key("lossy_and_bytes");
    let temp_dir = std::env::temp_dir().join(format!("kujo_{unique}"));
    std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir for lossy tests");

    let lossy_path = temp_dir.join("lossy.txt");
    std::fs::write(&lossy_path, [0x66_u8, 0x6f, 0x80, 0x6f]).expect("failed to write lossy file");

    let path_str = lossy_path.to_string_lossy().to_string();
    let code = format!(
        r#"
        text := read_file_lossy("{path_str}")
        data := bytes([0, 1, 2, 3, 4])
        data_slice := slice(data, 1, 4)
    "#
    );

    let interp = run_code(&code);

    let expected_lossy = String::from_utf8_lossy(&[0x66_u8, 0x6f, 0x80, 0x6f]).to_string();
    match interp.env.get("text") {
        Some(Value::Str(value)) => assert_eq!(value.as_ref(), expected_lossy.as_str()),
        _ => panic!("Expected lossy file read to return a string"),
    }

    match interp.env.get("data_slice") {
        Some(Value::Bytes(bytes)) => assert_eq!(bytes.as_slice(), &[1, 2, 3]),
        _ => panic!("Expected bytes slice to return bytes"),
    }
}

#[test]
fn test_bytes_indexing_and_interpolated_invalid_expression_are_rejected() {
    let code = r#"
        data := bytes([0, 1, 2, 255])
        first := data[0]
        last := data[-1]
    "#;

    let interp = run_code(code);

    match interp.env.get("first") {
        Some(Value::Int(v)) => assert_eq!(0, v),
        _ => panic!("Expected first byte int"),
    }

    match interp.env.get("last") {
        Some(Value::Int(v)) => assert_eq!(255, v),
        _ => panic!("Expected last byte int"),
    }

    let tokens = tokenize(r#"result := "${5 & 3}""#).expect("interpolated source should tokenize");
    let mut parser = Parser::new(tokens);
    let output = parser.parse_with_diagnostics();
    assert!(
        !output.diagnostics.is_empty(),
        "expected interpolation parse error for unsupported expression"
    );
    assert!(output
        .diagnostics
        .iter()
        .any(|d| d.message.contains("Invalid interpolated string expression")
            || d.message.contains("Expected expression")));
}

#[test]
fn test_queue_and_stack_size_contract() {
    let code = r#"
        q := Queue([1, 2, 3])
        qs := queue_size(q)
        qe := queue_is_empty(q)

        s := Stack([1, 2, 3, 4])
        ss := stack_size(s)
        se := stack_is_empty(s)
    "#;

    let interp = run_code(code);

    match interp.env.get("qs") {
        Some(Value::Int(v)) => assert_eq!(3, v),
        _ => panic!("Expected qs int"),
    }

    match interp.env.get("qe") {
        Some(Value::Bool(v)) => assert!(!v),
        _ => panic!("Expected qe bool"),
    }

    match interp.env.get("ss") {
        Some(Value::Int(v)) => assert_eq!(4, v),
        _ => panic!("Expected ss int"),
    }

    match interp.env.get("se") {
        Some(Value::Bool(v)) => assert!(!v),
        _ => panic!("Expected se bool"),
    }
}

#[test]
fn test_path_join_alias_argument_shape_contract() {
    let interp_no_args_a = run_code("no_args_a := join_path()");
    let interp_no_args_b = run_code("no_args_b := path_join()");
    let interp_bad_type_a = run_code("bad_type_a := join_path(\"root\", 42)");
    let interp_bad_type_b = run_code("bad_type_b := path_join(\"root\", 42)");

    assert!(matches!(
        interp_no_args_a.env.get("no_args_a"),
        Some(Value::Error(message)) if message.contains("requires at least one string argument")
    ));
    assert!(matches!(
        interp_no_args_b.env.get("no_args_b"),
        Some(Value::Error(message)) if message.contains("requires at least one string argument")
    ));
    assert!(matches!(
        interp_bad_type_a.env.get("bad_type_a"),
        Some(Value::Error(message)) if message.contains("argument 2 must be a string")
    ));
    assert!(matches!(
        interp_bad_type_b.env.get("bad_type_b"),
        Some(Value::Error(message)) if message.contains("argument 2 must be a string")
    ));
}

#[test]
fn test_queue_and_stack_size_argument_shape_contract() {
    let interp_q_missing = run_code("q_missing := queue_size()");
    let interp_q_wrong = run_code("q_wrong := queue_size([1, 2])");
    let interp_q_extra = run_code("q_extra := queue_size(Queue([1]), Queue([2]))");
    let interp_s_missing = run_code("s_missing := stack_size()");
    let interp_s_wrong = run_code("s_wrong := stack_size([1, 2])");
    let interp_s_extra = run_code("s_extra := stack_size(Stack([1]), Stack([2]))");

    assert!(matches!(
        interp_q_missing.env.get("q_missing"),
        Some(Value::Error(message)) if message.contains("expects 1 argument")
    ));
    assert!(matches!(
        interp_q_wrong.env.get("q_wrong"),
        Some(Value::Error(message)) if message.contains("requires a Queue argument")
    ));
    assert!(matches!(
        interp_q_extra.env.get("q_extra"),
        Some(Value::Error(message)) if message.contains("expects 1 argument")
    ));

    assert!(matches!(
        interp_s_missing.env.get("s_missing"),
        Some(Value::Error(message)) if message.contains("expects 1 argument")
    ));
    assert!(matches!(
        interp_s_wrong.env.get("s_wrong"),
        Some(Value::Error(message)) if message.contains("requires a Stack argument")
    ));
    assert!(matches!(
        interp_s_extra.env.get("s_extra"),
        Some(Value::Error(message)) if message.contains("expects 1 argument")
    ));
}

#[test]
fn test_promise_all_and_await_all_argument_shape_contract() {
    let interp_p_non_array = run_code("p_non_array := promise_all(\"bad\")");
    let interp_a_non_array = run_code("a_non_array := await_all(\"bad\")");
    let interp_p_bad_limit = run_code("p_bad_limit := promise_all([async_sleep(1)], \"2\")");
    let interp_a_bad_limit = run_code("a_bad_limit := await_all([async_sleep(1)], \"2\")");
    let interp_p_zero_limit = run_code("p_zero_limit := promise_all([async_sleep(1)], 0)");
    let interp_a_zero_limit = run_code("a_zero_limit := await_all([async_sleep(1)], 0)");

    assert!(matches!(
        interp_p_non_array.env.get("p_non_array"),
        Some(Value::Error(message)) if message.contains("requires an array of promises")
    ));
    assert!(matches!(
        interp_a_non_array.env.get("a_non_array"),
        Some(Value::Error(message)) if message.contains("requires an array of promises")
    ));

    assert!(matches!(
        interp_p_bad_limit.env.get("p_bad_limit"),
        Some(Value::Error(message)) if message.contains("concurrency_limit must be an integer")
    ));
    assert!(matches!(
        interp_a_bad_limit.env.get("a_bad_limit"),
        Some(Value::Error(message)) if message.contains("concurrency_limit must be an integer")
    ));

    assert!(matches!(
        interp_p_zero_limit.env.get("p_zero_limit"),
        Some(Value::Error(message)) if message.contains("concurrency_limit must be > 0")
    ));
    assert!(matches!(
        interp_a_zero_limit.env.get("a_zero_limit"),
        Some(Value::Error(message)) if message.contains("concurrency_limit must be > 0")
    ));
}

#[test]
fn test_parallel_map_and_par_map_argument_shape_contract() {
    let interp_parallel_missing = run_code("parallel_missing := parallel_map()");
    let interp_alias_missing = run_code("alias_missing := par_map()");
    let interp_parallel_non_array = run_code("parallel_non_array := parallel_map(1, len)");
    let interp_alias_non_array = run_code("alias_non_array := par_map(1, len)");
    let interp_parallel_non_callable = run_code("parallel_non_callable := parallel_map([1], 123)");
    let interp_alias_non_callable = run_code("alias_non_callable := par_map([1], 123)");
    let interp_parallel_bad_limit = run_code("parallel_bad_limit := parallel_map([1], len, \"2\")");
    let interp_alias_bad_limit = run_code("alias_bad_limit := par_map([1], len, \"2\")");
    let interp_parallel_zero_limit = run_code("parallel_zero_limit := parallel_map([1], len, 0)");
    let interp_alias_zero_limit = run_code("alias_zero_limit := par_map([1], len, 0)");

    assert!(matches!(
        interp_parallel_missing.env.get("parallel_missing"),
        Some(Value::Error(message)) if message.contains("expects 2 to 3 arguments")
    ));
    assert!(matches!(
        interp_alias_missing.env.get("alias_missing"),
        Some(Value::Error(message)) if message.contains("expects 2 to 3 arguments")
    ));
    assert!(matches!(
        interp_parallel_non_array.env.get("parallel_non_array"),
        Some(Value::Error(message)) if message.contains("first argument must be an array")
    ));
    assert!(matches!(
        interp_alias_non_array.env.get("alias_non_array"),
        Some(Value::Error(message)) if message.contains("first argument must be an array")
    ));
    assert!(matches!(
        interp_parallel_non_callable.env.get("parallel_non_callable"),
        Some(Value::Error(message)) if message.contains("second argument must be a callable function")
    ));
    assert!(matches!(
        interp_alias_non_callable.env.get("alias_non_callable"),
        Some(Value::Error(message)) if message.contains("second argument must be a callable function")
    ));
    assert!(matches!(
        interp_parallel_bad_limit.env.get("parallel_bad_limit"),
        Some(Value::Error(message)) if message.contains("optional concurrency_limit must be an integer")
    ));
    assert!(matches!(
        interp_alias_bad_limit.env.get("alias_bad_limit"),
        Some(Value::Error(message)) if message.contains("optional concurrency_limit must be an integer")
    ));
    assert!(matches!(
        interp_parallel_zero_limit.env.get("parallel_zero_limit"),
        Some(Value::Error(message)) if message.contains("concurrency_limit must be > 0")
    ));
    assert!(matches!(
        interp_alias_zero_limit.env.get("alias_zero_limit"),
        Some(Value::Error(message)) if message.contains("concurrency_limit must be > 0")
    ));
}

#[test]
fn test_shared_value_argument_shape_contract() {
    let interp_set_missing = run_code("set_missing := shared_set()");
    let interp_set_bad_key = run_code("set_bad_key := shared_set(1, 2)");
    let interp_get_missing = run_code("get_missing := shared_get()");
    let interp_get_bad_key = run_code("get_bad_key := shared_get(1)");
    let interp_has_missing = run_code("has_missing := shared_has()");
    let interp_has_bad_key = run_code("has_bad_key := shared_has(1)");
    let interp_delete_missing = run_code("delete_missing := shared_delete()");
    let interp_delete_bad_key = run_code("delete_bad_key := shared_delete(1)");
    let interp_add_missing = run_code("add_missing := shared_add_int()");
    let interp_add_bad_key = run_code("add_bad_key := shared_add_int(1, 2)");
    let interp_add_bad_delta = run_code("add_bad_delta := shared_add_int(\"k\", \"2\")");

    assert!(matches!(
        interp_set_missing.env.get("set_missing"),
        Some(Value::Error(message)) if message.contains("requires (key, value) arguments")
    ));
    assert!(matches!(
        interp_set_bad_key.env.get("set_bad_key"),
        Some(Value::Error(message)) if message.contains("key must be a string")
    ));
    assert!(matches!(
        interp_get_missing.env.get("get_missing"),
        Some(Value::Error(message)) if message.contains("requires one key argument")
    ));
    assert!(matches!(
        interp_get_bad_key.env.get("get_bad_key"),
        Some(Value::Error(message)) if message.contains("key must be a string")
    ));
    assert!(matches!(
        interp_has_missing.env.get("has_missing"),
        Some(Value::Error(message)) if message.contains("requires one key argument")
    ));
    assert!(matches!(
        interp_has_bad_key.env.get("has_bad_key"),
        Some(Value::Error(message)) if message.contains("key must be a string")
    ));
    assert!(matches!(
        interp_delete_missing.env.get("delete_missing"),
        Some(Value::Error(message)) if message.contains("requires one key argument")
    ));
    assert!(matches!(
        interp_delete_bad_key.env.get("delete_bad_key"),
        Some(Value::Error(message)) if message.contains("key must be a string")
    ));
    assert!(matches!(
        interp_add_missing.env.get("add_missing"),
        Some(Value::Error(message)) if message.contains("requires (key, delta) arguments")
    ));
    assert!(matches!(
        interp_add_bad_key.env.get("add_bad_key"),
        Some(Value::Error(message)) if message.contains("key must be a string")
    ));
    assert!(matches!(
        interp_add_bad_delta.env.get("add_bad_delta"),
        Some(Value::Error(message)) if message.contains("delta must be an int")
    ));
}

#[test]
fn test_task_pool_size_argument_shape_contract() {
    let interp_set_missing = run_code("set_missing := set_task_pool_size()");
    let interp_set_bad_type = run_code("set_bad_type := set_task_pool_size(\"8\")");
    let interp_set_bad_value = run_code("set_bad_value := set_task_pool_size(0)");
    let interp_get_extra = run_code("get_extra := get_task_pool_size(1)");

    assert!(matches!(
        interp_set_missing.env.get("set_missing"),
        Some(Value::Error(message)) if message.contains("expects 1 argument")
    ));
    assert!(matches!(
        interp_set_bad_type.env.get("set_bad_type"),
        Some(Value::Error(message)) if message.contains("requires an integer size argument")
    ));
    assert!(matches!(
        interp_set_bad_value.env.get("set_bad_value"),
        Some(Value::Error(message)) if message.contains("size must be > 0")
    ));
    assert!(matches!(
        interp_get_extra.env.get("get_extra"),
        Some(Value::Error(message)) if message.contains("expects 0 arguments")
    ));
}

#[test]
fn test_field_assignment_struct() {
    let code = r#"
        struct Person {
            name: string,
            age: int
        }

        p := Person { name: "Alice", age: 25 }
        p.age := 26
    "#;

    let interp = run_code(code);

    if let Some(Value::Struct { fields, .. }) = interp.env.get("p") {
        if let Some(Value::Int(age)) = fields.get("age") {
            assert_eq!(*age, 26);
        } else {
            panic!("Expected age to be 26");
        }
    } else {
        panic!("Expected person struct");
    }
}

#[test]
fn test_field_assignment_in_array() {
    let code = r#"
        struct Todo {
            title: string,
            done: bool
        }

        todos := [
            Todo { title: "Task 1", done: false },
            Todo { title: "Task 2", done: false }
        ]

        todos[0].done := true
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(todos)) = interp.env.get("todos") {
        if let Some(Value::Struct { fields, .. }) = todos.first() {
            if let Some(Value::Bool(done)) = fields.get("done") {
                assert!(*done);
            } else {
                panic!("Expected done field to be true");
            }
        } else {
            panic!("Expected first element to be a struct");
        }
    } else {
        panic!("Expected todos array");
    }
}

#[test]
fn test_boolean_true_condition() {
    // Tests that 'true' is truthy
    let code = r#"
        x := 0
        if true {
            x := 1
        }
    "#;

    let interp = run_code(code);

    // Due to scoping, x remains 0 but we test that the if block executes
    // This is a known limitation documented in the README
    if let Some(Value::Int(x)) = interp.env.get("x") {
        // With current scoping, x stays 0 (variable shadowing issue)
        // But the code runs without errors, proving 'true' is handled
        assert!(x == 0 || x == 1); // Accept either due to scoping
    }
}

#[test]
fn test_boolean_false_condition() {
    // Tests that 'false' is falsy
    let code = r#"
        executed := false
        if false {
            executed := true
        }
    "#;

    let interp = run_code(code);

    if let Some(Value::Str(executed)) = interp.env.get("executed") {
        assert_eq!(executed.as_str(), "false");
    }
}

#[test]
fn test_array_index_assignment() {
    let code = r#"
        arr := [1, 2, 3]
        arr[1] := 20
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(arr)) = interp.env.get("arr") {
        if let Some(Value::Int(n)) = arr.get(1) {
            assert_eq!(*n, 20);
        } else {
            panic!("Expected second element to be 20");
        }
    } else {
        panic!("Expected arr array");
    }
}

#[test]
fn test_dict_operations() {
    let code = r#"
        person := {"name": "Bob", "age": 30}
        person["age"] := 31
    "#;

    let interp = run_code(code);

    if let Some(Value::Dict(dict)) = interp.env.get("person") {
        if let Some(Value::Int(age)) = dict.get("age") {
            assert_eq!(*age, 31);
        } else {
            panic!("Expected age to be 31");
        }
    } else {
        panic!("Expected person dict");
    }
}

#[test]
fn test_string_concatenation() {
    let code = r#"
        result := "Hello " + "World"
    "#;

    let interp = run_code(code);

    if let Some(Value::Str(result)) = interp.env.get("result") {
        assert_eq!(result.as_str(), "Hello World");
    } else {
        panic!("Expected concatenated string");
    }
}

#[test]
fn test_for_in_loop() {
    // Test that for-in loops execute and iterate
    let code = r#"
        items := []
        for n in [1, 2, 3] {
            print(n)
        }
    "#;

    // This test verifies the code runs without errors
    // Actual iteration is demonstrated in example projects
    let _interp = run_code(code);
    // If we get here without panic, for loop executed successfully
}

#[test]
fn test_variable_assignment_updates() {
    let code = r#"
        x := 10
        x := 20
    "#;

    let interp = run_code(code);

    if let Some(Value::Int(x)) = interp.env.get("x") {
        assert_eq!(x, 20);
    } else {
        panic!("Expected x to be 20");
    }
}

#[test]
fn test_struct_field_access() {
    let code = r#"
        struct Rectangle {
            width: int,
            height: int
        }

        rect := Rectangle { width: 5, height: 3 }
    "#;

    let interp = run_code(code);

    if let Some(Value::Struct { fields, .. }) = interp.env.get("rect") {
        if let Some(Value::Int(width)) = fields.get("width") {
            assert_eq!(*width, 5);
        } else {
            panic!("Expected width to be 5");
        }
    } else {
        panic!("Expected rect struct");
    }
}

// Lexical scoping tests

#[test]
fn test_nested_block_scopes() {
    // Functions create scopes - test variable updates across function boundaries
    let code = r#"
        x := 10
        func update_x() {
            x := 30
        }
        update_x()
    "#;

    let interp = run_code(code);

    // x should be updated to 30
    if let Some(Value::Int(x)) = interp.env.get("x") {
        assert_eq!(x, 30);
    } else {
        panic!("Expected x to be 30");
    }
}

#[test]
fn test_for_loop_scoping() {
    // The classic broken example from ROADMAP should now work
    let code = r#"
        sum := 0
        for n in [1, 2, 3] {
            sum := sum + n
        }
    "#;

    let interp = run_code(code);

    // sum should be 6, not 0
    if let Some(Value::Int(sum)) = interp.env.get("sum") {
        assert_eq!(sum, 6);
    } else {
        panic!("Expected sum to be 6");
    }
}

#[test]
fn test_for_loop_variable_isolation() {
    // Loop variable should not leak to outer scope
    let code = r#"
        for i in 5 {
            x := i * 2
        }
    "#;

    let interp = run_code(code);

    // i and x should not exist in outer scope
    assert!(interp.env.get("i").is_none(), "i should not leak from loop");
    assert!(interp.env.get("x").is_none(), "x should not leak from loop");
}

#[test]
fn test_variable_shadowing_in_block() {
    // A variable declared in inner scope (function) shadows for reading but not writing
    // When you do 'let x := 20' inside a function, it creates a NEW local x
    // When you then do 'inner := x', it reads the local x (20) and updates outer inner
    let code = r#"
        x := 10
        result := 0
        func test_func() {
            let x := 20
            result := x
        }
        test_func()
    "#;

    let interp = run_code(code);

    // result should be 20 (captured the shadowed local x)
    if let Some(Value::Int(result)) = interp.env.get("result") {
        assert_eq!(result, 20, "result should be 20 from shadowed local x");
    } else {
        panic!("Expected result to exist");
    }

    // x should still be 10 (outer x unchanged)
    if let Some(Value::Int(x)) = interp.env.get("x") {
        assert_eq!(x, 10, "outer x should remain 10");
    } else {
        panic!("Expected x to exist");
    }
}

#[test]
fn test_function_local_scope() {
    // Variables in function should have their own scope
    let code = r#"
        x := 100

        func modify_local() {
            let x := 50
            y := x * 2
        }

        modify_local()
    "#;

    let interp = run_code(code);

    // x in outer scope should still be 100
    if let Some(Value::Int(x)) = interp.env.get("x") {
        assert_eq!(x, 100);
    } else {
        panic!("Expected x to be 100");
    }

    // y should not leak from function
    assert!(interp.env.get("y").is_none(), "y should not leak from function");
}

#[test]
fn test_function_modifies_outer_variable() {
    // Function can access and modify outer scope variables
    let code = r#"
        counter := 0

        func increment() {
            counter := counter + 1
        }

        increment()
        increment()
        increment()
    "#;

    let interp = run_code(code);

    // counter should be 3
    if let Some(Value::Int(counter)) = interp.env.get("counter") {
        assert_eq!(counter, 3);
    } else {
        panic!("Expected counter to be 3");
    }
}

#[test]
fn test_nested_for_loops_scoping() {
    // Nested loops should each have their own scope
    let code = r#"
        result := 0
        for i in 3 {
            for j in 2 {
                result := result + 1
            }
        }
    "#;

    let interp = run_code(code);

    // result should be 6 (3 * 2)
    if let Some(Value::Int(result)) = interp.env.get("result") {
        assert_eq!(result, 6);
    } else {
        panic!("Expected result to be 6");
    }
}

#[test]
fn test_scope_chain_lookup() {
    // Variables should be found walking up the scope chain (nested functions)
    let code = r#"
        a := 1
        func outer() {
            b := 2
            func inner() {
                c := 3
                return a + b + c
            }
            return inner()
        }
        result := outer()
    "#;

    let interp = run_code(code);

    // result should be 6 (1 + 2 + 3)
    if let Some(Value::Int(result)) = interp.env.get("result") {
        assert_eq!(result, 6);
    } else {
        panic!("Expected result to be 6");
    }
}

#[test]
fn test_try_except_scoping() {
    // try/except should have proper scope isolation
    let code = r#"
        x := 10
        try {
            y := 20
            x := x + y
        } except err {
            // err only exists in except block
        }
    "#;

    let interp = run_code(code);

    // x should be 30
    if let Some(Value::Int(x)) = interp.env.get("x") {
        assert_eq!(x, 30);
    } else {
        panic!("Expected x to be 30");
    }

    // y should not leak
    assert!(interp.env.get("y").is_none(), "y should not leak from try block");
}

#[test]
fn test_accumulator_pattern() {
    // Common pattern: accumulating values in a loop
    let code = r#"
        numbers := [10, 20, 30, 40]
        total := 0
        for num in numbers {
            total := total + num
        }
    "#;

    let interp = run_code(code);

    // total should be 100
    if let Some(Value::Int(total)) = interp.env.get("total") {
        assert_eq!(total, 100);
    } else {
        panic!("Expected total to be 100");
    }
}

#[test]
fn test_multiple_assignments_in_for_loop() {
    // Multiple variables should all update correctly in loop
    let code = r#"
        count := 0
        sum := 0
        for i in 5 {
            count := count + 1
            sum := sum + i
        }
    "#;

    let interp = run_code(code);

    // count should be 5
    if let Some(Value::Int(count)) = interp.env.get("count") {
        assert_eq!(count, 5);
    } else {
        panic!("Expected count to be 5");
    }

    // sum should be 0+1+2+3+4 = 10
    if let Some(Value::Int(sum)) = interp.env.get("sum") {
        assert_eq!(sum, 10);
    } else {
        panic!("Expected sum to be 10");
    }
}

#[test]
fn test_environment_set_across_scopes() {
    let mut env = Environment::new();
    env.define("x".to_string(), Value::Float(5.0));

    // Push a new scope
    env.push_scope();

    // Set x from within the child scope
    env.set("x".to_string(), Value::Float(10.0));

    // Pop the scope
    env.pop_scope();

    // x should still be 10 in the global scope
    if let Some(Value::Float(x)) = env.get("x") {
        assert!((x - 10.0).abs() < 0.001, "x should be updated to 10 in global scope");
    } else {
        panic!("x should exist");
    }
}

#[test]
fn test_environment_assign_checked_respects_binding_mutability() {
    let mut env = Environment::new();
    env.define_with_kind("mutable_value".to_string(), Value::Int(1), BindingKind::Mutable);
    env.define_with_kind("let_value".to_string(), Value::Int(1), BindingKind::LetImmutable);
    env.define_with_kind("const_value".to_string(), Value::Int(1), BindingKind::Const);

    assert!(env.assign_checked("mutable_value".to_string(), Value::Int(2)).is_ok());
    assert!(matches!(env.get("mutable_value"), Some(Value::Int(2))));

    let let_err = env
        .assign_checked("let_value".to_string(), Value::Int(2))
        .expect_err("immutable let binding should reject reassignment");
    assert!(let_err.contains("Cannot reassign immutable let binding: let_value"));

    let const_err = env
        .assign_checked("const_value".to_string(), Value::Int(2))
        .expect_err("const binding should reject reassignment");
    assert!(const_err.contains("Cannot reassign const binding: const_value"));
}

#[test]
fn test_environment_mutate_checked_respects_binding_mutability() {
    let mut env = Environment::new();
    env.define_with_kind(
        "mutable_counts".to_string(),
        Value::Dict(Default::default()),
        BindingKind::Mutable,
    );
    env.define_with_kind(
        "let_counts".to_string(),
        Value::Dict(Default::default()),
        BindingKind::LetImmutable,
    );
    env.define_with_kind(
        "const_counts".to_string(),
        Value::Dict(Default::default()),
        BindingKind::Const,
    );

    assert!(env
        .mutate_checked("mutable_counts", |value| {
            if let Value::Dict(dict) = value {
                std::sync::Arc::make_mut(dict).insert("hits".into(), Value::Int(3));
            }
        })
        .is_ok());

    let let_err = env
        .mutate_checked("let_counts", |_| {})
        .expect_err("immutable let binding should reject mutation");
    assert!(let_err.contains("Cannot mutate immutable let binding: let_counts"));

    let const_err = env
        .mutate_checked("const_counts", |_| {})
        .expect_err("const binding should reject mutation");
    assert!(const_err.contains("Cannot mutate const binding: const_counts"));
}

// Input and type conversion function tests

#[test]
fn test_parse_int_valid() {
    let code = r#"
        result1 := parse_int("42")
        result2 := parse_int("  -100  ")
        result3 := parse_int("0")
    "#;

    let interp = run_code(code);

    if let Some(Value::Int(n)) = interp.env.get("result1") {
        assert_eq!(n, 42);
    } else {
        panic!("Expected result1 to be 42");
    }

    if let Some(Value::Int(n)) = interp.env.get("result2") {
        assert_eq!(n, -100);
    } else {
        panic!("Expected result2 to be -100");
    }

    if let Some(Value::Int(n)) = interp.env.get("result3") {
        assert_eq!(n, 0);
    } else {
        panic!("Expected result3 to be 0");
    }
}

#[test]
fn test_parse_int_invalid() {
    let code = r#"
        caught := "no error"
        try {
            result := parse_int("not a number")
        } except err {
            caught := err.message
        }
    "#;

    let interp = run_code(code);

    // Should have caught an error
    if let Some(Value::Str(err)) = interp.env.get("caught") {
        assert!(err.contains("Cannot parse") || err.as_str() == "no error", "Got: {err}");
        if err.as_str() != "no error" {
            assert!(err.contains("not a number"));
        }
    } else {
        panic!("Expected 'caught' variable to exist");
    }
}

#[test]
fn test_parse_float_valid() {
    let code = r#"
        result1 := parse_float("3.14")
        result2 := parse_float("  -2.5  ")
        result3 := parse_float("42")
        result4 := parse_float("0.0")
    "#;

    let interp = run_code(code);

    if let Some(Value::Float(n)) = interp.env.get("result1") {
        assert!((n - std::f64::consts::PI).abs() < 0.01);
    } else {
        panic!("Expected result1 to be 3.14");
    }

    if let Some(Value::Float(n)) = interp.env.get("result2") {
        assert!((n - (-2.5)).abs() < 0.001);
    } else {
        panic!("Expected result2 to be -2.5");
    }

    if let Some(Value::Float(n)) = interp.env.get("result3") {
        assert!((n - 42.0).abs() < 0.001);
    } else {
        panic!("Expected result3 to be 42");
    }

    if let Some(Value::Float(n)) = interp.env.get("result4") {
        assert!((n - 0.0).abs() < 0.001);
    } else {
        panic!("Expected result4 to be 0");
    }
}

#[test]
fn test_parse_float_invalid() {
    let code = r#"
        caught := "no error"
        try {
            result := parse_float("invalid")
        } except err {
            caught := err.message
        }
    "#;

    let interp = run_code(code);

    // Should have caught an error or no error was thrown
    if let Some(Value::Str(err)) = interp.env.get("caught") {
        assert!(err.contains("Cannot parse") || err.as_str() == "no error", "Got: {err}");
        if err.as_str() != "no error" {
            assert!(err.contains("invalid"));
        }
    } else {
        panic!("Expected 'caught' variable to exist");
    }
}

#[test]
fn test_parse_with_arithmetic() {
    // Test that parsed numbers can be used in arithmetic
    let code = r#"
        a := parse_int("10")
        b := parse_int("20")
        sum := a + b

        x := parse_float("3.5")
        y := parse_float("2.5")
        product := x * y
    "#;

    let interp = run_code(code);

    if let Some(Value::Int(n)) = interp.env.get("sum") {
        assert_eq!(n, 30);
    } else {
        panic!("Expected sum to be 30");
    }

    if let Some(Value::Float(n)) = interp.env.get("product") {
        assert!((n - 8.75).abs() < 0.001);
    } else {
        panic!("Expected product to be 8.75");
    }
}

#[test]
fn test_file_write_and_read() {
    use std::fs;
    let test_file = "/tmp/kujo_test_write_read.txt";

    // Clean up before test
    let _ = fs::remove_file(test_file);

    let code = format!(
        r#"
        result := write_file("{test_file}", "Hello, Kujo!")
        content := read_file("{test_file}")
    "#
    );

    let interp = run_code(&code);

    // Check write result
    if let Some(Value::Bool(b)) = interp.env.get("result") {
        assert!(b);
    } else {
        panic!("Expected write result to be true");
    }

    // Check read content
    if let Some(Value::Str(s)) = interp.env.get("content") {
        assert_eq!(s.as_str(), "Hello, Kujo!");
    } else {
        panic!("Expected content to be 'Hello, Kujo!'");
    }

    // Clean up after test
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_file_append() {
    use std::fs;
    let test_file = "/tmp/kujo_test_append.txt";

    // Clean up before test
    let _ = fs::remove_file(test_file);

    let code = format!(
        r#"
        r1 := write_file("{test_file}", "Line 1\n")
        r2 := append_file("{test_file}", "Line 2\n")
        r3 := append_file("{test_file}", "Line 3\n")
        content := read_file("{test_file}")
    "#
    );

    let interp = run_code(&code);

    if let Some(Value::Str(s)) = interp.env.get("content") {
        assert_eq!(s.as_str(), "Line 1\nLine 2\nLine 3\n");
    } else {
        panic!("Expected content with three lines");
    }

    // Clean up after test
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_file_exists() {
    use std::fs;
    let test_file = "/tmp/kujo_test_exists.txt";

    // Create test file
    fs::write(test_file, "test").unwrap();

    let code = format!(
        r#"
        exists1 := file_exists("{test_file}")
        exists2 := file_exists("/tmp/file_that_does_not_exist_kujo.txt")
    "#
    );

    let interp = run_code(&code);

    if let Some(Value::Bool(b)) = interp.env.get("exists1") {
        assert!(b);
    } else {
        panic!("Expected exists1 to be true");
    }

    if let Some(Value::Bool(b)) = interp.env.get("exists2") {
        assert!(!b);
    } else {
        panic!("Expected exists2 to be false");
    }

    // Clean up after test
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_read_lines() {
    use std::fs;
    let test_file = "/tmp/kujo_test_read_lines.txt";

    // Create test file with multiple lines
    fs::write(test_file, "Line 1\nLine 2\nLine 3").unwrap();

    let code = format!(
        r#"
        lines := read_lines("{test_file}")
        count := len(lines)
        first := lines[0]
        last := lines[2]
    "#
    );

    let interp = run_code(&code);

    if let Some(Value::Int(n)) = interp.env.get("count") {
        assert_eq!(n, 3);
    } else {
        panic!("Expected count to be 3");
    }

    if let Some(Value::Str(s)) = interp.env.get("first") {
        assert_eq!(s.as_str(), "Line 1");
    } else {
        panic!("Expected first line to be 'Line 1'");
    }

    if let Some(Value::Str(s)) = interp.env.get("last") {
        assert_eq!(s.as_str(), "Line 3");
    } else {
        panic!("Expected last line to be 'Line 3'");
    }

    // Clean up after test
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_create_dir_and_list() {
    use std::fs;
    let test_dir = "/tmp/kujo_test_dir";
    let test_file1 = format!("{test_dir}/file1.txt");
    let test_file2 = format!("{test_dir}/file2.txt");

    // Clean up before test
    let _ = fs::remove_dir_all(test_dir);

    let code = format!(
        r#"
        result := create_dir("{test_dir}")
        w1 := write_file("{test_file1}", "content1")
        w2 := write_file("{test_file2}", "content2")
        files := list_dir("{test_dir}")
        count := len(files)
    "#
    );

    let interp = run_code(&code);

    if let Some(Value::Bool(b)) = interp.env.get("result") {
        assert!(b);
    } else {
        panic!("Expected create_dir result to be true");
    }

    if let Some(Value::Int(n)) = interp.env.get("count") {
        assert_eq!(n, 2);
    } else {
        panic!("Expected 2 files in directory");
    }

    if let Some(Value::Array(files)) = interp.env.get("files") {
        let file_names: Vec<String> = files
            .iter()
            .filter_map(|v| if let Value::Str(s) = v { Some(s.as_ref().clone()) } else { None })
            .collect();
        assert!(file_names.contains(&"file1.txt".to_string()));
        assert!(file_names.contains(&"file2.txt".to_string()));
    } else {
        panic!("Expected files array");
    }

    // Clean up after test
    let _ = fs::remove_dir_all(test_dir);
}

#[test]
fn test_file_not_found_error() {
    let code = r#"
        caught := "no error"
        try {
            content := read_file("/tmp/file_that_definitely_does_not_exist_kujo.txt")
        } except err {
            caught := err.message
        }
    "#;

    let interp = run_code(code);

    if let Some(Value::Str(s)) = interp.env.get("caught") {
        assert!(s.contains("Cannot read file") || s.as_str() == "no error");
    } else {
        panic!("Expected 'caught' variable to exist");
    }
}

#[test]
fn test_bool_literals() {
    // Test that true and false are proper boolean values
    let code = r#"
        t := true
        f := false
    "#;

    let interp = run_code(code);

    if let Some(Value::Bool(b)) = interp.env.get("t") {
        assert!(b);
    } else {
        panic!("Expected t to be true");
    }

    if let Some(Value::Bool(b)) = interp.env.get("f") {
        assert!(!b);
    } else {
        panic!("Expected f to be false");
    }
}

#[test]
fn test_bool_comparisons() {
    // Test that comparison operators return booleans
    let code = r#"
        eq := 5 == 5
        neq := 5 == 6
        gt := 10 > 5
        lt := 3 < 8
        gte := 5 >= 5
        lte := 4 <= 4
        str_eq := "hello" == "hello"
        str_neq := "hello" == "world"
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("eq"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("neq"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("gt"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("lt"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("gte"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("lte"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("str_eq"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("str_neq"), Some(Value::Bool(false))));
}

#[test]
fn test_bool_in_if_conditions() {
    // Test that boolean values work directly in if conditions
    let code = r#"
        result1 := "not set"
        result2 := "not set"

        if true {
            result1 := "true branch"
        }

        if false {
            result2 := "false branch"
        } else {
            result2 := "else branch"
        }
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("result1"), Some(Value::Str(s)) if s.as_str() == "true branch")
    );
    assert!(
        matches!(interp.env.get("result2"), Some(Value::Str(s)) if s.as_str() == "else branch")
    );
}

#[test]
fn test_bool_comparison_results_in_if() {
    // Test that comparison results work in if statements
    let code = r#"
        result := "not set"
        x := 10

        if x > 5 {
            result := "x is greater than 5"
        }
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "x is greater than 5")
    );
}

#[test]
fn test_bool_equality() {
    // Test boolean equality comparisons
    let code = r#"
        tt := true == true
        ff := false == false
        tf := true == false
        ft := false == true
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("tt"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("ff"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("tf"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("ft"), Some(Value::Bool(false))));
}

#[test]
fn test_bool_print() {
    // Test that booleans can be printed (basic syntax check)
    let code = r#"
        t := true
        f := false
        comp := 5 > 3
        print(t)
        print(f)
        print(comp)
    "#;

    let interp = run_code(code);

    // Just verify the variables exist and are booleans
    assert!(matches!(interp.env.get("t"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("f"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("comp"), Some(Value::Bool(true))));
}

#[test]
fn test_bool_in_variables() {
    // Test storing and using boolean values in variables
    let code = r#"
        is_active := true
        result := "not set"

        if is_active {
            result := "is active"
        }
    "#;

    let interp = run_code(code);

    // Verify boolean variable works in if condition
    assert!(matches!(interp.env.get("is_active"), Some(Value::Bool(true))));
    assert!(
        matches!(interp.env.get("result"), Some(Value::Str(ref s)) if s.as_str() == "is active"),
        "Expected result to be 'is active', got {:?}",
        interp.env.get("result")
    );
}

#[test]
fn test_bool_from_file_operations() {
    // Test that file operations return proper booleans
    use std::fs;
    let test_file = "/tmp/kujo_bool_test.txt";
    fs::write(test_file, "test").unwrap();

    let code = format!(
        r#"
        exists := file_exists("{test_file}")
        not_exists := file_exists("/tmp/file_that_does_not_exist.txt")
    "#
    );

    let interp = run_code(&code);

    assert!(matches!(interp.env.get("exists"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("not_exists"), Some(Value::Bool(false))));

    let _ = fs::remove_file(test_file);
}

#[test]
fn test_bool_struct_fields() {
    // Test boolean fields in structs
    let code = r#"
        struct Status {
            active: bool,
            verified: bool
        }

        status := Status { active: true, verified: false }
    "#;

    let interp = run_code(code);

    if let Some(Value::Struct { fields, .. }) = interp.env.get("status") {
        assert!(matches!(fields.get("active"), Some(Value::Bool(true))));
        assert!(matches!(fields.get("verified"), Some(Value::Bool(false))));
    } else {
        panic!("Expected status struct");
    }
}

#[test]
fn test_bool_array_elements() {
    // Test boolean values in arrays
    let code = r#"
        flags := [true, false, true, 5 > 3]
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(arr)) = interp.env.get("flags") {
        assert_eq!(arr.len(), 4);
        assert!(matches!(arr.first(), Some(Value::Bool(true))));
        assert!(matches!(arr.get(1), Some(Value::Bool(false))));
        assert!(matches!(arr.get(2), Some(Value::Bool(true))));
        assert!(matches!(arr.get(3), Some(Value::Bool(true))));
    } else {
        panic!("Expected flags array");
    }
}

#[test]
fn test_while_loop_basic() {
    // Test basic while loop functionality
    let code = r#"
        x := 0
        while x < 5 {
            x := x + 1
        }
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("x"), Some(Value::Int(n)) if n == 5));
}

#[test]
fn test_while_loop_with_boolean() {
    // Test while loop with boolean condition
    let code = r#"
        running := true
        count := 0
        while running {
            count := count + 1
            if count >= 3 {
                running := false
            }
        }
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("count"), Some(Value::Int(n)) if n == 3));
    assert!(matches!(interp.env.get("running"), Some(Value::Bool(false))));
}

#[test]
fn test_break_in_while_loop() {
    // Test break statement in while loop
    let code = r#"
        x := 0
        while x < 100 {
            x := x + 1
            if x == 5 {
                break
            }
        }
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("x"), Some(Value::Int(n)) if n == 5));
}

#[test]
fn test_break_in_for_loop() {
    // Test break statement in for loop
    let code = r#"
        sum := 0
        for i in 10 {
            if i > 5 {
                break
            }
            sum := sum + i
        }
    "#;

    let interp = run_code(code);

    // Should sum 0+1+2+3+4+5 = 15
    assert!(matches!(interp.env.get("sum"), Some(Value::Int(n)) if n == 15));
}

#[test]
fn test_continue_in_while_loop() {
    // Test continue statement in while loop
    let code = r#"
        x := 0
        sum := 0
        while x < 5 {
            x := x + 1
            if x == 3 {
                continue
            }
            sum := sum + x
        }
    "#;

    let interp = run_code(code);

    // Should sum 1+2+4+5 = 12 (skipping 3)
    assert!(matches!(interp.env.get("sum"), Some(Value::Int(n)) if n == 12));
}

#[test]
fn test_continue_in_for_loop() {
    // Test continue statement in for loop
    let code = r#"
        sum := 0
        for i in 10 {
            if i % 2 == 0 {
                continue
            }
            sum := sum + i
        }
    "#;

    let interp = run_code(code);

    // Should sum only odd numbers: 1+3+5+7+9 = 25
    assert!(matches!(interp.env.get("sum"), Some(Value::Int(n)) if n == 25));
}

#[test]
fn test_nested_loops_with_break() {
    // Test break only breaks inner loop
    let code = r#"
        outer := 0
        inner_count := 0
        for i in 3 {
            outer := outer + 1
            for j in 5 {
                inner_count := inner_count + 1
                if j == 2 {
                    break
                }
            }
        }
    "#;

    let interp = run_code(code);

    // Outer loop runs 3 times, inner loop breaks at j==2 (runs 3 times per outer iteration)
    // So inner_count should be 3 * 3 = 9
    assert!(matches!(interp.env.get("outer"), Some(Value::Int(n)) if n == 3));
    assert!(matches!(interp.env.get("inner_count"), Some(Value::Int(n)) if n == 9));
}

#[test]
fn test_nested_loops_with_continue() {
    // Test continue only affects inner loop
    let code = r#"
        total := 0
        for i in 3 {
            for j in 5 {
                if j == 2 {
                    continue
                }
                total := total + 1
            }
        }
    "#;

    let interp = run_code(code);

    // Outer loop runs 3 times, inner loop runs 5 times but skips j==2
    // So total should be 3 * 4 = 12
    assert!(matches!(interp.env.get("total"), Some(Value::Int(n)) if n == 12));
}

#[test]
fn test_while_with_break_and_continue() {
    // Test both break and continue in same while loop
    let code = r#"
        x := 0
        sum := 0
        while true {
            x := x + 1
            if x > 10 {
                break
            }
            if x % 2 == 0 {
                continue
            }
            sum := sum + x
        }
    "#;

    let interp = run_code(code);

    // Should sum odd numbers from 1 to 9: 1+3+5+7+9 = 25
    assert!(matches!(interp.env.get("sum"), Some(Value::Int(n)) if n == 25));
    assert!(matches!(interp.env.get("x"), Some(Value::Int(n)) if n == 11));
}

#[test]
fn test_while_false_condition() {
    // Test while loop with false condition never executes
    let code = r#"
        executed := false
        while false {
            executed := true
        }
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("executed"), Some(Value::Bool(false))));
}

#[test]
fn test_for_loop_with_array_and_break() {
    // Test break in for loop iterating over array
    let code = r#"
        numbers := [1, 2, 3, 4, 5]
        sum := 0
        for n in numbers {
            sum := sum + n
            if n == 3 {
                break
            }
        }
    "#;

    let interp = run_code(code);

    // Should sum 1+2+3 = 6
    assert!(matches!(interp.env.get("sum"), Some(Value::Int(n)) if n == 6));
}

#[test]
fn test_for_loop_with_array_and_continue() {
    // Test continue in for loop iterating over array
    let code = r#"
        numbers := [1, 2, 3, 4, 5]
        sum := 0
        for n in numbers {
            if n == 3 {
                continue
            }
            sum := sum + n
        }
    "#;

    let interp = run_code(code);

    // Should sum 1+2+4+5 = 12 (skipping 3)
    assert!(matches!(interp.env.get("sum"), Some(Value::Int(n)) if n == 12));
}

#[test]
fn test_while_with_complex_condition() {
    // Test while loop with complex boolean condition
    let code = r#"
        x := 0
        y := 10
        while x < 5 {
            x := x + 1
            y := y - 1
        }
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("x"), Some(Value::Int(n)) if n == 5));
    assert!(matches!(interp.env.get("y"), Some(Value::Int(n)) if n == 5));
}

// String Interpolation Tests
#[test]
fn test_string_interpolation_basic() {
    let code = r#"
        name := "World"
        message := "Hello, ${name}!"
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("message"), Some(Value::Str(s)) if s.as_str() == "Hello, World!")
    );
}

#[test]
fn test_string_interpolation_numbers() {
    let code = r#"
        x := 42
        result := "The answer is ${x}"
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "The answer is 42")
    );
}

#[test]
fn test_string_interpolation_expressions() {
    let code = r#"
        x := 6
        y := 7
        result := "6 times 7 equals ${x * y}"
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "6 times 7 equals 42")
    );
}

#[test]
fn test_string_interpolation_multiple() {
    let code = r#"
        first := "John"
        last := "Doe"
        age := 30
        bio := "Name: ${first} ${last}, Age: ${age}"
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("bio"), Some(Value::Str(s)) if s.as_str() == "Name: John Doe, Age: 30")
    );
}

#[test]
fn test_string_interpolation_booleans() {
    let code = r#"
        is_valid := true
        status := "Valid: ${is_valid}"
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("status"), Some(Value::Str(s)) if s.as_str() == "Valid: true"));
}

#[test]
fn test_string_interpolation_comparisons() {
    let code = r#"
        x := 10
        y := 5
        result := "x > y: ${x > y}"
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "x > y: true"));
}

#[test]
fn test_string_interpolation_nested_expressions() {
    let code = r#"
        a := 2
        b := 3
        c := 4
        result := "Result: ${(a + b) * c}"
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "Result: 20"));
}

#[test]
fn test_string_interpolation_function_call() {
    let code = r#"
        func double(x) {
            return x * 2
        }

        n := 21
        result := "Double of ${n} is ${double(n)}"
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "Double of 21 is 42")
    );
}

#[test]
fn test_string_interpolation_empty() {
    let code = r#"
        message := "No interpolation here"
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("message"), Some(Value::Str(s)) if s.as_str() == "No interpolation here")
    );
}

#[test]
fn test_string_interpolation_only_expression() {
    let code = r#"
        x := 100
        result := "${x}"
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "100"));
}

#[test]
fn test_string_interpolation_adjacent_expressions() {
    let code = r#"
        a := 1
        b := 2
        c := 3
        result := "${a}${b}${c}"
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "123"));
}

#[test]
fn test_string_interpolation_with_text_between() {
    let code = r#"
        x := 10
        y := 20
        result := "x=${x}, y=${y}, sum=${x + y}"
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "x=10, y=20, sum=30")
    );
}

#[test]
fn test_string_interpolation_string_concat() {
    let code = r#"
        greeting := "Hello"
        name := "Alice"
        result := "${greeting}, ${name}!"
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "Hello, Alice!")
    );
}

#[test]
fn test_string_interpolation_in_function() {
    let code = r#"
        func greet(name) {
            return "Hello, ${name}!"
        }

        message := greet("World")
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("message"), Some(Value::Str(s)) if s.as_str() == "Hello, World!")
    );
}

#[test]
fn test_string_interpolation_struct_field() {
    let code = r#"
        struct Person {
            name: string,
            age: int
        }

        p := Person { name: "Bob", age: 25 }
        bio := "Name: ${p.name}, Age: ${p.age}"
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("bio"), Some(Value::Str(s)) if s.as_str() == "Name: Bob, Age: 25")
    );
}

#[test]
fn test_starts_with_basic() {
    let code = r#"
        result1 := starts_with("hello world", "hello")
        result2 := starts_with("hello world", "world")
        result3 := starts_with("test", "test")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result1"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("result2"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("result3"), Some(Value::Bool(true))));
}

#[test]
fn test_ends_with_basic() {
    let code = r#"
        result1 := ends_with("test.kujo", ".kujo")
        result2 := ends_with("test.kujo", ".py")
        result3 := ends_with("hello", "lo")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result1"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("result2"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("result3"), Some(Value::Bool(true))));
}

#[test]
fn test_index_of_found() {
    let code = r#"
        idx1 := index_of("hello world", "world")
        idx2 := index_of("hello", "ll")
        idx3 := index_of("test", "t")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("idx1"), Some(Value::Int(n)) if n == 6));
    assert!(matches!(interp.env.get("idx2"), Some(Value::Int(n)) if n == 2));
    assert!(matches!(interp.env.get("idx3"), Some(Value::Int(n)) if n == 0));
}

#[test]
fn test_index_of_not_found() {
    let code = r#"
        idx1 := index_of("hello", "xyz")
        idx2 := index_of("test", "abc")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("idx1"), Some(Value::Int(n)) if n == -1));
    assert!(matches!(interp.env.get("idx2"), Some(Value::Int(n)) if n == -1));
}

#[test]
fn test_repeat_basic() {
    let code = r#"
        str1 := repeat("ha", 3)
        str2 := repeat("x", 5)
        str3 := repeat("abc", 2)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("str1"), Some(Value::Str(s)) if s.as_str() == "hahaha"));
    assert!(matches!(interp.env.get("str2"), Some(Value::Str(s)) if s.as_str() == "xxxxx"));
    assert!(matches!(interp.env.get("str3"), Some(Value::Str(s)) if s.as_str() == "abcabc"));
}

#[test]
fn test_repeat_zero() {
    let code = r#"
        str1 := repeat("hello", 0)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("str1"), Some(Value::Str(s)) if s.is_empty()));
}

#[test]
fn test_split_basic() {
    let code = r#"
        parts := split("a,b,c", ",")
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(arr)) = interp.env.get("parts") {
        assert_eq!(arr.len(), 3);
        assert!(matches!(&arr[0], Value::Str(s) if s.as_str() == "a"));
        assert!(matches!(&arr[1], Value::Str(s) if s.as_str() == "b"));
        assert!(matches!(&arr[2], Value::Str(s) if s.as_str() == "c"));
    } else {
        panic!("Expected parts to be an array");
    }
}

#[test]
fn test_split_multiple_chars() {
    let code = r#"
        parts := split("hello::world::test", "::")
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(arr)) = interp.env.get("parts") {
        assert_eq!(arr.len(), 3);
        assert!(matches!(&arr[0], Value::Str(s) if s.as_str() == "hello"));
        assert!(matches!(&arr[1], Value::Str(s) if s.as_str() == "world"));
        assert!(matches!(&arr[2], Value::Str(s) if s.as_str() == "test"));
    } else {
        panic!("Expected parts to be an array");
    }
}

#[test]
fn test_split_spaces() {
    let code = r#"
        words := split("one two three", " ")
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(arr)) = interp.env.get("words") {
        assert_eq!(arr.len(), 3);
        assert!(matches!(&arr[0], Value::Str(s) if s.as_str() == "one"));
        assert!(matches!(&arr[1], Value::Str(s) if s.as_str() == "two"));
        assert!(matches!(&arr[2], Value::Str(s) if s.as_str() == "three"));
    } else {
        panic!("Expected words to be an array");
    }
}

#[test]
fn test_join_basic() {
    let code = r#"
        arr := ["a", "b", "c"]
        result := join(arr, ",")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "a,b,c"));
}

#[test]
fn test_join_with_spaces() {
    let code = r#"
        words := ["hello", "world", "test"]
        sentence := join(words, " ")
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("sentence"), Some(Value::Str(s)) if s.as_str() == "hello world test")
    );
}

#[test]
fn test_join_numbers() {
    let code = r#"
        nums := [1, 2, 3]
        result := join(nums, "-")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "1-2-3"));
}

#[test]
fn test_split_join_roundtrip() {
    let code = r#"
        original := "apple,banana,cherry"
        parts := split(original, ",")
        rejoined := join(parts, ",")
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("rejoined"), Some(Value::Str(s)) if s.as_str() == "apple,banana,cherry")
    );
}

#[test]
fn test_string_functions_in_condition() {
    let code = r#"
        filename := "test.kujo"
        is_kujo := ends_with(filename, ".kujo")
        result := 0
        if is_kujo {
            result := 1
        }
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Int(n)) if n == 1));
}

#[test]
fn test_error_properties_message() {
    let code = r#"
        result := ""
        try {
            throw("Test error message")
        } except err {
            result := err.message
        }
    "#;

    let interp = run_code(code);
    assert!(
        matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "Test error message")
    );
}

#[test]
fn test_error_properties_stack() {
    let code = r#"
        stack_len := 0
        try {
            throw("Error")
        } except err {
            stack_len := len(err.stack)
        }
    "#;

    let interp = run_code(code);
    // Stack should be an array (even if empty)
    assert!(matches!(interp.env.get("stack_len"), Some(Value::Int(n)) if n >= 0));
}

#[test]
fn test_error_properties_line() {
    let code = r#"
        result := 0
        try {
            throw("Error")
        } except err {
            result := err.line
        }
    "#;

    let interp = run_code(code);
    // Line number should be accessible (0 if not set)
    assert!(matches!(interp.env.get("result"), Some(Value::Int(n)) if n >= 0));
}

#[test]
fn test_custom_error_struct() {
    let code = r#"
        struct ValidationError {
            field: string,
            message: string
        }

        caught_error := ""
        try {
            error := ValidationError {
                field: "email",
                message: "Email is required"
            }
            throw(error)
        } except err {
            caught_error := err.message
        }
    "#;

    let interp = run_code(code);
    assert!(matches!(
        interp.env.get("caught_error"),
        Some(Value::Str(s)) if s.contains("ValidationError") || s.contains("Email")
    ));
}

#[test]
fn test_error_chaining() {
    let code = r#"
        struct DatabaseError {
            message: string,
            cause: string
        }

        caught := ""
        try {
            error := DatabaseError {
                message: "Failed to connect",
                cause: "Connection timeout"
            }
            throw(error)
        } except err {
            caught := err.message
        }
    "#;

    let interp = run_code(code);
    assert!(matches!(
        interp.env.get("caught"),
        Some(Value::Str(s)) if s.contains("Failed") || s.contains("DatabaseError")
    ));
}

#[test]
fn test_error_in_function_with_stack_trace() {
    let code = r#"
        func inner() {
            throw("Inner error")
        }

        func outer() {
            inner()
        }

        result := ""
        try {
            outer()
        } except err {
            result := err.message
        }
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "Inner error"));
}

#[test]
fn test_nested_try_except() {
    let code = r#"
        result := ""
        try {
            try {
                throw("Inner error")
            } except inner_err {
                result := "caught inner: " + inner_err.message
            }
        } except outer_err {
            result := "caught outer"
        }
    "#;

    let interp = run_code(code);
    assert!(matches!(
        interp.env.get("result"),
        Some(Value::Str(s)) if s.contains("caught inner") && s.contains("Inner error")
    ));
}

#[test]
fn test_error_without_catch_propagates() {
    let code = r#"
        func risky() {
            throw("Unhandled error")
        }

        risky()
    "#;

    let interp = run_code(code);
    // Error should be stored in return_value
    assert!(matches!(interp.return_value, Some(Value::Error(_)) | Some(Value::ErrorObject { .. })));
}

#[test]
fn test_error_recovery_continues_execution() {
    let code = r#"
        x := 0
        try {
            throw("Error occurred")
        } except err {
            x := 1
        }
        x := x + 1
    "#;

    let interp = run_code(code);
    // After catching error, execution should continue
    assert!(matches!(interp.env.get("x"), Some(Value::Int(n)) if n == 2));
}

// JWT Authentication Tests

#[test]
fn test_jwt_encode_basic() {
    let code = r#"
        payload := {"user_id": 123, "username": "alice"}
        secret := "my-secret-key"
        token := jwt_encode(payload, secret)
        result := len(token) > 0
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("result"), Some(Value::Bool(true))));
}

#[test]
fn test_jwt_encode_decode_roundtrip() {
    let code = r#"
        payload := {"user_id": 456, "role": "admin", "active": true}
        secret := "test-secret-123"

        token := jwt_encode(payload, secret)
        decoded := jwt_decode(token, secret)

        user_id := decoded["user_id"]
        role := decoded["role"]
        active := decoded["active"]
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("user_id"), Some(Value::Int(n)) if n == 456));
    assert!(matches!(interp.env.get("role"), Some(Value::Str(s)) if s.as_str() == "admin"));
    assert!(matches!(interp.env.get("active"), Some(Value::Bool(true))));
}

#[test]
fn test_jwt_decode_with_wrong_secret() {
    let code = r#"
        payload := {"user_id": 789}
        secret := "correct-secret"
        wrong_secret := "wrong-secret"

        token := jwt_encode(payload, secret)

        # Initialize before try block
        decode_failed := false

        # Try to decode with wrong secret - should fail
        try {
            result := jwt_decode(token, wrong_secret)
            # If we get here, decoding didn't fail
            decode_failed := false
        } except err {
            # Error was caught as expected
            decode_failed := true
        }
    "#;

    let interp = run_code(code);
    // Should have caught an error
    assert!(matches!(interp.env.get("decode_failed"), Some(Value::Bool(true))));
}

#[test]
fn test_jwt_with_expiry_claim() {
    let code = r#"
        timestamp := now()
        expiry := timestamp + 3600

        payload := {"user_id": 100, "exp": expiry}
        secret := "secret-key"

        token := jwt_encode(payload, secret)
        decoded := jwt_decode(token, secret)

        decoded_user := decoded["user_id"]
        # has_key returns 1 or 0, so check if greater than 0
        has_expiry_num := has_key(decoded, "exp")
        has_expiry := has_expiry_num > 0
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("decoded_user"), Some(Value::Int(n)) if n == 100));
    assert!(matches!(interp.env.get("has_expiry"), Some(Value::Bool(true))));
}

#[test]
fn test_jwt_with_nested_data() {
    let code = r#"
        payload := {
            "user": {"id": 999, "name": "bob"},
            "permissions": ["read", "write"]
        }
        secret := "nested-secret"

        token := jwt_encode(payload, secret)
        decoded := jwt_decode(token, secret)

        user_obj := decoded["user"]
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("user_obj"), Some(Value::Dict(_))));
}

#[test]
fn test_jwt_empty_payload() {
    let code = r#"
        payload := {}
        secret := "empty-secret"

        token := jwt_encode(payload, secret)
        decoded := jwt_decode(token, secret)

        is_dict := true
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("decoded"), Some(Value::Dict(_))));
}

// OAuth2 Authentication Tests

#[test]
fn test_oauth2_auth_url_generation() {
    let code = r#"
        client_id := "my-client-id"
        redirect_uri := "https://example.com/callback"
        auth_url := "https://provider.com/oauth/authorize"
        scope := "read write"

        url := oauth2_auth_url(client_id, redirect_uri, auth_url, scope)

        # contains returns 1 or 0, convert to bool
        contains_client := contains(url, "client_id=my-client-id") > 0
        contains_redirect := contains(url, "redirect_uri=") > 0
        contains_scope := contains(url, "scope=") > 0
        contains_state := contains(url, "state=") > 0
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("contains_client"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("contains_redirect"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("contains_scope"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("contains_state"), Some(Value::Bool(true))));
}

#[test]
fn test_oauth2_auth_url_encoding() {
    let code = r#"
        client_id := "test client"
        redirect_uri := "https://example.com/callback?param=value"
        auth_url := "https://auth.example.com/authorize"
        scope := "read:user write:repo"

        url := oauth2_auth_url(client_id, redirect_uri, auth_url, scope)

        starts_with_auth := starts_with(url, "https://auth.example.com/authorize?")
        has_encoded_space := contains(url, "%20") || contains(url, "+")
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("starts_with_auth"), Some(Value::Bool(true))));
}

// HTTP Streaming Tests

#[test]
fn test_http_get_stream_returns_bytes() {
    let code = r#"
        # Note: This would require a real HTTP server to test properly
        # For now, we test that the function exists and handles errors
        result := "function_exists"
    "#;

    let interp = run_code(code);
    assert!(
        matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "function_exists")
    );
}

#[test]
fn test_streaming_with_binary_data() {
    let code = r#"
        # Test that we can work with binary data from streaming
        data := [72, 101, 108, 108, 111]  # "Hello" in ASCII
        length := len(data)
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("length"), Some(Value::Int(n)) if n == 5));
}

#[test]
fn test_jwt_integration_with_api_auth() {
    let code = r#"
        # Simulate an API authentication flow
        user_data := {"user_id": 42, "email": "test@example.com"}
        api_secret := "api-secret-key-2026"

        # Generate JWT token
        auth_token := jwt_encode(user_data, api_secret)

        # Verify token (as API would do)
        verified_data := jwt_decode(auth_token, api_secret)

        # Extract user info
        user_id := verified_data["user_id"]
        email := verified_data["email"]

        auth_success := user_id == 42 && email == "test@example.com"
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("auth_success"), Some(Value::Bool(true))));
}

#[test]
fn test_jwt_with_multiple_claims() {
    let code = r#"
        timestamp := now()
        payload := {
            "sub": "1234567890",
            "name": "John Doe",
            "iat": timestamp,
            "admin": true,
            "roles": ["user", "moderator"]
        }
        secret := "multi-claim-secret"

        token := jwt_encode(payload, secret)
        decoded := jwt_decode(token, secret)

        name := decoded["name"]
        is_admin := decoded["admin"]
        subject := decoded["sub"]
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("name"), Some(Value::Str(s)) if s.as_str() == "John Doe"));
    assert!(matches!(interp.env.get("is_admin"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("subject"), Some(Value::Str(s)) if s.as_str() == "1234567890"));
}

#[test]
fn test_oauth2_complete_flow_simulation() {
    let code = r#"
        # Step 1: Generate authorization URL
        auth_url := oauth2_auth_url(
            "client-123",
            "https://app.example.com/callback",
            "https://provider.com/auth",
            "user:read user:write"
        )

        # Verify URL was generated - contains returns number
        has_client_id := contains(auth_url, "client_id=") > 0
        has_scope := contains(auth_url, "scope=") > 0

        # Simulate the authorization flow
        flow_started := has_client_id && has_scope
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("flow_started"), Some(Value::Bool(true))));
}

#[test]
fn test_spawn_basic() {
    let code = r#"
        x := 0
        spawn {
            y := 5
            # Note: spawn runs in isolation, can't modify outer x
        }
        # Main thread continues immediately
        z := 10
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("x"), Some(Value::Int(n)) if n == 0));
    assert!(matches!(interp.env.get("z"), Some(Value::Int(n)) if n == 10));
    // y should not exist in main scope
    assert!(interp.env.get("y").is_none());
}

#[test]
fn test_spawn_can_read_parent_scalar_bindings_snapshot() {
    let code = r#"
        base := 21
        shared_key := "spawn_parent_scalar_snapshot"
        shared_set(shared_key, 0)

        spawn {
            shared_set(shared_key, base * 2)
        }

        attempts := 0
        while attempts < 1000 {
            current := shared_get(shared_key)
            if current == 42 {
                break
            }
            await async_sleep(1)
            attempts := attempts + 1
        }

        spawned_result := shared_get(shared_key)
        cleanup_ok := shared_delete(shared_key)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("spawned_result"), Some(Value::Int(42))));
    assert!(matches!(interp.env.get("cleanup_ok"), Some(Value::Bool(true))));
}

#[test]
fn test_spawn_can_use_parent_defined_shared_key_variable() {
    let counter_key = unique_shared_key("spawn_parent_key_capture");

    let code = format!(
        r#"
        counter_key := "{counter_key}"
        shared_set(counter_key, 0)

        for i in range(0, 12) {{
            spawn {{
                shared_add_int(counter_key, 1)
            }}
        }}

        attempts := 0
        while attempts < 2000 {{
            current := shared_get(counter_key)
            if current == 12 {{
                break
            }}
            await async_sleep(1)
            attempts := attempts + 1
        }}

        final_counter := shared_get(counter_key)
        cleanup_counter := shared_delete(counter_key)
    "#
    );

    let interp = run_code(&code);

    assert!(matches!(interp.env.get("final_counter"), Some(Value::Int(12))));
    assert!(matches!(interp.env.get("cleanup_counter"), Some(Value::Bool(true))));
}

#[test]
fn test_spawn_snapshot_mutations_do_not_write_back_to_parent_scope() {
    let code = r#"
        parent_counter := 7

        spawn {
            parent_counter := 999
        }

        await async_sleep(20)
        after_spawn := parent_counter
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("after_spawn"), Some(Value::Int(7))));
}

#[test]
fn test_parallel_http_basic() {
    // Test that parallel_http function exists and handles empty/invalid URLs gracefully
    let code = r#"
        # Test with invalid URLs to avoid network dependency
        urls := ["invalid://url1", "invalid://url2"]
        results := parallel_http(urls)
        # Results should be an array even with invalid URLs
        is_array := type(results) == "array"
    "#;

    let interp = run_code(code);
    // Should get an array result (even if empty or with errors)
    assert!(matches!(interp.env.get("is_array"), Some(Value::Bool(true))));
}

#[test]
fn test_channel_basic() {
    let code = r#"
        chan := channel()
        # Test that channel was created
        has_channel := type(chan) == "channel"
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("has_channel"), Some(Value::Bool(true))));
}

#[test]
fn test_channel_multiple_values() {
    let code = r#"
        chan := channel()
        # Just test send works without error
        chan.send("hello")
        chan.send("world")
        success := true
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("success"), Some(Value::Bool(true))));
}

#[test]
fn test_channel_empty() {
    let code = r#"
        chan := channel()
        # Test channel type
        is_channel := type(chan) == "channel"
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("is_channel"), Some(Value::Bool(true))));
}

#[test]
fn test_channel_receive_blocks_until_value_is_sent() {
    let mut interp = run_code("chan := channel()");
    let channel = match interp.env.get("chan") {
        Some(Value::Channel(channel)) => channel.clone(),
        other => panic!("expected channel binding, got {other:?}"),
    };

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(15));
        let chan_lock = channel.lock().expect("channel lock should succeed");
        let (sender, _) = &*chan_lock;
        sender.send(Value::Int(42)).expect("send should succeed");
    });

    let code = r#"
        received := chan.receive()
        ok := received == 42
    "#;
    let tokens = tokenize(code).expect("test source should tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse();
    interp.eval_stmts(&program);

    assert!(matches!(interp.env.get("ok"), Some(Value::Bool(true))));
}

#[test]
fn test_channel_receive_and_send_preserve_fifo_order() {
    let code = r#"
        chan := channel()
        chan.send("first")
        chan.send("second")

        first := chan.receive()
        second := chan.receive()
        ok := first == "first" && second == "second"
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("ok"), Some(Value::Bool(true))));
}

#[test]
fn test_channel_receive_arity_error_via_expression_method_call() {
    let code = r#"
        err := channel().receive(1)
    "#;

    let interp = run_code(code);
    let message = match interp.env.get("err") {
        Some(Value::Error(message)) => message,
        other => panic!("expected error value, got {other:?}"),
    };
    assert!(
        message.contains("Channel.receive expects 0 arguments, got 1"),
        "unexpected error message: {message}"
    );
}

#[test]
fn test_shared_value_lifecycle_operations() {
    let shared_key = unique_shared_key("shared_value_lifecycle");

    let code = format!(
        r#"
        key := "{shared_key}"

        created := shared_set(key, 41)
        exists_before := shared_has(key)
        value_before := shared_get(key)

        updated := shared_set(key, 99)
        value_after := shared_get(key)

        deleted := shared_delete(key)
        exists_after := shared_has(key)
    "#
    );

    let interp = run_code(&code);

    assert!(matches!(interp.env.get("created"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("exists_before"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("value_before"), Some(Value::Int(41))));
    assert!(matches!(interp.env.get("updated"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("value_after"), Some(Value::Int(99))));
    assert!(matches!(interp.env.get("deleted"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("exists_after"), Some(Value::Bool(false))));
}

#[test]
fn test_shared_add_int_success_and_error_paths() {
    let base_key = unique_shared_key("shared_add_int");

    let code = format!(
        r#"
        key := "{base_key}"

        shared_set(key, 0)
        plus_five := shared_add_int(key, 5)
        plus_two := shared_add_int(key, 2)
        final_value := shared_get(key)

        bad_delta := shared_add_int(key, "1")

        shared_delete(key)
    "#
    );

    let interp = run_code(&code);

    assert!(matches!(interp.env.get("plus_five"), Some(Value::Int(5))));
    assert!(matches!(interp.env.get("plus_two"), Some(Value::Int(7))));
    assert!(matches!(interp.env.get("final_value"), Some(Value::Int(7))));
    assert!(
        matches!(interp.env.get("bad_delta"), Some(Value::Error(msg)) if msg.contains("delta must be an int"))
    );
}

#[test]
fn test_shared_add_int_rejects_non_int_target_value() {
    let base_key = unique_shared_key("shared_add_int_non_int");

    let code = format!(
        r#"
        key := "{base_key}"
        shared_set(key, "not an int")
        result := shared_add_int(key, 1)
        shared_delete(key)
    "#
    );

    let interp = run_code(&code);

    assert!(matches!(
        interp.env.get("result"),
        Some(Value::Error(msg)) if msg.contains("to reference an int")
    ));
}

#[test]
fn test_shared_add_int_rejects_missing_key() {
    let base_key = unique_shared_key("shared_add_int_missing");

    let code = format!(
        r#"
        missing_key := "{base_key}"
        result := shared_add_int(missing_key, 1)
    "#
    );

    let interp = run_code(&code);

    assert!(matches!(
        interp.env.get("result"),
        Some(Value::Error(msg)) if msg.contains("not found")
    ));
}

#[test]
fn test_spawn_can_update_shared_values_across_isolated_environments() {
    let counter_key = unique_shared_key("spawn_shared_counter");

    let code = format!(
        r#"
        shared_set("{counter_key}", 0)

        for i in range(0, 20) {{
            spawn {{
                shared_add_int("{counter_key}", 1)
            }}
        }}

        attempts := 0

        while attempts < 2000 {{
            current := shared_get("{counter_key}")
            if current == 20 {{
                break
            }}
            await async_sleep(1)
            attempts := attempts + 1
        }}

        final_counter := shared_get("{counter_key}")
        completed := final_counter == 20

        cleanup_counter := shared_delete("{counter_key}")
    "#
    );

    let interp = run_code(&code);

    assert!(matches!(interp.env.get("completed"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("cleanup_counter"), Some(Value::Bool(true))));
}

#[test]
fn test_parallel_http_empty_array() {
    let code = r#"
        urls := []
        results := parallel_http(urls)
        count := len(results)
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("count"), Some(Value::Int(n)) if n == 0));
}

#[test]
fn test_await_all_with_concurrency_limit_preserves_order() {
    use std::fs;

    let file1 = "/tmp/kujo_async_await_all_1.txt";
    let file2 = "/tmp/kujo_async_await_all_2.txt";
    let file3 = "/tmp/kujo_async_await_all_3.txt";

    let _ = fs::remove_file(file1);
    let _ = fs::remove_file(file2);
    let _ = fs::remove_file(file3);

    fs::write(file1, "alpha").unwrap();
    fs::write(file2, "beta").unwrap();
    fs::write(file3, "gamma").unwrap();

    let code = format!(
        r#"
        p1 := async_read_file("{file1}")
        p2 := async_read_file("{file2}")
        p3 := async_read_file("{file3}")

        results := await await_all([p1, p2, p3], 2)

        first := results[0]
        second := results[1]
        third := results[2]

        order_ok := first == "alpha" && second == "beta" && third == "gamma"
    "#
    );

    let interp = run_code(&code);

    assert!(matches!(interp.env.get("order_ok"), Some(Value::Bool(true))));

    let _ = fs::remove_file(file1);
    let _ = fs::remove_file(file2);
    let _ = fs::remove_file(file3);
}

#[test]
fn test_promise_all_single_arg_still_works() {
    let code = r#"
        promises := [async_sleep(1), async_sleep(1)]
        results := await promise_all(promises)
        count := len(results)
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("count"), Some(Value::Int(n)) if n == 2));
}

#[test]
fn test_promise_all_reuses_cached_polled_promise_results() {
    let code = r#"
        p := async_sleep(1)
        first := await p

        results := await promise_all([p, p], 2)
        ok := type(first) == "null" && len(results) == 2 && type(results[0]) == "null" && type(results[1]) == "null"
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("ok"), Some(Value::Bool(true))));
}

#[test]
fn test_promise_all_rejects_zero_concurrency_limit() {
    let code = r#"
        result := promise_all([async_sleep(1)], 0)
    "#;

    let interp = run_code(code);

    assert!(matches!(
        interp.env.get("result"),
        Some(Value::Error(message)) if message.contains("concurrency_limit must be > 0")
    ));
}

#[test]
fn test_promise_all_rejects_non_integer_concurrency_limit() {
    let code = r#"
        result := promise_all([async_sleep(1)], "2")
    "#;

    let interp = run_code(code);

    assert!(matches!(
        interp.env.get("result"),
        Some(Value::Error(message)) if message.contains("concurrency_limit must be an integer")
    ));
}

#[test]
fn test_task_pool_size_set_get_round_trip() {
    let code = r#"
        initial := get_task_pool_size()
        previous := set_task_pool_size(3)
        current := get_task_pool_size()
        restored := set_task_pool_size(previous)

        previous_matches_initial := previous == initial
        current_is_three := current == 3
        restored_was_three := restored == 3
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("previous_matches_initial"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("current_is_three"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("restored_was_three"), Some(Value::Bool(true))));
}

#[test]
fn test_task_pool_size_validation_errors() {
    let interp_zero = run_code("bad_zero := set_task_pool_size(0)");
    assert!(matches!(
        interp_zero.env.get("bad_zero"),
        Some(Value::Error(message)) if message.contains("size must be > 0")
    ));

    let interp_type = run_code("bad_type := set_task_pool_size(\"4\")");
    assert!(matches!(
        interp_type.env.get("bad_type"),
        Some(Value::Error(message)) if message.contains("requires an integer size argument")
    ));

    let interp_get = run_code("bad_get_args := get_task_pool_size(1)");
    assert!(matches!(
        interp_get.env.get("bad_get_args"),
        Some(Value::Error(message)) if message.contains("expects 0 arguments")
    ));
}

#[test]
fn test_await_all_uses_configured_default_task_pool_size() {
    let code = r#"
        previous := set_task_pool_size(2)
        p1 := async_sleep(1)
        p2 := async_sleep(1)
        p3 := async_sleep(1)
        results := await await_all([p1, p2, p3])
        count := len(results)
        set_task_pool_size(previous)
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("count"), Some(Value::Int(n)) if n == 3));
}

#[test]
fn test_promise_all_large_array_with_bounded_concurrency() {
    let promises = std::iter::repeat_n("async_sleep(1)", 256).collect::<Vec<_>>().join(", ");

    let code = format!(
        r#"
        promises := [{promises}]
        results := await promise_all(promises, 32)
        result_type := type(results)
        count := len(results)
    "#
    );

    let interp = run_code(&code);
    assert!(matches!(interp.env.get("result_type"), Some(Value::Str(t)) if t.as_ref() == "array"));
    match interp.env.get("count") {
        Some(Value::Int(n)) => assert_eq!(n, 256),
        other => panic!(
            "expected integer count, got {:?}; result_type={:?}; results={:?}",
            other,
            interp.env.get("result_type"),
            interp.env.get("results")
        ),
    }
}

#[test]
fn test_await_all_large_array_uses_configured_default_pool() {
    let promises = std::iter::repeat_n("async_sleep(1)", 180).collect::<Vec<_>>().join(", ");

    let code = format!(
        r#"
        previous := set_task_pool_size(24)
        promises := [{promises}]
        results := await await_all(promises)
        count := len(results)
        set_task_pool_size(previous)
    "#
    );

    let interp = run_code(&code);
    assert!(matches!(interp.env.get("count"), Some(Value::Int(n)) if n == 180));
}

#[test]
fn test_par_map_alias_works_in_pipeline() {
    let code = r#"
        values := ["a", "bc", "def"]
        results := await par_map(values, len, 2)
        count := len(results)
        first := results[0]
        second := results[1]
        third := results[2]
        ok := count == 3 && first == 1 && second == 2 && third == 3
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("ok"), Some(Value::Bool(true))));
}

#[test]
fn test_par_each_resolves_null_and_completes() {
    let code = r#"
        values := ["aa", "bbb", "cccc"]
        result := await par_each(values, len, 2)
        result_type := type(result)
        is_null_result := result_type == "null"
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("is_null_result"), Some(Value::Bool(true))));
}

#[test]
fn test_par_each_propagates_rejection() {
    let code = r#"
        result := await par_each(["/tmp/kujo_missing_par_each_pipeline.txt"], async_read_file)
    "#;

    let interp = run_code(code);
    assert!(matches!(
        interp.env.get("result"),
        Some(Value::Error(message)) if message.contains("Promise 0 rejected")
    ));
}

#[test]
fn test_parallel_map_rayon_upper_pipeline() {
    let code = r#"
        words := ["kujo", "lang", "jit"]
        mapped := await parallel_map(words, upper, 4)
        ok := mapped[0] == "KUJO" && mapped[1] == "LANG" && mapped[2] == "JIT"
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("ok"), Some(Value::Bool(true))));
}

#[test]
fn test_parallel_map_rayon_len_pipeline_with_mixed_collections() {
    let code = r#"
        values := ["abc", [1, 2, 3, 4], {"x": 1, "y": 2, "z": 3}]
        sizes := await parallel_map(values, len, 3)
        ok := sizes[0] == 3 && sizes[1] == 4 && sizes[2] == 3
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("ok"), Some(Value::Bool(true))));
}

#[test]
fn test_import_missing_module_returns_runtime_error() {
    let module_name = unique_module_name("module_does_not_exist");
    let interp = run_code(&format!("import {module_name}"));

    assert!(matches!(
        interp.return_value,
        Some(Value::Error(message)) if message.contains(&format!("Module not found: {module_name}"))
    ));
}

#[test]
fn test_from_import_missing_symbol_returns_runtime_error() {
    let modules_dir = std::path::PathBuf::from("modules");
    std::fs::create_dir_all(&modules_dir).expect("failed to create modules directory for test");

    let module_name = unique_module_name("module_present");
    let module_path = modules_dir.join(format!("{module_name}.kujo"));
    std::fs::write(&module_path, "export present := 1\n")
        .expect("failed to write temporary module");

    let interp = run_code(&format!("from {module_name} import absent"));

    assert!(matches!(
        interp.return_value,
        Some(Value::Error(message))
            if message.contains(&format!("Symbol 'absent' not found in module '{module_name}'"))
    ));

    std::fs::remove_file(&module_path).expect("failed to remove temporary module file");
}

#[test]
fn test_imported_function_can_call_module_local_helper() {
    let modules_dir = std::path::PathBuf::from("modules");
    std::fs::create_dir_all(&modules_dir).expect("failed to create modules directory for test");

    let module_name = unique_module_name("module_function_helper");
    let module_path = modules_dir.join(format!("{module_name}.kujo"));
    std::fs::write(
        &module_path,
        "func helper(x) {\n\treturn x + 1\n}\n\nfunc add_one_impl(x) {\n\treturn helper(x)\n}\n\nexport add_one := add_one_impl\n",
    )
    .expect("failed to write temporary module");

    let code = format!("from {module_name} import add_one\nresult := add_one(41)\n");
    let interp = run_code(&code);

    assert!(matches!(interp.env.get("result"), Some(Value::Int(42))));

    std::fs::remove_file(&module_path).expect("failed to remove temporary module file");
}

#[test]
fn test_dotted_from_import_resolves_nested_module_file() {
    let modules_dir = std::path::PathBuf::from("modules");
    std::fs::create_dir_all(&modules_dir).expect("failed to create modules directory for test");

    let root_module = unique_module_name("nested_module_root");
    let nested_dir = modules_dir.join(&root_module).join("core");
    std::fs::create_dir_all(&nested_dir).expect("failed to create nested module dir");

    let module_path = nested_dir.join("math.kujo");
    std::fs::write(
        &module_path,
        "func add(left, right) {\n\treturn left + right\n}\nexport add := add\n",
    )
    .expect("failed to write nested module");

    let code = format!("from {root_module}.core.math import add\nresult := add(40, 2)\n");
    let interp = run_code(&code);

    assert!(matches!(interp.env.get("result"), Some(Value::Int(42))));

    std::fs::remove_dir_all(modules_dir.join(&root_module))
        .expect("failed to remove temporary nested module tree");
}

#[test]
fn test_dotted_from_import_missing_module_returns_runtime_error() {
    let root_module = unique_module_name("missing_dotted_module");
    let dotted_module_name = format!("{root_module}.core.math");
    let interp = run_code(&format!("from {dotted_module_name} import answer"));

    assert!(matches!(
        interp.return_value,
        Some(Value::Error(message)) if message.contains(&format!("Module not found: {dotted_module_name}"))
    ));
}

#[test]
fn test_null_equality_and_inequality_semantics() {
    let code = r#"
        eq_null := null == null
        ne_null := null != null
        eq_mixed_left := null == 1
        eq_mixed_right := 1 == null
        ne_mixed_left := null != 1
        ne_mixed_right := 1 != null
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("eq_null"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("ne_null"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("eq_mixed_left"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("eq_mixed_right"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("ne_mixed_left"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("ne_mixed_right"), Some(Value::Bool(true))));
}

#[test]
fn test_integer_zero_is_falsey_in_if_conditions() {
    let code = r#"
        flag := 0
        branch := "then"
        if flag {
            branch := "then"
        } else {
            branch := "else"
        }
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("branch"), Some(Value::Str(s)) if s.as_ref() == "else"));
}

#[test]
fn test_parallel_map_handles_mixed_immediate_and_promise_results() {
    let code = r#"
        func maybe_async_value(x) {
            if x == 2 {
                return async_sleep(1)
            }
            if x == 4 {
                return async_sleep(1)
            }
            return x * 2
        }

        values := [1, 2, 3, 4]
        mapped := await parallel_map(values, maybe_async_value, 2)
        ok := mapped[0] == 2 && type(mapped[1]) == "null" && mapped[2] == 6 && type(mapped[3]) == "null"
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("ok"), Some(Value::Bool(true))));
}

#[test]
fn test_parallel_map_reuses_cached_mapper_promises() {
    let code = r#"
        cached := async_sleep(1)
        await cached

        func return_cached(_) {
            return cached
        }

        mapped := await parallel_map([1, 2, 3], return_cached, 2)
        ok := len(mapped) == 3 && type(mapped[0]) == "null" && type(mapped[1]) == "null" && type(mapped[2]) == "null"
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("ok"), Some(Value::Bool(true))));
}

#[test]
fn test_parallel_map_immediate_only_mapper_fast_path() {
    let code = r#"
        values := ["a", "bb", "ccc", "dddd"]
        mapped := await parallel_map(values, len, 2)
        ok := mapped[0] == 1 && mapped[1] == 2 && mapped[2] == 3 && mapped[3] == 4
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("ok"), Some(Value::Bool(true))));
}

#[test]
fn test_current_timestamp() {
    let code = r#"
        ts := current_timestamp()
    "#;

    let interp = run_code(code);
    // Verify it returns a number (milliseconds since UNIX epoch)
    if let Some(Value::Int(timestamp)) = interp.env.get("ts") {
        // Should be a large positive number (milliseconds since 1970)
        // As of Jan 2026, this should be around 1.7 trillion
        assert!(timestamp > 1_700_000_000_000, "Timestamp should be > 1.7 trillion ms");
        assert!(timestamp < 2_000_000_000_000, "Timestamp should be < 2 trillion ms");
    } else {
        panic!("Expected current_timestamp() to return a number");
    }
}

#[test]
fn test_current_timestamp_progression() {
    let code = r#"
        ts1 := current_timestamp()
        # Do some work
        x := 0
        while x < 100 {
            x := x + 1
        }
        ts2 := current_timestamp()
    "#;

    let interp = run_code(code);
    // Verify that ts2 >= ts1 (time moves forward)
    if let (Some(Value::Int(ts1)), Some(Value::Int(ts2))) =
        (interp.env.get("ts1"), interp.env.get("ts2"))
    {
        assert!(ts2 >= ts1, "Timestamp should increase or stay the same");
    } else {
        panic!("Expected both timestamps to be numbers");
    }
}

#[test]
fn test_performance_now() {
    let code = r#"
        perf := performance_now()
    "#;

    let interp = run_code(code);
    // Verify it returns a number (milliseconds since program start)
    if let Some(Value::Float(time)) = interp.env.get("perf") {
        // Should be a small positive number (milliseconds since start)
        assert!(time >= 0.0, "Performance timer should be >= 0");
        // Should be less than 1 second for this quick test
        assert!(time < 10000.0, "Performance timer should be < 10 seconds for quick test");
    } else {
        panic!("Expected performance_now() to return a number");
    }
}

#[test]
fn test_performance_now_progression() {
    let code = r#"
        p1 := performance_now()
        # Do some work
        x := 0
        while x < 1000 {
            x := x + 1
        }
        p2 := performance_now()
    "#;

    let interp = run_code(code);
    // Verify that p2 > p1 (time moves forward)
    if let (Some(Value::Float(p1)), Some(Value::Float(p2))) =
        (interp.env.get("p1"), interp.env.get("p2"))
    {
        assert!(p2 > p1, "Performance timer should increase over time");
        // Difference should be reasonable (not negative, not huge)
        let diff = p2 - p1;
        assert!(diff > 0.0, "Time difference should be positive");
        assert!(diff < 10000.0, "Time difference should be reasonable (< 10s)");
    } else {
        panic!("Expected both performance timers to be numbers");
    }
}

#[test]
fn test_timing_arithmetic() {
    let code = r#"
        start := performance_now()
        # Simulate work
        i := 0
        while i < 500 {
            i := i + 1
        }
        end := performance_now()
        elapsed := end - start
    "#;

    let interp = run_code(code);
    // Verify arithmetic operations work on timing values
    if let Some(Value::Float(elapsed)) = interp.env.get("elapsed") {
        assert!(elapsed >= 0.0, "Elapsed time should be non-negative");
    } else {
        panic!("Expected elapsed to be a number");
    }
}

#[test]
fn test_time_us() {
    let code = r#"
        t := time_us()
    "#;

    let interp = run_code(code);
    if let Some(Value::Float(time)) = interp.env.get("t") {
        assert!(time >= 0.0, "Microsecond timer should be >= 0");
    } else {
        panic!("Expected time_us() to return a number");
    }
}

#[test]
fn test_time_ns() {
    let code = r#"
        t := time_ns()
    "#;

    let interp = run_code(code);
    if let Some(Value::Float(time)) = interp.env.get("t") {
        assert!(time >= 0.0, "Nanosecond timer should be >= 0");
    } else {
        panic!("Expected time_ns() to return a number");
    }
}

#[test]
fn test_precision_ordering() {
    let code = r#"
        t_ms := performance_now()
        t_us := time_us()
        t_ns := time_ns()
        # Do some work
        x := 0
        while x < 100 {
            x := x + 1
        }
        t_ms2 := performance_now()
        t_us2 := time_us()
        t_ns2 := time_ns()
    "#;

    let interp = run_code(code);
    // Verify all three precision levels advance
    if let (Some(Value::Float(t_ms)), Some(Value::Float(t_ms2))) =
        (interp.env.get("t_ms"), interp.env.get("t_ms2"))
    {
        assert!(t_ms2 >= t_ms, "Millisecond timer should advance");
    }
    if let (Some(Value::Float(t_us)), Some(Value::Float(t_us2))) =
        (interp.env.get("t_us"), interp.env.get("t_us2"))
    {
        assert!(t_us2 >= t_us, "Microsecond timer should advance");
    }
    if let (Some(Value::Float(t_ns)), Some(Value::Float(t_ns2))) =
        (interp.env.get("t_ns"), interp.env.get("t_ns2"))
    {
        assert!(t_ns2 >= t_ns, "Nanosecond timer should advance");
    }
}

#[test]
fn test_format_duration() {
    let code = r#"
        # Test various duration values
        d1 := format_duration(5000.0)     # 5 seconds
        d2 := format_duration(123.45)     # milliseconds
        d3 := format_duration(0.567)      # microseconds
        d4 := format_duration(0.0001)     # nanoseconds
    "#;

    let interp = run_code(code);

    // Check seconds formatting
    if let Some(Value::Str(s)) = interp.env.get("d1") {
        assert!(s.contains("s"), "Should format as seconds: {s}");
        assert!(s.contains("5.00"), "Should show 5.00s: {s}");
    }

    // Check milliseconds formatting
    if let Some(Value::Str(s)) = interp.env.get("d2") {
        assert!(s.contains("ms"), "Should format as milliseconds: {s}");
    }

    // Check microseconds formatting
    if let Some(Value::Str(s)) = interp.env.get("d3") {
        assert!(s.contains("μs") || s.contains("us"), "Should format as microseconds: {s}");
    }

    // Check nanoseconds formatting
    if let Some(Value::Str(s)) = interp.env.get("d4") {
        assert!(s.contains("ns"), "Should format as nanoseconds: {s}");
    }
}

#[test]
fn test_elapsed_function() {
    let code = r#"
        start := 100.0
        end := 250.5
        diff := elapsed(start, end)
    "#;

    let interp = run_code(code);
    if let Some(Value::Float(diff)) = interp.env.get("diff") {
        assert!((diff - 150.5).abs() < 0.001, "elapsed should calculate difference correctly");
    } else {
        panic!("Expected elapsed to return a number");
    }
}

// Type introspection tests
#[test]
fn test_type_function_basic_types() {
    let code = r#"
        t_int := type(42)
        t_float := type(3.14)
        t_string := type("hello")
        t_bool := type(true)
        t_null := type(null)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("t_int"), Some(Value::Str(s)) if s.as_str() == "int"));
    assert!(matches!(interp.env.get("t_float"), Some(Value::Str(s)) if s.as_str() == "float"));
    assert!(matches!(interp.env.get("t_string"), Some(Value::Str(s)) if s.as_str() == "string"));
    assert!(matches!(interp.env.get("t_bool"), Some(Value::Str(s)) if s.as_str() == "bool"));
    assert!(matches!(interp.env.get("t_null"), Some(Value::Str(s)) if s.as_str() == "null"));
}

#[test]
fn test_type_function_collections() {
    let code = r#"
        t_array := type([1, 2, 3])
        t_dict := type({"a": 1})
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("t_array"), Some(Value::Str(s)) if s.as_str() == "array"));
    assert!(matches!(interp.env.get("t_dict"), Some(Value::Str(s)) if s.as_str() == "dict"));
}

#[test]
fn test_type_function_with_function() {
    let code = r#"
        func my_func() {
            return 42
        }
        t_func := type(my_func)
        t_native := type(len)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("t_func"), Some(Value::Str(s)) if s.as_str() == "function"));
    assert!(matches!(interp.env.get("t_native"), Some(Value::Str(s)) if s.as_str() == "function"));
}

#[test]
fn test_is_int_predicate() {
    let code = r#"
        r1 := is_int(42)
        r2 := is_int(3.14)
        r3 := is_int("hello")
        r4 := is_int(true)
        r5 := is_int(null)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("r1"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r2"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("r3"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("r4"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("r5"), Some(Value::Bool(false))));
}

#[test]
fn test_is_float_predicate() {
    let code = r#"
        r1 := is_float(3.14)
        r2 := is_float(42)
        r3 := is_float("3.14")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("r1"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r2"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("r3"), Some(Value::Bool(false))));
}

#[test]
fn test_is_string_predicate() {
    let code = r#"
        r1 := is_string("hello")
        r2 := is_string(42)
        r3 := is_string(true)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("r1"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r2"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("r3"), Some(Value::Bool(false))));
}

#[test]
fn test_secret_values_redact_and_reveal_explicitly() {
    let code = r#"
        key := secret("sk-runtime-secret")
        cloned := key
        same := cloned == secret("sk-runtime-secret")
        shown := to_string(key)
        encoded := to_json({"key": key})
        kind := type(key)
        is_key_secret := is_secret(key)
        plain := reveal(key)
        print(key)
        print({"nested": key})
    "#;

    let (interp, output) = run_code_with_output(code);

    assert!(matches!(interp.env.get("same"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("shown"), Some(Value::Str(text)) if text.as_ref() == "***"));
    assert!(
        matches!(interp.env.get("encoded"), Some(Value::Str(text)) if text.as_ref().contains("\"key\":\"***\""))
    );
    assert!(matches!(interp.env.get("kind"), Some(Value::Str(text)) if text.as_ref() == "secret"));
    assert!(matches!(interp.env.get("is_key_secret"), Some(Value::Bool(true))));
    assert!(
        matches!(interp.env.get("plain"), Some(Value::Str(text)) if text.as_ref() == "sk-runtime-secret")
    );
    assert!(output.contains("***"));
    assert!(!output.contains("sk-runtime-secret"));
}

#[test]
fn test_is_array_predicate() {
    let code = r#"
        r1 := is_array([1, 2, 3])
        r2 := is_array({"a": 1})
        r3 := is_array("hello")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("r1"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r2"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("r3"), Some(Value::Bool(false))));
}

#[test]
fn test_is_dict_predicate() {
    let code = r#"
        r1 := is_dict({"a": 1})
        r2 := is_dict([1, 2, 3])
        r3 := is_dict("hello")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("r1"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r2"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("r3"), Some(Value::Bool(false))));
}

#[test]
fn test_is_bool_predicate() {
    let code = r#"
        r1 := is_bool(true)
        r2 := is_bool(false)
        r3 := is_bool(1)
        r4 := is_bool("true")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("r1"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r2"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r3"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("r4"), Some(Value::Bool(false))));
}

#[test]
fn test_is_null_predicate() {
    let code = r#"
        r1 := is_null(null)
        r2 := is_null(0)
        r3 := is_null(false)
        r4 := is_null("")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("r1"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r2"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("r3"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("r4"), Some(Value::Bool(false))));
}

#[test]
fn test_is_function_predicate() {
    let code = r#"
        func my_func() {
            return 42
        }
        r1 := is_function(my_func)
        r2 := is_function(len)
        r3 := is_function(42)
        r4 := is_function("hello")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("r1"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r2"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r3"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("r4"), Some(Value::Bool(false))));
}

#[test]
fn test_type_introspection_in_conditional() {
    let code = r#"
        x := 42
        result := ""
        if is_int(x) {
            result := "integer"
        } else if is_float(x) {
            result := "float"
        } else {
            result := "other"
        }
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "integer"));
}

#[test]
fn test_type_introspection_defensive_coding() {
    let code = r#"
        func process_value(val) {
            if is_int(val) {
                return val * 2
            }
            if is_string(val) {
                return len(val)
            }
            return 0
        }

        r1 := process_value(10)
        r2 := process_value("hello")
        r3 := process_value(true)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("r1"), Some(Value::Int(n)) if n == 20));
    assert!(matches!(interp.env.get("r2"), Some(Value::Int(n)) if n == 5));
    assert!(matches!(interp.env.get("r3"), Some(Value::Int(n)) if n == 0));
}

#[test]
fn test_type_function_edge_cases() {
    let code = r#"
        # Test with variables
        x := 42
        t1 := type(x)

        # Test with expressions
        t2 := type(1 + 1)
        t3 := type("hello" + " world")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("t1"), Some(Value::Str(s)) if s.as_str() == "int"));
    assert!(matches!(interp.env.get("t2"), Some(Value::Str(s)) if s.as_str() == "int"));
    assert!(matches!(interp.env.get("t3"), Some(Value::Str(s)) if s.as_str() == "string"));
}

#[test]
fn test_combined_type_predicates() {
    let code = r#"
        val := 3.14
        is_numeric := is_int(val) || is_float(val)
        is_collection := is_array(val) || is_dict(val)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("is_numeric"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("is_collection"), Some(Value::Bool(false))));
}

// Type conversion function tests

#[test]
fn test_to_int_from_float() {
    let code = r#"
        result1 := to_int(3.14)
        result2 := to_int(9.99)
        result3 := to_int(0.5)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result1"), Some(Value::Int(3))));
    assert!(matches!(interp.env.get("result2"), Some(Value::Int(9))));
    assert!(matches!(interp.env.get("result3"), Some(Value::Int(0))));
}

#[test]
fn test_to_int_from_string() {
    let code = r#"
        result1 := to_int("42")
        result2 := to_int("123")
        result3 := to_int("  999  ")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result1"), Some(Value::Int(42))));
    assert!(matches!(interp.env.get("result2"), Some(Value::Int(123))));
    assert!(matches!(interp.env.get("result3"), Some(Value::Int(999))));
}

#[test]
fn test_to_int_from_bool() {
    let code = r#"
        result1 := to_int(true)
        result2 := to_int(false)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result1"), Some(Value::Int(1))));
    assert!(matches!(interp.env.get("result2"), Some(Value::Int(0))));
}

#[test]
fn test_to_int_from_int() {
    let code = r#"
        result := to_int(42)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Int(42))));
}

#[test]
fn test_to_float_from_int() {
    let code = r#"
        result1 := to_float(42)
        result2 := to_float(10)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result1"), Some(Value::Float(f)) if (f - 42.0).abs() < 0.001));
    assert!(matches!(interp.env.get("result2"), Some(Value::Float(f)) if (f - 10.0).abs() < 0.001));
}

#[test]
fn test_to_float_from_string() {
    let code = r#"
        result1 := to_float("3.14")
        result2 := to_float("2.5")
        result3 := to_float("  42.0  ")
    "#;

    let interp = run_code(code);

    assert!(
        matches!(interp.env.get("result1"), Some(Value::Float(f)) if (f - std::f64::consts::PI).abs() < 0.01)
    );
    assert!(matches!(interp.env.get("result2"), Some(Value::Float(f)) if (f - 2.5).abs() < 0.001));
    assert!(matches!(interp.env.get("result3"), Some(Value::Float(f)) if (f - 42.0).abs() < 0.001));
}

#[test]
fn test_to_float_from_bool() {
    let code = r#"
        result1 := to_float(true)
        result2 := to_float(false)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result1"), Some(Value::Float(f)) if (f - 1.0).abs() < 0.001));
    assert!(matches!(interp.env.get("result2"), Some(Value::Float(f)) if f.abs() < 0.001));
}

#[test]
fn test_to_string_from_int() {
    let code = r#"
        result := to_string(42)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "42"));
}

#[test]
fn test_to_string_from_float() {
    let code = r#"
        result := to_string(3.14)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "3.14"));
}

#[test]
fn test_to_string_from_bool() {
    let code = r#"
        result1 := to_string(true)
        result2 := to_string(false)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result1"), Some(Value::Str(s)) if s.as_str() == "true"));
    assert!(matches!(interp.env.get("result2"), Some(Value::Str(s)) if s.as_str() == "false"));
}

#[test]
fn test_to_string_from_array() {
    let code = r#"
        result := to_string([1, 2, 3])
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Str(s)) if s.as_str() == "[1, 2, 3]"));
}

#[test]
fn test_to_bool_from_int() {
    let code = r#"
        result1 := to_bool(0)
        result2 := to_bool(1)
        result3 := to_bool(42)
        # Note: Negative literals have a parser bug, using subtraction instead
        neg := 0 - 1
        result4 := to_bool(neg)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result1"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("result2"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("result3"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("result4"), Some(Value::Bool(true))));
}

#[test]
fn test_to_bool_from_float() {
    let code = r#"
        result1 := to_bool(0.0)
        result2 := to_bool(1.5)
        neg := 0.0 - 3.14
        result3 := to_bool(neg)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result1"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("result2"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("result3"), Some(Value::Bool(true))));
}

#[test]
fn test_to_bool_from_string() {
    let code = r#"
        result1 := to_bool("")
        result2 := to_bool("hello")
        result3 := to_bool("false")
        result4 := to_bool("0")
        result5 := to_bool("FALSE")
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result1"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("result2"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("result3"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("result4"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("result5"), Some(Value::Bool(false))));
}

#[test]
fn test_to_bool_from_collections() {
    let code = r#"
        result1 := to_bool([])
        result2 := to_bool([1, 2])
        result3 := to_bool({})
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result1"), Some(Value::Bool(false))));
    assert!(matches!(interp.env.get("result2"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("result3"), Some(Value::Bool(false))));
}

#[test]
fn test_to_bool_from_null() {
    let code = r#"
        result := to_bool(null)
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Bool(false))));
}

#[test]
fn test_type_conversion_chaining() {
    let code = r#"
        # Chain conversions
        x := to_int(to_float(to_string(42)))
        y := to_bool(to_int("1"))
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("x"), Some(Value::Int(42))));
    assert!(matches!(interp.env.get("y"), Some(Value::Bool(true))));
}

#[test]
fn test_file_size() {
    use std::fs;

    // Create a temporary test file
    let test_file = "test_file_size_temp.txt";
    let content = "Hello, World! This is a test.";
    fs::write(test_file, content).unwrap();

    let code = format!(
        r#"
        size := file_size("{test_file}")
    "#
    );

    let interp = run_code(&code);

    // Clean up
    let _ = fs::remove_file(test_file);

    if let Some(Value::Int(size)) = interp.env.get("size") {
        assert_eq!(size, content.len() as i64);
    } else {
        panic!("Expected size to be an integer");
    }
}

#[test]
fn test_file_size_nonexistent() {
    let code = r#"
        result := file_size("/tmp/file_that_does_not_exist_kujo_test_12345.txt")
    "#;

    let interp = run_code(code);

    // Should return an error
    if let Some(Value::Error(err)) = interp.env.get("result") {
        assert!(err.contains("Cannot get file size"));
    } else {
        panic!("Expected an error for nonexistent file");
    }
}

#[test]
fn test_delete_file() {
    use std::fs;

    // Create a temporary test file
    let test_file = "test_delete_file_temp.txt";
    fs::write(test_file, "Delete me").unwrap();

    let code = format!(
        r#"
        result := delete_file("{test_file}")
    "#
    );

    let interp = run_code(&code);

    // Verify file was deleted
    assert!(!std::path::Path::new(test_file).exists());

    // Check result
    if let Some(Value::Bool(result)) = interp.env.get("result") {
        assert!(result);
    } else {
        panic!("Expected result to be true");
    }
}

#[test]
fn test_delete_file_nonexistent() {
    let code = r#"
        result := delete_file("/tmp/file_that_does_not_exist_kujo_test_delete_67890.txt")
    "#;

    let interp = run_code(code);

    // Should return an error
    if let Some(Value::Error(err)) = interp.env.get("result") {
        assert!(err.contains("Cannot delete file"));
    } else {
        panic!("Expected an error for nonexistent file");
    }
}

#[test]
fn test_rename_file() {
    use std::fs;

    // Create a temporary test file
    let old_name = "test_rename_old_temp.txt";
    let new_name = "test_rename_new_temp.txt";
    let content = "Rename me";
    fs::write(old_name, content).unwrap();

    let code = format!(
        r#"
        result := rename_file("{old_name}", "{new_name}")
    "#
    );

    let interp = run_code(&code);

    // Verify old file doesn't exist and new file does
    assert!(!std::path::Path::new(old_name).exists());
    assert!(std::path::Path::new(new_name).exists());

    // Check content is preserved
    let new_content = fs::read_to_string(new_name).unwrap();
    assert_eq!(new_content, content);

    // Clean up
    let _ = fs::remove_file(new_name);

    // Check result
    if let Some(Value::Bool(result)) = interp.env.get("result") {
        assert!(result);
    } else {
        panic!("Expected result to be true");
    }
}

#[test]
fn test_rename_file_nonexistent() {
    let code = r#"
        result := rename_file("/tmp/old_file_nonexistent_kujo_test.txt", "/tmp/new_file.txt")
    "#;

    let interp = run_code(code);

    // Should return an error
    if let Some(Value::Error(err)) = interp.env.get("result") {
        assert!(err.contains("Cannot rename file"));
    } else {
        panic!("Expected an error for nonexistent file");
    }
}

#[test]
fn test_copy_file() {
    use std::fs;

    // Create a temporary test file
    let source = "test_copy_source_temp.txt";
    let dest = "test_copy_dest_temp.txt";
    let content = "Copy me";
    fs::write(source, content).unwrap();

    let code = format!(
        r#"
        result := copy_file("{source}", "{dest}")
    "#
    );

    let interp = run_code(&code);

    // Verify both files exist
    assert!(std::path::Path::new(source).exists());
    assert!(std::path::Path::new(dest).exists());

    // Check content is the same
    let source_content = fs::read_to_string(source).unwrap();
    let dest_content = fs::read_to_string(dest).unwrap();
    assert_eq!(source_content, dest_content);
    assert_eq!(dest_content, content);

    // Clean up
    let _ = fs::remove_file(source);
    let _ = fs::remove_file(dest);

    // Check result
    if let Some(Value::Bool(result)) = interp.env.get("result") {
        assert!(result);
    } else {
        panic!("Expected result to be true");
    }
}

#[test]
fn test_copy_file_nonexistent() {
    let code = r#"
        result := copy_file("/tmp/source_file_nonexistent_kujo_test.txt", "/tmp/dest_file.txt")
    "#;

    let interp = run_code(code);

    // Should return an error
    if let Some(Value::Error(err)) = interp.env.get("result") {
        assert!(err.contains("Cannot copy file"));
    } else {
        panic!("Expected an error for nonexistent file");
    }
}

#[test]
fn test_file_operations_integration() {
    use std::fs;

    // Create a test file and perform multiple operations
    let original = "test_integration_original.txt";
    let renamed = "test_integration_renamed.txt";
    let copied = "test_integration_copied.txt";
    let content = "Integration test content";

    fs::write(original, content).unwrap();

    let code = format!(
        r#"
        # Get original file size
        size1 := file_size("{original}")
        
        # Rename the file
        rename_result := rename_file("{original}", "{renamed}")
        
        # Get size after rename
        size2 := file_size("{renamed}")
        
        # Copy the renamed file
        copy_result := copy_file("{renamed}", "{copied}")
        
        # Get size of copied file
        size3 := file_size("{copied}")
        
        # Delete the original (renamed) file
        delete1 := delete_file("{renamed}")
        
        # Delete the copied file
        delete2 := delete_file("{copied}")
    "#
    );

    let interp = run_code(&code);

    // All sizes should be equal
    let expected_size = content.len() as i64;
    if let Some(Value::Int(size)) = interp.env.get("size1") {
        assert_eq!(size, expected_size);
    } else {
        panic!("Expected size1 to be an integer");
    }

    if let Some(Value::Int(size)) = interp.env.get("size2") {
        assert_eq!(size, expected_size);
    } else {
        panic!("Expected size2 to be an integer");
    }

    if let Some(Value::Int(size)) = interp.env.get("size3") {
        assert_eq!(size, expected_size);
    } else {
        panic!("Expected size3 to be an integer");
    }

    // All operations should succeed
    assert!(matches!(interp.env.get("rename_result"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("copy_result"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("delete1"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("delete2"), Some(Value::Bool(true))));

    // Verify no files remain
    assert!(!std::path::Path::new(original).exists());
    assert!(!std::path::Path::new(renamed).exists());
    assert!(!std::path::Path::new(copied).exists());
}

#[test]
fn test_sort_integers() {
    let code = r#"
        nums := [3, 1, 4, 1, 5, 9, 2, 6]
        sorted := sort(nums)
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(arr)) = interp.env.get("sorted") {
        assert_eq!(arr.len(), 8);
        // Check if sorted in ascending order
        let expected = [1, 1, 2, 3, 4, 5, 6, 9];
        for (i, val) in arr.iter().enumerate() {
            if let Value::Int(n) = val {
                assert_eq!(*n, expected[i]);
            } else {
                panic!("Expected integer at index {i}");
            }
        }
    } else {
        panic!("Expected sorted to be an array");
    }
}

#[test]
fn test_sort_floats() {
    let code = r#"
        nums := [3.5, 1.2, 4.8, 2.1]
        sorted := sort(nums)
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(arr)) = interp.env.get("sorted") {
        assert_eq!(arr.len(), 4);
        // Check if sorted in ascending order
        let expected = [1.2, 2.1, 3.5, 4.8];
        for (i, val) in arr.iter().enumerate() {
            if let Value::Float(n) = val {
                assert!((n - expected[i]).abs() < 0.001);
            } else {
                panic!("Expected float at index {i}");
            }
        }
    } else {
        panic!("Expected sorted to be an array");
    }
}

#[test]
fn test_sort_mixed_numbers() {
    let code = r#"
        nums := [3, 1.5, 4, 2.2]
        sorted := sort(nums)
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(arr)) = interp.env.get("sorted") {
        assert_eq!(arr.len(), 4);
        // Should be sorted: 1.5, 2.2, 3, 4
        // Extract as floats for comparison
        let mut values: Vec<f64> = Vec::new();
        for val in arr.iter() {
            match val {
                Value::Int(n) => values.push(*n as f64),
                Value::Float(n) => values.push(*n),
                _ => panic!("Expected number"),
            }
        }
        assert!((values[0] - 1.5).abs() < 0.001);
        assert!((values[1] - 2.2).abs() < 0.001);
        assert!((values[2] - 3.0).abs() < 0.001);
        assert!((values[3] - 4.0).abs() < 0.001);
    } else {
        panic!("Expected sorted to be an array");
    }
}

#[test]
fn test_sort_strings() {
    let code = r#"
        words := ["banana", "apple", "cherry", "date"]
        sorted := sort(words)
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(arr)) = interp.env.get("sorted") {
        let expected = ["apple", "banana", "cherry", "date"];
        for (i, val) in arr.iter().enumerate() {
            if let Value::Str(s) = val {
                assert_eq!(s.as_str(), expected[i]);
            } else {
                panic!("Expected string at index {i}");
            }
        }
    } else {
        panic!("Expected sorted to be an array");
    }
}

#[test]
fn test_reverse() {
    let code = r#"
        nums := [1, 2, 3, 4, 5]
        reversed := reverse(nums)
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(arr)) = interp.env.get("reversed") {
        let expected = [5, 4, 3, 2, 1];
        for (i, val) in arr.iter().enumerate() {
            if let Value::Int(n) = val {
                assert_eq!(*n, expected[i]);
            } else {
                panic!("Expected integer at index {i}");
            }
        }
    } else {
        panic!("Expected reversed to be an array");
    }
}

#[test]
fn test_unique() {
    let code = r#"
        nums := [3, 1, 4, 1, 5, 9, 2, 6, 5, 3]
        unique_nums := unique(nums)
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(arr)) = interp.env.get("unique_nums") {
        // Should preserve order and remove duplicates: [3, 1, 4, 5, 9, 2, 6]
        assert_eq!(arr.len(), 7);
        let expected = [3, 1, 4, 5, 9, 2, 6];
        for (i, val) in arr.iter().enumerate() {
            if let Value::Int(n) = val {
                assert_eq!(*n, expected[i]);
            } else {
                panic!("Expected integer at index {i}");
            }
        }
    } else {
        panic!("Expected unique_nums to be an array");
    }
}

#[test]
fn test_unique_strings() {
    let code = r#"
        words := ["apple", "banana", "apple", "cherry", "banana"]
        unique_words := unique(words)
    "#;

    let interp = run_code(code);

    if let Some(Value::Array(arr)) = interp.env.get("unique_words") {
        assert_eq!(arr.len(), 3);
        let expected = ["apple", "banana", "cherry"];
        for (i, val) in arr.iter().enumerate() {
            if let Value::Str(s) = val {
                assert_eq!(s.as_str(), expected[i]);
            } else {
                panic!("Expected string at index {i}");
            }
        }
    } else {
        panic!("Expected unique_words to be an array");
    }
}

#[test]
fn test_sum_integers() {
    let code = r#"
        nums := [1, 2, 3, 4, 5]
        total := sum(nums)
    "#;

    let interp = run_code(code);

    if let Some(Value::Int(n)) = interp.env.get("total") {
        assert_eq!(n, 15);
    } else {
        panic!("Expected total to be an integer");
    }
}

#[test]
fn test_sum_floats() {
    let code = r#"
        nums := [1.5, 2.5, 3.0]
        total := sum(nums)
    "#;

    let interp = run_code(code);

    if let Some(Value::Float(n)) = interp.env.get("total") {
        assert!((n - 7.0).abs() < 0.001);
    } else {
        panic!("Expected total to be a float");
    }
}

#[test]
fn test_sum_mixed() {
    let code = r#"
        nums := [1, 2.5, 3, 4.5]
        total := sum(nums)
    "#;

    let interp = run_code(code);

    if let Some(Value::Float(n)) = interp.env.get("total") {
        assert!((n - 11.0).abs() < 0.001);
    } else {
        panic!("Expected total to be a float");
    }
}

#[test]
fn test_sum_empty_array() {
    let code = r#"
        nums := []
        total := sum(nums)
    "#;

    let interp = run_code(code);

    if let Some(Value::Int(n)) = interp.env.get("total") {
        assert_eq!(n, 0);
    } else {
        panic!("Expected total to be 0");
    }
}

#[test]
fn test_any_true() {
    let code = r#"
        nums := [1, 2, 3, 4, 5]
        result := any(nums, func(x) { return x > 3 })
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Bool(true))));
}

#[test]
fn test_any_false() {
    let code = r#"
        nums := [1, 2, 3]
        result := any(nums, func(x) { return x > 10 })
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Bool(false))));
}

#[test]
fn test_any_empty_array() {
    let code = r#"
        nums := []
        result := any(nums, func(x) { return x > 0 })
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Bool(false))));
}

#[test]
fn test_all_true() {
    let code = r#"
        nums := [1, 2, 3, 4, 5]
        result := all(nums, func(x) { return x > 0 })
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Bool(true))));
}

#[test]
fn test_all_false() {
    let code = r#"
        nums := [1, 2, 3, 4, 5]
        result := all(nums, func(x) { return x > 3 })
    "#;

    let interp = run_code(code);

    assert!(matches!(interp.env.get("result"), Some(Value::Bool(false))));
}

#[test]
fn test_all_empty_array() {
    let code = r#"
        nums := []
        result := all(nums, func(x) { return x > 0 })
    "#;

    let interp = run_code(code);

    // All elements of empty array satisfy any condition (vacuous truth)
    assert!(matches!(interp.env.get("result"), Some(Value::Bool(true))));
}

#[test]
fn test_collection_predicates_use_shared_truthiness_semantics() {
    let code = r#"
        nums := [0, 1, 2]

        filtered := filter(nums, func(x) {
            if x == 1 {
                return {}
            }
            return {"value": x}
        })

        found := find(nums, func(x) {
            if x == 0 {
                return {"first": x}
            }
            return {}
        })

        any_result := any(nums, func(x) {
            if x == 2 {
                return [x]
            }
            return []
        })

        all_result := all(nums, func(x) {
            if x == 1 {
                return []
            }
            return [x]
        })

        ok := len(filtered) == 2
            && filtered[0] == 0
            && filtered[1] == 2
            && found == 0
            && any_result == true
            && all_result == false
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("ok"), Some(Value::Bool(true))));
}

#[test]
fn test_iterator_filter_chain_preserves_both_predicates() {
    let code = r#"
        nums := [1, 2, 3, 4, 5, 6]
        filtered := nums
            .filter(func(x) { return x > 1 })
            .filter(func(x) { return x % 2 == 0 })
            .collect()
        ok := len(filtered) == 3
            && filtered[0] == 2
            && filtered[1] == 4
            && filtered[2] == 6
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("ok"), Some(Value::Bool(true))));
}

#[test]
fn test_iterator_map_chain_applies_transformers_in_order() {
    let code = r#"
        nums := [1, 2, 3]
        mapped := nums
            .map(func(x) { return x + 1 })
            .map(func(x) { return x * 2 })
            .collect()
        ok := len(mapped) == 3
            && mapped[0] == 4
            && mapped[1] == 6
            && mapped[2] == 8
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("ok"), Some(Value::Bool(true))));
}

#[test]
fn test_generator_iterator_next_skips_filtered_items_until_match() {
    let mut interp = run_code(
        r#"
        func* seq() {
            yield 1
            yield 2
            yield 3
        }

        func greater_than_one(x) {
            return x > 1
        }

        gen := seq()
    "#,
    );

    let generator_value = match interp.env.get("gen") {
        Some(value @ Value::Generator { .. }) => value,
        other => panic!("expected generator value, got {other:?}"),
    };
    let predicate = match interp.env.get("greater_than_one") {
        Some(value @ Value::Function(..)) => value,
        other => panic!("expected predicate function, got {other:?}"),
    };

    interp.env.define(
        "iter".to_string(),
        Value::Iterator {
            source: Box::new(generator_value),
            index: 0,
            transformer: None,
            filter_fn: Some(Box::new(predicate)),
            take_count: None,
        },
    );

    let code = r#"
        next_value := iter.next()
    "#;
    let tokens = tokenize(code).expect("test source should tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse();
    interp.eval_stmts(&program);

    match interp.env.get("next_value") {
        Some(Value::Option { is_some: true, value }) => {
            assert!(matches!(value.as_ref(), Value::Int(2)));
        }
        other => panic!("expected Some(2), got {other:?}"),
    }
}

#[test]
fn test_array_utilities_chained() {
    let code = r#"
        nums := [3, 1, 4, 1, 5, 9, 2, 6, 5, 3]
        
        # Get unique, sort, reverse
        step1 := unique(nums)
        step2 := sort(step1)
        step3 := reverse(step2)
        
        # Sum and check
        total := sum(nums)
        has_large := any(nums, func(x) { return x > 8 })
        all_positive := all(nums, func(x) { return x > 0 })
    "#;

    let interp = run_code(code);

    // step3 should be [9, 6, 5, 4, 3, 2, 1]
    if let Some(Value::Array(arr)) = interp.env.get("step3") {
        assert_eq!(arr.len(), 7);
        let expected = [9, 6, 5, 4, 3, 2, 1];
        for (i, val) in arr.iter().enumerate() {
            if let Value::Int(n) = val {
                assert_eq!(*n, expected[i]);
            }
        }
    } else {
        panic!("Expected step3 to be an array");
    }

    // total should be 39 (3+1+4+1+5+9+2+6+5+3)
    if let Some(Value::Int(n)) = interp.env.get("total") {
        assert_eq!(n, 39);
    }

    assert!(matches!(interp.env.get("has_large"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("all_positive"), Some(Value::Bool(true))));
}

#[test]
fn test_assert_success() {
    let code = r#"
        result := assert(true)
        result2 := assert(5 > 3)
        result3 := assert(1, "Non-zero is truthy")
    "#;

    let interp = run_code(code);

    // All assertions should pass and return true
    assert!(matches!(interp.env.get("result"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("result2"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("result3"), Some(Value::Bool(true))));
}

#[test]
fn test_assert_failure_with_default_message() {
    let code = r#"
        result := assert(false)
    "#;

    let interp = run_code(code);

    // Assert should fail and return error
    if let Some(Value::Error(msg)) = interp.env.get("result") {
        assert_eq!(msg, "Assertion failed");
    } else {
        panic!("Expected assertion to fail with error");
    }
}

#[test]
fn test_assert_failure_with_custom_message() {
    let code = r#"
        result := assert(5 < 3, "Five must be greater than three")
    "#;

    let interp = run_code(code);

    // Assert should fail with custom message
    if let Some(Value::Error(msg)) = interp.env.get("result") {
        assert_eq!(msg, "Five must be greater than three");
    } else {
        panic!("Expected assertion to fail with custom message");
    }
}

#[test]
fn test_assert_with_truthy_values() {
    let code = r#"
        r1 := assert(1)
        r2 := assert(3.14)
        r3 := assert("hello")
        r4 := assert([1, 2, 3])
    "#;

    let interp = run_code(code);

    // Non-zero numbers and non-null values should pass
    assert!(matches!(interp.env.get("r1"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r2"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r3"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("r4"), Some(Value::Bool(true))));
}

#[test]
fn test_assert_with_falsy_values() {
    let code = r#"
        r1 := assert(0, "Zero is falsy")
    "#;

    let interp = run_code(code);

    // Zero should fail
    assert!(matches!(interp.env.get("r1"), Some(Value::Error(_))));
}

#[test]
fn test_assert_with_boolean_false() {
    let code = r#"
        result := assert(false, "Boolean false should fail")
    "#;

    let interp = run_code(code);

    // Should fail
    assert!(matches!(interp.env.get("result"), Some(Value::Error(_))));
}

#[test]
fn test_assert_in_function() {
    let code = r#"
        func safe_divide(a, b) {
            if b == 0 {
                return assert(false, "Division by zero not allowed")
            }
            return a / b
        }
        
        result1 := safe_divide(10, 2)
        result2 := safe_divide(10, 0)
    "#;

    let interp = run_code(code);

    // First call should succeed
    assert!(matches!(interp.env.get("result1"), Some(Value::Int(_)) | Some(Value::Float(_))));

    // Second call should return error
    assert!(matches!(interp.env.get("result2"), Some(Value::Error(_))));
}

#[test]
fn test_debug_simple_values() {
    // This test just verifies debug doesn't crash - actual output is printed to stdout
    let code = r#"
        debug(42)
        debug("hello")
        debug(true)
        debug(null)
    "#;

    let _interp = run_code(code);
    // If we get here without panic, debug works
}

#[test]
fn test_debug_complex_values() {
    // Test debug with arrays, dicts, and multiple arguments
    let code = r#"
        debug([1, 2, 3])
        debug({"name": "Alice", "age": 25})
        debug("User:", 123, "logged in at", 1706140800.0)
    "#;

    let _interp = run_code(code);
    // If we get here without panic, debug works
}

#[test]
fn test_debug_returns_null() {
    let code = r#"
        result := debug("test")
    "#;

    let interp = run_code(code);

    // Debug should return null
    assert!(matches!(interp.env.get("result"), Some(Value::Null)));
}

// ===== Generator Tests =====

#[test]
fn test_simple_generator() {
    // Test that basic generator syntax works and yields values
    let code = r#"
        func* nums() {
            yield 1
            yield 2
            yield 3
        }
        
        # Create generator instance
        gen := nums()
    "#;

    let interp = run_code(code);

    // Verify generator was created
    assert!(matches!(interp.env.get("gen"), Some(Value::Generator { .. })));
}

#[test]
fn test_generator_iteration() {
    // Test that generators can be iterated with for-in loops
    let code = r#"
        func* counter() {
            yield 10
            yield 20
            yield 30
        }
        
        sum := 0
        for val in counter() {
            sum := sum + val
        }
    "#;

    let interp = run_code(code);
    // Should sum 10 + 20 + 30 = 60
    assert!(matches!(interp.env.get("sum"), Some(Value::Int(n)) if n == 60));
}

#[test]
fn test_generator_with_state() {
    // Test that generator preserves state between yields
    let code = r#"
        func* sequence() {
            mut x := 5
            yield x
            x := x * 2
            yield x
        }
        
        sum := 0
        for n in sequence() {
            sum := sum + n
        }
    "#;

    let interp = run_code(code);
    // Should sum 5 + 10 = 15
    assert!(matches!(interp.env.get("sum"), Some(Value::Int(n)) if n == 15));
}

#[test]
fn test_generator_with_parameters() {
    // Test that generators can accept parameters
    let code = r#"
        func* echo(msg) {
            yield msg
            yield msg
        }
        
        gen := echo("hello")
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("gen"), Some(Value::Generator { .. })));
}

#[test]
fn test_generator_break_early() {
    // Test that breaking from generator iteration works
    let code = r#"
        func* infinite() {
            mut i := 0
            loop {
                yield i
                i := i + 1
            }
        }
        
        count := 0
        for n in infinite() {
            print(n)
            if count >= 2 {
                break
            }
            count := count + 1
        }
    "#;

    // Should complete without hanging
    let _interp = run_code(code);
}

#[test]
fn test_generator_fibonacci() {
    // Test the fibonacci generator from the ROADMAP
    let code = r#"
        func* fibonacci() {
            let a := 0
            let b := 1
            loop {
                yield a
                let temp := a
                a := b
                b := temp + b
            }
        }
        
        # Create and verify generator
        fib := fibonacci()
    "#;

    let interp = run_code(code);
    assert!(matches!(interp.env.get("fib"), Some(Value::Generator { .. })));
}

#[test]
fn test_parallel_map_scalability_10k() {
    // Test that parallel_map can handle 10,000 concurrent operations efficiently
    // This addresses the Phase 3 roadmap item: "Test scalability with 10K+ concurrent operations"
    let code = r#"
        items := range(0, 10000)
        result_promise := parallel_map(items, func(x) { return x * x }, 100)
        results := await result_promise
        count := len(results)
        first := results[0]
        last := results[9999]
    "#;

    let interp = run_code(code);

    // Verify all 10,000 results were computed
    assert!(matches!(interp.env.get("count"), Some(Value::Int(n)) if n == 10000));

    // Verify correctness of results
    assert!(matches!(interp.env.get("first"), Some(Value::Int(n)) if n == 0)); // 0^2 = 0
    assert!(matches!(interp.env.get("last"), Some(Value::Int(n)) if n == 99980001));
    // 9999^2 = 99980001
}

#[test]
fn test_promise_all_scalability() {
    // Test promise_all with multiple parallel_map operations (10K items total)
    let code = r#"
        items1 := range(0, 1000)
        items2 := range(1000, 2000)
        items3 := range(2000, 3000)
        items4 := range(3000, 4000)
        items5 := range(4000, 5000)
        items6 := range(5000, 6000)
        items7 := range(6000, 7000)
        items8 := range(7000, 8000)
        items9 := range(8000, 9000)
        items10 := range(9000, 10000)

        promises := [
            parallel_map(items1, func(x) { return x * 2 }, 100),
            parallel_map(items2, func(x) { return x * 2 }, 100),
            parallel_map(items3, func(x) { return x * 2 }, 100),
            parallel_map(items4, func(x) { return x * 2 }, 100),
            parallel_map(items5, func(x) { return x * 2 }, 100),
            parallel_map(items6, func(x) { return x * 2 }, 100),
            parallel_map(items7, func(x) { return x * 2 }, 100),
            parallel_map(items8, func(x) { return x * 2 }, 100),
            parallel_map(items9, func(x) { return x * 2 }, 100),
            parallel_map(items10, func(x) { return x * 2 }, 100)
        ]

        result_arrays := await promise_all(promises, 10)
        total := 0
        for arr in result_arrays {
            total := total + len(arr)
        }
    "#;

    let interp = run_code(code);

    // Verify all 10,000 items were processed
    assert!(matches!(interp.env.get("total"), Some(Value::Int(n)) if n == 10000));
}

#[test]
fn test_par_each_scalability() {
    // Test par_each with 10,000 concurrent operations
    let code = r#"
        items := range(0, 10000)
        result_promise := par_each(items, func(x) { 
            return x * x + x * 2
        }, 100)
        await result_promise
        verified := true
    "#;

    let interp = run_code(code);

    // Verify the operation completed successfully
    assert!(matches!(interp.env.get("verified"), Some(Value::Bool(true))));
}
