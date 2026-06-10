// File: src/workflow_pack/types.rs
//
// Core types for the Kujo workflow-pack system.
// Defines check results, status enums, command context, and report structures.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Status of a single check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Skip,
    Info,
}

impl CheckStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckStatus::Pass => "pass",
            CheckStatus::Warn => "warn",
            CheckStatus::Fail => "fail",
            CheckStatus::Skip => "skip",
            CheckStatus::Info => "info",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pass" => Some(CheckStatus::Pass),
            "warn" => Some(CheckStatus::Warn),
            "fail" => Some(CheckStatus::Fail),
            "skip" => Some(CheckStatus::Skip),
            "info" => Some(CheckStatus::Info),
            _ => None,
        }
    }
}

/// Severity level for a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CheckSeverity {
    #[default]
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl CheckSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckSeverity::Info => "info",
            CheckSeverity::Low => "low",
            CheckSeverity::Medium => "medium",
            CheckSeverity::High => "high",
            CheckSeverity::Critical => "critical",
        }
    }
}

/// A single check result produced by a workflow command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub id: String,
    pub label: String,
    pub status: CheckStatus,
    #[serde(default)]
    pub severity: CheckSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<String>,
    /// Stable machine-readable reason code for action generation.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<String>,
    /// Grouping category for rendering and analysis.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub category: Option<String>,
    /// Parsed major version number from observed output.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub observed_major: Option<i64>,
    /// Minimum required major version for this tool.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub minimum_major: Option<i64>,
}

impl CheckResult {
    pub fn pass(id: &str, label: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Pass,
            severity: CheckSeverity::Info,
            observed: None,
            expected: None,
            message: None,
            suggested_fix: None,
            reason: None,
            category: None,
            observed_major: None,
            minimum_major: None,
        }
    }

    pub fn pass_with(id: &str, label: &str, observed: &str) -> Self {
        Self { observed: Some(observed.to_string()), ..Self::pass(id, label) }
    }

    pub fn warn(id: &str, label: &str, message: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Warn,
            severity: CheckSeverity::Medium,
            observed: None,
            expected: None,
            message: Some(message.to_string()),
            suggested_fix: None,
            reason: None,
            category: None,
            observed_major: None,
            minimum_major: None,
        }
    }

    pub fn warn_with_fix(id: &str, label: &str, message: &str, suggested_fix: &str) -> Self {
        Self {
            suggested_fix: Some(suggested_fix.to_string()),
            reason: None,
            ..Self::warn(id, label, message)
        }
    }

    pub fn fail(id: &str, label: &str, message: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Fail,
            severity: CheckSeverity::High,
            observed: None,
            expected: None,
            message: Some(message.to_string()),
            suggested_fix: None,
            reason: None,
            category: None,
            observed_major: None,
            minimum_major: None,
        }
    }

    pub fn fail_with_fix(id: &str, label: &str, message: &str, suggested_fix: &str) -> Self {
        Self {
            suggested_fix: Some(suggested_fix.to_string()),
            reason: None,
            ..Self::fail(id, label, message)
        }
    }

    pub fn info(id: &str, label: &str, observed: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Info,
            severity: CheckSeverity::Info,
            observed: Some(observed.to_string()),
            expected: None,
            message: None,
            suggested_fix: None,
            reason: None,
            category: None,
            observed_major: None,
            minimum_major: None,
        }
    }

    pub fn skip(id: &str, label: &str, message: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Skip,
            severity: CheckSeverity::Info,
            observed: None,
            expected: None,
            message: Some(message.to_string()),
            suggested_fix: None,
            reason: None,
            category: None,
            observed_major: None,
            minimum_major: None,
        }
    }
}

/// Summary statistics for a set of check results.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckSummary {
    pub pass: usize,
    pub warn: usize,
    pub fail: usize,
    pub skip: usize,
    pub info: usize,
}

impl CheckSummary {
    pub fn from_results(results: &[CheckResult]) -> Self {
        let mut summary = Self::default();
        for result in results {
            match result.status {
                CheckStatus::Pass => summary.pass += 1,
                CheckStatus::Warn => summary.warn += 1,
                CheckStatus::Fail => summary.fail += 1,
                CheckStatus::Skip => summary.skip += 1,
                CheckStatus::Info => summary.info += 1,
            }
        }
        summary
    }

    pub fn overall_status(&self) -> &'static str {
        if self.fail > 0 {
            "fail"
        } else if self.warn > 0 {
            "warn"
        } else {
            "pass"
        }
    }

    pub fn total(&self) -> usize {
        self.pass + self.warn + self.fail + self.skip + self.info
    }
}

/// Top-level report produced by a workflow command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool: Option<String>,
    pub pack: String,
    pub namespace: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cwd: Option<String>,
    pub status: String,
    pub summary: CheckSummary,
    pub checks: Vec<CheckResult>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub recommended_next_actions: Option<Vec<String>>,
}

impl DoctorReport {
    pub fn new(
        pack_id: &str,
        namespace: &str,
        command_name: &str,
        checks: Vec<CheckResult>,
    ) -> Self {
        let summary = CheckSummary::from_results(&checks);
        let status = summary.overall_status().to_string();
        Self {
            tool: None,
            pack: pack_id.to_string(),
            namespace: namespace.to_string(),
            command: command_name.to_string(),
            profile: None,
            schema_version: None,
            cwd: None,
            status,
            summary,
            checks,
            recommended_next_actions: None,
        }
    }
}

/// Context passed to workflow command implementations.
#[derive(Debug, Clone)]
pub struct CommandContext {
    /// Current working directory.
    pub cwd: std::path::PathBuf,
    /// Whether JSON output is requested.
    pub json_output: bool,
    /// CLI arguments after the command name.
    pub args: Vec<String>,
    /// Environment variable snapshot (selective, not full env).
    pub env_vars: std::collections::HashMap<String, String>,
}
