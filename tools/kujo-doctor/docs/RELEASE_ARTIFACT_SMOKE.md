# Release-Artifact Smoke Procedure

Run Doctor from the same release-style `kujo` binary that will be distributed.
The command must not require a network connection or write into the target
repository.

```bash
./kujo doctor --json > doctor.json
python3 -c 'import json; r=json.load(open("doctor.json")); assert r["schema_version"] == "0.1.0"; assert "checks" in r'
./kujo doctor --list-profiles
```

On 2026-07-10, the local release binary at `target/release/kujo` passed this
JSON contract check with 15 checks. The focused Rust CLI contract tests and
workflow-pack registry/manifest tests also passed.

## Platform handoff

The release owner should run the same commands against each downloaded Linux,
macOS, and Windows artifact after publication. Record binary version, platform,
JSON output schema version, exit status, and any missing-tool diagnostics. Do
not include environment variables, paths containing private project names, or
other sensitive diagnostic data in public evidence.
