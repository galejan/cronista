# Design: Editor Integration (TipTap + Debounce Save)

## Technical Approach

Component-by-component, matching the proposal's build order. Two new Rust commands extend the existing `lib.rs` pattern. A single `Editor.svelte` wraps TipTap imperatively via `onMount`/`onDestroy`. `$state` runes in `+page.svelte` manage chapter identity, content, and dirty flag — no separate store module. Debounce utility cancels on chapter switch to prevent cross-chapter saves.

## Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Rust commands location | `src-tauri/src/lib.rs` (same module) | Existing 5 commands + tests live here. Co-locating keeps cohesion — no file fragmentation for 2 small commands (~50 lines each). |
| State management | `$state` runes in `+page.svelte` | Svelte 5 runes are lightweight. Extracting to `stores/` would be premature for 3-4 reactive variables (active chapter, project path, content, save status). Extract later when sidebar/timeline complexity warrants it. |
| TipTap integration | `<script>` + `onMount`/`onDestroy` | TipTap v3 is framework-agnostic — no Svelte 5 `use:` directive. Imperative `new Editor({...})` in `onMount`, `editor.destroy()` in `onDestroy`. `onUpdate` callback bridges content to debounce. |
| `crear_capitulo` write order | Write `.md` first, then update `metadata.json` | Crash mid-operation → orphan `.md` file is harmless (not in index). Reverse order risks an index entry pointing to a missing file. |
| Debounce mechanism | Standalone `debounce.ts` utility replacing `checkpoint.ts` skeleton | `checkpoint.ts` is a skeleton (Nivel 2, 30min). Debounce is Nivel 1 (2s, no commit). Different concerns — rewrite, don't evolve. Future checkpoint timer can reuse the debounce pattern. |
| Bubble menu CTRL | TipTap `BubbleMenu` extension (h1, h2, h3, font-family) | Out of scope: bold, italic, underline, links. Minimal menu aligns with the literary-writing product vision — headings structure the manuscript, font-family aids readability. |

## Data Flow

```
+page.svelte ($state)                    Rust (lib.rs)
─────────────────────                    ──────────────
activeChapter ──→ cargarCapitulo ──────→ cargar_capitulo ──→ fs::read_to_string
    ↑                    │                       ↑
    │                    │                       │
    │              setContent()            fs::write + JSON merge
    │                    │                       ↑
    │                    ▼                       │
    │            Editor.svelte                    │
    │              (TipTap)                       │
    │                    │                       │
    │              onUpdate                       │
    │                    │                       │
    │                    ▼                       │
    │            debounce(2s) ────→ guardarCapitulo ──→ guardar_capitulo
    │              (cancels on                     │
    │               chapter switch)                │
    │                                              │
    └── crea nuevo capítulo ──→ crearCapitulo ──→ crear_capitulo ──→ write .md
                                                                     → update metadata.json
```

### Chapter Create → Edit → Save Flow

1. User clicks "Nuevo capítulo" → `crearCapitulo(projectPath, filename, initialHTML)`
2. Rust: writes `capitulos/{filename}.md` → appends to `chapters_order` in `metadata.json` → updates `last_modified`
3. Frontend: sets `activeChapter = filename`, loads content into editor via `setContent()`
4. Editor `onUpdate` → resets 2s debounce timer
5. After 2s idle → `guardarCapitulo(projectPath, filename, editor.getHTML())`

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/lib.rs` | Modify | Add `cargar_capitulo`, `crear_capitulo` commands + `#[cfg(test)]` tests |
| `src/lib/tauri.ts` | Modify | Add `cargarCapitulo` and `crearCapitulo` typed wrappers |
| `src/lib/components/Editor.svelte` | Create | TipTap wrapper: `onMount` init, `onDestroy` cleanup, bubble menu (h1/h2/h3 + font-family), exposes `onUpdate` callback |
| `src/lib/debounce.ts` | Create | `debounce(fn, ms)` → `{ cancel: () => void }` — resets timer on call, exposes cancel for unmount/chapter switch |
| `src/routes/+page.svelte` | Modify | Add `$state` vars (projectPath, chapters, activeChapter, editorContent); replace placeholder `<p>` with `<Editor>`; wire load/create/save |
| `src/app.css` | Modify | Add `.ProseMirror` typography styles: `font-family: Georgia, serif` default, heading scales, line-height for readability |

## Contracts / Interfaces

### Rust: `cargar_capitulo`

```rust
#[tauri::command]
fn cargar_capitulo(proyecto_path: String, filename: String) -> Result<String, String>
```
Reads `{proyecto_path}/capitulos/{filename}`. Returns content as `Ok(String)` or `Err(String)` for missing file / empty path / I/O error.

### Rust: `crear_capitulo`

```rust
#[tauri::command]
fn crear_capitulo(proyecto_path: String, filename: String, contenido: String) -> Result<String, String>
```
1. Reject if `capitulos/{filename}` already exists
2. Write chapter `.md` file
3. Read `metadata.json`, append `filename` to `chapters_order`, update `last_modified`, write back
4. Return `Ok(String)` success message or `Err(String)`

### TS: wrappers (in `src/lib/tauri.ts`)

```ts
export async function cargarCapitulo(proyectoPath: string, filename: string): Promise<string>
export async function crearCapitulo(proyectoPath: string, filename: string, contenido: string): Promise<string>
```

### TS: debounce (in `src/lib/debounce.ts`)

```ts
export function debounce<Args extends unknown[]>(
  fn: (...args: Args) => void,
  ms: number,
): { trigger: (...args: Args) => void; cancel: () => void }
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Rust unit (`#[cfg(test)]`) | `cargar_capitulo`: reads existing file, error on missing file, error on empty path. `crear_capitulo`: creates file + metadata update, rejects duplicates, handles Unicode. | New test functions in `mod tests` block, `tempfile::TempDir` isolation. Target: +6 tests. |
| Rust regression | All 21 existing tests must pass after adding new commands | `cargo test` — validate handler registration doesn't break |
| Vitest component | `Editor.svelte`: mounts TipTap, sets initial content, fires `onUpdate` on keystroke, cleans up on destroy. `debounce.ts`: triggers after interval, resets on repeated calls, cancels properly. | Need `vitest` + `@testing-library/svelte` scaffolded (currently absent — may be deferred) |
| Vitest unit | `cargarCapitulo`/`crearCapitulo` wrappers: mock `invoke`, verify correct argument passing | Vitest with `vi.mock("@tauri-apps/api/core")` |

## Open Questions

- [ ] Vitest/Svelte Testing Library not scaffolded — add task to install deps if component tests are in scope? Or defer component tests until test infrastructure change?
- [ ] Font-family selector UX: dropdown with serif/sans-serif presets, or freeform input? Proposal says only headings + font-family; exploration mentions no UI for this yet.
