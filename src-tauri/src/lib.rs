// Cronista — Tauri backend
//
// Phase 2: Rust backend commands for project management and git abstraction.
// All five Tauri commands + find_git() helper live here per the single-module design.

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use tauri::Manager;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Tauri managed state: tracks the active project for close-time checkpoint.
struct ProjectState {
    active_project: Mutex<Option<String>>,
    closing: Mutex<bool>,
}

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

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Character {
    id: String,
    name: String,
    #[serde(default)]
    physicalDescription: Option<String>,
    #[serde(default)]
    personality: Option<String>,
    #[serde(default)]
    traumas: Option<String>,
    #[serde(default)]
    relationships: Vec<Relationship>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Relationship {
    #[serde(default)]
    targetId: Option<String>,
    targetName: String,
    #[serde(rename = "type")]
    rel_type: String,
    #[serde(default)]
    notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CharacterIndexItem {
    id: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NoteIndexItem {
    id: String,
    title: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct TimelineEvent {
    #[serde(default)]
    id: String,
    date: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    relatedCharacters: Vec<String>,
    #[serde(default)]
    relatedChapters: Vec<String>,
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
        last_modified: Local::now().to_rfc3339(),
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

/// Copy the app icon into the project and set it as folder icon via gvfs.
///
/// Best-effort — never fails. Works on GNOME, Nemo, and other file
/// managers that respect GVFS metadata. Call after project creation.
#[tauri::command]
fn marcar_proyecto_cronista(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let base = Path::new(&path);

    // Copy icon from bundled resources to project root
    let icon_dest = base.join(".cronista-icon.png");
    if let Ok(resource_dir) = app.path().resource_dir() {
        let icon_src = resource_dir.join("icons/32x32.png");
        if icon_src.exists() {
            std::fs::copy(&icon_src, &icon_dest)
                .map_err(|e| format!("Error al copiar icono: {}", e))?;
        }
    }

    // Try to set folder icon via gvfs (GNOME, Cinnamon, Nemo, etc.)
    if let Ok(icon_abs) = icon_dest.canonicalize() {
        let icon_uri = format!("file://{}", icon_abs.display());
        let _ = std::process::Command::new("gio")
            .arg("set")
            .arg("-t")
            .arg("string")
            .arg(base)
            .arg("metadata::custom-icon")
            .arg(&icon_uri)
            .output();
    }

    Ok(())
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
        // Set anonymous user for commits (best-effort, silent on failure)
        let _ = Command::new(&git_path)
            .arg("config")
            .arg("user.name")
            .arg("Cronista")
            .current_dir(project_path)
            .output();
        let _ = Command::new(&git_path)
            .arg("config")
            .arg("user.email")
            .arg("cronista@local")
            .current_dir(project_path)
            .output();

        Ok("Repositorio Git inicializado correctamente.".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error al inicializar Git: {}", stderr.trim()))
    }
}

/// Initialize git with custom author and create the first commit.
///
/// Stores author info in `.config/git-config.json` for future reference.
/// After `git init` + user config, stages everything and creates an
/// initial commit: "Primera piedra — {project_name}".
#[tauri::command]
fn inicializar_git_con_autor(
    path: String,
    nombre: String,
    email: String,
) -> Result<String, String> {
    let project_path = Path::new(&path);

    if project_path.join(".git").exists() {
        return Ok("El repositorio ya estaba inicializado.".to_string());
    }

    let git_path = find_git()?;

    // git init
    let output = Command::new(&git_path)
        .arg("init")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git init: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Error al inicializar Git: {}", stderr.trim()));
    }

    // Configure author
    let _ = Command::new(&git_path)
        .arg("config")
        .arg("user.name")
        .arg(&nombre)
        .current_dir(project_path)
        .output();
    let _ = Command::new(&git_path)
        .arg("config")
        .arg("user.email")
        .arg(&email)
        .current_dir(project_path)
        .output();

    // Save config for future reference
    let config = serde_json::json!({ "name": nombre, "email": email });
    let config_json = serde_json::to_string_pretty(&config).unwrap_or_default();
    let _ = std::fs::write(
        project_path.join(".config/git-config.json"),
        config_json,
    );

    // First commit — "Primera piedra"
    let _ = Command::new(&git_path)
        .arg("add")
        .arg(".")
        .current_dir(project_path)
        .output();

    let commit_msg = "Primera piedra ✍️";
    let commit_output = Command::new(&git_path)
        .arg("commit")
        .arg("-m")
        .arg(commit_msg)
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error en primer commit: {}", e))?;

    if commit_output.status.success() {
        Ok("Repositorio Git inicializado y primer commit creado.".to_string())
    } else {
        // "nothing to commit" is fine — project might be empty
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        if stderr.contains("nothing to commit") || stderr.contains("nothing added") {
            Ok("Repositorio Git inicializado (sin archivos para commit aún).".to_string())
        } else {
            Err(format!("Error en primer commit: {}", stderr.trim()))
        }
    }
}

/// Check if the project has a git repository initialized (.git directory).
///
/// Returns true when `<project>/.git` exists, regardless of whether git
/// the binary is installed.
#[tauri::command]
fn verificar_git_inicializado(path: String) -> Result<bool, String> {
    Ok(Path::new(&path).join(".git").exists())
}

/// Return the list of .md files changed in a given commit.
fn get_changed_md_files(
    project_path: &Path,
    git_path: &str,
    hash: &str,
) -> Vec<String> {
    let output = Command::new(git_path)
        .arg("show")
        .arg("--name-only")
        .arg("--format=")
        .arg(hash)
        .current_dir(project_path)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| !l.is_empty() && l.ends_with(".md"))
                .map(|l| {
                    // Show just the filename, not the full path
                    Path::new(l)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| l.to_string())
                })
                .collect()
        }
        _ => vec![],
    }
}

/// Return the last N git log entries for the project.
///
/// Each entry is a JSON object: { hash, date, message, words }.
/// Words are extracted from the commit message's "— N palabras" suffix
/// when present, otherwise shown as "—".
#[tauri::command]
fn obtener_git_log(path: String, limit: usize) -> Result<String, String> {
    let project_path = Path::new(&path);
    let git_path = find_git()?;

    let output = Command::new(&git_path)
        .arg("log")
        .arg(format!("--format=%H|%ai|%s"))
        .arg(format!("-{}", limit.max(1).min(20)))
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al leer el historial: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Error en git log: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let entries: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            let hash_full = parts.first().map(|s| s.to_string()).unwrap_or_default();
            let hash = hash_full.chars().take(7).collect::<String>();
            let date = parts.get(1).unwrap_or(&"—").to_string();
            let raw_msg = parts.get(2).unwrap_or(&"—");

            // Extract word count from the commit message suffix
            let (message, words) = if let Some(pos) = raw_msg.rfind("—") {
                let suffix = raw_msg[pos..].trim();
                if suffix.contains("palabras") || suffix.contains("words") {
                    (raw_msg[..pos].trim().to_string(), suffix.to_string())
                } else {
                    (raw_msg.to_string(), "—".to_string())
                }
            } else {
                (raw_msg.to_string(), "—".to_string())
            };

            // Get changed .md files for this commit
            let files = get_changed_md_files(project_path, &git_path, &hash_full);

            serde_json::json!({
                "hash": hash,
                "date": date,
                "message": message,
                "words": words,
                "files": files,
            })
        })
        .collect();

    serde_json::to_string(&entries)
        .map_err(|e| format!("Error al serializar el historial: {}", e))
}

/// Detect whether Git is installed on the system.
///
/// Returns `true` when `find_git()` locates a valid Git binary.
/// Lightweight command — no I/O beyond binary discovery.
#[tauri::command]
fn detectar_git() -> Result<bool, String> {
    Ok(find_git().is_ok())
}

/// Tell the Rust backend which project is currently open in the frontend.
///
/// Called when a project is opened (path = Some) or closed (path = None).
/// The backend uses this to run a git checkpoint when the window is closed,
/// avoiding the JS→Rust IPC deadlock during `onCloseRequested`.
#[tauri::command]
fn set_active_project(
    state: tauri::State<ProjectState>,
    path: Option<String>,
) -> Result<(), String> {
    let mut active = state.active_project.lock().map_err(|e| e.to_string())?;
    *active = path;
    Ok(())
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
    let date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
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

/// Read a single chapter file from disk.
///
/// Returns the UTF-8 content of `{proyecto_path}/capitulos/{filename}`.
/// The frontend is responsible for parsing and rendering the markdown/HTML.
#[tauri::command]
fn cargar_capitulo(proyecto_path: String, filename: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if filename.trim().is_empty() {
        return Err("El nombre del archivo no puede estar vacío.".to_string());
    }

    let file_path = Path::new(&proyecto_path).join("capitulos").join(&filename);

    if !file_path.exists() {
        return Err(format!("Archivo no encontrado: {}", file_path.display()));
    }

    std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Error al leer el capítulo: {}", e))
}

/// Create a new chapter .md file and register it in metadata.json.
///
/// 1. Rejects duplicates (file already exists in `capitulos/`).
/// 2. Writes the `.md` file first.
/// 3. Updates `metadata.json`: appends `filename` to `chapters_order`
///    and refreshes `last_modified`.
///
/// Write order (file first, then metadata) prevents an index entry
/// pointing to a missing file in case of a crash mid-operation.
#[tauri::command]
fn crear_capitulo(
    proyecto_path: String,
    filename: String,
    contenido: String,
) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if filename.trim().is_empty() {
        return Err("El nombre del archivo no puede estar vacío.".to_string());
    }

    let cap_dir = Path::new(&proyecto_path).join("capitulos");
    let file_path = cap_dir.join(&filename);

    // Reject duplicates
    if file_path.exists() {
        return Err(format!("El capítulo '{}' ya existe.", filename));
    }

    // Ensure the capítulos directory exists
    std::fs::create_dir_all(&cap_dir)
        .map_err(|e| format!("No se pudo crear el directorio capítulos: {}", e))?;

    // 1) Write the .md file first
    std::fs::write(&file_path, &contenido)
        .map_err(|e| format!("Error al crear el capítulo: {}", e))?;

    // 2) Update metadata.json
    let metadata_path = Path::new(&proyecto_path).join(".config").join("metadata.json");

    if !metadata_path.exists() {
        return Err(format!(
            "Archivo de metadatos no encontrado: {}",
            metadata_path.display()
        ));
    }

    let metadata_str = std::fs::read_to_string(&metadata_path)
        .map_err(|e| format!("Error al leer metadata.json: {}", e))?;

    let mut metadata: Metadata = serde_json::from_str(&metadata_str)
        .map_err(|e| format!("Error al parsear metadata.json: {}", e))?;

    metadata.chapters_order.push(filename.clone());
    metadata.last_modified = Local::now().to_rfc3339();

    let updated_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Error al serializar metadata.json: {}", e))?;

    std::fs::write(&metadata_path, updated_json)
        .map_err(|e| format!("Error al escribir metadata.json: {}", e))?;

    Ok(format!("Capítulo creado: {}", file_path.display()))
}

/// Delete a chapter.
///
/// 1. Validates non-empty path and filename.
/// 2. Deletes `capitulos/{filename}` — returns error if not found.
/// 3. Removes `filename` from `chapters_order` in metadata.json.
/// 4. Cleans references from timeline events' `relatedChapters` arrays.
#[tauri::command]
fn eliminar_capitulo(proyecto_path: String, filename: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if filename.trim().is_empty() {
        return Err("El nombre del archivo no puede estar vacío.".to_string());
    }

    let file_path = Path::new(&proyecto_path).join("capitulos").join(&filename);

    if !file_path.exists() {
        return Err(format!("El capítulo '{}' no existe.", filename));
    }

    // Delete the chapter file
    std::fs::remove_file(&file_path)
        .map_err(|e| format!("Error al eliminar el capítulo: {}", e))?;

    // Remove from metadata chapters_order
    let metadata_path = Path::new(&proyecto_path).join(".config").join("metadata.json");

    if !metadata_path.exists() {
        return Err("Archivo de metadatos no encontrado.".to_string());
    }

    let metadata_str = std::fs::read_to_string(&metadata_path)
        .map_err(|e| format!("Error al leer metadata.json: {}", e))?;

    let mut metadata: Metadata = serde_json::from_str(&metadata_str)
        .map_err(|e| format!("Error al parsear metadata.json: {}", e))?;

    metadata.chapters_order.retain(|ch| ch != &filename);
    metadata.last_modified = Local::now().to_rfc3339();

    let updated_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Error al serializar metadata.json: {}", e))?;

    std::fs::write(&metadata_path, updated_json)
        .map_err(|e| format!("Error al escribir metadata.json: {}", e))?;

    // Clean references from timeline
    let timeline_path = Path::new(&proyecto_path).join(".config").join("timeline.json");
    if timeline_path.exists() {
        let raw = std::fs::read_to_string(&timeline_path)
            .map_err(|e| format!("Error al leer la línea de tiempo: {}", e))?;
        let mut timeline: Vec<TimelineEvent> =
            serde_json::from_str(&raw).unwrap_or_default();
        for event in &mut timeline {
            event.relatedChapters.retain(|ch| ch != &filename);
        }
        let timeline_json = serde_json::to_string_pretty(&timeline)
            .map_err(|e| format!("Error al serializar la línea de tiempo: {}", e))?;
        std::fs::write(&timeline_path, timeline_json)
            .map_err(|e| format!("Error al escribir la línea de tiempo: {}", e))?;
    }

    Ok(format!("Capítulo '{}' eliminado.", filename))
}

// ---------------------------------------------------------------------------
// Characters — personajes
// ---------------------------------------------------------------------------

/// List all characters in a project.
///
/// Reads `personajes/index.json`. Returns JSON array string.
/// If file is missing, returns "[]".
#[tauri::command]
fn listar_personajes(proyecto_path: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }

    let index_path = Path::new(&proyecto_path).join("personajes").join("index.json");

    if !index_path.exists() {
        return Ok("[]".to_string());
    }

    std::fs::read_to_string(&index_path)
        .map_err(|e| format!("No se pudo leer el índice de personajes: {}", e))
}

/// Create a new character.
///
/// Parses the input JSON to extract `id` and `name`. Rejects duplicates.
/// Creates `personajes/{id}.json` and updates `personajes/index.json`.
#[tauri::command]
fn crear_personaje(proyecto_path: String, personaje_json: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }

    let character: Character = serde_json::from_str(&personaje_json)
        .map_err(|e| format!("Error al parsear el personaje: {}", e))?;

    if character.id.trim().is_empty() {
        return Err("El ID del personaje no puede estar vacío.".to_string());
    }
    if character.name.trim().is_empty() {
        return Err("El nombre del personaje no puede estar vacío.".to_string());
    }

    let personajes_dir = Path::new(&proyecto_path).join("personajes");
    let char_file = personajes_dir.join(format!("{}.json", character.id));

    // Reject duplicates
    if char_file.exists() {
        return Err(format!("El personaje '{}' ya existe.", character.id));
    }

    // Ensure directory exists
    std::fs::create_dir_all(&personajes_dir)
        .map_err(|e| format!("No se pudo crear el directorio personajes: {}", e))?;

    // Write character file
    let char_json = serde_json::to_string_pretty(&character)
        .map_err(|e| format!("Error al serializar el personaje: {}", e))?;
    std::fs::write(&char_file, char_json)
        .map_err(|e| format!("Error al crear el personaje: {}", e))?;

    // Update index
    let index_path = personajes_dir.join("index.json");
    let mut index: Vec<CharacterIndexItem> = if index_path.exists() {
        let raw = std::fs::read_to_string(&index_path)
            .map_err(|e| format!("Error al leer el índice de personajes: {}", e))?;
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        vec![]
    };

    index.push(CharacterIndexItem {
        id: character.id.clone(),
        name: character.name.clone(),
    });

    let index_json = serde_json::to_string_pretty(&index)
        .map_err(|e| format!("Error al serializar el índice de personajes: {}", e))?;
    std::fs::write(&index_path, index_json)
        .map_err(|e| format!("Error al escribir el índice de personajes: {}", e))?;

    Ok(format!("Personaje '{}' creado.", character.name))
}

/// Load a character by ID.
///
/// Reads `personajes/{id}.json` and returns the full JSON string.
#[tauri::command]
fn cargar_personaje(proyecto_path: String, id: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if id.trim().is_empty() {
        return Err("El ID del personaje no puede estar vacío.".to_string());
    }

    let char_path = Path::new(&proyecto_path)
        .join("personajes")
        .join(format!("{}.json", id));

    if !char_path.exists() {
        return Err(format!("Personaje '{}' no encontrado.", id));
    }

    std::fs::read_to_string(&char_path)
        .map_err(|e| format!("Error al leer el personaje: {}", e))
}

/// Update a character.
///
/// Overwrites `personajes/{id}.json`. If the name changed, updates the index entry.
#[tauri::command]
fn actualizar_personaje(
    proyecto_path: String,
    id: String,
    personaje_json: String,
) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if id.trim().is_empty() {
        return Err("El ID del personaje no puede estar vacío.".to_string());
    }

    let personajes_dir = Path::new(&proyecto_path).join("personajes");
    let char_path = personajes_dir.join(format!("{}.json", id));

    if !char_path.exists() {
        return Err(format!("Personaje '{}' no encontrado.", id));
    }

    // Read old character to detect name change
    let old_raw = std::fs::read_to_string(&char_path)
        .map_err(|e| format!("Error al leer el personaje existente: {}", e))?;
    let old_char: Character = serde_json::from_str(&old_raw)
        .map_err(|e| format!("Error al parsear el personaje existente: {}", e))?;

    let character: Character = serde_json::from_str(&personaje_json)
        .map_err(|e| format!("Error al parsear el personaje actualizado: {}", e))?;

    // Overwrite file
    let char_json = serde_json::to_string_pretty(&character)
        .map_err(|e| format!("Error al serializar el personaje: {}", e))?;
    std::fs::write(&char_path, char_json)
        .map_err(|e| format!("Error al guardar el personaje: {}", e))?;

    // Update index if name changed
    if old_char.name != character.name {
        let index_path = personajes_dir.join("index.json");
        if index_path.exists() {
            let raw = std::fs::read_to_string(&index_path)
                .map_err(|e| format!("Error al leer el índice de personajes: {}", e))?;
            let mut index: Vec<CharacterIndexItem> =
                serde_json::from_str(&raw).unwrap_or_default();
            for item in &mut index {
                if item.id == id {
                    item.name = character.name.clone();
                    break;
                }
            }
            let index_json = serde_json::to_string_pretty(&index)
                .map_err(|e| format!("Error al serializar el índice de personajes: {}", e))?;
            std::fs::write(&index_path, index_json)
                .map_err(|e| format!("Error al escribir el índice de personajes: {}", e))?;
        }
    }

    Ok(format!("Personaje '{}' actualizado.", character.name))
}

/// Delete a character.
///
/// Deletes `personajes/{id}.json`, removes from `personajes/index.json`,
/// and removes references from timeline events' `relatedCharacters` arrays.
#[tauri::command]
fn eliminar_personaje(proyecto_path: String, id: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if id.trim().is_empty() {
        return Err("El ID del personaje no puede estar vacío.".to_string());
    }

    let personajes_dir = Path::new(&proyecto_path).join("personajes");
    let char_path = personajes_dir.join(format!("{}.json", id));

    if !char_path.exists() {
        return Err(format!("Personaje '{}' no encontrado.", id));
    }

    // Delete the file
    std::fs::remove_file(&char_path)
        .map_err(|e| format!("Error al eliminar el personaje: {}", e))?;

    // Remove from index
    let index_path = personajes_dir.join("index.json");
    if index_path.exists() {
        let raw = std::fs::read_to_string(&index_path)
            .map_err(|e| format!("Error al leer el índice de personajes: {}", e))?;
        let mut index: Vec<CharacterIndexItem> =
            serde_json::from_str(&raw).unwrap_or_default();
        index.retain(|item| item.id != id);
        let index_json = serde_json::to_string_pretty(&index)
            .map_err(|e| format!("Error al serializar el índice de personajes: {}", e))?;
        std::fs::write(&index_path, index_json)
            .map_err(|e| format!("Error al escribir el índice de personajes: {}", e))?;
    }

    // Remove references from timeline
    let timeline_path = Path::new(&proyecto_path).join(".config").join("timeline.json");
    if timeline_path.exists() {
        let raw = std::fs::read_to_string(&timeline_path)
            .map_err(|e| format!("Error al leer la línea de tiempo: {}", e))?;
        let mut timeline: Vec<TimelineEvent> =
            serde_json::from_str(&raw).unwrap_or_default();
        for event in &mut timeline {
            event.relatedCharacters.retain(|cid| cid != &id);
        }
        let timeline_json = serde_json::to_string_pretty(&timeline)
            .map_err(|e| format!("Error al serializar la línea de tiempo: {}", e))?;
        std::fs::write(&timeline_path, timeline_json)
            .map_err(|e| format!("Error al escribir la línea de tiempo: {}", e))?;
    }

    Ok(format!("Personaje '{}' eliminado.", id))
}

// ---------------------------------------------------------------------------
// Notes — notas
// ---------------------------------------------------------------------------

/// List all notes in a project.
///
/// Reads `notas/index.json`. Returns JSON array string.
/// If file is missing, returns "[]".
#[tauri::command]
fn listar_notas(proyecto_path: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }

    let index_path = Path::new(&proyecto_path).join("notas").join("index.json");

    if !index_path.exists() {
        return Ok("[]".to_string());
    }

    std::fs::read_to_string(&index_path)
        .map_err(|e| format!("No se pudo leer el índice de notas: {}", e))
}

/// Create or update a note (upsert — follows guardar_capitulo pattern).
///
/// Creates or overwrites `notas/{id}.md` with the given content.
/// Updates `notas/index.json` (adds or updates title).
#[tauri::command]
fn crear_nota(
    proyecto_path: String,
    id: String,
    titulo: String,
    contenido: String,
) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if id.trim().is_empty() {
        return Err("El ID de la nota no puede estar vacío.".to_string());
    }

    let notas_dir = Path::new(&proyecto_path).join("notas");
    let note_file = notas_dir.join(format!("{}.md", id));

    let existed = note_file.exists();

    // Ensure directory exists
    std::fs::create_dir_all(&notas_dir)
        .map_err(|e| format!("No se pudo crear el directorio notas: {}", e))?;

    // Write / overwrite note file
    std::fs::write(&note_file, &contenido)
        .map_err(|e| format!("Error al guardar la nota: {}", e))?;

    // Update index
    let index_path = notas_dir.join("index.json");
    let mut index: Vec<NoteIndexItem> = if index_path.exists() {
        let raw = std::fs::read_to_string(&index_path)
            .map_err(|e| format!("Error al leer el índice de notas: {}", e))?;
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        vec![]
    };

    if existed {
        // Update existing entry
        for item in &mut index {
            if item.id == id {
                item.title = titulo.clone();
                break;
            }
        }
    } else {
        index.push(NoteIndexItem {
            id: id.clone(),
            title: titulo.clone(),
        });
    }

    let index_json = serde_json::to_string_pretty(&index)
        .map_err(|e| format!("Error al serializar el índice de notas: {}", e))?;
    std::fs::write(&index_path, index_json)
        .map_err(|e| format!("Error al escribir el índice de notas: {}", e))?;

    let action = if existed { "actualizada" } else { "creada" };
    Ok(format!("Nota '{}' {}.", titulo, action))
}

/// Load a note by ID.
///
/// Reads `notas/{id}.md` and returns its markdown content.
#[tauri::command]
fn cargar_nota(proyecto_path: String, id: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if id.trim().is_empty() {
        return Err("El ID de la nota no puede estar vacío.".to_string());
    }

    let note_path = Path::new(&proyecto_path)
        .join("notas")
        .join(format!("{}.md", id));

    if !note_path.exists() {
        return Err(format!("Nota '{}' no encontrada.", id));
    }

    std::fs::read_to_string(&note_path)
        .map_err(|e| format!("Error al leer la nota: {}", e))
}

/// Delete a note.
///
/// Deletes `notas/{id}.md` and removes the entry from `notas/index.json`.
#[tauri::command]
fn eliminar_nota(proyecto_path: String, id: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if id.trim().is_empty() {
        return Err("El ID de la nota no puede estar vacío.".to_string());
    }

    let notas_dir = Path::new(&proyecto_path).join("notas");
    let note_path = notas_dir.join(format!("{}.md", id));

    if !note_path.exists() {
        return Err(format!("Nota '{}' no encontrada.", id));
    }

    // Delete the file
    std::fs::remove_file(&note_path)
        .map_err(|e| format!("Error al eliminar la nota: {}", e))?;

    // Remove from index
    let index_path = notas_dir.join("index.json");
    if index_path.exists() {
        let raw = std::fs::read_to_string(&index_path)
            .map_err(|e| format!("Error al leer el índice de notas: {}", e))?;
        let mut index: Vec<NoteIndexItem> = serde_json::from_str(&raw).unwrap_or_default();
        index.retain(|item| item.id != id);
        let index_json = serde_json::to_string_pretty(&index)
            .map_err(|e| format!("Error al serializar el índice de notas: {}", e))?;
        std::fs::write(&index_path, index_json)
            .map_err(|e| format!("Error al escribir el índice de notas: {}", e))?;
    }

    Ok(format!("Nota '{}' eliminada.", id))
}

// ---------------------------------------------------------------------------
// Timeline — .config/timeline.json
// ---------------------------------------------------------------------------

/// Read the timeline.
///
/// Reads `.config/timeline.json` and returns the JSON array.
#[tauri::command]
fn cargar_timeline(proyecto_path: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }

    let timeline_path = Path::new(&proyecto_path).join(".config").join("timeline.json");

    if !timeline_path.exists() {
        return Ok("[]".to_string());
    }

    std::fs::read_to_string(&timeline_path)
        .map_err(|e| format!("Error al leer la línea de tiempo: {}", e))
}

/// Add an event to the timeline.
///
/// Parses the event JSON. Generates an `id` if not provided.
/// Appends to the timeline array in `.config/timeline.json`.
#[tauri::command]
fn agregar_evento_timeline(proyecto_path: String, evento_json: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }

    let mut event: TimelineEvent = serde_json::from_str(&evento_json)
        .map_err(|e| format!("Error al parsear el evento: {}", e))?;

    // Generate ID if missing
    if event.id.trim().is_empty() {
        event.id = format!("evt-{}", Local::now().timestamp_millis());
    }

    if event.date.trim().is_empty() {
        return Err("La fecha del evento no puede estar vacía.".to_string());
    }
    if event.title.trim().is_empty() {
        return Err("El título del evento no puede estar vacío.".to_string());
    }

    let timeline_path = Path::new(&proyecto_path).join(".config").join("timeline.json");

    let mut timeline: Vec<TimelineEvent> = if timeline_path.exists() {
        let raw = std::fs::read_to_string(&timeline_path)
            .map_err(|e| format!("Error al leer la línea de tiempo: {}", e))?;
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        vec![]
    };

    // Reject duplicate IDs
    if timeline.iter().any(|e| e.id == event.id) {
        return Err(format!("Ya existe un evento con el ID '{}'.", event.id));
    }

    let event_id = event.id.clone();
    timeline.push(event);

    let timeline_json = serde_json::to_string_pretty(&timeline)
        .map_err(|e| format!("Error al serializar la línea de tiempo: {}", e))?;
    std::fs::write(&timeline_path, timeline_json)
        .map_err(|e| format!("Error al escribir la línea de tiempo: {}", e))?;

    Ok(format!("Evento '{}' agregado a la línea de tiempo.", event_id))
}

/// Reorder timeline events to match the given ID order.
///
/// `ids_json` is a JSON array of event IDs in the desired order.
/// Events with IDs not in the input are appended at the end.
/// IDs in the input that don't exist in the timeline are silently skipped.
#[tauri::command]
fn reordenar_timeline(proyecto_path: String, ids_json: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }

    let desired: Vec<String> = serde_json::from_str(&ids_json)
        .map_err(|e| format!("Error al parsear la lista de IDs: {}", e))?;

    let timeline_path = Path::new(&proyecto_path).join(".config").join("timeline.json");
    if !timeline_path.exists() {
        return Err("El archivo de línea de tiempo no existe.".to_string());
    }

    let raw = std::fs::read_to_string(&timeline_path)
        .map_err(|e| format!("Error al leer la línea de tiempo: {}", e))?;
    let mut timeline: Vec<TimelineEvent> = serde_json::from_str(&raw).unwrap_or_default();

    // Build a lookup: id -> event (take ownership, remove from vec)
    let mut event_map: std::collections::HashMap<String, TimelineEvent> = timeline
        .drain(..)
        .map(|e| (e.id.clone(), e))
        .collect();

    let mut reordered: Vec<TimelineEvent> = Vec::with_capacity(event_map.len());

    // Place events in the desired order
    for id in &desired {
        if let Some(event) = event_map.remove(id) {
            reordered.push(event);
        }
    }

    // Append any remaining events (IDs not in the input)
    for (_, event) in event_map {
        reordered.push(event);
    }

    let timeline_json = serde_json::to_string_pretty(&reordered)
        .map_err(|e| format!("Error al serializar la línea de tiempo: {}", e))?;
    std::fs::write(&timeline_path, timeline_json)
        .map_err(|e| format!("Error al escribir la línea de tiempo: {}", e))?;

    Ok("Línea de tiempo reordenada correctamente.".to_string())
}

/// Remove an event from the timeline.
///
/// Deletes the event with the matching `id` from `.config/timeline.json`.
#[tauri::command]
fn eliminar_evento_timeline(proyecto_path: String, id: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if id.trim().is_empty() {
        return Err("El ID del evento no puede estar vacío.".to_string());
    }

    let timeline_path = Path::new(&proyecto_path).join(".config").join("timeline.json");

    if !timeline_path.exists() {
        return Err("El archivo de línea de tiempo no existe.".to_string());
    }

    let raw = std::fs::read_to_string(&timeline_path)
        .map_err(|e| format!("Error al leer la línea de tiempo: {}", e))?;
    let mut timeline: Vec<TimelineEvent> = serde_json::from_str(&raw).unwrap_or_default();

    let len_before = timeline.len();
    timeline.retain(|e| e.id != id);

    if timeline.len() == len_before {
        return Err(format!(
            "Evento '{}' no encontrado en la línea de tiempo.",
            id
        ));
    }

    let timeline_json = serde_json::to_string_pretty(&timeline)
        .map_err(|e| format!("Error al serializar la línea de tiempo: {}", e))?;
    std::fs::write(&timeline_path, timeline_json)
        .map_err(|e| format!("Error al escribir la línea de tiempo: {}", e))?;

    Ok(format!("Evento '{}' eliminado de la línea de tiempo.", id))
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
            if path.extension().is_some_and(|ext| ext == "md") {
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

/// Internal checkpoint — same logic as the Tauri command but callable
/// from event handlers without going through the IPC layer.
fn do_checkpoint(project_path: &str) -> Result<String, String> {
    let project_path = Path::new(project_path);
    let git_path = find_git()?;

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

    let word_count = count_words_in_chapters(project_path);
    let date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let commit_msg = format!(
        "Progreso automático: {} - {} palabras",
        date, word_count
    );

    let commit_output = Command::new(&git_path)
        .arg("commit")
        .arg("-m")
        .arg(&commit_msg)
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git commit: {}", e))?;

    if commit_output.status.success() {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(ProjectState {
            active_project: Mutex::new(None),
            closing: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            crear_proyecto,
            marcar_proyecto_cronista,
            inicializar_git,
            inicializar_git_con_autor,
            verificar_git_inicializado,
            obtener_git_log,
            detectar_git,
            set_active_project,
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
            eliminar_personaje,
            listar_notas,
            crear_nota,
            cargar_nota,
            eliminar_nota,
            cargar_timeline,
            agregar_evento_timeline,
            reordenar_timeline,
            eliminar_evento_timeline,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<ProjectState>();

                // Guard against re-entry
                {
                    let mut closing = state.closing.lock().unwrap();
                    if *closing {
                        return; // Already in close flow, let it through
                    }
                    *closing = true;
                }

                let project_path = {
                    let active = state.active_project.lock().unwrap();
                    active.clone()
                };

                if let Some(ref path) = project_path {
                    // Prevent immediate close so we can checkpoint
                    api.prevent_close();

                    let path = path.clone();
                    let window_clone = window.clone();

                    // Spawn async task — checkpoint + close happens off the event loop
                    tauri::async_runtime::spawn(async move {
                        // Brief pause lets any in-flight autosave IPC complete
                        tokio::time::sleep(std::time::Duration::from_millis(600)).await;

                        // Checkpoint (git commit). Ignore errors — we close anyway.
                        let _ = do_checkpoint(&path);

                        // Force-close. This re-enters on_window_event but
                        // the `closing` guard lets it through immediately.
                        let _ = window_clone.destroy();
                    });
                }
                // If no active project, don't prevent close — window closes normally
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

        let result = crear_proyecto(path_with_slash, "Trailing Test".to_string());
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
        let _ = crear_proyecto(path.clone(), "Test Project".to_string());

        let contenido = "# Capítulo 1\n\n";
        let result = crear_capitulo(
            path.clone(),
            "0002_capitulo_1.md".to_string(),
            contenido.to_string(),
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

        let _ = crear_proyecto(path.clone(), "Test Project".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test Project".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

        let result = cargar_timeline(path);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(result.unwrap(), "[]");
    }

    // --- agregar_evento_timeline (2 tests) ---

    #[test]
    fn test_agregar_evento_timeline_creates_event() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

    // --- reordenar_timeline (1 test) ---

    #[test]
    fn test_reordenar_timeline_reorders_events() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().to_str().unwrap().to_string();

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

        // Create a chapter
        let _ = crear_capitulo(
            path.clone(),
            "0001_prologo.md".to_string(),
            "# Prólogo\n\n".to_string(),
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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

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

        let _ = crear_proyecto(path.clone(), "Test".to_string());

        // Create a chapter
        let _ = crear_capitulo(
            path.clone(),
            "cap1.md".to_string(),
            "# Cap 1\n\n".to_string(),
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
    // Boundary: close the tests module
    // ========================================================================

}
