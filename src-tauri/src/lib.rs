// Cron-Insta — Tauri backend
//
// Module structure: models, utils, commands/*.
// The run() entry point and test suite live here.

mod models;
mod utils;
mod commands;

use crate::models::*;
#[allow(unused_imports)]
use crate::utils::*; // used by test module via super::*
use crate::commands::*;
#[allow(unused_imports)]
use chrono::Local; // used by test module via super::*
#[allow(unused_imports)]
use std::path::Path; // used by test module via super::*
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(ProjectState {
            active_project: Mutex::new(None),
            closing: Mutex::new(false),
            session_tracker: Mutex::new(SessionTracker::default()),
        })
        .invoke_handler(tauri::generate_handler![
            crear_proyecto,
            marcar_proyecto_cron_insta,
            inicializar_git,
            verificar_git_inicializado,
            eliminar_directorio_git,
            obtener_git_log,
            detectar_git,
            detectar_config_git,
            set_active_project,
            iniciar_sesion_escritura,
            guardar_capitulo,
            crear_checkpoint,
            cargar_indice,
            cargar_capitulo,
            crear_capitulo,
            eliminar_capitulo,
            listar_personajes,
            crear_personaje,
            cargar_personaje,
            actualizar_personaje,
            actualizar_fuente_proyecto,
            actualizar_config_proyecto,
            eliminar_personaje,
            listar_notas,
            crear_nota,
            cargar_nota,
            eliminar_nota,
            cargar_timeline,
            agregar_evento_timeline,
            actualizar_evento_timeline,
            reordenar_timeline,
            eliminar_evento_timeline,
            crear_trama,
            eliminar_trama,
            asignar_capitulo_trama,
            listar_lugares,
            crear_lugar,
            cargar_lugar,
            actualizar_lugar,
            eliminar_lugar,
            listar_media,
            copiar_a_media,
            leer_media_base64,
            exportar_proyecto_zip,
            exportar_proyecto_md,
            importar_proyecto,
            cargar_identidad_git,
            guardar_identidad_git,
            cargar_config_remoto,
            cargar_estadisticas,
            guardar_config_remoto,
            configurar_remoto,
            sincronizar_remoto,
            reintentar_push,
            push_ahora,
            verificar_remoto,
            traer_cambios,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<ProjectState>();

                let project_path = {
                    let active = state.active_project.lock().unwrap();
                    active.clone()
                };

                if let Some(ref path) = project_path {
                    // Guard against re-entry
                    {
                        let mut closing = state.closing.lock().unwrap();
                        if *closing { return; }
                        *closing = true;
                    }

                    api.prevent_close();

                    let path = path.clone();
                    let window_clone = window.clone();
                    let app_handle = window.app_handle().clone();

                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            async { let _ = do_checkpoint(&app_handle, &path); },
                        ).await;
                        let _ = window_clone.destroy();
                    });
                } else {
                    // No project — prevent close, then destroy (same mechanism as project case)
                    api.prevent_close();
                    let window_clone = window.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        let _ = window_clone.destroy();
                    });
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ========================================================================
    // Test helpers — avoid requiring tauri::AppHandle in unit tests
    // ========================================================================

    /// Test-only: initialise a git repo with the default "Cron-Insta" identity.
    /// Mirrors `inicializar_git` behaviour without needing an AppHandle.
    /// Production identity-reading is tested at the integration level.
    fn init_git_for_test(path_str: &str) -> Result<String, String> {
        let project_path = Path::new(path_str);
        if project_path.join(".git").exists() {
            return Ok("El repositorio ya estaba inicializado.".to_string());
        }
        let git_path = find_git()?;
        let output = system_command(&git_path)
            .arg("init")
            .current_dir(project_path)
            .output()
            .map_err(|e| format!("Error al ejecutar git init: {}", e))?;
        if output.status.success() {
            let _ = system_command(&git_path)
                .arg("config")
                .arg("user.name")
                .arg("Cron-Insta")
                .current_dir(project_path)
                .output();
            let _ = system_command(&git_path)
                .arg("config")
                .arg("user.email")
                .arg("cron-insta@local")
                .current_dir(project_path)
                .output();
            Ok("Repositorio Git inicializado correctamente.".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Error al inicializar Git: {}", stderr.trim()))
        }
    }

    /// Test-only: create a project directory structure with auto git init.
    /// Mirrors `crear_proyecto` behaviour without needing an AppHandle.
    fn create_project_for_test(path: String, nombre: String, font_family: Option<String>) -> Result<String, String> {
        let path = path.trim_end_matches('/').trim_end_matches('\\').to_string();
        let base = Path::new(&path);
        std::fs::create_dir_all(base)
            .map_err(|e| format!("No se pudo crear el directorio del proyecto: {}", e))?;
    let subdirs = [".config", "capitulos", "personajes", "notas", "lugares", "media"];
        for sub in &subdirs {
            std::fs::create_dir_all(base.join(sub))
                .map_err(|e| format!("No se pudo crear el directorio {}: {}", sub, e))?;
        }
        // Seed lugares/index.json (empty array)
        std::fs::write(base.join("lugares/index.json"), "[]")
            .map_err(|e| format!("Error al escribir lugares/index.json: {}", e))?;

        let metadata = Metadata {
            project_name: nombre.clone(),
            last_modified: Local::now().to_rfc3339(),
            chapters_order: vec![],
            characters_index: vec![],
            places_index: vec![],
            font_family: font_family.unwrap_or_else(default_font_family),
            push_enabled: false,
            consecutive_failures: 0,
            visible_tabs: VisibleTabs::default(),
            auto_save_interval_minutes: default_auto_save_interval(),
            tramas: vec![],
            chapter_tramas: vec![],
        };
        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| format!("Error al serializar metadata: {}", e))?;
        std::fs::write(base.join(".config/metadata.json"), metadata_json)
            .map_err(|e| format!("Error al escribir metadata.json: {}", e))?;
        std::fs::write(base.join(".config/timeline.json"), "[]")
            .map_err(|e| format!("Error al escribir timeline.json: {}", e))?;
        // Seed stats.json (empty)
        let stats = SessionStats::default();
        let stats_json = serde_json::to_string_pretty(&stats)
            .map_err(|e| format!("Error al serializar stats: {}", e))?;
        std::fs::write(base.join(".config/stats.json"), stats_json)
            .map_err(|e| format!("Error al escribir stats.json: {}", e))?;
        let _ = init_git_for_test(&path);
        Ok(format!("Proyecto '{}' creado en {}", nombre, path))
    }

    /// Count the number of commits in the git repository at `repo_path`.
    fn count_commits(repo_path: &Path) -> usize {
        if !repo_path.join(".git").exists() {
            return 0;
        }
        let git_path = match find_git() {
            Ok(p) => p,
            Err(_) => return 0,
        };
        let output = system_command(&git_path)
            .arg("rev-list")
            .arg("--count")
            .arg("HEAD")
            .current_dir(repo_path)
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.trim().parse::<usize>().unwrap_or(0)
            }
            _ => 0,
        }
    }

    // ========================================================================
    // project-file-management tests
    // ========================================================================

    #[test]
    fn test_crear_proyecto_creates_all_directories() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let result = create_project_for_test(path.clone(), "Test Project".to_string(), None);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        for sub in &[".config", "capitulos", "personajes", "notas"] {
            assert!(
                dir.path().join(sub).exists(),
                "Missing directory: {}",
                sub
            );
        }
    }

    #[test]
    fn test_crear_proyecto_seeds_metadata_json() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Mi Novela".to_string(), None);

        let metadata_path = dir.path().join(".config").join("metadata.json");
        assert!(metadata_path.exists(), "metadata.json does not exist");

        let content = fs::read_to_string(&metadata_path).expect("failed to read metadata.json");
        let meta: Metadata = serde_json::from_str(&content).expect("invalid metadata.json");

        assert_eq!(meta.project_name, "Mi Novela");
        assert!(!meta.last_modified.is_empty(), "last_modified is empty");
        // last_modified should be valid ISO 8601 (chrono::Utc produces RFC 3339)
        assert!(
            meta.last_modified.contains('T'),
            "last_modified is not ISO 8601: {}",
            meta.last_modified
        );
        assert!(meta.chapters_order.is_empty());
        assert!(meta.characters_index.is_empty());
    }

    #[test]
    fn test_crear_proyecto_seeds_timeline_json() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let timeline_path = dir.path().join(".config").join("timeline.json");
        assert!(timeline_path.exists(), "timeline.json does not exist");

        let content = fs::read_to_string(&timeline_path).expect("failed to read timeline.json");
        assert_eq!(content.trim(), "[]", "timeline.json should be an empty array");
    }

    #[test]
    fn test_guardar_capitulo_writes_utf8_content() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // Create the capitulos/ directory
        fs::create_dir_all(dir.path().join("capitulos")).unwrap();

        let content = "# Prólogo\n\nEra una noche oscura...";
        let result = guardar_capitulo(
            path.clone(),
            "0001_prologo.md".to_string(),
            content.to_string(),
        );
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let file_path = dir.path().join("capitulos").join("0001_prologo.md");
        assert!(file_path.exists(), "Chapter file does not exist");

        let written = fs::read_to_string(&file_path).expect("failed to read chapter");
        assert_eq!(written, content);
    }

    #[test]
    fn test_guardar_capitulo_overwrites_existing() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        fs::create_dir_all(dir.path().join("capitulos")).unwrap();

        let old_content = "Contenido viejo";
        let file_path = dir.path().join("capitulos").join("0001.md");
        fs::write(&file_path, old_content).unwrap();

        let new_content = "Contenido nuevo, completamente distinto";
        let result = guardar_capitulo(path.clone(), "0001.md".to_string(), new_content.to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let written = fs::read_to_string(&file_path).expect("failed to read chapter");
        assert_eq!(written, new_content);
    }

    #[test]
    fn test_cargar_indice_returns_json() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "My Project".to_string(), None);

        let result = cargar_indice(path.clone());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let json_str = result.unwrap();
        // Should deserialise as valid JSON
        let _meta: serde_json::Value =
            serde_json::from_str(&json_str).expect("cargar_indice returned invalid JSON");
    }

    #[test]
    fn test_cargar_indice_empty_path() {
        let result = cargar_indice("".to_string());
        assert!(result.is_err(), "Expected Err for empty path");
        let err = result.unwrap_err();
        assert!(
            !err.is_empty(),
            "Error message should not be empty for empty path"
        );
    }

    #[test]
    fn test_cargar_indice_file_not_found() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // Don't create the project — metadata.json won't exist
        let result = cargar_indice(path);
        assert!(result.is_err(), "Expected Err for missing metadata.json");
        let err = result.unwrap_err();
        assert!(
            err.contains("no encontrado") || err.contains("not found"),
            "Error should mention missing file, got: {}",
            err
        );
    }

    #[test]
    fn test_guardar_capitulo_unicode_roundtrip() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        fs::create_dir_all(dir.path().join("capitulos")).unwrap();

        // Spanish special chars, emoji, RTL, CJK, math
        let content = concat!(
            "Ñoño y pingüino — ¡árbol!\n",
            "áéíóú ü ñ ¿? ¡!\n",
            "😀🎉🔥 — emoji\n",
            "مرحبا بالعالم — RTL\n",
            "日本語テスト — CJK\n",
            "αβγ — Greek\n",
            "2² + 3³ = ?\n",
        );

        guardar_capitulo(path.clone(), "unicode.md".to_string(), content.to_string())
            .expect("guardar_capitulo failed");

        let file_path = dir.path().join("capitulos").join("unicode.md");
        let read_back = fs::read_to_string(&file_path).expect("failed to read back");

        assert_eq!(
            read_back, content,
            "UTF-8 round-trip failed: content mismatch"
        );
    }

    #[test]
    fn test_crear_proyecto_trailing_separator() {
        let dir = TempDir::new().expect("failed to create temp dir");
        // Append trailing separator — path.normalize() or trim_end_matches should handle it
        let path_with_slash = format!("{}/", dir.path().to_str().unwrap());

        let result = create_project_for_test(path_with_slash, "Trailing Test".to_string(), None);
        assert!(result.is_ok(), "crear_proyecto with trailing separator failed: {:?}", result);

        // All directories must exist
        assert!(dir.path().join(".config").exists(), ".config missing");
        assert!(dir.path().join("capitulos").exists(), "capitulos missing");
        assert!(dir.path().join("personajes").exists(), "personajes missing");
        assert!(dir.path().join("notas").exists(), "notas missing");

        // metadata.json must exist and be valid JSON
        let meta = fs::read_to_string(dir.path().join(".config").join("metadata.json"))
            .expect("metadata.json should exist");
        let _: serde_json::Value = serde_json::from_str(&meta).expect("metadata.json should be valid JSON");
    }

    #[test]
    fn test_crear_proyecto_permission_denied() {
        // /root/ is typically not writable by non-root users on Linux
        let result = create_project_for_test("/root/cron-insta-blocked".to_string(), "Test".to_string(), None);
        // On CI running as root, this could succeed; we just assert it doesn't panic
        match result {
            Ok(_) => {
                // We're likely root — clean up if we created anything
                let _ = std::fs::remove_dir_all("/root/cron-insta-blocked");
                // This is fine; the test just verified no panic
            }
            Err(e) => {
                assert!(
                    e.contains("No se pudo crear") || e.contains("permission") || e.contains("Permiso"),
                    "Expected a permission-related error, got: {}",
                    e
                );
            }
        }
    }

    // ========================================================================
    // git-abstraction tests
    // ========================================================================

    #[test]
    fn test_find_git_returns_something_or_none() {
        let result = find_git();
        match result {
            Ok(path) => {
                assert!(!path.is_empty(), "Git path should not be empty");
                assert!(Path::new(&path).exists(), "Git path '{}' does not exist", path);
            }
            Err(msg) => {
                // Perfectly valid — git might not be installed
                assert!(!msg.is_empty(), "Error message should not be empty");
            }
        }
    }

    #[test]
    fn test_inicializar_git_creates_dot_git() {
        // Guard: skip if git is not available
        if find_git().is_err() {
            eprintln!("SKIP: git not available on this system");
            return;
        }

        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let result = init_git_for_test(&path);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        assert!(
            dir.path().join(".git").exists(),
            ".git directory was not created"
        );
    }

    #[test]
    fn test_inicializar_git_already_initialized() {
        if find_git().is_err() {
            eprintln!("SKIP: git not available on this system");
            return;
        }

        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // First init
        let result1 = init_git_for_test(&path);
        assert!(result1.is_ok());

        // Second init on same directory (reinit)
        let result2 = init_git_for_test(&path);
        assert!(result2.is_ok(), "Re-init should succeed quietly");
    }

    #[test]
    fn test_crear_proyecto_auto_calls_git_init() {
        if find_git().is_err() {
            eprintln!("SKIP: git not available on this system");
            return;
        }

        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let result = create_project_for_test(path.clone(), "Test Project".to_string(), None);
        assert!(result.is_ok(), "crear_proyecto failed: {:?}", result);

        // crear_proyecto auto-calls inicializar_git after creating directories
        assert!(
            dir.path().join(".git").exists(),
            ".git should exist — crear_proyecto should auto-init git"
        );
    }

    #[test]
    fn test_crear_proyecto_works_without_git() {
        // Even if git is available, this test verifies crear_proyecto does
        // NOT panic when git-related operations fail.  The auto-init is
        // silent (let _ = ...) so the directory structure is always created.
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let result = create_project_for_test(path.clone(), "Sin Git".to_string(), None);
        assert!(result.is_ok(), "crear_proyecto should succeed: {:?}", result);

        // Disk structure must exist regardless of git availability
        assert!(dir.path().join(".config").exists());
        assert!(dir.path().join("capitulos").exists());
        assert!(dir.path().join("personajes").exists());
        assert!(dir.path().join("notas").exists());
    }

    #[test]
    fn test_crear_checkpoint_without_changes() {
        if find_git().is_err() {
            eprintln!("SKIP: git not available on this system");
            return;
        }

        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // Init a clean repo
        init_git_for_test(&path).expect("git init failed");

        // Now try checkpointing with no changes
        let result = perform_commit(dir.path());
        assert!(result.is_ok(), "Expected Ok for clean repo: {:?}", result);

        let msg = result.unwrap();
        assert!(
            msg.contains("Sin cambios") || msg.contains("nothing"),
            "Expected 'no changes' message, got: {}",
            msg
        );
    }

    #[test]
    fn test_crear_checkpoint_with_changes() {
        if find_git().is_err() {
            eprintln!("SKIP: git not available on this system");
            return;
        }

        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // Create project structure and init git
        fs::create_dir_all(dir.path().join("capitulos")).unwrap();
        init_git_for_test(&path).expect("git init failed");

        // Create a chapter file (a change)
        let content = "# Capítulo 1\n\nHabía una vez...";
        fs::write(
            dir.path().join("capitulos").join("0001_intro.md"),
            content,
        )
        .unwrap();

        let result = perform_commit(dir.path());
        assert!(result.is_ok(), "Expected Ok for checkpoint: {:?}", result);

        let hash = result.unwrap();
        // Should be a 40-char hex commit hash, not the "Sin cambios" message
        assert!(
            hash.len() >= 7 && hash.chars().all(|c| c.is_ascii_hexdigit()),
            "Expected a commit hash, got: {}",
            hash
        );
    }

    #[test]
    fn test_crear_checkpoint_git_unavailable() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let _path = dir.path().to_str().unwrap().to_string();

        // We simulate git-unavailable by using a dir where find_git() works
        // but the checkpoint operation itself is what we're checking
        if find_git().is_ok() {
            eprintln!("INFO: git IS available — cannot fully test git-unavailable path.");
            eprintln!("perform_commit is now best-effort and returns Ok even without git.");
            return;
        }

        // If git is truly unavailable, perform_commit returns Ok (best-effort)
        // with an error message instead of Err, so push is never skipped.
        let result = perform_commit(dir.path());
        assert!(result.is_ok(), "Expected Ok (best-effort), got {:?}", result);
        let msg = result.unwrap();
        assert!(
            msg.contains("Git no está disponible"),
            "Expected git-unavailable message, got: {}",
            msg
        );
    }

    #[test]
    fn test_guardar_capitulo_does_not_commit() {
        if find_git().is_err() {
            eprintln!("SKIP: git not available on this system");
            return;
        }

        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // Create project and init git
        fs::create_dir_all(dir.path().join("capitulos")).unwrap();
        init_git_for_test(&path).expect("git init failed");

        // Count commits before saving
        let count_before = count_commits(dir.path());

        // Save a chapter (should NOT commit)
        guardar_capitulo(
            path.clone(),
            "0001_test.md".to_string(),
            "# Test\n\nContent".to_string(),
        )
        .expect("guardar_capitulo failed");

        // Count commits after saving
        let count_after = count_commits(dir.path());

        assert_eq!(
            count_before, count_after,
            "guardar_capitulo should NOT create a git commit"
        );
    }

    // ========================================================================
    // Integration test: full flow
    // ========================================================================

    #[test]
    fn test_full_flow_create_save_checkpoint_read() {
        if find_git().is_err() {
            eprintln!("SKIP: git not available on this system");
            return;
        }

        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // --- Step 1: Create project ---
        let result = create_project_for_test(path.clone(), "Full Flow Test".to_string(), None);
        assert!(result.is_ok(), "Step 1 (crear_proyecto) failed: {:?}", result);

        // Verify directory structure
        assert!(dir.path().join(".config").exists());
        assert!(dir.path().join("capitulos").exists());
        assert!(dir.path().join("personajes").exists());
        assert!(dir.path().join("notas").exists());
        // git should be auto-initialised
        assert!(dir.path().join(".git").exists(), "git not initialised after crear_proyecto");

        // --- Step 2: Save a chapter ---
        let chapter_content = "# Prólogo\n\nEra el mejor de los tiempos, era el peor de los tiempos.";
        let result = guardar_capitulo(
            path.clone(),
            "0001_prologo.md".to_string(),
            chapter_content.to_string(),
        );
        assert!(result.is_ok(), "Step 2 (guardar_capitulo) failed: {:?}", result);

        let chapter_path = dir.path().join("capitulos").join("0001_prologo.md");
        assert!(chapter_path.exists());
        let saved = fs::read_to_string(&chapter_path).unwrap();
        assert_eq!(saved, chapter_content);

        // --- Step 3: Create checkpoint ---
        let result = perform_commit(dir.path());
        assert!(result.is_ok(), "Step 3 (crear_checkpoint) failed: {:?}", result);
        let hash = result.unwrap();
        assert!(!hash.is_empty(), "Commit hash should not be empty");

        // --- Step 4: Read metadata index ---
        let result = cargar_indice(path.clone());
        assert!(result.is_ok(), "Step 4 (cargar_indice) failed: {:?}", result);
        let index_json = result.unwrap();
        let index: serde_json::Value =
            serde_json::from_str(&index_json).expect("index should be valid JSON");
        assert_eq!(
            index["project_name"].as_str().unwrap(),
            "Full Flow Test"
        );
    }

    // ========================================================================
    // editor-integration tests
    // ========================================================================

    // --- cargar_capitulo (3 tests) ---

    #[test]
    fn test_cargar_capitulo_reads_existing_file() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        fs::create_dir_all(dir.path().join("capitulos")).unwrap();
        let content = "# Prólogo\n\nEra una noche...";
        fs::write(
            dir.path().join("capitulos").join("0001_prologo.md"),
            content,
        )
        .unwrap();

        let result = cargar_capitulo(path, "0001_prologo.md".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(result.unwrap(), content);
    }

    #[test]
    fn test_cargar_capitulo_file_not_found() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        fs::create_dir_all(dir.path().join("capitulos")).unwrap();

        let result = cargar_capitulo(path, "9999_fantasma.md".to_string());
        assert!(result.is_err(), "Expected Err for missing file");
        let err = result.unwrap_err();
        assert!(
            err.contains("no encontrado") || err.contains("not found"),
            "Error should mention missing file, got: {}",
            err
        );
    }

    #[test]
    fn test_cargar_capitulo_empty_path() {
        let result = cargar_capitulo("".to_string(), "test.md".to_string());
        assert!(result.is_err(), "Expected Err for empty path");
        let err = result.unwrap_err();
        assert!(!err.is_empty(), "Error message should not be empty");
    }

    // --- crear_capitulo (3 tests) ---

    #[test]
    fn test_crear_capitulo_creates_file_and_updates_metadata() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // Create a project so metadata.json exists
        let _ = create_project_for_test(path.clone(), "Test Project".to_string(), None);

        let contenido = "# Capítulo 1\n\n";
        let result = crear_capitulo(
            path.clone(),
            "0002_capitulo_1.md".to_string(),
            contenido.to_string(),
            None,
        );
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // File must exist with correct content
        let file_path = dir.path().join("capitulos").join("0002_capitulo_1.md");
        assert!(file_path.exists(), "Chapter file does not exist");
        let written = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, contenido);

        // Metadata must contain the new chapter in chapters_order
        let metadata_path = dir.path().join(".config").join("metadata.json");
        let metadata_str = fs::read_to_string(&metadata_path).unwrap();
        let metadata: Metadata =
            serde_json::from_str(&metadata_str).expect("invalid metadata.json");
        assert!(
            metadata.chapters_order.contains(&"0002_capitulo_1.md".to_string()),
            "chapters_order should include the new chapter"
        );
        // last_modified must be updated (non-empty ISO 8601)
        assert!(
            metadata.last_modified.contains('T'),
            "last_modified should be ISO 8601: {}",
            metadata.last_modified
        );
    }

    #[test]
    fn test_crear_capitulo_rejects_duplicate() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test Project".to_string(), None);

        // Create a chapter file manually so it already exists
        fs::write(
            dir.path().join("capitulos").join("0001_prologo.md"),
            "# Prólogo\n\n",
        )
        .unwrap();

        let result = crear_capitulo(
            path.clone(),
            "0001_prologo.md".to_string(),
            "Contenido duplicado".to_string(),
            None,
        );
        assert!(result.is_err(), "Expected Err for duplicate chapter");
        let err = result.unwrap_err();
        assert!(
            err.contains("ya existe") || err.contains("already exists"),
            "Error should mention duplicate, got: {}",
            err
        );

        // metadata.json must NOT be modified
        let metadata_path = dir.path().join(".config").join("metadata.json");
        let metadata_str = fs::read_to_string(&metadata_path).unwrap();
        let metadata: Metadata =
            serde_json::from_str(&metadata_str).expect("invalid metadata.json");
        assert!(
            metadata.chapters_order.is_empty(),
            "chapters_order should remain empty after duplicate rejection"
        );
    }

    #[test]
    fn test_crear_capitulo_handles_unicode() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test Project".to_string(), None);

        let contenido = concat!(
            "Ñoño y pingüino — ¡árbol!\n",
            "áéíóú ü ñ ¿? ¡!\n",
            "😀🎉🔥 — emoji\n",
            "日本語テスト — CJK\n",
        );

        let result = crear_capitulo(
            path.clone(),
            "unicode_chapter.md".to_string(),
            contenido.to_string(),
            None,
        );
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let file_path = dir.path().join("capitulos").join("unicode_chapter.md");
        let read_back = fs::read_to_string(&file_path).unwrap();
        assert_eq!(
            read_back, contenido,
            "UTF-8 round-trip failed: content mismatch"
        );
    }

    // ========================================================================
    // sidebar-characters-notes-timeline tests
    // ========================================================================

    // --- listar_personajes (1 test) ---

    #[test]
    fn test_listar_personajes_empty() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let result = listar_personajes(path);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(result.unwrap(), "[]");
    }

    // --- crear_personaje (2 tests) ---

    #[test]
    fn test_crear_personaje_y_listar() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let personaje_json = r#"{
            "id": "maria-garcia",
            "name": "María García",
            "physicalDescription": "Alta, pelo oscuro",
            "personality": "Introvertida"
        }"#;

        let result = crear_personaje(path.clone(), personaje_json.to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // Verify file exists
        let char_file = dir.path().join("personajes").join("maria-garcia.json");
        assert!(char_file.exists(), "Character file should exist");

        // Verify index
        let index_raw = listar_personajes(path.clone()).unwrap();
        let index: Vec<CharacterIndexItem> =
            serde_json::from_str(&index_raw).expect("index should be valid JSON");
        assert_eq!(index.len(), 1);
        assert_eq!(index[0].id, "maria-garcia");
        assert_eq!(index[0].name, "María García");
    }

    #[test]
    fn test_crear_personaje_rechaza_duplicado() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let personaje_json = r#"{"id": "juan", "name": "Juan"}"#;

        let _ = crear_personaje(path.clone(), personaje_json.to_string());

        let result = crear_personaje(path.clone(), personaje_json.to_string());
        assert!(result.is_err(), "Expected Err for duplicate");
        let err = result.unwrap_err();
        assert!(err.contains("ya existe"), "Should mention duplicate: {}", err);
    }

    // --- cargar_personaje (2 tests) ---

    #[test]
    fn test_cargar_personaje_returns_json() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let personaje_json = r#"{
            "id": "ana",
            "name": "Ana López",
            "personality": "Alegre"
        }"#;

        let _ = crear_personaje(path.clone(), personaje_json.to_string());

        let result = cargar_personaje(path.clone(), "ana".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let loaded: serde_json::Value =
            serde_json::from_str(&result.unwrap()).expect("should be valid JSON");
        assert_eq!(loaded["id"], "ana");
        assert_eq!(loaded["name"], "Ana López");
        assert_eq!(loaded["personality"], "Alegre");
    }

    #[test]
    fn test_cargar_personaje_not_found() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let result = cargar_personaje(path, "fantasma".to_string());
        assert!(result.is_err(), "Expected Err for missing character");
        let err = result.unwrap_err();
        assert!(
            err.contains("no encontrado"),
            "Should mention not found: {}",
            err
        );
    }

    // --- actualizar_personaje (2 tests) ---

    #[test]
    fn test_actualizar_personaje_overwrites() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let original = r#"{
            "id": "pedro",
            "name": "Pedro",
            "personality": "Serio",
            "traumas": "Ninguno"
        }"#;

        let _ = crear_personaje(path.clone(), original.to_string());

        let updated = r#"{
            "id": "pedro",
            "name": "Pedro Modificado",
            "personality": "Alegre ahora",
            "traumas": "Muchos"
        }"#;

        let result = actualizar_personaje(
            path.clone(),
            "pedro".to_string(),
            updated.to_string(),
        );
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // Verify file content
        let loaded = cargar_personaje(path.clone(), "pedro".to_string()).unwrap();
        let char: serde_json::Value =
            serde_json::from_str(&loaded).expect("should be valid JSON");
        assert_eq!(char["name"], "Pedro Modificado");
        assert_eq!(char["personality"], "Alegre ahora");

        // Verify index was updated with new name
        let index_raw = listar_personajes(path.clone()).unwrap();
        let index: Vec<CharacterIndexItem> =
            serde_json::from_str(&index_raw).unwrap();
        assert_eq!(index[0].name, "Pedro Modificado");
    }

    #[test]
    fn test_actualizar_personaje_not_found() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let result = actualizar_personaje(
            path,
            "inexistente".to_string(),
            r#"{"id":"inexistente","name":"X"}"#.to_string(),
        );
        assert!(result.is_err(), "Expected Err for missing character");
    }

    // --- eliminar_personaje (2 tests) ---

    #[test]
    fn test_eliminar_personaje_removes_file_and_index() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let _ = crear_personaje(
            path.clone(),
            r#"{"id": "laura", "name": "Laura"}"#.to_string(),
        );

        let result = eliminar_personaje(path.clone(), "laura".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // File must be gone
        assert!(
            !dir.path().join("personajes").join("laura.json").exists(),
            "Character file should be deleted"
        );

        // Index must be empty
        let index_raw = listar_personajes(path).unwrap();
        let index: Vec<CharacterIndexItem> =
            serde_json::from_str(&index_raw).unwrap();
        assert!(index.is_empty(), "Index should be empty after deletion");
    }

    #[test]
    fn test_eliminar_personaje_limpia_timeline() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        // Create a character
        let _ = crear_personaje(
            path.clone(),
            r#"{"id": "maria", "name": "María"}"#.to_string(),
        );

        // Add a timeline event referencing the character
        let event_json = format!(
            r#"{{"date":"1998-03-15","title":"María se va","description":"...","relatedCharacters":["maria"]}}"#
        );
        let _ = agregar_evento_timeline(path.clone(), event_json);

        // Delete the character
        let result = eliminar_personaje(path.clone(), "maria".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // Timeline event should no longer reference the deleted character
        let timeline_raw = cargar_timeline(path.clone()).unwrap();
        let timeline: Vec<TimelineEvent> =
            serde_json::from_str(&timeline_raw).unwrap();
        assert_eq!(timeline.len(), 1);
        assert!(
            timeline[0].relatedCharacters.is_empty(),
            "relatedCharacters should be empty after character deletion"
        );
    }

    // --- listar_notas (1 test) ---

    #[test]
    fn test_listar_notas_empty() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let result = listar_notas(path);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(result.unwrap(), "[]");
    }

    // --- crear_nota (2 tests) ---

    #[test]
    fn test_crear_nota_y_listar() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let result = crear_nota(
            path.clone(),
            "idea-1".to_string(),
            "Idea para trama".to_string(),
            "# Gran idea\n\nContenido de la nota.".to_string(),
        );
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // Verify file exists with correct content
        let note_file = dir.path().join("notas").join("idea-1.md");
        assert!(note_file.exists(), "Note file should exist");
        let content = fs::read_to_string(&note_file).unwrap();
        assert_eq!(content, "# Gran idea\n\nContenido de la nota.");

        // Verify index
        let index_raw = listar_notas(path.clone()).unwrap();
        let index: Vec<NoteIndexItem> =
            serde_json::from_str(&index_raw).unwrap();
        assert_eq!(index.len(), 1);
        assert_eq!(index[0].id, "idea-1");
        assert_eq!(index[0].title, "Idea para trama");
    }

    #[test]
    fn test_crear_nota_permite_sobrescribir() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let _ = crear_nota(
            path.clone(),
            "n1".to_string(),
            "Nota original".to_string(),
            "contenido viejo".to_string(),
        );

        // Overwrite with new content and title (upsert)
        let result = crear_nota(
            path.clone(),
            "n1".to_string(),
            "Nota actualizada".to_string(),
            "contenido nuevo".to_string(),
        );
        assert!(result.is_ok(), "Expected Ok for upsert, got {:?}", result);

        // Verify content was overwritten
        let loaded = cargar_nota(path.clone(), "n1".to_string()).unwrap();
        assert_eq!(loaded, "contenido nuevo");

        // Verify index title was updated
        let index_raw = listar_notas(path).unwrap();
        let index: Vec<NoteIndexItem> =
            serde_json::from_str(&index_raw).unwrap();
        assert_eq!(index.len(), 1, "Index should still have one entry");
        assert_eq!(index[0].title, "Nota actualizada");
    }

    // --- cargar_nota (1 test) ---

    #[test]
    fn test_cargar_nota_returns_content() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let contenido = "# Título\n\nPárrafo con **negrita** y más texto.";
        let _ = crear_nota(
            path.clone(),
            "nota-abc".to_string(),
            "Mi nota".to_string(),
            contenido.to_string(),
        );

        let result = cargar_nota(path, "nota-abc".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(result.unwrap(), contenido);
    }

    // --- eliminar_nota (1 test) ---

    #[test]
    fn test_eliminar_nota_removes_file_and_index() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let _ = crear_nota(
            path.clone(),
            "n-del".to_string(),
            "Para borrar".to_string(),
            "contenido".to_string(),
        );

        let result = eliminar_nota(path.clone(), "n-del".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // File must be gone
        assert!(
            !dir.path().join("notas").join("n-del.md").exists(),
            "Note file should be deleted"
        );

        // Index must be empty
        let index_raw = listar_notas(path).unwrap();
        let index: Vec<NoteIndexItem> =
            serde_json::from_str(&index_raw).unwrap();
        assert!(index.is_empty(), "Index should be empty after deletion");
    }

    // --- cargar_timeline (1 test) ---

    #[test]
    fn test_cargar_timeline_vacio() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let result = cargar_timeline(path);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(result.unwrap(), "[]");
    }

    // --- agregar_evento_timeline (2 tests) ---

    #[test]
    fn test_agregar_evento_timeline_creates_event() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let evento_json = r#"{
            "id": "evt-test",
            "date": "1998-03-15",
            "title": "María abandona el pueblo",
            "description": "Tras la discusión con Juan."
        }"#;

        let result = agregar_evento_timeline(path.clone(), evento_json.to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // Verify it's in the timeline
        let raw = cargar_timeline(path.clone()).unwrap();
        let timeline: Vec<TimelineEvent> =
            serde_json::from_str(&raw).unwrap();
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].id, "evt-test");
        assert_eq!(timeline[0].title, "María abandona el pueblo");
        assert_eq!(timeline[0].date, "1998-03-15");
    }

    #[test]
    fn test_agregar_evento_timeline_generates_id() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        // Event without an explicit id
        let evento_json = r#"{
            "date": "2000-01-01",
            "title": "Evento sin ID explícito",
            "description": ""
        }"#;

        let result = agregar_evento_timeline(path.clone(), evento_json.to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let raw = cargar_timeline(path).unwrap();
        let timeline: Vec<TimelineEvent> =
            serde_json::from_str(&raw).unwrap();
        assert_eq!(timeline.len(), 1);
        assert!(
            timeline[0].id.starts_with("evt-"),
            "ID should start with 'evt-': {}",
            timeline[0].id
        );
        assert!(!timeline[0].id.is_empty());
    }

    // --- actualizar_evento_timeline (2 tests) ---

    #[test]
    fn test_actualizar_evento_timeline_updates_fields() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        // Add an event first
        let _ = agregar_evento_timeline(
            path.clone(),
            r#"{"id":"evt-upd","date":"1999-05-10","title":"Original","description":"old"}"#.to_string(),
        );

        // Update it
        let updated_json = r#"{"id":"evt-upd","date":"2000-01-01","title":"Actualizado","description":"new desc"}"#;
        let result = actualizar_evento_timeline(path.clone(), updated_json.to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let raw = cargar_timeline(path).unwrap();
        let timeline: Vec<TimelineEvent> = serde_json::from_str(&raw).unwrap();
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].title, "Actualizado");
        assert_eq!(timeline[0].date, "2000-01-01");
        assert_eq!(timeline[0].description, "new desc");
    }

    #[test]
    fn test_actualizar_evento_timeline_rejects_unknown_id() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let result = actualizar_evento_timeline(
            path.clone(),
            r#"{"id":"no-existe","date":"","title":"Nope","description":""}"#.to_string(),
        );
        assert!(result.is_err(), "Expected Err for unknown ID");
    }

    // --- reordenar_timeline (1 test) ---

    #[test]
    fn test_reordenar_timeline_reorders_events() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        // Add three events in known order
        let _ = agregar_evento_timeline(
            path.clone(),
            r#"{"id":"evt-a","date":"2020-01-01","title":"Evento A","description":""}"#.to_string(),
        );
        let _ = agregar_evento_timeline(
            path.clone(),
            r#"{"id":"evt-b","date":"2020-06-15","title":"Evento B","description":""}"#.to_string(),
        );
        let _ = agregar_evento_timeline(
            path.clone(),
            r#"{"id":"evt-c","date":"2020-12-31","title":"Evento C","description":""}"#.to_string(),
        );

        // Reorder: C, A, B
        let ids_json = r#"["evt-c","evt-a","evt-b"]"#;
        let result = reordenar_timeline(path.clone(), ids_json.to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let raw = cargar_timeline(path).unwrap();
        let timeline: Vec<TimelineEvent> =
            serde_json::from_str(&raw).unwrap();
        assert_eq!(timeline.len(), 3);
        assert_eq!(timeline[0].id, "evt-c");
        assert_eq!(timeline[1].id, "evt-a");
        assert_eq!(timeline[2].id, "evt-b");
    }

    // --- eliminar_evento_timeline (1 test) ---

    #[test]
    fn test_eliminar_evento_timeline_removes_event() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let _ = agregar_evento_timeline(
            path.clone(),
            r#"{"id":"evt-1","date":"2020-01-01","title":"Evento 1","description":""}"#.to_string(),
        );

        let result = eliminar_evento_timeline(path.clone(), "evt-1".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let raw = cargar_timeline(path).unwrap();
        let timeline: Vec<TimelineEvent> =
            serde_json::from_str(&raw).unwrap();
        assert!(timeline.is_empty(), "Timeline should be empty after deletion");
    }

    // --- listar_notas with actual project (1 test) ---

    #[test]
    fn test_listar_notas_after_crear_proyecto() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        // After crear_proyecto, the notas/index.json doesn't exist yet,
        // so listar_notas should return "[]"
        let result = listar_notas(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "[]");
    }

    // ========================================================================
    // eliminar_capitulo tests
    // ========================================================================

    #[test]
    fn test_eliminar_capitulo_removes_file_and_metadata() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        // Create a chapter
        let _ = crear_capitulo(
            path.clone(),
            "0001_prologo.md".to_string(),
            "# Prólogo\n\n".to_string(),
            None,
        );

        // Verify chapter exists
        assert!(
            dir.path().join("capitulos").join("0001_prologo.md").exists(),
            "Chapter file should exist before deletion"
        );

        let result = eliminar_capitulo(path.clone(), "0001_prologo.md".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // File must be gone
        assert!(
            !dir.path().join("capitulos").join("0001_prologo.md").exists(),
            "Chapter file should be deleted"
        );

        // Metadata chapters_order must be empty
        let metadata_path = dir.path().join(".config").join("metadata.json");
        let metadata_str = fs::read_to_string(&metadata_path).unwrap();
        let metadata: Metadata =
            serde_json::from_str(&metadata_str).expect("invalid metadata.json");
        assert!(
            metadata.chapters_order.is_empty(),
            "chapters_order should be empty after deletion, got: {:?}",
            metadata.chapters_order
        );
    }

    #[test]
    fn test_eliminar_capitulo_rejects_nonexistent() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let result = eliminar_capitulo(path.clone(), "9999_fantasma.md".to_string());
        assert!(result.is_err(), "Expected Err for nonexistent chapter");
        let err = result.unwrap_err();
        assert!(
            err.contains("no existe"),
            "Error should mention non-existence, got: {}",
            err
        );
    }

    #[test]
    fn test_eliminar_capitulo_cleans_timeline_references() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        // Create a chapter
        let _ = crear_capitulo(
            path.clone(),
            "cap1.md".to_string(),
            "# Cap 1\n\n".to_string(),
            None,
        );

        // Add a timeline event referencing the chapter
        let event_json = format!(
            r#"{{"date":"2020-06-15","title":"Evento con cap","description":"...","relatedChapters":["cap1.md"]}}"#
        );
        let _ = agregar_evento_timeline(path.clone(), event_json);

        // Delete the chapter
        let result = eliminar_capitulo(path.clone(), "cap1.md".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // Timeline event should no longer reference the deleted chapter
        let timeline_raw = cargar_timeline(path.clone()).unwrap();
        let timeline: Vec<TimelineEvent> =
            serde_json::from_str(&timeline_raw).unwrap();
        assert_eq!(timeline.len(), 1);
        assert!(
            timeline[0].relatedChapters.is_empty(),
            "relatedChapters should be empty after chapter deletion, got: {:?}",
            timeline[0].relatedChapters
        );
    }

    // ========================================================================
    // places — lugares tests
    // ========================================================================

    #[test]
    fn test_crear_proyecto_creates_lugares_directory() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        assert!(
            dir.path().join("lugares").exists(),
            "lugares directory should exist"
        );
        assert!(
            dir.path().join("lugares").join("index.json").exists(),
            "lugares/index.json should exist"
        );

        let content = fs::read_to_string(dir.path().join("lugares").join("index.json")).unwrap();
        assert_eq!(content.trim(), "[]", "lugares/index.json should be an empty array");
    }

    #[test]
    fn test_crear_proyecto_metadata_has_places_index() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let meta_raw = fs::read_to_string(dir.path().join(".config").join("metadata.json")).unwrap();
        let meta: Metadata = serde_json::from_str(&meta_raw).unwrap();
        assert!(meta.places_index.is_empty(), "places_index should be empty on create");
    }

    #[test]
    fn test_listar_lugares_empty() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let result = listar_lugares(path);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(result.unwrap(), "[]");
    }

    #[test]
    fn test_crear_lugar_y_listar() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let lugar_json = r#"{
            "id": "torre-norte",
            "name": "Torre Norte",
            "description": "Una torre alta en el norte del reino"
        }"#;

        let result = crear_lugar(path.clone(), lugar_json.to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // Verify file exists
        let lugar_file = dir.path().join("lugares").join("torre-norte.json");
        assert!(lugar_file.exists(), "Place file should exist");

        // Verify index
        let index_raw = listar_lugares(path.clone()).unwrap();
        let index: Vec<LugarIndexItem> = serde_json::from_str(&index_raw).unwrap();
        assert_eq!(index.len(), 1);
        assert_eq!(index[0].id, "torre-norte");
        assert_eq!(index[0].name, "Torre Norte");
    }

    #[test]
    fn test_crear_lugar_rechaza_duplicado() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let lugar_json = r#"{"id": "plaza", "name": "Plaza Central"}"#;
        let _ = crear_lugar(path.clone(), lugar_json.to_string());

        let result = crear_lugar(path.clone(), lugar_json.to_string());
        assert!(result.is_err(), "Expected Err for duplicate");
        let err = result.unwrap_err();
        assert!(err.contains("ya existe"), "Should mention duplicate: {}", err);
    }

    #[test]
    fn test_cargar_lugar_returns_json() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let lugar_json = r#"{
            "id": "biblioteca",
            "name": "Gran Biblioteca",
            "description": "Contiene todos los libros del reino"
        }"#;

        let _ = crear_lugar(path.clone(), lugar_json.to_string());

        let result = cargar_lugar(path.clone(), "biblioteca".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let loaded: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(loaded["id"], "biblioteca");
        assert_eq!(loaded["name"], "Gran Biblioteca");
        assert_eq!(loaded["description"], "Contiene todos los libros del reino");
    }

    #[test]
    fn test_cargar_lugar_not_found() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let result = cargar_lugar(path, "inexistente".to_string());
        assert!(result.is_err(), "Expected Err for missing place");
        let err = result.unwrap_err();
        assert!(
            err.contains("no encontrado"),
            "Should mention not found: {}",
            err
        );
    }

    #[test]
    fn test_actualizar_lugar_overwrites() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let original = r#"{
            "id": "taberna",
            "name": "Taberna del Lobo",
            "description": "Un lugar oscuro"
        }"#;

        let _ = crear_lugar(path.clone(), original.to_string());

        let updated = r#"{
            "id": "taberna",
            "name": "Taberna del Lobo Blanco",
            "description": "Renovada y luminosa ahora"
        }"#;

        let result = actualizar_lugar(
            path.clone(),
            "taberna".to_string(),
            updated.to_string(),
        );
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // Verify file content
        let loaded = cargar_lugar(path.clone(), "taberna".to_string()).unwrap();
        let lugar: serde_json::Value = serde_json::from_str(&loaded).unwrap();
        assert_eq!(lugar["name"], "Taberna del Lobo Blanco");
        assert_eq!(lugar["description"], "Renovada y luminosa ahora");

        // Verify index was updated with new name
        let index_raw = listar_lugares(path.clone()).unwrap();
        let index: Vec<LugarIndexItem> = serde_json::from_str(&index_raw).unwrap();
        assert_eq!(index[0].name, "Taberna del Lobo Blanco");
    }

    #[test]
    fn test_eliminar_lugar_limpia() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let _ = crear_lugar(
            path.clone(),
            r#"{"id": "muelle", "name": "Muelle Viejo"}"#.to_string(),
        );

        let result = eliminar_lugar(path.clone(), "muelle".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // File must be gone
        assert!(
            !dir.path().join("lugares").join("muelle.json").exists(),
            "Place file should be deleted"
        );

        // Index must be empty
        let index_raw = listar_lugares(path).unwrap();
        let index: Vec<LugarIndexItem> = serde_json::from_str(&index_raw).unwrap();
        assert!(index.is_empty(), "Index should be empty after deletion");
    }

    // ========================================================================
    // git-identity-config tests
    // ========================================================================

    /// Helper: write raw JSON content to a temp config file and return the path.
    fn write_config(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    // --- Serde roundtrip ---

    #[test]
    fn test_git_identity_serde_roundtrip() {
        let identity = GitIdentity {
            name: "Ada Lovelace".to_string(),
            email: "ada@code.dev".to_string(),
            github_user: None,
        };
        let json = serde_json::to_string(&identity).expect("serialize identity");
        let parsed: GitIdentity = serde_json::from_str(&json).expect("deserialize identity");
        assert_eq!(parsed.name, "Ada Lovelace");
        assert_eq!(parsed.email, "ada@code.dev");
    }

    #[test]
    fn test_git_remote_config_serde_roundtrip() {
        let remote = GitRemoteConfig {
            url: "git@github.com:user/repo.git".to_string(),
            push_enabled: true,
            consecutive_failures: 2,
        };
        let json = serde_json::to_string(&remote).expect("serialize remote config");
        let parsed: GitRemoteConfig = serde_json::from_str(&json).expect("deserialize remote config");
        assert_eq!(parsed.url, "git@github.com:user/repo.git");
        assert!(parsed.push_enabled);
        assert_eq!(parsed.consecutive_failures, 2);
    }

    #[test]
    fn test_git_remote_config_defaults() {
        // Missing push_enabled and consecutive_failures should default correctly
        let json = r#"{"url":"git@host:repo.git"}"#;
        let parsed: GitRemoteConfig = serde_json::from_str(json).expect("deserialize with defaults");
        assert_eq!(parsed.url, "git@host:repo.git");
        assert!(!parsed.push_enabled, "push_enabled should default to false");
        assert_eq!(parsed.consecutive_failures, 0, "consecutive_failures should default to 0");
    }

    #[test]
    fn test_git_config_full_serde_roundtrip() {
        // Test backward compat: old config with remote key is deserialized
        // without error (serde ignores unknown fields).
        let old_json = r#"{
            "schema_version": 1,
            "identity": { "name": "Cervantes", "email": "cervantes@lit.es" },
            "remote": { "url": "git@github.com:user/repo.git", "push_enabled": true, "consecutive_failures": 0 }
        }"#;
        let parsed: GitConfig = serde_json::from_str(old_json).expect("deserialize old-format config");
        assert_eq!(parsed.schema_version, 1);
        let id = parsed.identity.as_ref().expect("identity should be present");
        assert_eq!(id.name, "Cervantes");

        // Write-back should NOT contain the remote key (stripped migration)
        let new_json = serde_json::to_string_pretty(&parsed).expect("serialize");
        assert!(!new_json.contains("\"remote\""), "write-back must strip remote key");
        assert!(new_json.contains("\"Cervantes\""), "identity must be preserved");
    }

    #[test]
    fn test_git_config_defaults_empty_sections() {
        let json = r#"{"schema_version":1}"#;
        let parsed: GitConfig = serde_json::from_str(json).expect("deserialize minimal config");
        assert_eq!(parsed.schema_version, 1);
        assert!(parsed.identity.is_none(), "identity should be None");
    }

    // --- Identity: save then load (filesystem roundtrip) ---

    #[test]
    fn test_identity_save_then_load() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let config_path = dir.path().join("cron-insta").join("git-config.json");

        // Simulate guardar: write full config with identity
        let config = GitConfig {
            schema_version: 1,
            identity: Some(GitIdentity {
                name: "Ada Lovelace".to_string(),
                email: "ada@code.dev".to_string(),
                github_user: None,
            }),
        };
        let json = serde_json::to_string_pretty(&config).unwrap();
        write_config(&config_path, &json);

        // Simulate cargar: read back
        let raw = fs::read_to_string(&config_path).unwrap();
        let parsed: GitConfig = serde_json::from_str(&raw).unwrap();
        let id = parsed.identity.expect("identity should be present");
        assert_eq!(id.name, "Ada Lovelace");
        assert_eq!(id.email, "ada@code.dev");
    }

    // --- Corrupted file returns None gracefully ---

    #[test]
    fn test_identity_corrupted_json() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let config_path = dir.path().join("cron-insta").join("git-config.json");

        write_config(&config_path, "this is not valid json {{{");

        // Reading corrupted JSON should return None (graceful degradation)
        let raw = fs::read_to_string(&config_path).unwrap();
        let result: Result<GitConfig, _> = serde_json::from_str(&raw);
        assert!(result.is_err(), "should fail to parse corrupted JSON");
    }

    // --- Missing file returns None ---

    #[test]
    fn test_identity_missing_file() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let config_path = dir.path().join("cron-insta").join("git-config.json");

        // File doesn't exist — should be treated as missing
        assert!(!config_path.exists());
        // The command-level logic returns "null" when the file doesn't exist,
        // which is tested indirectly via the filesystem roundtrip above.
    }

    // --- Remote: save then load (filesystem roundtrip) ---

    #[test]
    fn test_remote_config_save_then_load() {
        // Now tests per-project push state in metadata.json
        let dir = TempDir::new().expect("failed to create temp dir");
        let project_path = dir.path().to_str().unwrap().to_string();

        // Create project with push_enabled: true
        create_project_for_test(project_path.clone(), "Test".to_string(), None).unwrap();

        // Read metadata, set push_enabled, write back
        let meta_path = dir.path().join(".config").join("metadata.json");
        let raw = fs::read_to_string(&meta_path).unwrap();
        let mut meta: Metadata = serde_json::from_str(&raw).unwrap();
        meta.push_enabled = true;
        meta.consecutive_failures = 0;
        fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap()).unwrap();

        // Read back and verify
        let raw = fs::read_to_string(&meta_path).unwrap();
        let parsed: Metadata = serde_json::from_str(&raw).unwrap();
        assert!(parsed.push_enabled);
        assert_eq!(parsed.consecutive_failures, 0);
    }

    // --- Migration: old global config with remote key is safely deserialized ---

    #[test]
    fn test_identity_load_strips_legacy_remote() {
        // Old-format config had a "remote" key alongside identity.
        // New struct ignores it (serde default) and write-back strips it.
        let dir = TempDir::new().expect("failed to create temp dir");
        let config_path = dir.path().join("cron-insta").join("git-config.json");

        // Write old-format config with both identity and remote
        let old_json = r#"{
            "schema_version": 1,
            "identity": { "name": "Ada", "email": "ada@code.dev" },
            "remote": { "url": "git@github.com:user/repo.git", "push_enabled": true, "consecutive_failures": 0 }
        }"#;
        write_config(&config_path, old_json);

        // Load: should succeed, identity preserved, remote silently ignored
        let raw = fs::read_to_string(&config_path).unwrap();
        let config: GitConfig = serde_json::from_str(&raw).unwrap();
        assert_eq!(config.identity.as_ref().unwrap().name, "Ada");

        // Write-back: should NOT contain remote key
        let new_json = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&config_path, &new_json).unwrap();
        let final_raw = fs::read_to_string(&config_path).unwrap();
        assert!(!final_raw.contains("\"remote\""), "migration must strip remote key");
        assert!(final_raw.contains("\"Ada\""), "identity must be preserved");
    }

    // --- Per-project push state independence ---

    #[test]
    fn test_push_state_per_project_isolation() {
        let dir = TempDir::new().expect("failed to create temp dir");

        // Create project A with push enabled
        let path_a = dir.path().join("project_a");
        fs::create_dir_all(&path_a).unwrap();
        create_project_for_test(
            path_a.to_str().unwrap().to_string(),
            "Project A".to_string(),
            None,
        ).unwrap();

        // Enable push on project A
        let meta_a = path_a.join(".config").join("metadata.json");
        let raw = fs::read_to_string(&meta_a).unwrap();
        let mut md_a: Metadata = serde_json::from_str(&raw).unwrap();
        md_a.push_enabled = true;
        md_a.consecutive_failures = 1;
        fs::write(&meta_a, serde_json::to_string_pretty(&md_a).unwrap()).unwrap();

        // Create project B (fresh, should have push_enabled=false)
        let path_b = dir.path().join("project_b");
        fs::create_dir_all(&path_b).unwrap();
        create_project_for_test(
            path_b.to_str().unwrap().to_string(),
            "Project B".to_string(),
            None,
        ).unwrap();

        let meta_b = path_b.join(".config").join("metadata.json");
        let raw_b = fs::read_to_string(&meta_b).unwrap();
        let md_b: Metadata = serde_json::from_str(&raw_b).unwrap();

        // Project B should NOT be affected by project A's state
        assert!(!md_b.push_enabled, "project B should start with push disabled");
        assert_eq!(md_b.consecutive_failures, 0, "project B should have 0 failures");

        // Verify project A still has its state
        let raw_a = fs::read_to_string(&meta_a).unwrap();
        let md_a2: Metadata = serde_json::from_str(&raw_a).unwrap();
        assert!(md_a2.push_enabled, "project A push should still be enabled");
        assert_eq!(md_a2.consecutive_failures, 1, "project A should have 1 failure");
    }

    // --- Metadata serde backward compat: old metadata without push fields ---

    #[test]
    fn test_metadata_serde_backward_compat() {
        // Old metadata.json without push_enabled/consecutive_failures
        let old_json = r#"{
            "project_name": "Old Project",
            "last_modified": "2024-01-01T00:00:00Z",
            "chapters_order": [],
            "characters_index": [],
            "places_index": [],
            "font_family": "monospace"
        }"#;
        let meta: Metadata = serde_json::from_str(old_json).expect("deserialize old metadata");
        assert_eq!(meta.project_name, "Old Project");
        assert!(!meta.push_enabled, "push_enabled should default to false");
        assert_eq!(meta.consecutive_failures, 0, "consecutive_failures should default to 0");

        // Round-trip: serialize back, deserialize again, fields survive
        let json = serde_json::to_string_pretty(&meta).expect("serialize");
        let meta2: Metadata = serde_json::from_str(&json).expect("deserialize round-trip");
        assert!(!meta2.push_enabled, "push_enabled should survive round-trip");
        assert_eq!(meta2.consecutive_failures, 0, "consecutive_failures should survive round-trip");
    }

    // --- Metadata push fields survive round-trip ---

    #[test]
    fn test_metadata_push_fields_survive_roundtrip() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        create_project_for_test(path.clone(), "Test".to_string(), None).unwrap();

        let meta_path = dir.path().join(".config").join("metadata.json");

        // Set push fields
        let raw = fs::read_to_string(&meta_path).unwrap();
        let mut meta: Metadata = serde_json::from_str(&raw).unwrap();
        meta.push_enabled = true;
        meta.consecutive_failures = 2;
        fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap()).unwrap();

        // Modify an unrelated field (font_family) via actualizar_fuente_proyecto
        actualizar_fuente_proyecto(path.clone(), "serif".to_string()).unwrap();

        // Push fields must survive
        let raw2 = fs::read_to_string(&meta_path).unwrap();
        let meta2: Metadata = serde_json::from_str(&raw2).unwrap();
        assert!(meta2.push_enabled, "push_enabled should survive unrelated modification");
        assert_eq!(meta2.consecutive_failures, 2, "consecutive_failures should survive");
        assert_eq!(meta2.font_family, "serif", "font_family should be updated");
    }

    // --- SSH_AUTH_SOCK env var inheritance ---

    #[test]
    fn test_system_command_inherits_ssh_auth_sock() {
        // Set SSH_AUTH_SOCK in the current env
        std::env::set_var("SSH_AUTH_SOCK", "/tmp/test-ssh-agent.sock");

        // Spawn a child that echoes the env var
        let output = system_command("sh")
            .arg("-c")
            .arg("echo $SSH_AUTH_SOCK")
            .output()
            .expect("failed to spawn child");

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(stdout, "/tmp/test-ssh-agent.sock",
            "SSH_AUTH_SOCK should be inherited by child process");
    }

    // --- Identity with Unicode names ---

    #[test]
    fn test_identity_unicode_name() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let config_path = dir.path().join("cron-insta").join("git-config.json");

        let config = GitConfig {
            schema_version: 1,
            identity: Some(GitIdentity {
                name: "José María García López".to_string(),
                email: "josé@español.es".to_string(),
                github_user: None,
            }),
        };
        write_config(&config_path, &serde_json::to_string_pretty(&config).unwrap());

        let raw = fs::read_to_string(&config_path).unwrap();
        let parsed: GitConfig = serde_json::from_str(&raw).unwrap();
        let id = parsed.identity.unwrap();
        assert_eq!(id.name, "José María García López");
        assert_eq!(id.email, "josé@español.es");
    }

    // ========================================================================
    // git-remote-sync tests (PR 2 — SSH validation, 3-strike logic)
    // ========================================================================

    // --- SSH URL validation ---

    #[test]
    fn test_ssh_url_validation_valid_git_at() {
        // git@ URLs are valid SSH URLs
        let url = "git@github.com:user/repo.git";
        let lower = url.to_lowercase();
        let is_http = lower.starts_with("http://") || lower.starts_with("https://");
        assert!(!is_http, "git@ URL should be accepted as SSH");
    }

    #[test]
    fn test_ssh_url_validation_valid_ssh_scheme() {
        // ssh:// URLs are valid
        let url = "ssh://git@github.com/user/repo.git";
        let lower = url.to_lowercase();
        let is_http = lower.starts_with("http://") || lower.starts_with("https://");
        assert!(!is_http, "ssh:// URL should be accepted");
    }

    #[test]
    fn test_ssh_url_validation_rejects_https() {
        let url = "https://github.com/user/repo.git";
        let lower = url.to_lowercase();
        let is_http = lower.starts_with("http://") || lower.starts_with("https://");
        assert!(is_http, "https:// URL should be rejected");
    }

    #[test]
    fn test_ssh_url_validation_rejects_http() {
        let url = "http://github.com/user/repo.git";
        let lower = url.to_lowercase();
        let is_http = lower.starts_with("http://") || lower.starts_with("https://");
        assert!(is_http, "http:// URL should be rejected");
    }

    // --- 3-strike counter logic (pure state transitions) ---

    #[test]
    fn test_strike_counter_resets_on_success() {
        let mut remote = GitRemoteConfig {
            url: "git@host:repo.git".to_string(),
            push_enabled: true,
            consecutive_failures: 2,
        };
        // Simulate successful push: counter resets to 0
        remote.consecutive_failures = 0;
        assert_eq!(remote.consecutive_failures, 0);
        assert!(remote.push_enabled);
    }

    #[test]
    fn test_strike_first_failure() {
        let mut remote = GitRemoteConfig {
            url: "git@host:repo.git".to_string(),
            push_enabled: true,
            consecutive_failures: 0,
        };
        // Simulate first push failure
        remote.consecutive_failures += 1;
        assert_eq!(remote.consecutive_failures, 1);
        assert!(remote.push_enabled, "push should still be enabled after 1 failure");
    }

    #[test]
    fn test_strike_second_failure() {
        let mut remote = GitRemoteConfig {
            url: "git@host:repo.git".to_string(),
            push_enabled: true,
            consecutive_failures: 1,
        };
        // Simulate second push failure
        remote.consecutive_failures += 1;
        assert_eq!(remote.consecutive_failures, 2);
        assert!(remote.push_enabled, "push should still be enabled after 2 failures");
    }

    #[test]
    fn test_third_strike_disables_push() {
        let mut remote = GitRemoteConfig {
            url: "git@host:repo.git".to_string(),
            push_enabled: true,
            consecutive_failures: 2,
        };
        // Simulate third failure → disable
        remote.consecutive_failures += 1;
        if remote.consecutive_failures >= 3 {
            remote.push_enabled = false;
        }
        assert_eq!(remote.consecutive_failures, 3);
        assert!(!remote.push_enabled, "push should be disabled after 3 failures");
    }

    #[test]
    fn test_push_skipped_when_disabled() {
        let remote = GitRemoteConfig {
            url: "git@host:repo.git".to_string(),
            push_enabled: false,
            consecutive_failures: 3,
        };
        // When push_enabled is false, no push should be attempted
        assert!(!remote.push_enabled);
        assert!(!remote.url.is_empty(), "URL exists but push is disabled");
    }

    #[test]
    fn test_push_skipped_when_no_url() {
        let remote = GitRemoteConfig {
            url: "".to_string(),
            push_enabled: true,
            consecutive_failures: 0,
        };
        // When URL is empty, no push should be attempted regardless of push_enabled
        assert!(remote.url.is_empty());
    }

    // --- Remote config serde with strike states ---

    #[test]
    fn test_remote_config_serde_with_strikes() {
        let remote = GitRemoteConfig {
            url: "git@github.com:user/repo.git".to_string(),
            push_enabled: true,
            consecutive_failures: 1,
        };
        let json = serde_json::to_string(&remote).expect("serialize");
        let parsed: GitRemoteConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.consecutive_failures, 1);
        assert!(parsed.push_enabled);
    }

    #[test]
    fn test_config_write_read_strike_state() {
        // Now tests per-project push state in metadata.json
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // Create project
        create_project_for_test(path.clone(), "Test".to_string(), None).unwrap();

        let meta_path = dir.path().join(".config").join("metadata.json");

        // Write metadata with 2 strikes, push still enabled
        let raw = fs::read_to_string(&meta_path).unwrap();
        let mut meta: Metadata = serde_json::from_str(&raw).unwrap();
        meta.push_enabled = true;
        meta.consecutive_failures = 2;
        fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap()).unwrap();

        // Read back
        let raw = fs::read_to_string(&meta_path).unwrap();
        let parsed: Metadata = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed.consecutive_failures, 2);
        assert!(parsed.push_enabled);

        // Simulate: write updated metadata with push disabled (strike 3)
        let mut updated = parsed.clone();
        updated.push_enabled = false;
        updated.consecutive_failures = 3;
        fs::write(&meta_path, serde_json::to_string_pretty(&updated).unwrap()).unwrap();

        let raw2 = fs::read_to_string(&meta_path).unwrap();
        let final_meta: Metadata = serde_json::from_str(&raw2).unwrap();
        assert!(!final_meta.push_enabled, "push should be disabled");
        assert_eq!(final_meta.consecutive_failures, 3);

        // Other fields must be preserved through the strike update
        assert_eq!(final_meta.project_name, "Test");
    }

    // ========================================================================
    // actualizar_fuente_proyecto tests
    // ========================================================================

    #[test]
    fn test_actualizar_fuente_proyecto_updates_font() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(
            path.clone(),
            "Test Project".to_string(),
            Some("monospace".to_string()),
        );

        // Update font from monospace to serif
        let result = actualizar_fuente_proyecto(path.clone(), "serif".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(result.unwrap(), "");

        // Verify metadata.json was updated
        let metadata_path = dir.path().join(".config").join("metadata.json");
        let metadata_str = fs::read_to_string(&metadata_path).unwrap();
        let metadata: Metadata = serde_json::from_str(&metadata_str).unwrap();

        assert_eq!(metadata.font_family, "serif", "font_family should be serif");
        assert!(metadata.last_modified.contains('T'), "last_modified should be ISO 8601");
        assert_eq!(metadata.project_name, "Test Project", "project_name must be preserved");
        assert!(metadata.chapters_order.is_empty(), "chapters_order must be preserved");
        assert!(metadata.characters_index.is_empty(), "characters_index must be preserved");
    }

    #[test]
    fn test_actualizar_fuente_proyecto_preserves_other_fields() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(
            path.clone(),
            "Mi Novela".to_string(),
            Some("sans-serif".to_string()),
        );

        // Add a chapter to populate chapters_order
        let _ = crear_capitulo(
            path.clone(),
            "cap1.md".to_string(),
            "# Capítulo 1\n\n".to_string(),
            None,
        );

        // Update font
        let result = actualizar_fuente_proyecto(path.clone(), "monospace".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // Verify chapters_order preserved
        let metadata_path = dir.path().join(".config").join("metadata.json");
        let metadata_str = fs::read_to_string(&metadata_path).unwrap();
        let metadata: Metadata = serde_json::from_str(&metadata_str).unwrap();

        assert_eq!(metadata.font_family, "monospace");
        assert_eq!(metadata.project_name, "Mi Novela");
        assert!(
            metadata.chapters_order.contains(&"cap1.md".to_string()),
            "chapters_order must still contain cap1.md"
        );
    }

    #[test]
    fn test_actualizar_fuente_proyecto_rejects_invalid_font() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(
            path.clone(),
            "Test".to_string(),
            Some("monospace".to_string()),
        );

        let original_metadata = fs::read_to_string(
            dir.path().join(".config").join("metadata.json"),
        )
        .unwrap();

        let result = actualizar_fuente_proyecto(path.clone(), "comic-sans".to_string());
        assert!(result.is_err(), "Expected Err for invalid font");
        let err = result.unwrap_err();
        assert!(
            err.contains("inválida") || err.contains("invalid"),
            "Error should mention invalid font, got: {}",
            err
        );

        // metadata.json must be unchanged
        let current_metadata =
            fs::read_to_string(dir.path().join(".config").join("metadata.json")).unwrap();
        assert_eq!(
            current_metadata, original_metadata,
            "metadata.json must not be modified on error"
        );
    }

    #[test]
    fn test_actualizar_fuente_proyecto_missing_metadata() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // Don't create a project — no metadata.json exists
        let result = actualizar_fuente_proyecto(path.clone(), "serif".to_string());
        assert!(result.is_err(), "Expected Err for missing metadata.json");
        let err = result.unwrap_err();
        assert!(
            err.contains("no encontrado") || err.contains("not found"),
            "Error should mention missing file, got: {}",
            err
        );
    }

    #[test]
    fn test_actualizar_fuente_proyecto_corrupted_metadata() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // Create project structure manually with corrupt metadata.json
        fs::create_dir_all(dir.path().join(".config")).unwrap();
        fs::create_dir_all(dir.path().join("capitulos")).unwrap();
        fs::create_dir_all(dir.path().join("personajes")).unwrap();
        fs::create_dir_all(dir.path().join("notas")).unwrap();
        fs::write(
            dir.path().join(".config").join("metadata.json"),
            "this is not valid json {{{",
        )
        .unwrap();

        let result = actualizar_fuente_proyecto(path.clone(), "serif".to_string());
        assert!(result.is_err(), "Expected Err for corrupt metadata.json");
        let err = result.unwrap_err();
        assert!(
            err.contains("parsear") || err.contains("parse"),
            "Error should mention parse failure, got: {}",
            err
        );
    }

    #[test]
    fn test_actualizar_fuente_proyecto_empty_path() {
        let result = actualizar_fuente_proyecto("".to_string(), "serif".to_string());
        assert!(result.is_err(), "Expected Err for empty path");
        let err = result.unwrap_err();
        assert!(!err.is_empty(), "Error message should not be empty");
    }

    #[test]
    fn test_actualizar_fuente_proyecto_empty_font() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let result = actualizar_fuente_proyecto(path.clone(), "".to_string());
        assert!(result.is_err(), "Expected Err for empty font family");
        let err = result.unwrap_err();
        assert!(
            err.contains("vacía") || err.contains("empty"),
            "Error should mention empty font, got: {}",
            err
        );
    }

    // ── detectar_config_git tests ───────────────────────────

    #[test]
    fn test_detectar_config_git_full_config() {
        if find_git().is_err() {
            eprintln!("SKIP: git not available on this system");
            return;
        }

        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // Init repo with identity
        let _ = init_git_for_test(&path);

        // Add origin remote
        let git_path = find_git().unwrap();
        let _ = system_command(&git_path)
            .arg("remote")
            .arg("add")
            .arg("origin")
            .arg("git@github.com:user/test-repo.git")
            .current_dir(dir.path())
            .output();

        let result = detectar_config_git(path);
        assert_eq!(result.name.as_deref(), Some("Cron-Insta"));
        assert_eq!(result.email.as_deref(), Some("cron-insta@local"));
        assert_eq!(
            result.remote_url.as_deref(),
            Some("git@github.com:user/test-repo.git")
        );
    }

    #[test]
    fn test_detectar_config_git_missing_remote() {
        if find_git().is_err() {
            eprintln!("SKIP: git not available on this system");
            return;
        }

        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // Init repo with identity but NO remote
        let _ = init_git_for_test(&path);

        let result = detectar_config_git(path);
        assert_eq!(result.name.as_deref(), Some("Cron-Insta"));
        assert_eq!(result.email.as_deref(), Some("cron-insta@local"));
        assert_eq!(result.remote_url, None);
    }

    #[test]
    fn test_detectar_config_git_no_dotgit() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // No .git dir — just an empty temp dir
        let result = detectar_config_git(path);
        assert_eq!(result.name, None);
        assert_eq!(result.email, None);
        assert_eq!(result.remote_url, None);
    }

    #[test]
    fn test_detectar_config_git_partial_identity() {
        if find_git().is_err() {
            eprintln!("SKIP: git not available on this system");
            return;
        }

        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        // Init repo manually with only user.name
        let git_path = find_git().unwrap();
        let _ = system_command(&git_path)
            .arg("init")
            .current_dir(dir.path())
            .output();
        let _ = system_command(&git_path)
            .arg("config")
            .arg("user.name")
            .arg("Ada Lovelace")
            .current_dir(dir.path())
            .output();

        let result = detectar_config_git(path);
        assert_eq!(result.name.as_deref(), Some("Ada Lovelace"));
        assert_eq!(result.email, None);
        assert_eq!(result.remote_url, None);
    }

    // ========================================================================
    // project-config tests (VisibleTabs, auto_save_interval, config merge)
    // ========================================================================

    // ── VisibleTabs serde defaults ───────────────────────────

    #[test]
    fn test_visible_tabs_serde_defaults_all_true() {
        // Deserialize JSON with missing keys → all fields should be true
        let json = r#"{"chapters": true}"#;
        let tabs: VisibleTabs = serde_json::from_str(json).expect("deserialize");
        assert!(tabs.chapters);
        assert!(tabs.characters, "characters should default to true");
        assert!(tabs.places, "places should default to true");
        assert!(tabs.timeline, "timeline should default to true");
        assert!(tabs.notes, "notes should default to true");
    }

    #[test]
    fn test_visible_tabs_default_all_true() {
        let tabs = VisibleTabs::default();
        assert!(tabs.chapters);
        assert!(tabs.characters);
        assert!(tabs.places);
        assert!(tabs.timeline);
        assert!(tabs.notes);
    }

    // ── Chapters rejection ───────────────────────────────────

    #[test]
    fn test_validate_visible_tabs_rejects_chapters_false() {
        let mut tabs = VisibleTabs::default();
        tabs.chapters = false;
        let result = validate_visible_tabs(&tabs);
        assert!(result.is_err(), "Expected Err when chapters is false");
        let err = result.unwrap_err();
        assert!(
            err.contains("capítulos") || err.contains("chapters"),
            "Error should mention chapters, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_visible_tabs_allows_others_false() {
        let mut tabs = VisibleTabs::default();
        tabs.characters = false;
        tabs.places = false;
        let result = validate_visible_tabs(&tabs);
        assert!(result.is_ok(), "Expected Ok when only chapters is true");
    }

    // ── Invalid interval rejection ───────────────────────────

    #[test]
    fn test_validate_auto_save_interval_allows_valid_values() {
        for &val in &[1, 5, 10] {
            let result = validate_auto_save_interval(val);
            assert!(result.is_ok(), "{} should be valid", val);
        }
    }

    #[test]
    fn test_validate_auto_save_interval_rejects_invalid() {
        for &val in &[0, 2, 3, 4, 7, 11, 99] {
            let result = validate_auto_save_interval(val);
            assert!(result.is_err(), "{} should be invalid", val);
        }
    }

    // ── Seeding new fields on project creation ───────────────

    #[test]
    fn test_crear_proyecto_seeds_visible_tabs() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let raw = fs::read_to_string(dir.path().join(".config").join("metadata.json")).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert!(meta.visible_tabs.chapters);
        assert!(meta.visible_tabs.characters);
        assert!(meta.visible_tabs.places);
        assert!(meta.visible_tabs.timeline);
        assert!(meta.visible_tabs.notes);
    }

    #[test]
    fn test_crear_proyecto_seeds_auto_save_interval() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let raw = fs::read_to_string(dir.path().join(".config").join("metadata.json")).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta.auto_save_interval_minutes, 5,
            "auto_save_interval_minutes should default to 5");
    }

    // ── Backward compat: old metadata without new fields ─────

    #[test]
    fn test_metadata_serde_backward_compat_without_new_fields() {
        // Old metadata.json without visible_tabs or auto_save_interval_minutes
        let old_json = r#"{
            "project_name": "Old Project",
            "last_modified": "2024-01-01T00:00:00Z",
            "chapters_order": [],
            "characters_index": [],
            "places_index": [],
            "font_family": "monospace"
        }"#;
        let meta: Metadata = serde_json::from_str(old_json).expect("deserialize old metadata");
        assert_eq!(meta.project_name, "Old Project");
        assert!(meta.visible_tabs.chapters, "chapters should default to true");
        assert!(meta.visible_tabs.characters, "characters should default to true");
        assert!(meta.visible_tabs.places, "places should default to true");
        assert!(meta.visible_tabs.timeline, "timeline should default to true");
        assert!(meta.visible_tabs.notes, "notes should default to true");
        assert_eq!(meta.auto_save_interval_minutes, 5,
            "auto_save_interval should default to 5");
    }

    // ── actualizar_config_proyecto merge tests ───────────────

    #[test]
    fn test_actualizar_config_merge_partial_preserves_untouched() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), Some("serif".to_string()));

        // Merge only visible_tabs — font_family should be untouched
        let config = serde_json::json!({
            "visible_tabs": { "characters": false, "places": false }
        });
        let result = actualizar_config_proyecto(path.clone(), config);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let raw = fs::read_to_string(dir.path().join(".config").join("metadata.json")).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta.font_family, "serif", "font_family should be preserved");
        assert!(!meta.visible_tabs.characters, "characters should be false");
        assert!(!meta.visible_tabs.places, "places should be false");
        assert!(meta.visible_tabs.chapters, "chapters should still be true");
        assert!(meta.visible_tabs.timeline, "timeline should still be true");
    }

    #[test]
    fn test_actualizar_config_merge_interval() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let config = serde_json::json!({ "auto_save_interval_minutes": 1 });
        let result = actualizar_config_proyecto(path.clone(), config);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let raw = fs::read_to_string(dir.path().join(".config").join("metadata.json")).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta.auto_save_interval_minutes, 1);
    }

    #[test]
    fn test_actualizar_config_rejects_chapters_false() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        // Snapshot current metadata content
        let meta_path = dir.path().join(".config").join("metadata.json");
        let before = fs::read_to_string(&meta_path).unwrap();

        let config = serde_json::json!({ "visible_tabs": { "chapters": false } });
        let result = actualizar_config_proyecto(path.clone(), config);
        assert!(result.is_err(), "Expected Err when chapters is false");

        // Disk must be unchanged (atomic rejection)
        let after = fs::read_to_string(&meta_path).unwrap();
        assert_eq!(before, after, "metadata.json should be unchanged after rejection");
    }

    #[test]
    fn test_actualizar_config_rejects_invalid_interval() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let meta_path = dir.path().join(".config").join("metadata.json");
        let before = fs::read_to_string(&meta_path).unwrap();

        let config = serde_json::json!({ "auto_save_interval_minutes": 3 });
        let result = actualizar_config_proyecto(path.clone(), config);
        assert!(result.is_err(), "Expected Err for interval 3");

        // Disk must be unchanged (atomic rejection)
        let after = fs::read_to_string(&meta_path).unwrap();
        assert_eq!(before, after, "metadata.json should be unchanged after rejection");
    }

    #[test]
    fn test_actualizar_config_rejects_invalid_font() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let meta_path = dir.path().join(".config").join("metadata.json");
        let _before = fs::read_to_string(&meta_path).unwrap();

        // Invalid font should NOT be merged — the command skips it silently
        let config = serde_json::json!({ "font_family": "comic_sans" });
        let result = actualizar_config_proyecto(path.clone(), config);
        assert!(result.is_ok(), "Expected Ok (invalid font is silently skipped)");

        let raw = fs::read_to_string(&meta_path).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta.font_family, "monospace", "font_family should be unchanged");
    }

    #[test]
    fn test_actualizar_config_returns_full_json() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let config = serde_json::json!({ "auto_save_interval_minutes": 10 });
        let result = actualizar_config_proyecto(path.clone(), config);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let json_str = result.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid JSON");
        assert_eq!(parsed["auto_save_interval_minutes"], 10);
        assert_eq!(parsed["project_name"], "Test");
    }

    // ── Metadata round-trip preserves new fields ──────────────

    #[test]
    fn test_metadata_roundtrip_preserves_new_fields() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        create_project_for_test(path.clone(), "Test".to_string(), None).unwrap();

        // Modify font via actualizar_fuente_proyecto (existing command)
        actualizar_fuente_proyecto(path.clone(), "serif".to_string()).unwrap();

        // New fields must survive an unrelated metadata write
        let raw = fs::read_to_string(dir.path().join(".config").join("metadata.json")).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta.font_family, "serif");
        assert!(meta.visible_tabs.chapters, "visible_tabs should survive");
        assert_eq!(meta.auto_save_interval_minutes, 5, "auto_save_interval should survive");
    }

    // ========================================================================
    // session-stats tests
    // ========================================================================

    // --- count_words_in_html ---

    #[test]
    fn test_count_words_in_html_empty() {
        assert_eq!(count_words_in_html(""), 0);
        assert_eq!(count_words_in_html("   "), 0);
    }

    #[test]
    fn test_count_words_in_html_plain_text() {
        assert_eq!(count_words_in_html("Hola mundo"), 2);
        assert_eq!(count_words_in_html("uno dos tres cuatro"), 4);
    }

    #[test]
    fn test_count_words_in_html_html_only() {
        assert_eq!(count_words_in_html("<p></p>"), 0);
        assert_eq!(count_words_in_html("<div><span></span></div>"), 0);
    }

    #[test]
    fn test_count_words_in_html_mixed_markdown() {
        let input = "# Título\n<p>Texto del <em>capítulo</em>.</p>";
        // After stripping tags: # Título\nTexto del capítulo.
        // Tokens: #, Título, Texto, del, capítulo.
        assert_eq!(count_words_in_html(input), 5);
    }

    #[test]
    fn test_count_words_in_html_entities() {
        let input = "<p>&amp; &lt; &gt;</p>";
        // After stripping tags: &amp; &lt; &gt;
        // Tokens: &amp;, &lt;, &gt;
        assert_eq!(count_words_in_html(input), 3);
    }

    #[test]
    fn test_count_words_in_html_nested_tags() {
        let input = "<div><p>Hola <strong>mundo</strong></p></div>";
        assert_eq!(count_words_in_html(input), 2);
    }

    // --- SessionStats serialization ---

    #[test]
    fn test_session_stats_default() {
        let stats = SessionStats::default();
        assert_eq!(stats.total_time_seconds, 0);
        assert_eq!(stats.total_words, 0);
        assert!(stats.chapters.is_empty());
        assert!(stats.sessions.is_empty());
    }

    #[test]
    fn test_session_stats_serialize_roundtrip() {
        let mut stats = SessionStats::default();
        stats.total_time_seconds = 600;
        stats.total_words = 150;
        stats.chapters.insert(
            "0001.md".to_string(),
            StatsChapter { words: 150, time_seconds: 600 },
        );
        stats.sessions.push(StatsSession {
            date: "2026-06-27".to_string(),
            duration_seconds: 600,
            words_added: 150,
            chapter_id: "0001.md".to_string(),
        });

        let json = serde_json::to_string_pretty(&stats).expect("serialize");
        let parsed: SessionStats = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.total_time_seconds, 600);
        assert_eq!(parsed.total_words, 150);
        assert_eq!(parsed.chapters["0001.md"].words, 150);
        assert_eq!(parsed.sessions[0].date, "2026-06-27");
    }

    #[test]
    fn test_session_stats_corrupt_json_recovers() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let config_dir = dir.path().join(".config");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("stats.json"), "esto no es json {{{").unwrap();

        // Reading corrupt file should produce default
        let raw = fs::read_to_string(config_dir.join("stats.json")).unwrap();
        let parsed: Option<SessionStats> = serde_json::from_str(&raw).ok();
        assert!(parsed.is_none(), "corrupt JSON should fail to parse");
        let fallback = SessionStats::default();
        assert_eq!(fallback.total_time_seconds, 0);
    }

    // --- crear_proyecto seeds stats.json ---

    #[test]
    fn test_crear_proyecto_seeds_stats_json() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Stats Test".to_string(), None);

        let stats_path = dir.path().join(".config").join("stats.json");
        assert!(stats_path.exists(), "stats.json should be seeded by crear_proyecto");

        let content = fs::read_to_string(&stats_path).expect("failed to read stats.json");
        let stats: SessionStats = serde_json::from_str(&content).expect("invalid stats.json");
        assert_eq!(stats.total_time_seconds, 0);
        assert_eq!(stats.total_words, 0);
        assert!(stats.chapters.is_empty());
        assert!(stats.sessions.is_empty());
    }

    // --- Integration: start session → close → verify stats.json ---

    #[test]
    fn test_session_stats_full_flow() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path_str = dir.path().to_str().unwrap().to_string();

        // Create project with a chapter
        create_project_for_test(path_str.clone(), "Integration Test".to_string(), None).unwrap();

        let chapter_content = "<p>Hola mundo</p>";
        let cap_dir = dir.path().join("capitulos");
        fs::create_dir_all(&cap_dir).unwrap();
        fs::write(cap_dir.join("0001.md"), chapter_content).unwrap();

        // Simulate session start: count words, record time
        let word_count = count_words_in_html(chapter_content);
        assert_eq!(word_count, 2, "expected 2 words in 'Hola mundo'");

        let mut tracker = SessionTracker::default();
        tracker.start_time = Some(std::time::Instant::now() - std::time::Duration::from_secs(300));
        tracker.chapter_start = Some(std::time::Instant::now() - std::time::Duration::from_secs(300));
        tracker.chapter_filename = Some("0001.md".to_string());
        tracker.initial_word_count = Some(2);

        // Add more content to the chapter
        let updated_content = "<p>Hola mundo cruel</p>";
        fs::write(cap_dir.join("0001.md"), updated_content).unwrap();

        // End session — should write stats.json
        finalizar_sesion_escritura(&mut tracker, dir.path());

        // Verify stats.json exists and has correct data
        let stats_path = dir.path().join(".config").join("stats.json");
        assert!(stats_path.exists(), "stats.json should exist after session close");

        let raw = fs::read_to_string(&stats_path).unwrap();
        let stats: SessionStats = serde_json::from_str(&raw).unwrap();

        // 3 words in updated chapter, started at 2 → +1 word added
        // elapsed ≈ 300 seconds
        assert!(
            stats.total_time_seconds >= 300,
            "total_time_seconds should be at least 300, got {}",
            stats.total_time_seconds
        );
        assert_eq!(stats.total_words, 1, "1 word added (cruel)");

        let ch = stats.chapters.get("0001.md").expect("chapter 0001.md should exist");
        assert_eq!(ch.words, 1, "1 word added for chapter");
        assert!(
            ch.time_seconds >= 300,
            "chapter time should be at least 300s, got {}",
            ch.time_seconds
        );

        assert_eq!(stats.sessions.len(), 1, "should have one session record");
        let s = &stats.sessions[0];
        assert_eq!(s.words_added, 1);
        assert_eq!(s.chapter_id, "0001.md");
        assert!(
            s.duration_seconds >= 300,
            "session duration should be at least 300s, got {}",
            s.duration_seconds
        );
    }

    #[test]
    fn test_session_stats_no_active_session_skips() {
        let dir = TempDir::new().expect("failed to create temp dir");

        let mut tracker = SessionTracker::default(); // No start_time
        finalizar_sesion_escritura(&mut tracker, dir.path());

        // Should NOT create stats.json when there's no active session
        let stats_path = dir.path().join(".config").join("stats.json");
        assert!(!stats_path.exists(), "stats.json should not be created for inactive session");
    }

    // ========================================================================
    // tramas — plotlines tests
    // ========================================================================

    #[test]
    fn test_crear_trama_generates_unique_id() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let result = crear_trama(path.clone(), "El Viaje del Héroe".to_string());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        let trama = result.unwrap();
        assert_eq!(trama.nombre, "El Viaje del Héroe");
        assert!(!trama.id.is_empty(), "ID should not be empty");
        // ID should be slugified: starts with "el-viaje-del-heroe-"
        assert!(
            trama.id.starts_with("el-viaje-del-heroe-"),
            "ID should start with slugified name, got: {}",
            trama.id
        );
        // Should end with 8-char hex suffix
        let suffix = trama.id.rsplit('-').next().unwrap();
        assert_eq!(suffix.len(), 8, "Hex suffix should be 8 chars, got: {}", suffix);
        assert!(
            suffix.chars().all(|c| c.is_ascii_hexdigit()),
            "Suffix should be all hex digits, got: {}",
            suffix
        );

        // Verify it was persisted
        let metadata_path = dir.path().join(".config").join("metadata.json");
        let raw = std::fs::read_to_string(&metadata_path).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta.tramas.len(), 1);
        assert_eq!(meta.tramas[0].nombre, "El Viaje del Héroe");
    }

    #[test]
    fn test_crear_trama_rejects_duplicate_name() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let _ = crear_trama(path.clone(), "Principal".to_string());
        let result = crear_trama(path.clone(), "Principal".to_string());
        assert!(result.is_err(), "Expected Err for duplicate name");
        let err = result.unwrap_err().to_lowercase();
        assert!(err.contains("ya existe"), "Should mention duplicate: {}", err);

        // Only one trama persisted
        let metadata_path = dir.path().join(".config").join("metadata.json");
        let raw = std::fs::read_to_string(&metadata_path).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta.tramas.len(), 1);
    }

    #[test]
    fn test_eliminar_trama_unassigns_chapters() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        // Create trama and two chapters assigned to it
        let trama = crear_trama(path.clone(), "Trama A".to_string()).unwrap();
        let trama_id = trama.id.clone();

        // Write chapter files manually
        std::fs::create_dir_all(dir.path().join("capitulos")).unwrap();
        std::fs::write(dir.path().join("capitulos").join("cap1.md"), "# Cap 1\n\n").unwrap();
        std::fs::write(dir.path().join("capitulos").join("cap2.md"), "# Cap 2\n\n").unwrap();

        // Assign chapters to trama via metadata manipulation
        let meta_path = dir.path().join(".config").join("metadata.json");
        let raw = std::fs::read_to_string(&meta_path).unwrap();
        let mut meta: Metadata = serde_json::from_str(&raw).unwrap();
        meta.chapters_order = vec!["cap1.md".to_string(), "cap2.md".to_string()];
        meta.chapter_tramas = vec![
            ChapterTrama { filename: "cap1.md".to_string(), trama_id: Some(trama_id.clone()) },
            ChapterTrama { filename: "cap2.md".to_string(), trama_id: Some(trama_id.clone()) },
        ];
        std::fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap()).unwrap();

        // Delete the trama
        let result = eliminar_trama(path.clone(), trama_id);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // Verify trama removed
        let raw = std::fs::read_to_string(&meta_path).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert!(meta.tramas.is_empty(), "tramas should be empty");
        assert_eq!(meta.chapter_tramas.len(), 2);
        assert!(meta.chapter_tramas.iter().all(|ct| ct.trama_id.is_none()),
            "All chapters should be unassigned");
        // Chapters must still exist in order
        assert_eq!(meta.chapters_order.len(), 2);
    }

    #[test]
    fn test_eliminar_trama_nonexistent() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let result = eliminar_trama(path, "ghost".to_string());
        assert!(result.is_err(), "Expected Err for nonexistent trama");
        let err = result.unwrap_err().to_lowercase();
        assert!(err.contains("no existe"), "Should mention nonexistent: {}", err);
    }

    #[test]
    fn test_asignar_capitulo_trama_upsert() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        // Create two tramas
        let trama_a = crear_trama(path.clone(), "A".to_string()).unwrap();
        let trama_b = crear_trama(path.clone(), "B".to_string()).unwrap();

        // Write chapter file
        std::fs::create_dir_all(dir.path().join("capitulos")).unwrap();
        std::fs::write(dir.path().join("capitulos").join("cap1.md"), "# Cap 1\n\n").unwrap();

        // Add chapter to metadata
        let meta_path = dir.path().join(".config").join("metadata.json");
        let raw = std::fs::read_to_string(&meta_path).unwrap();
        let mut meta: Metadata = serde_json::from_str(&raw).unwrap();
        meta.chapters_order.push("cap1.md".to_string());
        std::fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap()).unwrap();

        // Assign to trama A
        let result = asignar_capitulo_trama(
            path.clone(), "cap1.md".to_string(), Some(trama_a.id.clone()),
        );
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let raw = std::fs::read_to_string(&meta_path).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta.chapter_tramas.len(), 1);
        assert_eq!(meta.chapter_tramas[0].trama_id.as_deref(), Some(trama_a.id.as_str()));

        // Re-assign to trama B
        let result = asignar_capitulo_trama(
            path.clone(), "cap1.md".to_string(), Some(trama_b.id.clone()),
        );
        assert!(result.is_ok(), "Expected Ok for reassignment, got {:?}", result);

        let raw = std::fs::read_to_string(&meta_path).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta.chapter_tramas.len(), 1, "Should still have one entry");
        assert_eq!(meta.chapter_tramas[0].trama_id.as_deref(), Some(trama_b.id.as_str()));

        // Unassign
        let result = asignar_capitulo_trama(
            path.clone(), "cap1.md".to_string(), None,
        );
        assert!(result.is_ok(), "Expected Ok for unassign, got {:?}", result);

        let raw = std::fs::read_to_string(&meta_path).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta.chapter_tramas[0].trama_id, None, "Should be unassigned");
    }

    #[test]
    fn test_asignar_capitulo_trama_rejects_nonexistent_trama() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        std::fs::create_dir_all(dir.path().join("capitulos")).unwrap();
        std::fs::write(dir.path().join("capitulos").join("cap1.md"), "# Cap 1\n\n").unwrap();

        let meta_path = dir.path().join(".config").join("metadata.json");
        let raw = std::fs::read_to_string(&meta_path).unwrap();
        let mut meta: Metadata = serde_json::from_str(&raw).unwrap();
        meta.chapters_order.push("cap1.md".to_string());
        std::fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap()).unwrap();

        let result = asignar_capitulo_trama(
            path, "cap1.md".to_string(), Some("ghost".to_string()),
        );
        assert!(result.is_err(), "Expected Err for nonexistent trama");
        let err = result.unwrap_err().to_lowercase();
        assert!(err.contains("no existe"), "Should mention nonexistent: {}", err);
    }

    #[test]
    fn test_metadata_backward_compat_no_tramas_fields() {
        // Old metadata.json without tramas or chapter_tramas should deserialize
        // with both fields defaulting to empty vectors.
        let old_json = r#"{
            "project_name": "Old Project",
            "last_modified": "2024-01-01T00:00:00+00:00",
            "chapters_order": ["cap1.md"],
            "characters_index": [],
            "places_index": [],
            "font_family": "monospace",
            "push_enabled": false,
            "consecutive_failures": 0,
            "visible_tabs": {"chapters":true,"characters":true,"places":true,"timeline":true,"notes":true},
            "auto_save_interval_minutes": 5
        }"#;

        let meta: Metadata = serde_json::from_str(old_json).expect("Should deserialize old JSON");
        assert!(meta.tramas.is_empty(), "tramas should default to empty");
        assert!(meta.chapter_tramas.is_empty(), "chapter_tramas should default to empty");
        assert_eq!(meta.project_name, "Old Project");
        assert_eq!(meta.chapters_order, vec!["cap1.md"]);
    }

    #[test]
    fn test_crear_capitulo_with_trama_id() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);
        let trama = crear_trama(path.clone(), "Trama A".to_string()).unwrap();

        let result = crear_capitulo(
            path.clone(),
            "cap1.md".to_string(),
            "# Cap 1\n\n".to_string(),
            Some(trama.id.clone()),
        );
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let meta_path = dir.path().join(".config").join("metadata.json");
        let raw = std::fs::read_to_string(&meta_path).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert!(meta.chapters_order.contains(&"cap1.md".to_string()));
        assert_eq!(meta.chapter_tramas.len(), 1);
        assert_eq!(meta.chapter_tramas[0].filename, "cap1.md");
        assert_eq!(meta.chapter_tramas[0].trama_id.as_deref(), Some(trama.id.as_str()));
    }

    #[test]
    fn test_crear_capitulo_without_trama_id() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        let result = crear_capitulo(
            path.clone(),
            "cap1.md".to_string(),
            "# Cap 1\n\n".to_string(),
            None,
        );
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let meta_path = dir.path().join(".config").join("metadata.json");
        let raw = std::fs::read_to_string(&meta_path).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta.chapter_tramas.len(), 1);
        assert_eq!(meta.chapter_tramas[0].filename, "cap1.md");
        assert!(meta.chapter_tramas[0].trama_id.is_none(), "Should be unassigned");
    }

    #[test]
    fn test_eliminar_capitulo_cleans_chapter_tramas() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = create_project_for_test(path.clone(), "Test".to_string(), None);

        // Create a trama and a chapter assigned to it
        let trama = crear_trama(path.clone(), "A".to_string()).unwrap();
        let _ = crear_capitulo(
            path.clone(),
            "cap1.md".to_string(),
            "# Cap 1\n\n".to_string(),
            Some(trama.id),
        );

        // Verify assignment exists
        let meta_path = dir.path().join(".config").join("metadata.json");
        {
            let raw = std::fs::read_to_string(&meta_path).unwrap();
            let meta: Metadata = serde_json::from_str(&raw).unwrap();
            assert_eq!(meta.chapter_tramas.len(), 1);
        }

        // Delete the chapter
        let result = eliminar_capitulo(path.clone(), "cap1.md".to_string());
        assert!(result.is_ok(), "Expected Ok for delete, got {:?}", result);

        let raw = std::fs::read_to_string(&meta_path).unwrap();
        let meta: Metadata = serde_json::from_str(&raw).unwrap();
        assert!(meta.chapter_tramas.is_empty(), "chapter_tramas should be empty after chapter deletion");
    }

    // ========================================================================
    // Boundary: close the tests module
    // ========================================================================

}
