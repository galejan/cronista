# project-file-management Specification

## Purpose

Defines the backend file system operations for Cronista literary projects. Covers project folder scaffolding, chapter file persistence (Nivel 1 — disk save only), and metadata index reading. All operations run as Tauri Rust commands with structured error handling.

## Requirements

### Requirement: Project Folder Creation

The system MUST create a complete project directory structure when `crear_proyecto` is invoked.

Given a root path and project name, the system SHALL:
- Create subdirectories: `.config/`, `capitulos/`, `personajes/`, `notas/`
- Write `.config/metadata.json` with seed schema: `{ project_name, last_modified (ISO 8601), chapters_order: [], characters_index: [] }`
- Write `.config/timeline.json` as empty array `[]`
- Return success with the project path on completion
- Return `Err(String)` for any I/O failure (permission denied, disk full, invalid path)

#### Scenario: Creates project with valid name

- GIVEN a writable directory `/tmp/test-project`
- WHEN `crear_proyecto("/tmp/test-project", "Mi Novela")` is called
- THEN all four subdirectories exist under `/tmp/test-project/`
- AND `metadata.json` contains `"project_name": "Mi Novela"` with a valid ISO 8601 `last_modified`
- AND `timeline.json` contains `[]`

#### Scenario: Rejects inaccessible path

- GIVEN a path `/root/blocked` where the process lacks write permission
- WHEN `crear_proyecto("/root/blocked", "Test")` is called
- THEN the function returns `Err(String)` describing the permission error
- AND no partial directory structure is left behind

#### Scenario: Handles path with trailing separator

- GIVEN path `/tmp/test-project/` with trailing `/`
- WHEN `crear_proyecto("/tmp/test-project/", "Test")` is called
- THEN the function normalises the path and creates the project correctly

### Requirement: Chapter File Save

The system MUST persist chapter content to `.md` files with UTF-8 encoding. Save is Nivel 1 only — no git commit is triggered.

The frontend SHALL invoke this command with a 2-second debounce after the last keystroke. The backend SHALL write the full content string to disk, overwriting any existing file at the same path.

#### Scenario: Saves new chapter file

- GIVEN an existing project at `/tmp/proj` with `capitulos/` directory
- WHEN `guardar_capitulo("/tmp/proj", "0001_prologo.md", "# Prólogo\n\nEra una noche...")` is called
- THEN `capitulos/0001_prologo.md` exists with UTF-8 content `# Prólogo\n\nEra una noche...`
- AND no git commit is created in the project repository

#### Scenario: Overwrites existing chapter

- GIVEN `capitulos/0001_prologo.md` already exists with old content
- WHEN `guardar_capitulo` is called with new content for the same filename
- THEN the file contains the new content, fully replacing the old

#### Scenario: Handles Unicode and special characters

- GIVEN a project with `capitulos/` directory
- WHEN content includes ñ, áéíóú, emoji, and right-to-left characters
- THEN the file is written and read back with identical content (UTF-8 round-trip)

### Requirement: Index Reading

The system MUST read and return `metadata.json` content as a string. Parsing and validation are the caller's responsibility.

Error cases SHALL return `Err(String)`: missing file, unreadable file, or invalid path.

#### Scenario: Reads valid metadata.json

- GIVEN `metadata.json` exists with valid JSON content
- WHEN `cargar_indice("/tmp/proj")` is called
- THEN the function returns `Ok(json_string)` containing the exact file content

#### Scenario: Returns error for missing index

- GIVEN a project path where `.config/metadata.json` does not exist
- WHEN `cargar_indice` is called
- THEN the function returns `Err(String)` indicating file not found

#### Scenario: Returns error for malformed path

- GIVEN an empty string as project path
- WHEN `cargar_indice("")` is called
- THEN the function returns `Err(String)` indicating invalid path
