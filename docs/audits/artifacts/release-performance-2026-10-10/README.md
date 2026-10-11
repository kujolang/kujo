# Release performance evidence

See the [campaign audit](../../release-performance-campaign.md) for scope,
compatibility, verification and limitations; [measurements](measurements.md)
provides every workload's medians and A/A context. These are internal regression
observations, not universal speed claims.

Runtime baseline: `42269d9b3f924736a2f995e8334ec2e7025cc1d2`.
Runtime candidate: `f4abf5dd265d5b866a2cfc99d82344c3a8ebbd5f`.
Subsequent changes affect harness cleanup, testing and documentation only.

Archives preserve both the A/A control and paired comparison, with raw samples,
summary statistics, runtime/resource reports, workload sources, source/binary
hashes and host/compiler metadata. Each runtime series covers 21 families with
2 warmups and 11 measured observations per mode; static series cover 3 families
with 2 warmups and 21 measured observations per binary. No measured outlier is
discarded. `SHA256SUMS` identifies the reviewed archives.

- `linux-x64.zip`: original successful Linux artifact from GitHub run
  [38091114929](https://github.com/kujolang/kujo/actions/runs/38091114929), artifact
  `11684826212`. That run's original Windows build and arm64 calibration failed;
  their failures do not invalidate its separately successful Linux job.
- `local-macos-x64.zip`: local `baseline-aa-final` and `paired-final` series,
  plus the supplemental late-Unicode-prefix source, runner and raw observations.
- `macos-arm64.zip`: original successful hosted arm64 artifact from
  [38094214685](https://github.com/kujolang/kujo/actions/runs/38094214685), artifact
  `11686028410`, with both complete series under the calibrated budget.
- `windows-x64.zip`: successful Windows run
  [38092543276](https://github.com/kujolang/kujo/actions/runs/38092543276), artifact
  `11685613769`. Four regenerable benchmark-runner PDBs are excluded. The two
  `omitted-runner-symbols.json` files preserve their sizes/hashes and original
  archive identity; all performance evidence and build logs are unchanged.
- `failed-calibration.json.gz`: explicitly failed local/arm64 deadline calibration
  fragments and the initial Windows dependency-build failure. Partial runtime
  output is retained as text, not relabeled as successful JSON or mixed into
  accepted measurement tables.

## Review procedure

1. Verify `SHA256SUMS` (for example, `shasum -a 256 -c SHA256SUMS`).
2. Extract archives into a temporary directory. Verify each retained file against
   its evidence folder's original `sha256.json`. Empty logs may be omitted by
   GitHub upload. The only deliberately omitted nonempty files are the Windows
   PDBs named and hashed in `omitted-runner-symbols.json`.
3. Compare runner source with the recorded corpus commit. Windows CRLF differs
   from Unix LF at the byte level; preserve raw hashes and compare line endings
   explicitly. Generated workload strings and iteration counts match exactly.
4. Recompute statistics with the exported `summarize(runtime, static)` function
   in `scripts/release_performance_campaign.js`; compare with `summary.json`.
   Every accepted summary in this folder was reconstructed and checked this way.

The corpus dirty flag is retained faithfully: generated output directories and
local audit edits can make it true. The exact runner/driver hashes and recorded
runtime source commits distinguish that state from changes to measured code.
Unix CPU/RSS comes from separate instrumented series. Windows unavailable values
remain null; peak RSS is not retained heap. Build/cache state is not comparable
across hosts, and no build-time reduction is claimed.
