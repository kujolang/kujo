use jsonschema::{Draft, JSONSchema};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(path: &Path) -> Value {
    let body = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&body)
        .unwrap_or_else(|error| panic!("invalid JSON in {}: {error}", path.display()))
}

fn schema_path(name: &str) -> PathBuf {
    root().join("schemas/workflow-control").join(format!("{name}.schema.json"))
}

fn fixture_path(name: &str) -> PathBuf {
    root().join("tests/fixtures/workflow_control").join(format!("{name}.json"))
}

fn compile_schema(name: &str) -> JSONSchema {
    let schema = read_json(&schema_path(name));
    JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema)
        .unwrap_or_else(|error| panic!("invalid JSON Schema {name}: {error}"))
}

#[test]
fn workflow_control_valid_fixtures_conform() {
    let cases = [
        ("evidence-ref-v1", "evidence-ref.valid"),
        ("execution-result-v1", "execution-result.valid"),
        ("evaluation-result-v1", "evaluation-result.valid"),
        ("policy-decision-v1", "policy-decision.valid"),
        ("preservation-outcome-v1", "preservation-outcome.valid"),
        ("intervention-request-v2", "intervention-request.valid"),
        ("intervention-decision-v2", "intervention-decision.valid"),
        ("reexecution-descriptor-v1", "reexecution-descriptor.valid"),
    ];

    for (schema_name, fixture_name) in cases {
        let compiled = compile_schema(schema_name);
        let fixture = read_json(&fixture_path(fixture_name));
        if let Err(errors) = compiled.validate(&fixture) {
            let rendered = errors.map(|error| error.to_string()).collect::<Vec<_>>();
            panic!("{fixture_name} failed {schema_name}: {rendered:?}");
        };
    }
}

#[test]
fn evaluation_result_cannot_issue_workflow_control_action() {
    let compiled = compile_schema("evaluation-result-v1");
    let fixture = read_json(&fixture_path("evaluation-result.invalid-control-action"));
    assert!(compiled.validate(&fixture).is_err());

    let schema = read_json(&schema_path("evaluation-result-v1"));
    let properties = schema["properties"].as_object().expect("properties object");
    for orchestration_field in ["action", "disposition", "pause", "retry", "continue"] {
        assert!(
            !properties.contains_key(orchestration_field),
            "evaluation result must describe judgment, not orchestration: {orchestration_field}"
        );
    }
}

#[test]
fn workflow_control_schema_ids_and_contract_constants_are_unique() {
    let schemas = [
        ("evidence-ref-v1", "kujo.evidence-ref/v1"),
        ("execution-result-v1", "kujo.execution-result/v1"),
        ("evaluation-result-v1", "kujo.evaluation-result/v1"),
        ("policy-decision-v1", "kujo.policy-decision/v1"),
        ("preservation-outcome-v1", "kujo.preservation-outcome/v1"),
        ("intervention-request-v2", "kujo.intervention-request/v2"),
        ("intervention-decision-v2", "kujo.intervention-decision/v2"),
        ("reexecution-descriptor-v1", "kujo.reexecution-descriptor/v1"),
    ];
    let mut ids = HashSet::new();
    let mut contracts = HashSet::new();

    for (name, expected_contract) in schemas {
        let schema = read_json(&schema_path(name));
        let id = schema["$id"].as_str().expect("schema id");
        let contract = schema["properties"]["schema"]["const"].as_str().expect("contract const");
        assert!(ids.insert(id.to_string()), "duplicate schema id {id}");
        assert!(contracts.insert(contract.to_string()), "duplicate contract {contract}");
        assert_eq!(contract, expected_contract);
    }
}

#[test]
fn control_contracts_keep_secrets_as_references() {
    let fixture = read_json(&fixture_path("reexecution-descriptor.valid"));
    let serialized = serde_json::to_string(&fixture).expect("serialize fixture");
    assert!(serialized.contains("secret://provider/api-key"));
    for forbidden in ["api_key_value", "bearer ", "private_key"] {
        assert!(!serialized.to_lowercase().contains(forbidden));
    }
}

#[test]
fn workflow_control_rejects_missing_fields_bad_versions_and_bounds() {
    for (schema, fixture) in [
        ("execution-result-v1", "execution-result.valid"),
        ("evaluation-result-v1", "evaluation-result.valid"),
        ("evidence-ref-v1", "evidence-ref.valid"),
        ("policy-decision-v1", "policy-decision.valid"),
        ("preservation-outcome-v1", "preservation-outcome.valid"),
        ("intervention-request-v2", "intervention-request.valid"),
        ("intervention-decision-v2", "intervention-decision.valid"),
        ("reexecution-descriptor-v1", "reexecution-descriptor.valid"),
    ] {
        let definition = read_json(&schema_path(schema));
        let compiled = compile_schema(schema);
        let original = read_json(&fixture_path(fixture));
        for field in definition["required"].as_array().unwrap() {
            let mut invalid = original.clone();
            invalid.as_object_mut().unwrap().remove(field.as_str().unwrap());
            assert!(!compiled.is_valid(&invalid), "{schema}: missing {field}");
        }
        let mut invalid = original.clone();
        invalid["schema"] = "kujo.unknown/v99".into();
        assert!(!compiled.is_valid(&invalid));
        let mut extended = original;
        extended["third_party_metadata"] = serde_json::json!({"fixture": true});
        assert!(compiled.is_valid(&extended), "additive compatibility: {schema}");
    }
}

#[test]
fn embedded_evidence_has_the_same_integrity_requirements_as_standalone() {
    let mut result = read_json(&fixture_path("evaluation-result.valid"));
    let mut evidence = read_json(&fixture_path("evidence-ref.valid"));
    evidence["integrity"] = serde_json::json!({"status": "sha256"});
    result["evidence_refs"] = serde_json::json!([evidence]);
    assert!(!compile_schema("evaluation-result-v1").is_valid(&result));
    result["evidence_refs"][0]["integrity"]["sha256"] = "not-a-hash".into();
    assert!(!compile_schema("evaluation-result-v1").is_valid(&result));
    result["evidence_refs"][0]["integrity"]["sha256"] = "a".repeat(64).into();
    assert!(compile_schema("evaluation-result-v1").is_valid(&result));
}

#[test]
fn producer_neutral_evaluations_preserve_error_and_rule_facts() {
    for producer in ["eval", "shipcheck", "fence", "third-party"] {
        let mut result = read_json(&fixture_path("evaluation-result.valid"));
        result["evaluator"]["name"] = producer.into();
        result["verdict"] = "indeterminate".into();
        result["evaluation_status"] = "error".into();
        result["rule_ids"] = serde_json::json!(["quality.fixture"]);
        assert!(compile_schema("evaluation-result-v1").is_valid(&result));
        for command in ["action", "disposition", "pause", "retry", "continue"] {
            let mut extended = result.clone();
            extended[command] = true.into();
            // V1 keeps its extension policy; control admission owns authority checks.
            assert!(compile_schema("evaluation-result-v1").is_valid(&extended));
        }
    }
}

#[test]
fn portable_control_event_is_bounded_and_versioned() {
    let mut event = serde_json::json!({
        "schema": "kujo.control-event/v1", "sequence": 1, "event_id": "event-1",
        "run_id": "run-1", "state_revision": 7, "kind": "policy_decided",
        "subject": {"step_id": "gate", "attempt_id": "attempt-1"},
        "refs": ["result-1"], "details": {}, "occurred_at": "2026-09-25T00:00:00Z"
    });
    let compiled = compile_schema("control-event-v1");
    assert!(compiled.is_valid(&event));
    event["sequence"] = 0.into();
    assert!(!compiled.is_valid(&event));
    event["sequence"] = 1.into();
    event["event_sha256"] = "bad".into();
    assert!(!compiled.is_valid(&event));
}

#[test]
fn error_verdict_and_reexecution_modes_have_explicit_semantics() {
    let mut evaluation = read_json(&fixture_path("evaluation-result.valid"));
    evaluation["evaluation_status"] = "error".into();
    evaluation["verdict"] = "fail".into();
    assert!(!compile_schema("evaluation-result-v1").is_valid(&evaluation));
    let mut descriptor = read_json(&fixture_path("reexecution-descriptor.valid"));
    descriptor["mode"] = "same_workspace".into();
    assert!(!compile_schema("reexecution-descriptor-v1").is_valid(&descriptor));
    descriptor["preservation_ref"] = "preservation-1".into();
    assert!(compile_schema("reexecution-descriptor-v1").is_valid(&descriptor));
    descriptor["mode"] = "prohibited".into();
    descriptor["effects"]["automatic_retry_allowed"] = true.into();
    assert!(!compile_schema("reexecution-descriptor-v1").is_valid(&descriptor));
}

#[test]
fn oversized_evidence_and_invalid_effect_enums_are_rejected() {
    let mut evaluation = read_json(&fixture_path("evaluation-result.valid"));
    evaluation["evidence"] = serde_json::json!(vec![serde_json::json!({"$ref": "ev"}); 1001]);
    assert!(!compile_schema("evaluation-result-v1").is_valid(&evaluation));
    let mut execution = read_json(&fixture_path("execution-result.valid"));
    execution["effects"] =
        serde_json::json!([{"effect_id": "e", "class": "magic_retry", "state": "unknown"}]);
    assert!(!compile_schema("execution-result-v1").is_valid(&execution));
}

#[test]
#[ignore = "requires the local Dispatch failure-gate golden-path handoff"]
fn actual_cross_component_failure_evidence_conforms() {
    let handoff_path = std::env::var("KUJO_FAILURE_GATE_HANDOFF").expect("handoff JSON path");
    let dispatch_root =
        std::env::var("KUJO_FAILURE_GATE_DISPATCH_ROOT").expect("Dispatch checkout");
    let handoff = read_json(Path::new(&handoff_path));
    let output = Path::new(&handoff_path).parent().unwrap();
    let workcell = PathBuf::from(handoff["workcell"]["output_dir"].as_str().unwrap());
    let validate = |schema: &str, value: &Value| {
        if let Err(errors) = compile_schema(schema).validate(value) {
            panic!("{schema}: {:?}", errors.map(|e| e.to_string()).collect::<Vec<_>>());
        }
    };
    for (schema, file) in [
        ("execution-result-v1", "execution-result.json"),
        ("preservation-outcome-v1", "preservation.json"),
        ("reexecution-descriptor-v1", "reexecution.json"),
    ] {
        validate(schema, &read_json(&workcell.join(file)));
    }
    for attempt in [1, 2] {
        let evaluation =
            read_json(&output.join(format!("evaluation-{attempt}/evaluation-result.json")));
        validate("evaluation-result-v1", &evaluation);
        for reference in evaluation["evidence_refs"].as_array().unwrap() {
            validate("evidence-ref-v1", reference);
        }
    }
    validate("intervention-decision-v2", &handoff["decision"]);
    let state = read_json(&Path::new(&dispatch_root).join(handoff["state_path"].as_str().unwrap()));
    validate("intervention-request-v2", &state["human_intervention"]);
    for decision in state["control_decisions"].as_array().unwrap() {
        validate("policy-decision-v1", decision);
    }
    let journal = fs::read_to_string(
        Path::new(&dispatch_root).join(handoff["journal_path"].as_str().unwrap()),
    )
    .unwrap();
    for line in journal.lines().filter(|line| !line.is_empty()) {
        validate("control-event-v1", &serde_json::from_str(line).unwrap());
    }
}
