# Tasks: Editor Integration (TipTap + Debounce Save)

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~380–420 (6 files: 2 mod, 3 new, 1 mod) |
| 400-line budget risk | Medium |
| Chained PRs recommended | Yes |
| Suggested split | Unit 1 → Unit 2 → Unit 3 |
| Delivery strategy | auto-chain |
| Chain strategy | stacked-to-main |
| Exception | Commits directly to main (no PRs); work units as reviewable commits |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: Medium

### Suggested Work Units

| Unit | Goal | Lines | Depends on |
|------|------|-------|------------|
| 1 | Backend commands + wrappers + debounce utility | ~220 | — |
| 2 | Editor.svelte component + ProseMirror styles | ~110 | Unit 1 (Rust commands exist) |
| 3 | +page.svelte integration wiring | ~60 | Unit 2 (Editor component exists) |

## Phase 1: Foundation

- [x] 1.1 Create `src/lib/debounce.ts` — `debounce(fn, ms)` returns `{ trigger, cancel }`; 2s timer, resets on each call, cancel clears timeout
- [x] 1.2 Add `.ProseMirror` typography styles to `src/app.css`: serif defaults, heading scales, line-height 1.8, max-width 65ch centered

## Phase 2: Backend — Rust Commands

- [x] 2.1 Implement `cargar_capitulo` in `src-tauri/src/lib.rs` — reads `{path}/capitulos/{filename}`, returns `Ok(String)` or `Err(String)` for missing file/empty path
- [x] 2.2 Implement `crear_capitulo` in `src-tauri/src/lib.rs` — rejects duplicates, writes `.md` first, then reads/appends to `metadata.json` `chapters_order` + updates `last_modified`
- [x] 2.3 Register `cargar_capitulo` + `crear_capitulo` in `invoke_handler![]` macro
- [x] 2.4 Add `#[cfg(test)]` tests for `cargar_capitulo`: reads existing, error on missing, error on empty path (3 tests)
- [x] 2.5 Add `#[cfg(test)]` tests for `crear_capitulo`: creates file + updates metadata, rejects duplicate, handles Unicode (3 tests)
- [x] 2.6 Run `cargo test` — confirm all 21 existing tests + new tests pass, no regressions

## Phase 3: TypeScript Wrappers

- [x] 3.1 Add `cargarCapitulo(proyectoPath, filename): Promise<string>` to `src/lib/tauri.ts`
- [x] 3.2 Add `crearCapitulo(proyectoPath, filename, contenido): Promise<string>` to `src/lib/tauri.ts`

## Phase 4: Editor Component

- [x] 4.1 Create `src/lib/components/Editor.svelte` — TipTap `onMount` init with StarterKit (heading only), `onDestroy` cleanup, `export let content`, `onUpdate` callback prop
- [x] 4.2 Add TipTap `BubbleMenu` extension to Editor.svelte: h1/h2/h3 toggles + font-family selector (serif/sans-serif/monospace); no bold/italic/underline/links
- [x] 4.3 Wire `onUpdate` → resets external debounce; expose `setContent(content)` method via `editor.commands.setContent()`

## Phase 5: Integration — +page.svelte

- [x] 5.1 Add `$state` runes: `projectPath`, `chapters`, `activeChapter`, `editorContent`; import Editor + debounce + wrappers
- [x] 5.2 Replace `<p class="placeholder">` with `<Editor>` — bind content, onUpdate→debounce→guardarCapitulo; debounce cancelled on chapter switch via `debounce.cancel()`
- [x] 5.3 Wire chapter create flow: `crearCapitulo()` → set `activeChapter` → load via `cargarCapitulo()` → `editor.setContent()`
- [x] 5.4 Wire chapter load: sidebar click → `cargarCapitulo()` → cancel pending debounce → `editor.setContent()`
- [x] 5.5 Manual end-to-end test: create chapter, type content, wait 2s, reload app, verify content persisted
