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

## HLP-011: typed environment configuration

Build configuration at the boundary with the existing typed accessors. Use
env_required for mandatory values, env_or for string defaults, and env_int,
env_float, or env_bool when a value must have a specific type.

    host := env_or("KUJO_HOST", "127.0.0.1")
    port := env_int("KUJO_PORT")
    enabled := env_bool("KUJO_ENABLED")
    token := env_required("KUJO_TOKEN")

Missing required values and invalid typed values are errors. Keep the
environment read in one configuration function, then pass the resulting
values into the rest of the program; do not repeat parsing in each command.
