# Proposal: Configurable Tabs and Auto-Save

## Intent

Users cannot hide unused sidebar tabs or change the hardcoded 20-second auto-save. The new-project wizard and settings dialog use different UIs for overlapping config, creating inconsistency. This change makes tabs optional, auto-save configurable, and unifies all project configuration into one form used in both creation and editing.

## Scope

### In Scope
- Two new `metadata.json` fields: `visible_tabs` and `auto_save_interval_minutes`
- New `actualizar_config_proyecto` backend command (bulk read-modify-write)
- `#[serde(default)]` backward compat for old projects
- `ProjectConfigForm.svelte` — single form with `"new"`/`"edit"` modes
- Conditional sidebar tab rendering based on `visible_tabs`; Chapters always visible
- Dynamic auto-save timer that responds to runtime interval changes
- `crear_proyecto` extended to accept optional config fields

### Out of Scope
- UI theming, dark mode, or layout changes beyond tab visibility
- Checkpoint timer (separate, unused)
- Persisting active-tab state across sessions

## Capabilities

### Modified Capabilities
- `project-file-management`: Schema gains two fields. New `actualizar_config_proyecto` command.
- `project-settings-dialog`: Extends or replaced by unified form with Visible Tabs and Auto-Save panels.
- `user-interface`: Tab buttons become conditionally rendered. Chapters mandatory.

## Approach

1. **Backend**: Add `visible_tabs: VisibleTabs { chapters, characters, notes, timeline, places }` and `auto_save_interval_minutes: u32` to `Metadata`, with serde defaults (all true, 5). New command merges partial JSON into existing metadata atomically. Validation rejects `chapters: false`.
2. **Form**: `ProjectConfigForm.svelte` with wizard-style steps: Font → Visible Tabs → Auto-Save. Chapters checkbox disabled (always true). Interval: radio group (1/5/10 min). Embed in wizard (after directory picker) and `ProjectSettingsDialog`.
3. **Tabs**: Wrap each button in `{#if visibleTabs[key]}`. Chapters button renders unconditionally.
4. **Timer**: Store interval as `$state`. `$effect` clears and recreates `debounce()` on change.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/lib.rs` | Modified | Metadata struct, new command, registration |
| `src/routes/+page.svelte` | Modified | Timer, tab filtering, wizard refactor |
| `src/lib/components/ProjectConfigForm.svelte` | New | Unified config form |
| `src/lib/components/ProjectSettingsDialog.svelte` | Modified | Embed form |
| `src/lib/tauri.ts` | Modified | New IPC binding |
| `src/lib/i18n.svelte.ts` | Modified | Labels for tabs toggle, interval |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Old metadata crashes on load | Low | `#[serde(default)]` — all tabs visible, 5 min default |
| Breaking pickText/pickFont used elsewhere | Medium | Only wizard flow refactored; standalone modals untouched |
| Chapters accidentally hidden via data | Low | Backend rejects `chapters: false` on save |
| No frontend tests | High | Manual verification; Rust tests cover new command |

## Rollback Plan

Git revert. New metadata fields are backward-compatible via serde defaults — no data migration to undo.

## Dependencies

None.

## Success Criteria

- [ ] New-project wizard uses `ProjectConfigForm` for font, tabs, and interval
- [ ] Settings dialog offers same tabs-toggle and interval controls
- [ ] Pre-change project opens with all tabs visible, 5-min auto-save
- [ ] Hiding Characters/Places/Timeline removes their buttons and panels
- [ ] Chapters always visible, toggle disabled in form
- [ ] Changing interval clears old timer, starts new one immediately
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes
