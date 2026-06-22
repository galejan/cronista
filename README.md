# Cronista · Editor literario Local-First

**Cronista** es un editor de texto enriquecido diseñado para escribir novelas largas. Combina una zona de escritura libre de distracciones con un panel lateral que integra capítulos, fichas de personajes, notas y una línea de tiempo. Todo se guarda en local: archivos Markdown y JSON dentro de una carpeta de proyecto, sin depender de la nube.

---

## Qué lo hace distinto

| Principio | En la práctica |
|-----------|----------------|
| **Zona de escritura limpia** | La mayor parte de la pantalla es solo para escribir. Sin menús, sin barra de formato. |
| **Sidebar integrado** | Capítulos, personajes, notas y línea de tiempo en el panel lateral. Con drag and drop en la línea temporal. |
| **Local-First** | Todo en tu disco. `.md` para el texto, `.json` para índices y metadatos. |
| **Git invisible** | Cada cierre de la aplicación crea un checkpoint automático. Historial completo sin intervención manual. |
| **TipTap como motor** | Editor WYSIWYG con títulos semánticos (H1, H2). Limpio, sin distracciones de formato. |
| **Exportación integrada** | Permite exportar el proyecto completo en `.zip` o compartir solo los capítulos en un único `.md`. |
| **Accesibilidad** | Zoom de interfaz con `Ctrl++` / `Ctrl+-`. Tres niveles para adaptarse a cada vista. |

---

## Control de versiones con Git

Cronista integra Git de forma transparente para proteger el trabajo. **No es obligatorio**, pero sí muy recomendable.

### ¿Qué hace?

Cada vez que se cierra la aplicación, se crea un checkpoint automático: un commit de Git que registra el estado completo del proyecto en ese momento, incluyendo el recuento de palabras.

Los checkpoints son acumulativos: con el tiempo construyen un historial completo de la novela sin intervención manual.

> **Nota:** El guardado automático (cada 20 segundos) y el guardado manual (`Ctrl+S`) escriben los cambios en el disco. Los checkpoints de Git solo se generan al cerrar la aplicación, como copia de seguridad del trabajo de la sesión.

### ¿Qué pasa si no se tiene Git instalado?

La aplicación funciona sin Git. Lo único que se pierde es el historial de versiones. Al crear un proyecto, Cronista avisa si no encuentra Git y permite continuar sin él. También se puede instalar Git más adelante y el historial comenzará desde ese momento.

### Instalación de Git

| Plataforma | Comando |
|------------|---------|
| Linux (Debian/Ubuntu) | `sudo apt install git` |
| Linux (Arch) | `sudo pacman -S git` |
| macOS | Ya viene incluido, o `brew install git` |
| Windows | [git-scm.com](https://git-scm.com) |

### Consultar el historial

Dentro de la aplicación, el indicador `→` en el panel inferior abre una ventana con las últimas sesiones: fecha, recuento de palabras, archivos modificados y el hash del commit.

### Sincronización remota (SSH)

Cronista permite sincronizar proyectos con un repositorio remoto (GitHub, GitLab, Bitbucket) mediante SSH. Esto permite continuar el trabajo desde otro equipo sin perder el historial.

Al crear un proyecto, o desde la identidad Git en la barra de herramientas, se puede configurar una URL SSH. Cronista:

- Detecta si el repositorio remoto no existe y ofrece crearlo en GitHub.
- Detecta si el remoto ya tiene historial previo (de otra máquina) y ofrece sincronizar ambos historiales de forma segura.
- Aplica una política de 3 intentos: si el push falla tres veces, desactiva la sincronización automática para evitar interrupciones. Se puede reactivar desde la barra de herramientas.

**Nota:** Solo se admiten URLs SSH (`git@github.com:usuario/repo.git`). Las URLs HTTPS no son compatibles por seguridad.

---

## Exportación y uso compartido

Cronista ofrece dos formatos de exportación, accesibles desde los botones de la barra inferior.

| Botón | Formato | Contenido |
|-------|---------|-----------|
| 🗜️ **Exportar** | `.zip` | Proyecto completo: capítulos, personajes, notas, configuración e historial de Git. Ideal para copias de seguridad o para trasladar el proyecto a otro equipo. |
| 📄 **Compartir** | `.md` | Todos los capítulos concatenados en un solo archivo, en orden. Útil para enviar el texto a un lector beta o a un editor. |

Los archivos se guardan dentro de la carpeta `exportaciones/` del proyecto, con la fecha en el nombre (ej. `Mi Novela_2026-06-20.zip`).

### Detalles del formato `.md`

El archivo generado incluye los capítulos en orden, separados por el título de cada uno. El contenido mantiene el formato enriquecido (HTML) tal como se ve en el editor, lo que permite abrirlo en un navegador o conservar los estilos al importarlo en otras herramientas.

---

## Instalación

### Requisitos

- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) ≥ 18
- [pnpm](https://pnpm.io/installation)
- Dependencias de sistema para Tauri v2 ([guía oficial](https://v2.tauri.app/start/prerequisites/))

### Desarrollo

```bash
git clone git@github.com:galejan/cronista.git
cd cronista
pnpm install
pnpm tauri dev
```

### Build para producción

```bash
pnpm tauri build
```

El binario se genera en `src-tauri/target/release/`. En Arch Linux se puede ejecutar directamente (`./cronista`) o crear un archivo `.desktop` a mano.

---

## Estructura del proyecto

```
cronista/
├── src/                  # Frontend SvelteKit
│   ├── lib/
│   │   ├── components/   # Editor.svelte (TipTap wrapper)
│   │   ├── i18n.svelte.ts # Traducciones ES/EN con $state runes
│   │   ├── tauri.ts      # Wrappers de comandos Tauri
│   │   └── debounce.ts
│   └── routes/
│       └── +page.svelte  # Layout principal (sidebar + editor)
├── src-tauri/            # Backend Rust
│   └── src/
│       ├── lib.rs        # ~40 comandos Tauri + lógica de archivos
│       └── main.rs       # Entry point
├── docs/                 # Documentación de diseño
├── openspec/             # Artefactos SDD
└── static/               # Iconos y assets
```

### Estructura de un proyecto de escritura

Cada proyecto creado con Cronista es una carpeta en el disco con esta organización:

```
Mi Novela/
├── capitulos/            # Archivos .md (HTML enriquecido de TipTap)
│   ├── metadata.json     # Índice y orden de capítulos
│   ├── Capítulo 1.md
│   └── Capítulo 2.md
├── personajes/           # Fichas de personaje en .json
├── notas/                # Notas de investigación en .json
├── timeline.json         # Eventos de la línea de tiempo
├── .config/              # Configuración del proyecto (fuente, idioma)
├── .git/                 # Repositorio Git (si está instalado)
└── exportaciones/        # Archivos generados por Exportar y Compartir
```

### Stack técnico

| Capa | Tecnología |
|------|-----------|
| Escritorio | [Tauri v2](https://v2.tauri.app) (Rust) |
| Frontend | [SvelteKit](https://kit.svelte.dev) + [Svelte 5](https://svelte.dev) |
| Editor | [TipTap v3](https://tiptap.dev) (ProseMirror) |
| Estilos | CSS plano (sin dependencias de componentes) |
| Lenguajes | Rust, TypeScript |
| i18n | Sistema propio con Svelte 5 `$state` runes (ES/EN) |

---

## Atajos de teclado

| Atajo | Acción |
|-------|--------|
| <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>←</kbd> | Colapsar panel lateral (modo escritura) |
| <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>→</kbd> | Panel lateral a pantalla completa (modo referencia) |
| <kbd>Ctrl</kbd> + <kbd>←</kbd> / <kbd>→</kbd> | Reducir / ampliar panel lateral (20 % – 100 %) |
| <kbd>Ctrl</kbd> + <kbd>T</kbd> | Navegar pestañas (capítulos → personajes → notas) |
| <kbd>Ctrl</kbd> + <kbd>L</kbd> | Mostrar / ocultar línea de tiempo |
| <kbd>Ctrl</kbd> + <kbd>Enter</kbd> | Anclar / desanclar ficha del personaje seleccionado |
| <kbd>↑</kbd> / <kbd>↓</kbd> / <kbd>Inicio</kbd> / <kbd>Fin</kbd> | Navegar listas del panel lateral |
| <kbd>Ctrl</kbd> + <kbd>P</kbd> | Mostrar / ocultar panel de herramientas |
| <kbd>Ctrl</kbd> + <kbd>S</kbd> | Guardar ahora |
| <kbd>Ctrl</kbd> + <kbd>O</kbd> | Abrir otro proyecto (cierra el actual) |
| <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>N</kbd> | Nuevo proyecto (cierra el actual) |
| <kbd>Ctrl</kbd> + <kbd>↑</kbd> / <kbd>↓</kbd> | Subir / bajar nivel de título |
| <kbd>Ctrl</kbd> + <kbd>D</kbd> | Insertar guion de diálogo (`—`) |
| <kbd>Ctrl</kbd> + <kbd>+</kbd> / <kbd>-</kbd> | Aumentar / reducir tamaño de letra |
| <kbd>F11</kbd> | Pantalla completa |
| <kbd>F1</kbd> o <kbd>?</kbd> | Mostrar / ocultar panel de ayuda |

---

## Comandos del backend

El backend Rust expone los siguientes comandos Tauri:

- **Proyecto**: `crear_proyecto`, `set_active_project`, `marcar_proyecto_cronista`
- **Git**: `detectar_git`, `inicializar_git`, `inicializar_git_con_autor`, `crear_checkpoint`, `verificar_git_inicializado`, `obtener_git_log`, `configurar_remoto`, `sincronizar_remoto`, `reintentar_push`
- **Capítulos**: `guardar_capitulo`, `cargar_capitulo`, `crear_capitulo`, `eliminar_capitulo`, `cargar_indice`
- **Personajes**: `listar_personajes`, `crear_personaje`, `cargar_personaje`, `actualizar_personaje`, `eliminar_personaje`
- **Notas**: `listar_notas`, `crear_nota`, `cargar_nota`, `eliminar_nota`
- **Timeline**: `cargar_timeline`, `agregar_evento_timeline`, `reordenar_timeline`, `eliminar_evento_timeline`
- **Exportación**: `exportar_proyecto_zip`, `exportar_proyecto_md`

---

## Licencia

MIT © 2026 — [github.com/galejan/cronista](https://github.com/galejan/cronista)
