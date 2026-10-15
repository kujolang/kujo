#![cfg(feature = "runtime-network")]
use std::{fs, process::Command, sync::Arc, thread};

#[test]
fn ai_redirects_respect_origin_and_path_allowlists() {
    for interpreter in [false, true] {
        for cross_origin in [false, true] {
            for (status, structured) in [(307, false), (308, true)] {
                for allowed in [Some(false), Some(true), None] {
                    let source = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
                    let source_url = format!("http://{}", source.server_addr());
                    let destination = cross_origin
                        .then(|| Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap()));
                    let target_url = destination
                        .as_ref()
                        .map(|s| format!("http://{}", s.server_addr()))
                        .unwrap_or_else(|| source_url.clone());
                    let target = format!("{target_url}/outside");
                    let allowlist = if allowed != Some(false) {
                        format!("{source_url}/v1,{target}")
                    } else {
                        format!("{source_url}/v1")
                    };
                    let worker_source = Arc::clone(&source);
                    let worker_destination = destination.clone();
                    let worker = thread::spawn(move || {
                        let request = worker_source.recv().expect("initial AI request");
                        request
                            .respond(tiny_http::Response::empty(status).with_header(
                                tiny_http::Header::from_bytes("Location", target).unwrap(),
                            ))
                            .unwrap();
                        let receiver = worker_destination.as_ref().unwrap_or(&worker_source);
                        if let Ok(mut request) = receiver.recv() {
                            let mut body = String::new();
                            request.as_reader().read_to_string(&mut body).unwrap();
                            request
                                .respond(tiny_http::Response::from_string(
                                    r#"{"choices":[{"message":{"content":"ok"}}]}"#,
                                ))
                                .unwrap();
                            Some(body)
                        } else {
                            None
                        }
                    });
                    let dir = tempfile::tempdir().unwrap();
                    let path = dir.path().join("main.kujo");
                    let output_statement = if structured && allowed == Some(false) {
                        "match result { case Err(error): { print(to_json(error)) } case Ok(value): { print(\"unexpected success\") } }"
                    } else {
                        "print(result)"
                    };
                    fs::write(&path, format!("let result := ai_chat(\"private-prompt-marker\", {{\"endpoint\":\"{source_url}/v1/chat\",\"model\":\"mock\",\"structured_errors\":{structured}}})\n{output_statement}\n")).unwrap();
                    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
                    command
                        .arg("run")
                        .arg(path)
                        .env("KUJO_AI_ALLOWED_ENDPOINTS", allowlist)
                        .env("KUJO_NET_DESTINATION_POLICY", "allow_all")
                        .env_remove("KUJO_AI_REPLAY")
                        .env_remove("KUJO_AI_RECORD")
                        .env("NO_PROXY", "*");
                    if allowed.is_none() {
                        command.env_remove("KUJO_AI_ALLOWED_ENDPOINTS");
                    }
                    if interpreter {
                        command.arg("--interpreter");
                    }
                    let output = command.output().unwrap();
                    source.unblock();
                    if let Some(destination) = &destination {
                        destination.unblock();
                    }
                    let forwarded = worker.join().unwrap();
                    assert!(output.status.success(), "{output:?}");
                    if allowed != Some(false) {
                        assert!(forwarded.unwrap().contains("private-prompt-marker"));
                        assert!(String::from_utf8_lossy(&output.stdout).contains("ok"));
                    } else {
                        if structured {
                            let result: serde_json::Value =
                                serde_json::from_slice(&output.stdout).unwrap();
                            assert_eq!(result["kind"], "endpoint_denied");
                        }
                        assert!(forwarded.is_none(), "request body escaped allowlist");
                        assert!(
                            String::from_utf8_lossy(&output.stdout).contains("endpoint_denied"),
                            "{output:?}"
                        );
                    }
                }
            }
        }
    }
}
