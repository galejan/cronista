# Tasks: Tramas (Plotlines)

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~183 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | auto-chain |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

## Phase 1: Backend — Structs, Commands, Schema

- [x] 1.1 Add `Trama {id, nombre}` and `ChapterTrama {filename, trama_id: Option<String>}` structs with serde derives in `src-tauri/src/lib.rs`
- [x] 1.2 Add `tramas` and `chapter_tramas` Vec fields to `Metadata` with `#[serde(default)]`
- [x] 1.3 Implement `crear_trama(path, nombre)`: slugify name + random 8-char hex suffix, reject duplicate names, persist to metadata
- [x] 1.4 Implement `eliminar_trama(path, id)`: remove from vec, nullify assigned chapter_tramas entries, persist
- [x] 1.5 Implement `asignar_capitulo_trama(path, filename, trama_id)`: upsert entry, validate trama exists, persist
- [x] 1.6 Modify `crear_capitulo` to accept `trama_id: Option<String>`, register assignment in `chapter_tramas`
- [x] 1.7 Seed `tramas: [], chapter_tramas: []` in `crear_proyecto` and update `generate_schema()` with Trama entity docs
- [x] 1.8 Register new commands in `generate_handler![]`
- [x] 1.9 Write Rust tests: duplicate name, nonexistent id, assignment validation, backward compat (spec scenarios from trama-management + project-file-management)

## Phase 2: Frontend — State + IPC Bindings

- [x] 2.1 Add `tramas: Trama[]` and `chapterTramas: Map<string, string|null>` state in `src/routes/+page.svelte`
- [x] 2.2 Derive `chaptersByTrama` grouping chapters by trama_id, sorted by global `chapters_order`
- [x] 2.3 Add IPC functions in `src/lib/tauri.ts`: `crearTrama`, `eliminarTrama`, `asignarCapituloTrama`; optional `tramaId` on `crearCapitulo`
- [x] 2.4 Populate trama state from `cargarIndice` on project load and after each mutation

## Phase 3: Sidebar — Grouped Rendering + Controls

- [x] 3.1 Render collapsible trama groups from `chaptersByTrama`; unassigned section as "Sin trama"
- [x] 3.2 Add "Nueva trama" button wired to `pickText` prompt → `crearTrama` → refresh
- [x] 3.3 Add CSS: `.trama-group`, `.trama-header`, `.trama-delete`, `.drag-over`
- [x] 3.4 Add delete button per trama header with `pendingDelete` confirmation → `eliminarTrama` → refresh
- [x] 3.5 Add i18n keys (`tramas.*`) for prompts, unassigned label, delete confirmation, creation flow options (es/en)

## Phase 4: Creation Flow — Trama Selector

- [x] 4.1 After filename input, show 3-option dialog: existing trama, new trama, skip
- [x] 4.2 "Existing": `pickText` list of trama names → `crearCapitulo` with `tramaId`
- [x] 4.3 "New": `pickText` for name → `crearTrama` → `crearCapitulo` with new id
- [x] 4.4 "Skip": `crearCapitulo` with `tramaId: undefined`

## Phase 5: Drag & Drop

- [x] 5.1 Add `ondragover`/`ondrop` on trama headers and "Sin trama" section; toggle `.drag-over` class
- [x] 5.2 On drop, show confirmation dialog (reuse `pendingDelete` pattern) → `asignarCapituloTrama` → refresh
- [x] 5.3 Drop on "Sin trama" calls `asignarCapituloTrama(path, filename, null)`

## Phase 6: Verification

- [x] 6.1 `cargo test --manifest-path src-tauri/Cargo.toml` — all Rust tests pass (137 passed, 0 failed)
- [x] 6.2 Manual: open old project → loads unassigned; create trama → assign → drag → delete → verify
- [x] 6.3 Manual: new chapter flow — all 3 trama options work end-to-end
- [x] 6.4 `pnpm check` (svelte-check + TypeScript) — no errors (1 a11y warning, 0 errors)
