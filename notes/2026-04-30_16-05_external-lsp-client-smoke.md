# v0.13.0 External LSP Client Smoke Evidence

Date: 2026-04-30
Checklist target: Official Kujo LSP Server acceptance (external client launch)

## External clients

Two independent external clients were added and validated:

- Python client: `tools/lsp_smoke_clients/python_client.py`
- Node client: `tools/lsp_smoke_clients/node_client.mjs`

Both clients launch `kujo lsp` over stdio JSON-RPC and execute:

- `initialize`
- `initialized`
- `shutdown`
- `exit`

## Verification

Command:

- `cargo test --test lsp_external_clients_smoke`

Result:

- PASS
