# Proposal: Línea Temporal como Pestaña + Entidad Lugares

## Intent

La línea temporal (timeline) hoy es una sección colapsable al pie del sidebar, desconectada del sistema de pestañas que organiza capítulos, personajes y notas. Esto obliga al usuario a usar atajos distintos (Ctrl+L vs Ctrl+T) y rompe la coherencia de navegación. Además, el escritor carece de una entidad para modelar ubicaciones geográficas de su historia.

Este cambio transforma la timeline en una pestaña más del sidebar y agrega una quinta pestaña: **Lugares**, con su modelo de datos y CRUD completo.

## Scope

### In Scope

- Mover timeline de sección colapsable a la 4ª pestaña del sidebar, con su UI y estado actual
- Nueva entidad **Lugares** como 5ª pestaña: modelo `{id, name, description}`, archivos JSON en `lugares/`, CRUD en Rust + TypeScript
- Cambio de íconos de pestaña: Notebook (capítulos), User (personajes), Notepad (notas), Clock (timeline), MapTrifold (lugares)
- Ctrl+T cicla 5 pestañas en vez de 3. Se elimina Ctrl+L.
- Menú contextual: nueva opción "Agregar a lugar" al seleccionar texto
- Actualizar panel de ayuda con nuevos atajos y sección de lugares
- Bump de versión: 0.1.7 → 0.1.8 en `package.json`, `tauri.conf.json`, `Cargo.toml`

### Out of Scope

- Geolocalización, mapas o coordenadas en el modelo de lugares
- Arrastrar-y-soltar entre pestañas
- Modificar la sección de timeline existente en el pie del sidebar (se elimina)
- Cambiar el comportamiento del editor, guardado o checkpoints

## Capabilities

### Modified Capabilities

- `user-interface`: Las pestañas del sidebar pasan de 3 a 5, con nuevos íconos. Ctrl+T cicla 5 tabs; Ctrl+L se elimina. El panel de ayuda refleja los cambios.
- `editor-integration`: El menú contextual gana la opción "Agregar a lugar" para el texto seleccionado.

## Approach

**Migración de timeline**: La sección `.timeline-section` se extrae del pie del sidebar y se convierte en un panel de pestaña (`{#if activeTab === "timeline"}`), idéntico a los paneles existentes de capítulos/personajes/notas. El estado `timelineVisible` se reemplaza por `activeTab === "timeline"`.

**Entidad lugares**: Refleja el patrón existente de personajes. Nuevo struct `Place {id, name, description}` en Rust con comandos `listar_lugares`, `crear_lugar`, `cargar_lugar`, `actualizar_lugar`, `eliminar_lugar`. Los archivos se persisten como JSON en `<proyecto>/lugares/`. En TypeScript, 5 funciones análogas a las de personajes. La UI sigue el mismo patrón de lista + formulario inline que personajes.

**Íconos**: Se importan `Notebook`, `User`, `Notepad`, `Clock`, `MapTrifold` de phosphor-svelte. Los 3 íconos existentes de pestaña (`Article`, `Users`, `Note`) se reemplazan. Los nuevos íconos son aditivos.

**Ctrl+T**: La constante `order` en el handler de teclado se expande a 5 elementos (`"capitulos"`, `"personajes"`, `"notas"`, `"timeline"`, `"lugares"`). El handler Ctrl+L se elimina por completo.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src/routes/+page.svelte` | Modified | Extraer timeline a pestaña, agregar pestaña lugares, cambiar íconos, Ctrl+T, eliminar Ctrl+L, handler `handleAddAsPlace`, menú contexto, ayuda |
| `src/lib/tauri.ts` | Modified | 5 nuevas funciones para lugares (análogas a personajes) |
| `src/lib/i18n.svelte.ts` | Modified | Nuevas claves de traducción para places (~15), actualizar atajos en ayuda |
| `src/lib/components/EditorContextMenu.svelte` | Modified | Nueva opción "Agregar a lugar" + prop `onAddToPlace` |
| `src-tauri/src/lib.rs` | Modified | Struct `Place`, 5 comandos Tauri para lugares, manejo de `lugares/` en `crear_proyecto` |
| `package.json` | Modified | Versión 0.1.7 → 0.1.8 |
| `src-tauri/tauri.conf.json` | Modified | Versión 0.1.7 → 0.1.8 |
| `src-tauri/Cargo.toml` | Modified | Versión 0.1.7 → 0.1.8 |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Eliminar Ctrl+L rompe memoria muscular de usuarios existentes | Medium | El panel de ayuda se actualiza para reflejar nuevos atajos; Ctrl+T unifica toda la navegación de pestañas bajo una sola tecla |
| La pestaña de timeline pierde visibilidad al no estar siempre expandida | Low | La pestaña conserva el badge con la cantidad de eventos; el usuario puede fijarla como tab activo |
| `crear_proyecto` no crea el directorio `lugares/` y falla al guardar | Low | Se agrega la creación del directorio en el comando `crear_proyecto` con test en Rust |

## Rollback Plan

1. `git revert` — los cambios son aislados y no afectan el formato de datos existente
2. La timeline se restaura como sección colapsable; la pestaña lugares se elimina
3. Los archivos `lugares/*.json` creados quedan huérfanos pero no rompen nada (no son leídos si el código vuelve atrás)

## Dependencies

- `phosphor-svelte` ya instalado (v3.1.0) — no se requiere nueva dependencia
- Sin dependencias externas adicionales

## Success Criteria

- [ ] Timeline funciona como pestaña (crear, editar, eliminar, expandir, drag-and-drop) igual que antes
- [ ] Lugares permite CRUD completo: crear, editar, eliminar, listar, cargar
- [ ] Ctrl+T cicla las 5 pestañas; Ctrl+L no tiene efecto
- [ ] Los 5 íconos de pestaña son los nuevos especificados
- [ ] El menú contextual muestra "Agregar a lugar" y persiste el texto como descripción del lugar
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` pasa sin errores
- [ ] `pnpm build` completa sin errores
- [ ] Versión 0.1.8 en los 3 archivos de configuración
