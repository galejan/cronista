## Exploration: project-config

### Current State

Cronista is a single-file Svelte 5 frontend (`src/routes/+page.svelte`, ~5154 lines) with a Tauri v2 Rust backend (`src-tauri/src/lib.rs`, ~5670 lines single module). There is NO TypeScript type for the project configuration — metadata is parsed ad-hoc from raw JSON strings.

#### 1. metadata.json Schema

**Defined**: `src-tauri/src/lib.rs` line 88 — `struct Metadata` (Rust, serde)

| Field | Type | Default | Line |
|---|---|---|---|
| `project_name` | `String` | — | 89 |
| `last_modified` | `String` (ISO 8601) | — | 90 |
| `chapters_order` | `Vec<String>` | `[]` | 91 |
| `characters_index` | `Vec<CharacterIndex>` (id, file, name) | `[]` | 92-93 |
| `places_index` | `Vec<LugarIndexItem>` (id, name) | `[]` | 94-95 |
| `font_family` | `String` | `"monospace"` | 96-97 |
| `push_enabled` | `bool` | `false` | 98-99 |
| `consecutive_failures` | `u32` | `0` | 100-102 |

**Serialization**: serde `to_string_pretty` / `from_str`. No migrations, no versioning.
**Frontend parsing**: Ad-hoc `JSON.parse(raw)` extracting `chapters_order`, `font_family`, `project_name`. No shared TypeScript type.
**No `visible_tabs` or `auto_save_interval_minutes` field exists yet.**

**Read**: `cargar_indice` (line 889) returns raw JSON string.
**Write locations**: `crear_proyecto` (449), `crear_capitulo` (978), `eliminar_capitulo` (1028), `actualizar_fuente_proyecto` (1204), `guardar_config_remoto` (2725), push state in `do_push` block (2065-2147).

#### 2. New Project Wizard

**No separate wizard component** — it is inline in `src/routes/+page.svelte`, function `crearCapituloNuevo()` at line 771.

**Sequential modal flow** (each is a separate custom dialog, not a unified form):
1. **Directory picker** (line 775): Native `open()` dialog
2. **Git check** (line 783): `detectarGit()`, optionally show identity dialog
3. **Name prompt** (line 794): `pickText()` — custom Svelte modal at line 3112
4. **Font picker** (line 798): `pickFont()` — custom Svelte modal at line 3195
5. **Git identity** (line 808): `showIdentityDialog()` — `GitIdentityDialog` component
6. **Create project** (line 818): `crearProyecto(path, name, fontFamily)` — Rust command
7. **First chapter** (line 868): `pickText()` for filename, then `crearCapitulo()`

**Font picker modal** (line 3195-3256): Three radio options with live preview. Returns `"monospace"`, `"serif"`, or `"sans-serif"`.

**Text picker modal** (line 3112-3143): Generic text input with Enter/Button/Close binding.

**No "Cancel wizard" concept** — canceling any step aborts the entire flow. No back/forward navigation.

#### 3. Existing Project Config Dialog

**File**: `src/lib/components/ProjectSettingsDialog.svelte` (784 lines)

**Three tabs**: Font, Identity, Remote (Tab type at line 22: `"font" | "identity" | "remote"`)

**What it does NOT handle**:
- Visible tabs toggle
- Auto-save interval
- Project name change

**Opened via**: `settingsOpen = true` from Gear button in footer (line 2569).

**Font panel**: Radio group for monospace/serif/sans-serif. Calls `actualizarFuenteProyecto(path, fontFamily)` on save. Reads current value via `currentFontFamily` prop.

**Identity panel**: Loads from `cargarIdentidadGit()`, saves via `guardarIdentidadGit(name, email, githubUser)`.

**Remote panel**: URL input + validation. Calls `configurarRemoto(path, url)` then `guardarConfigRemoto(path, url, true)`.

**Lifecycle**: Full state reset on open (`$effect` at line 48). Each panel manages its own save lifecycle independently.

**Dialog site in page.svelte**: Lines 2976-2981, receives `currentFontFamily={fontFamily}`, `onFontSaved` callback.

#### 4. Auto-Save Timer

**Hardcoded location**: `src/routes/+page.svelte` line 491:
```ts
const save = debounce(doSave, 20_000);
```

**Debounce utility**: `src/lib/debounce.ts` (35 lines). Resettable timer: `trigger()` resets the countdown, `cancel()` clears it without execution.

**Trigger point**: `handleEditorUpdate()` at line 496 — fires on every editor content change:
```ts
function handleEditorUpdate(html: string): void {
  editorContent = html;
  saveStatus = "unsaved";
  save.trigger();
}
```

**Save function** (`doSave` at line 451): Saves chapter content via `guardarCapitulo()` or note content via `crearNota()`. Updates `saveStatus` to `"saving"` → `"saved"` or `"unsaved"` on error.

**Save states**: `"" | "saved" | "unsaved" | "saving"` (line 321).

**Manual save button** (line 2604): Calls `doSave()` directly without debounce.

**Help text** (i18n line 587): "Text is auto-saved every 20 seconds..."

**Checkpoint timer** (separate, unused skeleton): `src/lib/checkpoint.ts` — placeholder for git checkpoint on inactivity, NOT the auto-save.

#### 5. Sidebar Tabs

**Tab type**: `"capitulos" | "personajes" | "notas" | "timeline" | "lugares"` (line 411).

**Five tab buttons** (lines 1971-2006): Each is a `<button class="tab">` with Phosphor icon:
- Chapters: `Notebook` icon (line 1972)
- Characters: `User` icon (line 1978)
- Notes: `Notepad` icon (line 1984)
- Timeline: `Clock` icon + badge count (line 1990)
- Places: `MapTrifold` icon (line 2000)

**Tab panels**: Five `{#if activeTab === "xxx"}` blocks starting at line 2010, each in a `<div class="tab-panel">`.

**Current behavior**: All 5 tabs ALWAYS visible. No mechanism to hide/show individual tabs. Chapters is the default tab but NOT marked as mandatory in code — it's just what's set initially and what the app flow opens to.

**Data refresh**: All 5 tab datasources refresh in an `$effect` at line 1683 when `projectPath` changes.

**No visibility toggle or conditional rendering exists.**

**Icons**: `Notebook`, `User`, `Notepad`, `Clock` (with badge), `MapTrifold` — all from `phosphor-svelte/lib/*`.

**Tab state persists only in memory** (`activeTab` rune). Not persisted across sessions.

#### 6. Tauri Commands for Metadata

| Command | Line | Purpose |
|---|---|---|
| `crear_proyecto` | 429 | Creates project structure + writes initial `metadata.json` |
| `cargar_indice` | 889 | Returns raw `metadata.json` as JSON string |
| `actualizar_fuente_proyecto` | 1177 | Updates `font_family` field in `metadata.json` |
| `guardar_config_remoto` | 2708 | Updates `push_enabled` + `consecutive_failures` |
| `cargar_config_remoto` | 2644 | Reads push state + remote URL |
| `crear_capitulo` | 930 | Appends filename to `chapters_order` in metadata |
| `eliminar_capitulo` | 1000 | Removes filename from `chapters_order` |

**No command currently exists for**: updating `visible_tabs`, `auto_save_interval`, or bulk metadata rewrite.

**`read_metadata` helper** (line 2540): Internal function used by export/import commands. Returns parsed `Metadata` struct.

All commands registered at lines 3192-3241 in `invoke_handler`.

### Affected Areas

- **`src-tauri/src/lib.rs`** — Add fields to `Metadata` struct (line 88); add `#[serde(default)]` for backward compat; new Tauri command(s) for updating metadata; modify `crear_proyecto` to seed new fields; register new command
- **`src/routes/+page.svelte`** — Replace hardcoded `20_000` with interval from metadata (line 491); add `visible_tabs` filtering to tab render (lines 1971-2006); create unified config dialog (new component or extend existing); modify wizard flow (line 771) to reuse unified form
- **`src/lib/components/ProjectSettingsDialog.svelte`** — Extend with visible tabs panel + auto-save interval panel (or split into new shared component)
- **`src/lib/tauri.ts`** — New TypeScript binding(s) for metadata update commands
- **`src/lib/i18n.svelte.ts`** — New i18n keys for visible tabs labels, auto-save labels, and config form
- **`src/lib/debounce.ts`** — May need modification if interval changes dynamically (currently hardcoded at creation)
- **`src/lib/checkpoint.ts`** — Not affected (separate timer, unused skeleton)
- **`openspec/specs/project-file-management/spec.md`** (line 15-16) — Update schema documentation
- **`openspec/specs/project-settings-dialog/spec.md`** — Add new panel specs

### Approaches

#### 1. Extend ProjectSettingsDialog (reuse existing component)

Add two new tabs to the existing dialog: "Visible Tabs" and "Auto-Save". Keep the separate `pickFont`/`pickText` wizard modals for creation.

- **Pros**: Least new files; reuses existing modal CSS and patterns; minimal refactor; `onFontSaved` callback already wired
- **Cons**: Dialog grows to 5+ tabs; wizard and reconfigure UI remain different (two sources of truth); `ProjectSettingsDialog` only works when project is loaded — wizard needs separate form
- **Effort**: Medium

#### 2. Extract shared `ProjectConfigForm.svelte`, use in both wizard and dialog

Create a single `ProjectConfigForm.svelte` component that both the new-project wizard flow AND the existing ProjectSettingsDialog embed. The form handles: font, visible tabs, auto-save interval.

- **Pros**: True "unified config form" as requested; single source of truth for config UI; wizard becomes a single step (the form) instead of sequential modals; future config additions go in one place
- **Cons**: More refactor — wizard is currently 4 sequential modals, needs to become a single form; `pickText`/`pickFont` modal patterns are deeply embedded; project name prompt must be separate (not a metadata-configurable field)
- **Effort**: Medium-High

#### 3. Add new Rust command for bulk metadata update, client-side tab filtering

Add `actualizar_config_proyecto(path, config_json)` command that accepts a partial config object. Frontend filters tabs client-side based on `visible_tabs` from metadata. Auto-save interval is read on project open and passed to `debounce()`.

- **Pros**: Single backend endpoint for all config changes; backward compatible via `#[serde(default)]`; clean separation
- **Cons**: Client-side tab filtering means tabs still render and are hidden via CSS/conditional; need to handle dynamic debounce interval (new `debounce.ts` instance per interval change)
- **Effort**: Medium

### Recommendation

**Approach 2 (shared config form)** is the closest to the stated requirement of "Same form for new project wizard AND reconfiguring an existing project." However, **Approach 3** for the backend (single `actualizar_config_proyecto` command) is the pragmatic backend choice — one read-modify-write command accepting a partial JSON is cleaner than multiple field-specific commands.

**Hybrid recommendation**: 
- Backend: Add two new fields to `Metadata` struct with `#[serde(default)]`. Add one new command `actualizar_config_proyecto` that takes a partial config object and does read-modify-write. Modify `crear_proyecto` to accept optional config fields.
- Frontend: Extract `ProjectConfigForm.svelte` — a single form with: project name (disabled for existing projects), font picker, visible tabs checkboxes (Chapters disabled/checked always), auto-save interval radio. Embed it in BOTH the wizard flow AND a new tab in `ProjectSettingsDialog`.
- Wizard: Replace 4 sequential modals with a single `ProjectConfigForm` modal. Keep directory picker and git identity as separate steps.
- Auto-save: Store interval in a reactive variable initialized from metadata on project open. Re-create debounce instance when interval changes.
- Tabs: Wrap tab buttons in `{#if visible_tabs.xxx}` conditionals, with `chapters` always `true`.

### Risks

- **Metadata backward compatibility**: Old projects without `visible_tabs`/`auto_save_interval_minutes` must work. Mitigated by `#[serde(default)]` on both new fields — defaults keep current behavior (all tabs visible, 5-minute auto-save).
- **Dynamic debounce instance**: The current `debounce()` is created once as a module-level const. If the interval can change, we need to re-create it on the fly. The debounce utility supports this trivially via a reactive `$effect`.
- **Wizard refactor scope creep**: Converting 4 sequential modals to a single form changes the UX significantly. The text picker modal is also used for other purposes (character name, place name, etc.) — must not break those.
- **Chapters tab mandatory constraint**: The UI must disable the checkbox for Chapters (always checked). This is a UI constraint, not a data constraint — `visible_tabs.chapters` should always be `true` but validation on save ensures it.
- **Auto-save interval values**: Currently "20 seconds". New values are 1, 5, or 10 minutes. This is a UX change from "every few seconds of typing" to "minutes". Must update help text.
- **No frontend tests**: Only Rust tests exist (84 `#[cfg(test)]` in `lib.rs`). Manual verification required for all UI changes.

### Ready for Proposal

Yes — all areas mapped, data structures clear, no blocking unknowns.
