use kujo::interpreter::Interpreter;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
struct InventoryRow {
    function: String,
    signature: String,
    arity: String,
    return_type: String,
    errors: String,
    capability: String,
    example: String,
}

fn parse_inventory_rows(content: &str) -> Vec<InventoryRow> {
    let mut rows = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("| `") {
            continue;
        }

        let columns: Vec<&str> = trimmed.split('|').collect();
        if columns.len() != 9 {
            continue;
        }

        let function = columns[1].trim().trim_matches('`').to_string();
        let signature = columns[2].trim().trim_matches('`').to_string();
        let arity = columns[3].trim().to_string();
        let return_type = columns[4].trim().to_string();
        let errors = columns[5].trim().to_string();
        let capability = columns[6].trim().trim_matches('`').to_string();
        let example = columns[7].trim().trim_matches('`').to_string();

        if function.is_empty() {
            continue;
        }

        rows.push(InventoryRow {
            function,
            signature,
            arity,
            return_type,
            errors,
            capability,
            example,
        });
    }

    rows
}

fn expected_arity_label(function_name: &str) -> String {
    match Interpreter::native_function_arity(function_name) {
        Some(arity) => match arity.max_args {
            Some(max_args) if arity.min_args == max_args => format!("exact {}", arity.min_args),
            Some(max_args) => format!("{}..={}", arity.min_args, max_args),
            None => format!("variadic ({}+)", arity.min_args),
        },
        None => "handler-defined".to_string(),
    }
}

#[test]
fn stdlib_inventory_documents_all_runtime_builtins_exactly_once() {
    let inventory_path = Path::new("docs/STANDARD_LIBRARY.md");
    let inventory =
        fs::read_to_string(inventory_path).expect("failed to read standard library inventory");
    let rows = parse_inventory_rows(&inventory);

    assert!(
        rows.len() >= 250,
        "expected broad builtin coverage in docs, got {} entries",
        rows.len()
    );

    let runtime: Vec<String> =
        Interpreter::get_builtin_names().into_iter().map(|name| name.to_string()).collect();
    let runtime_set: HashSet<String> = runtime.iter().cloned().collect();

    let mut documented_counts: HashMap<String, usize> = HashMap::new();
    for row in &rows {
        *documented_counts.entry(row.function.clone()).or_insert(0) += 1;
    }

    let duplicates: Vec<String> = documented_counts
        .iter()
        .filter_map(|(name, count)| if *count > 1 { Some(name.clone()) } else { None })
        .collect();
    assert!(duplicates.is_empty(), "duplicate documented builtins found: {:?}", duplicates);

    for function_name in &runtime {
        assert!(
            documented_counts.contains_key(function_name),
            "runtime builtin '{}' is missing from docs/STANDARD_LIBRARY.md",
            function_name
        );
    }

    for documented_name in documented_counts.keys() {
        assert!(
            runtime_set.contains(documented_name),
            "documented builtin '{}' is not registered by runtime",
            documented_name
        );
    }
}

#[test]
fn stdlib_inventory_rows_include_required_contract_fields() {
    let inventory = fs::read_to_string("docs/STANDARD_LIBRARY.md")
        .expect("failed to read standard library inventory");
    let rows = parse_inventory_rows(&inventory);

    for row in &rows {
        assert!(!row.signature.is_empty(), "missing signature for builtin '{}'", row.function);
        assert!(!row.arity.is_empty(), "missing arity for builtin '{}'", row.function);
        assert!(!row.return_type.is_empty(), "missing return type for builtin '{}'", row.function);
        assert!(!row.errors.is_empty(), "missing errors column for builtin '{}'", row.function);
        assert!(!row.capability.is_empty(), "missing capability for builtin '{}'", row.function);
        assert!(!row.example.is_empty(), "missing example for builtin '{}'", row.function);
    }
}

#[test]
fn stdlib_inventory_capability_column_matches_runtime_policy() {
    let inventory = fs::read_to_string("docs/STANDARD_LIBRARY.md")
        .expect("failed to read standard library inventory");
    let rows = parse_inventory_rows(&inventory);

    let allowed_capability_values: HashSet<&str> = HashSet::from([
        "none",
        "filesystem-read",
        "filesystem-write",
        "filesystem-delete",
        "process-exec",
        "shell-exec",
        "env-read",
        "env-write",
        "network-client",
        "network-ai",
        "network-server",
        "database",
        "clock",
        "random",
    ]);

    for row in &rows {
        assert!(
            allowed_capability_values.contains(row.capability.as_str()),
            "unknown capability '{}' documented for '{}'",
            row.capability,
            row.function
        );

        let expected = Interpreter::native_function_capability(&row.function)
            .map(|capability| capability.as_str().to_string())
            .unwrap_or_else(|| "none".to_string());

        assert_eq!(row.capability, expected, "capability mismatch for builtin '{}'", row.function);
    }
}

#[test]
fn stdlib_inventory_arity_column_matches_centralized_arity_metadata() {
    let inventory = fs::read_to_string("docs/STANDARD_LIBRARY.md")
        .expect("failed to read standard library inventory");
    let rows = parse_inventory_rows(&inventory);

    for row in &rows {
        let expected = expected_arity_label(&row.function);
        assert_eq!(row.arity, expected, "arity mismatch for builtin '{}'", row.function);
    }
}

#[test]
fn stdlib_inventory_alias_rows_match_canonical_runtime_contracts() {
    let inventory = fs::read_to_string("docs/STANDARD_LIBRARY.md")
        .expect("failed to read standard library inventory");
    let rows = parse_inventory_rows(&inventory);
    let by_name: HashMap<String, InventoryRow> =
        rows.into_iter().map(|row| (row.function.clone(), row)).collect();

    for alias_name in ["println", "str", "time"] {
        let row = by_name
            .get(alias_name)
            .unwrap_or_else(|| panic!("missing alias row '{}' from inventory", alias_name));
        assert_eq!(row.arity, expected_arity_label(alias_name));

        let expected_capability = Interpreter::native_function_capability(alias_name)
            .map(|capability| capability.as_str().to_string())
            .unwrap_or_else(|| "none".to_string());
        assert_eq!(row.capability, expected_capability);
    }
}
