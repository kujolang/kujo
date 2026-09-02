use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const FILE_STREAM_CHUNK_BYTES: usize = 64 * 1024;
const FILE_STREAM_MAX_BYTES: u64 = 64 * 1024 * 1024;
const FILE_STREAM_MAX_PATH_BYTES: usize = 4096;

pub(super) fn write_file_range<W: Write>(
    writer: &mut W,
    path: &str,
    offset: i64,
    count: i64,
    operation: &str,
) -> Result<i64, String> {
    if path.is_empty() || path.len() > FILE_STREAM_MAX_PATH_BYTES {
        return Err(format!("{} path must be 1-4096 bytes", operation));
    }
    if offset < 0 || count < 0 {
        return Err(format!("{} offset and count must be non-negative", operation));
    }
    let offset = offset as u64;
    let count = count as u64;
    if count > FILE_STREAM_MAX_BYTES {
        return Err(format!("{} count exceeds the 64 MiB limit", operation));
    }
    let end =
        offset.checked_add(count).ok_or_else(|| format!("{} file range overflows", operation))?;
    let path = Path::new(path);
    let link_metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("{} cannot inspect file: {}", operation, error))?;
    if link_metadata.file_type().is_symlink() || !link_metadata.is_file() {
        return Err(format!("{} requires a non-symlink regular file", operation));
    }

    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("{} cannot open file safely: {}", operation, error))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("{} cannot inspect opened file: {}", operation, error))?;
    if !metadata.is_file() || end > metadata.len() {
        return Err(format!("{} range exceeds the regular file", operation));
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| format!("{} cannot seek file: {}", operation, error))?;

    let mut buffer = vec![0_u8; FILE_STREAM_CHUNK_BYTES];
    let mut remaining = count;
    let mut sent = 0_u64;
    while remaining > 0 {
        let requested = remaining.min(buffer.len() as u64) as usize;
        file.read_exact(&mut buffer[..requested]).map_err(|error| {
            format!("{} file read failed after {} bytes: {}", operation, sent, error)
        })?;
        let mut written = 0_usize;
        while written < requested {
            match writer.write(&buffer[written..requested]) {
                Ok(0) => {
                    return Err(format!(
                        "{} write made no progress after {} bytes",
                        operation,
                        sent + written as u64
                    ));
                }
                Ok(count) => written += count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    return Err(format!(
                        "{} write failed after {} bytes: {}",
                        operation,
                        sent + written as u64,
                        error
                    ));
                }
            }
        }
        sent += written as u64;
        remaining -= requested as u64;
    }
    writer
        .flush()
        .map_err(|error| format!("{} flush failed after {} bytes: {}", operation, sent, error))?;
    Ok(sent as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailAfterTwo {
        written: usize,
    }

    impl Write for FailAfterTwo {
        fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
            if self.written >= 2 {
                return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "fixture"));
            }
            let count = input.len().min(2 - self.written);
            self.written += count;
            Ok(count)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn exact_range_is_streamed_with_fixed_memory() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("body.bin");
        std::fs::write(&path, b"prefix-body-suffix").unwrap();
        let mut output = Vec::new();
        let sent =
            write_file_range(&mut output, path.to_str().unwrap(), 7, 4, "test_send_file_range")
                .unwrap();
        assert_eq!(sent, 4);
        assert_eq!(output, b"body");
    }

    #[test]
    fn invalid_ranges_and_symlinks_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("body.bin");
        std::fs::write(&path, b"body").unwrap();
        let mut output = Vec::new();
        assert!(write_file_range(
            &mut output,
            path.to_str().unwrap(),
            0,
            64 * 1024 * 1024 + 1,
            "test_send_file_range",
        )
        .unwrap_err()
        .contains("64 MiB"));
        assert!(write_file_range(
            &mut output,
            path.to_str().unwrap(),
            3,
            2,
            "test_send_file_range",
        )
        .unwrap_err()
        .contains("range exceeds"));
        #[cfg(unix)]
        {
            let link = directory.path().join("body-link.bin");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert!(write_file_range(
                &mut output,
                link.to_str().unwrap(),
                0,
                4,
                "test_send_file_range",
            )
            .unwrap_err()
            .contains("non-symlink"));
        }
    }

    #[test]
    fn partial_write_error_reports_exact_progress() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("body.bin");
        std::fs::write(&path, b"body").unwrap();
        let error = write_file_range(
            &mut FailAfterTwo { written: 0 },
            path.to_str().unwrap(),
            0,
            4,
            "test_send_file_range",
        )
        .unwrap_err();
        assert!(error.contains("write failed after 2 bytes"), "{error}");
    }
}
