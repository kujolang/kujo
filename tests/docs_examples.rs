use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SmokeMode {
    Run,
    ParseOnly,
    ExpectedFail,
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kujo_{prefix}_{nanos}"));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn kujo_binary() -> String {
    env!("CARGO_BIN_EXE_kujo").to_string()
}

fn run_kujo(args: &[&str], current_dir: &Path) -> Output {
    Command::new(kujo_binary())
        .current_dir(current_dir)
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to execute kujo binary")
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8")
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf-8")
}

fn collect_kujo_files_from_fs(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            if metadata.is_dir() {
                walk(&path, files);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("kujo") {
                files.push(path);
            }
        }
    }

    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    files
}

fn collect_tracked_kujo_files(root: &Path, tracked_root: &str) -> Vec<PathBuf> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-files", tracked_root])
        .output()
        .expect("failed to run git ls-files for tracked Kujo examples");
    if output.status.success() {
        let stdout = stdout_text(&output);
        let mut files: Vec<PathBuf> = stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && line.ends_with(".kujo"))
            .map(|line| root.join(line))
            .collect();
        files.sort();
        if !files.is_empty() {
            return files;
        }
    }

    collect_kujo_files_from_fs(&root.join(tracked_root))
}

fn relative_from_repo(path: &Path) -> String {
    let root = repo_root();
    path.strip_prefix(&root)
        .expect("path should be inside repository root")
        .to_string_lossy()
        .replace('\\', "/")
}

fn run_examples() -> HashSet<&'static str> {
    HashSet::from([
        "examples/00-hello.kujo",
        "examples/ai_egress_allowlist.kujo",
        "examples/ai_enterprise_replay_showcase.kujo",
        "examples/ai_multimodal_messages.kujo",
        "examples/ai_stream_callback.kujo",
        "examples/hello.kujo",
        "examples/arrays.kujo",
        "examples/dictionaries.kujo",
        "examples/helper_hlp_007_text_time.kujo",
        "examples/helper_hlp_011_env_config.kujo",
        "examples/helper_hlp_013_process_result.kujo",
        "examples/helper_hlp_015_canonical_json.kujo",
        "examples/math_module.kujo",
        "examples/string_interpolation.kujo",
        "examples/scoping_simple.kujo",
    ])
}

fn expected_fail_examples_with_reason() -> &'static [(&'static str, &'static str)] {
    &[
        ("examples/benchmark_async.kujo", "legacy control-flow syntax drift"),
        (
            "examples/benchmarks/sorting_algorithms.kujo",
            "benchmark fixture kept as negative-coverage debt",
        ),
        (
            "examples/benchmarks/string_processing.kujo",
            "benchmark fixture kept as negative-coverage debt",
        ),
        ("examples/csv_demo.kujo", "legacy stdlib/example syntax drift"),
        ("examples/database_mysql.kujo", "requires unsupported or drifted database demo syntax"),
        (
            "examples/destructuring_demo.kujo",
            "destructuring surface still has parse drift in docs example",
        ),
        ("examples/http_streaming.kujo", "legacy loop syntax drift"),
        ("examples/io_module_demo.kujo", "legacy IO module example drift"),
        (
            "examples/project_api_tester.kujo",
            "named-argument style not supported by current parser",
        ),
        (
            "examples/project_data_pipeline.kujo",
            "pipeline project example has unresolved syntax debt",
        ),
        (
            "examples/project_log_analyzer.kujo",
            "named-argument style not supported by current parser",
        ),
        (
            "examples/project_task_manager.kujo",
            "named-argument style not supported by current parser",
        ),
        (
            "examples/project_web_scraper.kujo",
            "named-argument style not supported by current parser",
        ),
        (
            "examples/projects/contact_manager.kujo",
            "project example has unresolved parse/runtime debt",
        ),
        ("examples/projects/streaming_downloader.kujo", "legacy loop syntax drift"),
        ("examples/spread_operator_demo.kujo", "spread/index syntax drift in legacy example"),
        ("examples/string_functions.kujo", "legacy single-quote argument syntax drift"),
        ("examples/struct_self_methods.kujo", "struct method example has unresolved syntax debt"),
        ("examples/testing_demo.kujo", "legacy test helper syntax drift"),
        ("examples/toml_demo.kujo", "intentional malformed string fixture"),
        ("examples/unary_operators.kujo", "legacy unary syntax drift"),
        ("examples/yaml_demo.kujo", "intentional malformed string fixture"),
    ]
}

fn expected_fail_examples() -> HashSet<&'static str> {
    expected_fail_examples_with_reason().iter().map(|(path, _reason)| *path).collect()
}

fn classify_example(path: &str) -> SmokeMode {
    if run_examples().contains(path) {
        return SmokeMode::Run;
    }
    if expected_fail_examples().contains(path) {
        return SmokeMode::ExpectedFail;
    }
    SmokeMode::ParseOnly
}

fn expected_fail_doc_blocks() -> HashSet<&'static str> {
    HashSet::new()
}

fn classify_doc_block(doc_block_id: &str) -> SmokeMode {
    if expected_fail_doc_blocks().contains(doc_block_id) {
        return SmokeMode::ExpectedFail;
    }
    SmokeMode::ParseOnly
}

fn markdown_files_for_doc_snippets() -> Vec<PathBuf> {
    let root = repo_root();
    let docs_dir = root.join("docs");
    let mut files = vec![root.join("README.md")];

    let entries = fs::read_dir(&docs_dir).expect("failed to read docs directory");
    for entry in entries {
        let entry = entry.expect("failed to read docs directory entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }

    files.sort();
    files
}

fn extract_kujo_blocks(markdown_path: &Path) -> Vec<(usize, String)> {
    let content = fs::read_to_string(markdown_path).expect("failed to read markdown file");
    let mut blocks = Vec::new();
    let mut in_kujo_block = false;
    let mut index = 0usize;
    let mut current = String::new();

    for line in content.lines() {
        if !in_kujo_block {
            if line.trim() == "```kujo" {
                in_kujo_block = true;
                index += 1;
                current.clear();
            }
            continue;
        }

        if line.trim() == "```" {
            in_kujo_block = false;
            blocks.push((index, current.clone()));
            current.clear();
            continue;
        }

        current.push_str(line);
        current.push('\n');
    }

    blocks
}

#[test]
fn examples_smoke_parse_run_or_expected_fail() {
    let root = repo_root();
    let files = collect_tracked_kujo_files(&root, "examples");
    assert!(!files.is_empty(), "expected at least one Kujo example file");

    let mut failures = Vec::new();

    for file in files {
        let rel = relative_from_repo(&file);
        let mode = classify_example(&rel);
        match mode {
            SmokeMode::Run => {
                let output = run_kujo(
                    &["run", "--interpreter", file.to_str().expect("path should be utf-8")],
                    &root,
                );
                if !output.status.success() {
                    failures.push(format!(
                        "RUN {} failed: status={:?} stdout={} stderr={}",
                        rel,
                        output.status.code(),
                        stdout_text(&output),
                        stderr_text(&output)
                    ));
                }
            }
            SmokeMode::ParseOnly => {
                let output = run_kujo(
                    &["check", file.to_str().expect("path should be utf-8"), "--quiet"],
                    &root,
                );
                if !output.status.success() {
                    failures.push(format!(
                        "PARSE {} failed unexpectedly: status={:?} stdout={} stderr={}",
                        rel,
                        output.status.code(),
                        stdout_text(&output),
                        stderr_text(&output)
                    ));
                }
            }
            SmokeMode::ExpectedFail => {
                let output = run_kujo(
                    &["check", file.to_str().expect("path should be utf-8"), "--quiet"],
                    &root,
                );
                if output.status.success() {
                    failures.push(format!(
                        "EXPECTED_FAIL {rel} now passes; reclassify as parse/run example"
                    ));
                }
            }
        }
    }

    assert!(failures.is_empty(), "example smoke mismatches:\n{}", failures.join("\n"));
}

#[test]
fn docs_kujo_snippets_parse_or_expected_fail() {
    let root = repo_root();
    let temp_dir = unique_temp_dir("docs_snippet_smoke");
    let mut failures = Vec::new();

    for markdown_path in markdown_files_for_doc_snippets() {
        let rel = relative_from_repo(&markdown_path);
        let blocks = extract_kujo_blocks(&markdown_path);
        for (index, snippet) in blocks {
            let block_id = format!("{rel}#{index}");
            let mode = classify_doc_block(&block_id);
            let snippet_file =
                temp_dir.join(format!("{}_{}.kujo", rel.replace(['/', '.'], "_"), index));
            fs::write(&snippet_file, snippet).expect("failed to write snippet file");

            let output = run_kujo(
                &["check", snippet_file.to_str().expect("snippet path should be utf-8"), "--quiet"],
                &root,
            );

            match mode {
                SmokeMode::ParseOnly | SmokeMode::Run => {
                    if !output.status.success() {
                        failures.push(format!(
                            "DOC {} failed unexpectedly: status={:?} stdout={} stderr={}",
                            block_id,
                            output.status.code(),
                            stdout_text(&output),
                            stderr_text(&output)
                        ));
                    }
                }
                SmokeMode::ExpectedFail => {
                    if output.status.success() {
                        failures.push(format!(
                            "DOC expected-fail {block_id} now passes; reclassify this snippet"
                        ));
                    }
                }
            }
        }
    }

    assert!(failures.is_empty(), "docs snippet smoke mismatches:\n{}", failures.join("\n"));
}

#[test]
fn expected_fail_examples_have_reasons_and_exist() {
    let root = repo_root();
    let expected_fails = expected_fail_examples_with_reason();
    assert!(!expected_fails.is_empty(), "expected-fail examples list should not be empty");

    let mut seen = HashSet::new();
    for (path, reason) in expected_fails {
        assert!(!reason.trim().is_empty(), "missing reason for {path}");
        assert!(seen.insert(path), "duplicate expected-fail entry: {path}");
        assert!(root.join(path).exists(), "expected-fail example does not exist on disk: {path}");
    }
}

#[test]
fn run_and_expected_fail_example_sets_do_not_overlap() {
    let run_set = run_examples();
    let expected_fail_set = expected_fail_examples();
    for run_example in run_set {
        assert!(
            !expected_fail_set.contains(run_example),
            "example cannot be both run and expected-fail: {run_example}"
        );
    }
}

#[test]
fn expected_fail_doc_blocks_remain_empty() {
    assert!(
        expected_fail_doc_blocks().is_empty(),
        "doc snippet expected-fail set should stay empty; add explicit rationale before introducing new debt"
    );
}
