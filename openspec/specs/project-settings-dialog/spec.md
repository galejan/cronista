# project-settings-dialog Specification

## Purpose

Defines the "Project Settings" dialog accessible from the editor toolbar when a project is loaded. Covers font family, Git identity, and remote URL editing with per-panel save semantics, state management, and clean dismissal.

## Requirements

### Requirement: Dialog Activation

The system MUST display a Gear icon button in the editor toolbar when `projectPath` is set. Clicking it SHALL open the Project Settings dialog. The button MUST NOT be visible when no project is loaded.

#### Scenario: Open dialog from toolbar

- GIVEN a project is loaded and editor is visible
- WHEN the user clicks the Gear icon in the toolbar
- THEN the Project Settings dialog opens with its first panel displayed

#### Scenario: Gear hidden without project

- GIVEN no project is loaded
- WHEN the editor toolbar is rendered
- THEN the Gear icon button is not visible

### Requirement: Panel Navigation

The dialog MUST present three panels selectable via tabs or segmented control: Font, Git Identity, Remote. Only one panel SHALL be visible at a time. The dialog title MUST read "Project Settings".

#### Scenario: Navigate between panels

- GIVEN the dialog is open on the Font panel
- WHEN the user clicks "Remote" tab
- THEN the Remote panel is displayed and Font panel is hidden

### Requirement: Font Panel

The Font panel MUST offer a font-family selector with options: monospace, serif, sans-serif. It SHALL show a live preview of the selected font. On save, it MUST invoke `actualizarFuenteProyecto(path, fontFamily)`. The panel MUST pre-select the current font from `metadata.json` on open.

#### Scenario: Change and save font

- GIVEN the dialog is open on the Font panel with current font "monospace"
- WHEN the user selects "serif" and clicks Save
- THEN `actualizarFuenteProyecto(projectPath, "serif")` is invoked
- AND the editor font updates to "serif"

#### Scenario: Cancel reverts font selection

- GIVEN the Font panel shows current font "monospace" and user selects "sans-serif" without saving
- WHEN the user clicks Cancel or closes the dialog
- THEN the font selection reverts to "monospace"
- AND `actualizarFuenteProyecto` is not called

### Requirement: Identity Panel

The Identity panel MUST pre-fill name, email, and GitHub username from `cargarIdentidadGit` on panel open. It SHALL invoke `guardar_identidad_git(name, email, githubUser)` on save. The GitHub username field MAY be left empty.

#### Scenario: Edit and save identity

- GIVEN identity fields are pre-filled with stored values
- WHEN the user edits the name and clicks Save
- THEN `guardar_identidad_git` is called with the new values
- AND a success confirmation is shown

#### Scenario: Save with empty name or email

- GIVEN the user clears the name field
- WHEN the user clicks Save
- THEN the dialog MUST show a validation error
- AND `guardar_identidad_git` is not called

### Requirement: Remote Panel

The Remote panel MUST display a URL input and a Save button. On save, it SHALL invoke `configurar_remoto(path, url)` followed by `guardar_config_remoto(url, true)`. It SHALL validate URL format client-side before invoking backend commands.

#### Scenario: Set valid remote URL

- GIVEN the Remote panel is open with a project that has no remote set
- WHEN the user enters `git@github.com:user/repo.git` and clicks Save
- THEN `configurar_remoto` is invoked with the URL
- AND `guardar_config_remoto` persists the URL on success

#### Scenario: Reject invalid URL format

- GIVEN the Remote panel is open
- WHEN the user enters `not-a-valid-url` and clicks Save
- THEN a validation error is shown before any backend call
- AND neither `configurar_remoto` nor `guardar_config_remoto` is invoked

### Requirement: Save State Management

Each panel MUST manage its own save lifecycle independently: idle, saving, success, error. During save, the Save button SHALL be disabled and show a loading indicator. Multiple rapid saves on the same panel MUST be serialized — the second save SHALL wait for the first to complete before executing.

#### Scenario: Save with loading state

- GIVEN the Identity panel is open
- WHEN the user clicks Save and the backend call is in progress
- THEN the Save button is disabled and shows a spinner
- AND clicking Save again has no effect

#### Scenario: Save error display

- GIVEN the Font panel is open
- WHEN the user clicks Save and `actualizarFuenteProyecto` returns an error
- THEN the panel displays the error message
- AND the dialog remains open for the user to retry or cancel

### Requirement: Dialog Dismissal

The dialog MUST close on Esc key, overlay click, or Cancel button. Closing SHALL revert all unsaved panel changes. Closing MUST NOT trigger any backend commands.

#### Scenario: Close via Esc

- GIVEN the dialog is open with unsaved changes
- WHEN the user presses Esc
- THEN the dialog closes and no backend command is invoked

#### Scenario: Close via overlay click

- GIVEN the dialog is open
- WHEN the user clicks the overlay area outside the dialog
- THEN the dialog closes without side effects
