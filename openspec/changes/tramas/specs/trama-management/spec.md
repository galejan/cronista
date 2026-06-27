# trama-management Specification

## Purpose
Trama entity CRUD and chapter-trama assignment. Tramas are metadata-only groupings — no filesystem changes. Chapters remain flat in `capitulos/` regardless of trama membership.

## Requirements

### Requirement: Trama Creation
The system MUST allow creating a trama with a unique auto-generated `id` and a user-provided `nombre`. The `id` SHALL be slugified from `nombre` with a random suffix to guarantee uniqueness.

#### Scenario: Create trama with unique id
- GIVEN no tramas exist in the project
- WHEN `crear_trama("El Viaje del Héroe")` is called
- THEN a trama with `nombre: "El Viaje del Héroe"` and a non-empty unique `id` is persisted
- AND `listar_tramas()` returns the new trama

#### Scenario: Reject duplicate trama name
- GIVEN a trama named "Principal" already exists
- WHEN `crear_trama("Principal")` is called
- THEN it returns `Err(String)` and no duplicate is created

### Requirement: Trama Deletion
The system MUST delete a trama by `id`. Chapters assigned to the deleted trama SHALL become unassigned (`trama_id: null`). No chapter `.md` files SHALL be deleted.

#### Scenario: Delete trama unassigns chapters
- GIVEN trama "A" has 3 chapters assigned
- WHEN `eliminar_trama(id_of_A)` is called
- THEN the trama is removed from `metadata.tramas`
- AND all 3 chapters now have `trama_id: null` in `chapter_tramas`
- AND all chapter `.md` files remain in `capitulos/`

#### Scenario: Delete nonexistent trama returns error
- GIVEN no trama with id "ghost"
- WHEN `eliminar_trama("ghost")` is called
- THEN it returns `Err(String)` and metadata is unchanged

### Requirement: Chapter-Trama Assignment
The system MUST support assigning a chapter to a trama or unassigning it (`trama_id: null`). Only the `chapter_tramas` entry SHALL be updated; `chapters_order` MUST NOT change.

#### Scenario: Assign chapter to existing trama
- GIVEN chapter "01_prologo.md" is unassigned and trama "A" exists
- WHEN `asignar_capitulo_trama("01_prologo.md", Some("id_of_A"))` is called
- THEN the chapter's `trama_id` becomes `id_of_A` in `chapter_tramas`
- AND `chapters_order` remains unchanged

#### Scenario: Unassign chapter from trama
- GIVEN chapter "01_prologo.md" is assigned to trama "A"
- WHEN `asignar_capitulo_trama("01_prologo.md", None)` is called
- THEN the chapter's `trama_id` becomes `null`
- AND the trama still exists

#### Scenario: Reject assignment to nonexistent trama
- GIVEN chapter "01_prologo.md" exists
- WHEN `asignar_capitulo_trama("01_prologo.md", Some("ghost"))` is called
- THEN it returns `Err(String)`

### Requirement: Trama Listing
The system MUST return all tramas with their assigned chapter count.

#### Scenario: List tramas with counts
- GIVEN tramas "A" (2 chapters), "B" (0 chapters), "C" (1 chapter) exist
- WHEN `listar_tramas()` is called
- THEN it returns `[{id, nombre: "A", chapter_count: 2}, {id, nombre: "B", chapter_count: 0}, {id, nombre: "C", chapter_count: 1}]`

### Requirement: Backward Compatibility
The `Metadata` struct MUST use `#[serde(default)]` on `tramas` and `chapter_tramas` fields. Old projects lacking these fields SHALL deserialize with empty arrays and all chapters unassigned.

#### Scenario: Old project loads without errors
- GIVEN `metadata.json` lacks `tramas` and `chapter_tramas` fields
- WHEN `cargar_indice` reads and deserializes it
- THEN `Metadata.tramas` is `[]` and `Metadata.chapter_tramas` is `[]`
- AND no error, panic, or migration prompt occurs
