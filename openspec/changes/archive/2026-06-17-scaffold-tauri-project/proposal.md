# Proposal: Scaffold Tauri Project

## Intent

Bootstrap the greenfield Tauri v2 + Svelte 5/TypeScript project for Cronista (literary editor). Establish five Rust backend commands for project/file management per `docs/proyecto editor.md`. Deliver a runnable skeleton that passes `pnpm tauri dev` with unit-tested commands.

## Scope

### In Scope

1. Scaffold via `pnpm create tauri-app@latest --template svelte-ts` in repo root
2. Post-scaffold: add Tailwind CSS v4, TipTap deps, configure Vite/Svelte
3. Five `#[tauri::command]` functions in `src-tauri/src/lib.rs`:
   - `crear_proyecto(path, nombre)` — creates `.config/`, `capitulos/`, `personajes/`, `notas/`; seeds `metadata.json` + empty `timeline.json`
   - `inicializar_git(path)` — `git init` with prior availability detection
   - `guardar_capitulo(path, filename, contenido)` — writes `.md` to disk (Nivel 1, no commit)
   - `crear_checkpoint(path)` — `git add . && git commit` (Nivel 2, callable)
   - `cargar_indice(path)` — reads and returns `metadata.json` content
4. `find_git()` helper: Linux `which git`, Windows PATH + `C:\Program Files\Git\bin\git.exe` fallback
5. Error handling: `Result<String, String>`, Spanish message when Git unavailable
6. `metadata.json` seed: `project_name`, `last_modified` (ISO 8601), empty `chapters_order`/`characters_index`
7. Minimal `App.svelte` with 60/40 CSS Grid skeleton
8. Rust unit tests for all commands using `tempfile` crate
9. Tauri v2 capability grants for filesystem read/write

### Out of Scope

- TipTap editor integration (only deps installed)
- Auto-numbering, drag-and-drop reordering, inactivity timer, word counting
- Character/timeline/note CRUD beyond folder creation
- Git branch management, build pipeline, UI beyond layout skeleton

## Capabilities

### New Capabilities
- **project-file-management**: Creates project folder structure, saves/reads chapter files and metadata.json, creates git checkpoints
- **git-abstraction**: Detects Git availability on Linux and Windows, wraps git init and commit operations transparently

### Modified Capabilities
None (greenfield project)

## Approach

**Approach 1** from exploration: scaffold via CLI, then customize. The CLI handles security/permissions scaffolding; we hand-write only the five Rust commands and `App.svelte` skeleton.

Post-scaffold: add `@tailwindcss/vite` + `@tiptap/*` deps, configure Tailwind v4 Vite plugin, write `find_git()` + commands in `lib.rs` with `Result<String, String>`, add `#[cfg(test)]` tests using tempfile, add `fs:allow-read`/`fs:allow-write` scopes in capabilities.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/Cargo.toml` | New | tauri 2, serde, serde_json, chrono, uuid, tempfile (dev) |
| `src-tauri/src/lib.rs` | New | Commands + find_git() + run() builder |
| `src-tauri/tauri.conf.json` | New | Window + bundle config |
| `src-tauri/capabilities/default.json` | New | FS permission grants |
| `package.json` | New | svelte 5, tauri api/cli 2, tiptap, tailwind 4, vite 6 |
| `vite.config.ts` | New | Vite + Svelte + Tailwind plugin |
| `src/App.svelte` | New | 60/40 CSS Grid skeleton |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Svelte 5 runes + Tauri v2 template mismatch | Medium | Verify `svelte.config.js` runes; test `pnpm tauri dev` immediately |
| Tailwind v4 Vite plugin not in scaffold | Medium | Manual addition: `pnpm add -D @tailwindcss/vite` + Vite config |
| Scaffold overwrites `docs/`/`openspec/` | Low | CLI only creates `src/`/`src-tauri/`; `git diff` after scaffold |
| Git undetected at runtime on Windows | Medium | find_git() fallback paths; Nivel 1 disk save works without Git |
| Spanish UTF-8 round-trip corruption | Low | `std::fs` defaults to UTF-8; test with ñ/accents |

## Rollback Plan

1. Scaffold failure: remove `src/`, `src-tauri/`, `package.json`, `pnpm-lock.yaml`, `node_modules/`; fix environment; re-run
2. Post-commit bug in commands: revert the commit; Rust logic isolated to `src-tauri/src/lib.rs`
3. Total reset: `git clean -fd` restores to pre-scaffold state (repo has no prior code)

## Dependencies

All installed ✓: Rust 1.96.0, pnpm 10.34.3, Node.js v20.20.2, Git 2.47.2, Tauri Linux system deps.

## Success Criteria

- [ ] `pnpm create tauri-app@latest --template svelte-ts` runs without error in repo root
- [ ] `pnpm install && cargo build` completes successfully post-scaffold
- [ ] `pnpm tauri dev` opens a window with the 60/40 layout skeleton
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes all 5 command tests
- [ ] `crear_proyecto` produces correct folders + valid metadata.json with seed values
- [ ] `inicializar_git` runs `git init` silently or returns Git-not-found error
- [ ] `guardar_capitulo` writes .md file; `cargar_indice` reads metadata.json back
- [ ] `crear_checkpoint` creates a git commit in the project repo
- [ ] `find_git()` returns correct path on Linux, falls back on Windows
