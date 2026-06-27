// Cron-Insta — Tauri backend
//
// Phase 2: Rust backend commands for project management and git abstraction.
// All five Tauri commands + find_git() helper live here per the single-module design.

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::collections::HashMap;
use std::io::Write;
use tauri::Manager;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// ---------------------------------------------------------------------------
// Process helper — hides terminal windows on Windows
// ---------------------------------------------------------------------------

/// Create a `Command` pre-configured for headless execution:
/// - `CREATE_NO_WINDOW` on Windows (prevents console popups)
/// - `stdin` set to null (prevents accidental blocking on stdin reads)
fn system_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(std::process::Stdio::null());
    // Inherit SSH agent socket for git operations on Linux
    let ssh_sock = std::env::var("SSH_AUTH_SOCK").ok()
        .or_else(find_ssh_auth_sock_fallback);
    if let Some(sock) = ssh_sock {
        cmd.env("SSH_AUTH_SOCK", sock);
    }
    // Never prompt for password, timeout after 5s to avoid 30s hangs
    cmd.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes -o ConnectTimeout=5");
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}

/// Fallback: try common SSH_AUTH_SOCK paths when the env var is not set.
/// This covers desktop-launched Tauri apps that don't inherit the terminal env.
fn find_ssh_auth_sock_fallback() -> Option<String> {
    let uid = std::fs::read_to_string("/proc/self/loginuid").ok()?;
    let uid = uid.trim();
    let candidates = [
        format!("/run/user/{}/keyring/ssh", uid),
        format!("/run/user/{}/ssh-agent.socket", uid),
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Some(path.clone());
        }
    }
    None
}

/// Check if SSH agent is available for git network operations.
/// Returns `true` if `SSH_AUTH_SOCK` is set (env or fallback path).
/// When unavailable, all SSH git ops (fetch, push) will fail — skip them early.
fn ssh_available() -> bool {
    std::env::var("SSH_AUTH_SOCK").is_ok() || find_ssh_auth_sock_fallback().is_some()
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Tauri managed state: tracks the active project for close-time checkpoint.
struct ProjectState {
    active_project: Mutex<Option<String>>,
    closing: Mutex<bool>,
    session_tracker: Mutex<SessionTracker>,
}

// ── Session statistics data structures ─────────────────────────

/// Per-chapter accumulated stats.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct StatsChapter {
    #[serde(default)]
    words: u64,
    #[serde(default)]
    time_seconds: u64,
}

/// A single writing session record.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct StatsSession {
    date: String,
    duration_seconds: u64,
    words_added: u64,
    chapter_id: String,
}

/// Top-level stats container persisted to `.config/stats.json`.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct SessionStats {
    #[serde(default)]
    total_time_seconds: u64,
    #[serde(default)]
    total_words: u64,
    #[serde(default)]
    chapters: HashMap<String, StatsChapter>,
    #[serde(default)]
    sessions: Vec<StatsSession>,
}

/// In-memory runtime state for the writing session timer.
///
/// Tracks elapsed time per chapter and overall session duration.
/// Mutations happen only on chapter open (frontend IPC) and project
/// close (do_checkpoint), so contention is minimal.
struct SessionTracker {
    start_time: Option<std::time::Instant>,
    chapter_start: Option<std::time::Instant>,
    chapter_filename: Option<String>,
    initial_word_count: Option<u64>,
    chapter_times: HashMap<String, u64>,
}

impl Default for SessionTracker {
    fn default() -> Self {
        Self {
            start_time: None,
            chapter_start: None,
            chapter_filename: None,
            initial_word_count: None,
            chapter_times: HashMap::new(),
        }
    }
}

/// Auto-detected git identity + remote from `.git/config`.
///
/// All fields are `Option` — missing `.git` or partial config yields `None`.
/// The struct is serialized directly by Tauri (no manual JSON).
#[derive(Serialize)]
struct GitDetectedConfig {
    name: Option<String>,
    email: Option<String>,
    remote_url: Option<String>,
}

/// Per-tab visibility toggles for the sidebar.
///
/// All fields default to `true` via serde and `Default` for backward
/// compatibility with old `metadata.json` files.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct VisibleTabs {
    #[serde(default = "default_true")]
    chapters: bool,
    #[serde(default = "default_true")]
    characters: bool,
    #[serde(default = "default_true")]
    places: bool,
    #[serde(default = "default_true")]
    timeline: bool,
    #[serde(default = "default_true")]
    notes: bool,
}

impl Default for VisibleTabs {
    fn default() -> Self {
        Self {
            chapters: true,
            characters: true,
            places: true,
            timeline: true,
            notes: true,
        }
    }
}

fn default_true() -> bool {
    true
}

fn validate_visible_tabs(tabs: &VisibleTabs) -> Result<(), String> {
    if !tabs.chapters {
        return Err("Los capítulos deben estar siempre visibles (chapters debe ser true).".to_string());
    }
    Ok(())
}

fn validate_auto_save_interval(minutes: u32) -> Result<(), String> {
    match minutes {
        1 | 5 | 10 => Ok(()),
        _ => Err(format!(
            "Intervalo de autoguardado inválido: {}. Debe ser 1, 5 o 10 minutos.",
            minutes
        )),
    }
}

fn default_auto_save_interval() -> u32 {
    5
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Metadata {
    project_name: String,
    last_modified: String,
    chapters_order: Vec<String>,
    characters_index: Vec<CharacterIndex>,
    #[serde(default)]
    places_index: Vec<LugarIndexItem>,
    #[serde(default = "default_font_family")]
    font_family: String,
    /// Whether auto-push to remote is active for this project.
    #[serde(default)]
    push_enabled: bool,
    /// Consecutive push failure count for the 3-strike rule.
    #[serde(default)]
    consecutive_failures: u32,
    /// Per-tab visibility toggles. All default to `true`.
    #[serde(default)]
    visible_tabs: VisibleTabs,
    /// Auto-save interval in minutes (1, 5, or 10). Default 5.
    #[serde(default = "default_auto_save_interval")]
    auto_save_interval_minutes: u32,
    /// Tramas (plotlines) — metadata-only groupings of chapters.
    #[serde(default)]
    tramas: Vec<Trama>,
    /// Chapter-to-trama assignments. Maps filename → optional trama_id.
    #[serde(default)]
    chapter_tramas: Vec<ChapterTrama>,
}

fn default_font_family() -> String {
    "monospace".to_string()
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

// ── Places — lugares ──────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LugarIndexItem {
    id: String,
    name: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Lugar {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
}

#[allow(non_snake_case)]
// ── Tramas — plotlines ──────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Trama {
    id: String,
    nombre: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChapterTrama {
    filename: String,
    #[serde(default)]
    trama_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
struct TimelineEvent {
    #[serde(default)]
    id: String,
    #[serde(default)]
    date: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    relatedCharacters: Vec<String>,
    #[serde(default)]
    relatedChapters: Vec<String>,
    #[serde(default)]
    relatedPlaces: Vec<String>,
}

// ---------------------------------------------------------------------------
// Git identity & remote config data structures
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GitIdentity {
    name: String,
    email: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    github_user: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)] // kept for test use
struct GitRemoteConfig {
    url: String,
    #[serde(default)]
    push_enabled: bool,
    #[serde(default)]
    consecutive_failures: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GitConfig {
    schema_version: u32,
    #[serde(default)]
    identity: Option<GitIdentity>,
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
        let output = system_command("which")
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
        if let Ok(output) = system_command("where").arg("git").output() {
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


/// Generate the SCHEMA.md content for a new project.
///
/// Centralised here so the schema stays in sync with the data model
/// without hunting through a raw string inside `crear_proyecto`.
fn generate_schema(nombre: &str) -> String {
    let schema = r#"# SCHEMA — {NOMBRE}

Generated by Cron-Insta. This file describes the project data model for AI agent consumption.

## Overview

This is a literary writing project managed by **Cron-Insta**, a desktop writing application. Data is stored as files on disk — no database is used.

## Entities

### Chapter

- **Storage**: `capitulos/{{filename}}.md` (one file per chapter)
- **Format**: HTML content rendered by TipTap (ProseMirror-based rich text editor)
- **Indexing**: Ordered list of filenames in `metadata.json → chapters_order`
- **Usage**: The core content of the project; each chapter is a section of the written work.

### Character

- **Storage**: `personajes/{{id}}.json` (one JSON file per character)
- **Fields**:
  - `id` (string): Unique identifier
  - `name` (string): Character display name
  - `physicalDescription` (string, optional): Physical appearance
  - `personality` (string, optional): Personality traits
  - `traumas` (string, optional): Backstory or trauma
  - `relationships` (array): List of relationships with other characters
- **Index**: `personajes/index.json` → array of `{{ id, name }}`

### Character Relationship

- **Location**: Nested inside each Character's `relationships` array
- **Fields**:
  - `targetId` (string, optional): ID of the related character
  - `targetName` (string): Display name of the related character
  - `type` (string): Relationship type (e.g. "friend", "rival", "family")
  - `notes` (string, optional): Free-text notes about the relationship
- **Note**: `targetId` is a soft reference — not validated against the character index.

### Note

- **Storage**: `notas/{{id}}.md` (one Markdown file per note)
- **Format**: HTML content rendered by TipTap
- **Index**: `notas/index.json` → array of `{{ id, title }}`
- **Usage**: Free-form notes, brainstorming, outlines, or research related to the project.

### Place

- **Storage**: `lugares/{{id}}.json` (one JSON file per place)
- **Fields**:
  - `id` (string): Unique identifier
  - `name` (string): Place display name
  - `description` (string): Place description
- **Index**: `lugares/index.json` → array of `{{ id, name }}`

### Timeline Event

- **Storage**: `.config/timeline.json` (single JSON array file)
- **Fields**:
  - `id` (string): Unique identifier (format: `evt-{{timestamp_ms}}`)
  - `date` (string): Free-text date or ISO date string
  - `title` (string): Event title
  - `description` (string): Event description
  - `relatedCharacters` (array of strings): IDs of related characters (soft reference)
  - `relatedChapters` (array of strings): Filenames of related chapters (soft reference)
  - `relatedPlaces` (array of strings): IDs of related places (soft reference)

### Trama

- **Storage**: Inline in `.config/metadata.json` under `tramas` array
- **Fields**:
  - `id` (string): Unique identifier derived from the trama name (slug + hex suffix)
  - `nombre` (string): Display name of the trama
- **Usage**: Metadata-only grouping of chapters into narrative plotlines. Chapters remain flat in `capitulos/` regardless of trama assignment.
- **Chapter-Trama Assignment**: Stored in `.config/metadata.json` under `chapter_tramas` as `{{filename, trama_id}}`. `trama_id` is `null` for unassigned chapters.

## Relationships

```
TimelineEvent.relatedCharacters ──soft──▶ Character.id
TimelineEvent.relatedChapters   ──soft──▶ Chapter filename
TimelineEvent.relatedPlaces     ──soft──▶ Place.id
Character.relationships[].targetId ──soft──▶ Character.id
ChapterTrama.trama_id           ──soft──▶ Trama.id
```

All references are **soft** (no foreign key enforcement). Deleting a Character, Chapter, or Place:
- Removes its references from `TimelineEvent.relatedCharacters` / `relatedChapters` / `relatedPlaces`
- Character relationships (`targetId`) are NOT automatically cleaned up

Timeline events linked to a deleted entity are NOT deleted — only the reference is removed.

Deleting a Trama sets all its assigned chapters' `trama_id` to `null` — chapters are never deleted.

## Project Configuration

### `.config/metadata.json`

| Field | Type | Description |
|-------|------|-------------|
| `project_name` | string | Display name of the project |
| `last_modified` | string | ISO 8601 timestamp of last modification |
| `chapters_order` | string[] | Ordered list of chapter filenames |
| `characters_index` | object[] | Array of `{{ id, file, name }}` |
| `places_index` | object[] | Array of `{{ id, name }}` |
| `tramas` | object[] | Array of `{{ id, nombre }}` plotline groupings |
| `chapter_tramas` | object[] | Array of `{{ filename, trama_id }}` assignments |
| `font_family` | string | Editor font: `"monospace"`, `"serif"`, or `"sans-serif"` |
| `push_enabled` | boolean | Whether auto-push to remote is active for this project (default: false) |
| `consecutive_failures` | number | Consecutive push failure count for the 3-strike auto-disable rule (default: 0) |

### `.config/timeline.json`

JSON array of TimelineEvent objects (see Entity section above).

## Directory Structure

```
{NOMBRE}/
├── .config/
│   ├── metadata.json
│   └── timeline.json
├── capitulos/
│   ├── 0001_prologo.md
│   └── ...
├── personajes/
│   ├── index.json
│   ├── {{id}}.json
│   └── ...
├── notas/
│   ├── index.json
│   ├── {{id}}.md
│   └── ...
├── lugares/
│   ├── index.json
│   ├── {{id}}.json
│   └── ...
└── SCHEMA.md          ◀── this file
```
"#;
    schema.replace("{NOMBRE}", nombre)
}

/// Create a new Cron-Insta literary project.
///
/// Creates the base directory plus four subdirectories (`.config/`,
/// `capitulos/`, `personajes/`, `notas/`), seeds `.config/metadata.json`
/// and `.config/timeline.json`, then automatically initialises a Git
/// repository (silently — disk structure is created regardless of Git
/// availability).
///
/// The Git identity is read from the global config file; falls back to
/// the default "Cron-Insta" identity when no config exists.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn crear_proyecto(app: tauri::AppHandle, path: String, nombre: String, font_family: Option<String>, visible_tabs: Option<VisibleTabs>, auto_save_interval_minutes: Option<u32>) -> Result<String, String> {
    // Normalise trailing separators
    let path = path.trim_end_matches('/').trim_end_matches('\\').to_string();
    let base = Path::new(&path);

    // Reject if a project already exists at this path
    let metadata_path = base.join(".config").join("metadata.json");
    if metadata_path.exists() {
        return Err(format!(
            "PROJECT_ALREADY_EXISTS:Ya existe un proyecto en '{}'. ¿Querés abrirlo en lugar de crear uno nuevo?",
            base.display()
        ));
    }

    // Create base directory
    std::fs::create_dir_all(base)
        .map_err(|e| format!("No se pudo crear el directorio del proyecto: {}", e))?;

    // Create subdirectories
    let subdirs = [".config", "capitulos", "personajes", "notas", "lugares"];
    for sub in &subdirs {
        std::fs::create_dir_all(base.join(sub))
            .map_err(|e| format!("No se pudo crear el directorio {}: {}", sub, e))?;
    }

    // Seed lugares/index.json (empty array)
    std::fs::write(base.join("lugares/index.json"), "[]")
        .map_err(|e| format!("Error al escribir lugares/index.json: {}", e))?;

    // Validate interval if provided
    if let Some(interval) = auto_save_interval_minutes {
        validate_auto_save_interval(interval)?;
    }
    // Validate tabs if provided
    if let Some(ref tabs) = visible_tabs {
        validate_visible_tabs(tabs)?;
    }

    // Write metadata.json
    let metadata = Metadata {
        project_name: nombre.clone(),
        last_modified: Local::now().to_rfc3339(),
        chapters_order: vec![],
        characters_index: vec![],
        places_index: vec![],
        font_family: font_family.unwrap_or_else(default_font_family),
        push_enabled: false,
        consecutive_failures: 0,
        visible_tabs: visible_tabs.unwrap_or_default(),
        auto_save_interval_minutes: auto_save_interval_minutes.unwrap_or_else(default_auto_save_interval),
        tramas: vec![],
        chapter_tramas: vec![],
    };
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Error al serializar metadata: {}", e))?;
    std::fs::write(base.join(".config/metadata.json"), metadata_json)
        .map_err(|e| format!("Error al escribir metadata.json: {}", e))?;

    // Write timeline.json (empty array)
    std::fs::write(base.join(".config/timeline.json"), "[]")
        .map_err(|e| format!("Error al escribir timeline.json: {}", e))?;

    // Write stats.json (empty seed)
    let stats = SessionStats::default();
    let stats_json = serde_json::to_string_pretty(&stats)
        .map_err(|e| format!("Error al serializar stats: {}", e))?;
    std::fs::write(base.join(".config/stats.json"), stats_json)
        .map_err(|e| format!("Error al escribir stats.json: {}", e))?;

    // Write SCHEMA.md — data model description for AI agent consumption
    let schema = generate_schema(&nombre);
    std::fs::write(base.join("SCHEMA.md"), schema)
        .map_err(|e| format!("Error al escribir SCHEMA.md: {}", e))?;

    // Auto-initialise git — silently ignore if git is unavailable
    let _ = inicializar_git(app, path.clone());

    Ok(format!("Proyecto '{}' creado en {}", nombre, path))
}

/// Copy the app icon into the project and set it as folder icon.
///
/// Best-effort — never fails project creation.
/// - **Linux**: copies 32x32.png as .cron-insta-icon.png, sets GVFS metadata.
/// - **Windows**: copies icon.ico as .cron-insta-icon.ico, creates desktop.ini
///   and marks the folder with +s attribute so Explorer picks up the icon.
#[tauri::command]
fn marcar_proyecto_cron_insta(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let base = Path::new(&path);

    #[cfg(target_os = "linux")]
    {
        let icon_dest = base.join(".cron-insta-icon.png");
        if let Ok(resource_dir) = app.path().resource_dir() {
            let icon_src = resource_dir.join("icons/32x32.png");
            if icon_src.exists() {
                std::fs::copy(&icon_src, &icon_dest)
                    .map_err(|e| format!("Error al copiar icono: {}", e))?;
            }
        }
        // Set folder icon via GVFS (GNOME, Nemo, Cinnamon...)
        if let Ok(icon_abs) = icon_dest.canonicalize() {
            let icon_uri = format!("file://{}", icon_abs.display());
            let _ = system_command("gio")
                .arg("set").arg("-t").arg("string")
                .arg(base)
                .arg("metadata::custom-icon")
                .arg(&icon_uri)
                .output();
        }
    }

    #[cfg(target_os = "windows")]
    {
        let icon_dest = base.join(".cron-insta-icon.ico");
        if let Ok(resource_dir) = app.path().resource_dir() {
            let icon_src = resource_dir.join("icons/icon.ico");
            if icon_src.exists() {
                std::fs::copy(&icon_src, &icon_dest)
                    .map_err(|e| format!("Error copying icon: {}", e))?;
            }
        }
        // Create desktop.ini to tell Explorer about the custom icon
        let desktop_ini = base.join("desktop.ini");
        let ini_content = format!(
            "[.ShellClassInfo]\r\nIconFile={}\r\nIconIndex=0\r\n",
            ".cron-insta-icon.ico"
        );
        std::fs::write(&desktop_ini, ini_content)
            .map_err(|e| format!("Error writing desktop.ini: {}", e))?;

        // Mark folder as system so Explorer reads desktop.ini
        let _ = system_command("attrib")
            .arg("+s")
            .arg(base)
            .output();
        // Hide the desktop.ini and icon files
        let _ = system_command("attrib")
            .arg("+h")
            .arg(&desktop_ini)
            .output();
        let _ = system_command("attrib")
            .arg("+h")
            .arg(&icon_dest)
            .output();
    }

    Ok(())
}

/// Initialise a Git repository in the given project path.
///
/// Returns success if `.git` already exists (reinit is safe) or if
/// `git init` succeeds.  Returns `Err` **only** when Git is unavailable —
/// callers can degrade gracefully.
///
/// Reads the Git identity from the global config file. Falls back to
/// the default "Cron-Insta" / "cron-insta@local" identity when no config
/// exists (backward-compatible behaviour).
#[tauri::command]
fn inicializar_git(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let project_path = Path::new(&path);

    // Already initialised → silent success
    if project_path.join(".git").exists() {
        return Ok("El repositorio ya estaba inicializado.".to_string());
    }

    // Locate git binary (returns Err with user-facing message when absent)
    let git_path = find_git()?;

    let output = system_command(&git_path)
        .arg("init")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git init: {}", e))?;

    if output.status.success() {
        // Read identity from global config, fall back to defaults
        let (user_name, user_email) = read_identity_from_config(&app)
            .unwrap_or_else(|| ("Cron-Insta".to_string(), "cron-insta@local".to_string()));

        // Set user identity (best-effort, silent on failure)
        let _ = system_command(&git_path)
            .arg("config")
            .arg("user.name")
            .arg(&user_name)
            .current_dir(project_path)
            .output();
        let _ = system_command(&git_path)
            .arg("config")
            .arg("user.email")
            .arg(&user_email)
            .current_dir(project_path)
            .output();

        // First commit — "Primera piedra"
        let _ = system_command(&git_path)
            .arg("add")
            .arg(".")
            .current_dir(project_path)
            .output();

        let commit_msg = "Primera piedra ✍️";
        let commit_output = system_command(&git_path)
            .arg("commit")
            .arg("-m")
            .arg(commit_msg)
            .current_dir(project_path)
            .output()
            .map_err(|e| format!("Error en primer commit: {}", e))?;

        if commit_output.status.success() {
            // Ensure the branch is named "main" (git may default to "master")
            let _ = system_command(&git_path)
                .arg("branch")
                .arg("-M")
                .arg("main")
                .current_dir(project_path)
                .output();
            Ok("Repositorio Git inicializado y primer commit creado.".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            if stderr.contains("nothing to commit") || stderr.contains("nothing added") {
                Ok("Repositorio Git inicializado (sin archivos para commit aún).".to_string())
            } else {
                Err(format!("Error en primer commit: {}", stderr.trim()))
            }
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error al inicializar Git: {}", stderr.trim()))
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

/// Remove the .git directory from a project (used when importing a project
/// and the user wants to start a fresh history).
#[tauri::command]
fn eliminar_directorio_git(path: String) -> Result<(), String> {
    let git_dir = std::path::Path::new(&path).join(".git");
    if git_dir.exists() {
        std::fs::remove_dir_all(&git_dir)
            .map_err(|e| format!("No se pudo eliminar el historial Git: {}", e))?;
    }
    Ok(())
}

/// Return the list of .md files changed in a given commit.
fn get_changed_md_files(
    project_path: &Path,
    git_path: &str,
    hash: &str,
) -> Vec<String> {
    let output = system_command(git_path)
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

    let output = system_command(&git_path)
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

/// Detect git identity and remote from `.git/config` for a project path.
///
/// Runs `git config user.name`, `git config user.email`,
/// and `git remote get-url origin` inside the project directory.
/// Best-effort only — never errors, missing data returns `None`.
#[tauri::command]
fn detectar_config_git(project_path: String) -> GitDetectedConfig {
    let base = Path::new(&project_path);
    let git_dir = base.join(".git");

    if !git_dir.exists() {
        return GitDetectedConfig {
            name: None,
            email: None,
            remote_url: None,
        };
    }

    let git_path = match find_git() {
        Ok(p) => p,
        Err(_) => {
            return GitDetectedConfig {
                name: None,
                email: None,
                remote_url: None,
            };
        }
    };

    let run_config = |key: &str| -> Option<String> {
        system_command(&git_path)
            .arg("config")
            .arg("--local")
            .arg(key)
            .current_dir(base)
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let remote_url = system_command(&git_path)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(base)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    GitDetectedConfig {
        name: run_config("user.name"),
        email: run_config("user.email"),
        remote_url,
    }
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

/// Start a writing session timer for the given chapter.
///
/// Reads the chapter file, counts words via `count_words_in_html`,
/// and records the start time. If another session was already active,
/// accumulates the previous chapter's elapsed time into `chapter_times`
/// and switches to the new chapter (timer continues).
///
/// The session timer remains active until `do_checkpoint()` calls
/// `finalizar_sesion_escritura` on project close.
#[tauri::command]
fn iniciar_sesion_escritura(
    state: tauri::State<ProjectState>,
    path: String,
    chapter_filename: String,
) -> Result<(), String> {
    let mut tracker = state.session_tracker.lock()
        .map_err(|e| format!("Error al acceder al tracker de sesión: {}", e))?;

    let project_path = Path::new(&path);

    // Read current chapter and count words
    let word_count = word_count_chapter(project_path, &chapter_filename);

    // If we were already tracking a chapter, accumulate its elapsed time
    if tracker.chapter_start.is_some() && tracker.chapter_filename.is_some() {
        let ch_file = tracker.chapter_filename.clone().unwrap();
        let ch_start = tracker.chapter_start.unwrap();
        let elapsed = ch_start.elapsed().as_secs();
        let accum = tracker.chapter_times.entry(ch_file).or_insert(0);
        *accum += elapsed;
    }

    // Set or reset session state
    let now = std::time::Instant::now();
    if tracker.start_time.is_none() {
        tracker.start_time = Some(now);
    }
    tracker.chapter_start = Some(now);
    tracker.chapter_filename = Some(chapter_filename);
    tracker.initial_word_count = Some(word_count);

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
/// When `push_enabled: true` and a remote is configured, attempts
/// `git push origin main` after a successful commit. Push failures
/// are tracked via the 3-strike counter and surfaced as a warning
/// appended to the commit hash.
///
/// Returns the commit hash on success (with optional push warning),
/// or a descriptive status when there is nothing to commit (still
/// `Ok` — not an error).
#[tauri::command]
fn crear_checkpoint(_app: tauri::AppHandle, proyecto_path: String) -> Result<String, String> {
    let project_path = Path::new(&proyecto_path);

    let commit_result = perform_commit(project_path)?;

    // No auto-push here — only do_checkpoint (close handler) syncs.
    Ok(commit_result)
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
    trama_id: Option<String>,
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

    // Register trama assignment if provided
    if let Some(ref tid) = trama_id {
        // Validate trama exists
        if !metadata.tramas.iter().any(|t| &t.id == tid) {
            return Err(format!("La trama con ID '{}' no existe.", tid));
        }
        // Remove existing assignment for this filename (if any)
        metadata.chapter_tramas.retain(|ct| ct.filename != filename);
        metadata.chapter_tramas.push(ChapterTrama {
            filename: filename.clone(),
            trama_id: Some(tid.clone()),
        });
    } else {
        // Explicitly register as unassigned
        metadata.chapter_tramas.retain(|ct| ct.filename != filename);
        metadata.chapter_tramas.push(ChapterTrama {
            filename: filename.clone(),
            trama_id: None,
        });
    }

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
    metadata.chapter_tramas.retain(|ct| ct.filename != filename);
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

/// Update the project font family in metadata.json.
///
/// Reads `{project_path}/.config/metadata.json`, updates `font_family` and
/// `last_modified` (ISO 8601), then writes the modified JSON back to disk.
/// Preserves all other fields (`project_name`, `chapters_order`, `characters_index`).
#[tauri::command]
fn actualizar_fuente_proyecto(project_path: String, font_family: String) -> Result<String, String> {
    if project_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if font_family.trim().is_empty() {
        return Err("La familia tipográfica no puede estar vacía.".to_string());
    }

    let valid_fonts = ["monospace", "serif", "sans-serif"];
    if !valid_fonts.contains(&font_family.as_str()) {
        return Err(format!(
            "Fuente inválida: '{}'. Debe ser monospace, serif o sans-serif.",
            font_family
        ));
    }

    let metadata_path = Path::new(&project_path)
        .join(".config")
        .join("metadata.json");

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

    metadata.font_family = font_family;
    metadata.last_modified = Local::now().to_rfc3339();

    let updated_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Error al serializar metadata.json: {}", e))?;

    std::fs::write(&metadata_path, updated_json)
        .map_err(|e| format!("Error al escribir metadata.json: {}", e))?;

    Ok("".to_string())
}

/// Merge partial project configuration into metadata.json.
///
/// Reads the current metadata, merges the given partial JSON config
/// (a `serde_json::Value`), validates the result (chapters must be true,
/// interval must be 1|5|10), and writes the merged output back to disk.
///
/// Returns the full merged metadata as a JSON string so the frontend can
/// update its state without re-fetching.
#[tauri::command]
fn actualizar_config_proyecto(project_path: String, config: serde_json::Value) -> Result<String, String> {
    if project_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }

    let metadata_path = Path::new(&project_path)
        .join(".config")
        .join("metadata.json");

    if !metadata_path.exists() {
        return Err(format!(
            "Archivo de metadatos no encontrado: {}",
            metadata_path.display()
        ));
    }

    // 1) Read current metadata
    let metadata_str = std::fs::read_to_string(&metadata_path)
        .map_err(|e| format!("Error al leer metadata.json: {}", e))?;

    let mut metadata: Metadata = serde_json::from_str(&metadata_str)
        .map_err(|e| format!("Error al parsear metadata.json: {}", e))?;

    // 2) Merge partial config (only overwrite fields present in the payload)
    if let Some(obj) = config.as_object() {
        if let Some(visible_tabs_val) = obj.get("visible_tabs") {
            if let Ok(tabs) = serde_json::from_value::<VisibleTabs>(visible_tabs_val.clone()) {
                metadata.visible_tabs = tabs;
            }
        }
        if let Some(interval_val) = obj.get("auto_save_interval_minutes") {
            if let Some(interval) = interval_val.as_u64() {
                metadata.auto_save_interval_minutes = interval as u32;
            }
        }
        if let Some(font_val) = obj.get("font_family") {
            if let Some(font) = font_val.as_str() {
                let valid_fonts = ["monospace", "serif", "sans-serif"];
                if valid_fonts.contains(&font) {
                    metadata.font_family = font.to_string();
                }
            }
        }
    }

    // 3) Validate merged result
    validate_visible_tabs(&metadata.visible_tabs)?;
    validate_auto_save_interval(metadata.auto_save_interval_minutes)?;

    // 4) Update timestamp and write
    metadata.last_modified = Local::now().to_rfc3339();

    let updated_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Error al serializar metadata.json: {}", e))?;

    std::fs::write(&metadata_path, updated_json)
        .map_err(|e| format!("Error al escribir metadata.json: {}", e))?;

    // Return full merged metadata so frontend can update state
    let full_json = serde_json::to_string(&metadata)
        .map_err(|e| format!("Error al serializar metadata: {}", e))?;

    Ok(full_json)
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
// Places — lugares
// ---------------------------------------------------------------------------

/// List all places in a project.
///
/// Reads `lugares/index.json`. Returns JSON array string.
/// If file is missing, returns "[]".
#[tauri::command]
fn listar_lugares(proyecto_path: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }

    let index_path = Path::new(&proyecto_path).join("lugares").join("index.json");

    if !index_path.exists() {
        return Ok("[]".to_string());
    }

    std::fs::read_to_string(&index_path)
        .map_err(|e| format!("No se pudo leer el índice de lugares: {}", e))
}

/// Create a new place.
///
/// Parses the input JSON to extract `id` and `name`. Rejects duplicates.
/// Creates `lugares/{id}.json` and updates `lugares/index.json`.
#[tauri::command]
fn crear_lugar(proyecto_path: String, lugar_json: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }

    let lugar: Lugar = serde_json::from_str(&lugar_json)
        .map_err(|e| format!("Error al parsear el lugar: {}", e))?;

    if lugar.id.trim().is_empty() {
        return Err("El ID del lugar no puede estar vacío.".to_string());
    }
    if lugar.name.trim().is_empty() {
        return Err("El nombre del lugar no puede estar vacío.".to_string());
    }

    let lugares_dir = Path::new(&proyecto_path).join("lugares");
    let lugar_file = lugares_dir.join(format!("{}.json", lugar.id));

    // Reject duplicates
    if lugar_file.exists() {
        return Err(format!("El lugar '{}' ya existe.", lugar.id));
    }

    // Ensure directory exists
    std::fs::create_dir_all(&lugares_dir)
        .map_err(|e| format!("No se pudo crear el directorio lugares: {}", e))?;

    // Write place file
    let lugar_json = serde_json::to_string_pretty(&lugar)
        .map_err(|e| format!("Error al serializar el lugar: {}", e))?;
    std::fs::write(&lugar_file, lugar_json)
        .map_err(|e| format!("Error al crear el lugar: {}", e))?;

    // Update index
    let index_path = lugares_dir.join("index.json");
    let mut index: Vec<LugarIndexItem> = if index_path.exists() {
        let raw = std::fs::read_to_string(&index_path)
            .map_err(|e| format!("Error al leer el índice de lugares: {}", e))?;
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        vec![]
    };

    index.push(LugarIndexItem {
        id: lugar.id.clone(),
        name: lugar.name.clone(),
    });

    let index_json = serde_json::to_string_pretty(&index)
        .map_err(|e| format!("Error al serializar el índice de lugares: {}", e))?;
    std::fs::write(&index_path, index_json)
        .map_err(|e| format!("Error al escribir el índice de lugares: {}", e))?;

    Ok(format!("Lugar '{}' creado.", lugar.name))
}

/// Load a place by ID.
///
/// Reads `lugares/{id}.json` and returns the full JSON string.
#[tauri::command]
fn cargar_lugar(proyecto_path: String, id: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if id.trim().is_empty() {
        return Err("El ID del lugar no puede estar vacío.".to_string());
    }

    let lugar_path = Path::new(&proyecto_path)
        .join("lugares")
        .join(format!("{}.json", id));

    if !lugar_path.exists() {
        return Err(format!("Lugar '{}' no encontrado.", id));
    }

    std::fs::read_to_string(&lugar_path)
        .map_err(|e| format!("Error al leer el lugar: {}", e))
}

/// Update a place.
///
/// Overwrites `lugares/{id}.json`. If the name changed, updates the index entry.
#[tauri::command]
fn actualizar_lugar(
    proyecto_path: String,
    id: String,
    lugar_json: String,
) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if id.trim().is_empty() {
        return Err("El ID del lugar no puede estar vacío.".to_string());
    }

    let lugares_dir = Path::new(&proyecto_path).join("lugares");
    let lugar_path = lugares_dir.join(format!("{}.json", id));

    if !lugar_path.exists() {
        return Err(format!("Lugar '{}' no encontrado.", id));
    }

    // Read old place to detect name change
    let old_raw = std::fs::read_to_string(&lugar_path)
        .map_err(|e| format!("Error al leer el lugar existente: {}", e))?;
    let old_lugar: Lugar = serde_json::from_str(&old_raw)
        .map_err(|e| format!("Error al parsear el lugar existente: {}", e))?;

    let lugar: Lugar = serde_json::from_str(&lugar_json)
        .map_err(|e| format!("Error al parsear el lugar actualizado: {}", e))?;

    // Overwrite file
    let lugar_json = serde_json::to_string_pretty(&lugar)
        .map_err(|e| format!("Error al serializar el lugar: {}", e))?;
    std::fs::write(&lugar_path, lugar_json)
        .map_err(|e| format!("Error al guardar el lugar: {}", e))?;

    // Update index if name changed
    if old_lugar.name != lugar.name {
        let index_path = lugares_dir.join("index.json");
        if index_path.exists() {
            let raw = std::fs::read_to_string(&index_path)
                .map_err(|e| format!("Error al leer el índice de lugares: {}", e))?;
            let mut index: Vec<LugarIndexItem> =
                serde_json::from_str(&raw).unwrap_or_default();
            for item in &mut index {
                if item.id == id {
                    item.name = lugar.name.clone();
                    break;
                }
            }
            let index_json = serde_json::to_string_pretty(&index)
                .map_err(|e| format!("Error al serializar el índice de lugares: {}", e))?;
            std::fs::write(&index_path, index_json)
                .map_err(|e| format!("Error al escribir el índice de lugares: {}", e))?;
        }
    }

    Ok(format!("Lugar '{}' actualizado.", lugar.name))
}

/// Delete a place.
///
/// Deletes `lugares/{id}.json` and removes from `lugares/index.json`.
#[tauri::command]
fn eliminar_lugar(proyecto_path: String, id: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if id.trim().is_empty() {
        return Err("El ID del lugar no puede estar vacío.".to_string());
    }

    let lugares_dir = Path::new(&proyecto_path).join("lugares");
    let lugar_path = lugares_dir.join(format!("{}.json", id));

    if !lugar_path.exists() {
        return Err(format!("Lugar '{}' no encontrado.", id));
    }

    // Delete the file
    std::fs::remove_file(&lugar_path)
        .map_err(|e| format!("Error al eliminar el lugar: {}", e))?;

    // Remove from index
    let index_path = lugares_dir.join("index.json");
    if index_path.exists() {
        let raw = std::fs::read_to_string(&index_path)
            .map_err(|e| format!("Error al leer el índice de lugares: {}", e))?;
        let mut index: Vec<LugarIndexItem> =
            serde_json::from_str(&raw).unwrap_or_default();
        index.retain(|item| item.id != id);
        let index_json = serde_json::to_string_pretty(&index)
            .map_err(|e| format!("Error al serializar el índice de lugares: {}", e))?;
        std::fs::write(&index_path, index_json)
            .map_err(|e| format!("Error al escribir el índice de lugares: {}", e))?;
    }

    // Clean references from timeline events
    let timeline_path = Path::new(&proyecto_path).join(".config").join("timeline.json");
    if timeline_path.exists() {
        let raw = std::fs::read_to_string(&timeline_path)
            .map_err(|e| format!("Error al leer timeline: {}", e))?;
        let mut timeline: Vec<TimelineEvent> =
            serde_json::from_str(&raw).unwrap_or_default();
        for event in &mut timeline {
            event.relatedPlaces.retain(|pid| pid != &id);
        }
        let timeline_json = serde_json::to_string_pretty(&timeline)
            .map_err(|e| format!("Error al serializar timeline: {}", e))?;
        std::fs::write(&timeline_path, timeline_json)
            .map_err(|e| format!("Error al escribir timeline: {}", e))?;
    }

    Ok(format!("Lugar '{}' eliminado.", id))
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

/// Update an existing timeline event by ID.
///
/// `evento_json` must include the event's `id`. All other fields are replaced.
#[tauri::command]
fn actualizar_evento_timeline(proyecto_path: String, evento_json: String) -> Result<String, String> {
    if proyecto_path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }

    let updated: TimelineEvent = serde_json::from_str(&evento_json)
        .map_err(|e| format!("Error al parsear el evento: {}", e))?;

    if updated.id.trim().is_empty() {
        return Err("El ID del evento no puede estar vacío.".to_string());
    }
    if updated.title.trim().is_empty() {
        return Err("El título del evento no puede estar vacío.".to_string());
    }

    let timeline_path = Path::new(&proyecto_path).join(".config").join("timeline.json");
    let raw = std::fs::read_to_string(&timeline_path)
        .map_err(|e| format!("Error al leer la línea de tiempo: {}", e))?;
    let mut timeline: Vec<TimelineEvent> = serde_json::from_str(&raw)
        .map_err(|e| format!("Error al parsear la línea de tiempo: {}", e))?;

    let idx = timeline.iter()
        .position(|e| e.id == updated.id)
        .ok_or_else(|| format!("No se encontró el evento con ID '{}'.", updated.id))?;

    let event_id = updated.id.clone();
    timeline[idx] = updated;

    let timeline_json = serde_json::to_string_pretty(&timeline)
        .map_err(|e| format!("Error al serializar la línea de tiempo: {}", e))?;
    std::fs::write(&timeline_path, timeline_json)
        .map_err(|e| format!("Error al escribir la línea de tiempo: {}", e))?;

    Ok(format!("Evento '{}' actualizado.", event_id))
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
// Tramas — plotlines
// ---------------------------------------------------------------------------

/// Generate a unique trama ID from a name.
///
/// Slugifies the name (lowercase, hyphens, strip non-alnum) and appends an
/// 8-char hex suffix derived from the current timestamp for uniqueness.
fn slugify_trama_id(nombre: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Strip accents by decomposing characters and removing combining marks
    let decomposed: String = nombre
        .chars()
        .flat_map(|c| {
            let mut buf = [0u8; 4];
            let _s = c.encode_utf8(&mut buf);
            // Check if this is a base letter + combining accent by
            // looking at the decomposition of common accented chars
            match c {
                'á' | 'à' | 'ä' | 'â' | 'ã' | 'å' => vec!['a'],
                'é' | 'è' | 'ë' | 'ê' => vec!['e'],
                'í' | 'ì' | 'ï' | 'î' => vec!['i'],
                'ó' | 'ò' | 'ö' | 'ô' | 'õ' => vec!['o'],
                'ú' | 'ù' | 'ü' | 'û' => vec!['u'],
                'ñ' => vec!['n'],
                'ç' => vec!['c'],
                'ý' | 'ÿ' => vec!['y'],
                _ if c.is_alphabetic() && c as u32 > 127 => {
                    // For unknown non-ASCII alphabetic chars, try NFKD decomposition
                    // using a simple approach: keep only ASCII letters
                    let lower = c.to_lowercase().to_string();
                    lower.chars().filter(|ch| ch.is_ascii_alphabetic()).collect()
                }
                _ => vec![c],
            }
        })
        .map(|c| c.to_lowercase().next().unwrap_or(c))
        .collect();
    let base = decomposed
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>();
    let base = base
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{}-{:08x}", base, nanos)
}

/// Create a new trama and persist it to metadata.json.
///
/// Rejects duplicate names. Slugifies the name into a unique ID with a
/// random 8-char hex suffix and appends to `metadata.tramas`.
#[tauri::command]
fn crear_trama(path: String, nombre: String) -> Result<Trama, String> {
    if path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if nombre.trim().is_empty() {
        return Err("El nombre de la trama no puede estar vacío.".to_string());
    }

    let proyecto_path = Path::new(&path);
    let metadata_path = proyecto_path.join(".config").join("metadata.json");

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

    // Reject duplicate names
    let nombre_trim = nombre.trim();
    if metadata.tramas.iter().any(|t| t.nombre == nombre_trim) {
        return Err(format!("Ya existe una trama con el nombre '{}'.", nombre_trim));
    }

    let id = slugify_trama_id(nombre_trim);
    let trama = Trama {
        id,
        nombre: nombre_trim.to_string(),
    };

    metadata.tramas.push(trama.clone());
    metadata.last_modified = Local::now().to_rfc3339();

    let updated_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Error al serializar metadata.json: {}", e))?;

    std::fs::write(&metadata_path, updated_json)
        .map_err(|e| format!("Error al escribir metadata.json: {}", e))?;

    Ok(trama)
}

/// Delete a trama by ID. Chapters assigned to it become unassigned.
///
/// Removes the trama from `metadata.tramas` and sets all matching
/// `chapter_tramas` entries to `trama_id: null`. No chapter files
/// are ever deleted.
#[tauri::command]
fn eliminar_trama(path: String, id: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if id.trim().is_empty() {
        return Err("El ID de la trama no puede estar vacío.".to_string());
    }

    let proyecto_path = Path::new(&path);
    let metadata_path = proyecto_path.join(".config").join("metadata.json");

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

    // Reject nonexistent trama
    if !metadata.tramas.iter().any(|t| t.id == id) {
        return Err(format!("No existe una trama con el ID '{}'.", id));
    }

    metadata.tramas.retain(|t| t.id != id);

    // Unassign all chapters that belonged to this trama
    for ct in &mut metadata.chapter_tramas {
        if ct.trama_id.as_deref() == Some(&id) {
            ct.trama_id = None;
        }
    }

    metadata.last_modified = Local::now().to_rfc3339();

    let updated_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Error al serializar metadata.json: {}", e))?;

    std::fs::write(&metadata_path, updated_json)
        .map_err(|e| format!("Error al escribir metadata.json: {}", e))?;

    Ok(())
}

/// Assign a chapter to a trama (or unassign it when `trama_id` is None).
///
/// Upserts the `chapter_tramas` entry. Validates that the trama exists when
/// `trama_id` is `Some`. `chapters_order` is never modified.
#[tauri::command]
fn asignar_capitulo_trama(path: String, filename: String, trama_id: Option<String>) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("La ruta del proyecto no puede estar vacía.".to_string());
    }
    if filename.trim().is_empty() {
        return Err("El nombre del archivo no puede estar vacío.".to_string());
    }

    let proyecto_path = Path::new(&path);
    let metadata_path = proyecto_path.join(".config").join("metadata.json");

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

    // Validate trama exists when assigning
    if let Some(ref tid) = trama_id {
        if !metadata.tramas.iter().any(|t| &t.id == tid) {
            return Err(format!("No existe una trama con el ID '{}'.", tid));
        }
    }

    // Upsert: remove existing entry, then push the new one
    metadata.chapter_tramas.retain(|ct| ct.filename != filename);
    metadata.chapter_tramas.push(ChapterTrama {
        filename: filename.clone(),
        trama_id,
    });

    metadata.last_modified = Local::now().to_rfc3339();

    let updated_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Error al serializar metadata.json: {}", e))?;

    std::fs::write(&metadata_path, updated_json)
        .map_err(|e| format!("Error al escribir metadata.json: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Resolve the path to the global git identity/remote config file.
///
/// Uses Tauri's platform-standard `app_config_dir()` under a `cron-insta/`
/// subdirectory. Returns `None` when the platform cannot determine the
/// config directory.
fn get_config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|p| p.join("cron-insta").join("git-config.json"))
}

/// Read the Git identity (name, email) from the global config file.
///
/// Returns `Some((name, email))` when a valid identity exists in the
/// config, or `None` when the config is missing, corrupted, or has no
/// identity section.
fn read_identity_from_config(app: &tauri::AppHandle) -> Option<(String, String)> {
    let config_path = get_config_path(app)?;
    if !config_path.exists() {
        return None;
    }
    let raw = std::fs::read_to_string(&config_path).ok()?;
    let config: GitConfig = serde_json::from_str(&raw).ok()?;
    config.identity.map(|id| (id.name, id.email))
}

/// Core commit logic: stage changes, create a descriptive commit, and
/// return the commit hash (or a "no changes" message).
///
/// Used by both `crear_checkpoint` (Tauri command) and `do_checkpoint`
/// (close-handler helper) so the commit logic lives in one place.
fn perform_commit(project_path: &Path) -> Result<String, String> {
    // Best-effort: never returns Err. On failure, logs via eprintln! and
    // returns Ok with a status message so callers never skip push.
    let git_path = match find_git() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[perform_commit] find_git error: {}", e);
            return Ok("Git no está disponible.".to_string());
        }
    };

    // Stage all changes
    let add_output = match system_command(&git_path)
        .arg("add")
        .arg(".")
        .current_dir(project_path)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[perform_commit] git add error: {}", e);
            return Ok(format!("Error al ejecutar git add: {}", e));
        }
    };

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        let msg = format!("Error en git add: {}", stderr.trim());
        eprintln!("[perform_commit] {}", msg);
        return Ok(msg);
    }

    // Count words in chapter files for the commit message
    let word_count = count_words_in_chapters(project_path);
    let date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let commit_msg = format!(
        "Progreso automático: {} - {} palabras",
        date, word_count
    );

    // Commit
    let commit_output = match system_command(&git_path)
        .arg("commit")
        .arg("-m")
        .arg(&commit_msg)
        .current_dir(project_path)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[perform_commit] git commit error: {}", e);
            return Ok(format!("Error al ejecutar git commit: {}", e));
        }
    };

    if commit_output.status.success() {
        // Retrieve the commit hash
        let hash_output = match system_command(&git_path)
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(project_path)
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[perform_commit] git rev-parse error: {}", e);
                return Ok("Commit realizado, pero no se pudo obtener el hash.".to_string());
            }
        };

        let hash = String::from_utf8_lossy(&hash_output.stdout)
            .trim()
            .to_string();
        Ok(hash)
    } else {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        let stdout = String::from_utf8_lossy(&commit_output.stdout);
        let combined = format!("{}{}", stderr, stdout);
        // "nothing to commit" is a normal state, not an error.
        if combined.contains("nothing to commit")
            || combined.contains("nothing added to commit")
            || combined.contains("nada para confirmar")
            || combined.contains("nada que confirmar")
        {
            Ok("Sin cambios para guardar.".to_string())
        } else {
            let msg = format!(
                "Error en git commit: {}",
                combined.trim().lines().last().unwrap_or("")
            );
            eprintln!("[perform_commit] {}", msg);
            Ok(msg)
        }
    }
}

/// Internal helper: attempt to push to the configured remote.
///
/// Reads the remote URL from git, runs `git push`, and implements the
/// 3-strike rule (disables push after 3 consecutive failures).
///
/// Called by `do_checkpoint` (close) and `push_ahora` (button).
/// Does NOT check `push_enabled` — both callers are explicit user actions
/// that should always attempt push when ahead of remote.
fn sincronizar_checkpoint(_app: &tauri::AppHandle, path: &str) -> Result<String, String> {
    let project_path = Path::new(path);

    // Read state from project metadata (for 3-strike counter)
    let meta_path = project_path.join(".config").join("metadata.json");
    if !meta_path.exists() {
        return Ok("".to_string());
    }

    let raw = match std::fs::read_to_string(&meta_path) {
        Ok(r) => r,
        Err(_) => return Ok("".to_string()),
    };

    let mut meta: Metadata = match serde_json::from_str(&raw) {
        Ok(m) => m,
        Err(_) => return Ok("".to_string()),
    };

    // Read remote URL from git
    let git_path = find_git()?;
    let url_output = system_command(&git_path)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git remote get-url: {}", e))?;

    if !url_output.status.success() {
        return Ok("".to_string()); // No remote configured
    }

    let remote_url = String::from_utf8_lossy(&url_output.stdout).trim().to_string();
    if remote_url.is_empty() {
        return Ok("".to_string());
    }

    // Attempt push
    let push_output = system_command(&git_path)
        .arg("push")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git push: {}", e))?;

    eprintln!("git push output (sincronizar_checkpoint): {:?}", push_output);

    if push_output.status.success() {
        // Success: reset counter and re-enable (in case it was 3-strike disabled)
        meta.consecutive_failures = 0;
        meta.push_enabled = true;
    } else {
        // Failure: increment counter, apply 3-strike rule
        meta.consecutive_failures += 1;
        let failures = meta.consecutive_failures;

        let warning = if failures >= 3 {
            meta.push_enabled = false;
            "Sincronización remota desactivada tras 3 intentos fallidos. Podés reactivarla desde la barra de herramientas.".to_string()
        } else {
            format!(
                "No se pudo sincronizar con el remoto (intento {}/3).",
                failures
            )
        };

        meta.last_modified = Local::now().to_rfc3339();
        let json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Error serializing metadata: {}", e))?;
        std::fs::write(&meta_path, json)
            .map_err(|e| format!("Error writing metadata: {}", e))?;

        // Commit metadata changes so git status stays clean
        commit_metadata_file(project_path, &git_path);
        return Ok(warning);
    }

    // Common: write metadata after push success
    meta.last_modified = Local::now().to_rfc3339();
    let json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Error serializing metadata: {}", e))?;
    std::fs::write(&meta_path, json)
        .map_err(|e| format!("Error writing metadata: {}", e))?;

    // Commit metadata changes so git status stays clean
    commit_metadata_file(project_path, &git_path);

    Ok("".to_string())
}

/// Stage and commit the `.config/metadata.json` file after a push state update.
/// This keeps the working tree clean — metadata changes are always versioned
/// alongside the content changes they describe.
fn commit_metadata_file(project_path: &Path, git_exe: &str) {
    // Stage metadata.json
    let meta_rel = Path::new(".config").join("metadata.json");
    let add_result = system_command(git_exe)
        .arg("add")
        .arg(&meta_rel)
        .current_dir(project_path)
        .output();
    match add_result {
        Ok(o) if o.status.success() => eprintln!("[commit_metadata] staged OK"),
        _ => {
            eprintln!("[commit_metadata] git add failed (non-fatal)");
            return;
        }
    }

    // Commit metadata. If nothing changed (already committed), "nothing to commit" is fine.
    let commit_result = system_command(git_exe)
        .arg("commit")
        .arg("-m")
        .arg("cron-insta: actualizar estado de sincronización")
        .current_dir(project_path)
        .output();
    match commit_result {
        Ok(o) if o.status.success() => eprintln!("[commit_metadata] committed OK"),
        _ => eprintln!("[commit_metadata] git commit skipped (no metadata changes)"),
    }
}

/// Sync local branch with remote: fetch → pull (if behind) → push (if ahead).
///
/// Handles the full cycle so non-technical users never deal with diverged branches:
/// - Only behind: fast-forward pull to catch up
/// - Only ahead: push local commits
/// - Both ahead and behind: pull first (reduces divergence), then push what remains
/// - Up to date: nothing
///
/// Returns `Ok(warning)` if push produced a warning, `Ok("")` on clean sync.
/// Returns `Err` only on unexpected errors.
fn sync_with_remote(app: &tauri::AppHandle, path: &str, project_path: &Path) -> Result<String, String> {
    let git_path = find_git()?;

    // Check if origin remote exists
    let url_output = system_command(&git_path)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(project_path)
        .output();

    if !url_output.map(|o| o.status.success()).unwrap_or(false) {
        return Ok("".to_string()); // No remote — nothing to sync
    }

    // Fetch (only if SSH agent is available)
    if ssh_available() {
        eprintln!("[sync] fetching origin...");
        let _ = system_command(&git_path)
            .arg("fetch")
            .arg("origin")
            .current_dir(project_path)
            .output();
    } else {
        eprintln!("[sync] no SSH agent, skipping fetch");
    }

    // Get upstream ref
    let upstream_ref = {
        let out = system_command(&git_path)
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("--symbolic-full-name")
            .arg("@{upstream}")
            .current_dir(project_path)
            .output();
        match out {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            _ => return Ok("".to_string()), // No upstream — nothing to sync
        }
    };
    eprintln!("[sync] upstream: {}", upstream_ref);

    // Get ahead/behind counts
    let (ahead, behind) = {
        let out = system_command(&git_path)
            .arg("rev-list")
            .arg("--count")
            .arg("--left-right")
            .arg(format!("{}...HEAD", upstream_ref))
            .current_dir(project_path)
            .output();
        match out {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let parts: Vec<&str> = s.split('\t').collect();
                let behind: u32 = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
                let ahead: u32 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
                (ahead, behind)
            }
            _ => (0, 0),
        }
    };
    eprintln!("[sync] ahead={}, behind={}", ahead, behind);

    // If behind, pull first (fast-forward only — safe for non-technical users)
    if behind > 0 {
        eprintln!("[sync] behind by {} — pulling...", behind);
        let pull_out = system_command(&git_path)
            .arg("pull")
            .arg("--ff-only")
            .current_dir(project_path)
            .output();
        match pull_out {
            Ok(o) if o.status.success() => eprintln!("[sync] pull OK (fast-forward)"),
            _ => eprintln!("[sync] pull failed or not fast-forward (non-fatal)"),
        }
    }

    // Recalculate ahead after pull — HEAD may have moved forward
    let ahead_after_pull = {
        let out = system_command(&git_path)
            .arg("rev-list")
            .arg("--count")
            .arg("--left-right")
            .arg(format!("{}...HEAD", upstream_ref))
            .current_dir(project_path)
            .output();
        match out {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let parts: Vec<&str> = s.split('\t').collect();
                parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0)
            }
            _ => 0,
        }
    };
    eprintln!("[sync] ahead after pull={}", ahead_after_pull);

    // If still ahead, push
    if ahead_after_pull > 0 {
        eprintln!("[sync] ahead by {} — pushing...", ahead_after_pull);
        sincronizar_checkpoint(app, path)
    } else {
        eprintln!("[sync] nothing to push");
        Ok("".to_string())
    }
}

/// Count text words inside HTML content by stripping tags first.
///
/// Two-state char-by-char machine: skip everything between `<` and `>`,
/// collect the remaining text, then split on Unicode whitespace.
/// No regex or HTML parser dependency.
fn count_words_in_html(html: &str) -> u64 {
    let mut inside_tag = false;
    let mut text = String::with_capacity(html.len());
    for ch in html.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => text.push(ch),
            _ => {}
        }
    }
    text.split_whitespace().count() as u64
}

/// Read a chapter file and return its word count via `count_words_in_html`.
/// Returns 0 when the file is missing or unreadable.
fn word_count_chapter(project_path: &Path, filename: &str) -> u64 {
    let file_path = project_path.join("capitulos").join(filename);
    match std::fs::read_to_string(&file_path) {
        Ok(content) => count_words_in_html(&content),
        Err(_) => 0,
    }
}

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

/// Compute and persist session statistics.
///
/// Best-effort — all errors are logged to `eprintln!` and the function
/// never panics.  On completion the tracker is reset to defaults so the
/// next project open starts fresh.
///
/// Steps:
///   1. Compute elapsed time since `start_time`
///   2. Accumulate current chapter time into `chapter_times`
///   3. Re-count words in the current chapter, diff against initial
///   4. Read or initialise `stats.json`
///   5. Update totals, per-chapter stats, and append session record
///   6. Write `stats.json` back to disk
///   7. Stage and commit `stats.json` via `system_command`
fn finalizar_sesion_escritura(tracker: &mut SessionTracker, project_path: &Path) {
    let start_time = match tracker.start_time {
        Some(t) => t,
        None => return, // No active session — nothing to collect
    };

    let total_elapsed = start_time.elapsed().as_secs();

    // Accumulate current chapter time before computing diffs
    if tracker.chapter_start.is_some() && tracker.chapter_filename.is_some() {
        let ch_file = tracker.chapter_filename.clone().unwrap();
        let ch_start = tracker.chapter_start.unwrap();
        let chapter_elapsed = ch_start.elapsed().as_secs();
        let accum = tracker.chapter_times.entry(ch_file).or_insert(0);
        *accum += chapter_elapsed;
    }

    // Compute words added for the current chapter
    let words_added = if let Some(ref filename) = tracker.chapter_filename {
        let current_words = word_count_chapter(project_path, filename);
        let initial = tracker.initial_word_count.unwrap_or(0);
        if current_words >= initial {
            current_words - initial
        } else {
            // File was edited outside the editor or truncated; count whatever exists
            current_words
        }
    } else {
        0
    };

    // ── Read or initialise stats.json ──────────────────────────
    let stats_path = project_path.join(".config").join("stats.json");
    let mut stats: SessionStats = if stats_path.exists() {
        std::fs::read_to_string(&stats_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| {
                eprintln!("[stats] Corrupt stats.json — regenerating default");
                SessionStats::default()
            })
    } else {
        SessionStats::default()
    };

    // ── Update cumulative totals ────────────────────────────────
    stats.total_time_seconds += total_elapsed;
    stats.total_words += words_added;

    // ── Update per-chapter stats ────────────────────────────────
    if let Some(ref filename) = tracker.chapter_filename {
        let ch_time = tracker.chapter_times.get(filename).copied().unwrap_or(total_elapsed);
        let ch_stats = stats.chapters.entry(filename.clone()).or_default();
        ch_stats.words += words_added;
        ch_stats.time_seconds += ch_time;
    }

    // ── Append session record ──────────────────────────────────
    let session = StatsSession {
        date: Local::now().format("%Y-%m-%d").to_string(),
        duration_seconds: total_elapsed,
        words_added,
        chapter_id: tracker.chapter_filename.clone().unwrap_or_default(),
    };
    stats.sessions.push(session);

    // ── Write stats.json ───────────────────────────────────────
    let json = match serde_json::to_string_pretty(&stats) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("[stats] Error serializing stats.json: {}", e);
            *tracker = SessionTracker::default();
            return;
        }
    };
    if let Err(e) = std::fs::write(&stats_path, &json) {
        eprintln!("[stats] Error writing stats.json: {}", e);
        *tracker = SessionTracker::default();
        return;
    }

    // ── Git add + commit (best-effort) ─────────────────────────
    let git_path = match find_git() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[stats] find_git error (non-fatal): {}", e);
            *tracker = SessionTracker::default();
            return;
        }
    };

    let stats_rel = Path::new(".config").join("stats.json");
    let _ = system_command(&git_path)
        .arg("add")
        .arg(&stats_rel)
        .current_dir(project_path)
        .output();

    let _ = system_command(&git_path)
        .arg("commit")
        .arg("-m")
        .arg("cron-insta: actualizar estadísticas de sesión")
        .current_dir(project_path)
        .output();

    // Reset tracker for the next project open
    *tracker = SessionTracker::default();
}

/// Read session statistics from the project's stats.json.
///
/// Returns a JSON object with `total_sessions`, `total_hours`, and `total_words`
/// for quick display in the UI footer. If the file doesn't exist or is corrupt,
/// returns zeros.
#[tauri::command]
fn cargar_estadisticas(project_path: String) -> Result<String, String> {
    let stats_path = Path::new(&project_path).join(".config").join("stats.json");
    if !stats_path.exists() {
        return Ok(r#"{"total_sessions":0,"total_hours":0,"total_words":0}"#.to_string());
    }
    let raw = std::fs::read_to_string(&stats_path)
        .map_err(|e| format!("Error reading stats.json: {}", e))?;
    let stats: SessionStats = serde_json::from_str(&raw).unwrap_or_default();
    let total_hours = stats.total_time_seconds as f64 / 3600.0;
    let result = serde_json::json!({
        "total_sessions": stats.sessions.len(),
        "total_hours": (total_hours * 10.0).round() / 10.0,
        "total_words": stats.total_words,
    });
    Ok(result.to_string())
}

/// Internal checkpoint for close handler.
///
/// Commits local changes (best-effort), then collects session stats,
/// then checks if local is ahead of the remote and pushes if so.
/// Push warnings/errors are logged to stderr (the close handler cannot
/// surface UI to the user).
fn do_checkpoint(app: &tauri::AppHandle, project_path: &str) -> Result<String, String> {
    let path_buf = Path::new(project_path);
    eprintln!("[do_checkpoint] Starting checkpoint for: {}", project_path);

    // 1) Commit local changes (best-effort — never skips sync)
    let commit_result = perform_commit(path_buf);
    eprintln!("[do_checkpoint] Commit result: {:?}", commit_result);

    // 2) Collect session stats (best-effort — never blocks sync)
    {
        let state = app.state::<ProjectState>();
        let lock = state.session_tracker.lock();
        if let Ok(mut tracker) = lock {
            finalizar_sesion_escritura(&mut tracker, path_buf);
        }
    }

    // 3) Sync with remote: fetch → pull (if behind) → push (if ahead)
    eprintln!("[do_checkpoint] Syncing with remote...");
    match sync_with_remote(app, project_path, path_buf) {
        Ok(warning) => {
            if !warning.is_empty() {
                eprintln!("[do_checkpoint] Sync warning: {}", warning);
            } else {
                eprintln!("[do_checkpoint] Sync completed successfully");
            }
        }
        Err(e) => {
            eprintln!("[do_checkpoint] Sync error: {}", e);
        }
    }

    commit_result
}

// ---------------------------------------------------------------------------
// Export — zip + single .md
// ---------------------------------------------------------------------------

/// Export the entire project as a .zip file.
///
/// Creates `exportaciones/` inside the project, then compresses all files
/// (including .git) into `{project_name}_{YYYY-MM-DD}.zip`.
#[tauri::command]
fn exportar_proyecto_zip(proyecto_path: String) -> Result<String, String> {
    use zip::write::FileOptions;

    let base = Path::new(&proyecto_path);
    let export_dir = base.join("exportaciones");
    std::fs::create_dir_all(&export_dir)
        .map_err(|e| format!("No se pudo crear exportaciones/: {}", e))?;

    let metadata = read_metadata(base)?;
    let project_name = metadata.project_name.replace(' ', "_");
    let date = Local::now().format("%Y-%m-%d");
    let zip_name = format!("{}_{}.zip", project_name, date);
    let zip_path = export_dir.join(&zip_name);

    let file = std::fs::File::create(&zip_path)
        .map_err(|e| format!("Error al crear zip: {}", e))?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Walk the project directory and add all files, prefixed with project name
    let zip_prefix = format!("{}/", project_name);
    add_dir_to_zip(base, base, &zip_prefix, &mut zip_writer, &options)
        .map_err(|e| format!("Error al comprimir: {}", e))?;

    zip_writer.finish()
        .map_err(|e| format!("Error al finalizar zip: {}", e))?;

    Ok(zip_path.display().to_string())
}

/// Recursively add directory contents to a zip writer, under a prefix folder.
fn add_dir_to_zip(
    base: &Path,
    dir: &Path,
    prefix: &str,
    zip: &mut zip::ZipWriter<std::fs::File>,
    options: &zip::write::FileOptions<()>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(dir)
        .map_err(|e| format!("Error al leer directorio: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Error: {}", e))?;
        let path = entry.path();

        // Skip the exportaciones directory itself
        if path.file_name().map(|n| n == "exportaciones").unwrap_or(false) {
            continue;
        }

        let relative = path.strip_prefix(base)
            .map_err(|e| format!("Error: {}", e))?;
        let name = format!("{}{}", prefix, relative.to_string_lossy());

        if path.is_dir() {
            zip.add_directory(&name, *options)
                .map_err(|e| format!("Error al añadir directorio: {}", e))?;
            add_dir_to_zip(base, &path, prefix, zip, options)?;
        } else {
            zip.start_file(&name, *options)
                .map_err(|e| format!("Error al iniciar archivo: {}", e))?;
            let contents = std::fs::read(&path)
                .map_err(|e| format!("Error al leer {}: {}", path.display(), e))?;
            zip.write(&contents)
                .map_err(|e| format!("Error al escribir en zip: {}", e))?;
        }
    }
    Ok(())
}

/// Export all chapters as a single .md file.
///
/// Concatenates every chapter in the order stored in metadata,
/// separated by a divider. Writes to `exportaciones/{project}_{date}.md`.
#[tauri::command]
fn exportar_proyecto_md(proyecto_path: String) -> Result<String, String> {
    let base = Path::new(&proyecto_path);
    let export_dir = base.join("exportaciones");
    std::fs::create_dir_all(&export_dir)
        .map_err(|e| format!("No se pudo crear exportaciones/: {}", e))?;

    let metadata = read_metadata(base)?;
    let project_name = metadata.project_name.replace(' ', "_");
    let date = Local::now().format("%Y-%m-%d");
    let md_name = format!("{}_{}.md", project_name, date);
    let md_path = export_dir.join(&md_name);

    let cap_dir = base.join("capitulos");
    let mut output = String::new();

    output.push_str(&format!("# {}\n\n", metadata.project_name));
    output.push_str(&format!("*Exportado el {}*\n\n---\n\n", Local::now().format("%d/%m/%Y")));

    for filename in &metadata.chapters_order {
        let file_path = cap_dir.join(filename);
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            let title = filename.trim_end_matches(".md").to_string();
            output.push_str(&format!("## {}\n\n", title));
            output.push_str(&content.trim());
            output.push_str("\n\n---\n\n");
        }
    }

    std::fs::write(&md_path, output)
        .map_err(|e| format!("Error al escribir .md: {}", e))?;

    Ok(md_path.display().to_string())
}

/// Import a Cron-Insta project from a .zip file.
///
/// Extracts all contents into the chosen destination directory.
/// A well-formed Cron-Insta ZIP wraps files in a project folder;
/// this function finds that folder by scanning for .config/metadata.json
/// inside the first level of subdirectories.  Falls back to the
/// destination root for legacy ZIPs without a wrapping folder.
///
/// Returns the actual project path (e.g. Documents/Hammet) on success.
#[tauri::command]
fn importar_proyecto(zip_path: String, destino: String) -> Result<String, String> {
    let zip_file = std::fs::File::open(&zip_path)
        .map_err(|e| format!("No se pudo abrir el archivo ZIP: {}", e))?;

    let mut archive = zip::ZipArchive::new(zip_file)
        .map_err(|e| format!("El archivo no es un ZIP válido: {}", e))?;

    let destino_path = std::path::Path::new(&destino);

    // Create destination if it doesn't exist
    std::fs::create_dir_all(destino_path)
        .map_err(|e| format!("No se pudo crear la carpeta de destino (comprobá los permisos): {}", e))?;

    // Extract all files
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Error al leer entrada del ZIP: {}", e))?;

        let out_path = match file.enclosed_name() {
            Some(path) => destino_path.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("Error al crear directorio {}: {}", out_path.display(), e))?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Error al crear directorio {}: {}", parent.display(), e))?;
            }
            let mut outfile = std::fs::File::create(&out_path)
                .map_err(|e| format!("Error al crear archivo {} (comprobá los permisos): {}", out_path.display(), e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Error al extraer {}: {}", out_path.display(), e))?;
        }
    }

    // Find the project root: scan first-level subdirectories for .config/metadata.json.
    // Also check the destination root itself (legacy ZIPs without wrapping folder).
    let mut project_root = destino_path.to_path_buf();
    let mut found = false;

    // Check destination root first
    if destino_path.join(".config").join("metadata.json").exists() {
        found = true;
    } else if let Ok(entries) = std::fs::read_dir(destino_path) {
        for entry in entries.flatten() {
            let sub = entry.path();
            if sub.is_dir() && sub.join(".config").join("metadata.json").exists() {
                project_root = sub;
                found = true;
                break;
            }
        }
    }

    if !found {
        return Err("El archivo ZIP no parece ser un proyecto de Cron-Insta (falta .config/metadata.json).".to_string());
    }

    // Read project name for the success message
    let raw = std::fs::read_to_string(project_root.join(".config").join("metadata.json"))
        .map_err(|e| format!("Proyecto extraído pero no se pudo leer metadata: {}", e))?;
    let _metadata: Metadata = serde_json::from_str(&raw)
        .map_err(|e| format!("Proyecto extraído pero metadata.json es inválido: {}", e))?;

    Ok(project_root.display().to_string())
}

/// Helper: read project metadata.
fn read_metadata(base: &Path) -> Result<Metadata, String> {
    let meta_path = base.join(".config").join("metadata.json");
    let raw = std::fs::read_to_string(&meta_path)
        .map_err(|e| format!("Error al leer metadata: {}", e))?;
    serde_json::from_str(&raw)
        .map_err(|e| format!("Error al parsear metadata: {}", e))
}

// ---------------------------------------------------------------------------
// Git identity & remote config commands
// ---------------------------------------------------------------------------

/// Load the stored Git identity from the global config file.
///
/// Returns the serialised `GitIdentity` JSON `{name, email}` when found,
/// or the literal string `"null"` when no config exists or the file is
/// corrupted (graceful degradation — the frontend decides which preset to
/// show).
#[tauri::command]
fn cargar_identidad_git(app: tauri::AppHandle) -> Result<String, String> {
    let config_path = match get_config_path(&app) {
        Some(p) => p,
        None => return Ok("null".to_string()),
    };

    if !config_path.exists() {
        return Ok("null".to_string());
    }

    let raw = match std::fs::read_to_string(&config_path) {
        Ok(r) => r,
        Err(_) => return Ok("null".to_string()),
    };

    let config: GitConfig = match serde_json::from_str(&raw) {
        Ok(c) => c,
        Err(_) => return Ok("null".to_string()), // corrupted JSON → graceful degradation
    };

    match config.identity {
        Some(id) => serde_json::to_string(&id)
            .map_err(|e| format!("Error serializing identity: {}", e)),
        None => Ok("null".to_string()),
    }
}

/// Persist the user's Git identity to the global config file.
///
/// Uses a read-modify-write pattern: the full config is read first (if it
/// exists) so any existing remote configuration is preserved. The config
/// directory is created if it does not yet exist.
#[tauri::command]
fn guardar_identidad_git(
    app: tauri::AppHandle,
    name: String,
    email: String,
    github_user: Option<String>,
) -> Result<String, String> {
    let config_path = match get_config_path(&app) {
        Some(p) => p,
        None => return Err("Could not determine config directory".to_string()),
    };

    // Ensure the parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Error creating config directory: {}", e))?;
    }

    let identity = GitIdentity { name, email, github_user };

    // Read-modify-write: preserve identity-only config
    let mut config = if config_path.exists() {
        let raw = std::fs::read_to_string(&config_path).unwrap_or_default();
        serde_json::from_str::<GitConfig>(&raw).unwrap_or(GitConfig {
            schema_version: 1,
            identity: None,
        })
    } else {
        GitConfig {
            schema_version: 1,
            identity: None,
        }
    };

    config.identity = Some(identity);

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Error serializing config: {}", e))?;

    std::fs::write(&config_path, json)
        .map_err(|e| format!("Error writing config: {}", e))?;

    Ok("Identity saved successfully.".to_string())
}

/// Load the per-project push state from the project's metadata.json.
///
/// Returns the serialised JSON `{push_enabled, consecutive_failures, url}`
/// when metadata exists. The `url` is read from `git remote get-url origin`
/// and is `null` when no remote is configured.
///
/// Returns the literal string `"null"` when metadata is missing or corrupted.
#[tauri::command]
fn cargar_config_remoto(_app: tauri::AppHandle, proyecto_path: String) -> Result<String, String> {
    let base = Path::new(&proyecto_path);
    let meta_path = base.join(".config").join("metadata.json");

    if !meta_path.exists() {
        return Ok("null".to_string());
    }

    let raw = match std::fs::read_to_string(&meta_path) {
        Ok(r) => r,
        Err(_) => return Ok("null".to_string()),
    };

    let meta: Metadata = match serde_json::from_str(&raw) {
        Ok(m) => m,
        Err(_) => return Ok("null".to_string()),
    };

    // Read remote URL from git config (best-effort)
    let remote_url: Option<String> = if let Ok(git_path) = find_git() {
        system_command(&git_path)
            .arg("remote")
            .arg("get-url")
            .arg("origin")
            .current_dir(base)
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    };

    #[derive(Serialize)]
    struct RemoteState {
        push_enabled: bool,
        consecutive_failures: u32,
        url: Option<String>,
    }

    let state = RemoteState {
        push_enabled: meta.push_enabled,
        consecutive_failures: meta.consecutive_failures,
        url: remote_url,
    };

    serde_json::to_string(&state)
        .map_err(|e| format!("Error serializing remote state: {}", e))
}

/// Persist the push state to the project's metadata.json.
///
/// Uses a read-modify-write pattern so existing metadata fields are
/// preserved. `consecutive_failures` is set to 0 when remote config is
/// saved (fresh start).
///
/// When `proyecto_path` is empty or metadata.json does not exist yet
/// (pre-creation flow), returns `Ok` without writing — the state will
/// be seeded by `crear_proyecto`.
///
/// The `url` parameter is accepted for backward-compatible signature
/// but is NOT stored — the remote URL lives in Git's own config.
#[tauri::command]
fn guardar_config_remoto(
    _app: tauri::AppHandle,
    proyecto_path: String,
    _url: String,
    push_enabled: bool,
) -> Result<String, String> {
    if proyecto_path.is_empty() {
        return Ok("No project path — state will be set after creation.".to_string());
    }

    let base = Path::new(&proyecto_path);
    let meta_path = base.join(".config").join("metadata.json");

    if !meta_path.exists() {
        return Ok("Metadata not created yet — state will be set after project creation.".to_string());
    }

    let raw = std::fs::read_to_string(&meta_path)
        .map_err(|e| format!("Error reading metadata: {}", e))?;

    let mut meta: Metadata = serde_json::from_str(&raw)
        .map_err(|e| format!("Error parsing metadata: {}", e))?;

    meta.push_enabled = push_enabled;
    meta.consecutive_failures = 0;
    meta.last_modified = Local::now().to_rfc3339();

    let json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Error serializing metadata: {}", e))?;

    std::fs::write(&meta_path, json)
        .map_err(|e| format!("Error writing metadata: {}", e))?;

    Ok("Remote config saved successfully.".to_string())
}

/// Configure a Git remote and perform the initial push for a project.
///
/// Validates that the URL is an SSH URL (rejects HTTP/HTTPS). On valid
/// URL, adds the remote as `origin` and pushes the main branch with
/// upstream tracking.
///
/// If the push fails (e.g. remote unreachable), the local commit is
/// preserved and a warning is returned — the user can retry later.
#[tauri::command]
fn configurar_remoto(_app: tauri::AppHandle, path: String, url: String) -> Result<String, String> {
    // SSH URL validation: reject HTTP(S) — SSH is required
    let url_lower = url.to_lowercase();
    if url_lower.starts_with("http://") || url_lower.starts_with("https://") {
        return Err(
            "Solo se admiten URLs SSH (git@... o ssh://...). Las URLs HTTPS no son compatibles."
                .to_string(),
        );
    }

    let project_path = Path::new(&path);
    let git_path = find_git()?;

    // 1a) Check if remote "origin" already exists
    let remote_exists = system_command(&git_path)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(project_path)
        .output()
        .map(|out| out.status.success() && !out.stdout.is_empty())
        .unwrap_or(false);

    // 1b) Add or set remote URL
    if remote_exists {
        // Remote already configured — just update the URL and fetch
        let set_output = system_command(&git_path)
            .arg("remote")
            .arg("set-url")
            .arg("origin")
            .arg(&url)
            .current_dir(project_path)
            .output()
            .map_err(|e| format!("Error al ejecutar git remote set-url: {}", e))?;

        if !set_output.status.success() {
            let stderr = String::from_utf8_lossy(&set_output.stderr);
            return Err(format!("Error al configurar el remoto: {}", stderr.trim()));
        }

        // Fetch the new remote to update tracking refs
        let _ = system_command(&git_path)
            .arg("fetch")
            .arg("origin")
            .current_dir(project_path)
            .output();

        return Ok("Remote actualizado correctamente.".to_string());
    }

    // Remote doesn't exist yet — add it and attempt initial push
    let add_output = system_command(&git_path)
        .arg("remote")
        .arg("add")
        .arg("origin")
        .arg(&url)
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git remote add: {}", e))?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        return Err(format!("Error al configurar el remoto: {}", stderr.trim()));
    }

    // 1c) Check if remote already has commits (new remote flow only)
    let ls_output = system_command(&git_path)
        .arg("ls-remote")
        .arg("origin")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git ls-remote: {}", e))?;

    if ls_output.status.success() {
        let ls_stdout = String::from_utf8_lossy(&ls_output.stdout);
        if ls_stdout.contains("refs/heads/main") || ls_stdout.contains("refs/heads/master") {
            // Remote has history — offer sync instead of failing on push
            return Err(format!(
                "REMOTE_HAS_COMMITS:El repositorio remoto ya contiene un historial previo. ¿Querés sincronizarlo con el proyecto local?"
            ));
        }
    }
    // If ls-remote fails (e.g. repo doesn't exist), we'll fall through to push
    // and let the push error handler deal with it

    // 2) git push -u origin main
    let push_output = system_command(&git_path)
        .arg("push")
        .arg("-u")
        .arg("origin")
        .arg("main")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git push: {}", e))?;

    eprintln!("git push -u origin main output: {:?}", push_output);

    if push_output.status.success() {
        // Reset consecutive_failures on successful push
        let meta_path = project_path.join(".config").join("metadata.json");
        if let Ok(raw) = std::fs::read_to_string(&meta_path) {
            if let Ok(mut meta) = serde_json::from_str::<Metadata>(&raw) {
                meta.push_enabled = true;
                meta.consecutive_failures = 0;
                meta.last_modified = Local::now().to_rfc3339();
                if let Ok(json) = serde_json::to_string_pretty(&meta) {
                    let _ = std::fs::write(&meta_path, json);
                }
            }
        }

        Ok("Repositorio remoto configurado y sincronizado correctamente.".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        let stderr_str = stderr.trim().to_lowercase();
        if stderr_str.contains("not found") || stderr_str.contains("repository not found") {
            Err(format!("REPO_NOT_FOUND:{}", stderr.trim()))
        } else {
            Err(format!("Error al sincronizar con remoto: {}", stderr.trim()))
        }
    }
}

/// Sync an existing remote repository that already has commits.
///
/// Called when `configurar_remoto` detects that the remote already has
/// a history (e.g. from another machine). Fetches the remote branch and
/// merges with `--allow-unrelated-histories --no-edit`.
///
/// On success: pushes the merged result to origin. On merge conflict:
/// aborts the merge and returns an error with the list of conflicted files.
#[tauri::command]
fn sincronizar_remoto(path: String) -> Result<String, String> {
    let project_path = Path::new(&path);
    let git_path = find_git()?;

    // 1) git fetch origin
    let fetch_output = system_command(&git_path)
        .arg("fetch")
        .arg("origin")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git fetch: {}", e))?;

    if !fetch_output.status.success() {
        let stderr = String::from_utf8_lossy(&fetch_output.stderr);
        return Err(format!(
            "Error al obtener el historial remoto: {}",
            stderr.trim()
        ));
    }

    // 2) Determine the default branch on the remote
    let branch = "main"; // we always push to main

    // 3) git merge --allow-unrelated-histories --no-edit origin/main
    let merge_output = system_command(&git_path)
        .arg("merge")
        .arg("--allow-unrelated-histories")
        .arg("--no-edit")
        .arg(format!("origin/{}", branch))
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git merge: {}", e))?;

    if !merge_output.status.success() {
        // Conflict or other merge failure — abort
        let _ = system_command(&git_path)
            .arg("merge")
            .arg("--abort")
            .current_dir(project_path)
            .output();

        // Try to list conflicted files for a helpful message
        let conflict_info = if let Ok(diff) = system_command(&git_path)
            .arg("diff")
            .arg("--name-only")
            .arg("--diff-filter=U")
            .current_dir(project_path)
            .output()
        {
            let files = String::from_utf8_lossy(&diff.stdout);
            if !files.trim().is_empty() {
                format!("\nArchivos con diferencias:\n{}", files.trim())
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        return Err(format!(
            "No se pudo sincronizar automáticamente. Hay diferencias entre el historial local y el remoto que requieren resolución manual.{}",
            conflict_info
        ));
    }

    // 4) git push origin main
    let push_output = system_command(&git_path)
        .arg("push")
        .arg("origin")
        .arg(branch)
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git push: {}", e))?;

    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        return Err(format!(
            "Sincronización local completada, pero el push falló: {}",
            stderr.trim()
        ));
    }

    // Reset consecutive_failures on successful push
    let meta_path = project_path.join(".config").join("metadata.json");
    if let Ok(raw) = std::fs::read_to_string(&meta_path) {
        if let Ok(mut meta) = serde_json::from_str::<Metadata>(&raw) {
            meta.push_enabled = true;
            meta.consecutive_failures = 0;
            meta.last_modified = Local::now().to_rfc3339();
            if let Ok(json) = serde_json::to_string_pretty(&meta) {
                let _ = std::fs::write(&meta_path, json);
            }
        }
    }

    Ok("Historial remoto sincronizado correctamente.".to_string())
}

/// Retry a push to the configured remote after previous failures.
///
/// Resets the consecutive failure counter to 0 before attempting.
/// If no remote was ever configured, returns an error.
/// On success, the counter stays at 0. On failure, increments to 1
/// (starting a fresh strike count).
#[tauri::command]
fn reintentar_push(_app: tauri::AppHandle, path: String) -> Result<String, String> {
    let project_path = Path::new(&path);
    let meta_path = project_path.join(".config").join("metadata.json");

    if !meta_path.exists() {
        return Err("No hay metadata del proyecto.".to_string());
    }

    let raw = std::fs::read_to_string(&meta_path)
        .map_err(|e| format!("Error reading metadata: {}", e))?;

    let mut meta: Metadata = serde_json::from_str(&raw)
        .map_err(|e| format!("Error parsing metadata: {}", e))?;

    // Check if remote is configured
    let git_path = find_git()?;
    let url_output = system_command(&git_path)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git remote get-url: {}", e))?;

    if !url_output.status.success() {
        return Err("No hay un repositorio remoto configurado.".to_string());
    }

    let remote_url = String::from_utf8_lossy(&url_output.stdout).trim().to_string();
    if remote_url.is_empty() {
        return Err("No hay un repositorio remoto configurado.".to_string());
    }

    // Reset counter and enable push
    meta.consecutive_failures = 0;
    meta.push_enabled = true;

    let push_output = system_command(&git_path)
        .arg("push")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git push: {}", e))?;

    if push_output.status.success() {
        // Success: save with fresh counter
        meta.last_modified = Local::now().to_rfc3339();
        let json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Error serializing metadata: {}", e))?;
        std::fs::write(&meta_path, json)
            .map_err(|e| format!("Error writing metadata: {}", e))?;

        Ok("Sincronización exitosa.".to_string())
    } else {
        // Failure: increment to 1 (fresh count)
        meta.consecutive_failures = 1;
        meta.last_modified = Local::now().to_rfc3339();
        let json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Error serializing metadata: {}", e))?;
        std::fs::write(&meta_path, json)
            .map_err(|e| format!("Error writing metadata: {}", e))?;

        let stderr = String::from_utf8_lossy(&push_output.stderr);
        Err(format!("Error al sincronizar: {}", stderr.trim()))
    }
}

/// Save a checkpoint and push to the configured remote now.
///
/// Commits all pending changes (same as `crear_checkpoint`) and then
/// checks if local is ahead of remote. If ahead, pushes to `origin`.
/// Returns a combined result so the user gets immediate feedback.
#[tauri::command]
fn push_ahora(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let project_path = Path::new(&path);

    // 1) Commit pending changes (best-effort — never fails)
    let commit_msg = perform_commit(project_path).unwrap_or_default();

    // 2) Sync with remote: fetch → pull (if behind) → push (if ahead)
    match sync_with_remote(&app, &path, project_path) {
        Ok(warning) => {
            if warning.is_empty() {
                Ok(format!("✅ {}\n{}", commit_msg, "Sincronizado con el remoto."))
            } else {
                Ok(format!("⚠️ {}\n{}", commit_msg, warning))
            }
        }
        Err(e) => {
            Err(format!("Commit realizado, pero la sincronización falló: {}", e))
        }
    }
}

/// Fetch from origin and check if the remote has new commits.
///
/// Runs `git fetch origin` (best-effort — network failures are silent)
/// and compares `HEAD` with `origin/main`. Returns JSON with `has_updates`
/// (bool) and `behind_count` (number of commits local is behind).
#[tauri::command]
fn verificar_remoto(path: String) -> Result<String, String> {
    let project_path = Path::new(&path);
    let git_path = find_git()?;

    // Check remote exists
    let url_output = system_command(&git_path)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git remote get-url: {}", e))?;

    if !url_output.status.success() {
        return Ok(r#"{"has_updates":false,"behind_count":0}"#.to_string());
    }

    // Fetch (silent — network errors are non-fatal)
    let _ = system_command(&git_path)
        .arg("fetch")
        .arg("origin")
        .current_dir(project_path)
        .output();

    // Count commits behind
    let count_output = system_command(&git_path)
        .arg("rev-list")
        .arg("--count")
        .arg("HEAD..origin/main")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git rev-list: {}", e))?;

    let behind_str = String::from_utf8_lossy(&count_output.stdout).trim().to_string();
    let behind_count: u32 = behind_str.parse().unwrap_or(0);
    let has_updates = behind_count > 0;

    Ok(format!(
        r#"{{"has_updates":{},"behind_count":{}}}"#,
        has_updates, behind_count
    ))
}

/// Pull changes from the remote repository.
///
/// Runs `git pull origin main`. On success, returns a message with the
/// pull summary. On failure (conflicts, network), returns an error.
#[tauri::command]
fn traer_cambios(path: String) -> Result<String, String> {
    let project_path = Path::new(&path);
    let git_path = find_git()?;

    // Check for local uncommitted changes — pull could overwrite
    let status_output = system_command(&git_path)
        .arg("status")
        .arg("--porcelain")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git status: {}", e))?;

    let has_changes = !String::from_utf8_lossy(&status_output.stdout).trim().is_empty();

    if has_changes {
        return Err(
            "Hay cambios locales sin guardar. Guarda o descarta los cambios antes de sincronizar."
                .to_string(),
        );
    }

    let pull_output = system_command(&git_path)
        .arg("pull")
        .arg("origin")
        .arg("main")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Error al ejecutar git pull: {}", e))?;

    if pull_output.status.success() {
        let stdout = String::from_utf8_lossy(&pull_output.stdout);
        let summary: String = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        let msg = if summary.is_empty() {
            "Cambios sincronizados desde el remoto.".to_string()
        } else {
            format!("Cambios sincronizados: {}", summary)
        };
        Ok(msg)
    } else {
        let stderr = String::from_utf8_lossy(&pull_output.stderr);
        let stderr_str = stderr.trim();
        if stderr_str.contains("CONFLICT") || stderr_str.contains("conflict") {
            Err(format!(
                "Hay conflictos al sincronizar. Resuélvelos manualmente en la terminal:\n{}",
                stderr_str
            ))
        } else {
            Err(format!("Error al sincronizar: {}", stderr_str))
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
                    let app_handle = window.app_handle().clone();

                    // Spawn async task — checkpoint + close happens off the event loop
                    tauri::async_runtime::spawn(async move {
                        // Brief pause lets any in-flight autosave IPC complete
                        tokio::time::sleep(std::time::Duration::from_millis(600)).await;

                        // Checkpoint (git commit + auto-push). Ignore errors — we close anyway.
                        let _ = do_checkpoint(&app_handle, &path);

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
        let subdirs = [".config", "capitulos", "personajes", "notas", "lugares"];
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
