# Local source readiness evidence

Source `c1cc06ff67e6b8dc1419e5b91adc259891090aad`; results.json records exact commands and exits.
The standalone failing program is retained as runtime-return-probe.kujo.txt to avoid
automatic fixture discovery treating an intentionally unfixed repro as a normal test.
Copy it to a temporary .kujo path before running. The original evidence-sha256.txt
uses its original runtime-return-probe.kujo name; bytes are unchanged.

The source review preceded final adopter results; its conditional recommendation
is resolved by the final docs/KUJO_1_6_READINESS_REVIEW.md. Hosted CI, other platforms
and release artifacts were not validated. Green canonical tests do not cover the
separately reproduced VM defect.

Raw logs are losslessly archived as `<original-name>.gz` (deterministic gzip, mtime 0).
Use `gzip -dc <file>.log.gz` to inspect; results.json and evidence-sha256.txt
name and hash the original uncompressed bytes. No whitespace was changed.
