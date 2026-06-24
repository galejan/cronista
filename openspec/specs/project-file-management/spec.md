# project-file-management Specification

## Purpose

Defines the backend file system operations for Cron-Insta literary projects. Covers project folder scaffolding, chapter file persistence (Nivel 1 — disk save only), chapter file reading, chapter creation with metadata registration, and metadata index reading. All operations run as Tauri Rust commands with structured error handling.

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

### Requisito: Lectura de Archivo de Capítulo

El sistema DEBE leer el contenido de un único archivo .md de capítulo y retornarlo como string UTF-8. El comando `cargar_capitulo` recibe la ruta del proyecto y el nombre del archivo. El parseo y validación del contenido son responsabilidad del frontend.

#### Escenario: Lee un capítulo existente

- DADO un proyecto en `/tmp/proj` con `capitulos/0001_prologo.md` que contiene "# Prólogo\n\nEra una noche..."
- CUANDO `cargar_capitulo("/tmp/proj", "0001_prologo.md")` es invocado
- ENTONCES retorna `Ok("# Prólogo\n\nEra una noche...")`
- Y el contenido preserva codificación UTF-8

#### Escenario: Retorna error para archivo inexistente

- DADO un proyecto donde `capitulos/9999_fantasma.md` no existe
- CUANDO `cargar_capitulo("/tmp/proj", "9999_fantasma.md")` es invocado
- ENTONCES retorna `Err(String)` indicando que el archivo no fue encontrado

#### Escenario: Retorna error para ruta inválida

- DADO un proyecto_path vacío
- CUANDO `cargar_capitulo("", "test.md")` es invocado
- ENTONCES retorna `Err(String)` indicando ruta inválida

### Requisito: Creación de Capítulo con Registro en Metadatos

El sistema DEBE crear un nuevo archivo .md de capítulo y registrar su entrada en `metadata.json` dentro de `chapters_order`. El comando `crear_capitulo` recibe ruta del proyecto, nombre del archivo y contenido inicial. La operación DEBE escribir primero el archivo y luego actualizar los metadatos para minimizar corrupción por crash.

#### Escenario: Crea capítulo y actualiza metadatos

- DADO un proyecto en `/tmp/proj` con `metadata.json` que tiene `chapters_order: ["0001_prologo.md"]`
- CUANDO `crear_capitulo("/tmp/proj", "0002_capitulo_1.md", "# Capítulo 1\n\n")` es invocado
- ENTONCES `capitulos/0002_capitulo_1.md` existe con el contenido provisto
- Y `metadata.json` contiene `chapters_order: ["0001_prologo.md", "0002_capitulo_1.md"]`
- Y `last_modified` se actualiza a la fecha/hora actual en ISO 8601

#### Escenario: Retorna error para capítulo duplicado

- DADO un proyecto donde "0001_prologo.md" ya existe en `capitulos/`
- CUANDO `crear_capitulo` es invocado con el mismo nombre de archivo
- ENTONCES retorna `Err(String)` indicando que el capítulo ya existe
- Y `metadata.json` no se modifica

#### Escenario: Maneja contenido Unicode

- DADO un proyecto con directorio `capitulos/`
- CUANDO el contenido incluye ñ, áéíóú, emojis y caracteres CJK
- ENTONCES el archivo se crea con codificación UTF-8 correcta
- Y la lectura posterior retorna contenido idéntico

### Requirement: Font Family Update

The system MUST provide an `actualizar_fuente_proyecto` Tauri command that atomically updates `font_family` in a project's `metadata.json`.

The command SHALL:
- Accept `project_path: String` and `font_family: String` parameters
- Read `metadata.json` from `{project_path}/.config/metadata.json`
- Validate `font_family` is one of: `"monospace"`, `"serif"`, `"sans-serif"`
- Update `font_family` and `last_modified` (ISO 8601) in-memory
- Write the modified JSON back to disk as a single atomic write
- Return `Ok("")` on success
- Return `Err(String)` for any failure

The frontend SHALL invoke this command only when the user explicitly saves from the settings dialog.

#### Scenario: Update font family in metadata

- GIVEN a project at `/tmp/proj` with `metadata.json` containing `"font_family": "monospace"`
- WHEN `actualizar_fuente_proyecto("/tmp/proj", "serif")` is called
- THEN `metadata.json` contains `"font_family": "serif"`
- AND `last_modified` is updated to the current date/time in ISO 8601
- AND all other fields (`project_name`, `chapters_order`, `characters_index`) remain unchanged

#### Scenario: Reject invalid font family

- GIVEN any valid project at `/tmp/proj`
- WHEN `actualizar_fuente_proyecto("/tmp/proj", "comic-sans")` is called
- THEN the function returns `Err(String)` indicating invalid font family
- AND `metadata.json` is not modified

#### Scenario: Handle missing metadata.json

- GIVEN a project at `/tmp/proj` where `.config/metadata.json` does not exist
- WHEN `actualizar_fuente_proyecto("/tmp/proj", "monospace")` is called
- THEN the function returns `Err(String)` indicating file not found

#### Scenario: Handle corrupted metadata.json

- GIVEN `metadata.json` exists but contains invalid or malformed JSON
- WHEN `actualizar_fuente_proyecto` is called
- THEN the function returns `Err(String)` indicating the file could not be parsed
- AND no file is written

#### Scenario: Handle invalid path

- GIVEN an empty string as project path
- WHEN `actualizar_fuente_proyecto("", "serif")` is called
- THEN the function returns `Err(String)` indicating invalid path
