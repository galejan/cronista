# Design: Configurable Tabs and Auto-Save

## Technical Approach

Two-pronged: (1) Backend schema extension with `VisibleTabs` sub-struct and `auto_save_interval_minutes` on `Metadata`, serde-default backward compat, plus atomic read-merge-validate-write command. (2) Frontend unification: single `ProjectConfigForm` component replacing inline wizard steps and `ProjectSettingsDialog` font panel. Sidebar tab rendering gates on `metadata.visible_tabs` (Chapters always renders). Auto-save timer becomes dynamic via `$effect` recreation on interval change.

## Architecture Decisions

| Decision | Options | Choice | Rationale |
|----------|---------|--------|-----------|
| Chapters validation | Reject write vs silently force | **Reject with error** | Preserves user intent transparency; `actualizar_config_proyecto` returns `Err` before touching disk so metadata.json is never corrupted. Spec mandates this. |
| `actualizar_config_proyecto` return type | `Result<String, String>` (empty) vs `Result<Metadata, String>` | **`Result<String, String>` with JSON-serialized Metadata** | Follows existing convention (`actualizar_fuente_proyecto` returns `Ok("".to_string())`). Frontend parses JSON to update state without re-fetching. |
| Atomic write | Direct `write` vs `write(temp) + rename` | **Direct `write`** (existing pattern) | Whole-file rewrite is already the project convention (`actualizar_fuente_proyecto`). OS guarantees atomic write for small JSON files. No partial write risk worth extra complexity. |
| Frontend type for Metadata | Inline `any` vs shared TypeScript interface | **Inline `any` (no new type)** | Frontend already parses metadata as `any` everywhere (`const meta = JSON.parse(raw)`). Adding a shared interface is scope creep — this change is about behavior, not typing. |
| Wizard steps in edit mode | Same 4 steps vs skip project info | **Skip step 1** (project name/folder) | `mode="edit"` hides the Info Básica step; only Tabs, Auto-Save, Review shown. Project name is immutable post-creation. |
| `ProjectSettingsDialog` changes | Replace font panel vs add new tabs | **Add two new tabs** (Visible Tabs, Auto-Save) | Keep `ProjectSettingsDialog` for identity/remote/git. Add tabs for new config panels that reuse `ProjectConfigForm`-internal sub-components, avoiding full dialog rewrite. Provides gradual migration path. |

## Data Flow

```
User clicks Settings → ProjectSettingsDialog (edit mode)
  → actualizar_config_proyecto(projectPath, partial_config)
    → Rust: read metadata.json → merge → validate → write → return full JSON
    → Frontend: parse response → update $state variables (fontFamily, visibleTabs, saveInterval)
      → $effect: recreate debounce with new interval
      → $effect: tab visibility reactively updates sidebar

New project wizard → ProjectConfigForm (new mode, step 1→4)
  → onComplete callback → crearProyecto(path, name, config)
    → Rust: creates directory + seeds metadata with config overrides
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/lib.rs` | Modify | Add `VisibleTabs` struct (+ serde defaults), extend `Metadata` with `visible_tabs` and `auto_save_interval_minutes`, add `actualizar_config_proyecto` command, register in `generate_handler!`, update `crear_proyecto`/`create_project_for_test` to seed new fields |
| `src/lib/ProjectConfigForm.svelte` | Create | Wizard component: step state machine, tab checkboxes, interval radio group, review step. Emits `onComplete(config)`. Exports sub-components for `ProjectSettingsDialog` reuse. |
| `src/routes/+page.svelte` | Modify | Import `actualizarConfigProyecto`, add `visibleTabs`/`autoSaveInterval` state, wrap tab buttons in `{#if}`, make Chapters unconditional, replace hardcoded 20s debounce with dynamic `$effect`, wire wizard flow to `ProjectConfigForm` |
| `src/lib/components/ProjectSettingsDialog.svelte` | Modify | Add "Visible Tabs" and "Auto-Save" tabs, embed `ProjectConfigForm` sub-components, wire save button to `actualizar_config_proyecto` |
| `src/lib/tauri.ts` | Modify | Add `actualizarConfigProyecto` IPC binding |
| `src/lib/i18n.svelte.ts` | Modify | Add labels: `config.tabsLabel`, `config.intervalLabel`, `config.interval1`, `config.interval5`, `config.interval10` |

## Interfaces

```typescript
// ProjectConfigForm.svelte props
interface ProjectConfigFormProps {
  mode: "new" | "edit";
  initialData?: {
    font_family?: string;
    visible_tabs?: { chapters: boolean; characters: boolean; places: boolean; timeline: boolean; notes: boolean };
    auto_save_interval_minutes?: number;
  };
  onComplete: (config: ProjectConfig) => void;
  onCancel?: () => void;
}

interface ProjectConfig {
  font_family: string;
  visible_tabs: { chapters: boolean; characters: boolean; places: boolean; timeline: boolean; notes: boolean };
  auto_save_interval_minutes: number; // 1 | 5 | 10
}
```

```rust
// New Tauri command
#[tauri::command]
fn actualizar_config_proyecto(project_path: String, config: serde_json::Value) -> Result<String, String>
```

## Testing Strategy

| Layer | What | How |
|-------|------|-----|
| Rust unit | `VisibleTabs` serde defaults | Deserialize JSON missing keys → verify all `true` |
| Rust unit | `actualizar_config_proyecto` merge | Create project, call with partial config, verify merged result |
| Rust unit | Validation: chapters=false rejected | Call with `{visible_tabs: {chapters: false}}` → expect `Err` |
| Rust unit | Validation: invalid interval rejected | Call with `auto_save_interval_minutes: 3` → expect `Err` |
| Rust unit | `crear_proyecto` seeds new fields | Create project, read metadata.json, verify `visible_tabs` and interval present |
| Manual | Frontend tab visibility | Open project, hide tabs via settings, verify sidebar buttons disappear |
| Manual | Dynamic debounce | Change interval in settings, trigger edit, verify save fires at new interval |

## Migration

No migration required. `#[serde(default)]` on both new fields means old projects load with all tabs visible and 5-minute auto-save. First save through new config form writes both fields, making them explicit.

## Open Questions

None.
