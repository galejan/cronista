// Cronista — Tauri backend
//
// Phase 1: Scaffold placeholder. Real commands implemented in Phase 2.

// Module declarations (Phase 2 will add command modules here)

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
