# Tasks: Edit Project Configuration

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 350–400 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | auto-forecast |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

## Phase 1: Foundation (Backend + i18n)

- [x] 1.1 Add `actualizar_fuente_proyecto` Rust command in `src-tauri/src/lib.rs`
  — Accepts `project_path: String`, `font_family: String`
  — Validates font ∈ {monospace, serif, sans-serif}
  — Reads `{path}/.config/metadata.json`, updates `font_family` + `last_modified` (ISO 8601), atomic write back
  — Returns `Ok("")` / `Err(String)`. Preserves all other fields (`project_name`, `chapters_order`, `characters_index`)
  — Register in `invoke_handler` array (alpha-sorted after `actualizar_personaje`)

- [x] 1.2 Add `actualizarFuenteProyecto` invoke binding in `src/lib/tauri.ts`
  — `export async function actualizarFuenteProyecto(projectPath: string, fontFamily: string): Promise<string>` — wraps `invoke("actualizar_fuente_proyecto", { projectPath, fontFamily })`

- [x] 1.3 Add `settings.*` i18n keys in `src/lib/i18n.svelte.ts`
  — ES block: `settings.title` ("Configuración del proyecto"), `settings.font`, `settings.identity`, `settings.remote` (tab labels), `settings.save`, `settings.cancel`, `settings.fontPreview` ("Vista previa"), `settings.urlLabel` ("URL del remoto"), `settings.saving` ("Guardando…"), `settings.saved` ("Guardado")
  — EN block: matching English strings ("Project Settings", "Font", "Identity", "Remote", "Save", "Cancel", "Preview", "Remote URL", "Saving…", "Saved")

## Phase 2: Core Component

- [x] 2.1 Create `src/lib/components/ProjectSettingsDialog.svelte` — modal shell with tab-based panels
  — Props: `open: $bindable(false)`, `projectPath: string`, `fontFamily: string`
  — Three tab buttons (Font | Identity | Remote) with active state, only one panel visible at a time
  — Esc key closes dialog (`onkeydown`), overlay click closes (`onclick` on backdrop), Cancel button in footer
  — Reuse existing `.modal-overlay` / `.modal-content` CSS patterns from `GitIdentityDialog.svelte`

- [x] 2.2 Implement FontPanel inside `ProjectSettingsDialog.svelte`
  — Radio group: monospace / serif / sans-serif (pre-selected from `fontFamily` prop)
  — Live preview paragraph in selected font
  — Local `$state` tracking selected font (buffered; only committed on Save)
  — Save: call `actualizarFuenteProyecto(projectPath, selected)` → update parent `fontFamily` via callback/new prop
  — Per-panel save lifecycle: idle → saving (spinner, button disabled) → success/error message

- [x] 2.3 Implement IdentityPanel inside `ProjectSettingsDialog.svelte`
  — On panel open: call `cargarIdentidadGit()` → pre-fill name, email, githubUser `$state` fields
  — Fallback to language-aware presets (ES: "Miguel de Cervantes" / EN: "William Shakespeare") when no config
  — Save: validate name + email non-empty → call `guardarIdentidadGit(name, email, githubUser)`
  — Show validation error for empty name/email; disable Save during in-flight call

- [x] 2.4 Implement RemotePanel inside `ProjectSettingsDialog.svelte`
  — On panel open: call `cargarConfigRemoto()` → pre-fill `url` `$state`
  — URL input field with client-side validation (must not be empty, must look like a git URL)
  — Save: validate URL → call `configurarRemoto(projectPath, url)` → on success call `guardarConfigRemoto(url, true)`
  — Show error on invalid URL (before backend call) and on backend failure

## Phase 3: Wiring

- [x] 3.1 Wire dialog in `src/routes/+page.svelte`
  — Import `ProjectSettingsDialog` + `Gear` icon from `@phosphor-icons/svelte`
  — Add `settingsDialogOpen = $state(false)`
  — Add `<button>` with `Gear` icon in `.editor-toolbar` div, after the help `?` button, guarded by `{#if projectPath}`
  — Add `<ProjectSettingsDialog>` with `bind:open={settingsDialogOpen}`, `{projectPath}`, `{fontFamily}`, callback to update `fontFamily` on Font save

## Phase 4: Testing

- [x] 4.1 Write Rust `#[cfg(test)]` unit tests for `actualizar_fuente_proyecto` in `src-tauri/src/lib.rs`
  — Test: update font from monospace → serif, verify metadata.json reflects change + preserved fields
  — Test: reject invalid font ("comic-sans") → returns Err, file unchanged
  — Test: missing metadata.json → returns Err indicating file not found
  — Test: corrupt JSON → returns Err indicating parse failure, no file written
  — Test: empty project path → returns Err indicating invalid path
  — Use `TempDir` for isolated filesystem; follow existing `#[cfg(test)] mod tests` pattern
