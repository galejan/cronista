## Exploration: scaffold-tauri-project

### Current State

**Greenfield repo**. The repository contains only:
- `docs/proyecto editor.md` — Complete system design v1.1.0 (369 lines, Spanish) defining architecture, data schemas, UI modules, and testing strategy
- `README.md` — placeholder (1 line: "# cronista")
- `openspec/` — SDD config and empty specs/changes directories
- `.atl/` — Skill registry cache
- `.git/` — Fresh git repo (no meaningful commits yet)

**No code scaffolded**. No `src/`, no `src-tauri/`, no `package.json`, no `Cargo.toml`.

**Architecture defined**: Tauri (Rust) + Svelte 5/TypeScript + Tailwind CSS + TipTap editor. Local-First desktop app with embedded Git.

### Environment Assessment

| Tool | Status | Version | Notes |
|------|--------|---------|-------|
| Node.js | ✅ Installed | v20.20.2 | Via /usr/bin/node |
| npm | ✅ Installed | 11.16.0 | Via /usr/bin/npm |
| Git | ✅ Installed | 2.47.2 | At /usr/bin/git |
| **Rust/cargo** | ❌ **NOT INSTALLED** | — | `rustup` not found, no `~/.cargo` |
| **pnpm** | ❌ **NOT INSTALLED** | — | Neither global nor via corepack |
| Tauri Linux deps | ✅ Present | — | Arch: webkit2gtk-4.1, openssl 3.6.3, libayatana-appindicator, libsoup3, gtk3/gtk4 all installed |

**Platform**: EndeavourOS (Arch-based rolling). All Tauri system dependencies are already satisfied.

### Affected Areas (post-scaffold)

- `src-tauri/Cargo.toml` — Rust dependencies (tauri, serde, serde_json, uuid, chrono)
- `src-tauri/src/lib.rs` — `#[tauri::command]` functions + app builder
- `src-tauri/src/main.rs` — Entry point delegating to lib
- `src-tauri/tauri.conf.json` — App metadata, window config, build commands
- `src-tauri/capabilities/default.json` — Permission grants
- `src/App.svelte` — Minimal layout placeholder (60/40 skeleton)
- `src/main.ts` — Svelte mount + Tailwind import
- `package.json` — Frontend deps (@tauri-apps/api, @tauri-apps/cli, svelte, vite, tailwindcss, @tiptap/*)
- `vite.config.ts` — Vite + Svelte + Tauri integration
- `tailwind.config.ts` — Tailwind v4 configuration
- `svelte.config.js` — Svelte 5 compiler options

### What the Scaffold Must Generate

Running `pnpm create tauri-app@latest --template svelte-ts` produces:

```
cronista/
├── index.html                 # Vite entry HTML
├── package.json               # Frontend scripts + deps
├── pnpm-lock.yaml             # Lockfile
├── vite.config.ts             # Vite + Tauri integration
├── tsconfig.json              # TypeScript config
├── svelte.config.js           # Svelte 5 runes mode
├── src/
│   ├── App.svelte             # Root component
│   ├── main.ts                # Mount point + CSS import
│   ├── app.css                # Global styles
│   ├── vite-env.d.ts          # Vite type augmentations
│   └── lib/                   # (empty by default — we'll add here)
└── src-tauri/
    ├── Cargo.toml             # Rust manifest
    ├── build.rs               # tauri-build hook
    ├── tauri.conf.json        # Tauri configuration
    ├── capabilities/
    │   └── default.json       # Default capability grants
    ├── icons/                 # App icons (auto-generated)
    └── src/
        ├── lib.rs             # Commands + run() function
        └── main.rs            # fn main() { app::run() }
```

### Tauri Commands for This Slice

Based on `docs/proyecto editor.md` lines 296-314 ("PROMPT 1"), five commands are required:

| # | Command | Signature | Behavior |
|---|---------|-----------|----------|
| 1 | `crear_proyecto` | `(path: String, nombre: String) -> Result<String, String>` | Creates folder structure: `.config/`, `capitulos/`, `personajes/`, `notas/`. Initializes `metadata.json` (with `project_name` and empty arrays) and empty `timeline.json`. Returns project path on success. |
| 2 | `inicializar_git` | `(path: String) -> Result<String, String>` | Runs `git init` silently inside the project folder. MUST detect Git availability first — on Linux: `which git`; on Windows: PATH + fallback to `C:\Program Files\Git\bin\git.exe`. Returns error message if Git not found. |
| 3 | `guardar_capitulo` | `(proyecto_path: String, filename: String, contenido: String) -> Result<String, String>` | Writes `.md` file to `capitulos/` directory. **Nivel 1 only** — disk write, no Git commit. No word count, no timestamp in this phase. |
| 4 | `crear_checkpoint` | `(proyecto_path: String) -> Result<String, String>` | Executes `git add . && git commit -m "Progreso automático: {fecha} - {recuento}"`. **Nivel 2** — deferred, triggered by inactivity timer (not implemented yet). |
| 5 | `cargar_indice` | `(proyecto_path: String) -> Result<String, String>` | Reads and returns the full contents of `.config/metadata.json` as a JSON string. |

### Dependencies

#### Rust (src-tauri/Cargo.toml)

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"    # opens URLs/files in system default
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dev-dependencies]
tempfile = "3"
```

**Justification**:
- `tauri` v2 — latest stable (2.11.3 on crates.io). Feature flags left minimal; add `tray-icon`, `window-unstable` later.
- `tauri-plugin-opener` v2 — needed eventually for opening the project folder; included now to avoid re-scaffolding.
- `serde` + `serde_json` — JSON serialization for metadata.json and timeline.json schemas.
- `uuid` v1 with `v4` — generates UUIDs for timeline event IDs as specified in the design doc.
- `chrono` v0.4 — ISO 8601 timestamps for `last_modified` and checkpoint commit messages.
- `tempfile` v3 (dev) — isolated temp directories for unit tests, as specified in testing strategy.
- `thiserror` — NOT included in v1. The doc specifies `Result<String, String>`. Custom error types add complexity; `map_err(|e| e.to_string())` suffices for now.

#### Frontend (package.json)

```json
{
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-opener": "^2"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^5",
    "@tauri-apps/cli": "^2",
    "@tiptap/core": "^2",
    "@tiptap/extension-bubble-menu": "^2",
    "@tiptap/pm": "^2",
    "@tiptap/starter-kit": "^2",
    "@tailwindcss/vite": "^4",
    "svelte": "^5",
    "tailwindcss": "^4",
    "typescript": "~5.7",
    "vite": "^6"
  }
}
```

**Note**: These come from the scaffold template. We should verify and pin versions after scaffolding.

### Key Decisions

1. **Scaffolding strategy**: Use `pnpm create tauri-app@latest --template svelte-ts` in a temp directory, then copy files into the existing repo OR scaffold directly in the repo (Tauri's CLI handles this). The repo already has `.git/`, `docs/`, `openspec/`, `README.md` — scaffolding directly is safe because it creates only `src/`, `src-tauri/`, and config files.

2. **Rust command organization**: Follow Tauri v2 convention — `lib.rs` defines all `#[tauri::command]` functions and the `run()` builder; `main.rs` just calls `app_lib::run()`. This enables unit testing commands directly without Tauri runtime.

3. **Error handling**: Use `Result<String, String>` as the design doc specifies. Map IO errors with `.map_err(|e| e.to_string())`. Use `?` operator for propagation. This keeps the API simple and the frontend receives readable error strings.

4. **Git detection**: Implement `find_git()` helper that:
   - Linux/macOS: tries `which git` via `std::process::Command`
   - Windows: checks PATH first, then falls back to `C:\Program Files\Git\bin\git.exe` and `C:\Program Files (x86)\Git\bin\git.exe`
   - Returns `Option<std::path::PathBuf>` — `None` means Git unavailable
   - `inicializar_git` returns `Err("Git no encontrado...")` when `None`

5. **metadata.json initialization**: Not truly "empty" — seeds with:
   ```json
   {
     "project_name": "<nombre>",
     "last_modified": "<ISO 8601 now>",
     "chapters_order": [],
     "characters_index": []
   }
   ```
   Matches the schema in the design doc (lines 108-118).

6. **File naming convention**: `guardar_capitulo` receives `filename` (e.g., `"0001_prologo.md"`) and saves to `{proyecto_path}/capitulos/{filename}`. The frontend is responsible for generating the numbered prefix. This phase does NOT implement auto-numbering or reordering — that comes in a later change.

### Approaches

1. **Full scaffold via `pnpm create tauri-app`** — Run the CLI generator, then customize the output.
   - Pros: Follows official conventions, gets security/permissions right, auto-generates icons
   - Cons: Requires Rust and pnpm installed first; may need manual tweaks for Tailwind v4 + Svelte 5
   - Effort: Low (scaffolding is automated)

2. **Manual scaffold** — Create all files by hand.
   - Pros: Full control over every file, no cleanup needed
   - Cons: Error-prone, easy to miss security config (capabilities), icons need separate generation
   - Effort: Medium-High

3. **Hybrid** — Scaffold with CLI, then hand-write only the Rust commands and Svelte components.
   - Pros: Best of both — security infrastructure from template, custom logic from us
   - Cons: Two-step process
   - Effort: Medium

### Recommendation

**Approach 1 — Full scaffold via `pnpm create tauri-app@latest --template svelte-ts`**, followed by targeted modifications:
1. Install Rust via rustup + pnpm via npm
2. Scaffold directly in `/home/alex/Documentos/GitHub/cronista` (Tauri CLI handles existing dirs)
3. Post-scaffold: add Tailwind CSS v4, TipTap deps, configure Vite/Svelte for Tailwind
4. Hand-write the five Rust commands in `src-tauri/src/lib.rs`
5. Create a minimal `App.svelte` with the 60/40 layout skeleton
6. Write Rust unit tests with `tempfile`

This is the lowest-risk path because it guarantees correct Tauri v2 scaffolding (capabilities, permissions, build.rs, icons).

### Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Rust not installed** | 🔴 Critical | Must be installed first via `rustup`. Without it, nothing compiles. The scaffold `cargo` commands will fail. |
| **pnpm not installed** | 🔴 Critical | Must be installed first via `npm i -g pnpm` or `corepack enable pnpm`. The scaffold tool needs pnpm to install frontend deps. |
| **Tauri v2 vs Svelte 5 compatibility** | 🟡 Medium | Svelte 5 with runes mode is newer than some Tauri templates. The `svelte-ts` template in `create-tauri-app` v3 should handle this, but verify `svelte.config.js` enables runes mode. |
| **Tailwind CSS v4 + Vite integration** | 🟡 Medium | Tailwind v4 uses `@tailwindcss/vite` plugin instead of PostCSS config. Different from v3. The scaffold may not include this by default — manual addition needed. |
| **Git not found at runtime** | 🟡 Medium | Design doc explicitly requires graceful handling. The `find_git()` fallback for Windows paths is essential. |
| **Security: file system access** | 🟢 Low | Tauri v2 capabilities system gates fs access. The default capability must include `fs:allow-read` and `fs:allow-write` scopes for the project directory. The scaffold's default capability should be verified. |
| **Character encoding** | 🟢 Low | Spanish text (tildes, ñ, ¡¿) must survive round-trip. Rust's `std::fs::write` / `std::fs::read_to_string` handles UTF-8 by default. Tests with Spanish text are essential. |

### Ready for Proposal

**No** — Two critical blockers must be resolved before scaffolding can begin:
1. **Rust must be installed** (`rustup` + stable toolchain)
2. **pnpm must be installed** (via npm or corepack)

The orchestrator should present these prerequisites to the user and confirm they're ready to install them before proceeding to the proposal phase. Alternatively, if the user prefers, the proposal can include these as explicit setup steps in the implementation tasks.
