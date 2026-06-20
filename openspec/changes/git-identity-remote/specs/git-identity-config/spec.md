# git-identity-config Specification

## Purpose

Manages the user's Git identity (name + email) as a global persistent config, shared across all Cronista projects. Replaces the previous hardcoded identity with a user-facing dialog on project creation, backed by language-aware literary presets.

## Requirements

### Requirement: Global Identity Storage

The system MUST store Git identity in the platform app config directory via Tauri's `app.path().app_config_dir()` at path `cronista/git-config.json`.

The config file SHALL include a `schema_version` field for forward compatibility.

#### Scenario: First project — no config exists
- GIVEN no `git-config.json` exists in the app config dir
- WHEN a new project is created
- THEN the identity dialog SHALL pre-fill with a language-aware preset:
  - Spanish UI → name: "Miguel de Cervantes", email: "cervantes@literatura.es"
  - English UI → name: "William Shakespeare", email: "shakespeare@literature.en"
- AND the user MAY accept or customize these values

#### Scenario: Subsequent project — config exists
- GIVEN `git-config.json` exists with `{"name":"Ada Lovelace","email":"ada@code.dev"}`
- WHEN a new project is created
- THEN the identity dialog SHALL pre-fill with the stored name and email

#### Scenario: User customizes identity
- GIVEN the identity dialog is shown with any pre-filled values
- WHEN the user modifies name and/or email and confirms
- THEN the new values MUST be saved to `git-config.json`
- AND `git config user.name` and `git config user.email` SHALL be set in the project repo

#### Scenario: User accepts defaults unchanged
- GIVEN the identity dialog is shown with pre-filled values (preset or prior config)
- WHEN the user confirms without changes
- THEN the values MUST be saved to `git-config.json` (may be identical to prior config)
- AND the repo identity SHALL be configured accordingly

#### Scenario: Config file corrupted
- GIVEN `git-config.json` exists but contains invalid JSON
- WHEN the system attempts to load identity
- THEN the system MUST fall back to language-aware presets
- AND no crash or unhandled error SHALL occur
- AND the user SHALL be notified that the config could not be read

### Requirement: Unified Identity Dialog

The system SHALL present a single dialog during project creation with name and email inputs pre-filled from global config or presets.

The dialog MUST:
- Display clearly labeled name and email fields
- Pre-fill fields as described in "Global Identity Storage"
- Allow the user to edit, accept, or skip (identity-only, no remote)
- NOT block project creation if Git is unavailable

#### Scenario: User opens dialog with presets
- GIVEN this is the first project and UI language is Spanish
- WHEN the project creation dialog appears
- THEN name field shows "Miguel de Cervantes" and email shows "cervantes@literatura.es"

#### Scenario: User skips identity dialog
- GIVEN the identity dialog is open
- WHEN the user clicks "Skip" or closes the dialog
- THEN the repo SHALL be initialized without author identity set
- AND the global config SHALL NOT be modified
