# Hyphenation codec compatibility

Run from repository root:

```sh
CARGO_TARGET_DIR=target/hyphenation-compat cargo run --locked --manifest-path tests/fixtures/hyphenation_compat/Cargo.toml
CARGO_TARGET_DIR=target/hyphenation-compat cargo run --locked --manifest-path tests/fixtures/hyphenation_compat/Cargo.toml --features rebuild
```

The fixture compares complete decoded content for all 82 original standard and
extended dictionaries, through both readers and embedded resources. It also
checks encoded lengths, English hyphenation, language mismatch, truncation and
reader position with trailing bytes. The rebuild feature exercises build-time
serialization using the original pattern sources.

`baseline.json` was generated at Kujo revision
`69e390537612bd725519089724cab341f73720a5` with the registry hyphenation 0.8.4 and
bincode 1.3.3, before replacing the codec. Hashes are SHA-256 of serde_json values
with recursively sorted object keys; this avoids HashMap iteration order while
covering all decoded fields, FST bytes, exceptions, language and minima.

Capture program: `scripts/hyphenation_compat_baseline.rs`. The actual baseline
command (artifact hashes are local-build-specific) was:

```sh
rustc --edition=2021 scripts/hyphenation_compat_baseline.rs \
  --extern hyphenation=target/debug/deps/libhyphenation-de31f43aa761ed9c.rlib \
  --extern serde_json=target/debug/deps/libserde_json-0220c919b27b63ee.rlib \
  --extern sha2=target/debug/deps/libsha2-06cd5052eaefa770.rlib \
  -L dependency=target/debug/deps -o /tmp/kujo-hyphenation-baseline
/tmp/kujo-hyphenation-baseline PATH_TO_ORIGINAL_DICTIONARIES > baseline.json
```

The fixture optionally writes its re-encoded dictionaries to a directory given
as its first argument. During migration, the original baseline executable read
all those outputs and reproduced baseline.json exactly, proving both-direction
compatibility. The original codec is not retained as a test dependency.
