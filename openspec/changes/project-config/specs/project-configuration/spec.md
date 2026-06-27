# Delta for project-configuration

## MODIFIED Requirements

### Requirement: Project Folder Creation

The system MUST create a complete project directory structure when `crear_proyecto` is invoked.

Given a root path and project name, the system SHALL:
- Create subdirectories: `.config/`, `capitulos/`, `personajes/`, `notas/`, `lugares/`
- Write `.config/metadata.json` with seed schema including `visible_tabs: { chapters: true, characters: true, places: true, timeline: true, notes: true }` and `auto_save_interval_minutes: 5`
- Accept optional `visible_tabs` and `auto_save_interval_minutes` overrides
- Write `.config/timeline.json` as empty array `[]`
- Write `lugares/index.json` as empty array `[]`
- Return success with the project path on completion
- Return `Err(String)` for any I/O failure
(Previously: metadata seed lacked `visible_tabs` and `auto_save_interval_minutes`; no optional overrides)

#### Scenario: Creates project with new config fields

- GIVEN a writable directory `/tmp/test-project`
- WHEN `crear_proyecto` is called with optional `visible_tabs` and interval
- THEN `metadata.json` contains `"visible_tabs": {"chapters":true,"characters":true,"places":true,"timeline":true,"notes":true}`
- AND `"auto_save_interval_minutes": 5`
- AND all existing seed fields remain present

#### Scenario: Rejects invalid auto-save interval

- GIVEN a writable directory
- WHEN `auto_save_interval_minutes: 7` (not in enum 1|5|10) is passed
- THEN the function returns `Err(String)` indicating invalid interval
- AND no project is created

## ADDED Requirements

### Requirement: Visible Tabs Field

The `Metadata` struct MUST include a `visible_tabs` field of type object with boolean keys: `chapters`, `characters`, `places`, `timeline`, `notes`. The `chapters` key MUST default to `true` and MUST NOT be set to `false`. All other keys SHALL default to `true`. Serde deserialization SHALL use `#[serde(default)]` for backward compatibility.

#### Scenario: Chapters always true

- GIVEN a valid metadata update request
- WHEN `visible_tabs.chapters` is `false`
- THEN the backend MUST reject the save with an error
- AND `metadata.json` is not modified

#### Scenario: All tabs visible by default

- GIVEN old `metadata.json` without `visible_tabs`
- WHEN `cargar_indice` deserializes it
- THEN all five `visible_tabs` keys evaluate to `true`

### Requirement: Auto-Save Interval Field

The `Metadata` struct MUST include an `auto_save_interval_minutes` field of type integer, constrained to enum values `1`, `5`, or `10`. The default SHALL be `5`. Deserialization SHALL use `#[serde(default)]`.

#### Scenario: Default interval on missing field

- GIVEN old `metadata.json` without `auto_save_interval_minutes`
- WHEN deserialized
- THEN `auto_save_interval_minutes` evaluates to `5`

#### Scenario: Reject invalid interval values

- GIVEN a request to save metadata with `auto_save_interval_minutes: 3`
- WHEN the backend validates the payload
- THEN it returns `Err` — only 1, 5, 10 are valid

### Requirement: Bulk Config Update Command

The system MUST provide an `actualizar_config_proyecto` Tauri command that atomically merges partial config into `metadata.json`. It SHALL accept `project_path: String` and `config: Value` (partial JSON). It SHALL read current metadata, merge fields, validate constraints (`chapters` must be `true`, interval in `[1,5,10]`), write atomically, and return `Ok(())` or `Err(String)`.

#### Scenario: Merge partial config

- GIVEN project with `font_family: "monospace"` and `visible_tabs.characters: true`
- WHEN `actualizar_config_proyecto` receives `{ "visible_tabs": { "characters": false } }`
- THEN `font_family` remains `"monospace"` (untouched)
- AND `visible_tabs.characters` becomes `false`

#### Scenario: Atomic write on failure

- GIVEN `actualizar_config_proyecto` receives invalid interval
- WHEN the command validates
- THEN it returns `Err(String)` before writing
- AND `metadata.json` is unchanged
