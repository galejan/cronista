---
name: cron-insta-project-structure
description: "Trigger: understand the Cron-Insta codebase, file layout, IPC architecture, data model, or how components and modules relate. Reference when delegating creative analysis, feature implementation, or architecture decisions to other skills."
license: Apache-2.0
metadata:
  author: "galejan"
  version: "1.0"
  depends: []
---

# Cron-Insta — Structure & Architecture

> Una app desktop local-first para escritores. Svelte 5 frontend + Tauri (Rust) backend, con Git versionado invisible.

## Quick Reference

| Aspect | Value |
|--------|-------|
| **Stack** | Svelte 5 (runes) + TypeScript + Tauri v2 + Rust |
| **Editor** | TipTap (ProseMirror) — minimal, writing-focused |
| **Package manager** | pnpm |
| **Desktop targets** | Linux (.deb, AppImage), Windows (.msi, .nsis) |
| **Persistence** | Local filesystem (disk-first), Git for versioning |
| **i18n** | es / en — lightweight runtime switch |
| **Port** | `1420` (dev), `1421` (HMR websocket) |

## Directory Map

```
cron-insta/
├── src/                          # Frontend (Svelte 5 + TS)
│   ├── app.html                  # HTML shell (lang="en", <title>Cron-Insta</title>)
│   ├── app.css                   # Global CSS — ProseMirror typography, dark mode
│   ├── routes/
│   │   ├── +layout.svelte        # Root layout: external link handler (Tauri)
│   │   ├── +layout.ts            # Disables SSR (SPA-only: `export const ssr = false`)
│   │   └── +page.svelte          # THE MAIN FILE — single-page app (~1700 lines)
│   └── lib/
│       ├── components/
│       │   ├── Editor.svelte     # TipTap editor wrapper (60% zone)
│       │   └── GitIdentityDialog.svelte  # Identity + remote config dialog
│       ├── i18n.svelte.ts        # Bilingual translations (es/en)
│       ├── tauri.ts              # IPC bridge: all invoke() calls typed here
│       ├── debounce.ts           # Generic debounce utility (used for auto-save)
│       └── checkpoint.ts         # Skeleton: inactivity timer for checkpoint
│
├── src-tauri/                    # Backend (Rust)
│   ├── src/
│   │   ├── main.rs               # Entry: calls cron_insta_lib::run()
│   │   └── lib.rs                # ALL backend logic (~2100 lines + tests)
│   ├── Cargo.toml                # Dependencies: tauri, serde, chrono, zip, tokio
│   ├── tauri.conf.json           # App config: window size, CSP, bundle, icons
│   ├── capabilities/default.json # Tauri v2 capability permissions
│   └── icons/                    # App icons (32x32, 128x128, icns, ico)
│
├── openspec/                     # SDD specifications (Spec-Driven Development)
│   ├── config.yaml               # Stack description + phase rules
│   ├── specs/
│   │   ├── project-file-management/  # Folder creation, chapter CRUD, metadata
│   │   ├── git-abstraction/         # Git detection, init, checkpoint, push
│   │   ├── git-identity-config/     # Global identity storage (Cervantes/Shakespeare)
│   │   ├── git-remote-sync/         # SSH-only remote, 3-strike auto-disable
│   │   └── editor-integration/      # TipTap mount, debounce save, bubble menu
│   └── changes/
│       └── archive/                  # Completed change artifacts
│           ├── 2026-06-17-scaffold-tauri-project/
│           ├── 2026-06-18-editor-integration/
│           └── 2026-06-20-git-identity-remote/
│
├── static/                       # Static assets (favicon, branding)
├── docs/                         # Documentation (analisis-creativo.md)
├── .github/workflows/build.yml   # CI: pnpm + Tauri build (linux + windows)
├── .atl/                         # Gentle AI skills registry
│   ├── skill-registry.md
│   └── .skill-registry.cache.json
├── svelte.config.js              # SvelteKit → adapter-static (SPA fallback)
├── vite.config.ts                # Vite + Tauri dev server (port 1420)
├── tsconfig.json                 # TypeScript config (strict, bundler resolution)
└── package.json                  # Deps: @tauri-apps/api, @tiptap/*, svelte 5
```

## Architecture Overview

### IPC Architecture (Tauri Commands)

```
┌──────────────────────┐          ┌─────────────────────────┐
│    Frontend (Svelte) │  invoke  │    Backend (Rust)       │
│                      │ ◄──────► │                         │
│  +page.svelte        │   IPC    │  lib.rs                 │
│    ├── state mgmt    │          │    ├── Tauri commands   │
│    ├── CRUD ops      │          │    ├── git helpers      │
│    └── event handlers│          │    ├── export logic     │
│                      │          │    └── #[cfg(test)]     │
│  Components          │          │                         │
│    ├── Editor.svelte │          │  Config files           │
│    └── GitIdentity   │          │    ├── git-config.json  │
│                      │          │    └── .config/*.json   │
│  Bridge (tauri.ts)   │          │                         │
│    └── typed invoke()│          │  Disk                   │
│                      │          │    ├── capitulos/*.md   │
│                      │          │    ├── personajes/*.json│
│                      │          │    ├── notas/*.md       │
│                      │          │    └── exportaciones/   │
└──────────────────────┘          └─────────────────────────┘
```

All IPC uses Tauri v2 `invoke()` calls. The frontend never touches disk directly — all I/O goes through Rust commands. `tauri.ts` is the **single typed bridge** — every function there maps 1:1 to a Rust `#[tauri::command]`.

### Project Disk Structure (per project)

```
{project}/
├── .config/
│   ├── metadata.json        # Project metadata: name, chapters_order, characters_index, font_family, last_modified
│   ├── timeline.json        # JSON array of TimelineEvent objects
│   └── git-config.json      # (global, per Tauri config dir) Identity + remote config
├── capitulos/
│   ├── 0001_prologo.md      # Chapter files (HTML content from TipTap)
│   └── 0002_capitulo_1.md
├── personajes/
│   ├── index.json           # [{id, name}] list
│   └── {id}.json            # Full character sheet
├── notas/
│   ├── index.json           # [{id, title}] list
│   └── {id}.md              # Note content
├── exportaciones/           # Created on first export
│   └── {project}_{date}.zip / .md
└── .git/                    # Auto-initialized if Git is available
```

### Data Model (Rust structs)

```rust
struct Metadata {
    project_name: String,
    last_modified: String,          // ISO 8601
    chapters_order: Vec<String>,    // filenames order
    characters_index: Vec<{id, file, name}>,
    font_family: String,            // "monospace" | "serif" | "sans-serif"
}

struct Character {
    id: String,
    name: String,
    physicalDescription: Option<String>,
    personality: Option<String>,
    traumas: Option<String>,
    relationships: Vec<{targetId, targetName, type, notes}>,
}

struct TimelineEvent {
    id: String,                     // auto-generated "evt-{timestamp}"
    date: String,
    title: String,
    description: String,
    relatedCharacters: Vec<String>, // character IDs
    relatedChapters: Vec<String>,   // chapter filenames
}

// Global config (per Tauri app_config_dir/cron-insta/git-config.json)
struct GitConfig {
    schema_version: u32,
    identity: Option<{name, email}>,
    remote: Option<{url, push_enabled, consecutive_failures}>,
}
```

## Key Architecture Decisions

### 1. Single-Module Rust Backend
**Decision**: All Tauri commands live in one `lib.rs` file (~2100 lines).
**Why**: At the current scale, splitting adds complexity without benefit. Tests are at the bottom of the same file.
**Impact**: As the app grows, extract modules: `git.rs`, `project.rs`, `export.rs`.

### 2. Disk-First, Git-Second
**Decision**: Save to disk (Nivel 1) is immediate; Git commit (Nivel 2) is deferred.
**Why**: The writer never loses text even if Git fails. Checkpoints run on save (auto-commit + push).
**Niveles**:
- Nivel 1: `guardar_capitulo` — write `.md` to disk (debounced 20s, not 2s as stated in old spec)
- Nivel 2: `crear_checkpoint` — `git add .` + `git commit` + optional `git push`

### 3. Global Git Config (not per-project)
**Decision**: Identity and remote config stored globally in Tauri's `app_config_dir()`.
**Why**: Writer sets their identity once. Remote config is global so reconnecting works across projects.
**Format**: `cron-insta/git-config.json` with `schema_version`, `identity`, `remote`.

### 4. SPA-Only, No SSR
**Decision**: SvelteKit with `adapter-static` + `ssr = false` + `fallback: "index.html"`.
**Why**: Tauri has no Node server. Everything runs client-side in the webview.

### 5. Minimal Editor (TipTap with StarterKit only)
**Decision**: Only headings (h1, h2, h3) + paragraph. No bold, italic, links, lists.
**Why**: Writing-focused. No formatting distractions. `Ctrl+D` inserts em-dash pair for dialogues.

## Frontend Component Relationships

```
+page.svelte (THE hub — owns ALL state)
├── State: projectPath, chapters[], personajes[], notas[], timeline[]
│          activeChapter, editorContent, gitStatus, theme, zoomLevel, ...
├── CRUD operations delegated to tauri.ts
│   ├── Chapters:   crearCapitulo, guardarCapitulo, cargarCapitulo, eliminarCapitulo
│   ├── Characters: crearPersonaje, cargarPersonaje, actualizarPersonaje, eliminarPersonaje
│   ├── Notes:      crearNota, cargarNota, eliminarNota
│   └── Timeline:   agregarEvento, eliminarEvento, reordenarTimeline
├── Sidebar: 40% (tabs: capítulos | personajes | notas + collapsible timeline)
├── Editor:  60% (Editor.svelte)
│   └── Communicates via expose (setContent, increaseHeading, etc.) + onUpdate callback
├── Git flows: detectarGit → showIdentityDialog → inicializarGit → configurarRemoto
│   └── Auto-save: debounce(20s) → guardarCapitulo → crearCheckpoint → auto-push
└── Persistence: localStorage for theme, zoom, sidebar width, window state, last project
```

## Save Flow (Critical Path)

```
User types → Editor.onUpdate(html) → handleEditorUpdate() → saveStatus="unsaved"
  → debounce(20_000ms) fires → guardarCapitulo(projectPath, filename, html)
    → Rust writes to disk → saveStatus="saved"
      → crearCheckpoint(projectPath) [best-effort, non-blocking]
        → git add . + git commit
          → if push_enabled: git push (3-strike auto-disable)
```

## Git Integration Points

| Feature | File | Approach |
|---------|------|----------|
| Binary detection | `lib.rs:find_git()` | `which git` (Linux) / `where git` + fallbacks (Windows) |
| Auto-init | `lib.rs:inicializar_git()` | Called from `crear_proyecto`, silent failure |
| Checkpoint | `lib.rs:crear_checkpoint()` | `git add .` + `git commit` with word count |
| Auto-push | `lib.rs:sincronizar_checkpoint()` | 3-strike disable, resets on success |
| Remote config | `lib.rs:configurar_remoto()` | SSH only, `git remote add origin + push -u` |
| Close handler | `lib.rs:on_window_event()` | Off-event-loop checkpoint before destroy |
| Identity | Global `git-config.json` | Cervantes (es) / Shakespeare (en) presets |

## i18n Architecture

- **File**: `src/lib/i18n.svelte.ts`
- **Reactivity**: `lang` is a `$state` object — `t("key")` is reactive in templates
- **Keys**: Dot-notation, e.g. `"tabs.chapters"`, `"dialog.projectName"`
- **Fallback**: Missing key → returns the key itself
- **Storage**: `localStorage.setItem("cron-insta-lang", lang)`

## CI/CD (GitHub Actions)

One workflow (`.github/workflows/build.yml`):
- Triggers: push to `main`, PR to `main`, tag `v*`
- Two jobs (parallel): `build-linux` (deb + AppImage), `build-windows` (msi + nsis)
- Steps: checkout → pnpm setup → system deps → `pnpm install` → `pnpm tauri build`
- Uploads artifacts per platform

## Testing Strategy

| Layer | Framework | Location |
|-------|-----------|----------|
| Rust unit | `#[cfg(test)] mod tests` | Bottom of `lib.rs` |
| Rust integration | `tempfile::TempDir` + real git | Same module |
| Frontend | Planned (Vitest + Svelte Testing Library) | Not yet scaffolded |

Rust tests use `create_project_for_test()` and `init_git_for_test()` helpers that mirror production commands without needing `tauri::AppHandle`. Tests that require git skip gracefully with `eprintln!("SKIP: git not available")`.

## Extending This App

### Adding a New Tauri Command
1. Add the Rust function in `lib.rs` with `#[tauri::command]`
2. Register it in `generate_handler![]` inside `run()`
3. Add the typed JS wrapper in `src/lib/tauri.ts`
4. Call it from `+page.svelte` or a component

### Adding a New Component
1. Create in `src/lib/components/`
2. Import in `+page.svelte` (hub pattern — components don't own state)
3. Pass callbacks and reactive values as props / bindable

### Adding a New File Type to the Project
1. Add the directory path in `subdirs` inside `crear_proyecto`
2. Create CRUD commands in Rust (always validate `proyecto_path` and `id`)
3. Add typed wrappers in `tauri.ts`
4. Add UI tab in the sidebar if needed

## Skill Integration Patterns

When delegating work from creative analysis skills (like `analisis-creativo.md`) to Cron-Insta operations:

```
Skill de Análisis → produce Chapter splits, Character sheets, Timeline events, Notes
  → Cron-Insta backend: crearCapitulo(), crearPersonaje(), agregarEventoTimeline(), crearNota()
  → All go through tauri.ts → Rust IPC → disk persistence
```

Key constraints:
- **Chapters** are `.md` with TipTap HTML (headings + paragraphs only)
- **Characters** are `.json` with `{physicalDescription, personality, traumas, relationships}`
- **Timeline events** link to character IDs and chapter filenames (not objects)
- **Notes** are freeform `.md` with a title and HTML body
- **NO edits to existing literary text** — analysis skills read, partition, catalogue, but never rewrite
