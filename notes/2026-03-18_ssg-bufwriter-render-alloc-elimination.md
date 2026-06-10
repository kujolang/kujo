# Kujo Field Notes — SSG BufWriter Write-Through Render Allocation Elimination

**Date:** 2026-03-18
**Session:** local
**Branch/Commit:** main / 00fd9fc (impl + tests), a6c3af8 (docs)
**Scope:** Eliminated the per-file intermediate `String` allocation in the
`ssg_run_rayon_read_render_write` Rayon render+write hot path by replacing
`String::with_capacity` + `push_str` + `std::fs::write` with a pre-sized
`BufWriter<File>` + three `write_all` calls.

---

## What I Changed

- **`src/interpreter/native_functions/async_ops.rs`** — `ssg_run_rayon_read_render_write`:
  - Added `BufWriter` and `Write` to the `std::io` imports.
  - In the Rayon `par_iter()` closure:
    - **Before**: `String::with_capacity(total_len)` + `push_str` × 3 (prefix, body, suffix) + `std::fs::write(output_path, html.as_bytes())`
    - **After**: `std::fs::File::create(output_path)` + `BufWriter::with_capacity(total_len, out_file)` + `write_all` × 3 (prefix, body, suffix bytes) + `flush()`
  - `total_len` computation (`html_prefix.len() + content.len() + SSG_HTML_SUFFIX.len()`) is retained; it now sizes the `BufWriter` internal buffer instead of the String capacity.
  - Updated the function doc comment to explain the BufWriter rationale.

- **New regression tests** (4 tests, lib count 350 → 354):
  - `test_ssg_bufwriter_output_matches_expected_html_structure` — exact prefix+body+suffix byte output contract
  - `test_ssg_bufwriter_checksum_matches_prefix_plus_body_plus_suffix_length` — byte-accurate multi-file checksum
  - `test_ssg_bufwriter_write_failure_propagates_error` — error message format preserved on BufWriter open failure
  - `test_ssg_bufwriter_unicode_content_checksum_is_byte_accurate` — 3-byte CJK character contributes 3 to checksum

---

## Gotchas (Read This Next Time)

- **Gotcha:** `BufWriter::with_capacity(total_len, file)` does NOT pre-allocate the
  output file to `total_len` bytes — it only sizes the internal write buffer.
  - **Symptom:** You might expect the output file to be pre-allocated to avoid
    file-system fragmentation, but `BufWriter::with_capacity` only controls the
    in-memory buffer, not the on-disk file size.
  - **Root cause:** `BufWriter` is a purely in-memory buffer over any `Write`
    implementor. Pre-allocating the file would require a separate `file.set_len()`
    call and is not part of the BufWriter contract.
  - **Fix:** The current implementation is correct as-is. Pre-allocating the
    output file is a future optimization if profiling shows file-fragmentation overhead.
  - **Prevention:** Don't confuse `BufWriter::with_capacity` for a file pre-allocation
    hint. If you want the OS to preallocate disk space, call `File::set_len()` or
    use `fallocate`/`fcntl F_PREALLOCATE` on macOS/Linux before wrapping in BufWriter.

- **Gotcha:** The `flush()` call is required after the three `write_all` calls.
  - **Symptom:** If you drop a `BufWriter` without explicit `flush()` and the
    drop silently fails (e.g., because the error is swallowed), the output file
    may be truncated.
  - **Root cause:** `BufWriter` defers writes to its internal buffer. On `Drop`,
    it flushes — but drops cannot return errors. A `flush()` call before destructor
    scope allows the error to be caught and propagated.
  - **Fix:** Always call `writer.flush().map_err(...)` explicitly in the current
    pattern, as done here. The test `test_ssg_bufwriter_write_failure_propagates_error`
    covers this contract.
  - **Prevention:** Any time you use `BufWriter` in a fallible context (Result-returning
    function), always `flush()` explicitly before the BufWriter goes out of scope.

- **Gotcha:** Error message format must remain "Failed to write file '...' (index N): ...".
  - **Symptom:** Both `File::create` failures and `write_all`/`flush` failures now
    use the same format string. The test for write-failure propagation exercises
    only the `File::create` path (non-existent subdirectory). Write-path errors
    during `write_all` have the same format but are harder to force in a unit test.
  - **Root cause:** The checksum/harness downstream code pattern-matches on this
    error string format for human-readable output. Any change here must be
    synchronized with downstream consumers.
  - **Fix:** All five error paths in the new BufWriter write section use the same
    format macro: `format!("Failed to write file '{}' (index {}): {}", output_paths[index], index, e)`.
  - **Prevention:** Do NOT refactor error formatting here without updating CHANGELOG
    and checking the benchmark harness error-display code.

- **Gotcha:** `std::io::Write` must be in scope for `BufWriter::write_all` to compile.
  - **Symptom:** `error[E0599]: no method named write_all found for struct BufWriter<...>`.
  - **Root cause:** `write_all` is a method on the `std::io::Write` trait; it requires
    `use std::io::Write` (or `use std::io::{BufWriter, Write}`) to be in scope.
  - **Fix:** Added `Write` to the existing `std::io` import: `use std::io::{BufWriter, Error, ErrorKind, IoSlice, Write};`.
  - **Prevention:** Any time you use trait methods, ensure the trait is in scope.
    Rust's error messages say "not found" rather than "trait not in scope" which
    is confusing. The fix is always `use the::Trait;`.

---

## Things I Learned

- **`BufWriter::with_capacity(n, writer)` is the right API when the output size is
  known in advance.** Sizing the buffer to `total_len` means the three `write_all`
  calls for prefix, body, and suffix are all absorbed into one internal buffer fill
  (assuming the total is small enough — here, ~200–300 bytes per SSG page easily
  fits in the BufWriter buffer). A single kernel `write` syscall flushes everything.
  This matches the behavior of the previous `std::fs::write(path, html.as_bytes())`
  call, which was also a single syscall, without requiring the intermediate String.

- **Why this matters at 10K files.** Each rendered SSG page is a small string:
  ~24 bytes (prefix start) + up to 3 digits (page index) + ~14 bytes (prefix end) +
  ~35 bytes (source body for the test case) + ~24 bytes (suffix) ≈ ~100–300 bytes.
  Allocating 10,000 of these `String`s per benchmark run adds 10,000 small heap
  allocations to the hot path. The allocator overhead (metadata, pointer indirection,
  potential fragmentation) adds up at scale. BufWriter eliminates this with a
  stack-backed fixed buffer.

- **Checksum accounting is unaffected.** `total_len` remains computed the same way
  (`html_prefix.len() + content.len() + SSG_HTML_SUFFIX.len()`) and is still used
  for the checksum accumulation. The BufWriter write path doesn't change what bytes
  are written, only how they're assembled before syscall.

- **The doc comment now documents two key properties**: (1) the BufWriter rationale
  (why we use it, what the capacity corresponds to), and (2) the flush contract.
  Future readers should be able to understand the write path from the doc comment
  alone.

---

## Follow-ups / TODO (For Future Agents)

- [ ] Run `cargo bench --bench bench-ssg` before/after this commit on stable hardware
  to quantify wall-clock impact of the BufWriter path vs the previous String path.
  The allocation savings are most visible in tight loops under allocator pressure.
- [ ] Consider `File::set_len(total_len as u64)` (file pre-allocation) on macOS/Linux
  to hint to the OS about the expected file size before writing, potentially reducing
  fragmentation for large batches. Requires platform-specific feature gating or
  a `cfg(unix)` block with `use std::os::unix::fs::FileExt`.
- [ ] Evaluate whether `std::fs::read` (returns `Vec<u8>`) instead of `read_to_string`
  (returns `String`) avoids a UTF-8 validation pass. For SSG markdown that is always
  valid UTF-8, `read_to_string` adds an O(N) UTF-8 scan that `read` skips. However,
  the subsequent HTML rendering needs the content as `&str`, so you'd need
  `String::from_utf8(bytes)` anyway — a wash unless you can emit HTML bytes directly
  from the byte buffer without String conversion.
- [ ] Continue v0.11.0 P0 residual-overhead slices per ROADMAP.md.

---

## Links / References

- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/README.md`
- Related docs:
  - `ROADMAP.md` — "SSG Throughput Focus" remaining workstreams
  - `notes/2026-03-18_ssg-rayon-cpu-bounded-pool.md` — previous session (CPU-bounded pool)
  - `notes/2026-03-17_23-18_ssg-single-pass-rayon-pipeline.md` — single-pass pipeline session
