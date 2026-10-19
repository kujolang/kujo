#![cfg(all(feature = "runtime-archive", feature = "runtime-network"))]
use std::{fs, io::Write, process::Command};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

fn archive() -> Vec<u8> {
    let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .start_file(
            "payload.dat",
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .unwrap();
    writer.write_all(&[0, 1, 128, 255]).unwrap();
    writer.finish().unwrap().into_inner()
}

fn run(temp: &std::path::Path, interpreter: bool) -> std::process::Output {
    fs::write(temp.join("extract.kujo"), "unzip(\"input.zip\", \"out\")\n").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.current_dir(temp).args(["run", "extract.kujo"]);
    if interpreter {
        command.arg("--interpreter");
    }
    command.output().unwrap()
}

#[test]
fn corrupt_crc_keeps_existing_payload_and_does_not_mislabel_size_error() {
    for interpreter in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let mut bytes = archive();
        let name_len = u16::from_le_bytes(bytes[26..28].try_into().unwrap()) as usize;
        let extra_len = u16::from_le_bytes(bytes[28..30].try_into().unwrap()) as usize;
        bytes[30 + name_len + extra_len] ^= 1;
        fs::write(temp.path().join("input.zip"), bytes).unwrap();
        fs::create_dir(temp.path().join("out")).unwrap();
        fs::write(temp.path().join("out/payload.dat"), b"preserve").unwrap();
        let output = run(temp.path(), interpreter);
        assert_eq!(output.status.code(), Some(4));
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostic.contains("Invalid checksum"), "{diagnostic}");
        assert!(!diagnostic.contains("decoded size does not match"), "{diagnostic}");
        assert_eq!(fs::read(temp.path().join("out/payload.dat")).unwrap(), b"preserve");
        assert_eq!(fs::read_dir(temp.path().join("out")).unwrap().count(), 1);
    }
}

#[cfg(unix)]
#[test]
fn archive_privileged_modes_are_ignored_and_existing_modes_preserved() {
    use std::os::unix::fs::PermissionsExt;
    for interpreter in [false, true] {
        for existing in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let mut bytes = archive();
            let eocd = bytes.len() - 22;
            let central =
                u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
            bytes[central + 5] = 3; // Unix creator, raw privileged mode (writer APIs mask it).
            bytes[central + 38..central + 42].copy_from_slice(&(0o107777_u32 << 16).to_le_bytes());
            fs::write(temp.path().join("input.zip"), bytes).unwrap();
            let target = temp.path().join("out/payload.dat");
            if existing {
                fs::create_dir(temp.path().join("out")).unwrap();
                fs::write(&target, b"old").unwrap();
                fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
            }
            let output = run(temp.path(), interpreter);
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            assert_eq!(fs::read(&target).unwrap(), [0, 1, 128, 255]);
            let mode = fs::metadata(target).unwrap().permissions().mode();
            assert_eq!(mode & 0o7000, 0);
            if existing {
                assert_eq!(mode & 0o777, 0o640);
            }
        }
    }
}
