# Design: Edit Project Configuration

## Technical Approach

One new Rust command (`actualizar_fuente_proyecto`) performs atomic read-modify-write on `metadata.json`. A new Svelte 5 component (`ProjectSettingsDialog`) provides tab-based Font/Identity/Remote panels. The dialog opens from a Gear icon in the editor toolbar (visible only when `projectPath` is set). Identity and Remote panels reuse existing `guardar_identidad_git` and `configurar_remoto` commands. All CSS follows existing modal patterns from `GitIdentityDialog` and `+page.svelte`.

## Architecture Decisions

| Decision | Option A | Option B | Choice | Rationale |
|----------|----------|----------|--------|-----------|
| Panel switching | Tabs (independent) | Step-based wizard (like GitIdentityDialog) | **Tabs** | Settings editing requires random access, not linear flow. Each panel is self-contained with its own Save. |
| Identity editing | Build into ProjectSettingsDialog | Reuse GitIdentityDialog (creation-only) | **Build into new dialog** | GitIdentityDialog is creation-oriented (wizard flow, auto-fill remote). Settings needs tab-based independence and pre-filled values without wizard steps. |
| Font persistence | New command `actualizar_fuente_proyecto` | Reuse `crear_capitulo` pattern inline in existing command | **New command** | Clean separation; `metadata.json` update is standalone, not tied to chapter creation. Follows `crear_proyecto` Metadata type. |
| Remote editing | Reuse `configurar_remoto` | New command wrapping git remote | **Reuse** | Existing command handles remote add/set-url, push, REPO_NOT_FOUND, and REMOTE_HAS_COMMITS. No need for new logic. |
| State management | Per-panel independent state (name/email/font/url) | Centralized dialog store | **Per-panel state** | Panels have different save backends; coupling them adds complexity with no benefit. Each panel tracks its own saving/error/success. |

## Data Flow

```
User clicks Gear → open=true
     │
     ├─ loadIdentity() → cargarIdentidadGit() → pre-fill name/email
     ├─ loadRemote()   → cargarConfigRemoto() → pre-fill URL
     └─ fontFamily prop (already in +page.svelte state from cargarIndice)
     
User edits Font panel → selects serif → clicks Save
     │
     ├─ setSaving("font", true)
     ├─ actualizarFuenteProyecto(projectPath, "serif")
     │       └─ Rust: read metadata.json → update font_family + last_modified → atomic write
     ├─ setSuccess("font") / setError("font", msg)
     └─ +page.svelte updates fontFamily $state → Editor re-renders

User clicks Cancel / Esc / overlay
     │
     └─ open=false → all unsaved changes discarded (no backend calls)
```

## Component Tree

```
+page.svelte
 ├─ Gear button (editor-toolbar, next to ? button)
 └─ ProjectSettingsDialog.svelte (modal overlay + panel)
      ├─ tabs: [Font | Identity | Remote]
      ├─ FontPanel (radio picker + live preview, reuses .font-picker CSS)
      ├─ IdentityPanel (name/email/githubUser fields)
      └─ RemotePanel (URL input + validate + save)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/lib.rs` | Modify | Add `actualizar_fuente_proyecto` command + register in handler |
| `src/lib/tauri.ts` | Modify | Add `actualizarFuenteProyecto` invoke binding |
| `src/lib/components/ProjectSettingsDialog.svelte` | **Create** | Tab-based settings dialog with FontPanel, IdentityPanel, RemotePanel |
| `src/routes/+page.svelte` | Modify | Add Gear icon, dialog state (`settingsDialogOpen`), import Gear + new component, wire callbacks |
| `src/lib/i18n.svelte.ts` | Modify | Add `settings.*` i18n keys (title, tabs, labels, save/cancel) |

## Interfaces / Contracts

```rust
// New command (src-tauri/src/lib.rs)
#[tauri::command]
fn actualizar_fuente_proyecto(project_path: String, font_family: String) -> Result<String, String>;
// Validates font_family ∈ {monospace, serif, sans-serif}
// Read-modify-write metadata.json
// Returns Ok("") on success
```

```typescript
// New binding (src/lib/tauri.ts)
export async function actualizarFuenteProyecto(
  projectPath: string, fontFamily: string
): Promise<string>;
```

```typescript
// Component props (ProjectSettingsDialog.svelte, Svelte 5)
let { open = $bindable(false), projectPath = "" } = $props();
// open: boolean, two-way bound for modal visibility
// projectPath: required for backend calls, passed to all save handlers
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit (Rust) | `actualizar_fuente_proyecto` — valid/invalid font, missing file, corrupt JSON | `#[cfg(test)]` with TempDir, assert metadata.json content |
| Unit (Rust) | `actualizar_fuente_proyecto` — preserves other fields | Verify `chapters_order`, `characters_index` unchanged after update |
| Manual (UI) | Tab switching, font preview update, dirty state reset on Cancel | Visual check in Tauri dev build |
| Manual (UI) | Identity save, Remote URL validation + REPO_NOT_FOUND handling | Visual check in Tauri dev build |

## Migration / Rollout

No migration required. New command is additive; dialog only appears on explicit user action. Rollback: remove Gear button and dialog, unregister command from `invoke_handler`.

## Open Questions

None — all commands exist except `actualizar_fuente_proyecto`, which follows a well-established pattern (`crear_capitulo` read-modify-write).
