use crate::docgen::adapters::{adapter_for_extension, adapter_for_language};
use crate::docgen::DocgenError;
use crate::path_security::{canonicalize_root, ensure_path_within_root};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    pub selected_languages: Option<Vec<String>>,
    pub max_file_size_bytes: u64,
    pub max_files: usize,
    pub max_depth: usize,
}

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub language: String,
    pub absolute_path: PathBuf,
    pub relative_path: PathBuf,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub root: PathBuf,
    pub files: Vec<DiscoveredFile>,
    pub detected_languages: Vec<String>,
    pub diagnostics: Vec<DiscoveryDiagnostic>,
    pub skip_counts: DiscoverySkipCounts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryDiagnostic {
    pub code: &'static str,
    pub message: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiscoverySkipCounts {
    pub max_file_size: usize,
    pub max_depth: usize,
    pub max_files: usize,
    pub invalid_encoding: usize,
}

pub fn discover(input: &Path, options: &DiscoveryOptions) -> Result<DiscoveryResult, DocgenError> {
    let input_abs = if input.is_absolute() {
        input.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| DocgenError::new(format!("failed to resolve working directory: {}", e)))?
            .join(input)
    };

    let metadata = fs::metadata(&input_abs).map_err(|e| {
        DocgenError::new(format!("failed to read input path '{}': {}", input.display(), e))
    })?;

    let root_candidate = if metadata.is_dir() {
        input_abs.clone()
    } else {
        input_abs
            .parent()
            .ok_or_else(|| DocgenError::new("file input has no parent directory".to_string()))?
            .to_path_buf()
    };

    let root =
        canonicalize_root(&root_candidate, "docgen project root").map_err(DocgenError::new)?;

    let mut paths = Vec::new();
    let mut diagnostics = Vec::new();
    let mut skip_counts = DiscoverySkipCounts::default();
    if metadata.is_file() {
        paths.push(input_abs);
    } else {
        walk_dir(&root, &root, 0, options, &mut paths, &mut diagnostics, &mut skip_counts)?;
    }

    paths.sort();
    if paths.len() > options.max_files {
        let skipped = paths.len() - options.max_files;
        paths.truncate(options.max_files);
        skip_counts.max_files += skipped;
        diagnostics.push(DiscoveryDiagnostic {
            code: "DOCGEN_DISCOVERY_MAX_FILES",
            message: format!(
                "skipped {} files because max file discovery limit ({}) was reached",
                skipped, options.max_files
            ),
            path: None,
        });
    }

    let selected: Option<BTreeSet<String>> = options
        .selected_languages
        .as_ref()
        .map(|langs| langs.iter().map(|lang| lang.to_ascii_lowercase()).collect());

    let mut files = Vec::new();
    let mut languages = BTreeSet::new();

    for file in paths {
        let canonical = fs::canonicalize(&file).map_err(|e| {
            DocgenError::new(format!("failed to resolve source path '{}': {}", file.display(), e))
        })?;
        ensure_path_within_root(&canonical, &root, "docgen source file")
            .map_err(DocgenError::new)?;

        let extension = canonical.extension().and_then(|value| value.to_str()).unwrap_or_default();

        let Some(adapter) = adapter_for_extension(extension) else {
            continue;
        };

        let language = adapter.language_id().to_string();
        if let Some(selected_languages) = &selected {
            if !selected_languages.contains(&language) {
                continue;
            }
        }

        let file_meta = fs::metadata(&canonical).map_err(|e| {
            DocgenError::new(format!("failed to stat source path '{}': {}", canonical.display(), e))
        })?;
        if file_meta.len() > options.max_file_size_bytes {
            let relative = canonical
                .strip_prefix(&root)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| canonical.clone());
            skip_counts.max_file_size += 1;
            diagnostics.push(DiscoveryDiagnostic {
                code: "DOCGEN_DISCOVERY_MAX_FILE_SIZE",
                message: format!(
                    "skipped file '{}' because size {} bytes exceeds max {} bytes",
                    relative.display(),
                    file_meta.len(),
                    options.max_file_size_bytes
                ),
                path: Some(relative),
            });
            continue;
        }

        let source_bytes = fs::read(&canonical).map_err(|e| {
            DocgenError::new(format!("failed to read source file '{}': {}", canonical.display(), e))
        })?;
        let source = match String::from_utf8(source_bytes) {
            Ok(source) => source,
            Err(_) => {
                let relative = canonical
                    .strip_prefix(&root)
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|_| canonical.clone());
                skip_counts.invalid_encoding += 1;
                diagnostics.push(DiscoveryDiagnostic {
                    code: "DOCGEN_DISCOVERY_INVALID_ENCODING",
                    message: format!(
                        "skipped file '{}' because it is not valid UTF-8 source text",
                        relative.display()
                    ),
                    path: Some(relative),
                });
                continue;
            }
        };

        let relative_path = canonical
            .strip_prefix(&root)
            .map_err(|e| DocgenError::new(format!("failed to normalize source path: {}", e)))?
            .to_path_buf();

        languages.insert(language.clone());
        files.push(DiscoveredFile { language, absolute_path: canonical, relative_path, source });
    }

    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path).then(a.language.cmp(&b.language)));
    diagnostics.sort_by(|a, b| {
        a.code.cmp(b.code).then(a.path.cmp(&b.path)).then(a.message.cmp(&b.message))
    });

    Ok(DiscoveryResult {
        root,
        files,
        detected_languages: languages.into_iter().collect(),
        diagnostics,
        skip_counts,
    })
}

fn walk_dir(
    root: &Path,
    current: &Path,
    depth: usize,
    options: &DiscoveryOptions,
    out: &mut Vec<PathBuf>,
    diagnostics: &mut Vec<DiscoveryDiagnostic>,
    skip_counts: &mut DiscoverySkipCounts,
) -> Result<(), DocgenError> {
    if depth > options.max_depth {
        let relative = current.strip_prefix(root).unwrap_or(current).to_path_buf();
        skip_counts.max_depth += 1;
        diagnostics.push(DiscoveryDiagnostic {
            code: "DOCGEN_DISCOVERY_MAX_DEPTH",
            message: format!(
                "skipped directory '{}' because max discovery depth ({}) was exceeded",
                relative.display(),
                options.max_depth
            ),
            path: Some(relative),
        });
        return Ok(());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(current).map_err(|e| {
        DocgenError::new(format!("failed to read directory '{}': {}", current.display(), e))
    })? {
        let entry = entry
            .map_err(|e| DocgenError::new(format!("failed to access directory entry: {}", e)))?;
        entries.push(entry);
    }

    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let symlink_meta = fs::symlink_metadata(&path).map_err(|e| {
            DocgenError::new(format!("failed to read metadata '{}': {}", path.display(), e))
        })?;

        if symlink_meta.file_type().is_symlink() {
            continue;
        }

        let canonical = fs::canonicalize(&path).map_err(|e| {
            DocgenError::new(format!("failed to resolve path '{}': {}", path.display(), e))
        })?;
        ensure_path_within_root(&canonical, root, "docgen discovery path")
            .map_err(DocgenError::new)?;

        if symlink_meta.is_dir() {
            walk_dir(root, &canonical, depth + 1, options, out, diagnostics, skip_counts)?;
        } else if symlink_meta.is_file() {
            out.push(canonical);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kujo_docgen_discovery_{}_{}", prefix, nanos));
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    #[test]
    fn discover_emits_max_file_size_diagnostic() {
        let root = temp_dir("max_file_size");
        let source = root.join("oversized.kujo");
        fs::write(&source, "a".repeat(32)).expect("failed to write source");

        let result = discover(
            &root,
            &DiscoveryOptions {
                selected_languages: Some(vec!["kujo".to_string()]),
                max_file_size_bytes: 8,
                max_files: 10,
                max_depth: 4,
            },
        )
        .expect("discovery should succeed");

        assert!(result.files.is_empty(), "oversized file should be skipped");
        assert!(
            result.diagnostics.iter().any(|diag| diag.code == "DOCGEN_DISCOVERY_MAX_FILE_SIZE"),
            "expected max-file-size discovery diagnostic"
        );
        assert_eq!(result.skip_counts.max_file_size, 1);
        assert_eq!(result.skip_counts.max_depth, 0);
        assert_eq!(result.skip_counts.max_files, 0);
        assert_eq!(result.skip_counts.invalid_encoding, 0);
    }

    #[test]
    fn discover_emits_max_depth_diagnostic() {
        let root = temp_dir("max_depth");
        let nested = root.join("a").join("b");
        fs::create_dir_all(&nested).expect("failed to create nested dirs");
        fs::write(nested.join("deep.kujo"), "func deep() { return 1 }").expect("failed to write");

        let result = discover(
            &root,
            &DiscoveryOptions {
                selected_languages: Some(vec!["kujo".to_string()]),
                max_file_size_bytes: 1024,
                max_files: 10,
                max_depth: 0,
            },
        )
        .expect("discovery should succeed");

        assert!(
            result.diagnostics.iter().any(|diag| diag.code == "DOCGEN_DISCOVERY_MAX_DEPTH"),
            "expected max-depth discovery diagnostic"
        );
        assert_eq!(result.skip_counts.max_file_size, 0);
        assert_eq!(result.skip_counts.max_depth, 1);
        assert_eq!(result.skip_counts.max_files, 0);
        assert_eq!(result.skip_counts.invalid_encoding, 0);
    }

    #[test]
    fn discover_emits_max_files_diagnostic_and_truncates() {
        let root = temp_dir("max_files");
        fs::write(root.join("one.kujo"), "func one() { return 1 }").expect("failed to write");
        fs::write(root.join("two.kujo"), "func two() { return 2 }").expect("failed to write");

        let result = discover(
            &root,
            &DiscoveryOptions {
                selected_languages: Some(vec!["kujo".to_string()]),
                max_file_size_bytes: 1024,
                max_files: 1,
                max_depth: 4,
            },
        )
        .expect("discovery should succeed");

        assert_eq!(result.files.len(), 1, "discovery should truncate to max_files");
        assert!(
            result.diagnostics.iter().any(|diag| diag.code == "DOCGEN_DISCOVERY_MAX_FILES"),
            "expected max-files discovery diagnostic"
        );
        assert_eq!(result.skip_counts.max_file_size, 0);
        assert_eq!(result.skip_counts.max_depth, 0);
        assert_eq!(result.skip_counts.max_files, 1);
        assert_eq!(result.skip_counts.invalid_encoding, 0);
    }

    #[test]
    fn discover_skip_counts_are_zero_without_skips() {
        let root = temp_dir("no_skips");
        fs::write(root.join("ok.kujo"), "pub func ok_api() { return 1 }").expect("write fixture");

        let result = discover(
            &root,
            &DiscoveryOptions {
                selected_languages: Some(vec!["kujo".to_string()]),
                max_file_size_bytes: 1024,
                max_files: 10,
                max_depth: 4,
            },
        )
        .expect("discovery should succeed");

        assert_eq!(result.skip_counts.max_file_size, 0);
        assert_eq!(result.skip_counts.max_depth, 0);
        assert_eq!(result.skip_counts.max_files, 0);
        assert_eq!(result.skip_counts.invalid_encoding, 0);
    }

    #[test]
    fn discover_diagnostics_have_deterministic_sorted_order() {
        let root = temp_dir("diagnostic_order");
        fs::write(root.join("a_oversized.kujo"), "x".repeat(128)).expect("write oversized");
        fs::write(root.join("b_small.kujo"), "pub func b() { return 1 }").expect("write small");
        fs::write(root.join("c_small.kujo"), "pub func c() { return 2 }").expect("write small");
        fs::create_dir_all(root.join("z_nested").join("deep")).expect("create nested dirs");
        fs::write(root.join("z_nested").join("deep").join("hidden.kujo"), "pub func d() {}")
            .expect("write nested");

        let options = DiscoveryOptions {
            selected_languages: Some(vec!["kujo".to_string()]),
            max_file_size_bytes: 16,
            max_files: 2,
            max_depth: 0,
        };

        let result_a = discover(&root, &options).expect("first discovery should succeed");
        let result_b = discover(&root, &options).expect("second discovery should succeed");

        let codes_a: Vec<&str> = result_a.diagnostics.iter().map(|diag| diag.code).collect();
        let codes_b: Vec<&str> = result_b.diagnostics.iter().map(|diag| diag.code).collect();

        assert_eq!(codes_a, codes_b, "diagnostic ordering should be stable across runs");
        assert_eq!(
            codes_a,
            vec![
                "DOCGEN_DISCOVERY_MAX_DEPTH",
                "DOCGEN_DISCOVERY_MAX_FILES",
                "DOCGEN_DISCOVERY_MAX_FILE_SIZE",
                "DOCGEN_DISCOVERY_MAX_FILE_SIZE"
            ],
            "diagnostic ordering should follow deterministic code/path/message sorting"
        );
    }

    #[test]
    fn discover_skips_non_utf8_source_with_deterministic_diagnostic() {
        let root = temp_dir("invalid_encoding");
        let source = root.join("invalid.kujo");
        fs::write(&source, vec![0xff, 0xfe, 0xfd]).expect("failed to write invalid bytes");

        let result = discover(
            &root,
            &DiscoveryOptions {
                selected_languages: Some(vec!["kujo".to_string()]),
                max_file_size_bytes: 1024,
                max_files: 10,
                max_depth: 4,
            },
        )
        .expect("discovery should succeed");

        assert!(result.files.is_empty(), "non-utf8 source should be skipped");
        assert_eq!(result.skip_counts.invalid_encoding, 1);
        assert!(
            result.diagnostics.iter().any(|diag| {
                diag.code == "DOCGEN_DISCOVERY_INVALID_ENCODING"
                    && diag.path.as_ref().is_some_and(|path| path == Path::new("invalid.kujo"))
            }),
            "expected deterministic invalid-encoding diagnostic with relative path"
        );
    }

    #[test]
    fn discover_handles_mixed_utf8_and_non_utf8_sources() {
        let root = temp_dir("mixed_encoding");
        fs::write(root.join("ok.kujo"), "pub func ok() { return 1 }\n").expect("write utf8 source");
        fs::write(root.join("bad.kujo"), vec![0xff, 0xfe, 0xfd]).expect("write invalid source");

        let result = discover(
            &root,
            &DiscoveryOptions {
                selected_languages: Some(vec!["kujo".to_string()]),
                max_file_size_bytes: 1024,
                max_files: 10,
                max_depth: 4,
            },
        )
        .expect("discovery should succeed");

        assert_eq!(result.files.len(), 1, "utf8 source should still be discovered");
        assert_eq!(result.files[0].relative_path, Path::new("ok.kujo"));
        assert_eq!(result.skip_counts.invalid_encoding, 1);
        assert!(result
            .diagnostics
            .iter()
            .any(|diag| diag.code == "DOCGEN_DISCOVERY_INVALID_ENCODING"));
    }
}

pub fn parse_language_filter(
    language: Option<&str>,
    languages: Option<&str>,
) -> Result<Option<Vec<String>>, DocgenError> {
    if let Some(single) = language {
        return Ok(Some(vec![single.trim().to_ascii_lowercase()]));
    }

    if let Some(csv) = languages {
        let parsed: Vec<String> = csv
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(|part| part.to_ascii_lowercase())
            .collect();

        if parsed.is_empty() {
            return Err(DocgenError::new(
                "--languages was provided but no values were parsed".to_string(),
            ));
        }

        return Ok(Some(parsed));
    }

    Ok(None)
}

pub fn validate_languages(languages: &[String]) -> Result<(), DocgenError> {
    for language in languages {
        if adapter_for_language(language).is_none() {
            return Err(DocgenError::new(format!("unsupported language '{}' requested", language)));
        }
    }
    Ok(())
}
