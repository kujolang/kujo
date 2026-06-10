use crate::docgen::model::DocProject;

pub fn render(project: &DocProject) -> Result<String, String> {
    serde_json::to_string_pretty(project)
        .map_err(|e| format!("failed to serialize doc project: {}", e))
}
