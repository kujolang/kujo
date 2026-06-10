// File: src/workflow_pack/builtins/mod.rs
//
// Built-in workflow pack registrations.
//
// Built-in packs are compiled into the Kujo binary and have the highest
// priority during namespace resolution. They cannot be overridden by
// external packs.
//
// To add a built-in pack:
//   1. Create a new module under builtins/ (e.g., myteam.rs)
//   2. Add `pub mod myteam;` below
//   3. Implement command handlers as fn(&CommandContext) -> DoctorReport
//   4. Register the pack in register_all() below
//
// Currently no built-in packs are registered. The workflow pack system
// is designed so that teams can create external packs without modifying
// Kujo core. See docs/WORKFLOW_PACKS.md for details.

use crate::workflow_pack::discovery::DiscoveredPack;
use crate::workflow_pack::registry::WorkflowRegistry;

pub mod doctor;

/// Register all built-in workflow packs into the registry.
pub fn register_all(registry: &mut WorkflowRegistry) -> Result<(), String> {
    let pack = doctor::discovered_pack()?;
    let handlers = doctor::handlers();
    registry.register_builtin_pack(&pack, handlers)?;
    Ok(())
}

/// Build the list of built-in packs for discovery priority tracking.
pub fn builtin_packs_list() -> Vec<DiscoveredPack> {
    doctor::discovered_pack().map(|pack| vec![pack]).unwrap_or_default()
}
