# Kujo Field Notes — CLI serve command for holistic local preview

**Date:** 2026-05-06
**Session:** 10:09 local
**Branch/Commit:** main / ad31700
**Scope:** Added a first-class `kujo serve` CLI command so local static preview is available to any Kujo user without project-specific scripts. Replaced `ssg` docs usage of a one-off preview script with the new shared command.

---

## What I Changed
- Added `Serve` subcommand to `src/main.rs`:
  - `kujo serve [dir] --host <host> --port <port> --index <file>`
- Implemented static file serving in CLI runtime using `tiny_http`:
  - canonical root resolution
  - GET-only request handling
  - safe path boundary check (`canonical_target.starts_with(root_dir)`)
  - index-file resolution for `/` and directory paths
  - extension-based content type header
- Updated Kujo CLI docs table in `README.md` to include `kujo serve [dir]`.
- Updated `ssg/README.md` preview section to use `kujo serve output --port 8080`.
- Removed `ssg/serve.kujo` to avoid maintaining a project-specific server implementation.

## Gotchas (Read This Next Time)
- **Gotcha:** `kujo run --interpreter` and `http_server(...).route(...)` behavior is not parity-complete for all server patterns.
  - **Symptom:** Earlier SSG preview script failed with `Unknown method: route` in interpreter mode.
  - **Root cause:** Method dispatch parity for HTTP server helper paths differs between runtime modes.
  - **Fix:** Move preview server responsibility to a CLI-native command (`kujo serve`) rather than script-level server code.
  - **Prevention:** For universal user workflows, prefer Kujo CLI subcommands over runtime-mode-sensitive script hacks.

- **Gotcha:** Security constraints must be explicit when serving arbitrary paths.
  - **Symptom:** Naive path joins allow traversal outside serve root.
  - **Root cause:** URL path segments can reference parent directories.
  - **Fix:** Canonicalize requested path and reject if it escapes root (`!starts_with(root_dir)`).
  - **Prevention:** Treat root-boundary checks as a non-optional invariant for any file-serving primitive.

## Things I Learned
- CLI-level primitives are the correct place for cross-project workflows that should “just work” for all Kujo users.
- Rule: if a capability is expected for every user (local preview/static serving), put it in `kujo <subcommand>` rather than embedding it in a single project script.
- Existing `tiny_http` dependency made this addition cheap and low-risk to introduce.

## Debug Notes (Only if applicable)
- **Failing test / error:** `Runtime Error: Unknown method: route` when launching preview script with `--interpreter`.
- **Repro steps:**
  - `cd /Users/robertdevore/Documents/Kujolang/kujo-repos/ssg`
  - `/Users/robertdevore/Documents/Kujolang/kujo-repos/kujo/target/release/kujo run ./serve.kujo --interpreter`
- **Breakpoints / logs used:** CLI help output + runtime smoke test with `curl`.
- **Final diagnosis:** The one-off script approach depended on runtime mode behavior; a CLI command is the stable holistic surface.

## Follow-ups / TODO (For Future Agents)
- [x] Added dedicated Rust tests for status code mapping, path traversal rejection, and MIME/security header behavior.
- [x] Extended static serving toward production-style behavior (ETag, range requests, precompressed assets, cache/security header controls).
- [ ] Consider optional SPA fallback flag (`--spa`) to map 404 paths to index file when needed.
- [ ] Consider explicit CLI toggle to disable cache headers entirely (`--no-cache`) for debugging-heavy preview workflows.

## Links / References
- Files touched:
  - `src/main.rs`
  - `README.md`
  - `../ssg/README.md`
  - `../ssg/serve.kujo`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
