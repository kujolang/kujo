//! Platform selection for the private agent credential store. Keep the same
//! native stores and Secret Service identity as keyring's former v1 wrapper.
use std::sync::LazyLock;

static STORE: LazyLock<keyring_core::Result<()>> = LazyLock::new(initialize);

pub(super) fn entry(service: &str, username: &str) -> keyring_core::Result<keyring_core::Entry> {
    if STORE.is_err() {
        return Err(keyring_core::Error::NoDefaultStore);
    }
    keyring_core::Entry::new(service, username)
}

fn initialize() -> keyring_core::Result<()> {
    #[cfg(target_os = "macos")]
    let store = apple_native_keyring_store::keychain::Store::new()?;
    #[cfg(target_os = "windows")]
    let store = windows_native_keyring_store::Store::new()?;
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
    let store = dbus_secret_service_keyring_store::Store::new()?;
    #[cfg(all(any(unix, windows), not(any(target_os = "ios", target_os = "android"))))]
    {
        keyring_core::set_default_store(store);
        Ok(())
    }
    #[cfg(not(all(any(unix, windows), not(any(target_os = "ios", target_os = "android")))))]
    Err(keyring_core::Error::Invalid(
        "platform".into(),
        "must be macOS, Windows, or a non-iOS, non-Android *nix variant".into(),
    ))
}
