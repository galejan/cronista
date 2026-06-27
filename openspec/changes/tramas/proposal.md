# Proposal: Tramas (Plotlines/Storylines)

## Intent

Writers need to organize chapters into plotlines (tramas) to see narrative structure at a glance. Currently chapters are a flat list — no grouping, no subplot visibility. This change adds metadata-only tramas with drag-and-drop assignment while keeping chapters flat in `capitulos/`.

## Scope

### In Scope
- `Trama` and `ChapterTrama` structs in Metadata with `#[serde(default)]`
- Tauri commands: `crear_trama`, `eliminar_trama`, `asignar_capitulo_trama`
- Chapters sidebar: visual grouping by trama with collapsible headers, "Nueva trama" button
- Chapter creation flow: optional trama assignment (existing/new/none)
- Drag & drop chapters between trama groups with confirmation dialog
- SCHEMA.md regeneration for the trama entity

### Out of Scope
- Trama tags on characters/places/timeline (Phase 2)
- Trama colors/icons (Phase 2)
- Chapter reordering within a trama (global numeric order preserved)
- Chapter editing/renaming UI (separate concern)

## Capabilities

### New Capabilities
- `trama-management`: CRUD for trama entities and chapter-trama assignment

### Modified Capabilities
- `project-file-management`: `Metadata` struct gains `tramas` + `chapter_tramas`; new IPC commands; SCHEMA.md update
- `user-interface`: Chapters sidebar gains trama grouping, "Nueva trama" button, creation-flow trama selector, drag-drop between groups

## Approach

**Approach 2** from exploration: add `tramas: Vec<Trama>` and `chapter_tramas: Vec<ChapterTrama>` to Metadata. `#[serde(default)]` ensures backward compat — old projects load with empty arrays, all chapters unassigned. `chapters_order` stays untouched for ordering.

```rust
struct Trama { id: String, nombre: String }
struct ChapterTrama { filename: String, trama_id: Option<String> }
```

Frontend groups chapters by `chapter_tramas` map. Drag-and-drop reuses existing timeline DnD pattern (`dragId`, `handleDrag*` handlers). Trama selector uses `pickText` for name with dropdown of existing tramas.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/lib.rs` | Modified | Metadata struct + 3 new commands + `generate_schema()` |
| `src/routes/+page.svelte` | Modified | Sidebar grouping, DnD, creation flow, state |
| `src/lib/tauri.ts` | Modified | IPC bindings for new commands |
| `src/lib/i18n.svelte.ts` | Modified | Translation keys (es/en) |
| `openspec/specs/project-file-management/spec.md` | Delta | New requirements for trama operations |
| `openspec/specs/user-interface/spec.md` | Delta | Sidebar grouping, creation flow UI |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Old projects fail to load | Low | `#[serde(default)]` — empty arrays, all chapters unassigned |
| DnD between tramas breaks chapter order | Low | `chapters_order` never changes on trama reassignment; only `chapter_tramas` updates |
| Confirmation dialog UX mismatch | Low | Reuse existing `pendingDelete` double-click confirmation pattern |

## Rollback Plan

Revert the Metadata struct to remove `tramas`/`chapter_tramas` fields. Old `metadata.json` files still contain the fields but serde ignores unknown fields by default — existing projects load fine. Remove DnD handlers and grouping from sidebar template. No data migration needed.

## Dependencies

None.

## Success Criteria

- [ ] Old projects open with no errors and all chapters unassigned
- [ ] "Nueva trama" creates a trama visible in sidebar
- [ ] New chapter can be assigned to existing/new/no trama
- [ ] Drag-and-drop reassignment persists and reloads correctly
- [ ] SCHEMA.md documents the trama model
