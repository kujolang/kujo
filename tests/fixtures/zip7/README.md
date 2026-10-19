# ZIP 7 codec compatibility inputs

These archives were generated with the locked ZIP **7.2.0** library from Kujo
revision `3fb30796760ac40917b0ede63f8584184f0051b3`, before migration to ZIP 9.
Each contains `payload.dat`: bytes 0..255 repeated four times, exactly 1,024 bytes.
The codec is identified by the filename. They are fixed backward-reading inputs,
not regenerated with the candidate dependency during tests.

Generator: `scripts/zip_compat_fixture.rs`. With the old library built:

```sh
rustc --edition=2021 scripts/zip_compat_fixture.rs \
  --extern zip=target/debug/deps/libzip-4a94861031d9f418.rlib \
  -L dependency=target/debug/deps -o /tmp/kujo-zip7-fixtures
/tmp/kujo-zip7-fixtures tests/fixtures/zip7
```

The library hash is host-specific. Use the ZIP 7.2.0 artifact, not ZIP 9.
ZIP 7 cannot write Deflate64 or LZMA; their existing decoder features remain
configured, but these fixtures do not claim coverage of those two formats.
