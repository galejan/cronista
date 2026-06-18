# Exploration: editor-integration

## Current State

The scaffold (`scaffold-tauri-project`, archived 2026-06-17) delivered a runnable Tauri v2 + Svelte 5 + TypeScript skeleton with:

**Backend (Rust)** — 5 Tauri commands, all tested (21/21 passing):
- `crear_proyecto` — creates full project dir structure + seeds metadata.json + timeline.json
- `inicializar_git` — silent `git init` with cross-platform git binary detection
- `guardar_capitulo` — writes .md chapter file to disk (Nivel 1, no git commit)
- `crear_checkpoint` — `git add . && git commit` with prose message (Nivel 2)
- `cargar_indice` — reads metadata.json as raw JSON string

**Frontend (Svelte/TS)** — minimal wiring:
- Layout 60/40 CSS Grid with `Ctrl+B` toggle in `+page.svelte`
- Typed `invoke()` wrappers for all 5 commands in `src/lib/tauri.ts`
- Checkpoint timer skeleton in `src/lib/checkpoint.ts` (only `console.log`, not connected)
- TipTap **deps installed** but **not integrated**: `@tiptap/core@3.27.0`, `@tiptap/extension-bubble-menu@3.27.0`, `@tiptap/starter-kit@3.27.0`
- No state management, no Svelte stores, no context
- Sidebar has tab buttons but renders a static placeholder `<p>`
- Editor zone is an empty `<main>` with a static placeholder `<p>`
- No chapter creation, loading, or navigation exists

**Key gap**: No backend command to read a single chapter's .md content. `cargar_indice` returns only metadata.json (chapter list + character index). A new `cargar_capitulo` command is needed.

## Affected Areas

| File | Current Role | What Needs to Change |
|------|-------------|---------------------|
| `src/routes/+page.svelte` | Layout shell (60/40 grid + Ctrl+B + tab buttons + placeholders) | Replace placeholders with real TipTap editor and chapter sidebar; add state wiring |
| `src/lib/tauri.ts` | Typed invoke wrappers for 5 commands | Add `cargarCapitulo()` wrapper for new backend command |
| `src/lib/checkpoint.ts` | Timer skeleton (only console.log) | Wire to real `crearCheckpoint` call with word-count threshold; track inactivity |
| `src-tauri/src/lib.rs` | All 5 backend commands + tests | Add `cargar_capitulo` command; potentially add `crear_capitulo` (new chapter file + metadata update) |
| `package.json` | TipTap deps already installed | No changes needed (deps present) |
| `src-tauri/Cargo.toml` | Rust deps: tauri, serde, chrono, tempfile | No new deps needed for file read (std::fs) |
| `src/app.css` | Tailwind CSS import only | May need TipTap editor content styles |
| `openspec/specs/project-file-management/spec.md` | Main spec for file ops | Will need delta for `cargar_capitulo` + `crear_capitulo` requirements |

### Files to Create (greenfield)
- `src/lib/components/Editor.svelte` — TipTap wrapper component with bubble menu
- `src/lib/components/Sidebar.svelte` — Chapter list with tabs, loading from metadata.json
- `src/lib/stores/project.svelte.ts` — Reactive state (project path, chapters, active chapter, content)
- `src/lib/debounce.ts` — Debounced save utility

## Approaches

### 1. Component-by-Component (Recommended)

Build each piece independently in dependency order, testing at each step.

**Order**:
1. Backend: add `cargar_capitulo` + `crear_capitulo` Rust commands with tests
2. Frontend: add `cargarCapitulo` + `crearCapitulo` invoke wrappers
3. Create `Editor.svelte` — TipTap with bubble menu, expose content via binding
4. Create `Sidebar.svelte` — load chapters from `cargarIndice`, display list
5. Create state stores (`project.svelte.ts`) to hold project path, active chapter, content
6. Wire debounce save (2s) from editor content → `guardarCapitulo`
7. Wire chapter navigation: click sidebar chapter → load via `cargarCapitulo` → set editor content
8. Wire checkpoint timer to real `crearCheckpoint` with word-count threshold

- **Pros**: Each step has clear boundaries, testable independently, progressive integration
- **Cons**: Several components to build, total LoC estimate ~400-600
- **Effort**: Medium

### 2. Vertical Slice — Single Chapter Flow

Build the complete flow for one chapter end-to-end (editor + save + sidebar list item), then expand to multi-chapter.

**Order**:
1. Backend: `cargar_capitulo` + `crear_capitulo`
2. Build the full chain: create chapter → load → edit → debounce save → sidebar list
3. Add navigation between chapters
4. Add checkpoint timer at the end

- **Pros**: User-visible value faster, validates full pipeline early
- **Cons**: Higher coupling during build, harder to test incrementally, more rework if assumptions change
- **Effort**: Medium-High

### 3. Store-First — State Architecture First

Design all reactive state (Svelte 5 `$state` runes, or stores) as the foundation, then build UI components on top.

**Order**:
1. Design `project.svelte.ts` with all state: project path, chapters array, active chapter, editor content, save status
2. Build TipTap component that reads/writes to store
3. Build sidebar that reads from store
4. Wire backend commands through the store layer

- **Pros**: Clean separation of concerns, state is the source of truth, easier to test state logic
- **Cons**: Over-engineering for this stage, more upfront design, risk of premature abstraction
- **Effort**: Medium-High

## Recommendation

**Approach 1 (Component-by-Component)** is the right fit.

Rationale:
- The scaffold is minimal — we need to build the core pieces and there's no existing complexity to refactor around
- Dependency order is natural: backend commands → invoke wrappers → UI components → wiring
- Each piece can be tested independently (Rust `#[cfg(test)]` for backend, Svelte component tests for UI)
- We already have the layout shell, so we're literally replacing placeholder `<p>` tags with real components
- The state management will emerge naturally as we wire components together (Svelte 5 `$state` runes are lightweight enough to start without a formal store layer)

### Specific Technical Decisions

1. **Svelte 5 runes for state**: Use `$state` and `$derived` in `+page.svelte` directly rather than a separate store module. Extracting to `src/lib/stores/` later when complexity warrants it.

2. **TipTap integration**: Use `<script>` + `onMount` pattern (Svelte 5 doesn't support `use:` directives for TipTap cleanly). Editor instance created in `onMount`, destroyed in `onDestroy`. Bubble menu configured with Bold, Italic, H1, H2 extensions from starter-kit.

3. **Debounce save**: A simple `debounce()` utility function (not a full library). On every TipTap `onUpdate`, reset a 2-second timer. When timer fires, call `guardarCapitulo(projectPath, filename, editor.getHTML())`.

4. **New backend commands needed**:
   - `cargar_capitulo(project_path, filename) -> String` — reads `capitulos/{filename}` and returns content
   - `crear_capitulo(project_path, filename, contenido) -> String` — creates new chapter file AND appends to `metadata.json` `chapters_order`, OR keep `guardar_capitulo` for save and add a separate `registrar_capitulo` that updates metadata

5. **Checkpoint timer scope**: Per the product doc, the checkpoint is Nivel 2 (30 min idle + 100 words). The timer skeleton exists. Wiring the real `crearCheckpoint` call into it is in scope for this change. Word-count tracking can be computed from TipTap's `editor.storage.characterCount` or by counting whitespace tokens in the editor content.

## Risks

1. **No backend command for reading chapters**: `cargar_indice` only returns metadata. A new `cargar_capitulo` command must be added to the Rust backend — this is a delta to the already-archived `project-file-management` spec. Need to ensure the new command follows the same error-handling patterns (`Result<String, String>`, UTF-8 round-trip, permission errors).

2. **TipTap v3 API surface**: The installed version is `@tiptap/core@3.27.0`. Svelte 5's `$state` runes and TipTap's imperative API need careful integration — TipTap is framework-agnostic and creates its own DOM, so it doesn't play automatically with Svelte's reactivity. The `onUpdate` callback is the bridge for saving content; the `editor.commands.setContent()` method is the bridge for loading.

3. **Metadata sync**: When creating a new chapter, `metadata.json`'s `chapters_order` must be updated. If the user creates a chapter but the app crashes before metadata is written, the chapter file exists but is "orphaned" (not in the index). Need to decide: write chapter file first then metadata, or both atomically? Recommendation: write chapter file first, then update metadata (file-system-level eventual consistency — acceptable for local-first, user won't notice sub-second gap).

4. **Bubble menu positioning in Tauri window**: TipTap's bubble menu uses Popper.js/floating-ui internally. In the Tauri webview environment, viewport calculations may behave differently than in a browser. Should test early.

5. **Debounce edge cases**: Rapid chapter switching while a debounced save is pending could save content to the wrong chapter file. Must cancel the pending debounce when the active chapter changes.

## Ready for Proposal

**Yes.** The exploration confirms:
- Backend is feature-complete for basic save/load with one gap (`cargar_capitulo` command)
- Frontend has the layout shell and TipTap deps, but zero integration code
- Clear component tree: Editor.svelte + Sidebar.svelte wired into +page.svelte
- State management can start simple (Svelte 5 runes in page component)
- The debounce pattern is well-understood (2s timer, cancel on unmount/chapter switch)
- Total estimated change: ~400-500 lines of new code across 5-6 files
