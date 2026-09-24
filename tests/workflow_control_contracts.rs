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
