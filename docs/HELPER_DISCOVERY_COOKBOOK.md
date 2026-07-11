# Helper discovery cookbook

This cookbook maps recurring wrapper patterns to the canonical Kujo APIs.
Use the native helper before introducing a local wrapper with the same
semantics.

## HLP-007: canonical text and time helpers

Use slugify for stable URL-safe labels, pad_right for aligned output, now_utc
for an ISO-8601 UTC timestamp, and format_date for formatting a numeric Unix
timestamp.

    slug := slugify("Release Candidate 1")
    label := pad_right(slug, 24, " ")
    timestamp := now_utc()
    day := format_date(current_timestamp(), "YYYY-MM-DD")

These helpers have fixed behavior and are already part of the standard
library contract. Do not add local slug, padding, or timestamp wrappers unless
the caller intentionally needs a different domain policy.
