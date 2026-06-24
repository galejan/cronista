# Design: Línea Temporal como Pestaña + Entidad Lugares

## Technical Approach

Dos cambios arquitectónicos independientes sobre el sidebar: (1) la timeline pasa de sección colapsable fija a cuarta pestaña, y (2) se agrega una quinta pestaña "Lugares" con CRUD completo. Ambos siguen los patrones existentes: pestañas `{#if activeTab}` para UI, wrappers `invoke` en TypeScript, comandos `#[tauri::command]` en Rust, y archivos JSON como persistencia.

## Architecture Decisions

| Decisión | Opción elegida | Alternativa | Por qué |
|---|---|---|---|
| Timeline como pestaña vs sección fija | `{#if activeTab === "timeline"}` dentro de `.sidebar-content` | Mantener sección colapsable abajo | Unifica navegación bajo Ctrl+T, elimina estado `timelineVisible`, consistent with chapters/characters/notes tabs |
| Eliminar Ctrl+L | Remover handler y row de ayuda | Mapearlo a `activeTab = "timeline"` | Ctrl+T ya cubre la navegación. Un solo atajo para todo el ciclo de pestañas reduce carga cognitiva |
| Estructura de Lugares | `lugares/index.json` (array de `{id, name}`) + `lugares/{id}.json` (objeto completo) | Un solo archivo `lugares.json` | Mismo patrón que `personajes/`. Index separado permite listar sin cargar archivos pesados |
| Íconos de pestañas | Notebook, User, Notepad, Clock, MapTrifold — imports direct-path de phosphor-svelte/lib/ | Usar íconos existentes (Article, Users, Note) | Los nuevos íconos comunican mejor el propósito visual de cada pestaña según el diseño de UX especificado |
| "Add to place" en menú contextual | Prompt para elegir lugar (crear si no existe), append al description | Selector dropdown con lista precargada | El prompt es más simple, no requiere overlay, y sigue el mismo patrón que `handleSaveAsTrait` |

## Data Flow

```
+page.svelte ──invoke──→ tauri.ts ──invoke──→ Rust #[tauri::command]
     │                                           │
     │  listar_lugares(projPath)                 │  lee lugares/index.json
     │  crear_lugar(projPath, json)              │  escribe lugares/{id}.json + index
     │  cargar_lugar(projPath, id)               │  lee lugares/{id}.json
     │  actualizar_lugar(projPath, id, json)     │  sobrescribe lugares/{id}.json
     │  eliminar_lugar(projPath, id)             │  borra archivo, remueve de index
     │                                           │
     ▼                                           ▼
  UI: lista + formulario inline             File System
  (mismo pattern que personajes)            {proj}/lugares/
```

**Flujo "Add to place" desde menú contextual:**
```
EditorContextMenu.onAddToPlace()
  → handleAddToPlace() en +page.svelte
    → prompt("¿A qué lugar?") → nombre
    → listar_lugares() → buscar match
    → si no existe: crear_lugar({name: nombre, description: ""})
    → cargar_lugar(id) → append selectedText + "\n" a description
    → actualizar_lugar(id, json)
    → refreshLugares()
```

## File Changes

| File | Action | Description |
|---|---|---|
| `src/routes/+page.svelte` | Modify | Extraer `.timeline-section` → `{#if activeTab === "timeline"}`. Agregar `{#if activeTab === "lugares"}` con lista + form. Cambiar íconos de tabs (Notebook, User, Notepad, Clock, MapTrifold). Expander `activeTab` type, Ctrl+T array a 5. Eliminar Ctrl+L, eliminar `timelineVisible`. Estados lugares. Handler `handleAddToPlace`. Context menu prop `onAddToPlace`. |
| `src/lib/tauri.ts` | Modify | 5 funciones: `listarLugares`, `crearLugar`, `cargarLugar`, `actualizarLugar`, `eliminarLugar` |
| `src/lib/i18n.svelte.ts` | Modify | ~20 nuevas keys: `tabs.timeline`, `tabs.places`, `places.*`, `context.addToPlace`. Actualizar `help.timelineDesc` y `help.shortcuts.cycleTabs`. Eliminar `help.shortcuts.toggleTimeline`. |
| `src/lib/components/EditorContextMenu.svelte` | Modify | Prop `onAddToPlace`, botón `{#if selectedText}` con `t("context.addToPlace")` |
| `src-tauri/src/lib.rs` | Modify | Structs `Lugar`, `LugarIndexItem`. 5 comandos: `listar_lugares`, `crear_lugar`, `cargar_lugar`, `actualizar_lugar`, `eliminar_lugar`. Agregar `"lugares"` a subdirs de `crear_proyecto` y `create_project_for_test`. Seed `lugares/index.json = []`. Registrar comandos en `invoke_handler`. |
| `package.json` | Modify | `version`: `0.1.7` → `0.1.8` |
| `src-tauri/Cargo.toml` | Modify | `version`: `0.1.7` → `0.1.8` |
| `src-tauri/tauri.conf.json` | Modify | `version`: `0.1.7` → `0.1.8` |

## Interfaces / Contracts

```rust
// lib.rs — nuevos structs (siguen patrón CharacterIndexItem / Character)
struct Lugar {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
}

struct LugarIndexItem {
    id: String,
    name: String,
}
```

```typescript
// tauri.ts — 5 nuevos wrappers (siguen patrón listarPersonajes / crearPersonaje / etc.)
export async function listarLugares(proyectoPath: string): Promise<string>
export async function crearLugar(proyectoPath: string, lugarJson: string): Promise<string>
export async function cargarLugar(proyectoPath: string, id: string): Promise<string>
export async function actualizarLugar(proyectoPath: string, id: string, lugarJson: string): Promise<string>
export async function eliminarLugar(proyectoPath: string, id: string): Promise<string>
```

```typescript
// +page.svelte — type expansion
activeTab: "capitulos" | "personajes" | "notas" | "timeline" | "lugares"
// Ctrl+T order array
const order = ["capitulos", "personajes", "notas", "timeline", "lugares"]
```

## Testing Strategy

| Layer | What to Test | Approach |
|---|---|---|
| Rust unit | `listar_lugares` con index vacío, `crear_lugar` con duplicado, `cargar_lugar` con ID inexistente, `eliminar_lugar` limpia archivo + index, `crear_proyecto` crea `lugares/` y seedea index.json | `#[test]` en `mod tests` dentro de `lib.rs`, usando `create_project_for_test` |
| Frontend | Sin test runner (vitest/jest no instalado) | Verificación manual: `pnpm check` para type-checking de Svelte. `cargo test` para Rust. |

## Migration / Rollout

- `timelineVisible` y `Ctrl+L` se eliminan. Si un proyecto existente tenía `timelineVisible: true` en memoria, se pierde al recargar (es estado de sesión, no persistido). Sin migración de datos requerida.
- Proyectos existentes que no tengan `lugares/index.json` son compatibles: `listar_lugares` retorna `"[]"` si el archivo no existe.
- Rollback: `git revert`. Los archivos `lugares/*.json` creados por esta versión quedan huérfanos pero no rompen nada si el código vuelve a 0.1.7 (no los lee).

## Open Questions

- Ninguna. Todas las decisiones están cubiertas por la propuesta.
