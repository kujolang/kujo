//! Bounded, namespace-aware XML parsing for hostile structured inputs.

use super::super::{DictMap, Value};
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{Namespace, ResolveResult};
use quick_xml::{NsReader, XmlVersion};
use std::collections::BTreeSet;
use std::sync::Arc;

const XML_INPUT_MAX: usize = 8 * 1024 * 1024;
const XML_DEPTH_MAX: usize = 64;
const XML_NODES_MAX: usize = 100_000;
const XML_ATTRIBUTES_MAX: usize = 200_000;
const XML_TEXT_MAX: usize = 8 * 1024 * 1024;
const XML_TREE_BYTES_MAX: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy)]
struct Limits {
    input: usize,
    depth: usize,
    nodes: usize,
    attributes: usize,
    text: usize,
    tree: usize,
}

#[derive(Debug)]
struct XmlAttribute {
    name: String,
    qualified_name: String,
    namespace: String,
    value: String,
}

#[derive(Debug)]
struct XmlNode {
    name: String,
    qualified_name: String,
    namespace: String,
    attributes: Vec<XmlAttribute>,
    children: Vec<XmlNode>,
    text: String,
}

fn string(value: impl Into<String>) -> Value {
    Value::Str(Arc::new(value.into()))
}

fn integer_option(
    options: &DictMap,
    name: &str,
    default: usize,
    absolute: usize,
) -> Result<usize, String> {
    let Some(value) = options.get(name) else { return Ok(default) };
    let Value::Int(value) = value else {
        return Err(format!("parse_xml_bounded option '{}' must be an integer", name));
    };
    if *value < 1 || *value as u128 > absolute as u128 {
        return Err(format!(
            "parse_xml_bounded option '{}' must be between 1 and {}",
            name, absolute
        ));
    }
    Ok(*value as usize)
}

fn limits(value: &Value) -> Result<Limits, String> {
    let options = match value {
        Value::Dict(options) => options.as_ref().clone(),
        Value::FixedDict { keys, values } => {
            if keys.len() != values.len() {
                return Err("parse_xml_bounded options dictionary is malformed".to_string());
            }
            let mut options = DictMap::default();
            for (key, value) in keys.iter().zip(values.iter()) {
                options.insert(key.clone(), value.clone());
            }
            options
        }
        _ => return Err("parse_xml_bounded requires an options dictionary".to_string()),
    };
    let allowed = [
        "max_input_bytes",
        "max_depth",
        "max_nodes",
        "max_attributes",
        "max_text_bytes",
        "max_tree_bytes",
    ];
    for key in options.keys() {
        if !allowed.contains(&key.as_ref()) {
            return Err(format!("parse_xml_bounded unknown option '{}'", key));
        }
    }
    Ok(Limits {
        input: integer_option(&options, "max_input_bytes", XML_INPUT_MAX, XML_INPUT_MAX)?,
        depth: integer_option(&options, "max_depth", XML_DEPTH_MAX, XML_DEPTH_MAX)?,
        nodes: integer_option(&options, "max_nodes", XML_NODES_MAX, XML_NODES_MAX)?,
        attributes: integer_option(
            &options,
            "max_attributes",
            XML_ATTRIBUTES_MAX,
            XML_ATTRIBUTES_MAX,
        )?,
        text: integer_option(&options, "max_text_bytes", XML_TEXT_MAX, XML_TEXT_MAX)?,
        tree: integer_option(&options, "max_tree_bytes", XML_TREE_BYTES_MAX, XML_TREE_BYTES_MAX)?,
    })
}

fn namespace_value(namespace: ResolveResult<'_>) -> Result<String, String> {
    match namespace {
        ResolveResult::Unbound => Ok(String::new()),
        ResolveResult::Bound(Namespace(value)) => Ok(value.to_string()),
        ResolveResult::Unknown(prefix) => Err(format!("XML_NAMESPACE_PREFIX_UNKNOWN:{}", prefix)),
    }
}

fn start_node(
    reader: &NsReader<&[u8]>,
    event: &BytesStart<'_>,
) -> Result<(XmlNode, usize, usize), String> {
    let (namespace, local) = reader.resolver().resolve_element(event.name());
    let namespace = namespace_value(namespace)?;
    let name = local.as_ref().to_string();
    let qualified_name = event.name().as_ref().to_string();
    let mut attributes = Vec::new();
    let mut attribute_count = 0usize;
    let mut tree_bytes = name
        .len()
        .checked_add(qualified_name.len())
        .and_then(|size| size.checked_add(namespace.len()))
        .ok_or_else(|| "XML_TREE_SIZE_LIMIT".to_string())?;
    let mut expanded_names = BTreeSet::new();
    for attribute in event.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| format!("XML_ATTRIBUTE_INVALID:{}", error))?;
        attribute_count =
            attribute_count.checked_add(1).ok_or_else(|| "XML_ATTRIBUTE_LIMIT".to_string())?;
        let qualified = attribute.key.as_ref().to_string();
        if qualified == "xmlns" || qualified.starts_with("xmlns:") {
            continue;
        }
        let (attribute_namespace, local) = reader.resolver().resolve_attribute(attribute.key);
        let attribute_namespace = namespace_value(attribute_namespace)?;
        let local = local.as_ref().to_string();
        if !expanded_names.insert((attribute_namespace.clone(), local.clone())) {
            return Err("XML_ATTRIBUTE_DUPLICATE_EXPANDED_NAME".to_string());
        }
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|error| format!("XML_ATTRIBUTE_VALUE_INVALID:{}", error))?
            .into_owned();
        tree_bytes = tree_bytes
            .checked_add(local.len())
            .and_then(|size| size.checked_add(qualified.len()))
            .and_then(|size| size.checked_add(attribute_namespace.len()))
            .and_then(|size| size.checked_add(value.len()))
            .ok_or_else(|| "XML_TREE_SIZE_LIMIT".to_string())?;
        attributes.push(XmlAttribute {
            name: local,
            qualified_name: qualified,
            namespace: attribute_namespace,
            value,
        });
    }
    Ok((
        XmlNode {
            name,
            qualified_name,
            namespace,
            attributes,
            children: Vec::new(),
            text: String::new(),
        },
        attribute_count,
        tree_bytes,
    ))
}

fn attribute_value(attribute: XmlAttribute) -> Value {
    let mut fields = DictMap::default();
    fields.insert("name".into(), string(attribute.name));
    fields.insert("qualified_name".into(), string(attribute.qualified_name));
    fields.insert("namespace".into(), string(attribute.namespace));
    fields.insert("value".into(), string(attribute.value));
    Value::dict(fields)
}

fn node_value(node: XmlNode) -> Value {
    let mut fields = DictMap::default();
    fields.insert("name".into(), string(node.name));
    fields.insert("qualified_name".into(), string(node.qualified_name));
    fields.insert("namespace".into(), string(node.namespace));
    fields.insert("text".into(), string(node.text));
    fields.insert(
        "attributes".into(),
        Value::array(node.attributes.into_iter().map(attribute_value).collect()),
    );
    fields.insert(
        "children".into(),
        Value::array(node.children.into_iter().map(node_value).collect()),
    );
    Value::dict(fields)
}

fn parse(xml: &str, limits: Limits) -> Result<Value, String> {
    if xml.is_empty() || xml.len() > limits.input {
        return Err("XML_INPUT_SIZE_INVALID".to_string());
    }
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = true;
    let mut buffer = Vec::new();
    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root: Option<XmlNode> = None;
    let mut nodes = 0usize;
    let mut attributes = 0usize;
    let mut text_bytes = 0usize;
    let mut tree_bytes = 0usize;
    let mut declaration_seen = false;
    let mut content_seen_before_declaration = false;

    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|error| format!("XML_DOCUMENT_INVALID:{}", error))?;
        if !matches!(&event, Event::Decl(_) | Event::Eof) {
            content_seen_before_declaration = true;
        }
        match event {
            Event::Decl(declaration) => {
                if declaration_seen
                    || content_seen_before_declaration
                    || root.is_some()
                    || !stack.is_empty()
                {
                    return Err("XML_DECLARATION_INVALID".to_string());
                }
                declaration_seen = true;
                let version = declaration
                    .version()
                    .map_err(|error| format!("XML_DECLARATION_INVALID:{}", error))?;
                if version.as_ref() != "1.0" {
                    return Err("XML_VERSION_UNSUPPORTED".to_string());
                }
                if let Some(encoding) = declaration.encoding() {
                    let encoding =
                        encoding.map_err(|error| format!("XML_DECLARATION_INVALID:{}", error))?;
                    let normalized = encoding.as_ref().to_ascii_lowercase();
                    if normalized != "utf-8" && normalized != "utf8" {
                        return Err("XML_ENCODING_UNSUPPORTED".to_string());
                    }
                }
                if let Some(standalone) = declaration.standalone() {
                    let standalone =
                        standalone.map_err(|error| format!("XML_DECLARATION_INVALID:{}", error))?;
                    if standalone.as_ref() != "yes" && standalone.as_ref() != "no" {
                        return Err("XML_STANDALONE_INVALID".to_string());
                    }
                }
            }
            Event::Start(start) => {
                if root.is_some() && stack.is_empty() {
                    return Err("XML_MULTIPLE_ROOTS".to_string());
                }
                if stack.len() + 1 > limits.depth {
                    return Err("XML_DEPTH_LIMIT".to_string());
                }
                nodes = nodes.checked_add(1).ok_or_else(|| "XML_NODE_LIMIT".to_string())?;
                if nodes > limits.nodes {
                    return Err("XML_NODE_LIMIT".to_string());
                }
                let (node, node_attributes, node_tree_bytes) = start_node(&reader, &start)?;
                attributes = attributes
                    .checked_add(node_attributes)
                    .ok_or_else(|| "XML_ATTRIBUTE_LIMIT".to_string())?;
                if attributes > limits.attributes {
                    return Err("XML_ATTRIBUTE_LIMIT".to_string());
                }
                tree_bytes = tree_bytes
                    .checked_add(node_tree_bytes)
                    .ok_or_else(|| "XML_TREE_SIZE_LIMIT".to_string())?;
                if tree_bytes > limits.tree {
                    return Err("XML_TREE_SIZE_LIMIT".to_string());
                }
                stack.push(node);
            }
            Event::Empty(start) => {
                if root.is_some() && stack.is_empty() {
                    return Err("XML_MULTIPLE_ROOTS".to_string());
                }
                if stack.len() + 1 > limits.depth {
                    return Err("XML_DEPTH_LIMIT".to_string());
                }
                nodes = nodes.checked_add(1).ok_or_else(|| "XML_NODE_LIMIT".to_string())?;
                if nodes > limits.nodes {
                    return Err("XML_NODE_LIMIT".to_string());
                }
                let (node, node_attributes, node_tree_bytes) = start_node(&reader, &start)?;
                attributes = attributes
                    .checked_add(node_attributes)
                    .ok_or_else(|| "XML_ATTRIBUTE_LIMIT".to_string())?;
                if attributes > limits.attributes {
                    return Err("XML_ATTRIBUTE_LIMIT".to_string());
                }
                tree_bytes = tree_bytes
                    .checked_add(node_tree_bytes)
                    .ok_or_else(|| "XML_TREE_SIZE_LIMIT".to_string())?;
                if tree_bytes > limits.tree {
                    return Err("XML_TREE_SIZE_LIMIT".to_string());
                }
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            Event::End(_) => {
                let node = stack.pop().ok_or_else(|| "XML_END_WITHOUT_START".to_string())?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else if root.replace(node).is_some() {
                    return Err("XML_MULTIPLE_ROOTS".to_string());
                }
            }
            Event::Text(text) => {
                let decoded = quick_xml::escape::unescape(text.xml10_content().as_ref())
                    .map_err(|error| format!("XML_TEXT_INVALID:{}", error))?
                    .into_owned();
                if let Some(node) = stack.last_mut() {
                    text_bytes = text_bytes
                        .checked_add(decoded.len())
                        .ok_or_else(|| "XML_TEXT_LIMIT".to_string())?;
                    if text_bytes > limits.text {
                        return Err("XML_TEXT_LIMIT".to_string());
                    }
                    tree_bytes = tree_bytes
                        .checked_add(decoded.len())
                        .ok_or_else(|| "XML_TREE_SIZE_LIMIT".to_string())?;
                    if tree_bytes > limits.tree {
                        return Err("XML_TREE_SIZE_LIMIT".to_string());
                    }
                    node.text.push_str(&decoded);
                } else if !decoded.trim().is_empty() {
                    return Err("XML_TEXT_OUTSIDE_ROOT".to_string());
                }
            }
            Event::CData(text) => {
                let decoded = text.xml10_content().into_owned();
                let node = stack.last_mut().ok_or_else(|| "XML_TEXT_OUTSIDE_ROOT".to_string())?;
                text_bytes = text_bytes
                    .checked_add(decoded.len())
                    .ok_or_else(|| "XML_TEXT_LIMIT".to_string())?;
                if text_bytes > limits.text {
                    return Err("XML_TEXT_LIMIT".to_string());
                }
                tree_bytes = tree_bytes
                    .checked_add(decoded.len())
                    .ok_or_else(|| "XML_TREE_SIZE_LIMIT".to_string())?;
                if tree_bytes > limits.tree {
                    return Err("XML_TREE_SIZE_LIMIT".to_string());
                }
                node.text.push_str(&decoded);
            }
            Event::Eof => break,
            Event::DocType(_) => return Err("XML_DOCTYPE_DENIED".to_string()),
            Event::PI(_) | Event::Comment(_) => {}
            Event::GeneralRef(reference) => {
                let encoded = format!("&{};", reference.xml10_content());
                let decoded = quick_xml::escape::unescape(&encoded)
                    .map_err(|_| "XML_ENTITY_REFERENCE_DENIED".to_string())?
                    .into_owned();
                let node = stack.last_mut().ok_or_else(|| "XML_TEXT_OUTSIDE_ROOT".to_string())?;
                text_bytes = text_bytes
                    .checked_add(decoded.len())
                    .ok_or_else(|| "XML_TEXT_LIMIT".to_string())?;
                if text_bytes > limits.text {
                    return Err("XML_TEXT_LIMIT".to_string());
                }
                tree_bytes = tree_bytes
                    .checked_add(decoded.len())
                    .ok_or_else(|| "XML_TREE_SIZE_LIMIT".to_string())?;
                if tree_bytes > limits.tree {
                    return Err("XML_TREE_SIZE_LIMIT".to_string());
                }
                node.text.push_str(&decoded);
            }
        }
        buffer.clear();
    }
    if !stack.is_empty() {
        return Err("XML_DOCUMENT_TRUNCATED".to_string());
    }
    let root = root.ok_or_else(|| "XML_ROOT_REQUIRED".to_string())?;
    let mut result = DictMap::default();
    result.insert("ok".into(), Value::Bool(true));
    result.insert("code".into(), string("XML_DOCUMENT_PARSED"));
    result.insert("schema".into(), string("dev.kujolang.xml-document.v1"));
    result.insert("node_count".into(), Value::Int(nodes as i64));
    result.insert("attribute_count".into(), Value::Int(attributes as i64));
    result.insert("text_bytes".into(), Value::Int(text_bytes as i64));
    result.insert("tree_bytes".into(), Value::Int(tree_bytes as i64));
    result.insert("root".into(), node_value(root));
    Ok(Value::dict(result))
}

pub fn handle(name: &str, arguments: &[Value]) -> Option<Value> {
    if name != "parse_xml_bounded" {
        return None;
    }
    if arguments.len() != 2 {
        return Some(Value::Error(format!(
            "parse_xml_bounded expects 2 arguments, got {}",
            arguments.len()
        )));
    }
    let Value::Str(xml) = &arguments[0] else {
        return Some(Value::Error("parse_xml_bounded requires an XML string".to_string()));
    };
    let limits = match limits(&arguments[1]) {
        Ok(value) => value,
        Err(error) => return Some(Value::Error(error)),
    };
    Some(parse(xml, limits).unwrap_or_else(Value::Error))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(entries: &[(&str, i64)]) -> Value {
        let mut values = DictMap::default();
        for (name, value) in entries {
            values.insert((*name).into(), Value::Int(*value));
        }
        Value::dict(values)
    }

    #[test]
    fn parses_namespaces_entities_and_bounded_tree() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?><feedback xmlns="urn:ietf:params:xml:ns:dmarc-2.0" xmlns:x="urn:test"><record x:id="1"><domain>Example &amp; Co &#x2713;</domain></record></feedback>"#;
        let parsed = handle("parse_xml_bounded", &[string(xml), options(&[])]).unwrap();
        let Value::Dict(result) = &parsed else { panic!("expected dictionary: {parsed:?}") };
        assert!(matches!(result.get("node_count"), Some(Value::Int(3))));
        assert!(matches!(result.get("attribute_count"), Some(Value::Int(3))));
        let Some(Value::Dict(root)) = result.get("root") else { panic!("expected root") };
        assert!(
            matches!(root.get("name"), Some(Value::Str(value)) if value.as_ref() == "feedback")
        );
        assert!(
            matches!(root.get("namespace"), Some(Value::Str(value)) if value.as_ref() == "urn:ietf:params:xml:ns:dmarc-2.0")
        );

        let repeated = handle("parse_xml_bounded", &[string(xml), options(&[])]).unwrap();
        let first_json = super::super::json::handle("to_json", &[parsed]).unwrap();
        let repeated_json = super::super::json::handle("to_json", &[repeated]).unwrap();
        assert!(matches!(
            (first_json, repeated_json),
            (Value::Str(first), Value::Str(second)) if first == second
        ));
    }

    #[test]
    fn accepts_inert_xml_nodes_and_rejects_active_content_unknown_prefixes_and_resource_exhaustion()
    {
        let inert = handle(
            "parse_xml_bounded",
            &[string("<?safe test?><!--comment--><root><![CDATA[x<y]]></root>"), options(&[])],
        )
        .unwrap();
        assert!(matches!(inert, Value::Dict(_)));
        for (xml, expected) in [
            ("<!DOCTYPE root><root/>", "XML_DOCTYPE_DENIED"),
            ("<root>&custom;</root>", "XML_ENTITY_REFERENCE_DENIED"),
            ("<x:root/>", "XML_NAMESPACE_PREFIX_UNKNOWN"),
            (" \n<?xml version=\"1.0\"?><root/>", "XML_DECLARATION_INVALID"),
            ("<?xml version=\"1.0\" standalone=\"maybe\"?><root/>", "XML_STANDALONE_INVALID"),
        ] {
            let result = handle("parse_xml_bounded", &[string(xml), options(&[])]).unwrap();
            assert!(
                matches!(result, Value::Error(message) if message.contains(expected)),
                "{}",
                expected
            );
        }
        let depth =
            handle("parse_xml_bounded", &[string("<a><b/></a>"), options(&[("max_depth", 1)])])
                .unwrap();
        assert!(matches!(depth, Value::Error(message) if message == "XML_DEPTH_LIMIT"));
        let nodes =
            handle("parse_xml_bounded", &[string("<a><b/></a>"), options(&[("max_nodes", 1)])])
                .unwrap();
        assert!(matches!(nodes, Value::Error(message) if message == "XML_NODE_LIMIT"));
        let attributes = handle(
            "parse_xml_bounded",
            &[string("<a xmlns:x=\"urn:x\" x:id=\"1\"/>"), options(&[("max_attributes", 1)])],
        )
        .unwrap();
        assert!(matches!(attributes, Value::Error(message) if message == "XML_ATTRIBUTE_LIMIT"));
        let text = handle(
            "parse_xml_bounded",
            &[string("<a><![CDATA[xy]]></a>"), options(&[("max_text_bytes", 1)])],
        )
        .unwrap();
        assert!(matches!(text, Value::Error(message) if message == "XML_TEXT_LIMIT"));
        let tree = handle(
            "parse_xml_bounded",
            &[string("<long-name/>"), options(&[("max_tree_bytes", 1)])],
        )
        .unwrap();
        assert!(matches!(tree, Value::Error(message) if message == "XML_TREE_SIZE_LIMIT"));
    }
}
