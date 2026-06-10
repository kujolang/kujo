use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("kujo_{label}_{nonce}"));
    fs::create_dir_all(&dir).expect("failed to create unique temp directory");
    dir
}

fn extract_checklist_ids(checklist: &str) -> Vec<String> {
    checklist
        .lines()
        .filter_map(|line| {
            if !line.contains("**V1U-") {
                return None;
            }
            let (_, rest) = line.split_once("**")?;
            let (id, _) = rest.split_once("**")?;
            Some(id.to_string())
        })
        .collect()
}

#[test]
fn unresolved_inventory_generator_produces_table_for_all_master_items() {
    let root = repo_root();
    let checklist_path = root.join("docs").join("PRE_V1_MASTER_UNFINISHED_CHECKLIST.md");
    let generated_dir = unique_temp_dir("pre_v1_inventory_success");
    let output_path = generated_dir.join("inventory.md");

    let output = Command::new("bash")
        .current_dir(&root)
        .args([
            "scripts/generate_pre_v1_unresolved_inventory.sh",
            checklist_path.to_str().expect("path should be utf-8"),
            output_path.to_str().expect("path should be utf-8"),
        ])
        .output()
        .expect("failed to run unresolved inventory generator");

    assert!(
        output.status.success(),
        "inventory generator should succeed, status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let inventory =
        fs::read_to_string(&output_path).expect("expected generated inventory markdown");
    let checklist = fs::read_to_string(&checklist_path).expect("expected master checklist");
    let checklist_ids = extract_checklist_ids(&checklist);

    assert!(
        inventory.contains("| Item ID | Status | Classification | Rationale | Summary | Source References | Last Touched | Current Owner |"),
        "inventory should include required table header"
    );

    for id in &checklist_ids {
        assert!(
            inventory.contains(&format!("| `{id}` |")),
            "inventory should include row for checklist id {id}"
        );
    }

    let row_count = inventory.lines().filter(|line| line.starts_with("| `V1U-")).count();
    assert_eq!(
        row_count,
        checklist_ids.len(),
        "inventory row count should match checklist item count"
    );

    assert!(
        inventory.contains("docs/PRE_V1_ACTION_CHECKLIST.md"),
        "inventory should include auditable source references"
    );
    for classification in ["`v1-blocker`", "`v1-should-fix`", "`post-v1`", "`archive`"] {
        assert!(
            inventory.contains(classification),
            "inventory should include classification marker {classification}"
        );
    }
    assert!(
        inventory.contains("retained for audit traceability")
            && inventory.contains("Tag-time release-publication tasks")
            && inventory.contains("deterministic release behavior"),
        "inventory should include per-item rationale text for classification"
    );

    let csv_path = generated_dir.join("inventory.csv");
    assert!(
        csv_path.is_file(),
        "generator should write csv companion file at {}",
        csv_path.display()
    );
    let csv = fs::read_to_string(csv_path).expect("expected generated inventory csv");
    assert!(
        csv.starts_with(
            "item_id,status,classification,rationale,summary,source_references,last_touched,current_owner"
        ),
        "inventory csv should include classification and rationale columns"
    );
}

#[test]
fn unresolved_inventory_generator_fails_on_unmapped_item_id() {
    let root = repo_root();
    let temp_dir = unique_temp_dir("pre_v1_inventory_unmapped");
    let checklist_path = temp_dir.join("synthetic.md");
    let output_path = temp_dir.join("inventory.md");

    fs::write(
        &checklist_path,
        "# Synthetic Checklist\n\n- [ ] **V1U-UNKNOWN-999**: synthetic unmapped item\n",
    )
    .expect("failed to write synthetic checklist");

    let output = Command::new("bash")
        .current_dir(&root)
        .args([
            "scripts/generate_pre_v1_unresolved_inventory.sh",
            checklist_path.to_str().expect("path should be utf-8"),
            output_path.to_str().expect("path should be utf-8"),
        ])
        .output()
        .expect("failed to run unresolved inventory generator");

    assert!(
        !output.status.success(),
        "generator should fail when a checklist item has no source mapping"
    );
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(
        stderr.contains("no source mapping configured"),
        "stderr should explain missing source mapping, got: {stderr}"
    );
}

#[test]
fn unresolved_inventory_generator_fails_on_duplicate_item_ids() {
    let root = repo_root();
    let temp_dir = unique_temp_dir("pre_v1_inventory_duplicate");
    let checklist_path = temp_dir.join("synthetic.md");
    let output_path = temp_dir.join("inventory.md");

    fs::write(
        &checklist_path,
        "# Synthetic Checklist\n\n- [ ] **V1U-RES-001**: first copy\n- [ ] **V1U-RES-001**: duplicate copy\n",
    )
    .expect("failed to write duplicate synthetic checklist");

    let output = Command::new("bash")
        .current_dir(&root)
        .args([
            "scripts/generate_pre_v1_unresolved_inventory.sh",
            checklist_path.to_str().expect("path should be utf-8"),
            output_path.to_str().expect("path should be utf-8"),
        ])
        .output()
        .expect("failed to run unresolved inventory generator");

    assert!(
        !output.status.success(),
        "generator should fail when checklist contains duplicate IDs"
    );
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(
        stderr.contains("duplicate checklist item id"),
        "stderr should explain duplicate item IDs, got: {stderr}"
    );
}
