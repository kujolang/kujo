use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn kujo_binary() -> String {
    env!("CARGO_BIN_EXE_kujo").to_string()
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kujo_{}_{}", prefix, nanos));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn run_kujo(args: &[&str], current_dir: &Path, envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(kujo_binary());
    command
        .current_dir(current_dir)
        .args(args)
        .env("NO_COLOR", "1")
        .env_remove("KUJO_AI_RECORD")
        .env_remove("KUJO_AI_REPLAY")
        .env_remove("KUJO_AI_REPLAY_MODE")
        .env_remove("KUJO_AI_ALLOWED_ENDPOINTS");

    for (key, value) in envs {
        command.env(key, value);
    }

    command.output().expect("failed to execute kujo binary")
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8")
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf-8")
}

#[test]
fn strict_ai_replay_miss_is_hermetic_and_does_not_fall_through_to_socket() {
    let project_root = unique_temp_dir("ai_replay_miss_hermetic");
    let script_path = project_root.join("replay_miss.kujo");
    fs::write(
        &script_path,
        r#"
let result := ai_chat("enterprise cassette miss", {
    "endpoint": "http://127.0.0.1:1/v1/chat/completions",
    "model": "gpt-replay",
    "structured_errors": true
})
print(to_string(result))
"#,
    )
    .expect("failed to write replay miss script");

    let replay_dir = repo_root().join("tests/fixtures/ai_cassettes");
    let output = run_kujo(
        &["run", script_path.to_str().expect("script path should be utf-8")],
        &project_root,
        &[
            ("KUJO_AI_REPLAY", replay_dir.to_str().expect("fixture path should be utf-8")),
            ("KUJO_AI_REPLAY_MODE", "strict"),
        ],
    );

    assert!(
        output.status.success(),
        "strict replay miss should return Result::Err, not fail the process: status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );

    let combined = format!("{}{}", stdout_text(&output), stderr_text(&output));
    assert!(combined.contains("kind:\"replay_miss\""), "expected replay_miss, got {combined}");
    assert!(
        combined.contains("strict replay does not use the network"),
        "expected strict replay no-network message, got {combined}"
    );
    assert!(
        !combined.contains("Connection refused") && !combined.contains("error sending request"),
        "strict replay should not fall through to a loopback socket: {combined}"
    );
}

#[test]
fn committed_ai_cassettes_do_not_contain_common_secret_markers() {
    let cassette_dir = repo_root().join("tests/fixtures/ai_cassettes");
    let mut scanned = 0usize;

    for entry in fs::read_dir(&cassette_dir).expect("failed to read AI cassette directory") {
        let entry = entry.expect("failed to read AI cassette entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        scanned += 1;
        let content = fs::read_to_string(&path).expect("failed to read AI cassette");
        for marker in [
            "Authorization",
            "Bearer ",
            "secret-token",
            "response-secret",
            "sk-",
            "api_key",
            "x-api-key",
        ] {
            assert!(
                !content.contains(marker),
                "cassette {} should not contain secret marker {marker:?}",
                path.display()
            );
        }
    }

    assert!(scanned > 0, "expected committed AI cassette fixtures");
}
