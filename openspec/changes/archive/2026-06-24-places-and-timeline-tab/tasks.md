# Tasks: Línea Temporal como Pestaña + Entidad Lugares

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~410-440 |
| 400-line budget risk | Medium |
| Chained PRs recommended | No |
| Suggested split | Single PR (exception-ok) |
| Delivery strategy | exception-ok |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: Medium

## Phase 1: Backend Rust (structs, commands, tests) ✅ COMPLETE

### T-01 — Structs `Lugar` + `LugarIndexItem` y extensión de `Metadata`
**Archivo**: `src-tauri/src/lib.rs`
**Líneas**: ~14 | **Dependencias**: ninguna

- Agregar `#[derive(Serialize, Deserialize, Debug, Clone)] struct Lugar { id, name, description }` (la description con `#[serde(default)]`)
- Agregar `struct LugarIndexItem { id, name }`
- Agregar campo `places_index: Vec<LugarIndexItem>` a `Metadata`, con `#[serde(default)]`

### T-02 — 5 comandos CRUD para lugares
**Archivo**: `src-tauri/src/lib.rs`
**Líneas**: ~135 | **Dependencias**: T-01

- `listar_lugares(proyecto_path)`: lee `lugares/index.json`, retorna `"[]"` si no existe
- `crear_lugar(proyecto_path, lugar_json)`: parsea JSON → valida id y name no vacíos → rechaza duplicado → escribe `lugares/{id}.json` → actualiza index
- `cargar_lugar(proyecto_path, id)`: lee y retorna `lugares/{id}.json`
- `actualizar_lugar(proyecto_path, id, lugar_json)`: sobrescribe archivo; si cambió name, actualiza index
- `eliminar_lugar(proyecto_path, id)`: borra archivo, remueve del index
- Todos siguen el patrón exacto de los comandos de personajes (`listar_personajes`, `crear_personaje`, etc.)

### T-03 — Adaptar `crear_proyecto` + registro de comandos
**Archivo**: `src-tauri/src/lib.rs`
**Líneas**: ~15 | **Dependencias**: T-01, T-02

- Agregar `"lugares"` al array `subdirs` en `crear_proyecto` y `create_project_for_test`
- Seed `lugares/index.json` como `[]` en ambos
- Incluir `places_index: vec![]` en la construcción de `Metadata`
- Registrar los 5 nuevos comandos en `invoke_handler`

### T-04 — Tests Rust para places CRUD + proyecto
**Archivo**: `src-tauri/src/lib.rs` (dentro de `mod tests`)
**Líneas**: ~53 | **Dependencias**: T-02, T-03

- `test_crear_proyecto_creates_lugares_directory`: verifica que `lugares/` existe y tiene `index.json` con `[]`
- `test_crear_proyecto_metadata_has_places_index`: verifica `places_index: []` en metadata.json
- `test_listar_lugares_empty`: retorna `"[]"` para proyecto sin lugares
- `test_crear_lugar_y_listar`: crea 1 lugar, verifica archivo + index + listado
- `test_crear_lugar_rechaza_duplicado`: error si ya existe
- `test_cargar_lugar_not_found`: error con ID inexistente
- `test_actualizar_lugar_overwrites`: cambia description y name, verifica archivo e index
- `test_eliminar_lugar_limpia`: borra archivo e index, verifica que ambos desaparecen

## Phase 2: Frontend Wrappers ✅ COMPLETE

### T-05 — 5 funciones TypeScript para lugares
**Archivo**: `src/lib/tauri.ts`
**Líneas**: ~30 | **Dependencias**: T-02 (contrato de comandos)

- `listarLugares(proyectoPath) → invoke("listar_lugares", ...)`
- `crearLugar(proyectoPath, lugarJson) → invoke("crear_lugar", ...)`
- `cargarLugar(proyectoPath, id) → invoke("cargar_lugar", ...)`
- `actualizarLugar(proyectoPath, id, lugarJson) → invoke("actualizar_lugar", ...)`
- `eliminarLugar(proyectoPath, id) → invoke("eliminar_lugar", ...)`

## Phase 3: i18n ✅ COMPLETE

### T-06 — Nuevas keys ES + EN para lugares, timeline tab y menú contextual
**Archivo**: `src/lib/i18n.svelte.ts`
**Líneas**: ~34 | **Dependencias**: ninguna

Sub-tareas:
- Agregar `tabs.timeline` y `tabs.places` (ES: "Línea de tiempo" / "Lugares", EN: "Timeline" / "Places")
- Agregar bloque `places.*`: `empty`, `new`, `namePlaceholder`, `nameRequired`, `descriptionPlaceholder`, `create`, `save`, `edit`, `delete`, `deleteConfirm`, `createError`, `saveError`, `deleteError`
- Agregar `context.addToPlace`: "Agregar a lugar" / "Add to place"
- Agregar `context.placePrompt`: "¿A qué lugar? (nombre exacto)" / "Which place? (exact name)"
- Modificar `help.timelineDesc`: actualizar descripción (ya no está en el footer, es una pestaña)
- Modificar `help.shortcuts.cycleTabs`: "Navegar 5 pestañas" / "Cycle 5 tabs"
- Eliminar `help.shortcuts.toggleTimeline` en ES y EN
- Agregar sección `help.placesTitle` y `help.placesDesc` en ES y EN

## Phase 4: UI (+page.svelte) ✅ COMPLETE

### T-07 — Expandir estado, íconos, Ctrl+T y remover Ctrl+L
**Archivo**: `src/routes/+page.svelte`
**Líneas**: ~35 | **Dependencias**: T-05, T-06

Sub-tareas:
- Importar `MapTrifold` y `Notepad` de phosphor-svelte/lib
- Importar `listarLugares, crearLugar, cargarLugar, actualizarLugar, eliminarLugar` de `$lib/tauri`
- Expandir tipo de `activeTab` a `"capitulos" | "personajes" | "notas" | "timeline" | "lugares"`
- Agregar estados de lugares: `lugares`, `lugarFormVisible`, `lugarNuevoNombre`, `lugarNuevaDescripcion`, `lugarExpandido`, `lugarEditando`
- Cambiar íconos en botones de pestaña: `Article`→`Notebook`, `Users`→`User`, `Note`→`Notepad`
- Agregar 2 nuevos botones de pestaña: Timeline (Clock) y Places (MapTrifold)
- Expandir array `order` en Ctrl+T a 5 elementos (`"timeline"`, `"lugares"`), cambiar tipo a `string[]`
- Eliminar bloque completo de Ctrl+L (~6 líneas)

### T-08 — Migrar timeline de footer colapsable a tab panel
**Archivo**: `src/routes/+page.svelte`
**Líneas**: ~25 | **Dependencias**: T-07

Sub-tareas:
- Eliminar `let timelineVisible = $state(false)`
- Eliminar bloque `.timeline-section` completo (toggle button + `{#if timelineVisible}` wrapper) del footer
- Envolver el contenido de timeline (lista, eventos, formulario) dentro de `{#if activeTab === "timeline"}` en `.sidebar-content`
- Eliminar `timelineVisible` de todos los handlers y condiciones

### T-09 — Places tab panel (lista + formulario inline)
**Archivo**: `src/routes/+page.svelte`
**Líneas**: ~65 | **Dependencias**: T-07, T-08

Sub-tareas:
- Agregar `{#if activeTab === "lugares"}` con panel que liste lugares, nombre + descripción truncada
- Botones editar (carga lugar en formulario inline) y eliminar (con confirmación) por cada lugar
- Formulario inline con nombre y descripción para crear/editar
- Funciones handler: `refreshLugares()`, `crearLugarHandler()`, `guardarLugarHandler()`, `eliminarLugarHandler()`
- Seguir el mismo patrón visual y de comportamiento que el panel de personajes

### T-10 — Context menu "Agregar a lugar" + actualizar ayuda
**Archivo**: `src/routes/+page.svelte`
**Líneas**: ~35 | **Dependencias**: T-05, T-06, T-09

Sub-tareas:
- Agregar handler `handleAddToPlace()`: prompt → busca lugar por nombre → si no existe lo crea → append texto seleccionado a descripción → `actualizarLugar`
- Wire `onAddToPlace={handleAddToPlace}` en `<EditorContextMenu>`
- Ayuda: agregar sección "Lugares" con descripción
- Ayuda: actualizar row Ctrl+T → "Navegar 5 pestañas", eliminar row Ctrl+L

## Phase 5: Componente Context Menu ✅ COMPLETE

### T-11 — Nueva prop "onAddToPlace" y botón en menú contextual
**Archivo**: `src/lib/components/EditorContextMenu.svelte`
**Líneas**: ~8 | **Dependencias**: T-06

- Agregar `onAddToPlace: () => void` a la interfaz `Props`
- Agregar botón `{#if selectedText}` con `t("context.addToPlace")` que llame a `onAddToPlace`
- Ubicarlo después de "Añadir como evento" en el grupo de opciones condicionales

## Phase 6: Version Bump ✅ COMPLETE

### T-12 — Bump versión 0.1.7 → 0.1.8
**Archivos**: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`
**Líneas**: ~3 | **Dependencias**: ninguna

- Cambiar `"version"` en los 3 archivos de `0.1.7` a `0.1.8`
