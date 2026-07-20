pub mod connectors;
pub mod domain;
pub mod fixtures;
pub mod route_engine;
pub mod security;

use fixtures::{PlannerRequest, PlannerSnapshot};

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri owns deserialized IPC command arguments.
fn planner_snapshot(request: PlannerRequest) -> Result<PlannerSnapshot, String> {
    fixtures::fixture_planner_snapshot(&request).map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Starts the `AssetRail` desktop runtime.
///
/// # Panics
/// Panics when Tauri cannot initialize or run the application event loop.
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![planner_snapshot])
        .run(tauri::generate_context!())
        .expect("AssetRail application runtime failed");
}
