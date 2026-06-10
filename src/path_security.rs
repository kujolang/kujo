use std::path::{Component, Path, PathBuf};

fn unsafe_path_message(context: &str, raw_path: &str, reason: &str) -> String {
    format!("Unsafe {} '{}': {}", context, raw_path, reason)
}

pub fn is_windows_drive_prefixed(component: &str) -> bool {
    let bytes = component.as_bytes();
    bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
}

pub fn sanitize_relative_path(raw_path: &str, context: &str) -> Result<PathBuf, String> {
    if raw_path.is_empty() {
        return Err(format!("Unsafe {}: empty path is not allowed", context));
    }

    if raw_path.contains('\0') {
        return Err(unsafe_path_message(context, raw_path, "null byte is not allowed"));
    }

    let normalized = raw_path.replace('\\', "/");
    if normalized.starts_with('/') {
        return Err(unsafe_path_message(context, raw_path, "absolute path is not allowed"));
    }

    let mut sanitized = PathBuf::new();

    for component in Path::new(&normalized).components() {
        match component {
            Component::Prefix(_) => {
                return Err(unsafe_path_message(
                    context,
                    raw_path,
                    "drive-prefixed path is not allowed",
                ));
            }
            Component::RootDir => {
                return Err(unsafe_path_message(context, raw_path, "absolute path is not allowed"));
            }
            Component::ParentDir => {
                return Err(unsafe_path_message(
                    context,
                    raw_path,
                    "parent directory traversal component is not allowed",
                ));
            }
            Component::CurDir => {}
            Component::Normal(segment) => {
                let segment_text = segment.to_string_lossy();
                if is_windows_drive_prefixed(&segment_text) {
                    return Err(unsafe_path_message(
                        context,
                        raw_path,
                        "drive-prefixed path is not allowed",
                    ));
                }
                sanitized.push(segment);
            }
        }
    }

    if sanitized.as_os_str().is_empty() {
        return Err(unsafe_path_message(context, raw_path, "empty normalized path is not allowed"));
    }

    Ok(sanitized)
}

pub fn join_within_root(
    root: &Path,
    relative_path: &Path,
    context: &str,
) -> Result<PathBuf, String> {
    if relative_path.is_absolute() {
        return Err(unsafe_path_message(
            context,
            &relative_path.to_string_lossy(),
            "absolute path is not allowed",
        ));
    }

    for component in relative_path.components() {
        match component {
            Component::Prefix(_) => {
                return Err(unsafe_path_message(
                    context,
                    &relative_path.to_string_lossy(),
                    "drive-prefixed path is not allowed",
                ));
            }
            Component::RootDir => {
                return Err(unsafe_path_message(
                    context,
                    &relative_path.to_string_lossy(),
                    "absolute path is not allowed",
                ));
            }
            Component::ParentDir => {
                return Err(unsafe_path_message(
                    context,
                    &relative_path.to_string_lossy(),
                    "parent directory traversal component is not allowed",
                ));
            }
            Component::CurDir => {}
            Component::Normal(segment) => {
                let segment_text = segment.to_string_lossy();
                if is_windows_drive_prefixed(&segment_text) {
                    return Err(unsafe_path_message(
                        context,
                        &relative_path.to_string_lossy(),
                        "drive-prefixed path is not allowed",
                    ));
                }
            }
        }
    }

    let joined_path = root.join(relative_path);
    if !joined_path.starts_with(root) {
        return Err(format!(
            "Unsafe {} '{}': path escapes root directory '{}'",
            context,
            joined_path.display(),
            root.display()
        ));
    }

    Ok(joined_path)
}

pub fn canonicalize_root(root: &Path, context: &str) -> Result<PathBuf, String> {
    std::fs::canonicalize(root)
        .map_err(|error| format!("Failed to resolve {} '{}': {}", context, root.display(), error))
}

pub fn ensure_path_within_root(
    path: &Path,
    canonical_root: &Path,
    context: &str,
) -> Result<(), String> {
    if !path.starts_with(canonical_root) {
        return Err(format!(
            "Unsafe {} '{}': path escapes root directory '{}'",
            context,
            path.display(),
            canonical_root.display()
        ));
    }

    Ok(())
}

pub fn ensure_canonical_path_within_root(
    path: &Path,
    canonical_root: &Path,
    context: &str,
) -> Result<(), String> {
    let canonical_path = std::fs::canonicalize(path).map_err(|error| {
        format!("Failed to resolve {} '{}': {}", context, path.display(), error)
    })?;
    ensure_path_within_root(&canonical_path, canonical_root, context)
}

pub fn reject_symlink_target_path(path: &Path, context: &str) -> Result<(), String> {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "Unsafe {} '{}': symbolic links are not allowed",
                context,
                path.display()
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
pub(crate) fn reject_url_encoded_parent_traversal(
    raw_path: &str,
    context: &str,
) -> Result<(), String> {
    let Some(decoded_path) = decode_percent_encoded(raw_path) else {
        return Ok(());
    };

    let trimmed = decoded_path.trim_start_matches('/');
    if trimmed.is_empty() {
        return Ok(());
    }

    if sanitize_relative_path(trimmed, context).is_err() {
        return Err(unsafe_path_message(context, raw_path, "URL-encoded traversal is not allowed"));
    }

    Ok(())
}

#[cfg(test)]
fn decode_percent_encoded(raw_path: &str) -> Option<String> {
    let bytes = raw_path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if b'%' == bytes[index] {
            if index + 2 >= bytes.len() {
                return None;
            }

            let hi = decode_hex_nibble(bytes[index + 1])?;
            let lo = decode_hex_nibble(bytes[index + 2])?;
            decoded.push((hi << 4) | lo);
            index += 3;
            continue;
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(decoded).ok()
}

#[cfg(test)]
fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_relative_path_rejects_parent_traversal() {
        let err = sanitize_relative_path("../escape.txt", "archive entry")
            .expect_err("expected parent traversal path to be rejected");
        assert!(err.contains("parent directory traversal component"));
    }

    #[test]
    fn sanitize_relative_path_rejects_absolute_paths() {
        let err = sanitize_relative_path("/tmp/escape.txt", "archive entry")
            .expect_err("expected absolute path to be rejected");
        assert!(err.contains("absolute path"));
    }

    #[test]
    fn sanitize_relative_path_rejects_windows_drive_paths() {
        let err = sanitize_relative_path("C:/escape.txt", "archive entry")
            .expect_err("expected drive-prefixed path to be rejected");
        assert!(err.contains("drive-prefixed path"));
    }

    #[test]
    fn sanitize_relative_path_accepts_valid_nested_path() {
        let path = sanitize_relative_path("safe/nested/file.txt", "archive entry")
            .expect("expected nested path to be accepted");
        assert_eq!(PathBuf::from("safe/nested/file.txt"), path);
    }

    #[test]
    fn reject_url_encoded_parent_traversal_rejects_encoded_parent_components() {
        let err = reject_url_encoded_parent_traversal("/%2e%2e/secret.txt", "request path")
            .expect_err("expected encoded traversal to be rejected");
        assert!(err.contains("URL-encoded traversal is not allowed"));
    }

    #[test]
    fn reject_url_encoded_parent_traversal_allows_safe_encoded_paths() {
        reject_url_encoded_parent_traversal("/safe%20name/file.txt", "request path")
            .expect("expected safe encoded path to pass traversal guard");
    }
}
