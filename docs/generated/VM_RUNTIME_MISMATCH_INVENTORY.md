# VM Runtime Mismatch Inventory

Generated: 2026-06-26
Runner: `/Users/robertdevore/2026/Kujolang/kujo-repos/kujo/target/debug/kujo`
Fixture root: `tests`

| Fixture | VM Exit | Interpreter Exit | VM Matches Snapshot | Interpreter Matches Snapshot | Delta Type | Mismatch Bucket | Owner | Priority | Rationale |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- |
| `tests/arg_parser.kujo` | 0 | 4 | yes | no | `interpreter_only_mismatch` | `runtime-parity-bug` | runtime-owner | `P0` | runtime-path mismatch against snapshot indicates parity defect or runtime-specific contract drift |
| `tests/array_methods_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/bytecode_vm.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/destructuring.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/dict_methods_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/enhanced_errors.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/env_and_args.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/error_call_stack_test.kujo` | 4 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/error_no_stack_test.kujo` | 4 | 4 | yes | no | `interpreter_only_mismatch` | `runtime-parity-bug` | runtime-owner | `P0` | runtime-path mismatch against snapshot indicates parity defect or runtime-specific contract drift |
| `tests/fixtures/diagnostics/lexer_invalid_escape.kujo` | 3 | 3 | no | no | `both_mismatch_same_output` | `stale-snapshot-expectation` | docs-owner | `P1` | both runtimes agree on output but snapshot expectation diverges |
| `tests/fixtures/diagnostics/parser_missing_paren.kujo` | 3 | 3 | no | no | `both_mismatch_same_output` | `stale-snapshot-expectation` | docs-owner | `P1` | both runtimes agree on output but snapshot expectation diverges |
| `tests/fixtures/diagnostics/runtime_break_outside_loop.kujo` | 4 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/fixtures/diagnostics/runtime_capability_denied.kujo` | 0 | 4 | yes | no | `interpreter_only_mismatch` | `runtime-parity-bug` | runtime-owner | `P0` | runtime-path mismatch against snapshot indicates parity defect or runtime-specific contract drift |
| `tests/fixtures/diagnostics/runtime_invalid_unary.kujo` | 4 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/fixtures/diagnostics/runtime_missing_module_entry.kujo` | 4 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/fixtures/diagnostics/runtime_non_callable_call.kujo` | 4 | 0 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/fixtures/diagnostics/runtime_undefined_identifier.kujo` | 4 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/fixtures/diagnostics/semantic_invalid_assignment.kujo` | 3 | 3 | no | no | `both_mismatch_same_output` | `stale-snapshot-expectation` | docs-owner | `P1` | both runtimes agree on output but snapshot expectation diverges |
| `tests/fixtures/docgen/conformance_edges.kujo` | 4 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/fixtures/docgen/kujo_async_strict_public.kujo` | 4 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/fixtures/docgen/kujo_async_visibility.kujo` | 4 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/fixtures/docgen/kujo_parser_assisted_fallback.kujo` | 3 | 3 | no | no | `both_mismatch_same_output` | `stale-snapshot-expectation` | docs-owner | `P1` | both runtimes agree on output but snapshot expectation diverges |
| `tests/fixtures/docgen/kujo_parser_assisted_success.kujo` | 0 | 0 | no | yes | `vm_only_mismatch` | `runtime-parity-bug` | runtime-owner | `P0` | runtime-path mismatch against snapshot indicates parity defect or runtime-specific contract drift |
| `tests/fixtures/fuzz/artifacts/parser/crash-synthetic.kujo` | 3 | 3 | no | no | `both_mismatch_same_output` | `stale-snapshot-expectation` | docs-owner | `P1` | both runtimes agree on output but snapshot expectation diverges |
| `tests/fixtures/fuzz/synthetic_crash_input.kujo` | 3 | 3 | no | no | `both_mismatch_same_output` | `stale-snapshot-expectation` | docs-owner | `P1` | both runtimes agree on output but snapshot expectation diverges |
| `tests/generators_test.kujo` | 3 | 3 | no | no | `both_mismatch_same_output` | `stale-snapshot-expectation` | docs-owner | `P1` | both runtimes agree on output but snapshot expectation diverges |
| `tests/image_processing_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/integer_types.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/iterators_test.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `runtime-parity-bug` | runtime-owner | `P0` | runtime-path mismatch against snapshot indicates parity defect or runtime-specific contract drift |
| `tests/jit_direct_recursion.kujo` | 143 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/jit_inline_cache.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/jit_loop_tests.kujo` | 0 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/jit_register_locals.kujo` | 143 | 0 | no | yes | `vm_only_mismatch` | `runtime-parity-bug` | runtime-owner | `P0` | runtime-path mismatch against snapshot indicates parity defect or runtime-specific contract drift |
| `tests/match_empty_body.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/match_no_param.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/minimal_match_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/modules/module_syntax/mymodule.kujo` | 0 | 0 | no | yes | `vm_only_mismatch` | `runtime-parity-bug` | runtime-owner | `P0` | runtime-path mismatch against snapshot indicates parity defect or runtime-specific contract drift |
| `tests/modules/module_syntax/utils.kujo` | 0 | 0 | no | yes | `vm_only_mismatch` | `runtime-parity-bug` | runtime-owner | `P0` | runtime-path mismatch against snapshot indicates parity defect or runtime-specific contract drift |
| `tests/net_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/range_format_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/result_option.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/simple_error_test.kujo` | 4 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/simple_image_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/simple_match_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/simple_ok.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/simple_result_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/spread_operator.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/stdlib_crypto_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/stdlib_io_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/stdlib_os_path_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/stdlib_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/string_methods_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_arithmetic.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_array_contains.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_array_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_assert_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_assertions.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_basic_print.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_binary_files.kujo` | 4 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/test_binary_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_call_method.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_chain.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_chain_debug.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_collections.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_comment_edge_cases.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_connection_pooling.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_database_transactions.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_debug_add.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_display.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_doc_comments.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_dunder.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_enhanced_collections.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_enum_err.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_enum_err_only.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_enum_nested.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_enum_none.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_enum_ok.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_exceptions_comprehensive.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_for_range.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_for_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_func_loop_correct.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_function_drop_fix.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_functions.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_generators.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_higher_order.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_http.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_http_headers.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_http_type_checking.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_json_edge_cases.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_json_parse.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_json_serialize.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_loop_correct.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_method_array.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_method_chaining.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_method_features.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_method_field_ref.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_method_name.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_method_param_minimal.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_method_print.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_method_with_print.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_minimal.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_minimal_hang.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_mixed_comments.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_module_syntax.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_multiline_comments.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_negative.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_no_semi.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_op_add.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_op_add_debug.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_operator_add_working.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_operator_overloading.kujo` | 4 | 4 | no | no | `both_mismatch_different_output` | `runtime-parity-bug` | runtime-owner | `P0` | both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations |
| `tests/test_operator_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_range_args.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_range_debug.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_range_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_reassign.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_regex.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_regex_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_self_backward_compat.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_self_field.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_self_minimal.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_self_param.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_self_return.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_self_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_simple_random.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_standalone_dunder.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_stdlib_datetime.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_stdlib_paths.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_stdlib_random.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_stdlib_system.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_struct_def_only.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_struct_empty_method.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_struct_instantiate.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_struct_method_debug.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_struct_method_print.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_struct_nomethod.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_struct_only.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_struct_parse.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_struct_return.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_struct_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_struct_simple_debug.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_tiny.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_trans_debug.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_trans_minimal.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_trans_newvar.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_trans_nostr.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_trans_vars.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_transaction_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_transactions_working.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_try_except.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_unary_current.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_unary_lit.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_unary_mixed.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_unary_ops.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_unary_overload.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_undefined_var.kujo` | 4 | 4 | yes | no | `interpreter_only_mismatch` | `runtime-parity-bug` | runtime-owner | `P0` | runtime-path mismatch against snapshot indicates parity defect or runtime-specific contract drift |
| `tests/test_vec_add.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_verifier.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_vm_optimizations.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_void_method.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/testing_framework.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `runtime-parity-bug` | runtime-owner | `P0` | runtime-path mismatch against snapshot indicates parity defect or runtime-specific contract drift |
| `tests/vm_closure_adder.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/vm_closure_debug.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/vm_closure_detailed.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/vm_closure_multiple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/vm_closure_order.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/vm_closure_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/vm_native_functions_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |

Summary: `168` fixtures scanned
- both match snapshot: `137`
- VM-only mismatch: `4`
- interpreter-only mismatch: `6`
- both mismatch: `21`

Mismatch classification totals (priority order):
- P0 runtime-parity-bug (`runtime-owner`): `24`
- P1 stale-snapshot-expectation (`docs-owner`): `7`
- P1 parser-invalid-fixture (`language-owner`): `0`
- P2 harness-debt (`harness-owner`): `0`
- P2 intentional-divergence (`runtime-owner`): `0`

VM coverage gate:
- metric: `vm_matches_snapshot / fixtures_scanned`
- vm_matches_snapshot: `143/168` (`85.1%`)
- target threshold: `70.0%`
- gate status: `PASS`
