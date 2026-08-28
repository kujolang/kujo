# Security analysis

Context minimization is security-sensitive. Required policy, capability, approval, redaction, and provenance data must never disappear through truncation or stale caching.

Existing defenses include explicit Kujo capability flags and AI egress controls (`docs/AI_RUNTIME.md:103-127`), secret wrappers/redaction (`docs/AI_RUNTIME.md:117-128`), Scent pattern redaction and bounded selection (`scent/README.md`), PackWrite secret-path exclusion (`packwrite/src/repo_context.kujo:11-37`), and Dispatch path/config limits (`dispatch/src/workflows/loader.kujo:47-50,221-248`).

Threats to test in future work: prompt injection in README/docs, malicious tool descriptions, poisoned cached state, forged headings/status, branch/path/control-character injection, stale hashes, schema confusion, and a truncated safety rule that still appears complete. Structured JSON fields, hashes, explicit trust labels, control-character sanitization, and fail-closed missing-context behavior are preferred over regex-only filtering.

Do not optimize away raw evidence storage, approval gates, redaction reports, deterministic replay, or security findings. Provide compact model views as references to those artifacts.
