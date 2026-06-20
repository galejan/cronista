# Design: Git Identity & Remote Sync

## Technical Approach

Replace the hardcoded "Cronista" Git identity and per-project author config with a persistent, cross-project identity stored in Tauri's `app_config_dir`. Add optional SSH-only remote sync with auto-push on checkpoints, governed by a 3-strike failure policy. The identity dialog is a single reusable Svelte component that replaces the inline Git init modal in `+page.svelte`.

## Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Config storage | Global `app_config_dir()/cronista/git-config.json` | Cross-project reuse; single source of truth; `app_config_dir` is Tauri's platform-standard config path |
| Remote transport | SSH only (reject `https://`) | HTTPS requires credential management (credential helper, token storage) — out of scope; SSH key setup is a one-time user responsibility documented in dialog |
| UI flow | Single dialog (not wizard) | Only 2 optional sections (identity required, remote optional); wizard overkill for 2-3 fields |
| Failure policy | 3 consecutive failures → auto-disable | Balances user awareness with noise: transient network issues are tolerated, persistent failure disables non-intrusively |
| Warning indicator | Toolbar icon (⚠️) on Git status row | Non-intrusive, contextual — only shown when remote WAS configured, doesn't disrupt writing workflow |

## Data Flow

### 1. Project Creation with Git Identity

```mermaid
sequenceDiagram
    participant User
    participant Page as +page.svelte
    participant Dialog as GitIdentityDialog
    participant Tauri as Rust Backend
    participant FS as ~/.config/cronista/

    User->>Page: Create/Open project
    Page->>Tauri: detectar_git()
    Tauri-->>Page: true
    Page->>Dialog: open={true}, projectPath
    Dialog->>Tauri: cargar_identidad_git()
    Tauri->>FS: read git-config.json
    Tauri-->>Dialog: {name, email} | null
    Dialog->>User: Show pre-filled form
    User->>Dialog: Fill identity + optional remote URL, click Save
    Dialog->>Tauri: guardar_identidad_git(name, email)
    Tauri->>FS: write git-config.json
    alt remote configured
        Dialog->>Tauri: guardar_config_remoto(url, true)
        Dialog->>Tauri: configurar_remoto(projectPath, url)
        Tauri->>Tauri: git init + git remote add origin + push -u
        Tauri-->>Dialog: success
    end
    Dialog-->>Page: onComplete()
    Page->>Tauri: actualizarGitStatus()
```

### 2. Auto-Push on Checkpoint

```mermaid
sequenceDiagram
    participant App as App (save/close)
    participant CK as crear_checkpoint
    participant Sync as sincronizar_checkpoint
    participant FS as git-config.json
    participant Git as git push

    App->>CK: crear_checkpoint(proyecto_path)
    CK->>CK: git add . + git commit
    CK->>Sync: sincronizar_checkpoint(proyecto_path)
    Sync->>FS: read remote config
    alt push_enabled = false or no remote_url
        Sync-->>CK: Ok (no-op)
    else push_enabled = true
        Sync->>Git: git push
        alt success
            Sync->>FS: consecutive_failures = 0
            Sync-->>CK: Ok
        else failure
            Sync->>FS: consecutive_failures += 1
            alt consecutive_failures >= 3
                Sync->>FS: push_enabled = false
                Sync-->>CK: warning "disabled after 3 failures"
            else < 3
                Sync-->>CK: warning "push failed (attempt N/3)"
            end
        end
    end
    CK-->>App: Result (commit hash or warning)
```

### 3. 3-Strike Disable & Recovery

```mermaid
sequenceDiagram
    participant User
    participant Toolbar as +page.svelte
    participant Tauri as Rust Backend

    Note over Toolbar: push failed 3 times, push_enabled=false
    Toolbar->>Tauri: cargar_config_remoto()
    Tauri-->>Toolbar: {push_enabled: false, url: "git@..."}
    Toolbar->>User: Show ⚠️ indicator on Git row
    User->>Toolbar: Click ⚠️
    Toolbar->>User: Mini-dialog: "Retry sync" / "Reconfigure remote"
    User->>Toolbar: Click "Retry sync"
    Toolbar->>Tauri: reintentar_push(projectPath)
    Tauri->>Tauri: git push
    alt success
        Tauri->>FS: consecutive_failures = 0
        Tauri-->>Toolbar: success
        Toolbar->>User: Remove ⚠️ indicator
    else failure
        Tauri-->>Toolbar: error
        Toolbar->>User: Show toast "push failed"
    end
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/lib.rs` | Modify | Add 6 Tauri commands (`cargar_identidad_git`, `guardar_identidad_git`, `cargar_config_remoto`, `guardar_config_remoto`, `configurar_remoto`, `reintentar_push`); add internal helper `sincronizar_checkpoint`; extend `crear_checkpoint` to call sync; extend `inicializar_git` with optional identity params; register commands in `invoke_handler![]` |
| `src/lib/components/GitIdentityDialog.svelte` | Create | Single-dialog component: identity fields + optional remote section with SSH validation, calls `cargar_identidad_git` on mount for pre-fill, saves via `guardar_identidad_git` + `guardar_config_remoto` + `configurar_remoto` |
| `src/routes/+page.svelte` | Modify | Replace inline git init modal (lines 1743-1797) with `<GitIdentityDialog>`; add `gitRemoteWarning` state + ⚠️ toolbar indicator on git status row; add retry/reconfigure mini-dialog on ⚠️ click; surface push-failure warnings from checkpoint responses |
| `src/lib/tauri.ts` | Modify | Add TypeScript wrappers: `cargarIdentidadGit`, `guardarIdentidadGit`, `cargarConfigRemoto`, `guardarConfigRemoto`, `configurarRemoto`, `reintentarPush` |
| `src/lib/i18n.svelte.ts` | Modify | Add ~12 new keys: `git.identity*`, `git.remote*`, `git.push*`, `git.toolbar*` in both ES and EN blocks |

## Global Config Schema

```json
{
  "schema_version": 1,
  "identity": { "name": "Miguel de Cervantes", "email": "cervantes@literatura.es" },
  "remote": { "url": "git@github.com:user/repo.git", "push_enabled": true, "consecutive_failures": 0 }
}
```

Path: `{app_config_dir}/cronista/git-config.json`. Created on first save, read on load. `schema_version` enables future migration.

## 3-Strike Logic (Rust internal)

`fn sincronizar_checkpoint(project_path: &str) -> Result<String, String>` (internal helper, NOT a Tauri command):

1. Read `git-config.json`. If `!push_enabled` or no `url` → return `Ok` (no-op).
2. Run `git push`. On success → set `consecutive_failures = 0`, save config, return `Ok`.
3. On failure → increment `consecutive_failures`. If `>= 3` → set `push_enabled = false`, save config, return warning string.
4. Otherwise return warning with attempt count.

`reintentar_push` (Tauri command): runs `git push`, resets counter to 0 on success.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Rust unit | `sincronizar_checkpoint` counter logic, config read/write | `#[cfg(test)]` with temp dir for config file; mock `system_command` or use real git in temp repo |
| Rust unit | Config schema serialization/deserialization | Serde round-trip tests |
| Rust unit | `guardar_identidad_git` / `cargar_identidad_git` | Temp dir + write + read back |
| Rust unit | `guardar_config_remoto` / `cargar_config_remoto` | Temp dir + write + read back |
| Rust integration | `configurar_remoto` with temp git repo | Real `git init` in temp dir, verify `git remote -v` output |
| Frontend unit | `GitIdentityDialog` component rendering | Vitest + Svelte Testing Library: mount component, verify inputs render, verify SSH rejection message |
| E2E | Full identity+remote flow | Manual: create project → fill dialog → verify `git-config.json` → verify `git log` author |

## Migration / Rollout

No data migration required — the global config file is purely additive. If the file does not exist, `cargar_identidad_git` returns `null` and the dialog shows empty fields with language-aware presets (`git.defaultName` / `git.defaultEmail`). To roll back, delete `git-config.json` and revert to `inicializar_git_con_autor` flow.

## Open Questions

- [ ] Should `sincronizar_checkpoint` run in a tokio task to avoid blocking the close handler? (Current `do_checkpoint` is sync in `tauri::async_runtime::spawn` — same pattern likely fine.)
- [ ] Confirm `app_config_dir` resolves correctly on all three platforms in Tauri v2 context (tested only on Linux so far).
