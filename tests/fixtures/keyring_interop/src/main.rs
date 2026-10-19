#[path = "../../../../src/agent_project/credential_store.rs"]
mod credential_store;

fn main() {
    const SERVICE: &str = "ai.kujo.agent";
    let user = format!("KUJO_INTEROP_{}", std::process::id());
    keyring_core::set_default_store(zbus_secret_service_keyring_store::Store::new().unwrap());
    let legacy = keyring_core::Entry::new(SERVICE, &user).unwrap();
    legacy.set_password("legacy-test-secret").unwrap();
    let current = credential_store::entry(SERVICE, &user).unwrap();
    assert_eq!(current.get_password().unwrap(), "legacy-test-secret");
    let attrs = current.get_attributes().unwrap();
    assert_eq!(attrs.get("service").unwrap(), SERVICE);
    assert_eq!(attrs.get("username").unwrap(), &user);
    current.set_password("updated-test-secret").unwrap();
    assert_eq!(legacy.get_password().unwrap(), "updated-test-secret");
    current.delete_credential().unwrap();
    assert!(matches!(legacy.get_password(), Err(keyring_core::Error::NoEntry)));

    let fresh_user = format!("{user}_NEW");
    let fresh = credential_store::entry(SERVICE, &fresh_user).unwrap();
    fresh.set_password("new-test-secret").unwrap();
    keyring_core::set_default_store(zbus_secret_service_keyring_store::Store::new().unwrap());
    let old_reader = keyring_core::Entry::new(SERVICE, &fresh_user).unwrap();
    assert_eq!(old_reader.get_password().unwrap(), "new-test-secret");
    old_reader.delete_credential().unwrap();
    assert!(matches!(fresh.get_password(), Err(keyring_core::Error::NoEntry)));
    println!(
        "Secret Service old/new read, write, identity, delete and NoEntry compatibility passed"
    );
}
