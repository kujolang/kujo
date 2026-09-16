use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).expect("system time").as_nanos();
    let path = std::env::temp_dir().join(format!(
        "kujo_pdf_{}_{}_{}",
        std::process::id(),
        nanos,
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).expect("temporary directory");
    path
}

fn run(runtime: &str, source_text: &str, allow_write: bool) -> Output {
    let directory = unique_temp_dir();
    let source = directory.join("render.kujo");
    fs::write(&source, source_text).expect("write fixture");
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(&source).arg("--untrusted");
    if runtime == "interpreter" {
        command.arg("--interpreter");
    }
    if allow_write {
        command.arg("--allow-fs-write");
    }
    let output = command.output().expect("Kujo process");
    let _ = fs::remove_dir_all(directory);
    output
}

#[test]
fn in_process_pdf_has_vm_and_interpreter_parity() {
    let source = r#"
let receipt := pdf_render_html("<h1>Estimate QF-1007</h1><p>Total: $9,850.00</p>", {}, {})
print(receipt["ok"])
print(receipt["pages"] >= 1)
print(receipt["bytes"] > 100)
print(len(receipt["pdf_bytes"]) == receipt["bytes"])
"#;
    for runtime in ["vm", "interpreter"] {
        let output = run(runtime, source, false);
        assert!(
            output.status.success(),
            "{runtime}: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "true\ntrue\ntrue\ntrue\n");
    }
}

#[test]
fn file_render_requires_filesystem_write_capability_in_both_runtimes() {
    for runtime in ["vm", "interpreter"] {
        let directory = unique_temp_dir();
        let destination = directory.join("quote.pdf");
        let source = format!(
            "pdf_render_html_to_file(\"<h1>Estimate</h1>\", {{}}, {{}}, {:?})\n",
            destination.to_string_lossy()
        );
        let output = run(runtime, &source, false);
        assert_eq!(output.status.code(), Some(4));
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(diagnostics.contains("filesystem-write required for pdf_render_html_to_file"));
        assert!(!destination.exists());
        let _ = fs::remove_dir_all(directory);
    }
}
