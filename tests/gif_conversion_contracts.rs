//! Opt-in codec interoperability; deterministic fake-converter bounds run in unit tests.
#[cfg(feature = "runtime-image")]
#[test]
#[ignore = "requires the gif2webp executable in PATH"]
fn real_gif_converter_preserves_pixels_in_both_runtimes() {
    use std::{fs, process::Command};
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("-input.gif");
    let source = temporary.path().join("convert.kujo");
    let expected = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
    expected.save(&input).unwrap();
    for interpreter in [false, true] {
        let output =
            temporary.path().join(if interpreter { "interpreter.webp" } else { "vm.webp" });
        let quote = |path: &std::path::Path| {
            format!(
                "parse_json({})",
                serde_json::to_string(&serde_json::to_string(path.to_str().unwrap()).unwrap())
                    .unwrap()
            )
        };
        fs::write(
            &source,
            format!(
                "print(gif_to_webp({}, {}, 100, 4, true))\n",
                quote(std::path::Path::new("-input.gif")),
                quote(&output)
            ),
        )
        .unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.current_dir(temporary.path());
        command.arg("run").arg(&source).args([
            "--untrusted",
            "--allow-fs-read",
            "--allow-fs-write",
            "--allow-process-exec",
        ]);
        if interpreter {
            command.arg("--interpreter");
        }
        let receipt = command.output().unwrap();
        assert!(receipt.status.success(), "{}", String::from_utf8_lossy(&receipt.stderr));
        assert_eq!(String::from_utf8_lossy(&receipt.stdout).trim(), "true");
        assert_eq!(image::open(output).unwrap().to_rgba8(), expected);
    }
}

#[cfg(unix)]
#[test]
fn public_gif_converter_bounds_failure_output_in_both_runtimes() {
    use std::{fs, os::unix::fs::PermissionsExt, process::Command};
    let temporary = tempfile::tempdir().unwrap();
    let converter = temporary.path().join("gif2webp");
    // Shell builtins only: emit exactly 2 MiB without relying on external codecs
    // or changing the test process's PATH. The fixture terminates on its own.
    fs::write(&converter, "#!/bin/sh\ni=0\nwhile [ \"$i\" -lt 4096 ]; do\n  printf '%0512d' 0 >&2\n  i=$((i + 1))\ndone\nexit 7\n").unwrap();
    fs::set_permissions(&converter, fs::Permissions::from_mode(0o700)).unwrap();
    let source = temporary.path().join("failure.kujo");
    fs::write(temporary.path().join("input.gif"), b"fixture").unwrap();
    fs::write(&source, "gif_to_webp(\"input.gif\", \"output.webp\")\n").unwrap();
    for interpreter in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(&source).current_dir(temporary.path()).env("PATH", temporary.path());
        if interpreter {
            command.arg("--interpreter");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(4));
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("[stderr truncated at 1048576 bytes]"));
        assert_eq!(stderr.matches(&"0".repeat(512)).count(), 2048);
        assert!(stderr.len() < 1048576 + 1024, "diagnostic exceeded capture plus framing");
    }
}
