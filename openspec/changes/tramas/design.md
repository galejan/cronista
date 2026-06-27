# Design: Tramas (Plotlines)

## Technical Approach

Add `tramas: Vec<Trama>` and `chapter_tramas: Vec<ChapterTrama>` to the `Metadata` struct with `#[serde(default)]` for zero-migration backward compatibility. Chapters remain flat in `capitulos/`; trama assignment is metadata-only. Three new Tauri commands handle CRUD. The frontend sidebar groups chapters by trama using a reactive `Map<string, string|null>` derived from metadata, reusing the existing HTML5 DnD pattern from timeline events.

## Architecture Decisions

| Decision | Option | Tradeoff | Choice |
|----------|--------|----------|--------|
| Data model | `Vec<ChapterTrama>` (separate vec) vs flat map vs embed in chapters_order | Separate vec mirrors existing `chapters_order` + `characters_index` pattern; map would be harder to serialize; embedding breaks all old projects | `Vec<ChapterTrama>` |
| ID generation | Inline slugify vs external crate vs UUID | Spec requires slug-from-name + random suffix for uniqueness. Inline avoids dep bloat for a 4-line function | Inline: lowercase, hyphens, strip non-alnum, append 8-char random hex |
| Trama selector UX | 3-button dialog vs `pickText` dropdown | Spec requires 3 options (existing/new/none). `pickText` is single-input. A simple `ask`-style dialog with buttons fits the existing no-dependency UI pattern | 3-button dialog: "tramas.selectExisting" (show pickText with list), "tramas.createNew" (pickText for name), "tramas.skip" |
| Confirmation pattern | `pendingDelete` double-click vs native `confirm()` | `pendingDelete` is already implemented and user-tested for chapter/character deletion. `confirm()` blocks the UI thread differently | Reuse `pendingDelete` for delete-trama confirmation |

## Data Flow

```
+page.svelte                    Tauri (lib.rs)                   Filesystem
─────────────                   ──────────────                   ──────────
refreshChapters()
  → cargarIndice(path) ────────→ cargar_indice ─────────────────→ metadata.json
  ← {tramas, chapter_tramas} ←── returns JSON  ←──────────────── read

crearTramaHandler(nombre)
  → crearTrama(path, nombre) ──→ crear_trama
                                  slugify + random suffix
                                  push to metadata.tramas
                                  write metadata ───────────────→ metadata.json
  ← Trama {id, nombre} ←──────── Ok(Trama)

handleDrop(tramaId)
  → ask("¿Mover a trama X?")
  → asignarCapituloTrama ──────→ asignar_capitulo_trama
    (path, filename, Some(id))    update chapter_tramas entry
                                  write metadata ───────────────→ metadata.json
  → refreshChapters()
```

## File Changes

| File | Action | Lines | Description |
|------|--------|-------|-------------|
| `src-tauri/src/lib.rs` | Modify | ~75 add | `Trama` + `ChapterTrama` structs; `crear_trama`, `eliminar_trama`, `asignar_capitulo_trama` commands; modify `crear_capitulo` signature to accept `trama_id: Option<String>`; add fields to `Metadata` and `crear_proyecto` seed; update `generate_schema`; add to `generate_handler![]` |
| `src/routes/+page.svelte` | Modify | ~85 add | `tramas` and `chapterTramas` state; `chaptersByTrama` derived; grouped sidebar template with collapsible headers; DnD handlers for chapters; new creation flow (3-option trama selector); CSS for `.trama-group`, `.trama-header`, `.trama-delete`, `.drag-over` |
| `src/lib/tauri.ts` | Modify | ~8 add | `crearTrama`, `eliminarTrama`, `asignarCapituloTrama` IPC bindings; optional `tramaId` param on `crearCapitulo` |
| `src/lib/i18n.svelte.ts` | Modify | ~15 add | Keys: `tramas.*` (es/en) for new-trama prompt, unassigned label, delete confirmation, creation flow options |

## Key Interfaces

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Trama { id: String, nombre: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChapterTrama { filename: String, #[serde(default)] trama_id: Option<String> }

// Metadata additions (both #[serde(default)])
tramas: Vec<Trama>,
chapter_tramas: Vec<ChapterTrama>,

// New/modified commands
fn crear_trama(path: String, nombre: String) -> Result<Trama, String>
fn eliminar_trama(path: String, id: String) -> Result<(), String>
fn asignar_capitulo_trama(path: String, filename: String, trama_id: Option<String>) -> Result<(), String>
fn crear_capitulo(path: String, filename: String, contenido: String, trama_id: Option<String>) -> Result<String, String>
```

Frontend state:
```ts
let tramas = $state<Trama[]>([]);
let chapterTramas = $state<Map<string, string | null>>(new Map());
// Derived: Map<string | null, string[]> grouping chapters by trama_id
```

## Testing Strategy

| Layer | What | How |
|-------|------|-----|
| Unit (Rust) | ID generation uniqueness; trama CRUD with temp dir | `cargo test` in `src-tauri/` — create temp project, exercise commands, assert metadata.json contents |
| Unit (Rust) | Backward compat: old metadata.json deserializes without error | Serialize Metadata without `tramas`/`chapter_tramas` fields, deserialize — assert defaults to empty vecs |
| Integration | End-to-end: create trama → assign chapter → delete trama → verify unassignment | Tauri test with temp project directory |
| UI | Sidebar groups, DnD visual feedback | Manual verification (Tauri app has no automated UI tests) |

## Migration

No migration required. `#[serde(default)]` ensures old `metadata.json` files deserialize with `tramas: []` and `chapter_tramas: []`. New projects seed both fields as empty arrays in `crear_proyecto`.

## Open Questions

None.
