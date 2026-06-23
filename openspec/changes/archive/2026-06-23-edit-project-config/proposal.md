# Proposal: Edit Project Configuration

## Intent

Cronista users cannot modify font family, Git identity, or remote URL after project creation — these are locked to the initial setup flow. This change adds a "Project Settings" dialog accessible from the toolbar so users can edit all three settings on an open project.

## Scope

### In Scope
- New Rust command `actualizar_fuente_proyecto` to update `font_family` in `metadata.json`
- "Project Settings" dialog with three panels: Font picker, Git identity, Remote URL
- Toolbar button (Phosphor `Gear` icon) opening the dialog, visible when a project is loaded
- Reuse existing `guardar_identidad_git`, `configurar_remoto`, and `guardar_config_remoto` commands

### Out of Scope
- Changing project name/path or project structure
- Adding new identity fields beyond name, email, and GitHub username

## Capabilities

### New Capabilities
- `project-settings-dialog`: UI dialog for editing font family, Git identity, and remote URL on an open project

### Modified Capabilities
- `project-file-management`: Add `actualizar_fuente_proyecto` command — updates `font_family` in `metadata.json`
- `git-identity-config`: Extend identity editing from creation-only to include post-creation modifications via the settings dialog

## Approach

**Backend**: One new Rust command `actualizar_fuente_proyecto(path, font_family)` reads `metadata.json`, updates `font_family`, writes back. Follows existing read-modify-write pattern (same as `crear_capitulo`). No new data structures.

**Frontend**: New `ProjectSettingsDialog.svelte` component. Three sections: Font (reuses existing font picker radio UI), Identity (name/email/githubUser fields, pre-filled from `cargarIdentidadGit`), Remote (URL input, calls `configurar_remoto` on save). Dialog reuses existing modal CSS patterns from `GitIdentityDialog`.

**Toolbar**: `Gear` icon button next to the help `?` button in the editor toolbar, shown only when `projectPath` is set.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/lib.rs` | New command | `actualizar_fuente_proyecto` + handler registration |
| `src/lib/tauri.ts` | New binding | `actualizarFuenteProyecto` invoke wrapper |
| `src/lib/components/` | New file | `ProjectSettingsDialog.svelte` |
| `src/routes/+page.svelte` | Modified | Toolbar button, dialog state, wire callbacks |
| `src/lib/i18n.svelte.ts` | Modified | New i18n keys for settings dialog labels |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `metadata.json` corruption on concurrent write | Low | Single-writer desktop app; atomic read-then-write in Rust |
| Remote URL change fails with repo-not-found or history conflict | Low | Existing `configurar_remoto` already handles REPO_NOT_FOUND and REMOTE_HAS_COMMITS error codes |
| Identity global config race | Low | Single-instance app; Rust read-modify-write protects consistency |

## Rollback Plan

Remove `Gear` button from toolbar, delete `ProjectSettingsDialog.svelte`, remove `actualizar_fuente_proyecto` from command handler. No data migration needed — all persisted commands are existing.

## Dependencies

None. All required backend commands exist except `actualizar_fuente_proyecto`.

## Success Criteria

- [ ] "Project Settings" dialog opens from toolbar when a project is loaded
- [ ] Font family change persists to `metadata.json` and reflects in editor on save
- [ ] Git identity changes persist to global `git-config.json` and apply to next commit
- [ ] Remote URL change updates git remote origin via `configurar_remoto`
- [ ] Dialog closes cleanly (Esc, overlay click, Cancel) without side effects
