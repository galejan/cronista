# Tasks: Configurable Tabs and Auto-Save

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 550–650 |
| 800-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | auto-forecast |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: Low

## Phase 1: Backend Schema & Validation

- [x] 1.1 Add `VisibleTabs` struct in `src-tauri/src/lib.rs` with `serde(default)` on all 5 bool fields (chapters, characters, places, timeline, notes)
- [x] 1.2 Extend `Metadata` struct with `visible_tabs: VisibleTabs` and `auto_save_interval_minutes: u32`, both `#[serde(default)]` with `default_visible_tabs()`/`default_auto_save()` helpers
- [x] 1.3 Add validation helpers: `validate_visible_tabs()` (rejects chapters=false) and `validate_auto_save_interval()` (rejects values outside [1,5,10])
- [x] 1.4 Write Rust tests: serde defaults (missing keys → all true/5), Chapters rejection, invalid interval rejection

## Phase 2: Backend Command & Project Seeding

- [x] 2.1 Implement `actualizar_config_proyecto` command: read metadata.json → merge partial `serde_json::Value` → validate → write → return full merged JSON
- [x] 2.2 Update `crear_proyecto` signature to accept optional `visible_tabs` and `auto_save_interval_minutes` params; seed into new `Metadata`
- [x] 2.3 Update `create_project_for_test` to match `crear_proyecto` changes
- [x] 2.4 Register `actualizar_config_proyecto` in `generate_handler!` macro
- [x] 2.5 Write Rust tests: merge partial config preserves untouched fields, atomic rejection (invalid payload → disk unchanged), seeding new fields on creation

## Phase 3: Frontend Foundation

- [x] 3.1 Add `actualizarConfigProyecto(projectPath, config)` to `src/lib/tauri.ts`
- [x] 3.2 Add 8+ i18n keys to `src/lib/i18n.svelte.ts`: `config.tabsLabel`, `config.intervalLabel`, `config.interval1`, `config.interval5`, `config.interval10` (es + en)
- [x] 3.3 Create `src/lib/ProjectConfigForm.svelte`: dual-mode wizard (`new`/`edit`), 4 steps (Font→Tabs→Auto-Save→Review in new mode, Tabs→Auto-Save→Review in edit), emits `onComplete(config)`
- [x] 3.4 Chapters checkbox in form always `checked` and `disabled`; interval radio group (1/5/10, default 5); review step shows summary before confirm

## Phase 4: Integration & Wiring

- [x] 4.1 In `+page.svelte`: import `actualizarConfigProyecto`; add `visibleTabs`/`autoSaveInterval` `$state`; parse from metadata on load
- [x] 4.2 In `+page.svelte`: wrap Characters/Places/Timeline tab buttons in `{#if visibleTabs[key]}`; Chapters always renders unconditionally
- [x] 4.3 In `+page.svelte`: replace hardcoded `debounce(doSave, 20_000)` with dynamic `$effect` that recreates debounce at `autoSaveInterval * 60_000` ms on interval change
- [x] 4.4 In `+page.svelte`: replace inline `pickFont()` call in new-project flow with `ProjectConfigForm` embed; pass config override to `crearProyecto`
- [x] 4.5 In `+page.svelte`: update Ctrl+T tab cycle to skip hidden tabs; Ctrl+L shortcut does not exist in codebase (skipped)
- [x] 4.6 In `ProjectSettingsDialog.svelte`: add "Visible Tabs" and "Auto-Save" tabs; inline form controls; wire save to `actualizarConfigProyecto`

## Phase 5: Verification

- [x] 5.1 Run `cargo test --manifest-path src-tauri/Cargo.toml` — all 115 backend tests pass
- [x] 5.2 Run `pnpm check` — Svelte compilation succeeds (0 errors, 7 warnings)
- [ ] 5.3 Manual: old project opens with all tabs visible and 5-min auto-save (backward compat)
- [ ] 5.4 Manual: hide Characters via settings → sidebar button disappears, panel hidden, Ctrl+T skips it
- [ ] 5.5 Manual: Chapters checkbox disabled in form; saving `chapters: false` shows backend error
- [ ] 5.6 Manual: change interval to 1 min → timer recreates; edit triggers save at new interval
