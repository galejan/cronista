# Tasks: Scaffold Tauri Project

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~550 (reviewable) + ~500 (scaffold boilerplate, trusted) |
| 800-line budget risk | Low |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (Scaffold + Config) → PR 2 (Rust Backend) → PR 3 (Frontend + Tests) |
| Delivery strategy | force-chained |
| Chain strategy | stacked-to-main |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
800-line budget risk: Low

### Suggested Work Units

| Unit | Goal | Likely PR | Base | Notes |
|------|------|-----------|------|-------|
| 1 | CLI scaffold + dep config | PR 1 | main | Run `pnpm create tauri-app`, add deps, config files |
| 2 | 5 Rust commands + find_git() | PR 2 | main | `lib.rs`, `main.rs`, `Cargo.toml`, capabilities |
| 3 | App.svelte skeleton + Rust tests | PR 3 | main | 60/40 layout, tauri.ts wrappers, all unit/integration tests |

## Phase 1: Scaffold & Configuration (PR 1)

- [x] 1.1 Run `pnpm create tauri-app@latest --template svelte-ts` in repo root
- [x] 1.2 Add deps: `pnpm add -D @tailwindcss/vite tailwindcss` for Tailwind v4
- [x] 1.3 Add deps: `pnpm add @tiptap/core @tiptap/starter-kit @tiptap/extension-bubble-menu`
- [x] 1.4 Configure `vite.config.ts` with `@tailwindcss/vite` plugin and Svelte plugin
- [x] 1.5 Configure `svelte.config.js` for Svelte 5 runes mode
- [x] 1.6 Write `src/app.css` with `@import "tailwindcss"` as Tailwind v4 entry point
- [x] 1.7 Configure `src-tauri/tauri.conf.json`: window title "Cronista", size 1200x800, min 800x600
- [x] 1.8 Configure `src-tauri/capabilities/default.json`: grant `fs:allow-read` + `fs:allow-write` with `$HOME/**` scope
- [x] 1.9 Configure `src-tauri/Cargo.toml` and `src-tauri/src/lib.rs` (placeholder, real work in Phase 2)
- [x] 1.10 Verify scaffold: `pnpm install && cargo build` succeeds

## Phase 2: Rust Backend Commands (PR 2)

- [x] 2.1 Write `find_git()` helper in `src-tauri/src/lib.rs`: Linux `which git`, Windows `PATH` + `C:\Program Files\Git\bin\git.exe` fallback (specs: git-abstraction S1-S3)
- [x] 2.2 Write `crear_proyecto(path, nombre)` in `src-tauri/src/lib.rs`: create `.config/`, `capitulos/`, `personajes/`, `notas/`; seed `metadata.json` + `timeline.json` (specs: project-file-management S1-S3)
- [x] 2.3 Write `inicializar_git(path)` in `src-tauri/src/lib.rs`: call `find_git()`, run `git init`, degrade gracefully (specs: git-abstraction S4-S6)
- [x] 2.4 Write `guardar_capitulo(proyecto_path, filename, contenido)` in `src-tauri/src/lib.rs`: write `.md` to `capitulos/`, no git commit (specs: project-file-management S4-S6)
- [x] 2.5 Write `crear_checkpoint(proyecto_path)` in `src-tauri/src/lib.rs`: `git add . && git commit` with progress message (specs: git-abstraction S7-S9)
- [x] 2.6 Write `cargar_indice(proyecto_path)` in `src-tauri/src/lib.rs`: read and return `metadata.json` as string (specs: project-file-management S7-S9)
- [x] 2.7 Write `run()` builder in `src-tauri/src/lib.rs` registering all 5 commands with `tauri::Builder`
- [x] 2.8 Write `src-tauri/src/main.rs` entry point calling `lib::run()`
- [x] 2.9 Wire `crear_proyecto` to auto-call `inicializar_git` after directory creation per design contract

## Phase 3: Frontend Skeleton (PR 3)

- [x] 3.1 Write `src/App.svelte`: 60/40 CSS Grid layout (editor left 60%, sidebar right 40%)
- [x] 3.2 Write `src/lib/tauri.ts`: typed `invoke()` wrappers for all 5 commands
- [x] 3.3 Write `src/lib/checkpoint.ts`: inactivity timer skeleton (30min idle, ≥100 words — deferred to future change)

## Phase 4: Testing (PR 3)

- [x] 4.1 Write `#[cfg(test)]` tests for `find_git()`: Linux PATH found, not found, Windows fallback (mark Windows test `#[ignore]` on Linux)
- [x] 4.2 Write tests for `crear_proyecto`: valid project, permission denied (chmod 000), trailing separator
- [x] 4.3 Write tests for `inicializar_git`: git init success, git unavailable (stub PATH), already-initialized repo (reinit safe)
- [x] 4.4 Write tests for `guardar_capitulo`: new file creation, overwrite existing, UTF-8 round-trip (ñ, áéíóú, emoji, RTL)
- [x] 4.5 Write tests for `crear_checkpoint`: successful commit with hash, clean repo (no changes), git unavailable
- [x] 4.6 Write tests for `cargar_indice`: valid metadata read, missing file error, empty path error
- [x] 4.7 Write integration test: full flow `crear_proyecto` → `inicializar_git` → `guardar_capitulo` → `crear_checkpoint` → `cargar_indice`; assert intermediate state
- [x] 4.8 Run `cargo test --manifest-path src-tauri/Cargo.toml` — all 18 scenarios pass
