use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn vscode_cursor_descriptor_points_to_kujo_lsp() {
    let path = root().join("docs/editor-adapters/vscode-cursor-settings.json");
    let content = fs::read_to_string(path).expect("failed to read vscode/cursor descriptor");

    assert!(content.contains("\"kujo\""));
    assert!(content.contains("\"lsp\""));
}

#[test]
fn neovim_descriptor_points_to_kujo_lsp() {
    let path = root().join("docs/editor-adapters/neovim-lspconfig.lua");
    let content = fs::read_to_string(path).expect("failed to read neovim descriptor");

    assert!(content.contains("'kujo'"));
    assert!(content.contains("'lsp'"));
}

#[test]
fn jetbrains_descriptor_points_to_kujo_lsp() {
    let path = root().join("docs/editor-adapters/jetbrains-lsp.md");
    let content = fs::read_to_string(path).expect("failed to read jetbrains descriptor");

    assert!(content.contains("`kujo`"));
    assert!(content.contains("`lsp`"));
}
