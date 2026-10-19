#![cfg(all(feature = "runtime-archive", feature = "runtime-network"))]
use base64::Engine;
use kujo::interpreter::Value;
use std::{
    fs,
    io::Write,
    process::Command,
    sync::{Arc, Mutex},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

#[test]
fn rust_embedder_can_construct_and_finish_the_public_zip_writer() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("embedded.zip");
    let value = Value::ZipArchive {
        writer: Arc::new(Mutex::new(Some(ZipWriter::new(fs::File::create(&path).unwrap())))),
        path: path.to_string_lossy().into_owned(),
    };
    let Value::ZipArchive { writer, .. } = value else { unreachable!() };
    let mut writer = writer.lock().unwrap().take().unwrap();
    writer.start_file("binary.dat", SimpleFileOptions::default()).unwrap();
    writer.write_all(&[0, 1, 128, 255]).unwrap();
    writer.finish().unwrap();
    let mut reader = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut reader.by_index(0).unwrap(), &mut bytes).unwrap();
    assert_eq!(bytes, [0, 1, 128, 255]);
}

#[test]
fn language_zip_names_follow_encoding_flags_in_both_runtimes() {
    for (raw, utf8, expected) in [
        (&b"\xc3\xa9.txt"[..], false, "├⌐.txt"),
        (&b"\xc3\xa9.txt"[..], true, "é.txt"),
        (&b"\x82x.txt"[..], false, "éx.txt"),
        (&b"\x82x.txt"[..], true, "�x.txt"),
    ] {
        for interpreter in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
            let placeholder = "a".repeat(raw.len());
            writer
                .start_file(
                    &placeholder,
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
                )
                .unwrap();
            writer.write_all(b"payload").unwrap();
            let mut bytes = writer.finish().unwrap().into_inner();
            let eocd = bytes.len() - 22;
            let central =
                u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
            bytes[30..30 + raw.len()].copy_from_slice(raw);
            bytes[central + 46..central + 46 + raw.len()].copy_from_slice(raw);
            let flags = if utf8 { 1_u16 << 11 } else { 0 };
            bytes[6..8].copy_from_slice(&flags.to_le_bytes());
            bytes[central + 8..central + 10].copy_from_slice(&flags.to_le_bytes());
            let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
            fs::write(temp.path().join("input.zip"), bytes).unwrap();
            fs::write(temp.path().join("run.kujo"), format!("let entry := zip_single_file_read(decode_base64(\"{encoded}\"), 1024)\nprint(entry[\"name\"])\nunzip(\"input.zip\", \"out\")\n")).unwrap();
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.current_dir(temp.path()).args(["run", "run.kujo"]);
            if interpreter {
                command.arg("--interpreter");
            }
            let output = command.output().unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            assert!(String::from_utf8_lossy(&output.stdout).contains(expected));
            assert_eq!(fs::read(temp.path().join("out").join(expected)).unwrap(), b"payload");
        }
    }
}

#[test]
fn zip7_codec_archives_remain_readable_by_both_language_runtimes() {
    let payload: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
    let expected = base64::engine::general_purpose::STANDARD.encode(&payload);
    for name in ["stored", "deflate", "bzip2", "zstd", "xz", "ppmd"] {
        let archive = fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/zip7/{name}.zip")),
        )
        .unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&archive);
        for interpreter in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            fs::write(temp.path().join("input.zip"), &archive).unwrap();
            fs::write(temp.path().join("run.kujo"), format!("let entry := zip_single_file_read(decode_base64(\"{encoded}\"), 2048)\nassert(entry[\"name\"] == \"payload.dat\", \"archive name\")\nassert(encode_base64(entry[\"bytes\"]) == \"{expected}\", \"archive bytes\")\nunzip(\"input.zip\", \"out\")\n")).unwrap();
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.current_dir(temp.path()).args(["run", "run.kujo"]);
            if interpreter {
                command.arg("--interpreter");
            }
            let output = command.output().unwrap();
            assert!(output.status.success(), "{name}: {}", String::from_utf8_lossy(&output.stderr));
            assert_eq!(fs::read(temp.path().join("out/payload.dat")).unwrap(), payload, "{name}");
        }
    }
}
