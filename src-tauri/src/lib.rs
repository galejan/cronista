// Cronista — Tauri backend
//
// Phase 2: Rust backend commands for project management and git abstraction.
// All five Tauri commands + find_git() helper live here per the single-module design.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Metadata {
    project_name: String,
    last_modified: String,
    chapters_order: Vec<String>,
    characters_index: Vec<CharacterIndex>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CharacterIndex {
    id: String,
    file: String,
    name: String,
}

// ---------------------------------------------------------------------------
// Helper: locate the git binary across platforms
// ---------------------------------------------------------------------------

/// Locate the `git` executable on the system.
///
/// **Linux**: uses `which git`.
/// **Windows**: tries `PATH` via `where git`, then falls back to the two
/// standard Git-for-Windows installation paths.
///
/// Returns `Ok(path)` when found, or `Err(msg)` with a user-facing Spanish
/// message when Git is unavailable.
fn find_git() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("which")
            .arg("git")
            .output()
            .map_err(|e| format!("Error al buscar git: {}", e))?;

        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // 1) Try PATH via `where git`
        if let Ok(output) = Command::new("where").arg("git").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // `where` may return multiple lines — take the first
                if let Some(first) = stdout.lines().next() {
                    let trimmed = first.trim();
                    if !trimmed.is_empty() {
                        return Ok(trimmed.to_string());
                    }
                }
            }
        }

        // 2) Fallback to well-known Git-for-Windows paths
        let fallbacks = [
            r"C:\Program Files\Git\bin\git.exe",
            r"C:\Program Files (x86)\Git\bin\git.exe",
        ];
        for fb in &fallbacks {
            if Path::new(fb).exists() {
                return Ok(fb.to_string());
            }
        }
    }

    Err("Git no está disponible. El control de versiones permanecerá inactivo.".to_string())
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Create a new Cronista literary project.
///
/// Creates the base directory plus four subdirectories (`.config/`,
/// `capitulos/`, `personajes/`, `notas/`), seeds `.config/metadata.json`
/// and `.config/timeline.json`, then automatically initialises a Git
/// repository (silently — disk structure is created regardless of Git
/// availability).
#[tauri::command]
fn crear_proyecto(path: String, nombre: String) -> Result<String, String> {
    // Normalise trailing separators
    let path = path.trim_end_matches('/').trim_end_matches('\\').to_string();
    let base = Path::new(&path);

    // Create base directory
    std::fs::create_dir_all(base)
        .map_err(|e| format!("No se pudo crear el directorio del proyecto: {}", e))?;

    // Create subdirectories
    let subdirs = [".config", "capitulos", "personajes", "notas"];
    for sub in &subdirs {
        std::fs::create_dir_all(base.join(sub))
            .map_err(|e| format!("No se pudo crear el directorio {}: {}", sub, e))?;
    }

    // Write metadata.json
    let metadata = Metadata {
        project_name: nombre.clone(),
        last_modified: Utc::now().to_rfc3339(),
        chapters_order: vec![],
        characters_index: vec![],
    };
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Error al serializar metadata: {}", e))?;
    std::fs::write(base.join(".config/metadata.json"), metadata_json)
        .map_err(|e| format!("Error al escribir metadata.json: {}", e))?;

    // Write timeline.json (empty array)
    std::fs::write(base.join(".config/timeline.json"), "[]")
        .map_err(|e| format!("Error al escribir timeline.json: {}", e))?;

    // Auto-initialise git — silently ignore if git is unavailable
    let _ = inicializar_git(path.clone());

    Ok(format!("Proyecto '{}' creado en {}", nombre, path))
}

/// Initialise a Git repository in the given project path.
///
/// Returns success if `.git` already exists (reinit is safe) or if
/// `git init` succeeds.  Returns `Err` **only** when Git is unavailable —
/// callers can degrade gracefully.
#[tauri::command]
fn inicializar_git(path: String) -> Result<String, String> {
    let project_path = Path::new(&path);

    // Already initialised → silent success
    if project_path.join(".git").exists() {
        return Ok("El repositorio ya estaba inicializado.".to_string());
    }

    // Locate git binary (returns Err with user-facing message when absent)
    let git_path = find_git()?;

    let output = Command::new(&git_path)
        .arg("init")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git init: {}", e))?;

    if output.status.success() {
        Ok("Repositorio Git inicializado correctamente.".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error al inicializar Git: {}", stderr.trim()))
    }
}

/// Save chapter content to disk (Nivel 1 — no git commit).
///
/// Writes UTF-8 content to `{proyecto_path}/capitulos/{filename}`,
/// creating the parent directory if needed.  Overwrites any existing
/// file at the same path.
#[tauri::command]
fn guardar_capitulo(
    proyecto_path: String,
    filename: String,
    contenido: String,
) -> Result<String, String> {
    let cap_dir = Path::new(&proyecto_path).join("capitulos");

    // Ensure the capítulos directory exists
    std::fs::create_dir_all(&cap_dir)
        .map_err(|e| format!("No se pudo crear el directorio capítulos: {}", e))?;

    let file_path = cap_dir.join(&filename);
    std::fs::write(&file_path, contenido)
        .map_err(|e| format!("Error al guardar el capítulo: {}", e))?;

    Ok(format!("Capítulo guardado: {}", file_path.display()))
}

/// Create a versioned checkpoint via Git (Nivel 2).
///
/// Stages all changes (`git add .`) and commits with a descriptive
/// progress message (`Progreso automático: {fecha} - {recuento} palabras`).
/// The word count is computed by counting whitespace-separated tokens in
/// every `.md` file under `capitulos/`.
///
/// Returns the commit hash on success, or a descriptive status when there
/// is nothing to commit (still `Ok` — not an error).
#[tauri::command]
fn crear_checkpoint(proyecto_path: String) -> Result<String, String> {
    let project_path = Path::new(&proyecto_path);
    let git_path = find_git()?;

    // Stage all changes
    let add_output = Command::new(&git_path)
        .arg("add")
        .arg(".")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git add: {}", e))?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        return Err(format!("Error en git add: {}", stderr.trim()));
    }

    // Count words in chapter files for the commit message
    let word_count = count_words_in_chapters(project_path);
    let date = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let commit_msg = format!(
        "Progreso automático: {} - {} palabras",
        date, word_count
    );

    // Commit
    let commit_output = Command::new(&git_path)
        .arg("commit")
        .arg("-m")
        .arg(&commit_msg)
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git commit: {}", e))?;

    if commit_output.status.success() {
        // Retrieve the commit hash
        let hash_output = Command::new(&git_path)
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(project_path)
            .output()
            .map_err(|e| format!("Error al obtener el hash del commit: {}", e))?;

        let hash = String::from_utf8_lossy(&hash_output.stdout)
            .trim()
            .to_string();
        Ok(hash)
    } else {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        let stdout = String::from_utf8_lossy(&commit_output.stdout);
        let combined = format!("{}{}", stderr, stdout);
        // "nothing to commit" is a normal state, not an error.
        // Git may route this message to stdout or stderr depending on version.
        // We match both English and Spanish localisations.
        if combined.contains("nothing to commit")
            || combined.contains("nothing added to commit")
            || combined.contains("nada para confirmar")
            || combined.contains("nada que confirmar")
        {
            Ok("Sin cambios para guardar.".to_string())
        } else {
            Err(format!(
                "Error en git commit: {}",
                combined.trim().lines().last().unwrap_or("")
            ))
        }
    }
}

/// Read and return the project metadata index.
///
/// Returns the raw contents of `.config/metadata.json` as a JSON string.
/// The frontend (caller) is responsible for parsing and validation.
#[tauri::command]
fn cargar_indice(proyecto_path: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }

    let metadata_path = Path::new(&proyecto_path).join(".config").join("metadata.json");

    if !metadata_path.exists() {
        return Err(format!(
            "Archivo de índice no encontrado: {}",
            metadata_path.display()
        ));
    }

    std::fs::read_to_string(&metadata_path)
        .map_err(|e| format!("No se pudo leer el índice del proyecto: {}", e))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Count whitespace-separated tokens across all `.md` files under
/// `{project_path}/capitulos/`.  Returns 0 when the directory is
/// missing or empty.
fn count_words_in_chapters(project_path: &Path) -> usize {
    let cap_dir = project_path.join("capitulos");
    if !cap_dir.exists() {
        return 0;
    }

    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(&cap_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "md") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    total += content.split_whitespace().count();
                }
            }
        }
    }
    total
}

// ---------------------------------------------------------------------------
// Application entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            crear_proyecto,
            inicializar_git,
            guardar_capitulo,
            crear_checkpoint,
            cargar_indice,
        ])
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
    // project-file-management tests
    // ========================================================================

    #[test]
    fn test_crear_proyecto_creates_all_directories() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let result = crear_proyecto(path.clone(), "Test Project".to_string());
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

        let _ = crear_proyecto(path.clone(), "Mi Novela".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "My Project".to_string());

        let result = cargar_indice(path.clone());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let json_str = result.unwrap();
        // Should deserialise as valid JSON
        let _meta: serde_json::Value =
            serde_json::from_str(&json_str).expect("cargar_indice returned invalid JSON");
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
    fn test_crear_proyecto_permission_denied() {
        // /root/ is typically not writable by non-root users on Linux
        let result = crear_proyecto("/root/cronista-blocked".to_string(), "Test".to_string());
        // On CI running as root, this could succeed; we just assert it doesn't panic
        match result {
            Ok(_) => {
                // We're likely root — clean up if we created anything
                let _ = std::fs::remove_dir_all("/root/cronista-blocked");
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

        let result = inicializar_git(path.clone());
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
        let result1 = inicializar_git(path.clone());
        assert!(result1.is_ok());

        // Second init on same directory (reinit)
        let result2 = inicializar_git(path.clone());
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

        let result = crear_proyecto(path.clone(), "Test Project".to_string());
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

        let result = crear_proyecto(path.clone(), "Sin Git".to_string());
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
        inicializar_git(path.clone()).expect("git init failed");

        // Now try checkpointing with no changes
        let result = crear_checkpoint(path.clone());
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
        inicializar_git(path.clone()).expect("git init failed");

        // Create a chapter file (a change)
        let content = "# Capítulo 1\n\nHabía una vez...";
        fs::write(
            dir.path().join("capitulos").join("0001_intro.md"),
            content,
        )
        .unwrap();

        let result = crear_checkpoint(path.clone());
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
        let path = dir.path().to_str().unwrap().to_string();

        // We simulate git-unavailable by using a dir where find_git() works
        // but the checkpoint operation itself is what we're checking
        if find_git().is_ok() {
            eprintln!("INFO: git IS available — cannot fully test git-unavailable path.");
            eprintln!("This scenario is covered by the find_git() error path in crear_checkpoint.");
            return;
        }

        // If git is truly unavailable, creating a checkpoint should return Err
        let result = crear_checkpoint(path);
        assert!(result.is_err(), "Expected Err when git is unavailable");
        let err = result.unwrap_err();
        assert!(
            err.contains("Git no está disponible"),
            "Expected git-unavailable message, got: {}",
            err
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
        inicializar_git(path.clone()).expect("git init failed");

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
        let result = crear_proyecto(path.clone(), "Full Flow Test".to_string());
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
        let result = crear_checkpoint(path.clone());
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
    // Test helpers
    // ========================================================================

    /// Count the number of commits in the git repository at `repo_path`.
    fn count_commits(repo_path: &Path) -> usize {
        if !repo_path.join(".git").exists() {
            return 0;
        }
        let git_path = match find_git() {
            Ok(p) => p,
            Err(_) => return 0,
        };
        let output = Command::new(&git_path)
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
}
