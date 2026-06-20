use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &str) -> String {
    fs::read_to_string(root().join(path))
        .unwrap_or_else(|err| panic!("failed to read {path}: {err}"))
}

#[test]
fn editor_launch_matrix_names_all_v1_editor_families() {
    let baselines = read("docs/EDITOR_ADAPTER_BASELINES.md");
    let install = read("docs/INSTALLATION_LSP_EDITORS.md");

    for marker in [
        "## v1.0 Launch Matrix",
        "VS Code / Cursor / VS Code-compatible forks",
        "Neovim",
        "JetBrains",
        "Generic LSP clients",
        "kujo lsp",
        "tools/lsp_smoke_clients/python_client.py",
        "tools/lsp_smoke_clients/node_client.mjs",
        "tests/editor_launch_matrix_contract.rs",
    ] {
        assert!(
            baselines.contains(marker),
            "editor adapter baselines should include marker {marker:?}"
        );
    }

    for marker in [
        "## v1.0 Editor Launch Matrix",
        "VS Code",
        "Cursor / VS Code-compatible forks",
        "Neovim",
        "JetBrains",
        "Generic LSP clients",
        "cargo test --test lsp_external_clients_smoke",
        "cargo test --test lsp_conformance_harness",
        "cargo test --test lsp_reliability_track",
        "cargo test --test lsp_latency_guardrails",
        "cargo test --test tree_sitter_kujo_assets",
    ] {
        assert!(
            install.contains(marker),
            "editor/LSP install doc should include marker {marker:?}"
        );
    }
}

#[test]
fn adapter_descriptors_and_extension_keep_canonical_lsp_command() {
    let vscode_settings = read("docs/editor-adapters/vscode-cursor-settings.json");
    let neovim = read("docs/editor-adapters/neovim-lspconfig.lua");
    let jetbrains = read("docs/editor-adapters/jetbrains-lsp.md");
    let extension_manifest = read("tools/vscode-kujo-extension/package.json");

    assert!(
        vscode_settings.contains("\"kujo\"") && vscode_settings.contains("\"lsp\""),
        "VS Code/Cursor settings should launch kujo lsp"
    );
    assert!(
        neovim.contains("cmd = { 'kujo', 'lsp' }") && neovim.contains("filetypes = { 'kujo' }"),
        "Neovim descriptor should launch kujo lsp for Kujo files"
    );
    assert!(
        jetbrains.contains("executable: `kujo`")
            && jetbrains.contains("args: `lsp`")
            && jetbrains.contains("generic LSP configuration path"),
        "JetBrains doc should use the generic kujo lsp adapter path"
    );
    assert!(
        extension_manifest.contains("\"kujo.lsp.command\"")
            && extension_manifest.contains("\"onLanguage:kujo\"")
            && extension_manifest.contains("\"kujo\"")
            && extension_manifest.contains("\"lsp\"")
            && extension_manifest.contains("\".kujo\""),
        "VS Code extension manifest should register .kujo and kujo lsp command settings"
    );
}
