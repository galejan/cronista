# Delta for project-file-management

## ADDED Requirements

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
