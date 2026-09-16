use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "kujo_secure_token_{}_{}_{}",
        std::process::id(),
        nanos,
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).expect("temporary directory should be created");
    path
}

fn run(runtime: &str, allow_random: bool) -> Output {
    let directory = unique_temp_dir();
    let source = directory.join("token.kujo");
    let program = if allow_random {
        "set_random_seed(99)\nlet first := secure_random_token(32)\nset_random_seed(99)\nlet second := secure_random_token(32)\nprint(is_secret(first))\nprint(len(reveal(first)))\nprint(reveal(first) != reveal(second))\n"
    } else {
        "secure_random_token(32)\n"
    };
    fs::write(&source, program).expect("fixture should be written");

    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(&source);
    if runtime == "interpreter" {
        command.arg("--interpreter");
    }
    command.arg("--untrusted");
    if allow_random {
        command.arg("--allow-random");
    }
    let output = command.output().expect("Kujo process should run");
    let _ = fs::remove_dir_all(directory);
    output
}

#[test]
fn secure_random_token_has_vm_and_interpreter_parity() {
    for runtime in ["vm", "interpreter"] {
        let output = run(runtime, true);
        assert!(
            output.status.success(),
            "{runtime} failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "true\n43\ntrue\n");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("Secret("));
    }
}

#[test]
fn secure_random_token_requires_random_capability_in_both_runtimes() {
    for runtime in ["vm", "interpreter"] {
        let output = run(runtime, false);
        assert_eq!(output.status.code(), Some(4));
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            diagnostics.contains("random required for secure_random_token"),
            "unexpected {runtime} diagnostic: {diagnostics}"
        );
        assert!(!diagnostics.contains("Secret("));
    }
}
