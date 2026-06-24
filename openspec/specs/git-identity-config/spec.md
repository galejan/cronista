# git-identity-config Specification

## Purpose

Manages the user's Git identity (name + email) as a global persistent config, shared across all Cron-Insta projects. Replaces the previous hardcoded identity with a user-facing dialog on project creation, backed by language-aware literary presets.

## Requirements

### Requirement: Global Identity Storage

The system MUST store Git identity in the platform app config directory via Tauri's `app.path().app_config_dir()` at path `cron-insta/git-config.json`.

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

The system SHALL present an identity dialog during project creation AND from the settings toolbar when a project is loaded.

The dialog MUST:
- Display clearly labeled name, email, and GitHub username fields
- Pre-fill fields from global config when available, or fall back to language-aware presets when no config exists
- Allow the user to edit, accept, or skip (during creation); edit or cancel (during post-creation settings)
- NOT block project creation if Git is unavailable
- Reuse the `guardar_identidad_git` command for persistence in both contexts

#### Scenario: User opens dialog with presets (unchanged)

- GIVEN this is the first project and UI language is Spanish
- WHEN the project creation dialog appears
- THEN name field shows "Miguel de Cervantes" and email shows "cervantes@literatura.es"

#### Scenario: User skips identity dialog (unchanged)

- GIVEN the identity dialog is open during project creation
- WHEN the user clicks "Skip" or closes the dialog
- THEN the repo SHALL be initialized without author identity set
- AND the global config SHALL NOT be modified

#### Scenario: Edit identity from settings dialog

- GIVEN a project is loaded and identity exists in global config
- WHEN the user opens Settings → Identity panel
- THEN the name, email, and GitHub username fields are pre-filled from the stored config
- AND the user MAY edit and save via `guardar_identidad_git`

#### Scenario: Identity panel shows presets when no config exists

- GIVEN a project is loaded but no global identity config exists
- WHEN the user opens Settings → Identity panel
- THEN fields are pre-filled with language-aware presets
