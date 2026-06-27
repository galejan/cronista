# Delta for user-interface

## ADDED Requirements

### Requirement: Trama-Grouped Chapter Sidebar
The chapters sidebar MUST group chapters by assigned trama. Unassigned chapters SHALL appear in a "Sin trama" (unassigned) section. Each trama group MUST render a collapsible header showing trama name and chapter count. Chapter order within groups SHALL follow global `chapters_order`.

#### Scenario: Chapters render grouped by trama
- GIVEN a project with tramas "A" (chapters 01, 04) and "B" (chapter 03) and unassigned chapter 02
- WHEN the chapters sidebar renders
- THEN "A" header shows 2 chapters listing 01 and 04
- AND "B" header shows 1 chapter listing 03
- AND "Sin trama" section shows chapter 02
- AND global order 01→02→03→04 is preserved across groups

#### Scenario: Collapsible trama group toggles visibility
- GIVEN trama "A" group header is rendered expanded
- WHEN the user clicks the header
- THEN the group collapses, hiding its chapters
- AND clicking again re-expands showing the same chapters

#### Scenario: Empty project shows only unassigned section
- GIVEN a project with no tramas and 2 chapters
- WHEN the sidebar renders
- THEN only the "Sin trama" section appears with both chapters

### Requirement: Nueva Trama Button
The chapters sidebar MUST display a "Nueva trama" button below the "Nuevo capítulo" button. Clicking it SHALL prompt the user for a trama name and invoke `crear_trama` via IPC.

#### Scenario: Button creates trama and refreshes sidebar
- GIVEN the chapters sidebar is visible
- WHEN the user clicks "Nueva trama", enters "Subplot", and confirms
- THEN `crear_trama` is called with "Subplot"
- AND the sidebar shows the new trama group with 0 chapters

#### Scenario: Empty name is rejected
- GIVEN the "Nueva trama" prompt is open
- WHEN the user submits an empty name
- THEN the UI rejects the input and asks for a non-empty name

### Requirement: Chapter Creation Trama Selector
After entering the chapter filename, the new-chapter flow MUST offer the user three options: assign to existing trama, create a new trama, or skip (leave unassigned).

#### Scenario: New chapter assigned to existing trama
- GIVEN trama "A" exists and the user enters a filename
- WHEN the user selects "A" from the trama dropdown and confirms
- THEN `crear_capitulo` is called with `trama_id: Some("id_of_A")`
- AND the new chapter appears inside trama "A" group

#### Scenario: New chapter creates a new trama during flow
- GIVEN the user enters a filename
- WHEN the user selects "Crear nueva trama", enters name "Flashback", and confirms
- THEN `crear_trama("Flashback")` is called first
- THEN `crear_capitulo` is called with the new trama's `id`
- AND the sidebar shows the chapter inside the new trama group

#### Scenario: New chapter skips trama assignment
- GIVEN the user enters a filename
- WHEN the user selects "Sin trama" and confirms
- THEN `crear_capitulo` is called with `trama_id: None`
- AND the chapter appears in the unassigned section

### Requirement: Chapter Drag-and-Drop Between Tramas
The chapters sidebar MUST support dragging a chapter from one trama group (or unassigned) to another using HTML5 Drag and Drop. Dropping a chapter SHALL show a confirmation dialog before invoking `asignar_capitulo_trama`. The existing DnD pattern (`dragId`, `handleDrag*` handlers) SHALL be reused.

#### Scenario: Drag chapter to different trama group
- GIVEN chapter "01" is in trama "A" group and trama "B" group is visible
- WHEN the user drags chapter "01" and drops it on trama "B" header
- THEN a confirmation dialog appears asking to move "01" to trama "B"
- AND upon confirming, `asignar_capitulo_trama("01.md", Some("id_of_B"))` is called
- AND the sidebar re-renders with chapter "01" inside trama "B"

#### Scenario: Drag chapter to unassigned section
- GIVEN chapter "03" is in trama "A" group
- WHEN the user drags chapter "03" to the "Sin trama" section and confirms
- THEN `asignar_capitulo_trama("03.md", None)` is called
- AND chapter "03" moves to the unassigned section

#### Scenario: Drag cancelled via dialog
- GIVEN the confirmation dialog is shown after a drop
- WHEN the user clicks "Cancelar"
- THEN no IPC call is made
- AND the chapter stays in its original trama group

#### Scenario: Visual feedback during drag
- GIVEN a chapter is being dragged
- WHEN hovering over a trama group header
- THEN the header shows a drop-target highlight class

### Requirement: Delete Trama Button
Each trama group header MUST include a delete button. Clicking it SHALL show a confirmation dialog. Upon confirming, `eliminar_trama` is called and chapters move to the unassigned section.

#### Scenario: Delete trama with confirmation
- GIVEN trama "A" has 2 chapters
- WHEN the user clicks the delete button on trama "A" header and confirms
- THEN `eliminar_trama(id_of_A)` is called
- AND the sidebar shows both chapters in the unassigned section
- AND the "A" trama group disappears

#### Scenario: Delete trama cancelled
- GIVEN the delete confirmation dialog is shown
- WHEN the user clicks "Cancelar"
- THEN no IPC call is made and the trama remains
