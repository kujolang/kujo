use kujo::lsp_server::{LspServer, LspServerConfig};
use serde_json::json;
use std::time::{Duration, Instant};

fn test_uri() -> &'static str {
    "file:///tmp/reliability_track.kujo"
}

fn open_message(source: &str) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": test_uri(),
                "text": source
            }
        }
    })
}

fn change_message(source: &str) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": test_uri()
            },
            "contentChanges": [
                {
                    "text": source
                }
            ]
        }
    })
}

fn close_message() -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didClose",
        "params": {
            "textDocument": {
                "uri": test_uri()
            }
        }
    })
}

#[test]
fn malformed_sequences_and_lifecycle_churn_are_resilient() {
    let mut server = LspServer::new(LspServerConfig::default());

    // didChange before didOpen should be handled safely and produce diagnostics for the provided content.
    let responses = server.process_message(&change_message("let value := 1\n"));
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["method"], json!("textDocument/publishDiagnostics"));
    assert_eq!(server.open_document_count(), 1);

    let _ = server.process_message(&close_message());
    assert_eq!(server.open_document_count(), 0);

    let malformed_request = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "textDocument/completion",
        "params": {
            "position": {
                "line": 0,
                "character": 0
            }
        }
    });
    let responses = server.process_message(&malformed_request);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["error"]["code"], json!(-32602));

    // Lifecycle churn: repeated didOpen -> didChange -> didClose should leave no leaked documents.
    for iteration in 0..200 {
        let opened = server.process_message(&open_message("let value := 1\n"));
        assert_eq!(opened.len(), 1, "didOpen should emit diagnostics on iteration {iteration}");
        assert_eq!(server.open_document_count(), 1);

        let changed = server.process_message(&change_message("let value := 2\npri\n"));
        assert_eq!(changed.len(), 1, "didChange should emit diagnostics on iteration {iteration}");
        assert_eq!(server.open_document_count(), 1);

        let closed = server.process_message(&close_message());
        assert_eq!(
            closed.len(),
            1,
            "didClose should emit diagnostics clear on iteration {iteration}"
        );
        assert_eq!(closed[0]["method"], json!("textDocument/publishDiagnostics"));
        assert_eq!(server.open_document_count(), 0);
    }
}

#[test]
fn repeated_completion_and_diagnostics_requests_keep_document_state_bounded() {
    let mut server = LspServer::new(LspServerConfig::default());
    let source = "func total(x) {\n  return x\n}\nlet out := total(1)\nout\n";

    let _ = server.process_message(&open_message(source));
    assert_eq!(server.open_document_count(), 1);

    for _ in 0..1500 {
        let diagnostics_req = json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": test_uri()
                },
                "position": {
                    "line": 3,
                    "character": 14
                }
            }
        });
        let _ = server.process_message(&diagnostics_req);

        let completion_req = json!({
            "jsonrpc": "2.0",
            "id": 21,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": test_uri()
                },
                "position": {
                    "line": 4,
                    "character": 2
                }
            }
        });
        let _ = server.process_message(&completion_req);
    }

    // Repeated request loops must not retain extra document state.
    assert_eq!(server.open_document_count(), 1);

    let _ = server.process_message(&close_message());
    assert_eq!(server.open_document_count(), 0);
}

fn average_duration<F>(iterations: usize, mut operation: F) -> Duration
where
    F: FnMut(),
{
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }

    let total = start.elapsed();
    let average_nanos = total.as_nanos() / (iterations as u128);
    Duration::from_nanos(average_nanos.min(u128::from(u64::MAX)) as u64)
}

#[test]
fn startup_and_first_response_latency_stay_within_guardrails() {
    let source = "func ping(value) {\n  return value\n}\nlet out := ping(1)\npi\n";

    let startup_avg = average_duration(30, || {
        let _ = LspServer::new(LspServerConfig::default());
    });

    let first_completion_avg = average_duration(30, || {
        let mut server = LspServer::new(LspServerConfig::default());
        let _ = server.process_message(&open_message(source));
        let completion = json!({
            "jsonrpc": "2.0",
            "id": 55,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": test_uri()
                },
                "position": {
                    "line": 3,
                    "character": 2
                }
            }
        });
        let _ = server.process_message(&completion);
    });

    assert!(
        startup_avg.as_millis() < 20,
        "startup average latency exceeded guardrail: {startup_avg:?}"
    );
    assert!(
        first_completion_avg.as_millis() < 80,
        "first-response average latency exceeded guardrail: {first_completion_avg:?}"
    );
}
