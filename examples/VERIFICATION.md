# Example Verification

Verified on 2026-08-07. Every tracked `.kujo` file under `examples/` passes the repository example contract: deterministic examples run through the interpreter, the test-framework example passes `test-run`, and effectful, interactive, long-running, or diagnostic examples pass `kujo check`.

Summary: 230 total; 36 runtime-smoked; 1 test-run; 193 syntax-checked; 0 expected failures.

| Example | Verification | Status |
|---|---|---|
| `examples/00-hello.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/01-variables.kujo` | Syntax (`kujo check`) | PASS |
| `examples/02-functions.kujo` | Syntax (`kujo check`) | PASS |
| `examples/03-control-flow.kujo` | Syntax (`kujo check`) | PASS |
| `examples/04-data.kujo` | Syntax (`kujo check`) | PASS |
| `examples/05-modules.kujo` | Syntax (`kujo check`) | PASS |
| `examples/06-agent-tool.kujo` | Syntax (`kujo check`) | PASS |
| `examples/ai_egress_allowlist.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/ai_enterprise_replay_showcase.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/ai_multimodal_messages.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/ai_replay_cassette.kujo` | Syntax (`kujo check`) | PASS |
| `examples/ai_request_hash.kujo` | Syntax (`kujo check`) | PASS |
| `examples/ai_response_envelope.kujo` | Syntax (`kujo check`) | PASS |
| `examples/ai_stream_callback.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/ai_token_budget.kujo` | Syntax (`kujo check`) | PASS |
| `examples/arg_parser_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/array_higher_order.kujo` | Syntax (`kujo check`) | PASS |
| `examples/array_utilities_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/arrays.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/assert_debug_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/async_await_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/async_comprehensive_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/async_with_check.kujo` | Syntax (`kujo check`) | PASS |
| `examples/await_test.kujo` | Syntax (`kujo check`) | PASS |
| `examples/backup_tool.kujo` | Syntax (`kujo check`) | PASS |
| `examples/basic_import.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmark.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmark_async.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/benchmark_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmark_fib.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmark_jit.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmark_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/array_ops.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/array_ops_comparison.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/bench_fib30.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/debug_assign.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/debug_loop.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/dict_ops.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/dict_ops_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/fib_recursive.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/fibonacci.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/file_io.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/func_calls.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/higher_order.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/jit/arithmetic_intensive.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/jit/comparison_generic.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/jit/comparison_specialized.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/jit/loop_nested.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/jit/run_all.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/jit/variable_heavy.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/json_parsing.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/math_ops.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/nested_loops.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/nested_loops_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/primes.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/run_all.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/run_benchmarks.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/simple_arithmetic.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/simple_loop_test.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/sorting.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/sorting_algorithms.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/benchmarks/string_ops.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/string_processing.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/benchmarks/strings.kujo` | Syntax (`kujo check`) | PASS |
| `examples/benchmarks/struct_ops.kujo` | Syntax (`kujo check`) | PASS |
| `examples/binary_file_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/builtins.kujo` | Syntax (`kujo check`) | PASS |
| `examples/cli_tool.kujo` | Syntax (`kujo check`) | PASS |
| `examples/closures_advanced.kujo` | Syntax (`kujo check`) | PASS |
| `examples/closures_counter.kujo` | Syntax (`kujo check`) | PASS |
| `examples/closures_higher_order.kujo` | Syntax (`kujo check`) | PASS |
| `examples/closures_nested.kujo` | Syntax (`kujo check`) | PASS |
| `examples/closures_partial.kujo` | Syntax (`kujo check`) | PASS |
| `examples/collections.kujo` | Syntax (`kujo check`) | PASS |
| `examples/collections_advanced.kujo` | Syntax (`kujo check`) | PASS |
| `examples/comments.kujo` | Syntax (`kujo check`) | PASS |
| `examples/concurrency_channels.kujo` | Syntax (`kujo check`) | PASS |
| `examples/concurrency_parallel_http.kujo` | Syntax (`kujo check`) | PASS |
| `examples/concurrency_spawn.kujo` | Syntax (`kujo check`) | PASS |
| `examples/config_manager.kujo` | Syntax (`kujo check`) | PASS |
| `examples/crypto_aes_example.kujo` | Syntax (`kujo check`) | PASS |
| `examples/crypto_rsa_example.kujo` | Syntax (`kujo check`) | PASS |
| `examples/csv_demo.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/database_mysql.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/database_pooling.kujo` | Syntax (`kujo check`) | PASS |
| `examples/database_postgres.kujo` | Syntax (`kujo check`) | PASS |
| `examples/database_transactions.kujo` | Syntax (`kujo check`) | PASS |
| `examples/database_unified.kujo` | Syntax (`kujo check`) | PASS |
| `examples/datetime_utility.kujo` | Syntax (`kujo check`) | PASS |
| `examples/debug_parse.kujo` | Syntax (`kujo check`) | PASS |
| `examples/demo_result.kujo` | Syntax (`kujo check`) | PASS |
| `examples/destructuring_demo.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/dictionaries.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/directory_tools.kujo` | Syntax (`kujo check`) | PASS |
| `examples/env_config.kujo` | Syntax (`kujo check`) | PASS |
| `examples/error_handling.kujo` | Syntax (`kujo check`) | PASS |
| `examples/error_handling_comprehensive.kujo` | Syntax (`kujo check`) | PASS |
| `examples/error_handling_enhanced.kujo` | Syntax (`kujo check`) | PASS |
| `examples/expense_tracker.kujo` | Syntax (`kujo check`) | PASS |
| `examples/file_logger.kujo` | Syntax (`kujo check`) | PASS |
| `examples/file_operations_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/for_loops.kujo` | Syntax (`kujo check`) | PASS |
| `examples/generators_test.kujo` | Syntax (`kujo check`) | PASS |
| `examples/guessing_game.kujo` | Syntax (`kujo check`) | PASS |
| `examples/hello.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/helper_hlp_007_text_time.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/helper_hlp_011_env_config.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/helper_hlp_013_process_result.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/helper_hlp_015_canonical_json.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/http_client.kujo` | Syntax (`kujo check`) | PASS |
| `examples/http_download.kujo` | Syntax (`kujo check`) | PASS |
| `examples/http_headers_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/http_rest_api.kujo` | Syntax (`kujo check`) | PASS |
| `examples/http_server_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/http_streaming.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/http_webhook.kujo` | Syntax (`kujo check`) | PASS |
| `examples/image_processing.kujo` | Syntax (`kujo check`) | PASS |
| `examples/interactive_calculator.kujo` | Syntax (`kujo check`) | PASS |
| `examples/interactive_greeting.kujo` | Syntax (`kujo check`) | PASS |
| `examples/io_module_demo.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/iterators_comprehensive.kujo` | Syntax (`kujo check`) | PASS |
| `examples/iterators_test.kujo` | Syntax (`kujo check`) | PASS |
| `examples/jit_loop_test.kujo` | Syntax (`kujo check`) | PASS |
| `examples/jit_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/json_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/json_schema_validate.kujo` | Syntax (`kujo check`) | PASS |
| `examples/jwt_auth.kujo` | Syntax (`kujo check`) | PASS |
| `examples/log_parser_regex.kujo` | Syntax (`kujo check`) | PASS |
| `examples/loop_control_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/math_module.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/method_chaining.kujo` | Syntax (`kujo check`) | PASS |
| `examples/minimal_async.kujo` | Syntax (`kujo check`) | PASS |
| `examples/mixed_types.kujo` | Syntax (`kujo check`) | PASS |
| `examples/note_taking_app.kujo` | Syntax (`kujo check`) | PASS |
| `examples/oauth_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/operator_overloading.kujo` | Syntax (`kujo check`) | PASS |
| `examples/password_generator.kujo` | Syntax (`kujo check`) | PASS |
| `examples/path_utilities.kujo` | Syntax (`kujo check`) | PASS |
| `examples/pattern_matching.kujo` | Syntax (`kujo check`) | PASS |
| `examples/project_api_tester.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/project_data_pipeline.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/project_log_analyzer.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/project_markdown_converter.kujo` | Syntax (`kujo check`) | PASS |
| `examples/project_task_manager.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/project_web_scraper.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/projects/ai_model_comparison.kujo` | Syntax (`kujo check`) | PASS |
| `examples/projects/blog_api.kujo` | Syntax (`kujo check`) | PASS |
| `examples/projects/contact_manager.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/projects/data_analyzer.kujo` | Syntax (`kujo check`) | PASS |
| `examples/projects/inventory_system.kujo` | Syntax (`kujo check`) | PASS |
| `examples/projects/jwt_auth_api.kujo` | Syntax (`kujo check`) | PASS |
| `examples/projects/log_parser.kujo` | Syntax (`kujo check`) | PASS |
| `examples/projects/oauth_github_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/projects/streaming_downloader.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/projects/todo_manager.kujo` | Syntax (`kujo check`) | PASS |
| `examples/projects/url_shortener.kujo` | Syntax (`kujo check`) | PASS |
| `examples/projects/weather_dashboard.kujo` | Syntax (`kujo check`) | PASS |
| `examples/quiz_game.kujo` | Syntax (`kujo check`) | PASS |
| `examples/random_generator.kujo` | Syntax (`kujo check`) | PASS |
| `examples/result_option_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/scoping.kujo` | Syntax (`kujo check`) | PASS |
| `examples/scoping_simple.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/secrets_redaction.kujo` | Syntax (`kujo check`) | PASS |
| `examples/selective_import.kujo` | Syntax (`kujo check`) | PASS |
| `examples/showcase.kujo` | Syntax (`kujo check`) | PASS |
| `examples/simple_async_test.kujo` | Syntax (`kujo check`) | PASS |
| `examples/spread_operator_demo.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/stdlib_compression.kujo` | Syntax (`kujo check`) | PASS |
| `examples/stdlib_crypto.kujo` | Syntax (`kujo check`) | PASS |
| `examples/stdlib_os.kujo` | Syntax (`kujo check`) | PASS |
| `examples/stdlib_path.kujo` | Syntax (`kujo check`) | PASS |
| `examples/stdlib_process.kujo` | Syntax (`kujo check`) | PASS |
| `examples/stdlib_test.kujo` | Syntax (`kujo check`) | PASS |
| `examples/string_functions.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/string_interpolation.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/struct_basic.kujo` | Syntax (`kujo check`) | PASS |
| `examples/struct_methods.kujo` | Syntax (`kujo check`) | PASS |
| `examples/struct_nested.kujo` | Syntax (`kujo check`) | PASS |
| `examples/struct_self_methods.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/structs_comprehensive.kujo` | Syntax (`kujo check`) | PASS |
| `examples/student_grade_tracker.kujo` | Syntax (`kujo check`) | PASS |
| `examples/system_info.kujo` | Syntax (`kujo check`) | PASS |
| `examples/tcp_client.kujo` | Syntax (`kujo check`) | PASS |
| `examples/tcp_echo_server.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_async_phase5.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_async_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_bool.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_closure_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_countdown.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_debug_todo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_error.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_errors_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_factorial.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_factorial_compact.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_factorial_debug.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_field_assign.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_if_assign.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_if_else.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_mult.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_mult_debug.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_parallel_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_print.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_scalability_10k.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_simple_func.kujo` | Syntax (`kujo check`) | PASS |
| `examples/test_todo_issue.kujo` | Syntax (`kujo check`) | PASS |
| `examples/testing_demo.kujo` | Tests (`kujo test-run`) | PASS |
| `examples/timing_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/toml_demo.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/try_throw.kujo` | Syntax (`kujo check`) | PASS |
| `examples/type_annotations.kujo` | Syntax (`kujo check`) | PASS |
| `examples/type_conversion.kujo` | Syntax (`kujo check`) | PASS |
| `examples/type_errors.kujo` | Syntax (`kujo check`) | PASS |
| `examples/type_inference.kujo` | Syntax (`kujo check`) | PASS |
| `examples/type_introspection_demo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/udp_echo.kujo` | Syntax (`kujo check`) | PASS |
| `examples/unary_operators.kujo` | Runtime (`kujo run --interpreter`) | PASS |
| `examples/validator.kujo` | Syntax (`kujo check`) | PASS |
| `examples/vector_math.kujo` | Syntax (`kujo check`) | PASS |
| `examples/vm_func_test.kujo` | Syntax (`kujo check`) | PASS |
| `examples/vm_print_correct.kujo` | Syntax (`kujo check`) | PASS |
| `examples/vm_print_noargs.kujo` | Syntax (`kujo check`) | PASS |
| `examples/vm_print_onearg.kujo` | Syntax (`kujo check`) | PASS |
| `examples/vm_print_test.kujo` | Syntax (`kujo check`) | PASS |
| `examples/vm_print_test2.kujo` | Syntax (`kujo check`) | PASS |
| `examples/vm_test_builtin_functions.kujo` | Syntax (`kujo check`) | PASS |
| `examples/vm_test_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/vm_ultra_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/vm_userprint_test.kujo` | Syntax (`kujo check`) | PASS |
| `examples/while_loops_simple.kujo` | Syntax (`kujo check`) | PASS |
| `examples/yaml_demo.kujo` | Runtime (`kujo run --interpreter`) | PASS |

