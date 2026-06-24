# Delta for project-file-management

## MODIFIED Requirements

### Requirement: Project Folder Creation

The system MUST create a complete project directory structure when `crear_proyecto` is invoked.

Given a root path and project name, the system SHALL:
- Create subdirectories: `.config/`, `capitulos/`, `personajes/`, `notas/`, `lugares/`
- Write `.config/metadata.json` with seed schema: `{ project_name, last_modified (ISO 8601), chapters_order: [], characters_index: [], places_index: [] }`
- Write `.config/timeline.json` as empty array `[]`
- Write `lugares/index.json` as empty array `[]`
- Return success with the project path on completion
- Return `Err(String)` for any I/O failure (permission denied, disk full, invalid path)  
(Previously: 4 subdirectories without `lugares/`; metadata lacked `places_index`)

#### Scenario: Creates project with lugares directory

- GIVEN a writable directory `/tmp/test-project`
- WHEN `crear_proyecto("/tmp/test-project", "Mi Novela")` is called
- THEN all five subdirectories exist including `lugares/`
- AND `lugares/index.json` contains `[]`
- AND `metadata.json` contains `"places_index": []`

#### Scenario: Rejects inaccessible path

- GIVEN a path `/root/blocked` where the process lacks write permission
- WHEN `crear_proyecto("/root/blocked", "Test")` is called
- THEN the function returns `Err(String)` describing the permission error
- AND no partial directory structure is left behind

## ADDED Requirements

### Requirement: Place CRUD Operations

The system MUST provide five Tauri commands for place management: `listar_lugares`, `crear_lugar`, `cargar_lugar`, `actualizar_lugar`, `eliminar_lugar`.

Each place SHALL be stored as a JSON file `{id}.json` inside the project's `lugares/` directory. The place model MUST contain `id: String`, `name: String`, and `description: String`. The index file `lugares/index.json` SHALL maintain the ordered list of place IDs.

#### Scenario: List places from index

- GIVEN `lugares/index.json` contains `["torre-norte", "plaza-central"]`
- WHEN `listar_lugares("/tmp/proj")` is called
- THEN it returns `Ok(Vec<Place>)` with both places loaded from their individual JSON files

#### Scenario: Create a new place

- GIVEN a project with empty `lugares/`
- WHEN `crear_lugar("/tmp/proj", "torre-norte", "Torre Norte", "Alta torre de piedra")` is called
- THEN `lugares/torre-norte.json` is created with the place data
- AND `lugares/index.json` includes `"torre-norte"`

#### Scenario: Load a single place

- GIVEN `lugares/torre-norte.json` exists
- WHEN `cargar_lugar("/tmp/proj", "torre-norte")` is called
- THEN it returns `Ok(Place { id: "torre-norte", name: "Torre Norte", description: "..." })`

#### Scenario: Update place description

- GIVEN `lugares/torre-norte.json` exists
- WHEN `actualizar_lugar("/tmp/proj", "torre-norte", "Torre Norte", "Nueva descripción")` is called
- THEN the file is overwritten with the new data
- AND the index remains unchanged

#### Scenario: Delete a place

- GIVEN `lugares/torre-norte.json` and its index entry exist
- WHEN `eliminar_lugar("/tmp/proj", "torre-norte")` is called
- THEN the JSON file is removed
- AND the ID is removed from `lugares/index.json`

#### Scenario: Error on duplicate place ID

- GIVEN `lugares/torre-norte.json` already exists
- WHEN `crear_lugar` is called with the same ID
- THEN it returns `Err(String)` and no files are overwritten

#### Scenario: Handle Unicode in place content

- GIVEN place name and description contain ñ, áéíóú, emojis
- WHEN the place is created and loaded back
- THEN content is preserved identically (UTF-8 round-trip)

### Requirement: Place Export/Import

The project export and import operations MUST include the `lugares/` directory automatically. No explicit configuration SHALL be required to include places in exports.

#### Scenario: Export includes lugares directory

- GIVEN a project with `lugares/` containing place files
- WHEN the project is exported
- THEN the export archive includes the entire `lugares/` directory tree

#### Scenario: Import restores lugares

- GIVEN an export archive containing `lugares/`
- WHEN the project is imported
- THEN `lugares/` is restored with all place files intact
