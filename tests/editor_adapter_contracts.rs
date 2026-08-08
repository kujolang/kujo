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

#[test]
fn helix_descriptor_points_to_kujo_lsp() {
    let path = root().join("docs/editor-adapters/helix-languages.toml");
    let content = fs::read_to_string(path).expect("failed to read Helix descriptor");

    assert!(content.contains("command = \"kujo\""));
    assert!(content.contains("args = [\"lsp\"]"));
    assert!(content.contains("language-id = \"kujo\""));
    assert!(content.contains("file-types = [\"kujo\"]"));
}

#[test]
fn emacs_eglot_descriptor_points_to_kujo_lsp() {
    let path = root().join("docs/editor-adapters/emacs-eglot.el");
    let content = fs::read_to_string(path).expect("failed to read Emacs Eglot descriptor");

    assert!(content.contains("define-derived-mode kujo-mode"));
    assert!(content.contains(":language-id \"kujo\""));
    assert!(content.contains("(\"kujo\" \"lsp\")"));
    assert!(content.contains("eglot-ensure"));
}
