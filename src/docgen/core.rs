use crate::docgen::adapters::{
    adapter_for_language, capability_index, kujo::extract_symbols_with_parser_fallback,
    language_ids,
};
use crate::docgen::discovery::{
    discover, parse_language_filter, validate_languages, DiscoveryOptions,
};
use crate::docgen::gaps::{
    build_gaps, detect_broken_doc_links, BrokenLinkKind, LinkValidationOptions,
};
use crate::docgen::model::{
    DocComment, DocDiagnostic, DocDiagnosticSeverity, DocModule, DocProject, DocSymbol,
    DocSymbolKind, DocVisibility,
};
use crate::docgen::render;
use crate::docgen::DocgenError;
use crate::interpreter::Interpreter;
use crate::path_security::{canonicalize_root, ensure_path_within_root};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const DOCGEN_DEFAULT_MAX_FILE_SIZE_BYTES: u64 = 2 * 1024 * 1024;
pub const DOCGEN_DEFAULT_MAX_FILES: usize = 20_000;
pub const DOCGEN_DEFAULT_MAX_DEPTH: usize = 64;
pub const DOCGEN_CACHE_SCHEMA_VERSION: &str = "docgen-cache/v1";
pub const DOCGEN_ADAPTER_CACHE_VERSION: &str = "2026-05-19";

#[derive(Debug, Clone, Copy, Default)]
pub struct DocgenExtractionOptions {
    pub kujo_parser_assisted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocOutputFormat {
    Html,
    Markdown,
    Json,
    All,
}

#[derive(Debug, Clone)]
pub struct DocgenConfig {
    pub input: PathBuf,
    pub out_dir: PathBuf,
    pub format: DocOutputFormat,
    pub include_builtins: bool,
    pub language: Option<String>,
    pub languages: Option<String>,
    pub emit_ai_tasks: bool,
    pub search_index: bool,
    pub source_links: bool,
    pub source_link_template: Option<String>,
    pub fail_on_undocumented: bool,
    pub fail_on_broken_links: bool,
    pub fail_on_warnings: bool,
    pub public_only: bool,
    pub include_private: bool,
    pub max_discovery_file_size_bytes: Option<u64>,
    pub max_discovery_files: Option<usize>,
    pub max_discovery_depth: Option<usize>,
    pub cache_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocgenDashboardSummary {
    pub schema_version: String,
    pub item_count: usize,
    pub project_symbol_count: usize,
    pub builtin_symbol_count: usize,
    pub symbol_kind_counts: BTreeMap<String, usize>,
    pub diagnostics_count: usize,
    pub undocumented_count: usize,
    pub broken_link_count: usize,
    pub warning_count: usize,
    pub adapter_health: BTreeMap<String, DocgenAdapterHealth>,
    pub cache_stats: DocgenCacheStats,
    pub discovery_limits: BTreeMap<String, usize>,
    pub discovery_skip_counts: BTreeMap<String, usize>,
    pub link_validation_skip_counts: BTreeMap<String, usize>,
    pub gate_failures_count: usize,
    pub gate_failed: bool,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DocgenAdapterHealth {
    pub files_scanned: usize,
    pub symbols_extracted: usize,
    pub doc_blocks_attached: usize,
    pub placeholders_emitted: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DocgenCacheStats {
    pub hits: usize,
    pub misses: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocgenRunSummary {
    pub output_dir: PathBuf,
    pub module_doc_path: PathBuf,
    pub builtin_doc_path: Option<PathBuf>,
    pub item_count: usize,
    pub project_symbol_count: usize,
    pub builtin_symbol_count: usize,
    pub symbol_kind_counts: BTreeMap<String, usize>,
    pub project_json_path: PathBuf,
    pub gaps_json_path: PathBuf,
    pub capabilities_json_path: PathBuf,
    pub ai_tasks_path: Option<PathBuf>,
    pub languages: Vec<String>,
    pub diagnostics_count: usize,
    pub undocumented_count: usize,
    pub broken_link_count: usize,
    pub warning_count: usize,
    pub adapter_health: BTreeMap<String, DocgenAdapterHealth>,
    pub cache_stats: DocgenCacheStats,
    pub discovery_limits: BTreeMap<String, usize>,
    pub discovery_skip_counts: BTreeMap<String, usize>,
    pub link_validation_skip_counts: BTreeMap<String, usize>,
    pub gate_failures: Vec<String>,
    pub dashboard_summary: DocgenDashboardSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocgenCliJsonPayload {
    pub command: String,
    pub file: String,
    pub output_dir: String,
    pub module_doc_path: String,
    pub builtin_doc_path: Option<String>,
    pub item_count: usize,
    pub project_symbol_count: usize,
    pub builtin_symbol_count: usize,
    pub symbol_kind_counts: BTreeMap<String, usize>,
    pub languages: Vec<String>,
    pub project_json_path: String,
    pub gaps_json_path: String,
    pub capabilities_json_path: String,
    pub ai_tasks_path: Option<String>,
    pub diagnostics_count: usize,
    pub undocumented_count: usize,
    pub broken_link_count: usize,
    pub warning_count: usize,
    pub adapter_health: BTreeMap<String, DocgenAdapterHealth>,
    pub cache_stats: DocgenCacheStats,
    pub discovery_limits: BTreeMap<String, usize>,
    pub discovery_skip_counts: BTreeMap<String, usize>,
    pub link_validation_skip_counts: BTreeMap<String, usize>,
    pub gate_failures: Vec<String>,
    pub summary: DocgenDashboardSummary,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
struct DocgenCacheEntry {
    schema_version: String,
    adapter_version: String,
    language: String,
    relative_path: String,
    symbols: Vec<DocSymbol>,
}

pub fn build_cli_json_payload(
    input_path: &Path,
    summary: &DocgenRunSummary,
) -> DocgenCliJsonPayload {
    DocgenCliJsonPayload {
        command: "docgen".to_string(),
        file: input_path.display().to_string(),
        output_dir: summary.output_dir.display().to_string(),
        module_doc_path: summary.module_doc_path.display().to_string(),
        builtin_doc_path: summary.builtin_doc_path.as_ref().map(|path| path.display().to_string()),
        item_count: summary.item_count,
        project_symbol_count: summary.project_symbol_count,
        builtin_symbol_count: summary.builtin_symbol_count,
        symbol_kind_counts: summary.symbol_kind_counts.clone(),
        languages: summary.languages.clone(),
        project_json_path: summary.project_json_path.display().to_string(),
        gaps_json_path: summary.gaps_json_path.display().to_string(),
        capabilities_json_path: summary.capabilities_json_path.display().to_string(),
        ai_tasks_path: summary.ai_tasks_path.as_ref().map(|path| path.display().to_string()),
        diagnostics_count: summary.diagnostics_count,
        undocumented_count: summary.undocumented_count,
        broken_link_count: summary.broken_link_count,
        warning_count: summary.warning_count,
        adapter_health: summary.adapter_health.clone(),
        cache_stats: summary.cache_stats.clone(),
        discovery_limits: summary.discovery_limits.clone(),
        discovery_skip_counts: summary.discovery_skip_counts.clone(),
        link_validation_skip_counts: summary.link_validation_skip_counts.clone(),
        gate_failures: summary.gate_failures.clone(),
        summary: summary.dashboard_summary.clone(),
    }
}

pub fn run(config: &DocgenConfig) -> Result<(DocProject, DocgenRunSummary), DocgenError> {
    run_with_link_validation_and_options(
        config,
        LinkValidationOptions::default(),
        DocgenExtractionOptions::default(),
    )
}

#[allow(dead_code)]
pub fn run_with_link_validation(
    config: &DocgenConfig,
    link_validation: LinkValidationOptions,
) -> Result<(DocProject, DocgenRunSummary), DocgenError> {
    run_with_link_validation_and_options(
        config,
        link_validation,
        DocgenExtractionOptions::default(),
    )
}

pub fn run_with_link_validation_and_options(
    config: &DocgenConfig,
    link_validation: LinkValidationOptions,
    extraction_options: DocgenExtractionOptions,
) -> Result<(DocProject, DocgenRunSummary), DocgenError> {
    let max_file_size_bytes = resolve_discovery_u64_limit(
        config.max_discovery_file_size_bytes,
        "KUJO_DOCGEN_MAX_FILE_SIZE_BYTES",
        DOCGEN_DEFAULT_MAX_FILE_SIZE_BYTES,
        "max discovery file size bytes",
    )?;
    let max_files = resolve_discovery_usize_limit(
        config.max_discovery_files,
        "KUJO_DOCGEN_MAX_FILES",
        DOCGEN_DEFAULT_MAX_FILES,
        "max discovery files",
    )?;
    let max_depth = resolve_discovery_usize_limit(
        config.max_discovery_depth,
        "KUJO_DOCGEN_MAX_DEPTH",
        DOCGEN_DEFAULT_MAX_DEPTH,
        "max discovery depth",
    )?;

    let selected_languages =
        parse_language_filter(config.language.as_deref(), config.languages.as_deref())?;
    if let Some(ref languages) = selected_languages {
        validate_languages(languages)?;
    }

    let discovery = discover(
        &config.input,
        &DiscoveryOptions { selected_languages, max_file_size_bytes, max_files, max_depth },
    )?;

    let mut modules = Vec::new();
    let mut symbols = Vec::new();
    let mut diagnostics: Vec<DocDiagnostic> = discovery
        .diagnostics
        .iter()
        .map(|diag| DocDiagnostic {
            severity: DocDiagnosticSeverity::Warning,
            code: diag.code.to_string(),
            message: diag.message.clone(),
            path: diag.path.clone(),
            line: None,
        })
        .collect();
    let mut source_map: BTreeMap<String, String> = BTreeMap::new();
    let mut adapter_health: BTreeMap<String, DocgenAdapterHealth> = BTreeMap::new();
    let mut cache_stats = DocgenCacheStats::default();

    for file in &discovery.files {
        adapter_health.entry(file.language.clone()).or_default().files_scanned += 1;
        let Some(adapter) = adapter_for_language(&file.language) else {
            diagnostics.push(DocDiagnostic {
                severity: DocDiagnosticSeverity::Warning,
                code: "DOCGEN001".to_string(),
                message: format!("no adapter found for language '{}'", file.language),
                path: Some(file.relative_path.clone()),
                line: None,
            });
            continue;
        };

        source_map.insert(file.relative_path.display().to_string(), file.source.clone());

        let kujo_extraction_mode = if file.language.eq_ignore_ascii_case("kujo")
            && extraction_options.kujo_parser_assisted
        {
            "kujo-parser-assisted"
        } else {
            "regex-only"
        };
        let adapter_version =
            format!("{}:{}:{}", file.language, DOCGEN_ADAPTER_CACHE_VERSION, kujo_extraction_mode);
        let attached = if let Some(cache_dir) = config.cache_dir.as_ref() {
            let cache_key = compute_docgen_cache_key(
                &file.language,
                &adapter_version,
                &file.relative_path,
                &file.source,
            );
            if let Some(mut cached_symbols) = load_docgen_cached_symbols(
                cache_dir,
                &cache_key,
                &file.language,
                &adapter_version,
                &file.relative_path,
            )? {
                cache_stats.hits += 1;
                for symbol in &mut cached_symbols {
                    symbol.source_path = file.relative_path.clone();
                }
                cached_symbols
            } else {
                cache_stats.misses += 1;
                let raw_symbols = if file.language.eq_ignore_ascii_case("kujo")
                    && extraction_options.kujo_parser_assisted
                {
                    extract_symbols_with_parser_fallback(&file.source, &file.absolute_path)?.symbols
                } else {
                    adapter.extract_symbols(&file.source, &file.absolute_path)?
                };
                let docs = adapter.extract_inline_docs(&file.source, &file.absolute_path)?;
                let mut computed = adapter.attach_docs(raw_symbols, docs);
                for symbol in &mut computed {
                    symbol.source_path = file.relative_path.clone();
                }
                store_docgen_cached_symbols(
                    cache_dir,
                    &cache_key,
                    &file.language,
                    &adapter_version,
                    &file.relative_path,
                    &computed,
                )?;
                computed
            }
        } else {
            let raw_symbols = if file.language.eq_ignore_ascii_case("kujo")
                && extraction_options.kujo_parser_assisted
            {
                extract_symbols_with_parser_fallback(&file.source, &file.absolute_path)?.symbols
            } else {
                adapter.extract_symbols(&file.source, &file.absolute_path)?
            };
            let docs = adapter.extract_inline_docs(&file.source, &file.absolute_path)?;
            let mut computed = adapter.attach_docs(raw_symbols, docs);
            for symbol in &mut computed {
                symbol.source_path = file.relative_path.clone();
            }
            computed
        };
        if let Some(entry) = adapter_health.get_mut(&file.language) {
            entry.symbols_extracted += attached.len();
            entry.doc_blocks_attached +=
                attached.iter().filter(|symbol| !symbol.docs.lines.is_empty()).count();
        }

        modules.push(DocModule {
            name: file
                .relative_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module")
                .to_string(),
            language: file.language.clone(),
            path: file.relative_path.clone(),
            symbols: attached.iter().map(|symbol| symbol.id.clone()).collect(),
        });

        symbols.extend(attached);
    }

    if config.include_builtins {
        add_kujo_builtins(&mut symbols);
    }

    if config.public_only && !config.include_private {
        symbols.retain(|symbol| symbol.visibility == DocVisibility::Public);
    }

    symbols.sort_by(|a, b| {
        a.language
            .cmp(&b.language)
            .then(a.source_path.cmp(&b.source_path))
            .then(a.line.cmp(&b.line))
            .then(a.qualified_name.cmp(&b.qualified_name))
    });

    let mut language_set: BTreeSet<String> = discovery.detected_languages.iter().cloned().collect();
    for symbol in &symbols {
        language_set.insert(symbol.language.clone());
    }

    let mut project = DocProject {
        name: config.input.file_name().and_then(|f| f.to_str()).map(ToOwned::to_owned),
        root: discovery.root.clone(),
        languages: language_set.into_iter().collect(),
        modules,
        symbols,
        gaps: Vec::new(),
        diagnostics,
    };

    build_gaps(&mut project, &source_map);

    for symbol in project.symbols.iter().filter(|symbol| symbol.docs.lines.is_empty()) {
        adapter_health.entry(symbol.language.clone()).or_default().placeholders_emitted += 1;
    }

    for (language, counters) in &adapter_health {
        if counters.files_scanned >= 3 && counters.symbols_extracted == 0 {
            project.diagnostics.push(DocDiagnostic {
                severity: DocDiagnosticSeverity::Warning,
                code: "DOCGEN_ADAPTER_LOW_YIELD".to_string(),
                message: format!(
                    "adapter extraction yield is suspiciously low for language '{}': files_scanned={}, symbols_extracted={}",
                    language, counters.files_scanned, counters.symbols_extracted
                ),
                path: None,
                line: None,
            });
            continue;
        }
        if counters.files_scanned >= 10 && (counters.symbols_extracted * 5) < counters.files_scanned
        {
            project.diagnostics.push(DocDiagnostic {
                severity: DocDiagnosticSeverity::Warning,
                code: "DOCGEN_ADAPTER_LOW_YIELD".to_string(),
                message: format!(
                    "adapter extraction yield is suspiciously low for language '{}': files_scanned={}, symbols_extracted={}",
                    language, counters.files_scanned, counters.symbols_extracted
                ),
                path: None,
                line: None,
            });
        }
    }

    if link_validation.validate_external_links && link_validation.external_link_allowlist.is_empty()
    {
        project.diagnostics.push(DocDiagnostic {
            severity: DocDiagnosticSeverity::Warning,
            code: "DOCGEN_LINK_EXTERNAL_ALLOWLIST_EMPTY".to_string(),
            message: "external link validation is enabled, but no allowlisted hosts were provided; external links were skipped".to_string(),
            path: None,
            line: None,
        });
    }
    if !link_validation.validate_external_links
        && !link_validation.external_link_allowlist.is_empty()
    {
        project.diagnostics.push(DocDiagnostic {
            severity: DocDiagnosticSeverity::Warning,
            code: "DOCGEN_LINK_EXTERNAL_ALLOWLIST_IGNORED".to_string(),
            message: "external link allowlist was provided without --validate-external-links; external links were not validated".to_string(),
            path: None,
            line: None,
        });
    }

    let link_validation_report =
        detect_broken_doc_links(&project.root, &project, link_validation.clone());
    for broken_link in &link_validation_report.broken_links {
        let (code, mode_label) = match broken_link.kind {
            BrokenLinkKind::LocalFileMissing => ("DOCGEN_LINK_BROKEN_LOCAL_FILE", "local-file"),
            BrokenLinkKind::LocalAnchorMissing => {
                ("DOCGEN_LINK_BROKEN_LOCAL_ANCHOR", "local-anchor")
            }
            BrokenLinkKind::ExternalUnreachable => ("DOCGEN_LINK_BROKEN_EXTERNAL", "external"),
            BrokenLinkKind::ExternalRedirectDisallowed => {
                ("DOCGEN_LINK_BROKEN_EXTERNAL_REDIRECT_ALLOWLIST", "external-redirect-allowlist")
            }
            BrokenLinkKind::ExternalPrivateAddressBlocked => {
                ("DOCGEN_LINK_BROKEN_EXTERNAL_PRIVATE_ADDRESS", "external-private-address")
            }
        };
        project.diagnostics.push(DocDiagnostic {
            severity: DocDiagnosticSeverity::Warning,
            code: code.to_string(),
            message: format!(
                "broken {} doc link '{}' in symbol '{}'",
                mode_label, broken_link.target, broken_link.symbol
            ),
            path: None,
            line: Some(broken_link.line),
        });
    }
    if link_validation_report.skip_counts.max_link_checks > 0 {
        project.diagnostics.push(DocDiagnostic {
            severity: DocDiagnosticSeverity::Warning,
            code: "DOCGEN_LINK_VALIDATION_BUDGET_MAX_LINK_CHECKS".to_string(),
            message: format!(
                "link validation skipped {} links after reaching max_link_checks budget",
                link_validation_report.skip_counts.max_link_checks
            ),
            path: None,
            line: None,
        });
    }
    if link_validation_report.skip_counts.max_external_checks > 0 {
        project.diagnostics.push(DocDiagnostic {
            severity: DocDiagnosticSeverity::Warning,
            code: "DOCGEN_LINK_VALIDATION_BUDGET_MAX_EXTERNAL_CHECKS".to_string(),
            message: format!(
                "link validation skipped {} external links after reaching max_external_link_checks budget",
                link_validation_report.skip_counts.max_external_checks
            ),
            path: None,
            line: None,
        });
    }
    if link_validation_report.skip_counts.max_total_time > 0 {
        project.diagnostics.push(DocDiagnostic {
            severity: DocDiagnosticSeverity::Warning,
            code: "DOCGEN_LINK_VALIDATION_BUDGET_TOTAL_TIME".to_string(),
            message: format!(
                "link validation skipped {} links after reaching max_total_validation_time_ms budget",
                link_validation_report.skip_counts.max_total_time
            ),
            path: None,
            line: None,
        });
    }
    project.diagnostics.sort_by(|a, b| {
        diagnostic_severity_rank(&a.severity)
            .cmp(&diagnostic_severity_rank(&b.severity))
            .then(a.code.cmp(&b.code))
            .then(a.path.cmp(&b.path))
            .then(a.line.cmp(&b.line))
            .then(a.message.cmp(&b.message))
    });

    let output_dir = prepare_output_dir(&config.out_dir)?;
    fs::create_dir_all(&output_dir).map_err(|e| {
        DocgenError::new(format!(
            "failed to create output directory '{}': {}",
            output_dir.display(),
            e
        ))
    })?;

    let module_doc_path = output_dir.join("index.html");
    let builtin_doc_path = if config.include_builtins {
        let path = output_dir.join("builtins.html");
        write_file(&path, &render_builtins_html())?;
        Some(path)
    } else {
        None
    };

    write_outputs(config, &project, &output_dir)?;

    let gaps_json_path = output_dir.join("docgen-gaps.json");
    let gaps_json = serde_json::to_string_pretty(&project.gaps)
        .map_err(|e| DocgenError::new(format!("failed to serialize docgen gaps: {}", e)))?;
    write_file(&gaps_json_path, &gaps_json)?;

    let ai_tasks_path = if config.emit_ai_tasks {
        let path = output_dir.join("docgen-ai-tasks.md");
        write_file(&path, &render_ai_tasks(&project))?;
        Some(path)
    } else {
        None
    };

    let capabilities_json_path = output_dir.join("docgen-capabilities.json");
    write_file(&capabilities_json_path, &render_capabilities_json()?)?;

    if config.search_index {
        let search_index_path = output_dir.join("search-index.json");
        write_file(&search_index_path, &render_search_index(&project)?)?;

        let symbol_index_path = output_dir.join("symbol-index.json");
        write_file(&symbol_index_path, &render_symbol_index(&project)?)?;
    }

    let undocumented_count = project
        .gaps
        .iter()
        .filter(|gap| {
            gap.missing_sections
                .iter()
                .any(|entry| matches!(entry, crate::docgen::model::DocGapKind::MissingDocs))
        })
        .count();
    let warning_count = project
        .diagnostics
        .iter()
        .filter(|diag| diag.severity == DocDiagnosticSeverity::Warning)
        .count();

    let mut gate_failures = Vec::new();
    if config.fail_on_undocumented && undocumented_count > 0 {
        gate_failures.push(format!("{} undocumented public symbols detected", undocumented_count));
    }
    if config.fail_on_broken_links && !link_validation_report.broken_links.is_empty() {
        let mut local_file_missing = 0usize;
        let mut local_anchor_missing = 0usize;
        let mut external_unreachable = 0usize;
        let mut external_redirect_allowlist = 0usize;
        let mut external_private_address = 0usize;
        for broken_link in &link_validation_report.broken_links {
            match broken_link.kind {
                BrokenLinkKind::LocalFileMissing => local_file_missing += 1,
                BrokenLinkKind::LocalAnchorMissing => local_anchor_missing += 1,
                BrokenLinkKind::ExternalUnreachable => external_unreachable += 1,
                BrokenLinkKind::ExternalRedirectDisallowed => external_redirect_allowlist += 1,
                BrokenLinkKind::ExternalPrivateAddressBlocked => external_private_address += 1,
            }
        }
        gate_failures.push(format!(
            "{} broken links detected (local_file={}, local_anchor={}, external={}, external_redirect_allowlist={}, external_private_address={})",
            link_validation_report.broken_links.len(),
            local_file_missing,
            local_anchor_missing,
            external_unreachable,
            external_redirect_allowlist,
            external_private_address
        ));
    }
    if config.fail_on_warnings && warning_count > 0 {
        gate_failures.push(format!("{} warnings detected", warning_count));
    }

    let project_json_path = output_dir.join("docgen.json");

    let item_count = project.symbols.len();
    let builtin_symbol_count =
        project.symbols.iter().filter(|symbol| symbol.kind == DocSymbolKind::Builtin).count();
    let project_symbol_count = item_count.saturating_sub(builtin_symbol_count);
    let mut symbol_kind_counts: BTreeMap<String, usize> = BTreeMap::new();
    for symbol in &project.symbols {
        let key = doc_symbol_kind_key(&symbol.kind).to_string();
        *symbol_kind_counts.entry(key).or_insert(0) += 1;
    }
    let languages = project.languages.clone();
    let diagnostics_count = project.diagnostics.len();
    let broken_link_count = link_validation_report.broken_links.len();
    let discovery_limits = BTreeMap::from([
        ("max_file_size_bytes".to_string(), max_file_size_bytes as usize),
        ("max_depth".to_string(), max_depth),
        ("max_files".to_string(), max_files),
    ]);
    let discovery_skip_counts = BTreeMap::from([
        ("max_depth".to_string(), discovery.skip_counts.max_depth),
        ("max_file_size".to_string(), discovery.skip_counts.max_file_size),
        ("max_files".to_string(), discovery.skip_counts.max_files),
        ("invalid_encoding".to_string(), discovery.skip_counts.invalid_encoding),
    ]);
    let link_validation_skip_counts = BTreeMap::from([
        ("max_link_checks".to_string(), link_validation_report.skip_counts.max_link_checks),
        ("max_external_checks".to_string(), link_validation_report.skip_counts.max_external_checks),
        ("max_total_time".to_string(), link_validation_report.skip_counts.max_total_time),
    ]);
    let dashboard_summary = DocgenDashboardSummary {
        schema_version: "docgen-summary/v1".to_string(),
        item_count,
        project_symbol_count,
        builtin_symbol_count,
        symbol_kind_counts: symbol_kind_counts.clone(),
        diagnostics_count,
        undocumented_count,
        broken_link_count,
        warning_count,
        adapter_health: adapter_health.clone(),
        cache_stats: cache_stats.clone(),
        discovery_limits: discovery_limits.clone(),
        discovery_skip_counts: discovery_skip_counts.clone(),
        link_validation_skip_counts: link_validation_skip_counts.clone(),
        gate_failures_count: gate_failures.len(),
        gate_failed: !gate_failures.is_empty(),
        languages: languages.clone(),
    };

    Ok((
        project,
        DocgenRunSummary {
            output_dir,
            module_doc_path,
            builtin_doc_path,
            item_count,
            project_symbol_count,
            builtin_symbol_count,
            symbol_kind_counts,
            project_json_path,
            gaps_json_path,
            capabilities_json_path,
            ai_tasks_path,
            languages,
            diagnostics_count,
            undocumented_count,
            broken_link_count,
            warning_count,
            adapter_health,
            cache_stats,
            discovery_limits,
            discovery_skip_counts,
            link_validation_skip_counts,
            gate_failures,
            dashboard_summary,
        },
    ))
}

fn diagnostic_severity_rank(severity: &DocDiagnosticSeverity) -> u8 {
    match severity {
        DocDiagnosticSeverity::Info => 0,
        DocDiagnosticSeverity::Warning => 1,
        DocDiagnosticSeverity::Error => 2,
    }
}

fn doc_symbol_kind_key(kind: &DocSymbolKind) -> &'static str {
    match kind {
        DocSymbolKind::Module => "module",
        DocSymbolKind::Function => "function",
        DocSymbolKind::Method => "method",
        DocSymbolKind::Class => "class",
        DocSymbolKind::Struct => "struct",
        DocSymbolKind::Enum => "enum",
        DocSymbolKind::EnumVariant => "enum_variant",
        DocSymbolKind::Interface => "interface",
        DocSymbolKind::Trait => "trait",
        DocSymbolKind::TypeAlias => "type_alias",
        DocSymbolKind::Constant => "constant",
        DocSymbolKind::Variable => "variable",
        DocSymbolKind::Property => "property",
        DocSymbolKind::Builtin => "builtin",
        DocSymbolKind::Unknown => "unknown",
    }
}

fn write_outputs(
    config: &DocgenConfig,
    project: &DocProject,
    out_dir: &Path,
) -> Result<(), DocgenError> {
    let html_path = out_dir.join("index.html");
    let markdown_path = out_dir.join("docgen.md");
    let json_path = out_dir.join("docgen.json");

    match config.format {
        DocOutputFormat::Html => {
            write_file(
                &html_path,
                &render::html::render(
                    project,
                    config.source_links,
                    config.source_link_template.as_deref(),
                ),
            )?;
            write_file(&json_path, &render::json::render(project).map_err(DocgenError::new)?)?;
        }
        DocOutputFormat::Markdown => {
            write_file(
                &markdown_path,
                &render::markdown::render(
                    project,
                    config.source_links,
                    config.source_link_template.as_deref(),
                ),
            )?;
            write_file(&json_path, &render::json::render(project).map_err(DocgenError::new)?)?;
            write_file(
                &html_path,
                &render::html::render(
                    project,
                    config.source_links,
                    config.source_link_template.as_deref(),
                ),
            )?;
        }
        DocOutputFormat::Json => {
            write_file(&json_path, &render::json::render(project).map_err(DocgenError::new)?)?;
            write_file(
                &html_path,
                &render::html::render(
                    project,
                    config.source_links,
                    config.source_link_template.as_deref(),
                ),
            )?;
        }
        DocOutputFormat::All => {
            write_file(
                &html_path,
                &render::html::render(
                    project,
                    config.source_links,
                    config.source_link_template.as_deref(),
                ),
            )?;
            write_file(
                &markdown_path,
                &render::markdown::render(
                    project,
                    config.source_links,
                    config.source_link_template.as_deref(),
                ),
            )?;
            write_file(&json_path, &render::json::render(project).map_err(DocgenError::new)?)?;
        }
    }

    Ok(())
}

fn write_file(path: &Path, content: &str) -> Result<(), DocgenError> {
    fs::write(path, content)
        .map_err(|e| DocgenError::new(format!("failed to write '{}': {}", path.display(), e)))
}

fn prepare_output_dir(path: &Path) -> Result<PathBuf, DocgenError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| DocgenError::new(format!("failed to resolve cwd: {}", e)))?
            .join(path)
    };

    let parent = absolute
        .parent()
        .ok_or_else(|| DocgenError::new("output directory has no parent".to_string()))?;
    if !parent.exists() {
        fs::create_dir_all(parent).map_err(|e| {
            DocgenError::new(format!(
                "failed to create output parent '{}': {}",
                parent.display(),
                e
            ))
        })?;
    }

    let canonical_parent =
        canonicalize_root(parent, "docgen output parent").map_err(DocgenError::new)?;
    let candidate = canonical_parent.join(
        absolute
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| DocgenError::new("invalid output directory name".to_string()))?,
    );
    ensure_path_within_root(&candidate, &canonical_parent, "docgen output directory")
        .map_err(DocgenError::new)?;
    Ok(candidate)
}

fn add_kujo_builtins(symbols: &mut Vec<DocSymbol>) {
    let mut names: Vec<&str> = Interpreter::get_builtin_names().to_vec();
    names.sort_unstable();
    for name in names {
        symbols.push(DocSymbol {
            id: format!("kujo:builtin:{}", name),
            language: "kujo".to_string(),
            kind: DocSymbolKind::Builtin,
            name: name.to_string(),
            qualified_name: format!("builtin::{}", name),
            signature: None,
            visibility: DocVisibility::Public,
            source_path: PathBuf::from("<builtins>"),
            line: 0,
            docs: DocComment {
                lines: vec!["Kujo builtin/native API.".to_string()],
                summary: Some("Kujo builtin/native API.".to_string()),
                placeholder: false,
            },
            examples: Vec::new(),
            gaps: Vec::new(),
            parent: Some("builtins".to_string()),
        });
    }
}

fn render_builtins_html() -> String {
    let mut names: Vec<&str> = Interpreter::get_builtin_names().to_vec();
    names.sort_unstable();
    let mut html = String::new();
    html.push_str("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Kujo Builtins</title></head><body>");
    html.push_str("<h1>Kujo Builtin API Reference</h1><ul>");
    for name in names {
        html.push_str(&format!("<li><code>{}</code></li>", name));
    }
    html.push_str("</ul></body></html>");
    html
}

fn render_ai_tasks(project: &DocProject) -> String {
    let mut out = String::new();
    out.push_str("# DocGen AI Tasks\n\n");
    for gap in &project.gaps {
        out.push_str(&format!("## {} ({:?})\n\n", gap.symbol_name, gap.symbol_kind));
        out.push_str(&format!("- Language: {}\n", gap.language));
        out.push_str(&format!("- File: `{}`:{}\n", gap.source_path.display(), gap.line));
        if let Some(signature) = &gap.signature {
            out.push_str(&format!("- Signature: `{}`\n", signature));
        }
        out.push_str(&format!("- Missing sections: {:?}\n\n", gap.missing_sections));
        out.push_str("### Existing docs\n\n");
        if gap.existing_docs.is_empty() {
            out.push_str("_none_\n\n");
        } else {
            for line in &gap.existing_docs {
                out.push_str(&format!("- {}\n", line));
            }
            out.push('\n');
        }
        out.push_str("### Source context\n\n```text\n");
        for line in &gap.bounded_source_context {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("```\n\n");
        out.push_str("### Suggested task prompt\n\n");
        out.push_str(&gap.suggested_ai_prompt);
        out.push_str("\n\n");
    }
    out
}

fn compute_docgen_cache_key(
    language: &str,
    adapter_version: &str,
    relative_path: &Path,
    source: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(DOCGEN_CACHE_SCHEMA_VERSION.as_bytes());
    hasher.update([0u8]);
    hasher.update(language.as_bytes());
    hasher.update([0u8]);
    hasher.update(adapter_version.as_bytes());
    hasher.update([0u8]);
    hasher.update(relative_path.to_string_lossy().as_bytes());
    hasher.update([0u8]);
    hasher.update(source.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{:02x}", byte)).collect::<String>()
}

fn docgen_cache_path(cache_dir: &Path, cache_key: &str) -> PathBuf {
    cache_dir.join(format!("{}.json", cache_key))
}

fn load_docgen_cached_symbols(
    cache_dir: &Path,
    cache_key: &str,
    language: &str,
    adapter_version: &str,
    relative_path: &Path,
) -> Result<Option<Vec<DocSymbol>>, DocgenError> {
    let cache_path = docgen_cache_path(cache_dir, cache_key);
    if !cache_path.exists() {
        return Ok(None);
    }
    let cache_text = fs::read_to_string(&cache_path).map_err(|e| {
        DocgenError::new(format!(
            "failed to read docgen cache entry '{}': {}",
            cache_path.display(),
            e
        ))
    })?;
    let entry: DocgenCacheEntry = match serde_json::from_str(&cache_text) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    if entry.schema_version != DOCGEN_CACHE_SCHEMA_VERSION
        || entry.language != language
        || entry.adapter_version != adapter_version
        || entry.relative_path != relative_path.to_string_lossy()
    {
        return Ok(None);
    }
    Ok(Some(entry.symbols))
}

fn store_docgen_cached_symbols(
    cache_dir: &Path,
    cache_key: &str,
    language: &str,
    adapter_version: &str,
    relative_path: &Path,
    symbols: &[DocSymbol],
) -> Result<(), DocgenError> {
    fs::create_dir_all(cache_dir).map_err(|e| {
        DocgenError::new(format!(
            "failed to create docgen cache directory '{}': {}",
            cache_dir.display(),
            e
        ))
    })?;
    let cache_path = docgen_cache_path(cache_dir, cache_key);
    let entry = DocgenCacheEntry {
        schema_version: DOCGEN_CACHE_SCHEMA_VERSION.to_string(),
        adapter_version: adapter_version.to_string(),
        language: language.to_string(),
        relative_path: relative_path.to_string_lossy().to_string(),
        symbols: symbols.to_vec(),
    };
    let payload = serde_json::to_string(&entry).map_err(|e| {
        DocgenError::new(format!(
            "failed to serialize docgen cache entry '{}': {}",
            cache_path.display(),
            e
        ))
    })?;
    fs::write(&cache_path, payload).map_err(|e| {
        DocgenError::new(format!(
            "failed to write docgen cache entry '{}': {}",
            cache_path.display(),
            e
        ))
    })
}

fn resolve_discovery_u64_limit(
    cli_value: Option<u64>,
    env_key: &str,
    default_value: u64,
    label: &str,
) -> Result<u64, DocgenError> {
    if let Some(value) = cli_value {
        if value == 0 {
            return Err(DocgenError::new(format!("{} must be greater than 0", label)));
        }
        return Ok(value);
    }

    if let Ok(raw_value) = std::env::var(env_key) {
        let parsed = raw_value.parse::<u64>().map_err(|_| {
            DocgenError::new(format!(
                "{} environment value '{}' is not a valid integer",
                env_key, raw_value
            ))
        })?;
        if parsed == 0 {
            return Err(DocgenError::new(format!("{} must be greater than 0", env_key)));
        }
        return Ok(parsed);
    }

    Ok(default_value)
}

fn resolve_discovery_usize_limit(
    cli_value: Option<usize>,
    env_key: &str,
    default_value: usize,
    label: &str,
) -> Result<usize, DocgenError> {
    if let Some(value) = cli_value {
        if value == 0 {
            return Err(DocgenError::new(format!("{} must be greater than 0", label)));
        }
        return Ok(value);
    }

    if let Ok(raw_value) = std::env::var(env_key) {
        let parsed = raw_value.parse::<usize>().map_err(|_| {
            DocgenError::new(format!(
                "{} environment value '{}' is not a valid integer",
                env_key, raw_value
            ))
        })?;
        if parsed == 0 {
            return Err(DocgenError::new(format!("{} must be greater than 0", env_key)));
        }
        return Ok(parsed);
    }

    Ok(default_value)
}

fn render_search_index(project: &DocProject) -> Result<String, DocgenError> {
    #[derive(Serialize)]
    struct SearchEntry {
        language: String,
        name: String,
        kind: String,
        path: String,
        line: usize,
        summary: Option<String>,
    }

    let entries: Vec<SearchEntry> = project
        .symbols
        .iter()
        .map(|symbol| SearchEntry {
            language: symbol.language.clone(),
            name: symbol.qualified_name.clone(),
            kind: format!("{:?}", symbol.kind),
            path: symbol.source_path.display().to_string(),
            line: symbol.line,
            summary: symbol.docs.summary.clone(),
        })
        .collect();

    serde_json::to_string_pretty(&entries)
        .map_err(|e| DocgenError::new(format!("failed to serialize search index: {}", e)))
}

fn render_symbol_index(project: &DocProject) -> Result<String, DocgenError> {
    let mut index: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for symbol in &project.symbols {
        index.entry(symbol.language.clone()).or_default().push(symbol.qualified_name.clone());
    }
    for names in index.values_mut() {
        names.sort();
        names.dedup();
    }

    serde_json::to_string_pretty(&index)
        .map_err(|e| DocgenError::new(format!("failed to serialize symbol index: {}", e)))
}

fn render_capabilities_json() -> Result<String, DocgenError> {
    let payload = capability_index();
    serde_json::to_string_pretty(&payload)
        .map_err(|e| DocgenError::new(format!("failed to serialize adapter capabilities: {}", e)))
}

pub fn parse_output_format(value: Option<&str>) -> Result<DocOutputFormat, DocgenError> {
    match value.unwrap_or("html").to_ascii_lowercase().as_str() {
        "html" => Ok(DocOutputFormat::Html),
        "markdown" | "md" => Ok(DocOutputFormat::Markdown),
        "json" => Ok(DocOutputFormat::Json),
        "all" => Ok(DocOutputFormat::All),
        other => Err(DocgenError::new(format!(
            "unsupported docgen format '{}' (supported: html, markdown, json, all)",
            other
        ))),
    }
}

#[allow(dead_code)]
pub fn supported_languages() -> Vec<&'static str> {
    language_ids()
}
