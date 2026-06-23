# Delta for git-identity-config

## MODIFIED Requirements

### Requirement: Unified Identity Dialog

The system SHALL present an identity dialog during project creation AND from the settings toolbar when a project is loaded.

The dialog MUST:
- Display clearly labeled name, email, and GitHub username fields
- Pre-fill fields from global config when available, or fall back to language-aware presets when no config exists
- Allow the user to edit, accept, or skip (during creation); edit or cancel (during post-creation settings)
- NOT block project creation if Git is unavailable
- Reuse the `guardar_identidad_git` command for persistence in both contexts

(Previously: Dialog was scoped to project creation only; no post-creation editing.)

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
