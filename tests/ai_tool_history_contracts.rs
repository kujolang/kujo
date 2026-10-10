#[path = "support/native_values.rs"]
mod native_values;
use kujo::interpreter::Value;
use native_values::{array, call, json};
use std::sync::Arc;

#[test]
fn ai_dictionary_literals_and_multimodal_blocks_work_in_both_runtimes() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("history.kujo");
    std::fs::write(&source, r#"
        let options := {"model":"mock","endpoint":"https://example.test/v1/chat/completions"}
        let a := ai_request_hash([{"role":"tool","content":"result","tool_call_id":"a"}], options)
        let b := ai_request_hash([{"role":"tool","content":"result","tool_call_id":"b"}], options)
        print(a != b)
        let c := ai_request_hash([{"role":"assistant","content":null,"tool_calls":[{"id":"a","type":"function","function":{"name":"lookup","arguments":"{}"}}]}], options)
        print(len(c) == 64)
        let d := ai_request_hash([{"role":"user","content":[{"type":"text","text":"hi"},{"type":"image_url","image_url":{"url":"https://example.test/image.png","detail":"low"}}]}], options)
        print(len(d) == 64)
    "#).unwrap();
    for interpreter in [false, true] {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(&source);
        if interpreter {
            command.arg("--interpreter");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "interpreter={interpreter}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "true\ntrue\ntrue");
    }
}

#[test]
fn ai_request_hash_preserves_tool_call_linkage() {
    let options = json(r#"{"model":"mock","endpoint":"https://example.test/v1/chat/completions"}"#);
    let left = json(r#"[{"role":"tool","content":"result","tool_call_id":"call_a"}]"#);
    let right = json(r#"[{"role":"tool","content":"result","tool_call_id":"call_b"}]"#);
    let a = call("ai_request_hash", &[left, options.clone()]);
    let b = call("ai_request_hash", &[right, options]);
    assert!(matches!((&a, &b), (Value::Str(_), Value::Str(_))));
    assert!(!Value::equals(&a, &b), "different tool replies collided: {a:?}");
}

#[test]
fn ai_history_accepts_null_tool_turns_and_rejects_malformed_calls() {
    let options = json(r#"{"model":"mock","endpoint":"https://example.test/v1/chat/completions"}"#);
    let good = json(
        r#"[{"role":"assistant","content":null,"tool_calls":[{"id":"a","type":"function","function":{"name":"lookup","arguments":"{}"}}]}]"#,
    );
    assert!(matches!(call("ai_request_hash", &[good, options.clone()]), Value::Str(_)));
    for calls in [
        r#"[{"function":{"name":"lookup","arguments":"{}"}}]"#,
        r#"[{"id":"a","function":{"name":"lookup","arguments":{}}}]"#,
        r#"[{"id":"a","function":{"name":"lookup","arguments":"{}"}},{"id":"a","function":{"name":"lookup","arguments":"{}"}}]"#,
    ] {
        let input = json(&format!(r#"[{{"role":"assistant","content":"","tool_calls":{calls}}}]"#));
        assert!(
            matches!(call("ai_request_hash", &[input, options.clone()]), Value::Error(s) if s.contains("tool_calls"))
        );
    }
}

#[test]
fn ai_tool_loop_replays_a_protocol_valid_two_step_conversation() {
    use serde_json::json;
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let endpoint = "http://127.0.0.1:1/v1/chat/completions";
    let assistant = json!({"role":"assistant", "content":"", "tool_calls":[
        {"id":"call_a","type":"function","function":{"name":"lookup","arguments":"{\"q\":1}"}},
        {"id":"call_b","type":"function","function":{"name":"lookup","arguments":"{\"q\":2}"}}
    ]});
    let initial = json!([{"role":"user","content":"lookup twice"}]);
    let continued = json!([
        {"role":"user","content":"lookup twice"}, assistant.clone(),
        {"role":"tool","name":"lookup","content":"found","tool_call_id":"call_a"},
        {"role":"tool","name":"lookup","content":"found","tool_call_id":"call_b"}
    ]);
    for (messages, response) in [
        (initial, json!({"choices":[{"message":assistant}]})),
        (continued.clone(), json!({"choices":[{"message":{"role":"assistant","content":"done"}}]})),
    ] {
        let normalized = json!({"_hash_version":1,"endpoint":endpoint,"model":"mock","headers":[],"body":{"model":"mock","messages":messages,"stream":false}});
        let key = format!("{:x}", Sha256::digest(serde_json::to_vec(&normalized).unwrap()));
        let cassette = json!({"_cassette_version":1,"request_meta":{"hash":key,"surface":"ai_tool_loop","endpoint":endpoint,"model":"mock","normalized":normalized},"response":{"status":200,"headers":{},"body":response.to_string()}});
        std::fs::write(dir.path().join(format!("{key}.json")), cassette.to_string()).unwrap();
    }
    let options = json(&json!({"model":"mock","endpoint":endpoint,"max_steps":2,"tool_results":{"lookup":"found"},"cassette":{"mode":"replay","dir":dir.path().to_str().unwrap()}}).to_string());
    let result = call("ai_tool_loop", &[Value::Str(Arc::new("lookup twice".into())), options]);
    let Value::Result { is_ok: true, value } = result else { panic!("{result:?}") };
    let Value::Dict(result) = value.as_ref() else { panic!("expected dict") };
    assert!(matches!(result.get("steps"), Some(Value::Int(2))));
    assert!(matches!(result.get("message"), Some(Value::Str(s)) if s.as_str() == "done"));
    let Value::Array(messages) = result.get("messages").unwrap() else {
        panic!("expected messages")
    };
    assert_eq!(messages.len(), 5);
    assert!(Value::equals(&array(messages[..4].to_vec()), &json(&continued.to_string())));
}
