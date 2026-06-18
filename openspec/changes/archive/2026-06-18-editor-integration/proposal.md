# Proposal: Editor Integration (TipTap + Debounce Save)

## Intent

Wire TipTap rich text editor into the 60% zone with 2-second debounced save. Backend needs two missing commands: read chapter content and create new chapters. Sidebar stays as placeholder.

## Scope

### In Scope
- Rust: `cargar_capitulo` (read chapter .md) and `crear_capitulo` (create chapter file + append to metadata.json)
- TS: invoke wrappers for both commands
- `Editor.svelte` — TipTap with bubble menu (h1, h2, h3 headings + font-family selector only)
- Debounce (2s) from editor content → `guardarCapitulo`; cancel on chapter switch
- Chapter flow: create → load → edit → save
- Svelte 5 `$state` runes in `+page.svelte` for active chapter/project-path/content

### Out of Scope
- Sidebar implementation (placeholder stays)
- Bold, italic, underline, links, or any rich text beyond headings + font-family
- Checkpoint timer wiring
- Collision/conflict handling
- Separate state management store module

## Capabilities

### New Capabilities
- `editor-integration`: TipTap editor component, debounce save pipeline, chapter create/load flow

### Modified Capabilities
- `project-file-management`: adds `cargar_capitulo` (read single .md chapter) and `crear_capitulo` (create chapter file + update `metadata.json` `chapters_order`)

## Approach

Component-by-component (per exploration recommendation):
1. Backend: `cargar_capitulo` + `crear_capitulo` with `#[cfg(test)]` tests
2. Frontend: invoke wrappers in `src/lib/tauri.ts`
3. `Editor.svelte`: TipTap via `onMount`/`onDestroy`, StarterKit extensions (headings + font-family only), `onUpdate` callback → debounce save
4. Wire in `+page.svelte`: `$state` for active chapter, project path, and content; connect load/save/create

TipTap uses imperative API — `onUpdate` bridges content to debounce; `editor.commands.setContent()` loads chapters. Debounce utility cancels on unmount and on chapter switch.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/lib.rs` | Modified | Add `cargar_capitulo`, `crear_capitulo` commands + tests |
| `src/lib/tauri.ts` | Modified | Add `cargarCapitulo`, `crearCapitulo` wrappers |
| `src/lib/components/Editor.svelte` | New | TipTap component with bubble menu + debounce hookup |
| `src/routes/+page.svelte` | Modified | Replace placeholder, add `$state` wiring |
| `src/app.css` | Modified | TipTap editor content typography styles |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| TipTap v3 + Svelte 5 runes impedance | Med | `onMount`/`onDestroy` pattern; content bridged via `onUpdate` callback |
| Metadata sync on chapter creation (crash = orphan file) | Low | Write chapter file first, then update metadata; sub-second gap acceptable |
| Debounce fires for wrong chapter on rapid switch | Med | Cancel pending debounce in chapter switch handler |

## Rollback Plan

Remove `Editor.svelte`, restore placeholder `<p>` in `+page.svelte`. Delete `cargar_capitulo`/`crear_capitulo` from `lib.rs` and Tauri handler registry. No migration needed — new commands are additive.

## Dependencies

- TipTap deps installed (v3.27.0 in `package.json`)
- Existing `guardar_capitulo` backend command

## Success Criteria

- [ ] TipTap renders in 60% zone with h1/h2/h3 and font-family bubble menu
- [ ] Debounce persists content to `capitulos/*.md` within 2s of last keystroke
- [ ] Chapter create → load → edit → save round-trip works end-to-end
- [ ] Debounce cancelled on chapter switch (no cross-chapter content save)
- [ ] All new Rust `#[cfg(test)]` tests pass; all 21 existing tests still pass
