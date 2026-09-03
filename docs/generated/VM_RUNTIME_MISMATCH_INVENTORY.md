# VM Runtime Mismatch Inventory

Generated: 2026-09-02
Runner: `/Users/robertdevore/2026/Kujolang/kujo-repos/kujo/target/debug/kujo`
Fixture root: `tests`

| Fixture | VM Exit | Interpreter Exit | VM Matches Snapshot | Interpreter Matches Snapshot | Delta Type | Mismatch Bucket | Owner | Priority | Rationale |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- |
| `tests/arg_parser.kujo` | 0 | 4 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/array_methods_test.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/base64_utf8_test.kujo` | 0 | 0 | no | no | `both_mismatch_same_output` | `stale-snapshot-expectation` | docs-owner | `P1` | both runtimes agree on output but snapshot expectation diverges |
| `tests/bytecode_vm.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/cli_module.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/destructuring.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/dict_methods_test.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/enhanced_errors.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/env_and_args.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/error_call_stack_test.kujo` | 4 | 4 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/error_no_stack_test.kujo` | 4 | 4 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/image_processing_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/integer_types.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/jit_inline_cache.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/match_empty_body.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/match_no_param.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/minimal_match_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/net_test.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/private_spool_runtime_test.kujo` | 0 | 0 | no | no | `both_mismatch_same_output` | `stale-snapshot-expectation` | docs-owner | `P1` | both runtimes agree on output but snapshot expectation diverges |
| `tests/range_format_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/result_option.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/simple_error_test.kujo` | 4 | 4 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/simple_image_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/simple_match_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/simple_ok.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/simple_result_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/spread_operator.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/stdlib_io_test.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/stdlib_os_path_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/stdlib_test.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/string_methods_test.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_arithmetic.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_array_contains.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_array_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_assert_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_assertions.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_basic_print.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_binary_files.kujo` | 4 | 4 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_binary_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_call_method.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_chain.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_chain_debug.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_collections.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_comment_edge_cases.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_connection_pooling.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_database_transactions.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
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
| `tests/test_function_drop_fix.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_functions.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_generators.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_higher_order.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_http.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_http_headers.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
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
| `tests/test_operator_overloading.kujo` | 4 | 4 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_operator_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_range_args.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_range_debug.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_range_simple.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_reassign.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_regex.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
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
| `tests/test_trans_debug.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_trans_minimal.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_trans_newvar.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_trans_nostr.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_trans_vars.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_transaction_simple.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_transactions_working.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_try_except.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_unary_current.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_unary_lit.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_unary_mixed.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_unary_ops.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_unary_overload.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_undefined_var.kujo` | 4 | 4 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/test_vec_add.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_verifier.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_vm_optimizations.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/test_void_method.kujo` | 0 | 0 | yes | yes | `both_match_snapshot` | `none` | n/a | `P4` | snapshot matches in both runtimes |
| `tests/testing_framework.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/vm_closure_adder.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/vm_closure_debug.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/vm_closure_detailed.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/vm_closure_multiple.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/vm_closure_order.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/vm_closure_simple.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |
| `tests/vm_native_functions_test.kujo` | 0 | 0 | yes | no | `interpreter_only_mismatch` | `intentional-divergence` | runtime-owner | `P2` | default VM output matches the release snapshot; legacy interpreter-only drift is documented post-v1 compatibility debt |

Summary: `147` fixtures scanned
- both match snapshot: `108`
- VM-only mismatch: `0`
- interpreter-only mismatch: `37`
- both mismatch: `2`

Mismatch classification totals (priority order):
- P0 runtime-parity-bug (`runtime-owner`): `0`
- P1 stale-snapshot-expectation (`docs-owner`): `2`
- P1 parser-invalid-fixture (`language-owner`): `0`
- P2 harness-debt (`harness-owner`): `0`
- P2 intentional-divergence (`runtime-owner`): `37`

VM coverage gate:
- metric: `vm_matches_snapshot / fixtures_scanned`
- vm_matches_snapshot: `145/147` (`98.6%`)
- target threshold: `70.0%`
- gate status: `PASS`
