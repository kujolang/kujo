// File: src/interpreter/native_functions/async_ops.rs
//
// Asynchronous operations native functions for Kujo.
// Provides async functions for sleep, timeouts, Promise.all, Promise.race, etc.

use crate::interpreter::{AsyncRuntime, DictMap, Value};
use crate::network_policy;
use crate::vm::VM;
use futures::stream::{FuturesUnordered, StreamExt};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::collections::HashMap;
use std::io::{Error, ErrorKind, IoSlice, Read, Write};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

const SSG_HTML_PREFIX_START: &str = "<html><body><h1>Post ";
const SSG_HTML_PREFIX_END: &str = "</h1><article>";
const SSG_HTML_SUFFIX: &str = "</article></body></html>";
const SSG_PROFILE_ASYNC_ENV: &str = "KUJO_BENCH_SSG_PROFILE_ASYNC";
const SSG_SKIP_FLUSH_ENV: &str = "KUJO_SSG_SKIP_FLUSH";

static SSG_RAYON_POOL_CACHE: OnceLock<Mutex<HashMap<usize, Arc<rayon::ThreadPool>>>> =
    OnceLock::new();
static SSG_RENDER_PREFIX_CACHE: OnceLock<Mutex<HashMap<usize, Arc<Vec<Arc<str>>>>>> =
    OnceLock::new();
static SSG_OUTPUT_FILE_SUFFIX_CACHE: OnceLock<Mutex<HashMap<usize, Arc<Vec<Arc<str>>>>>> =
    OnceLock::new();

#[derive(Clone)]
enum RayonMapInput {
    Str(String),
    ArrayLen(usize),
    DictLen(usize),
}

fn ssg_render_prefix_cache() -> &'static Mutex<HashMap<usize, Arc<Vec<Arc<str>>>>> {
    SSG_RENDER_PREFIX_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn ssg_output_file_suffix_cache() -> &'static Mutex<HashMap<usize, Arc<Vec<Arc<str>>>>> {
    SSG_OUTPUT_FILE_SUFFIX_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn ssg_get_or_build_render_prefixes(file_count: usize) -> Arc<Vec<Arc<str>>> {
    {
        let cache =
            ssg_render_prefix_cache().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(prefixes) = cache.get(&file_count) {
            return prefixes.clone();
        }
    }

    let mut prefixes = Vec::with_capacity(file_count);
    for index in 0..file_count {
        let index_text = index.to_string();
        let mut prefix = String::with_capacity(
            SSG_HTML_PREFIX_START.len() + index_text.len() + SSG_HTML_PREFIX_END.len(),
        );
        prefix.push_str(SSG_HTML_PREFIX_START);
        prefix.push_str(index_text.as_str());
        prefix.push_str(SSG_HTML_PREFIX_END);
        prefixes.push(Arc::<str>::from(prefix));
    }
    let new_prefixes = Arc::new(prefixes);

    let mut cache =
        ssg_render_prefix_cache().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(existing) = cache.get(&file_count) {
        return existing.clone();
    }
    cache.insert(file_count, new_prefixes.clone());
    new_prefixes
}

fn ssg_get_or_build_output_file_suffixes(file_count: usize) -> Arc<Vec<Arc<str>>> {
    {
        let cache =
            ssg_output_file_suffix_cache().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(suffixes) = cache.get(&file_count) {
            return suffixes.clone();
        }
    }

    let mut suffixes = Vec::with_capacity(file_count);
    for index in 0..file_count {
        suffixes.push(Arc::<str>::from(format!("post_{}.html", index)));
    }
    let new_suffixes = Arc::new(suffixes);

    let mut cache =
        ssg_output_file_suffix_cache().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(existing) = cache.get(&file_count) {
        return existing.clone();
    }
    cache.insert(file_count, new_suffixes.clone());
    new_suffixes
}

fn ssg_get_cached_output_suffixes_and_prefixes(
    file_count: usize,
) -> (Arc<Vec<Arc<str>>>, Arc<Vec<Arc<str>>>) {
    (
        ssg_get_or_build_output_file_suffixes(file_count),
        ssg_get_or_build_render_prefixes(file_count),
    )
}

fn ssg_build_output_paths_and_prefixes_for_batch(
    output_dir: &str,
    file_count: usize,
) -> (Vec<String>, Arc<Vec<Arc<str>>>) {
    let (output_file_suffixes, render_prefixes) =
        ssg_get_cached_output_suffixes_and_prefixes(file_count);
    let mut paths = Vec::with_capacity(file_count);

    for suffix in output_file_suffixes.iter() {
        let mut output_path = String::with_capacity(output_dir.len() + suffix.len() + 1);
        output_path.push_str(output_dir);
        output_path.push('/');
        output_path.push_str(suffix);
        paths.push(output_path);
    }

    (paths, render_prefixes)
}

#[cfg(test)]
fn ssg_build_output_paths_for_batch(output_dir: &str, file_count: usize) -> Vec<String> {
    ssg_build_output_paths_and_prefixes_for_batch(output_dir, file_count).0
}

#[cfg(test)]
fn ssg_build_render_prefixes_for_batch(file_count: usize) -> Vec<String> {
    ssg_get_or_build_render_prefixes(file_count).iter().map(|p| p.to_string()).collect()
}

#[cfg(test)]
fn ssg_read_ahead_limit(concurrency_limit: usize, file_count: usize) -> usize {
    let bounded_file_count = file_count.max(1);
    let bounded_concurrency = concurrency_limit.max(1);
    let expanded_window = bounded_concurrency.saturating_mul(2);
    expanded_window.min(bounded_file_count)
}

#[cfg(test)]
fn ssg_target_read_in_flight(
    read_ahead_limit: usize,
    write_concurrency_limit: usize,
    pending_writes_len: usize,
    write_in_flight_len: usize,
) -> usize {
    let bounded_read_ahead = read_ahead_limit.max(1);
    let bounded_write_concurrency = write_concurrency_limit.max(1);
    let write_backlog_budget = bounded_write_concurrency.saturating_mul(2);
    let current_write_backlog = pending_writes_len.saturating_add(write_in_flight_len);
    let available_backlog_budget = write_backlog_budget.saturating_sub(current_write_backlog);
    let bounded_available_budget = available_backlog_budget.max(1);

    bounded_read_ahead.min(bounded_available_budget)
}

#[cfg(test)]
fn ssg_should_refill_writes_first(
    pending_writes_len: usize,
    write_in_flight_len: usize,
    write_concurrency_limit: usize,
) -> bool {
    let bounded_write_concurrency = write_concurrency_limit.max(1);
    pending_writes_len > 0 && write_in_flight_len < bounded_write_concurrency
}

#[cfg(test)]
fn ssg_should_prefetch_single_worker_read(
    remaining_reads: usize,
    read_in_flight_len: usize,
    has_pending_write: bool,
) -> bool {
    remaining_reads > 0 && read_in_flight_len == 0 && !has_pending_write
}

fn ssg_escape_html(source: &str) -> String {
    let mut out = String::with_capacity(source.len() + 16);
    for c in source.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

async fn ssg_write_rendered_html_page(
    output_path: &str,
    html_prefix: &str,
    source_body: &str,
) -> std::io::Result<usize> {
    let mut output_file = tokio::fs::File::create(output_path).await?;
    let mut total_written: usize = 0;
    let escaped_source_body = ssg_escape_html(source_body);

    let mut segments = [
        IoSlice::new(html_prefix.as_bytes()),
        IoSlice::new(escaped_source_body.as_bytes()),
        IoSlice::new(SSG_HTML_SUFFIX.as_bytes()),
    ];
    let mut remaining_segments = &mut segments[..];

    while !remaining_segments.is_empty() {
        let written = output_file.write_vectored(remaining_segments).await?;
        if written == 0 {
            return Err(Error::new(ErrorKind::WriteZero, "Failed to write rendered HTML segments"));
        }
        total_written += written;
        IoSlice::advance_slices(&mut remaining_segments, written);
    }

    if !ssg_skip_flush_enabled_from_env() {
        output_file.flush().await?;
    }

    Ok(total_written)
}

#[cfg(test)]
fn ssg_write_rendered_html_page_sync(
    output_path: &str,
    html_prefix: &str,
    source_body: &str,
) -> std::io::Result<usize> {
    ssg_write_rendered_html_page_sync_bytes(
        output_path,
        html_prefix.as_bytes(),
        source_body.as_bytes(),
    )
}

fn ssg_write_rendered_html_page_sync_bytes(
    output_path: &str,
    html_prefix: &[u8],
    source_body: &[u8],
) -> std::io::Result<usize> {
    let mut output_file = std::fs::File::create(output_path)?;
    let mut total_written: usize = 0;
    let escaped_source_body = std::str::from_utf8(source_body).ok().map(ssg_escape_html);

    let mut segments = [
        IoSlice::new(html_prefix),
        IoSlice::new(escaped_source_body.as_deref().map(str::as_bytes).unwrap_or(source_body)),
        IoSlice::new(SSG_HTML_SUFFIX.as_bytes()),
    ];
    let mut remaining_segments = &mut segments[..];

    while !remaining_segments.is_empty() {
        let written = output_file.write_vectored(remaining_segments)?;
        if written == 0 {
            return Err(Error::new(ErrorKind::WriteZero, "Failed to write rendered HTML segments"));
        }
        total_written += written;
        IoSlice::advance_slices(&mut remaining_segments, written);
    }

    if !ssg_skip_flush_enabled_from_env() {
        output_file.flush()?;
    }

    Ok(total_written)
}

fn ssg_read_source_file_bytes(source_path: &str, read_buffer: &mut Vec<u8>) -> std::io::Result<()> {
    read_buffer.clear();
    let mut source_file = std::fs::File::open(source_path)?;
    source_file.read_to_end(read_buffer)?;
    Ok(())
}

/// Runs a Rayon-parallel single-pass SSG pipeline: each Rayon task reads, renders,
/// and writes its own file independently, using a single bounded thread pool.
///
/// Eliminates the two-phase read-barrier from the previous implementation, allowing
/// write I/O to begin as soon as each individual file is read rather than waiting for
/// ALL reads to complete first. Reduces peak memory (only one file's content live per
/// worker at a time) and improves read/write overlap across workers.
///
/// HTML rendering uses synchronous vectored writes over three immutable segments
/// (prefix, source body, suffix), eliminating per-file intermediate rendered buffers
/// and per-file `BufWriter` allocations in the Rayon hot path while preserving exact
/// rendered byte output and write-failure propagation contracts.
///
/// `read_ms` and `render_write_ms` are reported as cumulative CPU-time sums across
/// all Rayon workers (sum of per-task durations), preserving the stage-metric contract
/// while reflecting single-pass execution rather than wall-clock phase boundaries.
///
/// # Returns
/// `Ok((checksum, read_ms, render_write_ms))` on success.
/// `Err(message)` on the first read or write failure, with the same error-message
/// format as the previous Tokio pipeline ("Failed to read file '...' (index ...): ..."
/// / "Failed to write file '...' (index ...): ...").

/// Returns the number of logical CPUs available for parallel work, falling back to 1.
///
/// Used to cap the Rayon thread-pool size so we never over-subscribe the pool beyond
/// the machine's physical parallelism budget (e.g. when `concurrency_limit` is set to a
/// high value like the async task pool default of 256).
#[inline]
fn ssg_rayon_cpu_cap() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

fn ssg_rayon_pool_cache() -> &'static Mutex<HashMap<usize, Arc<rayon::ThreadPool>>> {
    SSG_RAYON_POOL_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn ssg_get_or_create_rayon_pool(rayon_threads: usize) -> Result<Arc<rayon::ThreadPool>, String> {
    {
        let cache = ssg_rayon_pool_cache().lock().map_err(|e| {
            format!("ssg_read_render_and_write_pages() thread-pool cache lock failed: {}", e)
        })?;

        if let Some(existing_pool) = cache.get(&rayon_threads) {
            return Ok(existing_pool.clone());
        }
    }

    let new_pool =
        Arc::new(ThreadPoolBuilder::new().num_threads(rayon_threads).build().map_err(|e| {
            format!("ssg_read_render_and_write_pages() failed to initialize thread pool: {}", e)
        })?);

    let mut cache = ssg_rayon_pool_cache().lock().map_err(|e| {
        format!("ssg_read_render_and_write_pages() thread-pool cache lock failed: {}", e)
    })?;

    if let Some(existing_pool) = cache.get(&rayon_threads) {
        return Ok(existing_pool.clone());
    }

    cache.insert(rayon_threads, new_pool.clone());
    Ok(new_pool)
}

fn ssg_stage_profile_enabled_from_env() -> bool {
    match std::env::var(SSG_PROFILE_ASYNC_ENV) {
        Ok(value) => {
            let normalized = value.trim().to_ascii_lowercase();
            !matches!(normalized.as_str(), "0" | "false" | "off" | "no")
        }
        Err(_) => false,
    }
}

fn ssg_skip_flush_enabled_from_env() -> bool {
    match std::env::var(SSG_SKIP_FLUSH_ENV) {
        Ok(value) => {
            let normalized = value.trim().to_ascii_lowercase();
            !matches!(normalized.as_str(), "0" | "false" | "off" | "no")
        }
        Err(_) => false,
    }
}

#[cfg(test)]
fn ssg_run_rayon_read_render_write(
    source_paths: Vec<String>,
    output_paths: Vec<String>,
    render_prefixes: Arc<Vec<Arc<str>>>,
    concurrency_limit: usize,
    collect_stage_metrics: bool,
) -> Result<(i64, f64, f64), String> {
    if source_paths.len() != output_paths.len() || source_paths.len() != render_prefixes.len() {
        return Err(format!(
            "ssg_read_render_and_write_pages() internal SSG batch shape mismatch (sources={}, outputs={}, prefixes={})",
            source_paths.len(),
            output_paths.len(),
            render_prefixes.len()
        ));
    }

    // Cap the Rayon thread count to the machine's logical CPU count so a high
    // `concurrency_limit` (e.g. 256 from the default async task pool) does not
    // cause thread over-subscription and context-switch thrash on typical hardware.
    let cpu_cap = ssg_rayon_cpu_cap();
    let rayon_threads = concurrency_limit.min(cpu_cap).max(1);
    let pool = ssg_get_or_create_rayon_pool(rayon_threads)?;

    // Single-pass: each Rayon task reads, renders, and writes its own file.
    // No barrier between read and render+write stages — write I/O starts as soon
    // as each individual file is read, without waiting for all reads to complete.
    //
    // Render path uses synchronous vectored writes (prefix + source body + suffix)
    // so each task writes immutable slices directly without allocating an additional
    // per-file output buffer in the Rayon hot path.
    //
    // We iterate over owned zipped vectors so each task receives pre-matched
    // (source, output, prefix) data without repeated indexed lookups.
    //
    // Aggregation uses try_fold/try_reduce directly on the parallel iterator to
    // avoid collecting one intermediate result entry per file.
    let (checksum, total_read_ns, total_rw_ns) = pool.install(|| {
        source_paths
            .into_par_iter()
            .zip(output_paths.into_par_iter())
            .zip(render_prefixes.par_iter())
            .enumerate()
            .map_init(
                || Vec::<u8>::new(),
                |read_buffer, (index, ((source_path, output_path), render_prefix))| {
                    let read_start = collect_stage_metrics.then(Instant::now);
                    ssg_read_source_file_bytes(source_path.as_str(), read_buffer).map_err(|e| {
                        format!("Failed to read file '{}' (index {}): {}", source_path, index, e)
                    })?;
                    let read_ns =
                        read_start.map(|start| start.elapsed().as_nanos() as u64).unwrap_or(0);

                    let rw_start = collect_stage_metrics.then(Instant::now);
                    let written_bytes = ssg_write_rendered_html_page_sync_bytes(
                        &output_path,
                        render_prefix.as_ref().as_bytes(),
                        read_buffer.as_slice(),
                    )
                    .map_err(|e| {
                        format!("Failed to write file '{}' (index {}): {}", output_path, index, e)
                    })?;
                    let rw_ns =
                        rw_start.map(|start| start.elapsed().as_nanos() as u64).unwrap_or(0);

                    Ok::<(i64, u64, u64), String>((written_bytes as i64, read_ns, rw_ns))
                },
            )
            .try_fold(
                || (0i64, 0u64, 0u64),
                |(checksum_acc, read_ns_acc, rw_ns_acc), item| {
                    let (checksum, read_ns, rw_ns) = item?;
                    Ok::<(i64, u64, u64), String>((
                        checksum_acc + checksum,
                        read_ns_acc + read_ns,
                        rw_ns_acc + rw_ns,
                    ))
                },
            )
            .try_reduce(
                || (0i64, 0u64, 0u64),
                |left, right| {
                    Ok::<(i64, u64, u64), String>((
                        left.0 + right.0,
                        left.1 + right.1,
                        left.2 + right.2,
                    ))
                },
            )
    })?;

    let read_ms = total_read_ns as f64 / 1_000_000.0;
    let render_write_ms = total_rw_ns as f64 / 1_000_000.0;
    Ok((checksum, read_ms, render_write_ms))
}

fn ssg_run_rayon_read_render_write_with_reused_output_path_buffer(
    source_paths: Vec<String>,
    output_dir: String,
    output_file_suffixes: Arc<Vec<Arc<str>>>,
    render_prefixes: Arc<Vec<Arc<str>>>,
    concurrency_limit: usize,
    collect_stage_metrics: bool,
) -> Result<(i64, f64, f64), String> {
    if source_paths.len() != output_file_suffixes.len()
        || source_paths.len() != render_prefixes.len()
    {
        return Err(format!(
            "ssg_read_render_and_write_pages() internal SSG batch shape mismatch (sources={}, output_suffixes={}, prefixes={})",
            source_paths.len(),
            output_file_suffixes.len(),
            render_prefixes.len()
        ));
    }

    let cpu_cap = ssg_rayon_cpu_cap();
    let rayon_threads = concurrency_limit.min(cpu_cap).max(1);
    let pool = ssg_get_or_create_rayon_pool(rayon_threads)?;
    let output_dir = Arc::new(output_dir);

    let suffix_max_len = output_file_suffixes.iter().map(|suffix| suffix.len()).max().unwrap_or(0);
    let output_path_capacity = output_dir.len().saturating_add(1).saturating_add(suffix_max_len);

    let (checksum, total_read_ns, total_rw_ns) = pool.install(|| {
        source_paths
            .into_par_iter()
            .zip(output_file_suffixes.par_iter())
            .zip(render_prefixes.par_iter())
            .enumerate()
            .map_init(move || (Vec::<u8>::new(), String::with_capacity(output_path_capacity)), {
                let output_dir = output_dir.clone();
                move |(read_buffer, output_path_buffer): &mut (Vec<u8>, String),
                      (index, ((source_path, output_suffix), render_prefix))| {
                    output_path_buffer.clear();
                    output_path_buffer.push_str(output_dir.as_str());
                    output_path_buffer.push('/');
                    output_path_buffer.push_str(output_suffix.as_ref());

                    let read_start = collect_stage_metrics.then(Instant::now);
                    ssg_read_source_file_bytes(source_path.as_str(), read_buffer).map_err(|e| {
                        format!("Failed to read file '{}' (index {}): {}", source_path, index, e)
                    })?;
                    let read_ns =
                        read_start.map(|start| start.elapsed().as_nanos() as u64).unwrap_or(0);

                    let rw_start = collect_stage_metrics.then(Instant::now);
                    let written_bytes = ssg_write_rendered_html_page_sync_bytes(
                        output_path_buffer.as_str(),
                        render_prefix.as_ref().as_bytes(),
                        read_buffer.as_slice(),
                    )
                    .map_err(|e| {
                        format!(
                            "Failed to write file '{}' (index {}): {}",
                            output_path_buffer, index, e
                        )
                    })?;
                    let rw_ns =
                        rw_start.map(|start| start.elapsed().as_nanos() as u64).unwrap_or(0);

                    Ok::<(i64, u64, u64), String>((written_bytes as i64, read_ns, rw_ns))
                }
            })
            .try_fold(
                || (0i64, 0u64, 0u64),
                |(checksum_acc, read_ns_acc, rw_ns_acc), item| {
                    let (checksum, read_ns, rw_ns) = item?;
                    Ok::<(i64, u64, u64), String>((
                        checksum_acc + checksum,
                        read_ns_acc + read_ns,
                        rw_ns_acc + rw_ns,
                    ))
                },
            )
            .try_reduce(
                || (0i64, 0u64, 0u64),
                |left, right| {
                    Ok::<(i64, u64, u64), String>((
                        left.0 + right.0,
                        left.1 + right.1,
                        left.2 + right.2,
                    ))
                },
            )
    })?;

    let read_ms = total_read_ns as f64 / 1_000_000.0;
    let render_write_ms = total_rw_ns as f64 / 1_000_000.0;
    Ok((checksum, read_ms, render_write_ms))
}

fn resolved_promise(result: Result<Value, String>) -> Value {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let _ = tx.send(result);
    Value::Promise {
        receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
        is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
        cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
        task_handle: None,
    }
}

fn lock_or_async_error<'a, T>(
    mutex: &'a Mutex<T>,
    context: &str,
) -> Result<MutexGuard<'a, T>, String> {
    mutex.lock().map_err(|_| format!("{}: shared async state lock poisoned", context))
}

fn read_cached_promise_result(
    is_polled: &Arc<Mutex<bool>>,
    cached_result: &Arc<Mutex<Option<Result<Value, String>>>>,
) -> Option<Result<Value, String>> {
    let polled = match lock_or_async_error(is_polled.as_ref(), "promise.is_polled") {
        Ok(guard) => guard,
        Err(error) => return Some(Err(error)),
    };
    if !*polled {
        return None;
    }

    let cached = match lock_or_async_error(cached_result.as_ref(), "promise.cached_result") {
        Ok(guard) => guard,
        Err(error) => return Some(Err(error)),
    };
    match cached.as_ref() {
        Some(result) => Some(result.clone()),
        None => Some(Err("Promise polled but no result cached".to_string())),
    }
}

fn cache_promise_result(
    is_polled: &Arc<Mutex<bool>>,
    cached_result: &Arc<Mutex<Option<Result<Value, String>>>>,
    result: Result<Value, String>,
) -> Result<(), String> {
    let mut polled = lock_or_async_error(is_polled.as_ref(), "promise.is_polled")?;
    let mut cached = lock_or_async_error(cached_result.as_ref(), "promise.cached_result")?;
    *cached = Some(result);
    *polled = true;
    Ok(())
}

fn supports_rayon_parallel_map_native(mapper_name: &str) -> bool {
    matches!(mapper_name, "len" | "to_upper" | "upper" | "to_lower" | "lower")
}

fn build_rayon_inputs_for_mapper(
    mapper_name: &str,
    array: &Arc<Vec<Value>>,
) -> Result<Vec<RayonMapInput>, String> {
    let mut inputs = Vec::with_capacity(array.len());

    for value in array.iter() {
        let mapped = match mapper_name {
            "len" => match value {
                Value::Str(s) => RayonMapInput::Str(s.as_ref().clone()),
                Value::Array(arr) => RayonMapInput::ArrayLen(arr.len()),
                Value::Dict(dict) => RayonMapInput::DictLen(dict.len()),
                _ => {
                    return Err(
                        "parallel_map(len, ...) expects string/array/dict elements".to_string()
                    );
                }
            },
            "to_upper" | "upper" | "to_lower" | "lower" => match value {
                Value::Str(s) => RayonMapInput::Str(s.as_ref().clone()),
                _ => {
                    return Err(format!(
                        "parallel_map({}, ...) expects string elements",
                        mapper_name
                    ));
                }
            },
            _ => {
                return Err(format!(
                    "parallel_map() mapper '{}' not supported by rayon fast path",
                    mapper_name
                ));
            }
        };

        inputs.push(mapped);
    }

    Ok(inputs)
}

fn apply_rayon_mapper(mapper_name: &str, input: RayonMapInput) -> Value {
    match mapper_name {
        "len" => match input {
            RayonMapInput::Str(s) => Value::Int(s.chars().count() as i64),
            RayonMapInput::ArrayLen(n) | RayonMapInput::DictLen(n) => Value::Int(n as i64),
        },
        "to_upper" | "upper" => match input {
            RayonMapInput::Str(s) => Value::Str(Arc::new(s.to_uppercase())),
            _ => Value::Error("parallel_map(to_upper, ...) received invalid element".to_string()),
        },
        "to_lower" | "lower" => match input {
            RayonMapInput::Str(s) => Value::Str(Arc::new(s.to_lowercase())),
            _ => Value::Error("parallel_map(to_lower, ...) received invalid element".to_string()),
        },
        _ => Value::Error(format!(
            "parallel_map() mapper '{}' not supported by rayon fast path",
            mapper_name
        )),
    }
}

fn try_parallel_map_with_rayon(
    mapper_name: &str,
    array: &Arc<Vec<Value>>,
    concurrency_limit: usize,
) -> Option<Value> {
    if !supports_rayon_parallel_map_native(mapper_name) {
        return None;
    }

    let inputs = match build_rayon_inputs_for_mapper(mapper_name, array) {
        Ok(values) => values,
        Err(err) => return Some(Value::Error(err)),
    };

    if inputs.is_empty() {
        return Some(resolved_promise(Ok(Value::Array(Arc::new(vec![])))));
    }

    let pool = match ThreadPoolBuilder::new().num_threads(concurrency_limit.max(1)).build() {
        Ok(pool) => pool,
        Err(err) => {
            return Some(Value::Error(format!(
                "parallel_map() failed to initialize rayon pool: {}",
                err
            )));
        }
    };

    let results: Vec<Value> = pool.install(|| {
        inputs.into_par_iter().map(|input| apply_rayon_mapper(mapper_name, input)).collect()
    });

    if let Some(Value::Error(err)) = results.iter().find(|value| matches!(value, Value::Error(_))) {
        return Some(Value::Error(err.clone()));
    }

    Some(resolved_promise(Ok(Value::Array(Arc::new(results)))))
}

fn try_parallel_map_with_jit_bytecode(
    interp: &mut crate::interpreter::Interpreter,
    mapper: &Value,
    array: &Arc<Vec<Value>>,
) -> Option<Value> {
    let is_bytecode_mapper = matches!(mapper, Value::BytecodeFunction { .. });
    if !is_bytecode_mapper {
        return None;
    }

    let mut vm = VM::new();
    vm.set_jit_enabled(true);
    vm.set_globals(Arc::new(Mutex::new(interp.env.clone())));

    if let Err(err) = vm.jit_compile_bytecode_function(mapper) {
        if std::env::var("DEBUG_JIT").is_ok() {
            eprintln!("parallel_map: eager JIT compile for bytecode mapper failed: {}", err);
        }
    }

    let mut mapped_values = Vec::with_capacity(array.len());
    for element in array.iter() {
        match vm.call_function_from_jit(mapper.clone(), vec![element.clone()]) {
            Ok(value) => mapped_values.push(value),
            Err(err) => {
                return Some(Value::Error(format!("parallel_map() mapper call failed: {}", err)))
            }
        }
    }

    Some(resolved_promise(Ok(Value::Array(Arc::new(mapped_values)))))
}

/// Handle async operations native functions
pub fn handle(
    _interp: &mut crate::interpreter::Interpreter,
    name: &str,
    args: &[Value],
) -> Option<Value> {
    match name {
        "async_sleep" => {
            // async_sleep(milliseconds: Int) -> Promise<Null>
            if args.len() != 1 {
                return Some(Value::Error(format!(
                    "async_sleep() expects 1 argument (milliseconds), got {}",
                    args.len()
                )));
            }

            let ms = match &args[0] {
                Value::Int(n) if *n >= 0 => *n as u64,
                Value::Int(n) => {
                    return Some(Value::Error(format!(
                        "async_sleep() requires non-negative milliseconds, got {}",
                        n
                    )));
                }
                _ => {
                    return Some(Value::Error(
                        "async_sleep() requires an integer milliseconds argument".to_string(),
                    ));
                }
            };

            // Create a tokio oneshot channel
            let (tx, rx) = tokio::sync::oneshot::channel();

            // Spawn async task that sleeps then resolves
            AsyncRuntime::spawn_task(async move {
                AsyncRuntime::sleep(Duration::from_millis(ms)).await;
                let _ = tx.send(Ok(Value::Null));
                Value::Null // Task return value (not used)
            });

            // Return promise immediately
            Some(Value::Promise {
                receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                task_handle: None,
            })
        }

        "async_timeout" => {
            // async_timeout(promise: Promise, timeout_ms: Int) -> Promise<Value>
            // Race a promise against a timeout - returns Error if timeout expires
            if args.len() != 2 {
                return Some(Value::Error(format!(
                    "async_timeout() expects 2 arguments (promise, timeout_ms), got {}",
                    args.len()
                )));
            }

            // Extract timeout duration
            let timeout_ms = match &args[1] {
                Value::Int(n) if *n > 0 => *n as u64,
                Value::Int(n) => {
                    return Some(Value::Error(format!(
                        "async_timeout() requires positive timeout_ms, got {}",
                        n
                    )));
                }
                _ => {
                    return Some(Value::Error(
                        "async_timeout() requires an integer timeout_ms argument".to_string(),
                    ));
                }
            };

            // Extract the promise
            match &args[0] {
                Value::Promise { receiver, is_polled, cached_result, .. } => {
                    // Clone the Arc pointers to move into async task
                    let receiver = receiver.clone();
                    let is_polled = is_polled.clone();
                    let cached_result = cached_result.clone();

                    // Create new channel for timeout result
                    let (tx, rx) = tokio::sync::oneshot::channel();

                    // Spawn task that races promise against timeout
                    AsyncRuntime::spawn_task(async move {
                        // Check if promise already polled (has cached result)
                        {
                            let polled = match lock_or_async_error(
                                is_polled.as_ref(),
                                "async_timeout.promise.is_polled",
                            ) {
                                Ok(guard) => guard,
                                Err(error) => {
                                    let _ = tx.send(Err(error));
                                    return Value::Null;
                                }
                            };
                            let cached = match lock_or_async_error(
                                cached_result.as_ref(),
                                "async_timeout.promise.cached_result",
                            ) {
                                Ok(guard) => guard,
                                Err(error) => {
                                    let _ = tx.send(Err(error));
                                    return Value::Null;
                                }
                            };

                            if *polled {
                                // Use cached result
                                let result = match cached.as_ref() {
                                    Some(Ok(val)) => Ok(val.clone()),
                                    Some(Err(err)) => Err(format!("Promise rejected: {}", err)),
                                    None => Err("Promise polled but no result cached".to_string()),
                                };
                                let _ = tx.send(result);
                                return Value::Null;
                            }
                        }

                        // Extract the receiver from the mutex
                        let actual_rx = {
                            let mut recv_guard = match lock_or_async_error(
                                receiver.as_ref(),
                                "async_timeout.promise.receiver",
                            ) {
                                Ok(guard) => guard,
                                Err(error) => {
                                    let _ = tx.send(Err(error));
                                    return Value::Null;
                                }
                            };
                            let (dummy_tx, dummy_rx) = tokio::sync::oneshot::channel();
                            drop(dummy_tx);
                            std::mem::replace(&mut *recv_guard, dummy_rx)
                        };

                        // Race the promise against the timeout
                        let timeout_result =
                            AsyncRuntime::timeout(Duration::from_millis(timeout_ms), actual_rx)
                                .await;

                        // Process result
                        let result = match timeout_result {
                            Ok(Ok(Ok(value))) => {
                                // Promise resolved successfully within timeout
                                Ok(value)
                            }
                            Ok(Ok(Err(err))) => {
                                // Promise rejected (not a timeout)
                                Err(format!("Promise rejected: {}", err))
                            }
                            Ok(Err(_)) => {
                                // Channel closed without sending
                                Err("Promise never resolved".to_string())
                            }
                            Err(_elapsed) => {
                                // Timeout elapsed
                                Err(format!("Timeout after {}ms", timeout_ms))
                            }
                        };

                        let _ = tx.send(result);
                        Value::Null
                    });

                    // Return new promise
                    Some(Value::Promise {
                        receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                        is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                        cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                        task_handle: None,
                    })
                }
                _ => Some(Value::Error(
                    "async_timeout() requires a Promise as first argument".to_string(),
                )),
            }
        }

        "async_http_get" => {
            // async_http_get(url: String) -> Promise<Dict>
            // Non-blocking HTTP GET request
            if args.len() != 1 {
                return Some(Value::Error(format!(
                    "async_http_get() expects 1 argument (url), got {}",
                    args.len()
                )));
            }

            let url = match &args[0] {
                Value::Str(s) => s.clone(),
                _ => {
                    return Some(Value::Error(
                        "async_http_get() requires a string URL argument".to_string(),
                    ));
                }
            };

            if let Err(error) =
                network_policy::enforce_http_url_destination_policy(url.as_ref(), "async_http_get")
            {
                return Some(Value::Error(error));
            }

            // Create channel for result
            let (tx, rx) = tokio::sync::oneshot::channel();

            // Spawn async HTTP request
            AsyncRuntime::spawn_task(async move {
                let result = async {
                    let client = network_policy::build_async_http_client(
                        network_policy::default_http_timeout(),
                    )?;
                    let response = client
                        .get(url.as_ref())
                        .send()
                        .await
                        .map_err(|e| format!("async_http_get failed: {}", e))?;

                    let (status, headers_map, body) =
                        network_policy::read_async_http_response_text(response, "async_http_get")
                            .await?;

                    // Build result dictionary
                    let mut result_dict = DictMap::default();
                    result_dict.insert("status".into(), Value::Int(status as i64));
                    result_dict.insert("body".into(), Value::Str(Arc::new(body)));

                    // Convert headers to dict
                    let mut headers_dict = DictMap::default();
                    for (name, value) in headers_map.iter() {
                        if let Ok(value_str) = value.to_str() {
                            headers_dict.insert(
                                name.to_string().into(),
                                Value::Str(Arc::new(value_str.to_string())),
                            );
                        }
                    }
                    result_dict.insert("headers".into(), Value::Dict(Arc::new(headers_dict)));

                    Ok::<Value, String>(Value::Dict(Arc::new(result_dict)))
                }
                .await;

                let _ = tx.send(result);
                Value::Null
            });

            // Return promise
            Some(Value::Promise {
                receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                task_handle: None,
            })
        }

        "async_http_post" => {
            // async_http_post(url: String, body: String, headers?: Dict) -> Promise<Dict>
            // Non-blocking HTTP POST request
            if args.len() < 2 || args.len() > 3 {
                return Some(Value::Error(format!(
                    "async_http_post() expects 2-3 arguments (url, body, headers?), got {}",
                    args.len()
                )));
            }

            let url = match &args[0] {
                Value::Str(s) => s.clone(),
                _ => {
                    return Some(Value::Error(
                        "async_http_post() requires a string URL argument".to_string(),
                    ));
                }
            };

            let body = match &args[1] {
                Value::Str(s) => s.clone(),
                _ => {
                    return Some(Value::Error(
                        "async_http_post() requires a string body argument".to_string(),
                    ));
                }
            };

            if let Err(error) =
                network_policy::enforce_http_url_destination_policy(url.as_ref(), "async_http_post")
            {
                return Some(Value::Error(error));
            }

            // Optional headers
            let headers = if args.len() == 3 {
                match &args[2] {
                    Value::Dict(dict) => Some(dict.clone()),
                    _ => {
                        return Some(Value::Error(
                            "async_http_post() headers must be a dictionary".to_string(),
                        ));
                    }
                }
            } else {
                None
            };

            // Create channel for result
            let (tx, rx) = tokio::sync::oneshot::channel();

            // Spawn async HTTP request
            AsyncRuntime::spawn_task(async move {
                let result = async {
                    let client = network_policy::build_async_http_client(
                        network_policy::default_http_timeout(),
                    )?;
                    let mut request = client.post(url.as_ref()).body(body.as_ref().clone());

                    // Add custom headers if provided
                    if let Some(headers_dict) = headers {
                        for (key, value) in headers_dict.iter() {
                            if let Value::Str(value_str) = value {
                                request = request.header(key.as_ref(), value_str.as_str());
                            }
                        }
                    }

                    let response = request
                        .send()
                        .await
                        .map_err(|e| format!("async_http_post failed: {}", e))?;

                    let (status, headers_map, response_body) =
                        network_policy::read_async_http_response_text(response, "async_http_post")
                            .await?;

                    // Build result dictionary
                    let mut result_dict = DictMap::default();
                    result_dict.insert("status".into(), Value::Int(status as i64));
                    result_dict.insert("body".into(), Value::Str(Arc::new(response_body)));

                    // Convert headers to dict
                    let mut headers_dict = DictMap::default();
                    for (name, value) in headers_map.iter() {
                        if let Ok(value_str) = value.to_str() {
                            headers_dict.insert(
                                name.to_string().into(),
                                Value::Str(Arc::new(value_str.to_string())),
                            );
                        }
                    }
                    result_dict.insert("headers".into(), Value::Dict(Arc::new(headers_dict)));

                    Ok::<Value, String>(Value::Dict(Arc::new(result_dict)))
                }
                .await;

                let _ = tx.send(result);
                Value::Null
            });

            // Return promise
            Some(Value::Promise {
                receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                task_handle: None,
            })
        }

        "async_read_file" => {
            // async_read_file(path: String) -> Promise<String>
            // Non-blocking file read
            if args.len() != 1 {
                return Some(Value::Error(format!(
                    "async_read_file() expects 1 argument (path), got {}",
                    args.len()
                )));
            }

            let path = match &args[0] {
                Value::Str(s) => s.clone(),
                _ => {
                    return Some(Value::Error(
                        "async_read_file() requires a string path argument".to_string(),
                    ));
                }
            };

            // Create channel for result
            let (tx, rx) = tokio::sync::oneshot::channel();

            // Spawn async file read
            AsyncRuntime::spawn_task(async move {
                let result = async {
                    tokio::fs::read_to_string(path.as_ref())
                        .await
                        .map(|s| Value::Str(Arc::new(s)))
                        .map_err(|e| format!("Failed to read file '{}': {}", path.as_ref(), e))
                }
                .await;

                let _ = tx.send(result);
                Value::Null
            });

            // Return promise
            Some(Value::Promise {
                receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                task_handle: None,
            })
        }

        "async_read_files" => {
            // async_read_files(paths: Array<String>, concurrency_limit?: Int) -> Promise<Array<String>>
            // Reads multiple files concurrently with bounded in-task concurrency.
            if args.len() != 1 && args.len() != 2 {
                return Some(Value::Error(format!(
                    "async_read_files() expects 1 or 2 arguments (paths, optional concurrency_limit), got {}",
                    args.len()
                )));
            }

            let path_values = match &args[0] {
                Value::Array(arr) => arr.clone(),
                _ => {
                    return Some(Value::Error(
                        "async_read_files() first argument must be an array of string paths"
                            .to_string(),
                    ));
                }
            };

            let concurrency_limit = if args.len() == 2 {
                match &args[1] {
                    Value::Int(n) if *n > 0 => *n as usize,
                    Value::Int(n) => {
                        return Some(Value::Error(format!(
                            "async_read_files() concurrency_limit must be > 0, got {}",
                            n
                        )));
                    }
                    _ => {
                        return Some(Value::Error(
                            "async_read_files() optional concurrency_limit must be an integer"
                                .to_string(),
                        ));
                    }
                }
            } else {
                _interp.get_async_task_pool_size()
            };

            if path_values.is_empty() {
                return Some(resolved_promise(Ok(Value::Array(Arc::new(vec![])))));
            }

            let mut paths = Vec::with_capacity(path_values.len());
            for (idx, value) in path_values.iter().enumerate() {
                match value {
                    Value::Str(path) => paths.push(path.as_ref().clone()),
                    _ => {
                        return Some(Value::Error(format!(
                            "async_read_files() path at index {} must be a string",
                            idx
                        )));
                    }
                }
            }

            let (tx, rx) = tokio::sync::oneshot::channel();
            let count = paths.len();
            let effective_batch_size = concurrency_limit.min(count.max(1));

            AsyncRuntime::spawn_task(async move {
                let mut results = vec![Value::Null; count];
                let mut pending = paths.into_iter().enumerate();
                let mut in_flight = FuturesUnordered::new();

                let make_read_future = |idx: usize, path: String| async move {
                    let read_result = tokio::fs::read_to_string(path.as_str()).await;
                    (idx, path, read_result)
                };

                for _ in 0..effective_batch_size {
                    match pending.next() {
                        Some((idx, path)) => in_flight.push(make_read_future(idx, path)),
                        None => break,
                    }
                }

                while let Some((idx, path, read_result)) = in_flight.next().await {
                    match read_result {
                        Ok(content) => {
                            results[idx] = Value::Str(Arc::new(content));
                        }
                        Err(e) => {
                            let _ = tx.send(Err(format!(
                                "Failed to read file '{}' (index {}): {}",
                                path, idx, e
                            )));
                            return Value::Null;
                        }
                    }

                    if let Some((next_idx, next_path)) = pending.next() {
                        in_flight.push(make_read_future(next_idx, next_path));
                    }
                }

                let _ = tx.send(Ok(Value::Array(Arc::new(results))));
                Value::Null
            });

            Some(Value::Promise {
                receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                task_handle: None,
            })
        }

        "async_write_file" => {
            // async_write_file(path: String, content: String) -> Promise<Bool>
            // Non-blocking file write
            if args.len() != 2 {
                return Some(Value::Error(format!(
                    "async_write_file() expects 2 arguments (path, content), got {}",
                    args.len()
                )));
            }

            let path = match &args[0] {
                Value::Str(s) => s.clone(),
                _ => {
                    return Some(Value::Error(
                        "async_write_file() requires a string path argument".to_string(),
                    ));
                }
            };

            let content = match &args[1] {
                Value::Str(s) => s.clone(),
                _ => {
                    return Some(Value::Error(
                        "async_write_file() requires a string content argument".to_string(),
                    ));
                }
            };

            // Create channel for result
            let (tx, rx) = tokio::sync::oneshot::channel();

            // Spawn async file write
            AsyncRuntime::spawn_task(async move {
                let result = async {
                    tokio::fs::write(path.as_ref(), content.as_ref())
                        .await
                        .map(|_| Value::Bool(true))
                        .map_err(|e| format!("Failed to write file '{}': {}", path.as_ref(), e))
                }
                .await;

                let _ = tx.send(result);
                Value::Null
            });

            // Return promise
            Some(Value::Promise {
                receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                task_handle: None,
            })
        }

        "async_write_files" => {
            // async_write_files(paths: Array<String>, contents: Array<String>, concurrency_limit?: Int)
            //   -> Promise<Array<Bool>>
            // Writes multiple files concurrently with bounded in-task concurrency.
            if args.len() != 2 && args.len() != 3 {
                return Some(Value::Error(format!(
                    "async_write_files() expects 2 or 3 arguments (paths, contents, optional concurrency_limit), got {}",
                    args.len()
                )));
            }

            let path_values = match &args[0] {
                Value::Array(arr) => arr.clone(),
                _ => {
                    return Some(Value::Error(
                        "async_write_files() first argument must be an array of string paths"
                            .to_string(),
                    ));
                }
            };

            let content_values = match &args[1] {
                Value::Array(arr) => arr.clone(),
                _ => {
                    return Some(Value::Error(
                        "async_write_files() second argument must be an array of string contents"
                            .to_string(),
                    ));
                }
            };

            if path_values.len() != content_values.len() {
                return Some(Value::Error(format!(
                    "async_write_files() paths and contents arrays must have the same length (paths={}, contents={})",
                    path_values.len(),
                    content_values.len()
                )));
            }

            let concurrency_limit = if args.len() == 3 {
                match &args[2] {
                    Value::Int(n) if *n > 0 => *n as usize,
                    Value::Int(n) => {
                        return Some(Value::Error(format!(
                            "async_write_files() concurrency_limit must be > 0, got {}",
                            n
                        )));
                    }
                    _ => {
                        return Some(Value::Error(
                            "async_write_files() optional concurrency_limit must be an integer"
                                .to_string(),
                        ));
                    }
                }
            } else {
                _interp.get_async_task_pool_size()
            };

            if path_values.is_empty() {
                return Some(resolved_promise(Ok(Value::Array(Arc::new(vec![])))));
            }

            let mut writes = Vec::with_capacity(path_values.len());
            for (idx, (path_value, content_value)) in
                path_values.iter().zip(content_values.iter()).enumerate()
            {
                let path = match path_value {
                    Value::Str(path) => path.as_ref().clone(),
                    _ => {
                        return Some(Value::Error(format!(
                            "async_write_files() path at index {} must be a string",
                            idx
                        )));
                    }
                };

                let content = match content_value {
                    Value::Str(content) => content.as_ref().clone(),
                    _ => {
                        return Some(Value::Error(format!(
                            "async_write_files() content at index {} must be a string",
                            idx
                        )));
                    }
                };

                writes.push((path, content));
            }

            let (tx, rx) = tokio::sync::oneshot::channel();
            let count = writes.len();
            let effective_batch_size = concurrency_limit.min(count.max(1));

            AsyncRuntime::spawn_task(async move {
                let mut results = vec![Value::Null; count];
                let mut pending = writes.into_iter().enumerate();
                let mut in_flight = FuturesUnordered::new();

                let make_write_future = |idx: usize, path: String, content: String| async move {
                    let write_result = tokio::fs::write(path.as_str(), content.as_str()).await;
                    (idx, path, write_result)
                };

                for _ in 0..effective_batch_size {
                    match pending.next() {
                        Some((idx, (path, content))) => {
                            in_flight.push(make_write_future(idx, path, content))
                        }
                        None => break,
                    }
                }

                while let Some((idx, path, write_result)) = in_flight.next().await {
                    match write_result {
                        Ok(_) => {
                            results[idx] = Value::Bool(true);
                        }
                        Err(e) => {
                            let _ = tx.send(Err(format!(
                                "Failed to write file '{}' (index {}): {}",
                                path, idx, e
                            )));
                            return Value::Null;
                        }
                    }

                    if let Some((next_idx, (next_path, next_content))) = pending.next() {
                        in_flight.push(make_write_future(next_idx, next_path, next_content));
                    }
                }

                let _ = tx.send(Ok(Value::Array(Arc::new(results))));
                Value::Null
            });

            Some(Value::Promise {
                receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                task_handle: None,
            })
        }

        "ssg_render_and_write_pages" => {
            // ssg_render_and_write_pages(source_pages: Array<String>, output_dir: String, concurrency_limit?: Int)
            //   -> Promise<Dict{ checksum: Int, files: Int }>
            // Renders HTML pages and writes them to indexed output paths in one bounded-concurrency async pass.
            if args.len() != 2 && args.len() != 3 {
                return Some(Value::Error(format!(
                    "ssg_render_and_write_pages() expects 2 or 3 arguments (source_pages, output_dir, optional concurrency_limit), got {}",
                    args.len()
                )));
            }

            let source_pages = match &args[0] {
                Value::Array(arr) => arr.clone(),
                _ => {
                    return Some(Value::Error(
                        "ssg_render_and_write_pages() first argument must be an array of string source pages"
                            .to_string(),
                    ));
                }
            };

            let output_dir = match &args[1] {
                Value::Str(dir) => dir.as_ref().clone(),
                _ => {
                    return Some(Value::Error(
                        "ssg_render_and_write_pages() second argument must be a string output_dir"
                            .to_string(),
                    ));
                }
            };

            let concurrency_limit = if args.len() == 3 {
                match &args[2] {
                    Value::Int(n) if *n > 0 => *n as usize,
                    Value::Int(n) => {
                        return Some(Value::Error(format!(
                            "ssg_render_and_write_pages() concurrency_limit must be > 0, got {}",
                            n
                        )));
                    }
                    _ => {
                        return Some(Value::Error(
                            "ssg_render_and_write_pages() optional concurrency_limit must be an integer"
                                .to_string(),
                        ));
                    }
                }
            } else {
                _interp.get_async_task_pool_size()
            };

            let mut source_bodies = Vec::with_capacity(source_pages.len());

            for (index, page) in source_pages.iter().enumerate() {
                let source_body = match page {
                    Value::Str(body) => body,
                    _ => {
                        return Some(Value::Error(format!(
                            "ssg_render_and_write_pages() source page at index {} must be a string",
                            index
                        )));
                    }
                };

                source_bodies.push(source_body.clone());
            }

            if source_bodies.is_empty() {
                let mut result = DictMap::default();
                result.insert("checksum".into(), Value::Int(0));
                result.insert("files".into(), Value::Int(0));
                return Some(resolved_promise(Ok(Value::Dict(Arc::new(result)))));
            }

            let file_count = source_bodies.len();
            let effective_batch_size = concurrency_limit.min(file_count.max(1));
            let (output_paths_for_batch, render_prefixes_for_batch) =
                ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), file_count);
            let output_paths_for_tasks = Arc::new(output_paths_for_batch);
            let render_prefixes_for_tasks = render_prefixes_for_batch;

            let (tx, rx) = tokio::sync::oneshot::channel();

            AsyncRuntime::spawn_task(async move {
                if effective_batch_size == 1 {
                    let mut checksum: i64 = 0;

                    for (index, source_body) in source_bodies.into_iter().enumerate() {
                        let write_result = ssg_write_rendered_html_page(
                            output_paths_for_tasks[index].as_str(),
                            render_prefixes_for_tasks[index].as_ref(),
                            source_body.as_ref().as_str(),
                        )
                        .await;

                        match write_result {
                            Ok(written_len) => checksum += written_len as i64,
                            Err(e) => {
                                let _ = tx.send(Err(format!(
                                    "Failed to write file '{}' (index {}): {}",
                                    output_paths_for_tasks[index].as_str(),
                                    index,
                                    e
                                )));
                                return Value::Null;
                            }
                        }
                    }

                    let mut result = DictMap::default();
                    result.insert("checksum".into(), Value::Int(checksum));
                    result.insert("files".into(), Value::Int(file_count as i64));

                    let _ = tx.send(Ok(Value::Dict(Arc::new(result))));
                    return Value::Null;
                }

                let mut pending = source_bodies.into_iter().enumerate();
                let mut in_flight = FuturesUnordered::new();
                let mut checksum: i64 = 0;

                let make_write_future = |index: usize, source_body: Arc<String>| {
                    let output_paths = output_paths_for_tasks.clone();
                    let render_prefixes = render_prefixes_for_tasks.clone();

                    async move {
                        let write_result = ssg_write_rendered_html_page(
                            output_paths[index].as_str(),
                            render_prefixes[index].as_ref(),
                            source_body.as_ref().as_str(),
                        )
                        .await;
                        (index, write_result)
                    }
                };

                for _ in 0..effective_batch_size {
                    match pending.next() {
                        Some((index, source_body)) => {
                            in_flight.push(make_write_future(index, source_body))
                        }
                        None => break,
                    }
                }

                while let Some((index, write_result)) = in_flight.next().await {
                    match write_result {
                        Ok(written_len) => {
                            checksum += written_len as i64;
                        }
                        Err(e) => {
                            let _ = tx.send(Err(format!(
                                "Failed to write file '{}' (index {}): {}",
                                output_paths_for_tasks[index].as_str(),
                                index,
                                e
                            )));
                            return Value::Null;
                        }
                    }

                    if let Some((next_index, next_source_body)) = pending.next() {
                        in_flight.push(make_write_future(next_index, next_source_body));
                    }
                }

                let mut result = DictMap::default();
                result.insert("checksum".into(), Value::Int(checksum));
                result.insert("files".into(), Value::Int(file_count as i64));

                let _ = tx.send(Ok(Value::Dict(Arc::new(result))));
                Value::Null
            });

            Some(Value::Promise {
                receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                task_handle: None,
            })
        }

        "ssg_read_render_and_write_pages" => {
            // ssg_read_render_and_write_pages(source_paths: Array<String>, output_dir: String, concurrency_limit?: Int)
            //   -> Promise<Dict{ checksum: Int, files: Int, read_ms: Float, render_write_ms: Float }>
            // Reads source files, renders HTML, and writes indexed output pages in one bounded-concurrency async operation.
            if args.len() != 2 && args.len() != 3 {
                return Some(Value::Error(format!(
                    "ssg_read_render_and_write_pages() expects 2 or 3 arguments (source_paths, output_dir, optional concurrency_limit), got {}",
                    args.len()
                )));
            }

            let source_path_values = match &args[0] {
                Value::Array(arr) => arr.clone(),
                _ => {
                    return Some(Value::Error(
                        "ssg_read_render_and_write_pages() first argument must be an array of string source paths"
                            .to_string(),
                    ));
                }
            };

            let output_dir = match &args[1] {
                Value::Str(dir) => dir.as_ref().clone(),
                _ => {
                    return Some(Value::Error(
                        "ssg_read_render_and_write_pages() second argument must be a string output_dir"
                            .to_string(),
                    ));
                }
            };

            let concurrency_limit = if args.len() == 3 {
                match &args[2] {
                    Value::Int(n) if *n > 0 => *n as usize,
                    Value::Int(n) => {
                        return Some(Value::Error(format!(
                            "ssg_read_render_and_write_pages() concurrency_limit must be > 0, got {}",
                            n
                        )));
                    }
                    _ => {
                        return Some(Value::Error(
                            "ssg_read_render_and_write_pages() optional concurrency_limit must be an integer"
                                .to_string(),
                        ));
                    }
                }
            } else {
                _interp.get_async_task_pool_size()
            };

            let mut source_paths = Vec::with_capacity(source_path_values.len());
            for (index, path_value) in source_path_values.iter().enumerate() {
                match path_value {
                    Value::Str(path) => source_paths.push(path.as_ref().clone()),
                    _ => {
                        return Some(Value::Error(format!(
                            "ssg_read_render_and_write_pages() source path at index {} must be a string",
                            index
                        )));
                    }
                }
            }

            if source_paths.is_empty() {
                let mut result = DictMap::default();
                result.insert("checksum".into(), Value::Int(0));
                result.insert("files".into(), Value::Int(0));
                result.insert("read_ms".into(), Value::Float(0.0));
                result.insert("render_write_ms".into(), Value::Float(0.0));
                return Some(resolved_promise(Ok(Value::Dict(Arc::new(result)))));
            }

            let file_count = source_paths.len();
            let effective_batch_size = concurrency_limit.min(file_count.max(1));
            let (output_suffixes_vec, render_prefixes_vec) =
                ssg_get_cached_output_suffixes_and_prefixes(file_count);
            let collect_stage_metrics = ssg_stage_profile_enabled_from_env();

            let (tx, rx) = tokio::sync::oneshot::channel();

            // Dispatch the two-phase Rayon pipeline via spawn_blocking so the
            // Tokio runtime thread is not blocked during synchronous I/O.
            AsyncRuntime::spawn_task(async move {
                let blocking_result = tokio::task::spawn_blocking(move || {
                    ssg_run_rayon_read_render_write_with_reused_output_path_buffer(
                        source_paths,
                        output_dir,
                        output_suffixes_vec,
                        render_prefixes_vec,
                        effective_batch_size,
                        collect_stage_metrics,
                    )
                })
                .await;

                let send_result = match blocking_result {
                    Ok(Ok((checksum, read_ms, render_write_ms))) => {
                        let mut result = DictMap::default();
                        result.insert("checksum".into(), Value::Int(checksum));
                        result.insert("files".into(), Value::Int(file_count as i64));
                        result.insert("read_ms".into(), Value::Float(read_ms));
                        result.insert("render_write_ms".into(), Value::Float(render_write_ms));
                        Ok(Value::Dict(Arc::new(result)))
                    }
                    Ok(Err(e)) => Err(e),
                    Err(join_error) => Err(format!(
                        "ssg_read_render_and_write_pages() blocking task failed: {}",
                        join_error
                    )),
                };

                let _ = tx.send(send_result);
                Value::Null
            });

            Some(Value::Promise {
                receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                task_handle: None,
            })
        }

        "spawn_task" => {
            // spawn_task(async_func: AsyncFunction) -> TaskHandle
            // Spawn a background task that runs independently
            if args.len() != 1 {
                return Some(Value::Error(format!(
                    "spawn_task() expects 1 argument (async function), got {}",
                    args.len()
                )));
            }

            let func = match &args[0] {
                Value::AsyncFunction(params, body, env) => {
                    (params.clone(), body.clone(), env.clone())
                }
                Value::Function(params, body, env) => {
                    // Allow regular functions to be spawned as tasks too
                    (params.clone(), body.clone(), env.clone())
                }
                _ => {
                    return Some(Value::Error(
                        "spawn_task() requires an async function argument".to_string(),
                    ));
                }
            };

            let (_params, _body, _env) = func;

            // Clone interpreter context needed for execution
            // Note: We need to pass the interpreter or create a way to execute
            // For now, we'll create a simple task that just returns the function
            // In a real implementation, we'd need to execute the function body
            // Deferred post-v1 runtime backlog: execute task bodies with full interpreter context
            // (see docs/V1_SCOPE.md deferred runtime execution section).

            // Create the task handle
            let is_cancelled = std::sync::Arc::new(std::sync::Mutex::new(false));
            let is_cancelled_clone = is_cancelled.clone();

            // Spawn the task
            let handle = AsyncRuntime::spawn_task(async move {
                // Check if cancelled
                {
                    let cancelled = match lock_or_async_error(
                        is_cancelled_clone.as_ref(),
                        "spawn_task.is_cancelled",
                    ) {
                        Ok(guard) => guard,
                        Err(error) => return Value::Error(error),
                    };
                    if *cancelled {
                        return Value::Error("Task was cancelled".to_string());
                    }
                }

                // For now, just sleep to simulate work
                // Deferred post-v1 runtime backlog: execute the actual function body with interpreter.
                AsyncRuntime::sleep(std::time::Duration::from_millis(1)).await;

                // Return placeholder - in full implementation would execute function
                Value::Null
            });

            Some(Value::TaskHandle {
                handle: std::sync::Arc::new(std::sync::Mutex::new(Some(handle))),
                is_cancelled,
            })
        }

        "await_task" => {
            // await_task(task_handle: TaskHandle) -> Promise<Value>
            // Wait for a spawned task to complete and get its result
            if args.len() != 1 {
                return Some(Value::Error(format!(
                    "await_task() expects 1 argument (task handle), got {}",
                    args.len()
                )));
            }

            match &args[0] {
                Value::TaskHandle { handle: handle_arc, is_cancelled } => {
                    let handle_arc = handle_arc.clone();
                    let is_cancelled = is_cancelled.clone();

                    // Create channel for result
                    let (tx, rx) = tokio::sync::oneshot::channel();

                    // Spawn task to await the handle
                    AsyncRuntime::spawn_task(async move {
                        // Extract the handle
                        let handle = {
                            let mut handle_guard =
                                match lock_or_async_error(handle_arc.as_ref(), "await_task.handle")
                                {
                                    Ok(guard) => guard,
                                    Err(error) => {
                                        let _ = tx.send(Err(error));
                                        return Value::Null;
                                    }
                                };
                            handle_guard.take()
                        };

                        let result = if let Some(h) = handle {
                            // Check if cancelled (drop guard before await)
                            let is_task_cancelled = {
                                let cancelled = match lock_or_async_error(
                                    is_cancelled.as_ref(),
                                    "await_task.is_cancelled",
                                ) {
                                    Ok(guard) => guard,
                                    Err(error) => {
                                        let _ = tx.send(Err(error));
                                        return Value::Null;
                                    }
                                };
                                *cancelled
                            };

                            if is_task_cancelled {
                                Err("Task was cancelled".to_string())
                            } else {
                                // Await the task completion
                                match h.await {
                                    Ok(value) => Ok(value),
                                    Err(e) => Err(format!("Task panicked: {}", e)),
                                }
                            }
                        } else {
                            Err("Task handle already consumed".to_string())
                        };

                        let _ = tx.send(result);
                        Value::Null
                    });

                    // Return promise
                    Some(Value::Promise {
                        receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                        is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                        cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                        task_handle: None,
                    })
                }
                _ => Some(Value::Error("await_task() requires a TaskHandle argument".to_string())),
            }
        }

        "cancel_task" => {
            // cancel_task(task_handle: TaskHandle) -> Bool
            // Request cancellation of a running task
            if args.len() != 1 {
                return Some(Value::Error(format!(
                    "cancel_task() expects 1 argument (task handle), got {}",
                    args.len()
                )));
            }

            match &args[0] {
                Value::TaskHandle { handle: handle_arc, is_cancelled } => {
                    // Mark as cancelled
                    {
                        let mut cancelled = match lock_or_async_error(
                            is_cancelled.as_ref(),
                            "cancel_task.is_cancelled",
                        ) {
                            Ok(guard) => guard,
                            Err(error) => return Some(Value::Error(error)),
                        };
                        *cancelled = true;
                    }

                    // Abort the task if possible
                    let mut handle_guard =
                        match lock_or_async_error(handle_arc.as_ref(), "cancel_task.handle") {
                            Ok(guard) => guard,
                            Err(error) => return Some(Value::Error(error)),
                        };
                    if let Some(handle) = handle_guard.take() {
                        handle.abort();
                        Some(Value::Bool(true))
                    } else {
                        Some(Value::Bool(false)) // Already consumed
                    }
                }
                _ => Some(Value::Error("cancel_task() requires a TaskHandle argument".to_string())),
            }
        }

        "Promise.all" | "promise_all" => {
            // Promise.all(promises: Array<Promise>, concurrency_limit?: Int) -> Promise<Array<Value>>
            // Await all promises concurrently, return array of results
            // If any promise rejects, the whole operation fails with first error
            if args.len() != 1 && args.len() != 2 {
                return Some(Value::Error(format!(
                    "Promise.all() expects 1 or 2 arguments (array of promises, optional concurrency_limit), got {}",
                    args.len()
                )));
            }

            // Extract array of promises
            let promises = match &args[0] {
                Value::Array(arr) => arr.clone(),
                _ => {
                    return Some(Value::Error(
                        "Promise.all() requires an array of promises".to_string(),
                    ));
                }
            };

            // Optional concurrency limit (batch size for awaiting)
            let concurrency_limit = if args.len() == 2 {
                match &args[1] {
                    Value::Int(n) if *n > 0 => *n as usize,
                    Value::Int(n) => {
                        return Some(Value::Error(format!(
                            "Promise.all() concurrency_limit must be > 0, got {}",
                            n
                        )));
                    }
                    _ => {
                        return Some(Value::Error(
                            "Promise.all() optional concurrency_limit must be an integer"
                                .to_string(),
                        ));
                    }
                }
            } else {
                _interp.get_async_task_pool_size()
            };

            let debug_async = std::env::var("DEBUG_ASYNC").is_ok();

            // Debug logging
            if debug_async {
                eprintln!("Promise.all: received {} promises", promises.len());
            }

            if promises.is_empty() {
                // Empty array - return immediately resolved promise with empty array
                let (tx, rx) = tokio::sync::oneshot::channel();
                let _ = tx.send(Ok(Value::Array(Arc::new(vec![]))));
                return Some(Value::Promise {
                    receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                    is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                    cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                    task_handle: None,
                });
            }

            let mut results = vec![Value::Null; promises.len()];
            // (result_index, receiver, is_polled, cached_result)
            let mut pending_promises = Vec::with_capacity(promises.len());

            for (idx, promise) in promises.iter().enumerate() {
                match promise {
                    Value::Promise { receiver, is_polled, cached_result, .. } => {
                        if let Some(cached) = read_cached_promise_result(is_polled, cached_result) {
                            match cached {
                                Ok(value) => {
                                    results[idx] = value;
                                }
                                Err(err) => {
                                    return Some(resolved_promise(Err(format!(
                                        "Promise {} rejected: {}",
                                        idx, err
                                    ))));
                                }
                            }
                            continue;
                        }

                        if debug_async {
                            eprintln!("Promise.all: extracting receiver from promise {}", idx);
                        }
                        pending_promises.push((
                            idx,
                            receiver.clone(),
                            is_polled.clone(),
                            cached_result.clone(),
                        ));
                    }
                    _ => {
                        return Some(Value::Error(format!(
                            "Promise.all() array element {} is not a Promise",
                            idx
                        )));
                    }
                }
            }

            if pending_promises.is_empty() {
                return Some(resolved_promise(Ok(Value::Array(Arc::new(results)))));
            }

            if debug_async {
                eprintln!(
                    "Promise.all: extracted {} pending receivers ({} served from cache), spawning task",
                    pending_promises.len(),
                    promises.len() - pending_promises.len()
                );
            }

            // Create channel for final result
            let (tx, rx) = tokio::sync::oneshot::channel();
            let count = pending_promises.len();
            let effective_batch_size = concurrency_limit.min(count.max(1));

            // Spawn task that awaits all promises concurrently
            AsyncRuntime::spawn_task(async move {
                if debug_async {
                    eprintln!(
                        "Promise.all: inside spawned task, extracting {} receivers",
                        pending_promises.len()
                    );
                }

                // Extract all receivers
                let mut futures = Vec::with_capacity(count);
                for (idx, receiver_arc, is_polled, cached_result) in pending_promises {
                    let actual_rx = {
                        let mut recv_guard = match lock_or_async_error(
                            receiver_arc.as_ref(),
                            "Promise.all.receiver",
                        ) {
                            Ok(guard) => guard,
                            Err(error) => {
                                let _ = tx.send(Err(error));
                                return Value::Null;
                            }
                        };
                        let (dummy_tx, dummy_rx) = tokio::sync::oneshot::channel();
                        drop(dummy_tx);
                        std::mem::replace(&mut *recv_guard, dummy_rx)
                    };
                    futures.push((idx, actual_rx, is_polled, cached_result));
                }

                if debug_async {
                    eprintln!(
                        "Promise.all: prepared {} futures, awaiting with bounded in-task concurrency",
                        futures.len()
                    );
                }

                // Collect all results
                let mut results = vec![Value::Null; count];
                let mut completed = 0;

                // Await receivers with bounded in-task concurrency (no per-receiver tokio::spawn overhead)
                let mut pending = futures.into_iter();
                let mut in_flight = FuturesUnordered::new();
                let make_wait_future = |idx, rx, is_polled, cached_result| async move {
                    (idx, rx.await, is_polled, cached_result)
                };

                for _ in 0..effective_batch_size {
                    match pending.next() {
                        Some((idx, rx, is_polled, cached_result)) => {
                            in_flight.push(make_wait_future(idx, rx, is_polled, cached_result));
                        }
                        None => break,
                    }
                }

                while let Some((idx, recv_result, is_polled, cached_result)) =
                    in_flight.next().await
                {
                    if debug_async {
                        eprintln!("Promise.all: awaiting task {}/{}", completed + 1, count);
                    }

                    match recv_result {
                        Ok(Ok(value)) => {
                            if let Err(error) =
                                cache_promise_result(&is_polled, &cached_result, Ok(value.clone()))
                            {
                                let _ = tx.send(Err(error));
                                return Value::Null;
                            }
                            if debug_async {
                                eprintln!("Promise.all: task {} resolved successfully", idx);
                            }
                            results[idx] = value;
                            completed += 1;
                        }
                        Ok(Err(err)) => {
                            if let Err(error) =
                                cache_promise_result(&is_polled, &cached_result, Err(err.clone()))
                            {
                                let _ = tx.send(Err(error));
                                return Value::Null;
                            }
                            if debug_async {
                                eprintln!("Promise.all: task {} rejected: {}", idx, err);
                            }
                            let _ = tx.send(Err(format!("Promise {} rejected: {}", idx, err)));
                            return Value::Null;
                        }
                        Err(_) => {
                            let channel_error =
                                "Promise never resolved (channel closed)".to_string();
                            if let Err(error) = cache_promise_result(
                                &is_polled,
                                &cached_result,
                                Err(channel_error.clone()),
                            ) {
                                let _ = tx.send(Err(error));
                                return Value::Null;
                            }
                            if debug_async {
                                eprintln!(
                                    "Promise.all: task {} channel closed (Err from oneshot)",
                                    idx
                                );
                            }
                            let _ = tx.send(Err(format!(
                                "Promise {} never resolved (channel closed)",
                                idx
                            )));
                            return Value::Null;
                        }
                    }

                    if let Some((next_idx, next_rx, next_is_polled, next_cached_result)) =
                        pending.next()
                    {
                        in_flight.push(make_wait_future(
                            next_idx,
                            next_rx,
                            next_is_polled,
                            next_cached_result,
                        ));
                    }
                }

                if debug_async {
                    eprintln!("Promise.all: all tasks resolved, sending {} results", results.len());
                }

                // All promises resolved successfully
                let _ = tx.send(Ok(Value::Array(Arc::new(results))));
                Value::Null
            });

            // Return promise that resolves to array of results
            Some(Value::Promise {
                receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                task_handle: None,
            })
        }

        "await_all" => {
            // await_all is an alias for Promise.all
            // Redirect to Promise.all handler
            handle(_interp, "Promise.all", args)
        }

        "parallel_map" => {
            // parallel_map(array: Array, func: Function|NativeFunction, concurrency_limit?: Int) -> Promise<Array<Value>>
            // Apply a mapper across array elements and await all results with optional bounded batching.
            if args.len() != 2 && args.len() != 3 {
                return Some(Value::Error(format!(
                    "parallel_map() expects 2 or 3 arguments (array, function, optional concurrency_limit), got {}",
                    args.len()
                )));
            }

            let array = match &args[0] {
                Value::Array(arr) => arr.clone(),
                _ => {
                    return Some(Value::Error(
                        "parallel_map() first argument must be an array".to_string(),
                    ));
                }
            };

            let mapper = match &args[1] {
                func @ Value::NativeFunction(_)
                | func @ Value::Function(_, _, _)
                | func @ Value::BytecodeFunction { .. }
                | func @ Value::GeneratorDef(_, _) => func.clone(),
                _ => {
                    return Some(Value::Error(
                        "parallel_map() second argument must be a callable function; pass a function, closure, or imported callable value instead of a literal".to_string(),
                    ));
                }
            };

            let concurrency_limit = if args.len() == 3 {
                match &args[2] {
                    Value::Int(n) if *n > 0 => Some(*n),
                    Value::Int(n) => {
                        return Some(Value::Error(format!(
                            "parallel_map() concurrency_limit must be > 0, got {}",
                            n
                        )));
                    }
                    _ => {
                        return Some(Value::Error(
                            "parallel_map() optional concurrency_limit must be an integer"
                                .to_string(),
                        ));
                    }
                }
            } else {
                Some(_interp.get_async_task_pool_size() as i64)
            };

            if let Value::NativeFunction(mapper_name) = &mapper {
                let rayon_limit = concurrency_limit
                    .unwrap_or(_interp.get_async_task_pool_size() as i64)
                    .max(1) as usize;
                if let Some(rayon_result) =
                    try_parallel_map_with_rayon(mapper_name.as_str(), &array, rayon_limit)
                {
                    return Some(rayon_result);
                }
            }

            if let Some(jit_bytecode_result) =
                try_parallel_map_with_jit_bytecode(_interp, &mapper, &array)
            {
                return Some(jit_bytecode_result);
            }

            let mut mapped_results = vec![Value::Null; array.len()];
            // (result_index, receiver, is_polled, cached_result)
            let mut pending_receivers = Vec::new();

            for (idx, element) in array.iter().enumerate() {
                let mapped = match &mapper {
                    Value::NativeFunction(name) => {
                        _interp.call_native_function_impl(name, &[element.clone()])
                    }
                    _ => _interp.call_user_function(&mapper, &[element.clone()]),
                };

                match mapped {
                    Value::Error(_) | Value::ErrorObject { .. } => {
                        return Some(mapped);
                    }
                    Value::Promise { receiver, is_polled, cached_result, .. } => {
                        if let Some(cached) = read_cached_promise_result(&is_polled, &cached_result)
                        {
                            match cached {
                                Ok(value) => {
                                    mapped_results[idx] = value;
                                }
                                Err(err) => {
                                    return Some(resolved_promise(Err(format!(
                                        "Promise {} rejected: {}",
                                        idx, err
                                    ))));
                                }
                            }
                            continue;
                        }

                        pending_receivers.push((idx, receiver, is_polled, cached_result));
                    }
                    immediate => {
                        mapped_results[idx] = immediate;
                    }
                }
            }

            if pending_receivers.is_empty() {
                return Some(resolved_promise(Ok(Value::Array(Arc::new(mapped_results)))));
            }

            let count = pending_receivers.len();
            let effective_batch_size = (concurrency_limit
                .unwrap_or(_interp.get_async_task_pool_size() as i64)
                .max(1) as usize)
                .min(count.max(1));

            let (tx, rx) = tokio::sync::oneshot::channel();

            AsyncRuntime::spawn_task(async move {
                let mut futures = Vec::with_capacity(count);
                for (idx, receiver_arc, is_polled, cached_result) in pending_receivers {
                    let actual_rx = {
                        let mut recv_guard = match lock_or_async_error(
                            receiver_arc.as_ref(),
                            "parallel_map.receiver",
                        ) {
                            Ok(guard) => guard,
                            Err(error) => {
                                let _ = tx.send(Err(error));
                                return Value::Null;
                            }
                        };
                        let (dummy_tx, dummy_rx) = tokio::sync::oneshot::channel();
                        drop(dummy_tx);
                        std::mem::replace(&mut *recv_guard, dummy_rx)
                    };
                    futures.push((idx, actual_rx, is_polled, cached_result));
                }

                let mut pending = futures.into_iter();
                let mut in_flight = FuturesUnordered::new();
                let make_wait_future = |idx, rx, is_polled, cached_result| async move {
                    (idx, rx.await, is_polled, cached_result)
                };

                for _ in 0..effective_batch_size {
                    match pending.next() {
                        Some((idx, rx, is_polled, cached_result)) => {
                            in_flight.push(make_wait_future(idx, rx, is_polled, cached_result))
                        }
                        None => break,
                    }
                }

                while let Some((idx, recv_result, is_polled, cached_result)) =
                    in_flight.next().await
                {
                    match recv_result {
                        Ok(Ok(value)) => {
                            if let Err(error) =
                                cache_promise_result(&is_polled, &cached_result, Ok(value.clone()))
                            {
                                let _ = tx.send(Err(error));
                                return Value::Null;
                            }
                            mapped_results[idx] = value;
                        }
                        Ok(Err(err)) => {
                            if let Err(error) =
                                cache_promise_result(&is_polled, &cached_result, Err(err.clone()))
                            {
                                let _ = tx.send(Err(error));
                                return Value::Null;
                            }
                            let _ = tx.send(Err(format!("Promise {} rejected: {}", idx, err)));
                            return Value::Null;
                        }
                        Err(_) => {
                            let channel_error =
                                "Promise never resolved (channel closed)".to_string();
                            if let Err(error) = cache_promise_result(
                                &is_polled,
                                &cached_result,
                                Err(channel_error.clone()),
                            ) {
                                let _ = tx.send(Err(error));
                                return Value::Null;
                            }
                            let _ = tx.send(Err(format!(
                                "Promise {} never resolved (channel closed)",
                                idx
                            )));
                            return Value::Null;
                        }
                    }

                    if let Some((next_idx, next_rx, next_is_polled, next_cached_result)) =
                        pending.next()
                    {
                        in_flight.push(make_wait_future(
                            next_idx,
                            next_rx,
                            next_is_polled,
                            next_cached_result,
                        ));
                    }
                }

                let _ = tx.send(Ok(Value::Array(Arc::new(mapped_results))));
                Value::Null
            });

            Some(Value::Promise {
                receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                task_handle: None,
            })
        }

        "par_map" => {
            // par_map is an alias for parallel_map
            handle(_interp, "parallel_map", args)
        }

        "par_each" => {
            // par_each(array: Array, func: Function|NativeFunction, concurrency_limit?: Int) -> Promise<Null>
            // Apply a mapper across array elements concurrently and resolve when all work is complete.
            // Return value is discarded, but any rejection is propagated.
            let mapped_result = match handle(_interp, "parallel_map", args) {
                Some(value) => value,
                None => {
                    return Some(Value::Error(
                        "par_each() internal error: parallel_map handler unavailable".to_string(),
                    ));
                }
            };

            match mapped_result {
                Value::Promise { receiver, .. } => {
                    let (tx, rx) = tokio::sync::oneshot::channel();

                    AsyncRuntime::spawn_task(async move {
                        let await_result = {
                            let rx = {
                                let mut receiver_guard = match lock_or_async_error(
                                    receiver.as_ref(),
                                    "par_each.receiver",
                                ) {
                                    Ok(guard) => guard,
                                    Err(error) => {
                                        let _ = tx.send(Err(error));
                                        return Value::Null;
                                    }
                                };
                                let (dummy_tx, dummy_rx) = tokio::sync::oneshot::channel();
                                drop(dummy_tx);
                                std::mem::replace(&mut *receiver_guard, dummy_rx)
                            };

                            rx.await
                        };

                        match await_result {
                            Ok(Ok(_)) => {
                                let _ = tx.send(Ok(Value::Null));
                            }
                            Ok(Err(err)) => {
                                let _ = tx.send(Err(err));
                            }
                            Err(_) => {
                                let _ = tx.send(Err(
                                    "Promise channel closed before resolution".to_string()
                                ));
                            }
                        }

                        Value::Null
                    });

                    Some(Value::Promise {
                        receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                        is_polled: std::sync::Arc::new(std::sync::Mutex::new(false)),
                        cached_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
                        task_handle: None,
                    })
                }
                immediate => Some(immediate),
            }
        }

        "set_task_pool_size" => {
            // set_task_pool_size(size: Int) -> Int
            // Sets default async task pool size used when no explicit concurrency_limit is provided.
            // Returns previous size.
            if args.len() != 1 {
                return Some(Value::Error(format!(
                    "set_task_pool_size() expects 1 argument (size), got {}",
                    args.len()
                )));
            }

            let size = match &args[0] {
                Value::Int(n) if *n > 0 => *n as usize,
                Value::Int(n) => {
                    return Some(Value::Error(format!(
                        "set_task_pool_size() size must be > 0, got {}",
                        n
                    )));
                }
                _ => {
                    return Some(Value::Error(
                        "set_task_pool_size() requires an integer size argument".to_string(),
                    ));
                }
            };

            let previous_size = _interp.set_async_task_pool_size(size);
            Some(Value::Int(previous_size as i64))
        }

        "get_task_pool_size" => {
            // get_task_pool_size() -> Int
            // Returns default async task pool size used by promise_all/await_all/parallel_map
            // when no explicit concurrency_limit is provided.
            if !args.is_empty() {
                return Some(Value::Error(format!(
                    "get_task_pool_size() expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Some(Value::Int(_interp.get_async_task_pool_size() as i64))
        }

        _ => None, // Not handled by this module
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{BytecodeChunk, Constant, OpCode};
    use crate::interpreter::Interpreter;
    use std::collections::HashMap;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn string_value(s: &str) -> Value {
        Value::Str(Arc::new(s.to_string()))
    }

    fn await_promise(value: Value) -> Result<Value, String> {
        match value {
            Value::Promise { receiver, .. } => AsyncRuntime::block_on(async {
                let rx = {
                    let mut receiver_guard = receiver.lock().unwrap();
                    let (dummy_tx, dummy_rx) = tokio::sync::oneshot::channel();
                    drop(dummy_tx);
                    std::mem::replace(&mut *receiver_guard, dummy_rx)
                };

                match rx.await {
                    Ok(result) => result,
                    Err(_) => Err("Promise channel closed before resolution".to_string()),
                }
            }),
            _ => panic!("Expected Promise value"),
        }
    }

    fn poison_mutex<T: Send + 'static>(mutex: Arc<Mutex<T>>) {
        let _ = std::thread::spawn(move || {
            let _guard = mutex.lock().expect("poison helper should acquire lock");
            panic!("intentional poison");
        })
        .join();
    }

    fn with_outbound_policy_env<F>(policy: Option<&str>, run: F)
    where
        F: FnOnce(),
    {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let previous_policy =
            std::env::var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV).ok();

        match policy {
            Some(value) => {
                std::env::set_var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV, value)
            }
            None => std::env::remove_var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV),
        }

        run();

        match previous_policy {
            Some(value) => {
                std::env::set_var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV, value)
            }
            None => std::env::remove_var(crate::network_policy::OUTBOUND_DESTINATION_POLICY_ENV),
        }
    }

    #[test]
    fn read_cached_promise_result_returns_error_when_lock_poisoned() {
        let is_polled = Arc::new(Mutex::new(false));
        let cached_result = Arc::new(Mutex::new(None::<Result<Value, String>>));
        poison_mutex(is_polled.clone());

        let result = read_cached_promise_result(&is_polled, &cached_result);
        match result {
            Some(Err(message)) => {
                assert!(message.contains("shared async state lock poisoned"), "{}", message)
            }
            other => panic!("expected poisoned lock error, got {:?}", other),
        }
    }

    #[test]
    fn cache_promise_result_returns_error_when_lock_poisoned() {
        let is_polled = Arc::new(Mutex::new(false));
        let cached_result = Arc::new(Mutex::new(None::<Result<Value, String>>));
        poison_mutex(cached_result.clone());

        let result = cache_promise_result(&is_polled, &cached_result, Ok(Value::Null));
        match result {
            Err(message) => assert!(message.contains("shared async state lock poisoned")),
            Ok(_) => panic!("expected poisoned lock error"),
        }
    }

    #[test]
    fn async_http_helpers_enforce_destination_policy_before_spawning_requests() {
        with_outbound_policy_env(Some("deny_private"), || {
            let mut interp = Interpreter::new();

            let get_result =
                handle(&mut interp, "async_http_get", &[string_value("http://127.0.0.1:1")])
                    .expect("async_http_get should be handled");
            assert!(
                matches!(&get_result, Value::Error(message) if message.contains("blocked by outbound destination policy")),
                "expected strict destination policy error, got {:?}",
                get_result
            );

            let post_result = handle(
                &mut interp,
                "async_http_post",
                &[string_value("http://127.0.0.1:1"), string_value("payload")],
            )
            .expect("async_http_post should be handled");
            assert!(
                matches!(&post_result, Value::Error(message) if message.contains("blocked by outbound destination policy")),
                "expected strict destination policy error, got {:?}",
                post_result
            );
        });
    }

    fn unique_temp_dir(prefix: &str) -> String {
        static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let counter = UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let process_id = std::process::id();
        let mut path = std::env::temp_dir();
        path.push(format!("{}_{}_{}_{}", prefix, process_id, nanos, counter));
        path.to_string_lossy().to_string()
    }

    fn bytecode_increment_mapper() -> Value {
        let mut chunk = BytecodeChunk::new();
        chunk.name = Some("jit_parallel_increment".to_string());
        chunk.params = vec!["x".to_string()];
        chunk.local_names = vec!["x".to_string()];
        chunk.local_count = 1;

        let one_idx = chunk.add_constant(Constant::Int(1));
        chunk.emit(OpCode::LoadVar("x".to_string()));
        chunk.emit(OpCode::LoadConst(one_idx));
        chunk.emit(OpCode::Add);
        chunk.emit(OpCode::Return);

        Value::BytecodeFunction {
            chunk,
            captured: HashMap::<String, Arc<Mutex<Value>>>::new(),
            captured_binding_kinds: HashMap::new(),
        }
    }

    #[test]
    fn test_parallel_map_async_read_file_preserves_order_with_limit() {
        let temp_dir = unique_temp_dir("kujo_parallel_map");
        fs::create_dir_all(&temp_dir).unwrap();

        let file_a = format!("{}/a.txt", temp_dir);
        let file_b = format!("{}/b.txt", temp_dir);
        let file_c = format!("{}/c.txt", temp_dir);

        fs::write(&file_a, "alpha").unwrap();
        fs::write(&file_b, "beta").unwrap();
        fs::write(&file_c, "gamma").unwrap();

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![
                string_value(&file_a),
                string_value(&file_b),
                string_value(&file_c),
            ])),
            Value::NativeFunction("async_read_file".to_string()),
            Value::Int(2),
        ];

        let result = handle(&mut interp, "parallel_map", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Array(values) => {
                assert_eq!(values.len(), 3);
                match &values[0] {
                    Value::Str(s) => assert_eq!(s.as_str(), "alpha"),
                    _ => panic!("Expected string at index 0"),
                }
                match &values[1] {
                    Value::Str(s) => assert_eq!(s.as_str(), "beta"),
                    _ => panic!("Expected string at index 1"),
                }
                match &values[2] {
                    Value::Str(s) => assert_eq!(s.as_str(), "gamma"),
                    _ => panic!("Expected string at index 2"),
                }
            }
            _ => panic!("Expected Array result"),
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parallel_map_handles_non_promise_results() {
        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![
                string_value("a"),
                string_value("bc"),
                string_value("def"),
            ])),
            Value::NativeFunction("len".to_string()),
        ];

        let result = handle(&mut interp, "parallel_map", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Array(values) => {
                assert_eq!(values.len(), 3);
                match &values[0] {
                    Value::Int(n) => assert_eq!(*n, 1),
                    _ => panic!("Expected Int at index 0"),
                }
                match &values[1] {
                    Value::Int(n) => assert_eq!(*n, 2),
                    _ => panic!("Expected Int at index 1"),
                }
                match &values[2] {
                    Value::Int(n) => assert_eq!(*n, 3),
                    _ => panic!("Expected Int at index 2"),
                }
            }
            _ => panic!("Expected Array result"),
        }
    }

    #[test]
    fn test_parallel_map_rejects_non_callable_mapper() {
        let mut interp = Interpreter::new();
        let args = vec![Value::Array(Arc::new(vec![Value::Int(1)])), Value::Int(123)];

        let result = handle(&mut interp, "parallel_map", &args).unwrap();
        match result {
            Value::Error(msg) => {
                assert!(msg.contains("second argument must be a callable function"));
            }
            _ => panic!("Expected Value::Error for non-callable mapper"),
        }
    }

    #[test]
    fn test_parallel_map_validates_concurrency_limit() {
        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![Value::Int(1)])),
            Value::NativeFunction("len".to_string()),
            Value::Int(0),
        ];

        let result = handle(&mut interp, "parallel_map", &args).unwrap();
        match result {
            Value::Error(msg) => {
                assert!(msg.contains("concurrency_limit must be > 0"));
            }
            _ => panic!("Expected Value::Error for invalid concurrency limit"),
        }
    }

    #[test]
    fn test_parallel_map_propagates_rejected_promises() {
        let missing_file = unique_temp_dir("kujo_parallel_map_missing");
        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![string_value(&missing_file)])),
            Value::NativeFunction("async_read_file".to_string()),
        ];

        let result = handle(&mut interp, "parallel_map", &args).unwrap();
        let resolved = await_promise(result);
        match resolved {
            Ok(_) => panic!("Expected Promise rejection for missing file"),
            Err(msg) => assert!(msg.contains("Promise 0 rejected")),
        }
    }

    #[test]
    fn test_par_map_alias_matches_parallel_map_behavior() {
        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![
                string_value("x"),
                string_value("yz"),
                string_value("wxyz"),
            ])),
            Value::NativeFunction("len".to_string()),
            Value::Int(2),
        ];

        let result = handle(&mut interp, "par_map", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Array(values) => {
                assert_eq!(values.len(), 3);
                match &values[0] {
                    Value::Int(n) => assert_eq!(*n, 1),
                    _ => panic!("Expected Int at index 0"),
                }
                match &values[1] {
                    Value::Int(n) => assert_eq!(*n, 2),
                    _ => panic!("Expected Int at index 1"),
                }
                match &values[2] {
                    Value::Int(n) => assert_eq!(*n, 4),
                    _ => panic!("Expected Int at index 2"),
                }
            }
            _ => panic!("Expected Array result"),
        }
    }

    #[test]
    fn test_par_each_resolves_to_null_on_success() {
        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![
                string_value("hello"),
                string_value("kujo"),
                string_value("world"),
            ])),
            Value::NativeFunction("len".to_string()),
            Value::Int(2),
        ];

        let result = handle(&mut interp, "par_each", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        assert!(matches!(resolved, Value::Null));
    }

    #[test]
    fn test_par_each_propagates_rejected_promise() {
        let missing_file = unique_temp_dir("kujo_par_each_missing");
        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![string_value(&missing_file)])),
            Value::NativeFunction("async_read_file".to_string()),
        ];

        let result = handle(&mut interp, "par_each", &args).unwrap();
        let resolved = await_promise(result);

        match resolved {
            Ok(_) => panic!("Expected Promise rejection for missing file"),
            Err(msg) => assert!(msg.contains("Promise 0 rejected")),
        }
    }

    #[test]
    fn test_par_each_rejects_non_array_input() {
        let mut interp = Interpreter::new();
        let args = vec![Value::Int(7), Value::NativeFunction("len".to_string()), Value::Int(2)];

        let result = handle(&mut interp, "par_each", &args).unwrap();
        match result {
            Value::Error(msg) => {
                assert!(msg.contains("first argument must be an array"));
            }
            _ => panic!("Expected Value::Error for non-array input"),
        }
    }

    #[test]
    fn test_par_each_rejects_non_callable_mapper() {
        let mut interp = Interpreter::new();
        let args = vec![Value::Array(Arc::new(vec![Value::Int(1)])), Value::Int(123)];

        let result = handle(&mut interp, "par_each", &args).unwrap();
        match result {
            Value::Error(msg) => {
                assert!(msg.contains("second argument must be a callable function"));
            }
            _ => panic!("Expected Value::Error for non-callable mapper"),
        }
    }

    #[test]
    fn test_par_each_validates_concurrency_limit() {
        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![Value::Int(1)])),
            Value::NativeFunction("len".to_string()),
            Value::Int(0),
        ];

        let result = handle(&mut interp, "par_each", &args).unwrap();
        match result {
            Value::Error(msg) => {
                assert!(msg.contains("concurrency_limit must be > 0"));
            }
            _ => panic!("Expected Value::Error for invalid concurrency limit"),
        }
    }

    #[test]
    fn test_par_each_alias_error_shape_matches_parallel_map_for_validation() {
        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![Value::Int(1)])),
            Value::NativeFunction("len".to_string()),
            Value::Int(0),
        ];

        let parallel_map_result = handle(&mut interp, "parallel_map", &args).unwrap();
        let par_each_result = handle(&mut interp, "par_each", &args).unwrap();

        let parallel_message = match parallel_map_result {
            Value::Error(msg) => msg,
            other => panic!("Expected Value::Error from parallel_map validation, got {:?}", other),
        };

        let par_each_message = match par_each_result {
            Value::Error(msg) => msg,
            other => panic!("Expected Value::Error from par_each validation, got {:?}", other),
        };

        assert_eq!(par_each_message, parallel_message);
    }

    #[test]
    fn test_async_read_files_reads_multiple_files_in_order() {
        let temp_dir = unique_temp_dir("kujo_async_read_files");
        fs::create_dir_all(&temp_dir).unwrap();

        let file_a = format!("{}/a.txt", temp_dir);
        let file_b = format!("{}/b.txt", temp_dir);
        let file_c = format!("{}/c.txt", temp_dir);

        fs::write(&file_a, "alpha").unwrap();
        fs::write(&file_b, "beta").unwrap();
        fs::write(&file_c, "gamma").unwrap();

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![
                string_value(&file_a),
                string_value(&file_b),
                string_value(&file_c),
            ])),
            Value::Int(2),
        ];

        let result = handle(&mut interp, "async_read_files", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Array(values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(&values[0], Value::Str(s) if s.as_str() == "alpha"));
                assert!(matches!(&values[1], Value::Str(s) if s.as_str() == "beta"));
                assert!(matches!(&values[2], Value::Str(s) if s.as_str() == "gamma"));
            }
            _ => panic!("Expected Array result"),
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_async_read_files_rejects_invalid_path_element() {
        let mut interp = Interpreter::new();
        let args = vec![Value::Array(Arc::new(vec![Value::Int(1)]))];

        let result = handle(&mut interp, "async_read_files", &args).unwrap();
        match result {
            Value::Error(msg) => assert!(msg.contains("path at index 0 must be a string")),
            _ => panic!("Expected Value::Error for invalid path element"),
        }
    }

    #[test]
    fn test_async_read_files_propagates_missing_file_error() {
        let missing_path = unique_temp_dir("kujo_async_read_files_missing");
        let mut interp = Interpreter::new();
        let args = vec![Value::Array(Arc::new(vec![string_value(&missing_path)])), Value::Int(4)];

        let result = handle(&mut interp, "async_read_files", &args).unwrap();
        let resolved = await_promise(result);
        match resolved {
            Ok(_) => panic!("Expected async_read_files promise rejection for missing file"),
            Err(msg) => assert!(msg.contains("Failed to read file")),
        }
    }

    #[test]
    fn test_async_write_files_writes_all_files_and_returns_bools() {
        let temp_dir = unique_temp_dir("kujo_async_write_files");
        fs::create_dir_all(&temp_dir).unwrap();

        let file_a = format!("{}/a.txt", temp_dir);
        let file_b = format!("{}/b.txt", temp_dir);

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![string_value(&file_a), string_value(&file_b)])),
            Value::Array(Arc::new(vec![string_value("first"), string_value("second")])),
            Value::Int(2),
        ];

        let result = handle(&mut interp, "async_write_files", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Array(values) => {
                assert_eq!(values.len(), 2);
                assert!(matches!(&values[0], Value::Bool(true)));
                assert!(matches!(&values[1], Value::Bool(true)));
            }
            _ => panic!("Expected Array result"),
        }

        assert_eq!(fs::read_to_string(&file_a).unwrap(), "first");
        assert_eq!(fs::read_to_string(&file_b).unwrap(), "second");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_async_write_files_validates_lengths_and_types() {
        let mut interp = Interpreter::new();

        let mismatched = handle(
            &mut interp,
            "async_write_files",
            &[
                Value::Array(Arc::new(vec![string_value("a.txt")])),
                Value::Array(Arc::new(vec![string_value("first"), string_value("second")])),
            ],
        )
        .unwrap();
        match mismatched {
            Value::Error(msg) => assert!(msg.contains("must have the same length")),
            _ => panic!("Expected Value::Error for mismatched lengths"),
        }

        let invalid_content = handle(
            &mut interp,
            "async_write_files",
            &[
                Value::Array(Arc::new(vec![string_value("a.txt")])),
                Value::Array(Arc::new(vec![Value::Int(123)])),
            ],
        )
        .unwrap();
        match invalid_content {
            Value::Error(msg) => assert!(msg.contains("content at index 0 must be a string")),
            _ => panic!("Expected Value::Error for invalid content type"),
        }
    }

    #[test]
    fn test_ssg_render_and_write_pages_writes_html_and_returns_summary() {
        let temp_dir = unique_temp_dir("kujo_ssg_render_and_write_pages");
        fs::create_dir_all(&temp_dir).unwrap();

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![
                string_value("# Post 0\n\nGenerated page 0"),
                string_value("# Post 1\n\nGenerated page 1"),
            ])),
            string_value(&temp_dir),
            Value::Int(2),
        ];

        let result = handle(&mut interp, "ssg_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Dict(dict) => {
                assert!(matches!(dict.get("files"), Some(Value::Int(count)) if *count == 2));
                assert!(
                    matches!(dict.get("checksum"), Some(Value::Int(checksum)) if *checksum > 0)
                );
            }
            _ => panic!("Expected Dict result"),
        }

        let html_a = fs::read_to_string(format!("{}/post_0.html", temp_dir)).unwrap();
        let html_b = fs::read_to_string(format!("{}/post_1.html", temp_dir)).unwrap();

        assert_eq!(
            html_a,
            "<html><body><h1>Post 0</h1><article># Post 0\n\nGenerated page 0</article></body></html>"
        );
        assert_eq!(
            html_b,
            "<html><body><h1>Post 1</h1><article># Post 1\n\nGenerated page 1</article></body></html>"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ssg_render_and_write_pages_checksum_matches_rendered_outputs() {
        let temp_dir = unique_temp_dir("kujo_ssg_render_and_write_pages_checksum");
        fs::create_dir_all(&temp_dir).unwrap();

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![
                string_value("# A"),
                string_value("# B\n\nBody"),
                string_value("# C\n\nLonger body content"),
            ])),
            string_value(&temp_dir),
            Value::Int(5),
        ];

        let result = handle(&mut interp, "ssg_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        let checksum = match resolved {
            Value::Dict(dict) => {
                assert!(matches!(dict.get("files"), Some(Value::Int(count)) if *count == 3));
                match dict.get("checksum") {
                    Some(Value::Int(value)) => *value,
                    _ => panic!("Expected checksum int"),
                }
            }
            _ => panic!("Expected Dict result"),
        };

        let html_a = fs::read_to_string(format!("{}/post_0.html", temp_dir)).unwrap();
        let html_b = fs::read_to_string(format!("{}/post_1.html", temp_dir)).unwrap();
        let html_c = fs::read_to_string(format!("{}/post_2.html", temp_dir)).unwrap();
        let expected_checksum = (html_a.len() + html_b.len() + html_c.len()) as i64;

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ssg_render_and_write_pages_unicode_checksum_matches_written_outputs() {
        let temp_dir = unique_temp_dir("kujo_ssg_render_and_write_pages_unicode_checksum");
        fs::create_dir_all(&temp_dir).unwrap();

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![
                string_value("# Café ☕\\n\\nnaïve façade"),
                string_value("# Emoji 🚀\\n\\nUnicode ✅✨"),
            ])),
            string_value(&temp_dir),
            Value::Int(2),
        ];

        let result = handle(&mut interp, "ssg_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        let checksum = match resolved {
            Value::Dict(dict) => {
                assert!(matches!(dict.get("files"), Some(Value::Int(count)) if *count == 2));
                match dict.get("checksum") {
                    Some(Value::Int(value)) => *value,
                    _ => panic!("Expected checksum int"),
                }
            }
            _ => panic!("Expected Dict result"),
        };

        let html_a = fs::read_to_string(format!("{}/post_0.html", temp_dir)).unwrap();
        let html_b = fs::read_to_string(format!("{}/post_1.html", temp_dir)).unwrap();
        let expected_checksum = (html_a.len() + html_b.len()) as i64;

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ssg_build_output_paths_for_batch_generates_stable_indexed_paths() {
        let paths = ssg_build_output_paths_for_batch("tmp/output", 4);
        assert_eq!(paths.len(), 4);
        assert_eq!(paths[0], "tmp/output/post_0.html");
        assert_eq!(paths[1], "tmp/output/post_1.html");
        assert_eq!(paths[2], "tmp/output/post_2.html");
        assert_eq!(paths[3], "tmp/output/post_3.html");

        let empty_paths = ssg_build_output_paths_for_batch("tmp/output", 0);
        assert!(empty_paths.is_empty());
    }

    #[test]
    fn test_ssg_build_render_prefixes_for_batch_generates_stable_prefixes() {
        let prefixes = ssg_build_render_prefixes_for_batch(3);
        assert_eq!(prefixes.len(), 3);
        assert_eq!(prefixes[0], "<html><body><h1>Post 0</h1><article>");
        assert_eq!(prefixes[1], "<html><body><h1>Post 1</h1><article>");
        assert_eq!(prefixes[2], "<html><body><h1>Post 2</h1><article>");

        let empty_prefixes = ssg_build_render_prefixes_for_batch(0);
        assert!(empty_prefixes.is_empty());
    }

    #[test]
    fn test_ssg_build_output_paths_and_prefixes_for_batch_generates_parallel_outputs() {
        let (paths, prefixes) = ssg_build_output_paths_and_prefixes_for_batch("tmp/output", 3);

        assert_eq!(paths.len(), 3);
        assert_eq!(prefixes.len(), 3);

        assert_eq!(paths[0], "tmp/output/post_0.html");
        assert_eq!(paths[1], "tmp/output/post_1.html");
        assert_eq!(paths[2], "tmp/output/post_2.html");

        assert_eq!(prefixes[0].as_ref(), "<html><body><h1>Post 0</h1><article>");
        assert_eq!(prefixes[1].as_ref(), "<html><body><h1>Post 1</h1><article>");
        assert_eq!(prefixes[2].as_ref(), "<html><body><h1>Post 2</h1><article>");
    }

    #[test]
    fn test_ssg_build_output_paths_and_prefixes_for_batch_handles_empty_input() {
        let (paths, prefixes) = ssg_build_output_paths_and_prefixes_for_batch("tmp/output", 0);

        assert!(paths.is_empty());
        assert!(prefixes.is_empty());
    }

    #[test]
    fn test_ssg_build_output_paths_and_prefixes_for_batch_reuses_cached_prefixes_for_same_count() {
        let (_paths_a, prefixes_a) =
            ssg_build_output_paths_and_prefixes_for_batch("tmp/output-a", 4);
        let (_paths_b, prefixes_b) =
            ssg_build_output_paths_and_prefixes_for_batch("tmp/output-b", 4);

        assert!(
            Arc::ptr_eq(&prefixes_a, &prefixes_b),
            "expected cached render prefixes to be reused for identical file_count"
        );
    }

    #[test]
    fn test_ssg_get_or_build_output_file_suffixes_reuses_cached_arc_for_same_count() {
        let suffixes_a = ssg_get_or_build_output_file_suffixes(3);
        let suffixes_b = ssg_get_or_build_output_file_suffixes(3);
        let suffixes_c = ssg_get_or_build_output_file_suffixes(2);

        assert!(
            Arc::ptr_eq(&suffixes_a, &suffixes_b),
            "expected cached output suffixes to be reused for identical file_count"
        );
        assert!(
            !Arc::ptr_eq(&suffixes_a, &suffixes_c),
            "expected different output suffix cache entries for different file counts"
        );
        assert_eq!(suffixes_a[0].as_ref(), "post_0.html");
        assert_eq!(suffixes_a[1].as_ref(), "post_1.html");
        assert_eq!(suffixes_a[2].as_ref(), "post_2.html");
    }

    #[test]
    fn test_ssg_write_rendered_html_page_streams_exact_content_and_length() {
        let output_dir = unique_temp_dir("kujo_ssg_streamed_html_output");
        fs::create_dir_all(&output_dir).unwrap();
        let output_path = format!("{}/post_42.html", output_dir);
        let source_body = "# Post 42\n\nLarge body section";
        let html_prefix = "<html><body><h1>Post 42</h1><article>";

        let html_len = AsyncRuntime::block_on(async {
            ssg_write_rendered_html_page(output_path.as_str(), html_prefix, source_body)
                .await
                .unwrap()
        });

        let mut html = String::new();
        for _attempt in 0..20 {
            html = fs::read_to_string(output_path.as_str()).unwrap();
            if html.len() == html_len {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }

        let expected_html =
            "<html><body><h1>Post 42</h1><article># Post 42\n\nLarge body section</article></body></html>";

        assert_eq!(html, expected_html);
        assert_eq!(html_len, expected_html.len());

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_write_rendered_html_page_escapes_source_html() {
        let output_dir = unique_temp_dir("kujo_ssg_streamed_html_escape");
        fs::create_dir_all(&output_dir).unwrap();
        let output_path = format!("{}/post_0.html", output_dir);
        let html_prefix = "<html><body><h1>Post 0</h1><article>";

        AsyncRuntime::block_on(async {
            ssg_write_rendered_html_page(
                output_path.as_str(),
                html_prefix,
                "<script>alert(1)</script>",
            )
            .await
            .unwrap()
        });

        let html = fs::read_to_string(output_path.as_str()).unwrap();
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>"));

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_write_rendered_html_page_handles_empty_source_body() {
        let output_dir = unique_temp_dir("kujo_ssg_streamed_html_empty_body");
        fs::create_dir_all(&output_dir).unwrap();
        let output_path = format!("{}/post_0.html", output_dir);
        let html_prefix = "<html><body><h1>Post 0</h1><article>";

        let html_len = AsyncRuntime::block_on(async {
            ssg_write_rendered_html_page(output_path.as_str(), html_prefix, "").await.unwrap()
        });

        let html = fs::read_to_string(output_path.as_str()).unwrap();
        let expected_html = "<html><body><h1>Post 0</h1><article></article></body></html>";

        assert_eq!(html, expected_html);
        assert_eq!(html_len, expected_html.len());

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_write_rendered_html_page_handles_large_source_body_without_truncation() {
        let output_dir = unique_temp_dir("kujo_ssg_streamed_html_large_body");
        fs::create_dir_all(&output_dir).unwrap();
        let output_path = format!("{}/post_9.html", output_dir);
        let source_body = "segment-".repeat(16_384);
        let html_prefix = "<html><body><h1>Post 9</h1><article>";

        let html_len = AsyncRuntime::block_on(async {
            ssg_write_rendered_html_page(output_path.as_str(), html_prefix, source_body.as_str())
                .await
                .unwrap()
        });

        let html = fs::read_to_string(output_path.as_str()).unwrap();
        let expected_len = html_prefix.len() + source_body.len() + SSG_HTML_SUFFIX.len();

        assert_eq!(html_len, expected_len);
        assert_eq!(html.len(), expected_len);
        assert!(html.starts_with(html_prefix));
        assert!(html.ends_with(SSG_HTML_SUFFIX));
        assert!(html.contains(source_body.as_str()));

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_write_rendered_html_page_propagates_create_failure() {
        let missing_dir = unique_temp_dir("kujo_ssg_streamed_html_missing");
        let output_path = format!("{}/post_0.html", missing_dir);

        let result = AsyncRuntime::block_on(async {
            ssg_write_rendered_html_page(
                output_path.as_str(),
                "<html><body><h1>Post 0</h1><article>",
                "# Missing",
            )
            .await
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_ssg_read_ahead_limit_expands_up_to_double_concurrency() {
        assert_eq!(ssg_read_ahead_limit(1, 100), 2);
        assert_eq!(ssg_read_ahead_limit(2, 100), 4);
        assert_eq!(ssg_read_ahead_limit(8, 100), 16);
    }

    #[test]
    fn test_ssg_read_ahead_limit_is_capped_by_file_count() {
        assert_eq!(ssg_read_ahead_limit(4, 1), 1);
        assert_eq!(ssg_read_ahead_limit(4, 3), 3);
        assert_eq!(ssg_read_ahead_limit(4, 8), 8);
    }

    #[test]
    fn test_ssg_read_ahead_limit_handles_zero_inputs_defensively() {
        assert_eq!(ssg_read_ahead_limit(0, 0), 1);
        assert_eq!(ssg_read_ahead_limit(0, 5), 2);
    }

    #[test]
    fn test_ssg_write_rendered_html_page_returns_total_written_bytes() {
        let output_dir = unique_temp_dir("kujo_ssg_streamed_html_written_bytes_ascii");
        fs::create_dir_all(&output_dir).unwrap();
        let output_path = format!("{}/post_11.html", output_dir);
        let source_body = "# Post\n\nPlain text payload";
        let html_prefix = "<html><body><h1>Post 11</h1><article>";

        let written_bytes = AsyncRuntime::block_on(async {
            ssg_write_rendered_html_page(output_path.as_str(), html_prefix, source_body)
                .await
                .unwrap()
        });

        let rendered_html = fs::read_to_string(output_path.as_str()).unwrap();
        assert_eq!(written_bytes, rendered_html.len());
        assert_eq!(written_bytes, html_prefix.len() + source_body.len() + SSG_HTML_SUFFIX.len());

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_write_rendered_html_page_returns_utf8_written_bytes() {
        let output_dir = unique_temp_dir("kujo_ssg_streamed_html_written_bytes_utf8");
        fs::create_dir_all(&output_dir).unwrap();
        let output_path = format!("{}/post_12.html", output_dir);
        let source_body = "# Café 🚀\n\nnaïve façade";
        let html_prefix = "<html><body><h1>Post 12</h1><article>";

        let written_bytes = AsyncRuntime::block_on(async {
            ssg_write_rendered_html_page(output_path.as_str(), html_prefix, source_body)
                .await
                .unwrap()
        });

        let rendered_bytes = fs::read(output_path.as_str()).unwrap();
        assert_eq!(written_bytes, rendered_bytes.len());
        assert_eq!(
            written_bytes,
            html_prefix.as_bytes().len()
                + source_body.as_bytes().len()
                + SSG_HTML_SUFFIX.as_bytes().len()
        );

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_write_rendered_html_page_sync_streams_exact_content_and_length() {
        let output_dir = unique_temp_dir("kujo_ssg_sync_vectored_output");
        fs::create_dir_all(&output_dir).unwrap();
        let output_path = format!("{}/post_42.html", output_dir);
        let source_body = "# Post 42\n\nLarge body section";
        let html_prefix = "<html><body><h1>Post 42</h1><article>";

        let written_bytes =
            ssg_write_rendered_html_page_sync(output_path.as_str(), html_prefix, source_body)
                .unwrap();

        let html = fs::read_to_string(output_path.as_str()).unwrap();
        let expected_html =
            "<html><body><h1>Post 42</h1><article># Post 42\n\nLarge body section</article></body></html>";

        assert_eq!(html, expected_html);
        assert_eq!(written_bytes, expected_html.len());

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_write_rendered_html_page_sync_handles_empty_source_body() {
        let output_dir = unique_temp_dir("kujo_ssg_sync_vectored_empty_body");
        fs::create_dir_all(&output_dir).unwrap();
        let output_path = format!("{}/post_0.html", output_dir);
        let html_prefix = "<html><body><h1>Post 0</h1><article>";

        let written_bytes =
            ssg_write_rendered_html_page_sync(output_path.as_str(), html_prefix, "").unwrap();

        let html = fs::read_to_string(output_path.as_str()).unwrap();
        let expected_html = "<html><body><h1>Post 0</h1><article></article></body></html>";

        assert_eq!(html, expected_html);
        assert_eq!(written_bytes, expected_html.len());

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_write_rendered_html_page_sync_handles_large_source_body_without_truncation() {
        let output_dir = unique_temp_dir("kujo_ssg_sync_vectored_large_body");
        fs::create_dir_all(&output_dir).unwrap();
        let output_path = format!("{}/post_9.html", output_dir);
        let source_body = "segment-".repeat(16_384);
        let html_prefix = "<html><body><h1>Post 9</h1><article>";

        let written_bytes = ssg_write_rendered_html_page_sync(
            output_path.as_str(),
            html_prefix,
            source_body.as_str(),
        )
        .unwrap();

        let html = fs::read_to_string(output_path.as_str()).unwrap();
        let expected_len = html_prefix.len() + source_body.len() + SSG_HTML_SUFFIX.len();

        assert_eq!(written_bytes, expected_len);
        assert_eq!(html.len(), expected_len);
        assert!(html.starts_with(html_prefix));
        assert!(html.ends_with(SSG_HTML_SUFFIX));
        assert!(html.contains(source_body.as_str()));

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_write_rendered_html_page_sync_propagates_create_failure() {
        let missing_dir = unique_temp_dir("kujo_ssg_sync_vectored_missing");
        let output_path = format!("{}/post_0.html", missing_dir);

        let result = ssg_write_rendered_html_page_sync(
            output_path.as_str(),
            "<html><body><h1>Post 0</h1><article>",
            "# Missing",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_ssg_write_rendered_html_page_sync_returns_utf8_written_bytes() {
        let output_dir = unique_temp_dir("kujo_ssg_sync_vectored_written_bytes_utf8");
        fs::create_dir_all(&output_dir).unwrap();
        let output_path = format!("{}/post_12.html", output_dir);
        let source_body = "# Cafe 🚀\n\nnaive facade";
        let html_prefix = "<html><body><h1>Post 12</h1><article>";

        let written_bytes =
            ssg_write_rendered_html_page_sync(output_path.as_str(), html_prefix, source_body)
                .unwrap();

        let rendered_bytes = fs::read(output_path.as_str()).unwrap();
        assert_eq!(written_bytes, rendered_bytes.len());
        assert_eq!(
            written_bytes,
            html_prefix.as_bytes().len()
                + source_body.as_bytes().len()
                + SSG_HTML_SUFFIX.as_bytes().len()
        );

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_write_rendered_html_page_sync_bytes_supports_non_utf8_source_body() {
        let output_dir = unique_temp_dir("kujo_ssg_sync_vectored_non_utf8_bytes");
        fs::create_dir_all(&output_dir).unwrap();
        let output_path = format!("{}/post_14.html", output_dir);
        let html_prefix = b"<html><body><h1>Post 14</h1><article>";
        let source_body: [u8; 6] = [0x66, 0x6f, 0x80, 0x81, 0x82, 0x6f];

        let written_bytes = ssg_write_rendered_html_page_sync_bytes(
            output_path.as_str(),
            html_prefix,
            &source_body,
        )
        .unwrap();

        let rendered_bytes = fs::read(output_path.as_str()).unwrap();
        let expected_len = html_prefix.len() + source_body.len() + SSG_HTML_SUFFIX.as_bytes().len();

        assert_eq!(written_bytes, expected_len);
        assert_eq!(rendered_bytes.len(), expected_len);
        assert!(rendered_bytes.starts_with(html_prefix));
        assert!(rendered_bytes.ends_with(SSG_HTML_SUFFIX.as_bytes()));
        assert!(rendered_bytes.windows(source_body.len()).any(|window| window == source_body));

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_target_read_in_flight_allows_full_window_with_empty_write_backlog() {
        assert_eq!(ssg_target_read_in_flight(8, 4, 0, 0), 8);
        assert_eq!(ssg_target_read_in_flight(2, 2, 0, 0), 2);
    }

    #[test]
    fn test_ssg_target_read_in_flight_scales_down_as_write_backlog_grows() {
        assert_eq!(ssg_target_read_in_flight(8, 4, 1, 1), 6);
        assert_eq!(ssg_target_read_in_flight(8, 4, 4, 1), 3);
        assert_eq!(ssg_target_read_in_flight(8, 4, 5, 2), 1);
    }

    #[test]
    fn test_ssg_target_read_in_flight_handles_zero_limits_defensively() {
        assert_eq!(ssg_target_read_in_flight(0, 0, 0, 0), 1);
        assert_eq!(ssg_target_read_in_flight(0, 3, 6, 6), 1);
    }

    #[test]
    fn test_ssg_should_refill_writes_first_with_pending_backlog_and_available_slot() {
        assert!(ssg_should_refill_writes_first(1, 0, 2));
        assert!(ssg_should_refill_writes_first(3, 1, 2));
    }

    #[test]
    fn test_ssg_should_refill_writes_first_false_without_backlog() {
        assert!(!ssg_should_refill_writes_first(0, 0, 2));
        assert!(!ssg_should_refill_writes_first(0, 1, 2));
    }

    #[test]
    fn test_ssg_should_refill_writes_first_false_when_write_lane_is_saturated() {
        assert!(!ssg_should_refill_writes_first(1, 2, 2));
        assert!(!ssg_should_refill_writes_first(4, 3, 2));
    }

    #[test]
    fn test_ssg_should_refill_writes_first_handles_zero_limit_defensively() {
        assert!(ssg_should_refill_writes_first(1, 0, 0));
        assert!(!ssg_should_refill_writes_first(1, 1, 0));
    }

    #[test]
    fn test_ssg_should_prefetch_single_worker_read_requires_remaining_without_pending_write() {
        assert!(ssg_should_prefetch_single_worker_read(3, 0, false));
        assert!(!ssg_should_prefetch_single_worker_read(0, 0, false));
        assert!(!ssg_should_prefetch_single_worker_read(2, 1, false));
        assert!(!ssg_should_prefetch_single_worker_read(2, 0, true));
    }

    #[test]
    fn test_ssg_render_and_write_pages_large_batch_low_concurrency_preserves_outputs() {
        let temp_dir = unique_temp_dir("kujo_ssg_render_and_write_pages_large_low_concurrency");
        fs::create_dir_all(&temp_dir).unwrap();

        let file_count = 96usize;
        let mut source_pages = Vec::with_capacity(file_count);
        for index in 0..file_count {
            source_pages.push(string_value(
                format!("# Entry {}\n\n{}", index, "body ".repeat((index % 5) + 1)).as_str(),
            ));
        }

        let mut interp = Interpreter::new();
        let args =
            vec![Value::Array(Arc::new(source_pages)), string_value(&temp_dir), Value::Int(2)];

        let result = handle(&mut interp, "ssg_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        let checksum = match resolved {
            Value::Dict(dict) => {
                assert!(
                    matches!(dict.get("files"), Some(Value::Int(count)) if *count == file_count as i64)
                );
                match dict.get("checksum") {
                    Some(Value::Int(value)) => *value,
                    _ => panic!("Expected checksum int"),
                }
            }
            _ => panic!("Expected Dict result"),
        };

        let mut expected_checksum = 0i64;
        for index in 0..file_count {
            let html = fs::read_to_string(format!("{}/post_{}.html", temp_dir, index)).unwrap();
            expected_checksum += html.len() as i64;
        }

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ssg_render_and_write_pages_high_concurrency_preserves_outputs() {
        let temp_dir =
            unique_temp_dir("kujo_ssg_render_and_write_pages_high_concurrency_preserves_outputs");
        fs::create_dir_all(&temp_dir).unwrap();

        let file_count = 48usize;
        let mut source_pages = Vec::with_capacity(file_count);
        for index in 0..file_count {
            let body = format!("# High {}\n\n{}", index, "payload ".repeat((index % 5) + 1));
            source_pages.push(string_value(body.as_str()));
        }

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(source_pages)),
            string_value(&temp_dir),
            Value::Int((file_count + 10) as i64),
        ];

        let result = handle(&mut interp, "ssg_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        let checksum = match resolved {
            Value::Dict(dict) => {
                assert!(
                    matches!(dict.get("files"), Some(Value::Int(count)) if *count == file_count as i64)
                );
                match dict.get("checksum") {
                    Some(Value::Int(value)) => *value,
                    _ => panic!("Expected checksum int"),
                }
            }
            _ => panic!("Expected Dict result"),
        };

        let mut expected_checksum = 0i64;
        for index in 0..file_count {
            let html = fs::read_to_string(format!("{}/post_{}.html", temp_dir, index)).unwrap();
            expected_checksum += html.len() as i64;
        }

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ssg_render_and_write_pages_single_worker_preserves_output_contracts() {
        let temp_dir = unique_temp_dir("kujo_ssg_render_and_write_pages_single_worker_output");
        fs::create_dir_all(&temp_dir).unwrap();

        let file_count = 40usize;
        let mut source_pages = Vec::with_capacity(file_count);
        for index in 0..file_count {
            let body = format!("# Single {}\n\n{}", index, "content ".repeat((index % 6) + 1));
            source_pages.push(string_value(body.as_str()));
        }

        let mut interp = Interpreter::new();
        let args =
            vec![Value::Array(Arc::new(source_pages)), string_value(&temp_dir), Value::Int(1)];

        let result = handle(&mut interp, "ssg_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        let checksum = match resolved {
            Value::Dict(dict) => {
                assert!(
                    matches!(dict.get("files"), Some(Value::Int(count)) if *count == file_count as i64)
                );
                match dict.get("checksum") {
                    Some(Value::Int(value)) => *value,
                    _ => panic!("Expected checksum int"),
                }
            }
            _ => panic!("Expected Dict result"),
        };

        let mut expected_checksum = 0i64;
        for index in 0..file_count {
            let html = fs::read_to_string(format!("{}/post_{}.html", temp_dir, index)).unwrap();
            expected_checksum += html.len() as i64;
        }

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ssg_render_and_write_pages_preserves_large_index_heading_contract() {
        let temp_dir = unique_temp_dir("kujo_ssg_render_and_write_pages_large_index_heading");
        fs::create_dir_all(&temp_dir).unwrap();

        let file_count = 1005usize;
        let mut source_pages = Vec::with_capacity(file_count);
        for index in 0..file_count {
            source_pages.push(string_value(format!("# Entry {}", index).as_str()));
        }

        let mut interp = Interpreter::new();
        let args =
            vec![Value::Array(Arc::new(source_pages)), string_value(&temp_dir), Value::Int(8)];

        let result = handle(&mut interp, "ssg_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Dict(dict) => {
                assert!(
                    matches!(dict.get("files"), Some(Value::Int(count)) if *count == file_count as i64)
                );
                assert!(matches!(dict.get("checksum"), Some(Value::Int(sum)) if *sum > 0));
            }
            _ => panic!("Expected Dict result"),
        }

        let html = fs::read_to_string(format!("{}/post_1004.html", temp_dir)).unwrap();
        assert!(html.contains("<h1>Post 1004</h1>"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ssg_render_and_write_pages_empty_input_returns_zero_summary() {
        let temp_dir = unique_temp_dir("kujo_ssg_render_and_write_pages_empty");
        fs::create_dir_all(&temp_dir).unwrap();

        let mut interp = Interpreter::new();
        let args = vec![Value::Array(Arc::new(vec![])), string_value(&temp_dir)];

        let result = handle(&mut interp, "ssg_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Dict(dict) => {
                assert!(matches!(dict.get("files"), Some(Value::Int(count)) if *count == 0));
                assert!(
                    matches!(dict.get("checksum"), Some(Value::Int(checksum)) if *checksum == 0)
                );
            }
            _ => panic!("Expected Dict result"),
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ssg_render_and_write_pages_validates_argument_contracts() {
        let mut interp = Interpreter::new();

        let wrong_arity = handle(&mut interp, "ssg_render_and_write_pages", &[]).unwrap();
        assert!(
            matches!(wrong_arity, Value::Error(msg) if msg.contains("expects 2 or 3 arguments"))
        );

        let bad_first = handle(
            &mut interp,
            "ssg_render_and_write_pages",
            &[Value::Int(1), string_value("tmp/out")],
        )
        .unwrap();
        assert!(
            matches!(bad_first, Value::Error(msg) if msg.contains("first argument must be an array"))
        );

        let bad_second = handle(
            &mut interp,
            "ssg_render_and_write_pages",
            &[Value::Array(Arc::new(vec![])), Value::Int(1)],
        )
        .unwrap();
        assert!(
            matches!(bad_second, Value::Error(msg) if msg.contains("second argument must be a string output_dir"))
        );

        let bad_page_element = handle(
            &mut interp,
            "ssg_render_and_write_pages",
            &[Value::Array(Arc::new(vec![Value::Int(1)])), string_value("tmp/out")],
        )
        .unwrap();
        assert!(
            matches!(bad_page_element, Value::Error(msg) if msg.contains("source page at index 0 must be a string"))
        );

        let bad_limit = handle(
            &mut interp,
            "ssg_render_and_write_pages",
            &[
                Value::Array(Arc::new(vec![string_value("# Post")])),
                string_value("tmp/out"),
                Value::Int(0),
            ],
        )
        .unwrap();
        assert!(
            matches!(bad_limit, Value::Error(msg) if msg.contains("concurrency_limit must be > 0"))
        );
    }

    #[test]
    fn test_ssg_render_and_write_pages_propagates_write_failure() {
        let missing_dir = unique_temp_dir("kujo_ssg_render_and_write_pages_missing");

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![string_value("# Post 0")])),
            string_value(&missing_dir),
            Value::Int(1),
        ];

        let result = handle(&mut interp, "ssg_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result);

        match resolved {
            Ok(_) => panic!("Expected promise rejection for missing output dir"),
            Err(msg) => assert!(msg.contains("Failed to write file")),
        }
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_writes_html_and_returns_stage_metrics() {
        let input_dir = unique_temp_dir("kujo_ssg_read_render_input");
        let output_dir = unique_temp_dir("kujo_ssg_read_render_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source_a = format!("{}/post_0.md", input_dir);
        let source_b = format!("{}/post_1.md", input_dir);
        fs::write(&source_a, "# Post 0\n\nGenerated page 0").unwrap();
        fs::write(&source_b, "# Post 1\n\nGenerated page 1").unwrap();

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![string_value(&source_a), string_value(&source_b)])),
            string_value(&output_dir),
            Value::Int(2),
        ];

        let result = handle(&mut interp, "ssg_read_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Dict(dict) => {
                assert!(matches!(dict.get("files"), Some(Value::Int(count)) if *count == 2));
                assert!(
                    matches!(dict.get("checksum"), Some(Value::Int(checksum)) if *checksum > 0)
                );
                assert!(matches!(dict.get("read_ms"), Some(Value::Float(ms)) if *ms >= 0.0));
                assert!(
                    matches!(dict.get("render_write_ms"), Some(Value::Float(ms)) if *ms >= 0.0)
                );
            }
            _ => panic!("Expected Dict result"),
        }

        let html_a = fs::read_to_string(format!("{}/post_0.html", output_dir)).unwrap();
        let html_b = fs::read_to_string(format!("{}/post_1.html", output_dir)).unwrap();
        assert_eq!(
            html_a,
            "<html><body><h1>Post 0</h1><article># Post 0\n\nGenerated page 0</article></body></html>"
        );
        assert_eq!(
            html_b,
            "<html><body><h1>Post 1</h1><article># Post 1\n\nGenerated page 1</article></body></html>"
        );

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_checksum_matches_written_outputs() {
        let input_dir = unique_temp_dir("kujo_ssg_read_render_checksum_input");
        let output_dir = unique_temp_dir("kujo_ssg_read_render_checksum_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source_a = format!("{}/post_0.md", input_dir);
        let source_b = format!("{}/post_1.md", input_dir);
        let source_c = format!("{}/post_2.md", input_dir);
        fs::write(&source_a, "# A").unwrap();
        fs::write(&source_b, "# B\n\nBody").unwrap();
        fs::write(&source_c, "# C\n\nLonger body content").unwrap();

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![
                string_value(&source_a),
                string_value(&source_b),
                string_value(&source_c),
            ])),
            string_value(&output_dir),
            Value::Int(3),
        ];

        let result = handle(&mut interp, "ssg_read_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        let checksum = match resolved {
            Value::Dict(dict) => {
                assert!(matches!(dict.get("files"), Some(Value::Int(count)) if *count == 3));
                match dict.get("checksum") {
                    Some(Value::Int(value)) => *value,
                    _ => panic!("Expected checksum int"),
                }
            }
            _ => panic!("Expected Dict result"),
        };

        let html_a = fs::read_to_string(format!("{}/post_0.html", output_dir)).unwrap();
        let html_b = fs::read_to_string(format!("{}/post_1.html", output_dir)).unwrap();
        let html_c = fs::read_to_string(format!("{}/post_2.html", output_dir)).unwrap();
        let expected_checksum = (html_a.len() + html_b.len() + html_c.len()) as i64;

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_unicode_checksum_matches_written_outputs() {
        let input_dir = unique_temp_dir("kujo_ssg_read_render_unicode_checksum_input");
        let output_dir = unique_temp_dir("kujo_ssg_read_render_unicode_checksum_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source_a = format!("{}/post_0.md", input_dir);
        let source_b = format!("{}/post_1.md", input_dir);
        fs::write(&source_a, "# Café ☕\\n\\nnaïve façade").unwrap();
        fs::write(&source_b, "# Emoji 🚀\\n\\nUnicode ✅✨").unwrap();

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![string_value(&source_a), string_value(&source_b)])),
            string_value(&output_dir),
            Value::Int(2),
        ];

        let result = handle(&mut interp, "ssg_read_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        let checksum = match resolved {
            Value::Dict(dict) => {
                assert!(matches!(dict.get("files"), Some(Value::Int(count)) if *count == 2));
                assert!(matches!(dict.get("read_ms"), Some(Value::Float(ms)) if *ms >= 0.0));
                assert!(
                    matches!(dict.get("render_write_ms"), Some(Value::Float(ms)) if *ms >= 0.0)
                );
                match dict.get("checksum") {
                    Some(Value::Int(value)) => *value,
                    _ => panic!("Expected checksum int"),
                }
            }
            _ => panic!("Expected Dict result"),
        };

        let html_a = fs::read_to_string(format!("{}/post_0.html", output_dir)).unwrap();
        let html_b = fs::read_to_string(format!("{}/post_1.html", output_dir)).unwrap();
        let expected_checksum = (html_a.len() + html_b.len()) as i64;

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_empty_input_returns_zero_summary() {
        let output_dir = unique_temp_dir("kujo_ssg_read_render_empty_output");
        fs::create_dir_all(&output_dir).unwrap();

        let mut interp = Interpreter::new();
        let args = vec![Value::Array(Arc::new(vec![])), string_value(&output_dir)];

        let result = handle(&mut interp, "ssg_read_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Dict(dict) => {
                assert!(matches!(dict.get("files"), Some(Value::Int(count)) if *count == 0));
                assert!(
                    matches!(dict.get("checksum"), Some(Value::Int(checksum)) if *checksum == 0)
                );
                assert!(matches!(dict.get("read_ms"), Some(Value::Float(ms)) if *ms == 0.0));
                assert!(
                    matches!(dict.get("render_write_ms"), Some(Value::Float(ms)) if *ms == 0.0)
                );
            }
            _ => panic!("Expected Dict result"),
        }

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_single_worker_preserves_output_contracts() {
        let input_dir = unique_temp_dir("kujo_ssg_read_render_single_worker_input");
        let output_dir = unique_temp_dir("kujo_ssg_read_render_single_worker_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source_a = format!("{}/post_0.md", input_dir);
        let source_b = format!("{}/post_1.md", input_dir);
        let source_c = format!("{}/post_2.md", input_dir);
        fs::write(&source_a, "# One").unwrap();
        fs::write(&source_b, "# Two\n\nBody").unwrap();
        fs::write(&source_c, "# Three\n\nBody\n\nTail").unwrap();

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![
                string_value(&source_a),
                string_value(&source_b),
                string_value(&source_c),
            ])),
            string_value(&output_dir),
            Value::Int(1),
        ];

        let result = handle(&mut interp, "ssg_read_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        let checksum = match resolved {
            Value::Dict(dict) => {
                assert!(matches!(dict.get("files"), Some(Value::Int(count)) if *count == 3));
                assert!(matches!(dict.get("read_ms"), Some(Value::Float(ms)) if *ms >= 0.0));
                assert!(
                    matches!(dict.get("render_write_ms"), Some(Value::Float(ms)) if *ms >= 0.0)
                );
                match dict.get("checksum") {
                    Some(Value::Int(value)) => *value,
                    _ => panic!("Expected checksum int"),
                }
            }
            _ => panic!("Expected Dict result"),
        };

        let html_a = fs::read_to_string(format!("{}/post_0.html", output_dir)).unwrap();
        let html_b = fs::read_to_string(format!("{}/post_1.html", output_dir)).unwrap();
        let html_c = fs::read_to_string(format!("{}/post_2.html", output_dir)).unwrap();
        let expected_checksum = (html_a.len() + html_b.len() + html_c.len()) as i64;

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_large_batch_single_worker_completes() {
        let input_dir = unique_temp_dir("kujo_ssg_read_render_large_single_worker_input");
        let output_dir = unique_temp_dir("kujo_ssg_read_render_large_single_worker_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let file_count = 64usize;
        let mut source_paths = Vec::with_capacity(file_count);
        for index in 0..file_count {
            let source_path = format!("{}/post_{}.md", input_dir, index);
            let body = format!("# Post {}\n\nGenerated body {}", index, index * 3);
            fs::write(&source_path, body).unwrap();
            source_paths.push(string_value(source_path.as_str()));
        }

        let mut interp = Interpreter::new();
        let args =
            vec![Value::Array(Arc::new(source_paths)), string_value(&output_dir), Value::Int(1)];

        let result = handle(&mut interp, "ssg_read_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        let checksum = match resolved {
            Value::Dict(dict) => {
                assert!(
                    matches!(dict.get("files"), Some(Value::Int(count)) if *count == file_count as i64)
                );
                assert!(matches!(dict.get("read_ms"), Some(Value::Float(ms)) if *ms >= 0.0));
                assert!(
                    matches!(dict.get("render_write_ms"), Some(Value::Float(ms)) if *ms >= 0.0)
                );
                match dict.get("checksum") {
                    Some(Value::Int(value)) => *value,
                    _ => panic!("Expected checksum int"),
                }
            }
            _ => panic!("Expected Dict result"),
        };

        let mut expected_checksum = 0i64;
        for index in 0..file_count {
            let html = fs::read_to_string(format!("{}/post_{}.html", output_dir, index)).unwrap();
            expected_checksum += html.len() as i64;
        }

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_single_worker_prefetch_preserves_index_mapping() {
        let input_dir = unique_temp_dir("kujo_ssg_read_render_single_worker_prefetch_input");
        let output_dir = unique_temp_dir("kujo_ssg_read_render_single_worker_prefetch_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let file_count = 24usize;
        let mut source_paths = Vec::with_capacity(file_count);

        for index in 0..file_count {
            let source_path = format!("{}/post_{}.md", input_dir, index);
            let marker = format!("MARKER_{}_{}", index, (index * 13) + 7);
            let source_body =
                format!("# Entry {}\n\n{}\n\n{}", index, marker, "body ".repeat((index % 5) + 1));
            fs::write(&source_path, source_body).unwrap();
            source_paths.push(string_value(source_path.as_str()));
        }

        let mut interp = Interpreter::new();
        let args =
            vec![Value::Array(Arc::new(source_paths)), string_value(&output_dir), Value::Int(1)];

        let result = handle(&mut interp, "ssg_read_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Dict(dict) => {
                assert!(
                    matches!(dict.get("files"), Some(Value::Int(count)) if *count == file_count as i64)
                );
                assert!(matches!(dict.get("checksum"), Some(Value::Int(sum)) if *sum > 0));
                assert!(matches!(dict.get("read_ms"), Some(Value::Float(ms)) if *ms >= 0.0));
                assert!(
                    matches!(dict.get("render_write_ms"), Some(Value::Float(ms)) if *ms >= 0.0)
                );
            }
            _ => panic!("Expected Dict result"),
        }

        for index in 0..file_count {
            let marker = format!("MARKER_{}_{}", index, (index * 13) + 7);
            let html = fs::read_to_string(format!("{}/post_{}.html", output_dir, index)).unwrap();
            assert!(html.contains(format!("<h1>Post {}</h1>", index).as_str()));
            assert!(html.contains(marker.as_str()));
        }

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_large_batch_low_concurrency_preserves_outputs() {
        let input_dir = unique_temp_dir("kujo_ssg_read_render_large_low_concurrency_input");
        let output_dir = unique_temp_dir("kujo_ssg_read_render_large_low_concurrency_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let file_count = 96usize;
        let mut source_paths = Vec::with_capacity(file_count);
        for index in 0..file_count {
            let source_path = format!("{}/post_{}.md", input_dir, index);
            let body = format!("# Entry {}\n\n{}", index, "content ".repeat((index % 7) + 1));
            fs::write(&source_path, body).unwrap();
            source_paths.push(string_value(source_path.as_str()));
        }

        let mut interp = Interpreter::new();
        let args =
            vec![Value::Array(Arc::new(source_paths)), string_value(&output_dir), Value::Int(2)];

        let result = handle(&mut interp, "ssg_read_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        let checksum = match resolved {
            Value::Dict(dict) => {
                assert!(
                    matches!(dict.get("files"), Some(Value::Int(count)) if *count == file_count as i64)
                );
                assert!(matches!(dict.get("read_ms"), Some(Value::Float(ms)) if *ms >= 0.0));
                assert!(
                    matches!(dict.get("render_write_ms"), Some(Value::Float(ms)) if *ms >= 0.0)
                );
                match dict.get("checksum") {
                    Some(Value::Int(value)) => *value,
                    _ => panic!("Expected checksum int"),
                }
            }
            _ => panic!("Expected Dict result"),
        };

        let mut expected_checksum = 0i64;
        for index in 0..file_count {
            let html = fs::read_to_string(format!("{}/post_{}.html", output_dir, index)).unwrap();
            expected_checksum += html.len() as i64;
        }

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_high_concurrency_preserves_outputs() {
        let input_dir = unique_temp_dir("kujo_ssg_read_render_high_concurrency_input");
        let output_dir = unique_temp_dir("kujo_ssg_read_render_high_concurrency_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let file_count = 72usize;
        let mut source_paths = Vec::with_capacity(file_count);
        for index in 0..file_count {
            let source_path = format!("{}/post_{}.md", input_dir, index);
            let body = format!("# Post {}\n\n{}", index, "content ".repeat((index % 6) + 1));
            fs::write(&source_path, body).unwrap();
            source_paths.push(string_value(source_path.as_str()));
        }

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(source_paths)),
            string_value(&output_dir),
            Value::Int((file_count + 12) as i64),
        ];

        let result = handle(&mut interp, "ssg_read_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        let checksum = match resolved {
            Value::Dict(dict) => {
                assert!(
                    matches!(dict.get("files"), Some(Value::Int(count)) if *count == file_count as i64)
                );
                assert!(matches!(dict.get("read_ms"), Some(Value::Float(ms)) if *ms >= 0.0));
                assert!(
                    matches!(dict.get("render_write_ms"), Some(Value::Float(ms)) if *ms >= 0.0)
                );
                match dict.get("checksum") {
                    Some(Value::Int(value)) => *value,
                    _ => panic!("Expected checksum int"),
                }
            }
            _ => panic!("Expected Dict result"),
        };

        let mut expected_checksum = 0i64;
        for index in 0..file_count {
            let html = fs::read_to_string(format!("{}/post_{}.html", output_dir, index)).unwrap();
            expected_checksum += html.len() as i64;
        }

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_extreme_concurrency_limit_preserves_outputs() {
        let input_dir = unique_temp_dir("kujo_ssg_read_render_extreme_concurrency_input");
        let output_dir = unique_temp_dir("kujo_ssg_read_render_extreme_concurrency_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let file_count = 12usize;
        let mut source_paths = Vec::with_capacity(file_count);
        for index in 0..file_count {
            let source_path = format!("{}/post_{}.md", input_dir, index);
            let body = format!("# Extreme {}\n\n{}", index, "payload ".repeat((index % 4) + 1));
            fs::write(&source_path, body).unwrap();
            source_paths.push(string_value(source_path.as_str()));
        }

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(source_paths)),
            string_value(&output_dir),
            Value::Int(10_000),
        ];

        let result = handle(&mut interp, "ssg_read_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        let checksum = match resolved {
            Value::Dict(dict) => {
                assert!(
                    matches!(dict.get("files"), Some(Value::Int(count)) if *count == file_count as i64)
                );
                assert!(matches!(dict.get("read_ms"), Some(Value::Float(ms)) if *ms >= 0.0));
                assert!(
                    matches!(dict.get("render_write_ms"), Some(Value::Float(ms)) if *ms >= 0.0)
                );
                match dict.get("checksum") {
                    Some(Value::Int(value)) => *value,
                    _ => panic!("Expected checksum int"),
                }
            }
            _ => panic!("Expected Dict result"),
        };

        let mut expected_checksum = 0i64;
        for index in 0..file_count {
            let html = fs::read_to_string(format!("{}/post_{}.html", output_dir, index)).unwrap();
            expected_checksum += html.len() as i64;
        }

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_validates_argument_contracts() {
        let mut interp = Interpreter::new();

        let wrong_arity = handle(&mut interp, "ssg_read_render_and_write_pages", &[]).unwrap();
        assert!(
            matches!(wrong_arity, Value::Error(msg) if msg.contains("expects 2 or 3 arguments"))
        );

        let bad_first = handle(
            &mut interp,
            "ssg_read_render_and_write_pages",
            &[Value::Int(1), string_value("tmp/out")],
        )
        .unwrap();
        assert!(
            matches!(bad_first, Value::Error(msg) if msg.contains("first argument must be an array"))
        );

        let bad_source_path_element = handle(
            &mut interp,
            "ssg_read_render_and_write_pages",
            &[Value::Array(Arc::new(vec![Value::Int(1)])), string_value("tmp/out")],
        )
        .unwrap();
        assert!(
            matches!(bad_source_path_element, Value::Error(msg) if msg.contains("source path at index 0 must be a string"))
        );

        let bad_second = handle(
            &mut interp,
            "ssg_read_render_and_write_pages",
            &[Value::Array(Arc::new(vec![])), Value::Int(1)],
        )
        .unwrap();
        assert!(
            matches!(bad_second, Value::Error(msg) if msg.contains("second argument must be a string output_dir"))
        );

        let bad_limit = handle(
            &mut interp,
            "ssg_read_render_and_write_pages",
            &[
                Value::Array(Arc::new(vec![string_value("tmp/in/post_0.md")])),
                string_value("tmp/out"),
                Value::Int(0),
            ],
        )
        .unwrap();
        assert!(
            matches!(bad_limit, Value::Error(msg) if msg.contains("concurrency_limit must be > 0"))
        );
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_propagates_read_failure() {
        let output_dir = unique_temp_dir("kujo_ssg_read_render_read_fail_output");
        fs::create_dir_all(&output_dir).unwrap();

        let missing_source = unique_temp_dir("kujo_ssg_read_render_missing_source");
        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![string_value(&missing_source)])),
            string_value(&output_dir),
            Value::Int(1),
        ];

        let result = handle(&mut interp, "ssg_read_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result);

        match resolved {
            Ok(_) => panic!("Expected promise rejection for missing source file"),
            Err(msg) => assert!(msg.contains("Failed to read file")),
        }

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_read_render_and_write_pages_propagates_write_failure() {
        let input_dir = unique_temp_dir("kujo_ssg_read_render_write_fail_input");
        fs::create_dir_all(&input_dir).unwrap();

        let source = format!("{}/post_0.md", input_dir);
        fs::write(&source, "# Post 0").unwrap();

        let missing_output_dir = unique_temp_dir("kujo_ssg_read_render_write_fail_output");

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![string_value(&source)])),
            string_value(&missing_output_dir),
            Value::Int(1),
        ];

        let result = handle(&mut interp, "ssg_read_render_and_write_pages", &args).unwrap();
        let resolved = await_promise(result);

        match resolved {
            Ok(_) => panic!("Expected promise rejection for missing output directory"),
            Err(msg) => assert!(msg.contains("Failed to write file")),
        }

        let _ = fs::remove_dir_all(&input_dir);
    }

    // --- ssg_run_rayon_read_render_write helper tests ---

    #[test]
    fn test_ssg_run_rayon_read_render_write_reads_and_writes_correctly() {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_input");
        let output_dir = unique_temp_dir("kujo_rayon_rrw_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source_a = format!("{}/post_0.md", input_dir);
        let source_b = format!("{}/post_1.md", input_dir);
        fs::write(&source_a, "# Post 0\n\nBody A").unwrap();
        fs::write(&source_b, "# Post 1\n\nBody B").unwrap();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), 2);

        let result = ssg_run_rayon_read_render_write(
            vec![source_a.clone(), source_b.clone()],
            output_paths,
            render_prefixes,
            2,
            true,
        )
        .unwrap();

        let (checksum, read_ms, render_write_ms) = result;
        assert!(checksum > 0, "checksum must be positive");
        assert!(read_ms >= 0.0, "read_ms must be non-negative");
        assert!(render_write_ms >= 0.0, "render_write_ms must be non-negative");

        let html_a = fs::read_to_string(format!("{}/post_0.html", output_dir)).unwrap();
        let html_b = fs::read_to_string(format!("{}/post_1.html", output_dir)).unwrap();

        assert_eq!(
            html_a,
            "<html><body><h1>Post 0</h1><article># Post 0\n\nBody A</article></body></html>"
        );
        assert_eq!(
            html_b,
            "<html><body><h1>Post 1</h1><article># Post 1\n\nBody B</article></body></html>"
        );

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_read_source_file_bytes_reuses_buffer_across_large_then_small_reads() {
        let input_dir = unique_temp_dir("kujo_ssg_reused_read_buffer_large_small");
        fs::create_dir_all(&input_dir).unwrap();

        let large_path = format!("{}/large.md", input_dir);
        let small_path = format!("{}/small.md", input_dir);
        let large_payload = "L".repeat(8192);
        let small_payload = "tiny";
        fs::write(&large_path, large_payload.as_bytes()).unwrap();
        fs::write(&small_path, small_payload.as_bytes()).unwrap();

        let mut read_buffer: Vec<u8> = Vec::new();
        ssg_read_source_file_bytes(large_path.as_str(), &mut read_buffer).unwrap();
        let large_capacity = read_buffer.capacity();
        assert_eq!(read_buffer.as_slice(), large_payload.as_bytes());

        ssg_read_source_file_bytes(small_path.as_str(), &mut read_buffer).unwrap();
        assert_eq!(read_buffer.as_slice(), small_payload.as_bytes());
        assert!(
            read_buffer.capacity() >= large_capacity,
            "buffer capacity should be retained across reused reads"
        );

        let _ = fs::remove_dir_all(&input_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_checksum_matches_written_bytes() {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_checksum_input");
        let output_dir = unique_temp_dir("kujo_rayon_rrw_checksum_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source_paths: Vec<String> = (0..3)
            .map(|i| {
                let p = format!("{}/post_{}.md", input_dir, i);
                fs::write(&p, format!("# Post {}\n\nGenerated page {}", i, i)).unwrap();
                p
            })
            .collect();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), 3);

        let (checksum, _read_ms, _render_write_ms) =
            ssg_run_rayon_read_render_write(source_paths, output_paths, render_prefixes, 3, true)
                .unwrap();

        let expected_checksum: i64 = (0..3_usize)
            .map(|i| {
                let path = format!("{}/post_{}.html", output_dir, i);
                fs::read_to_string(&path).unwrap().len() as i64
            })
            .sum();

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_can_skip_stage_timers() {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_skip_stage_timers_input");
        let output_dir = unique_temp_dir("kujo_rayon_rrw_skip_stage_timers_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source_paths: Vec<String> = (0..4)
            .map(|i| {
                let p = format!("{}/post_{}.md", input_dir, i);
                fs::write(&p, format!("# Post {}\n\nGenerated page {}", i, i)).unwrap();
                p
            })
            .collect();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), source_paths.len());

        let (checksum, read_ms, render_write_ms) =
            ssg_run_rayon_read_render_write(source_paths, output_paths, render_prefixes, 4, false)
                .unwrap();

        let expected_checksum: i64 = (0..4_usize)
            .map(|i| {
                let path = format!("{}/post_{}.html", output_dir, i);
                fs::read(path).unwrap().len() as i64
            })
            .sum();

        assert_eq!(checksum, expected_checksum);
        assert_eq!(read_ms, 0.0);
        assert_eq!(render_write_ms, 0.0);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_propagates_read_failure() {
        let output_dir = unique_temp_dir("kujo_rayon_rrw_read_fail_output");
        fs::create_dir_all(&output_dir).unwrap();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), 1);

        let result = ssg_run_rayon_read_render_write(
            vec!["/nonexistent/path/post_0.md".to_string()],
            output_paths,
            render_prefixes,
            1,
            true,
        );

        assert!(result.is_err(), "Expected error for missing source file");
        let msg = result.unwrap_err();
        assert!(msg.contains("Failed to read file"), "Expected 'Failed to read file' in: {}", msg);

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_propagates_write_failure() {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_write_fail_input");
        fs::create_dir_all(&input_dir).unwrap();

        let source = format!("{}/post_0.md", input_dir);
        fs::write(&source, "# Post 0").unwrap();

        let missing_output_dir = unique_temp_dir("kujo_rayon_rrw_write_fail_output");
        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(missing_output_dir.as_str(), 1);

        // Do NOT create missing_output_dir so writes will fail.
        let result =
            ssg_run_rayon_read_render_write(vec![source], output_paths, render_prefixes, 1, true);

        assert!(result.is_err(), "Expected error for missing output directory");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("Failed to write file"),
            "Expected 'Failed to write file' in: {}",
            msg
        );

        let _ = fs::remove_dir_all(&input_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_rejects_shape_mismatch() {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_shape_mismatch_input");
        let output_dir = unique_temp_dir("kujo_rayon_rrw_shape_mismatch_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source = format!("{}/post_0.md", input_dir);
        fs::write(&source, "# Post 0").unwrap();

        let output_path = format!("{}/post_0.html", output_dir);

        let result = ssg_run_rayon_read_render_write(
            vec![source],
            vec![output_path],
            Arc::new(Vec::new()),
            1,
            true,
        );

        assert!(result.is_err(), "Expected shape mismatch to return an error");
        let msg = result.unwrap_err();
        assert!(msg.contains("internal SSG batch shape mismatch"), "Unexpected error: {}", msg);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_with_reused_output_path_buffer_reads_and_writes_correctly(
    ) {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_reused_path_input");
        let output_dir = unique_temp_dir("kujo_rayon_rrw_reused_path_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source_a = format!("{}/post_0.md", input_dir);
        let source_b = format!("{}/post_1.md", input_dir);
        fs::write(&source_a, "# Post 0\n\nBody A").unwrap();
        fs::write(&source_b, "# Post 1\n\nBody B").unwrap();

        let (output_suffixes, render_prefixes) = ssg_get_cached_output_suffixes_and_prefixes(2);
        let (checksum, read_ms, render_write_ms) =
            ssg_run_rayon_read_render_write_with_reused_output_path_buffer(
                vec![source_a, source_b],
                output_dir.clone(),
                output_suffixes,
                render_prefixes,
                2,
                true,
            )
            .unwrap();

        assert!(checksum > 0, "checksum must be positive");
        assert!(read_ms >= 0.0, "read_ms must be non-negative");
        assert!(render_write_ms >= 0.0, "render_write_ms must be non-negative");

        let html_a = fs::read_to_string(format!("{}/post_0.html", output_dir)).unwrap();
        let html_b = fs::read_to_string(format!("{}/post_1.html", output_dir)).unwrap();
        assert_eq!(
            html_a,
            "<html><body><h1>Post 0</h1><article># Post 0\n\nBody A</article></body></html>"
        );
        assert_eq!(
            html_b,
            "<html><body><h1>Post 1</h1><article># Post 1\n\nBody B</article></body></html>"
        );

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_with_reused_output_path_buffer_preserves_checksum_and_stage_toggle(
    ) {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_reused_path_checksum_input");
        let output_dir = unique_temp_dir("kujo_rayon_rrw_reused_path_checksum_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source_paths: Vec<String> = (0..4)
            .map(|i| {
                let p = format!("{}/post_{}.md", input_dir, i);
                fs::write(&p, format!("# Post {}\n\nGenerated page {}", i, i)).unwrap();
                p
            })
            .collect();

        let (output_suffixes, render_prefixes) =
            ssg_get_cached_output_suffixes_and_prefixes(source_paths.len());

        let (checksum, read_ms, render_write_ms) =
            ssg_run_rayon_read_render_write_with_reused_output_path_buffer(
                source_paths,
                output_dir.clone(),
                output_suffixes,
                render_prefixes,
                4,
                false,
            )
            .unwrap();

        let expected_checksum: i64 = (0..4usize)
            .map(|i| fs::read(format!("{}/post_{}.html", output_dir, i)).unwrap().len() as i64)
            .sum();

        assert_eq!(checksum, expected_checksum);
        assert_eq!(read_ms, 0.0);
        assert_eq!(render_write_ms, 0.0);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_with_reused_output_path_buffer_rejects_shape_mismatch()
    {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_reused_shape_mismatch_input");
        fs::create_dir_all(&input_dir).unwrap();

        let source = format!("{}/post_0.md", input_dir);
        fs::write(&source, "# Post 0").unwrap();

        let output_suffixes = Arc::new(Vec::<Arc<str>>::new());
        let render_prefixes = Arc::new(Vec::<Arc<str>>::new());

        let result = ssg_run_rayon_read_render_write_with_reused_output_path_buffer(
            vec![source],
            "/tmp".to_string(),
            output_suffixes,
            render_prefixes,
            1,
            true,
        );

        assert!(result.is_err(), "Expected shape mismatch to return an error");
        let msg = result.unwrap_err();
        assert!(msg.contains("internal SSG batch shape mismatch"), "Unexpected error: {}", msg);
        assert!(msg.contains("output_suffixes=0"), "Unexpected error: {}", msg);

        let _ = fs::remove_dir_all(&input_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_with_reused_output_path_buffer_propagates_write_failure(
    ) {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_reused_write_fail_input");
        fs::create_dir_all(&input_dir).unwrap();

        let source = format!("{}/post_0.md", input_dir);
        fs::write(&source, "# Post 0").unwrap();

        let missing_output_dir = unique_temp_dir("kujo_rayon_rrw_reused_write_fail_output");
        let (output_suffixes, render_prefixes) = ssg_get_cached_output_suffixes_and_prefixes(1);

        let result = ssg_run_rayon_read_render_write_with_reused_output_path_buffer(
            vec![source],
            missing_output_dir.clone(),
            output_suffixes,
            render_prefixes,
            1,
            true,
        );

        assert!(result.is_err(), "Expected error for missing output directory");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("Failed to write file"),
            "Expected 'Failed to write file' in: {}",
            msg
        );
        assert!(
            msg.contains(format!("{}/post_0.html", missing_output_dir).as_str()),
            "Expected output file path in error: {}",
            msg
        );

        let _ = fs::remove_dir_all(&input_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_unicode_checksum_matches_written_bytes() {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_unicode_input");
        let output_dir = unique_temp_dir("kujo_rayon_rrw_unicode_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source_a = format!("{}/post_0.md", input_dir);
        let source_b = format!("{}/post_1.md", input_dir);
        // UTF-8 multibyte content
        fs::write(&source_a, "# Café ☕").unwrap();
        fs::write(&source_b, "# Emoji 🚀✅").unwrap();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), 2);

        let (checksum, _read_ms, _render_write_ms) = ssg_run_rayon_read_render_write(
            vec![source_a, source_b],
            output_paths,
            render_prefixes,
            2,
            true,
        )
        .unwrap();

        let html_a = fs::read_to_string(format!("{}/post_0.html", output_dir)).unwrap();
        let html_b = fs::read_to_string(format!("{}/post_1.html", output_dir)).unwrap();
        let expected_checksum = (html_a.len() + html_b.len()) as i64;

        assert_eq!(checksum, expected_checksum, "Unicode checksum must match written byte count");

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_allows_non_utf8_source_bytes() {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_non_utf8_input");
        let output_dir = unique_temp_dir("kujo_rayon_rrw_non_utf8_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source = format!("{}/post_0.md", input_dir);
        let source_bytes: [u8; 8] = [0x23, 0x20, 0x66, 0x80, 0x81, 0x82, 0x0a, 0x0a];
        fs::write(&source, source_bytes).unwrap();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), 1);

        let (checksum, read_ms, render_write_ms) =
            ssg_run_rayon_read_render_write(vec![source], output_paths, render_prefixes, 1, true)
                .unwrap();

        let rendered_bytes = fs::read(format!("{}/post_0.html", output_dir)).unwrap();
        assert_eq!(checksum, rendered_bytes.len() as i64);
        assert!(read_ms >= 0.0);
        assert!(render_write_ms >= 0.0);
        assert!(rendered_bytes.starts_with(b"<html><body><h1>Post 0</h1><article>"));
        assert!(rendered_bytes.ends_with(SSG_HTML_SUFFIX.as_bytes()));
        assert!(rendered_bytes.windows(source_bytes.len()).any(|window| window == source_bytes));

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_mixed_source_sizes_preserve_page_isolation() {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_mixed_sizes_input");
        let output_dir = unique_temp_dir("kujo_rayon_rrw_mixed_sizes_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let large_marker = "LARGE-PAYLOAD-MARKER";
        let tiny_marker = "tiny-marker";
        let source_large = format!("{}/post_0.md", input_dir);
        let source_tiny = format!("{}/post_1.md", input_dir);
        let source_medium = format!("{}/post_2.md", input_dir);
        let large_body = format!("{}\n{}", large_marker, "x".repeat(16000));
        let medium_body = "medium-body-123".repeat(16);
        fs::write(&source_large, large_body.as_bytes()).unwrap();
        fs::write(&source_tiny, tiny_marker.as_bytes()).unwrap();
        fs::write(&source_medium, medium_body.as_bytes()).unwrap();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), 3);

        let (_checksum, read_ms, render_write_ms) = ssg_run_rayon_read_render_write(
            vec![source_large, source_tiny, source_medium],
            output_paths,
            render_prefixes,
            3,
            true,
        )
        .unwrap();

        assert!(read_ms >= 0.0);
        assert!(render_write_ms >= 0.0);

        let tiny_output = fs::read_to_string(format!("{}/post_1.html", output_dir)).unwrap();
        assert!(tiny_output.contains(tiny_marker));
        assert!(
            !tiny_output.contains(large_marker),
            "tiny output must not leak content from larger reused buffers"
        );

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    // --- Single-pass pipeline regression tests ---
    // These tests specifically verify the behavioral contracts of the single-pass
    // Rayon optimization: per-file read+render+write without a phase barrier.

    #[test]
    fn test_ssg_run_rayon_single_pass_timing_fields_are_non_negative() {
        // Timing fields must be non-negative for any batch size including single file.
        let input_dir = unique_temp_dir("kujo_rayon_sp_timing_input");
        let output_dir = unique_temp_dir("kujo_rayon_sp_timing_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source = format!("{}/post_0.md", input_dir);
        fs::write(&source, "# Timing Test\n\nSingle file payload").unwrap();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), 1);

        let (checksum, read_ms, render_write_ms) =
            ssg_run_rayon_read_render_write(vec![source], output_paths, render_prefixes, 1, true)
                .unwrap();

        assert!(checksum > 0, "checksum must be positive for non-empty content");
        assert!(read_ms >= 0.0, "read_ms must be non-negative (cumulative nanoseconds)");
        assert!(render_write_ms >= 0.0, "render_write_ms must be non-negative");

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_single_pass_cumulative_timing_grows_with_file_count() {
        // Timing values are environment-sensitive and can vary with OS cache and scheduler
        // behavior. This test validates deterministic correctness growth instead: processing
        // more files must produce a strictly larger checksum while preserving non-negative
        // stage-metric contracts.
        let input_dir = unique_temp_dir("kujo_rayon_sp_cumulative_timing_input");
        let output_dir_1 = unique_temp_dir("kujo_rayon_sp_cumulative_timing_output_1");
        let output_dir_n = unique_temp_dir("kujo_rayon_sp_cumulative_timing_output_n");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir_1).unwrap();
        fs::create_dir_all(&output_dir_n).unwrap();

        let file_count = 16usize;
        let mut source_paths: Vec<String> = Vec::with_capacity(file_count);
        for index in 0..file_count {
            let p = format!("{}/post_{}.md", input_dir, index);
            fs::write(&p, format!("# Post {}\n\n{}", index, "body ".repeat(8))).unwrap();
            source_paths.push(p);
        }

        // Single-file run
        let (out_1, pfx_1) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir_1.as_str(), 1);
        let (checksum_1, read_ms_1, rw_ms_1) =
            ssg_run_rayon_read_render_write(vec![source_paths[0].clone()], out_1, pfx_1, 1, true)
                .unwrap();

        // Multi-file run
        let (out_n, pfx_n) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir_n.as_str(), file_count);
        let (checksum_n, read_ms_n, rw_ms_n) =
            ssg_run_rayon_read_render_write(source_paths, out_n, pfx_n, 4, true).unwrap();

        assert!(read_ms_1 >= 0.0, "single-file read_ms must be non-negative");
        assert!(rw_ms_1 >= 0.0, "single-file render_write_ms must be non-negative");
        assert!(read_ms_n >= 0.0, "multi-file read_ms must be non-negative");
        assert!(rw_ms_n >= 0.0, "multi-file render_write_ms must be non-negative");
        assert!(
            checksum_n > checksum_1,
            "checksum for {} files ({}) must be greater than single-file checksum ({})",
            file_count,
            checksum_n,
            checksum_1
        );

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir_1);
        let _ = fs::remove_dir_all(&output_dir_n);
    }

    #[test]
    fn test_ssg_run_rayon_single_pass_large_batch_preserves_checksum() {
        // Validate single-pass correctness with a larger batch (32 files) where
        // Rayon work-stealing interleaves reads and writes across multiple workers.
        let input_dir = unique_temp_dir("kujo_rayon_sp_large_batch_input");
        let output_dir = unique_temp_dir("kujo_rayon_sp_large_batch_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let file_count = 32usize;
        let source_paths: Vec<String> = (0..file_count)
            .map(|i| {
                let p = format!("{}/post_{}.md", input_dir, i);
                let body = format!("# Post {}\n\n{}", i, "content ".repeat((i % 8) + 1));
                fs::write(&p, &body).unwrap();
                p
            })
            .collect();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), file_count);

        let (checksum, read_ms, render_write_ms) =
            ssg_run_rayon_read_render_write(source_paths, output_paths, render_prefixes, 8, true)
                .unwrap();

        assert!(read_ms >= 0.0);
        assert!(render_write_ms >= 0.0);

        let expected_checksum: i64 = (0..file_count)
            .map(|i| {
                fs::read_to_string(format!("{}/post_{}.html", output_dir, i)).unwrap().len() as i64
            })
            .sum();

        assert_eq!(checksum, expected_checksum, "Large single-pass batch checksum mismatch");

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_single_pass_single_worker_preserves_index_mapping() {
        // Single-worker mode must still correctly route each file to its indexed output
        // path and render prefix (no cross-contamination from task interleaving).
        let input_dir = unique_temp_dir("kujo_rayon_sp_single_worker_index_input");
        let output_dir = unique_temp_dir("kujo_rayon_sp_single_worker_index_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let file_count = 8usize;
        let source_paths: Vec<String> = (0..file_count)
            .map(|i| {
                let p = format!("{}/post_{}.md", input_dir, i);
                let marker = format!("UNIQUE_MARKER_{}_X{}", i, i * 17 + 3);
                fs::write(&p, format!("# Entry {}\n\n{}", i, marker)).unwrap();
                p
            })
            .collect();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), file_count);

        let (checksum, _, _) =
            ssg_run_rayon_read_render_write(source_paths, output_paths, render_prefixes, 1, true)
                .unwrap();

        assert!(checksum > 0);

        for i in 0..file_count {
            let html = fs::read_to_string(format!("{}/post_{}.html", output_dir, i)).unwrap();
            let marker = format!("UNIQUE_MARKER_{}_X{}", i, i * 17 + 3);
            assert!(html.contains(&marker), "post_{}.html missing marker '{}'", i, marker);
            assert!(
                html.contains(&format!("<h1>Post {}</h1>", i)),
                "post_{}.html has wrong heading",
                i
            );
        }

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_partitioned_large_batch_checksum_matches_written_bytes()
    {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_partitioned_checksum_input");
        let output_dir = unique_temp_dir("kujo_rayon_rrw_partitioned_checksum_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        // Use a count that is large enough to force multiple Rayon chunk reductions.
        let file_count = 257usize;
        let source_paths: Vec<String> = (0..file_count)
            .map(|i| {
                let p = format!("{}/post_{}.md", input_dir, i);
                let body = format!("# Post {}\n\n{}", i, "chunk ".repeat((i % 7) + 1));
                fs::write(&p, body).unwrap();
                p
            })
            .collect();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), file_count);

        let (checksum, read_ms, render_write_ms) =
            ssg_run_rayon_read_render_write(source_paths, output_paths, render_prefixes, 8, true)
                .unwrap();

        assert!(read_ms >= 0.0);
        assert!(render_write_ms >= 0.0);

        let expected_checksum: i64 = (0..file_count)
            .map(|i| {
                fs::read_to_string(format!("{}/post_{}.html", output_dir, i)).unwrap().len() as i64
            })
            .sum();

        assert_eq!(checksum, expected_checksum);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_read_render_write_reports_error_when_any_partition_read_fails() {
        let input_dir = unique_temp_dir("kujo_rayon_rrw_partitioned_error_input");
        let output_dir = unique_temp_dir("kujo_rayon_rrw_partitioned_error_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let mut source_paths: Vec<String> = Vec::new();
        for i in 0..64usize {
            let p = format!("{}/post_{}.md", input_dir, i);
            fs::write(&p, format!("# Post {}\n\nbody", i)).unwrap();
            source_paths.push(p);
        }

        // Insert a single missing path among otherwise valid files.
        source_paths.push(format!("{}/missing_64.md", input_dir));

        let file_count = source_paths.len();
        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), file_count);

        let result =
            ssg_run_rayon_read_render_write(source_paths, output_paths, render_prefixes, 8, true);
        assert!(result.is_err());

        let msg = result.unwrap_err();
        assert!(msg.contains("Failed to read file"));
        assert!(msg.contains("index 64"));

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    // --- CPU-cap sizing tests ---
    // Verify that ssg_rayon_cpu_cap() + the clamped pool correctly handle
    // concurrency_limit values both above and below the CPU core count.

    #[test]
    fn test_ssg_rayon_cpu_cap_returns_at_least_one() {
        // ssg_rayon_cpu_cap() must always return >= 1, even on unusual hosts.
        let cap = ssg_rayon_cpu_cap();
        assert!(cap >= 1, "ssg_rayon_cpu_cap() must be >= 1, got {}", cap);
    }

    #[test]
    fn test_ssg_get_or_create_rayon_pool_reuses_existing_pool_for_same_size() {
        let pool_a = ssg_get_or_create_rayon_pool(1).unwrap();
        let pool_b = ssg_get_or_create_rayon_pool(1).unwrap();

        assert!(
            Arc::ptr_eq(&pool_a, &pool_b),
            "expected the same cached Rayon pool Arc for identical thread count"
        );
    }

    #[test]
    fn test_ssg_get_or_create_rayon_pool_distinguishes_thread_counts() {
        let pool_one = ssg_get_or_create_rayon_pool(1).unwrap();
        let pool_two = ssg_get_or_create_rayon_pool(2).unwrap();

        assert!(
            !Arc::ptr_eq(&pool_one, &pool_two),
            "expected different cached Rayon pools for different thread counts"
        );
    }

    #[test]
    fn test_ssg_run_rayon_cached_pool_repeated_calls_preserve_checksum_contract() {
        let input_dir = unique_temp_dir("kujo_rayon_cached_pool_repeat_input");
        let output_dir_a = unique_temp_dir("kujo_rayon_cached_pool_repeat_output_a");
        let output_dir_b = unique_temp_dir("kujo_rayon_cached_pool_repeat_output_b");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir_a).unwrap();
        fs::create_dir_all(&output_dir_b).unwrap();

        let file_count = 10usize;
        let source_paths: Vec<String> = (0..file_count)
            .map(|index| {
                let path = format!("{}/post_{}.md", input_dir, index);
                fs::write(&path, format!("# Post {}\n\n{}", index, "body ".repeat(4))).unwrap();
                path
            })
            .collect();

        let (out_a, pfx_a) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir_a.as_str(), file_count);
        let (checksum_a, read_ms_a, render_write_ms_a) =
            ssg_run_rayon_read_render_write(source_paths.clone(), out_a, pfx_a, 4, true).unwrap();

        let (out_b, pfx_b) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir_b.as_str(), file_count);
        let (checksum_b, read_ms_b, render_write_ms_b) =
            ssg_run_rayon_read_render_write(source_paths, out_b, pfx_b, 4, true).unwrap();

        assert!(read_ms_a >= 0.0 && render_write_ms_a >= 0.0);
        assert!(read_ms_b >= 0.0 && render_write_ms_b >= 0.0);
        assert_eq!(checksum_a, checksum_b, "repeated cached-pool runs must preserve checksum");

        let expected_checksum_a: i64 = (0..file_count)
            .map(|index| {
                fs::read_to_string(format!("{}/post_{}.html", output_dir_a, index)).unwrap().len()
                    as i64
            })
            .sum();
        let expected_checksum_b: i64 = (0..file_count)
            .map(|index| {
                fs::read_to_string(format!("{}/post_{}.html", output_dir_b, index)).unwrap().len()
                    as i64
            })
            .sum();

        assert_eq!(checksum_a, expected_checksum_a);
        assert_eq!(checksum_b, expected_checksum_b);

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir_a);
        let _ = fs::remove_dir_all(&output_dir_b);
    }

    #[test]
    fn test_ssg_run_rayon_oversized_concurrency_limit_clamps_to_cpu_count() {
        // When concurrency_limit >> CPU count (e.g. 256 default async pool size),
        // the actual thread pool should be capped to at most ssg_rayon_cpu_cap().
        // We validate correctness: output must still match expected checksums.
        let input_dir = unique_temp_dir("kujo_rayon_cpu_cap_input");
        let output_dir = unique_temp_dir("kujo_rayon_cpu_cap_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let file_count = 8usize;
        let source_paths: Vec<String> = (0..file_count)
            .map(|i| {
                let p = format!("{}/post_{}.md", input_dir, i);
                fs::write(&p, format!("# Post {}\n\npage {}", i, i)).unwrap();
                p
            })
            .collect();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), file_count);

        // Use a very large concurrency_limit (256) that is certain to exceed CPU count
        // on any CI or development machine; the pool must clamp and still produce
        // correct results.
        let (checksum, read_ms, render_write_ms) =
            ssg_run_rayon_read_render_write(source_paths, output_paths, render_prefixes, 256, true)
                .unwrap();

        assert!(checksum > 0, "checksum must be positive");
        assert!(read_ms >= 0.0, "read_ms must be non-negative");
        assert!(render_write_ms >= 0.0, "render_write_ms must be non-negative");

        let expected_checksum: i64 = (0..file_count)
            .map(|i| {
                fs::read_to_string(format!("{}/post_{}.html", output_dir, i)).unwrap().len() as i64
            })
            .sum();

        assert_eq!(
            checksum, expected_checksum,
            "checksum must match written bytes even with oversized concurrency_limit"
        );

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_small_concurrency_limit_respected_below_cpu_count() {
        // When concurrency_limit < CPU count, the pool should use concurrency_limit
        // threads (i.e. the cap does not inflate below the requested size).
        // Validate correctness: output checksums must match regardless.
        let input_dir = unique_temp_dir("kujo_rayon_small_limit_input");
        let output_dir = unique_temp_dir("kujo_rayon_small_limit_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let file_count = 6usize;
        let source_paths: Vec<String> = (0..file_count)
            .map(|i| {
                let p = format!("{}/post_{}.md", input_dir, i);
                fs::write(&p, format!("# Post {}\n\nbody {}", i, i)).unwrap();
                p
            })
            .collect();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), file_count);

        // concurrency_limit=1 is always <= CPU count — must produce correct output.
        let (checksum, read_ms, render_write_ms) =
            ssg_run_rayon_read_render_write(source_paths, output_paths, render_prefixes, 1, true)
                .unwrap();

        assert!(checksum > 0, "checksum must be positive");
        assert!(read_ms >= 0.0, "read_ms must be non-negative");
        assert!(render_write_ms >= 0.0, "render_write_ms must be non-negative");

        let expected: i64 = (0..file_count)
            .map(|i| {
                fs::read_to_string(format!("{}/post_{}.html", output_dir, i)).unwrap().len() as i64
            })
            .sum();

        assert_eq!(checksum, expected, "small concurrency_limit checksum must match written bytes");

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_run_rayon_cpu_cap_equal_concurrency_limit_produces_correct_output() {
        // When concurrency_limit == ssg_rayon_cpu_cap(), the pool size is unchanged
        // and output must remain correct (no off-by-one in the cap logic).
        let cpu_cap = ssg_rayon_cpu_cap();
        let input_dir = unique_temp_dir("kujo_rayon_exact_cap_input");
        let output_dir = unique_temp_dir("kujo_rayon_exact_cap_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        // Use exactly cpu_cap files so each worker gets at most one file.
        let file_count = cpu_cap.max(2);
        let source_paths: Vec<String> = (0..file_count)
            .map(|i| {
                let p = format!("{}/post_{}.md", input_dir, i);
                fs::write(&p, format!("# Post {}\n\nexact {}", i, i)).unwrap();
                p
            })
            .collect();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), file_count);

        let (checksum, _, _) = ssg_run_rayon_read_render_write(
            source_paths,
            output_paths,
            render_prefixes,
            cpu_cap,
            true,
        )
        .unwrap();

        let expected: i64 = (0..file_count)
            .map(|i| {
                fs::read_to_string(format!("{}/post_{}.html", output_dir, i)).unwrap().len() as i64
            })
            .sum();

        assert_eq!(
            checksum, expected,
            "cpu_cap-equal concurrency_limit checksum must match written bytes"
        );

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_parallel_map_rayon_len_mixed_collection_inputs() {
        let mut dict = DictMap::default();
        dict.insert("a".to_string().into(), Value::Int(1));
        dict.insert("b".to_string().into(), Value::Int(2));

        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![
                string_value("abc"),
                Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)])),
                Value::Dict(Arc::new(dict)),
            ])),
            Value::NativeFunction("len".to_string()),
            Value::Int(2),
        ];

        let result = handle(&mut interp, "parallel_map", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Array(values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(&values[0], Value::Int(n) if *n == 3));
                assert!(matches!(&values[1], Value::Int(n) if *n == 3));
                assert!(matches!(&values[2], Value::Int(n) if *n == 2));
            }
            _ => panic!("Expected Array result"),
        }
    }

    #[test]
    fn test_parallel_map_rayon_upper_and_lower_aliases() {
        let mut interp = Interpreter::new();
        let upper_args = vec![
            Value::Array(Arc::new(vec![string_value("kujo"), string_value("lang")])),
            Value::NativeFunction("upper".to_string()),
            Value::Int(4),
        ];

        let upper_result = handle(&mut interp, "parallel_map", &upper_args).unwrap();
        let upper_resolved = await_promise(upper_result).unwrap();
        match upper_resolved {
            Value::Array(values) => {
                assert!(matches!(&values[0], Value::Str(s) if s.as_str() == "KUJO"));
                assert!(matches!(&values[1], Value::Str(s) if s.as_str() == "LANG"));
            }
            _ => panic!("Expected Array result"),
        }

        let lower_args = vec![
            Value::Array(Arc::new(vec![string_value("A"), string_value("BC")])),
            Value::NativeFunction("to_lower".to_string()),
            Value::Int(4),
        ];

        let lower_result = handle(&mut interp, "parallel_map", &lower_args).unwrap();
        let lower_resolved = await_promise(lower_result).unwrap();
        match lower_resolved {
            Value::Array(values) => {
                assert!(matches!(&values[0], Value::Str(s) if s.as_str() == "a"));
                assert!(matches!(&values[1], Value::Str(s) if s.as_str() == "bc"));
            }
            _ => panic!("Expected Array result"),
        }
    }

    #[test]
    fn test_parallel_map_rayon_validates_mapper_input_types() {
        let mut interp = Interpreter::new();
        let args = vec![
            Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)])),
            Value::NativeFunction("upper".to_string()),
            Value::Int(2),
        ];

        let result = handle(&mut interp, "parallel_map", &args).unwrap();
        match result {
            Value::Error(msg) => {
                assert!(msg.contains("expects string elements"));
            }
            _ => panic!("Expected Value::Error for invalid mapper element types"),
        }
    }

    #[test]
    fn test_parallel_map_bytecode_mapper_uses_vm_jit_lane() {
        let mut interp = Interpreter::new();
        let mapper = bytecode_increment_mapper();

        let args = vec![
            Value::Array(Arc::new(vec![Value::Int(1), Value::Int(9), Value::Int(41)])),
            mapper,
        ];

        let result = handle(&mut interp, "parallel_map", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Array(values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(&values[0], Value::Int(n) if *n == 2));
                assert!(matches!(&values[1], Value::Int(n) if *n == 10));
                assert!(matches!(&values[2], Value::Int(n) if *n == 42));
            }
            _ => panic!("Expected Array result"),
        }
    }

    #[test]
    fn test_par_map_alias_with_bytecode_mapper() {
        let mut interp = Interpreter::new();
        let mapper = bytecode_increment_mapper();

        let args = vec![Value::Array(Arc::new(vec![Value::Int(0), Value::Int(5)])), mapper];

        let result = handle(&mut interp, "par_map", &args).unwrap();
        let resolved = await_promise(result).unwrap();

        match resolved {
            Value::Array(values) => {
                assert_eq!(values.len(), 2);
                assert!(matches!(&values[0], Value::Int(n) if *n == 1));
                assert!(matches!(&values[1], Value::Int(n) if *n == 6));
            }
            _ => panic!("Expected Array result"),
        }
    }

    #[test]
    fn test_set_task_pool_size_round_trip() {
        let mut interp = Interpreter::new();

        let initial = handle(&mut interp, "get_task_pool_size", &[]).unwrap();
        let initial_size = match initial {
            Value::Int(n) => n,
            _ => panic!("Expected Int from get_task_pool_size"),
        };

        let previous = handle(&mut interp, "set_task_pool_size", &[Value::Int(7)]).unwrap();
        match previous {
            Value::Int(n) => assert_eq!(n, initial_size),
            _ => panic!("Expected Int previous size from set_task_pool_size"),
        }

        let current = handle(&mut interp, "get_task_pool_size", &[]).unwrap();
        match current {
            Value::Int(n) => assert_eq!(n, 7),
            _ => panic!("Expected Int from get_task_pool_size"),
        }
    }

    #[test]
    fn test_set_task_pool_size_validates_input() {
        let mut interp = Interpreter::new();

        let non_positive = handle(&mut interp, "set_task_pool_size", &[Value::Int(0)]).unwrap();
        match non_positive {
            Value::Error(msg) => assert!(msg.contains("size must be > 0")),
            _ => panic!("Expected Value::Error for non-positive pool size"),
        }

        let wrong_type =
            handle(&mut interp, "set_task_pool_size", &[Value::Str(Arc::new("2".to_string()))])
                .unwrap();
        match wrong_type {
            Value::Error(msg) => assert!(msg.contains("requires an integer size argument")),
            _ => panic!("Expected Value::Error for non-integer pool size"),
        }
    }

    #[test]
    fn test_get_task_pool_size_rejects_arguments() {
        let mut interp = Interpreter::new();

        let result = handle(&mut interp, "get_task_pool_size", &[Value::Int(1)]).unwrap();
        match result {
            Value::Error(msg) => assert!(msg.contains("expects 0 arguments")),
            _ => panic!("Expected Value::Error for get_task_pool_size arguments"),
        }
    }

    // --- sync-vectored-write Rayon path tests ---
    // Verify that the sync-vectored render path produces byte-for-byte identical
    // output and correct checksums in ssg_run_rayon_read_render_write.

    #[test]
    fn test_ssg_sync_vectored_output_matches_expected_html_structure() {
        // Sync-vectored path must produce the same HTML structure that fs::write
        // previously produced: prefix + source body + suffix bytes verbatim.
        let input_dir = unique_temp_dir("kujo_bw_structure_input");
        let output_dir = unique_temp_dir("kujo_bw_structure_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let source_body = "Hello, world!";
        let source_path = format!("{}/post_0.md", input_dir);
        fs::write(&source_path, source_body).unwrap();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), 1);

        ssg_run_rayon_read_render_write(
            vec![source_path],
            output_paths.clone(),
            render_prefixes.clone(),
            1,
            true,
        )
        .unwrap();

        let written = fs::read_to_string(&output_paths[0]).unwrap();
        let expected = format!(
            "{}{}{}{}{}",
            SSG_HTML_PREFIX_START, "0", SSG_HTML_PREFIX_END, source_body, SSG_HTML_SUFFIX
        );
        assert_eq!(
            written, expected,
            "Sync-vectored path must produce prefix + body + suffix verbatim"
        );

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_sync_vectored_checksum_matches_prefix_plus_body_plus_suffix_length() {
        // Checksum = sum of rendered HTML byte lengths. With the sync-vectored path,
        // checksum must still equal len(prefix) + len(body) + len(suffix) per file.
        let input_dir = unique_temp_dir("kujo_bw_checksum_input");
        let output_dir = unique_temp_dir("kujo_bw_checksum_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let bodies = vec!["# Post 0\n\nbody zero", "# Post 1\n\nbody one and more"];
        let source_paths: Vec<String> = bodies
            .iter()
            .enumerate()
            .map(|(i, body)| {
                let p = format!("{}/post_{}.md", input_dir, i);
                fs::write(&p, body).unwrap();
                p
            })
            .collect();

        let file_count = bodies.len();
        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), file_count);

        let (checksum, _, _) =
            ssg_run_rayon_read_render_write(source_paths, output_paths, render_prefixes, 2, true)
                .unwrap();

        // Expected checksum is the sum of all written HTML byte lengths.
        let expected_checksum: i64 = bodies
            .iter()
            .enumerate()
            .map(|(i, body)| {
                let index_str = i.to_string();
                let prefix_len =
                    SSG_HTML_PREFIX_START.len() + index_str.len() + SSG_HTML_PREFIX_END.len();
                (prefix_len + body.len() + SSG_HTML_SUFFIX.len()) as i64
            })
            .sum();

        assert_eq!(
            checksum, expected_checksum,
            "Sync-vectored path checksum must equal sum of rendered HTML byte lengths"
        );

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_ssg_sync_vectored_write_failure_propagates_error() {
        // Sync-vectored write errors must propagate as Err with the same message format
        // as the previous fs::write path: "Failed to write file '...' (index N): ...".
        let input_dir = unique_temp_dir("kujo_bw_wfail_input");
        fs::create_dir_all(&input_dir).unwrap();

        let source_path = format!("{}/post_0.md", input_dir);
        fs::write(&source_path, "body").unwrap();

        // Use a non-existent output directory to force a write failure.
        let bad_output = format!("{}/nonexistent_subdir/post_0.html", input_dir);
        let render_prefixes = Arc::new(vec![Arc::<str>::from(format!(
            "{}{}{}",
            SSG_HTML_PREFIX_START, "0", SSG_HTML_PREFIX_END
        ))]);

        let result = ssg_run_rayon_read_render_write(
            vec![source_path],
            vec![bad_output.clone()],
            render_prefixes,
            1,
            true,
        );

        assert!(result.is_err(), "Sync-vectored write failure must return Err");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("Failed to write file"),
            "Error message must contain 'Failed to write file', got: {}",
            msg
        );
        assert!(msg.contains("(index 0)"), "Error message must contain '(index 0)', got: {}", msg);

        let _ = fs::remove_dir_all(&input_dir);
    }

    #[test]
    fn test_ssg_sync_vectored_unicode_content_checksum_is_byte_accurate() {
        // Checksum must count UTF-8 bytes, not Unicode scalar values.  A 3-byte
        // CJK character must contribute 3 to the checksum, not 1.
        let input_dir = unique_temp_dir("kujo_bw_unicode_input");
        let output_dir = unique_temp_dir("kujo_bw_unicode_output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        // "日本語" = 9 UTF-8 bytes (3 chars × 3 bytes each)
        let source_body = "日本語";
        let source_path = format!("{}/post_0.md", input_dir);
        fs::write(&source_path, source_body.as_bytes()).unwrap();

        let (output_paths, render_prefixes) =
            ssg_build_output_paths_and_prefixes_for_batch(output_dir.as_str(), 1);

        let (checksum, _, _) = ssg_run_rayon_read_render_write(
            vec![source_path],
            output_paths.clone(),
            render_prefixes,
            1,
            true,
        )
        .unwrap();

        let prefix_len = SSG_HTML_PREFIX_START.len() + "0".len() + SSG_HTML_PREFIX_END.len();
        let expected = (prefix_len + source_body.len() + SSG_HTML_SUFFIX.len()) as i64;

        assert_eq!(
            checksum, expected,
            "Sync-vectored checksum must be byte-accurate for multibyte Unicode content"
        );

        let written = fs::read_to_string(&output_paths[0]).unwrap();
        assert!(
            written.contains(source_body),
            "Sync-vectored path must preserve multibyte Unicode content verbatim"
        );

        let _ = fs::remove_dir_all(&input_dir);
        let _ = fs::remove_dir_all(&output_dir);
    }
}
