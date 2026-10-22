use std::process::Command;

pub fn run(source: &str, interpreter: bool) -> std::process::Output {
    run_modules(source, interpreter, &[])
}

pub fn run_modules(
    source: &str,
    interpreter: bool,
    modules: &[(&str, &str)],
) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in modules {
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    let path = dir.path().join("contract.kujo");
    std::fs::write(&path, source).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kujo"));
    cmd.arg("run").arg(path).arg("--untrusted");
    if interpreter {
        cmd.arg("--interpreter");
    }
    cmd.output().unwrap()
}

// Module-level suites can call run_modules directly instead of this helper.
#[allow(dead_code)]
pub fn success(source: &str) {
    for interpreter in [false, true] {
        let out = run(source, interpreter);
        assert!(
            out.status.success(),
            "interpreter={interpreter}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

// This support file is compiled separately by suites that may only test success.
#[allow(dead_code)]
pub fn failure(source: &str, message: &str) {
    for interpreter in [false, true] {
        let out = run(source, interpreter);
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success() && err.contains(message),
            "interpreter={interpreter}: status={:?}, {err}",
            out.status.code()
        );
    }
}
