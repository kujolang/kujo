# Secret Service migration compatibility

This Linux-only fixture runs the production credential-store selector against a
fresh D-Bus session and GNOME Keyring. It verifies old-to-new and new-to-old
password access, updates, deletion, `NoEntry`, and service/username attributes.

The legacy zbus backend is **test-only**. Its unmaintained transitive dependencies
are deliberately confined to this separate fixture lockfile so we can prove
compatibility with already-stored credentials; they are not runtime dependencies.
All credentials used here are synthetic and removed on success. Use an isolated
session, never a user's normal login keyring.

```sh
cargo build --locked --manifest-path tests/fixtures/keyring_interop/Cargo.toml
dbus-run-session -- sh -c '
  printf test-only | gnome-keyring-daemon --unlock --components=secrets >/dev/null
  tests/fixtures/keyring_interop/target/debug/kujo-keyring-interop
'
```

Set `CARGO_TARGET_DIR` and adjust the executable path together when needed.
