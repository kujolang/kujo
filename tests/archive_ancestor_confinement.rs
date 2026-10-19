#![cfg(all(unix, feature = "runtime-archive", feature = "runtime-network"))]

use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::symlink;
use std::process::Command;
use zip::{write::SimpleFileOptions, ZipWriter};

fn extract_through_alias(directory_entry: bool, inside: bool, interpreter: bool) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("output");
    let target = if inside { root.join("safe") } else { temp.path().join("outside") };
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("sentinel"), b"unchanged").unwrap();
    symlink(&target, root.join("alias")).unwrap();
    let archive = temp.path().join("input.zip");
    let mut writer = ZipWriter::new(File::create(&archive).unwrap());
    if directory_entry {
        writer.add_directory("alias/new/deep/", SimpleFileOptions::default()).unwrap();
    } else {
        writer.start_file("alias/new/deep/payload.txt", SimpleFileOptions::default()).unwrap();
        writer.write_all(b"payload").unwrap();
    }
    writer.finish().unwrap();
    let script = temp.path().join("extract.kujo");
    fs::write(
        &script,
        format!(
            "print(unzip({}, {}))\n",
            serde_json::to_string(&archive.to_str().unwrap()).unwrap(),
            serde_json::to_string(&root.to_str().unwrap()).unwrap(),
        ),
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run");
    if interpreter {
        command.arg("--interpreter");
    }
    let output = command.arg(&script).output().unwrap();
    assert_eq!(fs::read(target.join("sentinel")).unwrap(), b"unchanged");
    if inside {
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(target.join("new/deep").is_dir());
        if !directory_entry {
            assert_eq!(fs::read(target.join("new/deep/payload.txt")).unwrap(), b"payload");
        }
    } else {
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(diagnostics.contains("escapes output directory"), "{diagnostics}");
        assert!(!target.join("new").exists(), "rejected extraction mutated the outside tree");
        assert_eq!(fs::read_dir(&target).unwrap().count(), 1);
    }
}

#[test]
fn unzip_rejects_outside_ancestors_before_creating_file_parents() {
    for interpreter in [false, true] {
        extract_through_alias(false, false, interpreter);
    }
}

#[test]
fn unzip_rejects_outside_ancestors_before_creating_directory_entries() {
    for interpreter in [false, true] {
        extract_through_alias(true, false, interpreter);
    }
}

#[test]
fn unzip_preserves_inside_ancestor_aliases() {
    for interpreter in [false, true] {
        for directory_entry in [false, true] {
            extract_through_alias(directory_entry, true, interpreter);
        }
    }
}
