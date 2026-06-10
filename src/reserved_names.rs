use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

const RESERVED_NAMES_TOML: &str = include_str!("../config/reserved_names.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct ReservedNamesConfig {
    pub core_commands: Vec<String>,
    pub workflow_families: Vec<String>,
    pub first_party_tools: Vec<String>,
    pub reserved_namespaces: Vec<String>,
    pub reserved_package_names: Vec<String>,
    #[serde(default)]
    pub reserved_profile_names: Vec<String>,
    #[serde(default)]
    pub blocked_aliases: Vec<String>,
    #[serde(default)]
    pub first_party_pack_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum WorkflowPackTrust {
    FirstParty,
    ThirdParty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameReservationKind {
    CoreCommand,
    WorkflowFamily,
    FirstPartyTool,
    ReservedNamespace,
    ReservedPackageName,
    ReservedProfileName,
    BlockedAlias,
}

impl NameReservationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            NameReservationKind::CoreCommand => "core_commands",
            NameReservationKind::WorkflowFamily => "workflow_families",
            NameReservationKind::FirstPartyTool => "first_party_tools",
            NameReservationKind::ReservedNamespace => "reserved_namespaces",
            NameReservationKind::ReservedPackageName => "reserved_package_names",
            NameReservationKind::ReservedProfileName => "reserved_profile_names",
            NameReservationKind::BlockedAlias => "blocked_aliases",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameReservation {
    pub kind: NameReservationKind,
}

impl NameReservation {
    pub fn category_label(self) -> &'static str {
        self.kind.as_str()
    }
}

static RESERVED_NAMES: OnceLock<ReservedNamesConfig> = OnceLock::new();

pub fn config() -> &'static ReservedNamesConfig {
    RESERVED_NAMES.get_or_init(|| {
        let parsed: ReservedNamesConfig = toml::from_str(RESERVED_NAMES_TOML)
            .expect("config/reserved_names.toml must be valid TOML");
        validate_reserved_names(&parsed).expect("config/reserved_names.toml failed validation");
        parsed
    })
}

pub fn is_trusted_first_party_pack_id(pack_id: &str) -> bool {
    let normalized = normalize_name(pack_id);
    !normalized.is_empty() && contains_case_insensitive(&config().first_party_pack_ids, normalized)
}

pub fn reservation_for_top_level_command(name: &str) -> Option<NameReservation> {
    let normalized = normalize_name(name);
    if normalized.is_empty() {
        return None;
    }

    if contains_case_insensitive(&config().core_commands, normalized) {
        return Some(NameReservation { kind: NameReservationKind::CoreCommand });
    }
    if contains_case_insensitive(&config().workflow_families, normalized) {
        return Some(NameReservation { kind: NameReservationKind::WorkflowFamily });
    }
    if contains_case_insensitive(&config().first_party_tools, normalized) {
        return Some(NameReservation { kind: NameReservationKind::FirstPartyTool });
    }
    if contains_case_insensitive(&config().blocked_aliases, normalized) {
        return Some(NameReservation { kind: NameReservationKind::BlockedAlias });
    }
    None
}

pub fn reservation_for_namespace(name: &str) -> Option<NameReservation> {
    let normalized = normalize_name(name);
    if normalized.is_empty() {
        return None;
    }

    if contains_case_insensitive(&config().reserved_namespaces, normalized) {
        return Some(NameReservation { kind: NameReservationKind::ReservedNamespace });
    }
    if contains_case_insensitive(&config().core_commands, normalized) {
        return Some(NameReservation { kind: NameReservationKind::CoreCommand });
    }
    if contains_case_insensitive(&config().workflow_families, normalized) {
        return Some(NameReservation { kind: NameReservationKind::WorkflowFamily });
    }
    if contains_case_insensitive(&config().first_party_tools, normalized) {
        return Some(NameReservation { kind: NameReservationKind::FirstPartyTool });
    }
    if contains_case_insensitive(&config().blocked_aliases, normalized) {
        return Some(NameReservation { kind: NameReservationKind::BlockedAlias });
    }
    None
}

pub fn reservation_for_package_name(name: &str) -> Option<NameReservation> {
    let normalized = normalize_name(name);
    if normalized.is_empty() {
        return None;
    }

    if contains_case_insensitive(&config().reserved_package_names, normalized) {
        return Some(NameReservation { kind: NameReservationKind::ReservedPackageName });
    }
    if contains_case_insensitive(&config().reserved_namespaces, normalized) {
        return Some(NameReservation { kind: NameReservationKind::ReservedNamespace });
    }
    if contains_case_insensitive(&config().first_party_tools, normalized) {
        return Some(NameReservation { kind: NameReservationKind::FirstPartyTool });
    }
    None
}

pub fn reservation_for_profile_name(name: &str) -> Option<NameReservation> {
    let normalized = normalize_name(name);
    if normalized.is_empty() {
        return None;
    }

    if contains_case_insensitive(&config().reserved_profile_names, normalized) {
        return Some(NameReservation { kind: NameReservationKind::ReservedProfileName });
    }
    if contains_case_insensitive(&config().blocked_aliases, normalized) {
        return Some(NameReservation { kind: NameReservationKind::BlockedAlias });
    }
    None
}

pub fn external_reserved_command_error(command_name: &str) -> String {
    format!(
        "Command name '{}' is reserved by the Kujo CLI.\n\nExternal packages cannot register reserved top-level commands.\nUse a supported contribution point such as contributes.doctor_profiles, or expose a pack-local command through `kujo pack run <namespace> <command>`.",
        command_name
    )
}

pub fn external_reserved_namespace_error(namespace: &str, reservation: NameReservation) -> String {
    format!(
        "Namespace '{}' is reserved by Kujo (category: {}).\n\nChoose a different namespace, or publish under a scoped Kennel package name when namespace support is available.",
        namespace,
        reservation.category_label()
    )
}

pub fn validate_reserved_names(config: &ReservedNamesConfig) -> Result<(), String> {
    ensure_non_empty("core_commands", &config.core_commands)?;
    ensure_non_empty("workflow_families", &config.workflow_families)?;
    ensure_non_empty("first_party_tools", &config.first_party_tools)?;
    ensure_non_empty("reserved_namespaces", &config.reserved_namespaces)?;
    ensure_non_empty("reserved_package_names", &config.reserved_package_names)?;
    ensure_no_duplicate_entries("core_commands", &config.core_commands)?;
    ensure_no_duplicate_entries("workflow_families", &config.workflow_families)?;
    ensure_no_duplicate_entries("first_party_tools", &config.first_party_tools)?;
    ensure_no_duplicate_entries("reserved_namespaces", &config.reserved_namespaces)?;
    ensure_no_duplicate_entries("reserved_package_names", &config.reserved_package_names)?;
    ensure_no_duplicate_entries("reserved_profile_names", &config.reserved_profile_names)?;
    ensure_no_duplicate_entries("blocked_aliases", &config.blocked_aliases)?;
    ensure_no_duplicate_entries("first_party_pack_ids", &config.first_party_pack_ids)?;
    Ok(())
}

fn ensure_non_empty(field: &str, entries: &[String]) -> Result<(), String> {
    if entries.is_empty() {
        return Err(format!("reserved names field '{}' must not be empty", field));
    }
    for entry in entries {
        if entry.trim().is_empty() {
            return Err(format!("reserved names field '{}' must not contain empty entries", field));
        }
    }
    Ok(())
}

fn ensure_no_duplicate_entries(field: &str, entries: &[String]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let normalized = normalize_name(entry);
        if normalized.is_empty() {
            return Err(format!("reserved names field '{}' must not contain empty entries", field));
        }
        if !seen.insert(normalized.to_string()) {
            return Err(format!(
                "reserved names field '{}' contains duplicate entry '{}'",
                field, entry
            ));
        }
    }
    Ok(())
}

fn contains_case_insensitive(entries: &[String], candidate: &str) -> bool {
    entries.iter().any(|entry| entry.eq_ignore_ascii_case(candidate))
}

fn normalize_name(name: &str) -> &str {
    name.trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_loads_and_validates() {
        let cfg = config();
        assert!(cfg.core_commands.iter().any(|name| name == "run"));
        assert!(cfg.reserved_namespaces.iter().any(|name| name == "kujo"));
    }

    #[test]
    fn top_level_command_reservations_cover_core_and_first_party() {
        assert_eq!(
            reservation_for_top_level_command("run").map(|value| value.kind),
            Some(NameReservationKind::CoreCommand)
        );
        assert_eq!(
            reservation_for_top_level_command("eval").map(|value| value.kind),
            Some(NameReservationKind::FirstPartyTool)
        );
        assert_eq!(
            reservation_for_top_level_command("dev").map(|value| value.kind),
            Some(NameReservationKind::BlockedAlias)
        );
    }

    #[test]
    fn namespace_reservations_cover_core_and_namespaces() {
        assert_eq!(
            reservation_for_namespace("kujo").map(|value| value.kind),
            Some(NameReservationKind::ReservedNamespace)
        );
        assert_eq!(
            reservation_for_namespace("doctor").map(|value| value.kind),
            Some(NameReservationKind::ReservedNamespace)
        );
        assert_eq!(
            reservation_for_namespace("run").map(|value| value.kind),
            Some(NameReservationKind::CoreCommand)
        );
    }

    #[test]
    fn package_name_reservations_cover_reserved_package_names() {
        assert_eq!(
            reservation_for_package_name("kennel").map(|value| value.kind),
            Some(NameReservationKind::ReservedPackageName)
        );
        assert_eq!(
            reservation_for_package_name("doctor").map(|value| value.kind),
            Some(NameReservationKind::ReservedNamespace)
        );
    }

    #[test]
    fn first_party_pack_id_allowlist_is_explicit() {
        assert!(is_trusted_first_party_pack_id("kujo-doctor"));
        assert!(!is_trusted_first_party_pack_id("official"));
    }

    #[test]
    fn reserved_profile_names_are_blocked() {
        assert_eq!(
            reservation_for_profile_name("default").map(|value| value.kind),
            Some(NameReservationKind::ReservedProfileName)
        );
    }
}
