use std::process::Command;

#[test]
fn rooted_digest_is_registered_in_both_runtimes_and_requires_read_capability() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("file"), b"abc").unwrap();
    let code = format!(
        "print(to_json(digest_file_beneath({}, \"file\", 67108864)))",
        serde_json::to_string(root.path().to_str().unwrap()).unwrap()
    );
    let script = root.path().join("digest.kujo");
    std::fs::write(&script, code).unwrap();
    for interpreter in [false, true] {
        for allowed in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.arg("run").arg(&script).arg("--untrusted");
            if interpreter {
                command.arg("--interpreter");
            }
            if allowed {
                command.arg("--allow-fs-read");
            }
            let result = command.output().unwrap();
            assert_eq!(
                result.status.success(),
                allowed,
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            if allowed {
                let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
                assert_eq!(value["bytes"], 3);
                assert_eq!(
                    value["sha256"],
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                );
            }
        }
    }
}
