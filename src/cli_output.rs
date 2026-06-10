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

    #[derive(Debug)]
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
    fn format_helpers_render_stable_shapes() {
        assert_eq!(format_section("Summary"), "Summary");
        assert_eq!(format_kv("files", 12), "  files: 12");
        assert_eq!(format_list_item("hint"), "  - hint");
        assert_eq!(format_list_item_with_prefix("•", "hint"), "  • hint");
    }
}
