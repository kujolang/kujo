# Kujo patch notes

This is tiny_http 0.12.0 from crates.io. Kujo vendors it to apply a socket read timeout before tiny_http parses request headers or bodies and to treat the Unix `WouldBlock` timeout result as HTTP 408. The public upstream behavior remains the default unless `Server::http_with_read_timeout` or `Server::from_listener_with_read_timeout` is used.


## Registry identity

The Kujo-maintained fork is packaged as `kujolang-tiny-http` version
`0.12.0-kujo.1`, retaining the Rust library name `tiny_http` and the upstream
MIT/Apache-2.0 licenses and attribution. Its implementation is unchanged from
Kujo's reviewed vendored patch. This is not an upstream tiny_http release.

Kujo uses a versioned direct dependency with a local source path, rather than a
root `[patch.crates-io]` override. Cargo normalizes the path away for publication
but preserves the fork's registry identity and exact version. Publish and verify
this dependency before attempting the root crate's normalized dry run. Do not
replace it with upstream tiny_http or remove socket deadlines to make packaging
pass. The signed 1.7.0 release is unchanged; applying this dependency change to a
new runtime release requires a new signed version and the release gates.
