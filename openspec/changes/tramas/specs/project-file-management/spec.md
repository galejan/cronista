# Delta for project-file-management

## ADDED Requirements

### Requirement: Metadata Trama Fields
The `Metadata` struct SHALL contain `tramas: Vec<Trama>` and `chapter_tramas: Vec<ChapterTrama>`. `Trama` SHALL have fields `id: String` and `nombre: String`. `ChapterTrama` SHALL have fields `filename: String` and `trama_id: Option<String>`. Both `Vec` fields MUST use `#[serde(default)]` for backward compatibility.

#### Scenario: Metadata serializes tramas correctly
- GIVEN a project with one trama and one assigned chapter
- WHEN `metadata.json` is written
- THEN the JSON contains `"tramas": [{"id": "...", "nombre": "..."}]`
- AND `"chapter_tramas": [{"filename": "...", "trama_id": "..."}]`

#### Scenario: Missing fields default to empty
- GIVEN a `metadata.json` serialized without `tramas` or `chapter_tramas`
- WHEN the `Metadata` struct is deserialized
- THEN both fields default to empty `Vec` with no error

### Requirement: Trama CRUD Commands
The system MUST provide `crear_trama` and `eliminar_trama` Tauri commands. `crear_trama` SHALL accept `project_path` and `nombre`, generate a unique `id`, and persist to `metadata.tramas`. `eliminar_trama` SHALL accept `project_path` and `id`, remove the trama, and set all assigned chapters' `trama_id` to `null`.

#### Scenario: crear_trama persists to metadata
- GIVEN a valid project at `/tmp/proj`
- WHEN `crear_trama("/tmp/proj", "Principal")` is called
- THEN `metadata.tramas` contains the new trama
- AND `last_modified` is updated

#### Scenario: eliminar_trama cleans up assignments
- GIVEN trama "A" has 2 chapters assigned
- WHEN `eliminar_trama("/tmp/proj", "id_of_A")` is called
- THEN the trama is removed from `metadata.tramas`
- AND both chapters have `trama_id: null` in `metadata.chapter_tramas`

### Requirement: Chapter-Trama Assignment Command
The system MUST provide `asignar_capitulo_trama` Tauri command accepting `project_path`, `filename`, and `trama_id: Option<String>`. It SHALL update the `chapter_tramas` entry and `last_modified`. When `trama_id` is `None`, the chapter SHALL be unassigned.

#### Scenario: Assign chapter during creation
- GIVEN trama "A" exists
- WHEN `crear_capitulo` is called with `trama_id: Some("id_of_A")`
- THEN the chapter appears in `chapter_tramas` with the given `trama_id`
- AND the chapter appears in `chapters_order` as usual

#### Scenario: Reassign chapter to different trama
- GIVEN chapter "01.md" assigned to trama "A"
- WHEN `asignar_capitulo_trama` is called with `trama_id: Some("id_of_B")`
- THEN the chapter's `trama_id` updates to `id_of_B`
- AND `chapters_order` is unchanged

#### Scenario: Reject assignment to nonexistent trama
- GIVEN chapter "01.md" exists but trama "ghost" does not
- WHEN `asignar_capitulo_trama` is called with `trama_id: Some("ghost")`
- THEN it returns `Err(String)`

### Requirement: SCHEMA.md Trama Documentation
The `generate_schema()` function MUST include a "Tramas" entity section describing `Trama { id, nombre }` and `ChapterTrama { filename, trama_id }`. It SHALL document that chapters remain flat in `capitulos/` regardless of trama assignment.

#### Scenario: SCHEMA.md includes tramas entity
- GIVEN a project with tramas enabled
- WHEN `generate_schema()` runs
- THEN the output includes a "Tramas" entity section
- AND it describes the chapter-trama relationship

## MODIFIED Requirements

### Requirement: Project Folder Creation

The system MUST create a complete project directory structure when `crear_proyecto` is invoked.

Given a root path and project name, the system SHALL:
- Create subdirectories: `.config/`, `capitulos/`, `personajes/`, `notas/`, `lugares/`
- Write `.config/metadata.json` with seed schema: `{ project_name, last_modified (ISO 8601), chapters_order: [], characters_index: [], places_index: [], tramas: [], chapter_tramas: [] }`
- Write `.config/timeline.json` as empty array `[]`
- Write `.config/stats.json` with seed schema: `{ total_time_seconds: 0, total_words: 0, chapters: {}, sessions: [] }`
- Write `lugares/index.json` as empty array `[]`
- Return success with the project path on completion
- Return `Err(String)` for any I/O failure (permission denied, disk full, invalid path)
(Previously: Seed schema did not include `tramas` or `chapter_tramas` fields)

#### Scenario: Creates project with tramas seed
- GIVEN a writable directory `/tmp/test-project`
- WHEN `crear_proyecto("/tmp/test-project", "Mi Novela")` is called
- THEN `metadata.json` contains `"tramas": []` and `"chapter_tramas": []`
- AND all five subdirectories exist including `lugares/`
- AND `stats.json` and `timeline.json` contain their seed schemas

#### Scenario: Rejects inaccessible path
- GIVEN a path `/root/blocked` where the process lacks write permission
- WHEN `crear_proyecto("/root/blocked", "Test")` is called
- THEN the function returns `Err(String)` describing the permission error
- AND no partial directory structure is left behind

#### Scenario: Handles path with trailing separator
- GIVEN path `/tmp/test-project/` with trailing `/`
- WHEN `crear_proyecto("/tmp/test-project/", "Test")` is called
- THEN the function normalises the path and creates the project correctly

### Requisito: Creación de Capítulo con Registro en Metadatos

El sistema DEBE crear un nuevo archivo .md de capítulo y registrar su entrada en `metadata.json` dentro de `chapters_order`. El comando `crear_capitulo` recibe ruta del proyecto, nombre del archivo, contenido inicial y un `trama_id` opcional (`Option<String>`). Si se proporciona `trama_id`, el sistema DEBE registrar también la asignación en `chapter_tramas`. La operación DEBE escribir primero el archivo y luego actualizar los metadatos para minimizar corrupción por crash.

#### Escenario: Crea capítulo y actualiza metadatos con trama
- DADO un proyecto en `/tmp/proj` con `chapters_order: ["0001_prologo.md"]` y trama "A" existente
- CUANDO `crear_capitulo("/tmp/proj", "0002_capitulo_1.md", "# Capítulo 1\n\n", Some("id_of_A"))` es invocado
- ENTONCES `capitulos/0002_capitulo_1.md` existe con el contenido provisto
- Y `chapters_order` contiene `["0001_prologo.md", "0002_capitulo_1.md"]`
- Y `chapter_tramas` contiene `{ filename: "0002_capitulo_1.md", trama_id: "id_of_A" }`
- Y `last_modified` se actualiza a la fecha/hora actual en ISO 8601

#### Escenario: Crea capítulo sin trama
- DADO un proyecto en `/tmp/proj`
- CUANDO `crear_capitulo("/tmp/proj", "0003_suelto.md", "...", None)` es invocado
- ENTONCES `chapters_order` contiene "0003_suelto.md"
- Y `chapter_tramas` contiene `{ filename: "0003_suelto.md", trama_id: null }`

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
