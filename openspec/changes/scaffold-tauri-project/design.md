# Design: Scaffold Tauri Project

## Technical Approach

Bootstrap via `pnpm create tauri-app@latest --template svelte-ts`, then customize. All five Rust commands live in a single `src-tauri/src/lib.rs` (~200 lines) with a shared `find_git()` helper. Error propagation uses `Result<String, String>` per the product design doc. File I/O uses `std::fs` and `serde_json`. Git operations use `std::process::Command`. No Tauri managed state is needed — all commands are stateless, operating on filesystem paths passed as parameters.

## Architecture Decisions

### Decision: Module Organization

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Single `lib.rs` | Simple, 5 commands fit in ~200 lines. Harder to scale past 10 commands. | **Chosen** |
| `commands/` modules | Cleaner separation. Over-engineered for 5 simple functions at scaffold stage. | Rejected for now |

**Rationale**: Greenfield with 5 stateless functions. Refactor into `commands/` + `git.rs` + `error.rs` when commands exceed 300 lines or gain dependencies.

### Decision: Error Propagation

| Option | Tradeoff | Decision |
|--------|----------|----------|
| `Result<String, String>` | Simple, matches Tauri IPC serialization. No typed matching. Spanish messages directly usable in UI. | **Chosen** |
| Custom error enum `impl Into<InvokeError>` | Typed errors, pattern matching. More code, requires `thiserror` or manual `Display`/`Into` impls. | Rejected |

**Rationale**: Proposal and product design explicitly specify `Result<String, String>` with Spanish messages. This is the simplest path that satisfies all specs.

### Decision: Git Execution

| Option | Tradeoff | Decision |
|--------|----------|----------|
| `std::process::Command` | Zero deps, full control over args. Platform-specific PATH handling needed. | **Chosen** |
| `git2` crate | Libgit2 bindings, no shell calls. Adds C compilation dependency, heavier build for a desktop app. | Rejected |

**Rationale**: `find_git()` already handles binary location across platforms. Only two git operations needed (`init`, `add . && commit`) — `Command` is sufficient.

### Decision: JSON Serialization

| Option | Tradeoff | Decision |
|--------|----------|----------|
| `serde` + `serde_json` | Type-safe, industry standard, validates structure. Extra deps (already planned in Cargo.toml). | **Chosen** |
| Manual string building | Zero deps. Error-prone, no structural validation. | Rejected |

**Rationale**: `serde` is already in the planned dependencies. Required for structured `metadata.json` with `project_name`, `last_modified`, `chapters_order`, `characters_index`.

### Decision: Tauri Command Signatures

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Structured params (`path: String, nombre: String`) | Type-safe, auto-deserialized by Tauri. Clear contract. | **Chosen** |
| JSON string param | Flexible. Requires manual parsing in each command. | Rejected |

**Rationale**: Specs define fixed, small parameter sets. Tauri handles deserialization automatically for structured params.

## Project Structure (Post-Scaffold)

```
cronista/
├── src/                          # Svelte frontend
│   ├── App.svelte                # 60/40 CSS Grid skeleton
│   ├── lib/
│   │   ├── tauri.ts              # Typed invoke() wrappers
│   │   └── checkpoint.ts         # Inactivity timer logic (Nivel 2)
│   ├── app.css                   # Tailwind v4 entry
│   └── vite-env.d.ts
├── src-tauri/                    # Rust backend
│   ├── Cargo.toml                # tauri 2, serde, serde_json, chrono, uuid, tempfile(dev)
│   ├── tauri.conf.json           # Window + bundle config
│   ├── capabilities/
│   │   └── default.json          # fs:allow-read, fs:allow-write scopes
│   └── src/
│       ├── main.rs               # Entry: calls lib::run()
│       └── lib.rs                # All 5 commands + find_git() + run() builder
├── package.json                  # svelte 5, tauri api/cli 2, tiptap, tailwind 4, vite 6
├── svelte.config.js
├── vite.config.ts                # Vite + Svelte + Tailwind plugin
└── tsconfig.json
```

## Data Flow

```
App.svelte ──invoke()──▶ lib.rs #[tauri::command]
                              │
                    ┌─────────┼──────────┐
                    ▼         ▼          ▼
               crear_     guardar_   cargar_
               proyecto   capitulo   indice
                    │         │          │
                    ▼         ▼          ▼
              std::fs +  std::fs   std::fs
              serde_json  write    read_to_string
                    │
                    ▼
              find_git() → inicializar_git / crear_checkpoint
                    │
                    ▼
              std::process::Command("git")
```

**Debounce flow (Nivel 1)**: TipTap `onUpdate` → 2s debounce in `App.svelte` → `invoke('guardar_capitulo', { path, filename, content })`.

**Checkpoint flow (Nivel 2)**: `checkpoint.ts` timer (30min idle, ≥100 words accumulated) → `invoke('crear_checkpoint', { path })` → `git add . && git commit -m "Progreso automático: {date} - {word_count}"`.

## Interface Contracts

```rust
#[tauri::command]
fn crear_proyecto(path: String, nombre: String) -> Result<String, String>

#[tauri::command]
fn inicializar_git(path: String) -> Result<String, String>

#[tauri::command]
fn guardar_capitulo(proyecto_path: String, filename: String, contenido: String)
    -> Result<String, String>

#[tauri::command]
fn crear_checkpoint(proyecto_path: String) -> Result<String, String>

#[tauri::command]
fn cargar_indice(proyecto_path: String) -> Result<String, String>

// Internal helper (not #[tauri::command])
fn find_git() -> Result<String, String>
```

`inicializar_git` and `crear_checkpoint` internally call `find_git()` and degrade gracefully: return `Err("Git no está disponible. El control de versiones permanecerá inactivo.")` when Git is absent. `crear_proyecto` calls `inicializar_git` after directory creation — disk structure is always created regardless of Git availability.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Each command function | `#[cfg(test)] mod tests` inline in `lib.rs`. `tempfile::TempDir` for isolated directories. |
| Unit | `find_git()` | Test on real system; mark `#[ignore]` if no git in CI. |
| Unit | JSON round-trip | Write metadata with `serde_json::to_string_pretty`, read with `fs::read_to_string`, assert equality. |
| Unit | Error paths | Permission denied (chmod 000), missing directories, empty paths, invalid UTF-8 edge cases. |
| Integration | Full flow | Single test: `crear_proyecto` → `inicializar_git` → `guardar_capitulo` → `crear_checkpoint` → `cargar_indice`. Asserts intermediate state at each step. |

All tests run via `cargo test --manifest-path src-tauri/Cargo.toml`. Frontend tests (Vitest) are out of scope for this change.

## State Management

No Tauri managed state is needed. All commands are stateless — they receive the project path as a parameter and operate on the filesystem directly. The frontend owns UI state (active chapter, word count, timer).

## Migration / Rollout

No migration required. Greenfield scaffold. Rollback: `git clean -fd` removes all scaffolded files.

## Open Questions

None. All decisions resolved by proposal, specs, and product design doc.
