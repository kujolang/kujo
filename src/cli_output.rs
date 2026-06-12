use serde::Serialize;

/// Serialize any value as pretty JSON.
pub fn to_pretty_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(value)
}

/// Emit pretty JSON to stdout.
pub fn emit_pretty_json<T: Serialize>(value: &T) -> Result<(), serde_json::Error> {
    let serialized = to_pretty_json(value)?;
    println!("{}", serialized);
    Ok(())
}

/// Serialize a mapped list as pretty JSON.
pub fn to_json_array<T, U, F>(items: &[T], mut map: F) -> Result<String, serde_json::Error>
where
    U: Serialize,
    F: FnMut(&T) -> U,
{
    let rows: Vec<U> = items.iter().map(&mut map).collect();
    to_pretty_json(&rows)
}

/// Emit a mapped list as pretty JSON.
pub fn emit_json_array<T, U, F>(items: &[T], map: F) -> Result<(), serde_json::Error>
where
    U: Serialize,
    F: FnMut(&T) -> U,
{
    let serialized = to_json_array(items, map)?;
    println!("{}", serialized);
    Ok(())
}

/// Serialize an optional mapped record as pretty JSON, using `null` for no record.
pub fn to_optional_json_record<T, U, F>(
    value: Option<&T>,
    mut map: F,
) -> Result<String, serde_json::Error>
where
    U: Serialize,
    F: FnMut(&T) -> U,
{
    match value {
        Some(record) => to_pretty_json(&map(record)),
        None => to_pretty_json(&Option::<U>::None),
    }
}

/// Emit an optional mapped record as pretty JSON, using `null` for no record.
pub fn emit_optional_json_record<T, U, F>(
    value: Option<&T>,
    map: F,
) -> Result<(), serde_json::Error>
where
    U: Serialize,
    F: FnMut(&T) -> U,
{
    let serialized = to_optional_json_record(value, map)?;
    println!("{}", serialized);
    Ok(())
}

/// Render a tab-delimited row.
pub fn format_tsv_row<I, S>(columns: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    columns.into_iter().map(|column| column.as_ref().to_string()).collect::<Vec<_>>().join("\t")
}

/// Emit tab-delimited rows to stdout.
pub fn emit_tsv_rows<I, R, S>(rows: I)
where
    I: IntoIterator<Item = R>,
    R: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for row in rows {
        println!("{}", format_tsv_row(row));
    }
}

/// Render a section heading line.
pub fn format_section(title: &str) -> String {
    title.to_string()
}

/// Render a key/value line with a two-space indent.
pub fn format_kv(label: &str, value: impl std::fmt::Display) -> String {
    format!("  {}: {}", label, value)
}

/// Render a bullet line.
pub fn format_list_item(item: impl std::fmt::Display) -> String {
    format_list_item_with_prefix("-", item)
}

/// Render a list item line with a custom bullet prefix.
pub fn format_list_item_with_prefix(prefix: &str, item: impl std::fmt::Display) -> String {
    format!("  {} {}", prefix, item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug)]
    struct AlwaysFailSerialize;

    impl Serialize for AlwaysFailSerialize {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(serde::ser::Error::custom("forced serialization failure"))
        }
    }

    #[test]
    fn to_pretty_json_serializes_objects() {
        let value = serde_json::json!({"ok": true, "count": 2});
        let output = to_pretty_json(&value).expect("json serialization should succeed");
        assert!(output.contains("\"ok\": true"));
        assert!(output.contains("\"count\": 2"));
    }

    #[test]
    fn to_pretty_json_surfaces_serialization_errors() {
        let error = to_pretty_json(&AlwaysFailSerialize)
            .expect_err("serialization should fail intentionally");
        assert!(error.to_string().contains("forced serialization failure"));
    }

    #[test]
    fn to_json_array_maps_rows_and_surfaces_serialization_errors() {
        #[derive(Serialize)]
        struct Row {
            label: &'static str,
            kind: &'static str,
        }

        let output = to_json_array(&["print"], |label| Row { label: *label, kind: "function" })
            .expect("json array should serialize");
        assert!(output.contains("\"label\": \"print\""));
        assert!(output.contains("\"kind\": \"function\""));

        let values = [AlwaysFailSerialize];
        let error = to_json_array(&values, |value| *value)
            .expect_err("mapped row serialization should fail intentionally");
        assert!(error.to_string().contains("forced serialization failure"));
    }

    #[test]
    fn optional_json_record_serializes_record_or_null() {
        #[derive(Serialize)]
        struct Location {
            line: usize,
            column: usize,
        }

        let location = Location { line: 3, column: 14 };
        let record = to_optional_json_record(Some(&location), |location| Location {
            line: location.line,
            column: location.column,
        })
        .expect("optional record should serialize");
        assert!(record.contains("\"line\": 3"));
        assert!(record.contains("\"column\": 14"));

        let none = to_optional_json_record::<Location, Location, _>(None, |location| Location {
            line: location.line,
            column: location.column,
        })
        .expect("missing optional record should serialize as null");
        assert_eq!(none, "null");
    }

    #[test]
    fn format_helpers_render_stable_shapes() {
        assert_eq!(format_section("Summary"), "Summary");
        assert_eq!(format_kv("files", 12), "  files: 12");
        assert_eq!(format_list_item("hint"), "  - hint");
        assert_eq!(format_list_item_with_prefix("•", "hint"), "  • hint");
        assert_eq!(format_tsv_row(["label", "function"]), "label\tfunction");
    }
}
