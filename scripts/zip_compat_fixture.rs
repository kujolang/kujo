//! Generate compatibility inputs with the pre-migration ZIP 7.2.0 library.
//! See tests/fixtures/zip7/README.md for the exact build and fixture contract.
use std::{fs, io::Write};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};
fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));
    fs::create_dir_all(&root).unwrap();
    let payload: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
    for (name, method) in [
        ("stored", CompressionMethod::Stored),
        ("deflate", CompressionMethod::Deflated),
        ("bzip2", CompressionMethod::Bzip2),
        ("zstd", CompressionMethod::Zstd),
        ("xz", CompressionMethod::Xz),
        ("ppmd", CompressionMethod::Ppmd),
    ] {
        let mut writer =
            ZipWriter::new(fs::File::create(root.join(format!("{name}.zip"))).unwrap());
        writer
            .start_file("payload.dat", SimpleFileOptions::default().compression_method(method))
            .unwrap();
        writer.write_all(&payload).unwrap();
        writer.finish().unwrap();
    }
}
