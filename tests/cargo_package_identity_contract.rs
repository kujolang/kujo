use std::fs;

#[test]
fn cargo_registry_identity_preserves_public_kujo_targets() {
    let manifest = fs::read_to_string("Cargo.toml").expect("Cargo.toml should be readable");

    assert!(
        manifest.starts_with("[package]\nname = \"kujolang\"\n"),
        "the crates.io package must use the unambiguous kujolang name"
    );
    assert!(
        manifest.contains("[lib]\nname = \"kujo\"\npath = \"src/lib.rs\""),
        "the public Rust library crate must remain kujo"
    );
    assert!(
        manifest.contains("[[bin]]\nname = \"kujo\"\npath = \"src/main.rs\""),
        "cargo install kujolang must continue to install the kujo executable"
    );
}
