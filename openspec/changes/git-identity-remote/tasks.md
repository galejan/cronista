# Tasks: Git Identity & Remote Sync

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~900–1000 |
| 400-line budget risk | High |
| 800-line budget risk | High |
| Chained PRs recommended | Yes |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: High
800-line budget risk: High

### Suggested Work Units

| Unit | Goal | PR | Notes |
|------|------|-----|-------|
| 1 | Backend foundation | 1 (~250L) | Data structs, identity/remote CRUD, invoke_handler, config tests |
| 2 | Push logic + modified commands | 2 (~350L) | Sync helper, auto-push in checkpoint, modify init, push tests |
| 3 | Frontend dialog + toolbar | 3 (~350L) | Dialog component, tauri.ts wrappers, page wiring, i18n |

## Phase 1: Backend — Identity Config

- [x] 1.1 Add `GitIdentity`, `GitRemoteConfig`, `GitConfig` structs with serde in `src-tauri/src/lib.rs`
- [x] 1.2 Implement `cargar_identidad_git()` — read `app_config_dir()/cronista/git-config.json`, returns null on missing/corrupt (frontend handles presets per design)
- [x] 1.3 Implement `guardar_identidad_git(name, email)` — write identity to global config with `schema_version: 1`, read-modify-write preserves remote
- [x] 1.4 Unit tests: serialization roundtrip, corrupted file fallback, missing file → null, read-modify-write, unicode names

## Phase 2: Backend — Remote Config & Sync

- [x] 2.1 Implement `cargar_config_remoto()` — read remote section from config, return `{url, push_enabled, failures}` or null
- [x] 2.2 Implement `guardar_config_remoto(url, push_enabled)` — write remote config; preserves identity (read-modify-write); consecutive_failures initialised to 0
- [x] 2.3 Implement `configurar_remoto(path, url)` — `git remote add origin` + `git push -u origin main` in project repo
- [x] 2.4 Implement `sincronizar_checkpoint(path)` helper — push if enabled, 3-strike increment/disable, save config
- [x] 2.5 Implement `reintentar_push(path)` — run git push, reset counter on success
- [x] 2.6 Register all 6 new commands in `invoke_handler![]`
- [x] 2.7 Unit tests: SSH rejection, remote config roundtrip, 3-strike disable, counter reset on success

## Phase 3: Backend — Modified Commands

- [x] 3.1 Extend `crear_checkpoint` — after commit, call `sincronizar_checkpoint`; return warning on push failure
- [x] 3.2 Modify `inicializar_git` — read identity from global config (Cervantes/Shakespeare fallback) instead of hardcoded "Cronista"
- [x] 3.3 Update existing tests for both commands — verify no-push when unconfigured, identity from config, preset fallback

## Phase 4: Frontend — GitIdentityDialog Component

- [ ] 4.1 Create `src/lib/components/GitIdentityDialog.svelte` — identity inputs (pre-filled), collapsible remote section with SSH validation, Save/Skip buttons
- [ ] 4.2 Add TypeScript wrappers in `src/lib/tauri.ts` — 6 new functions matching backend command names
- [ ] 4.3 Wire dialog: mount → `cargarIdentidadGit`, save → `guardarIdentidadGit` + optional remote config/push

## Phase 5: Frontend — Toolbar & Integration

- [ ] 5.1 Replace `inicializar_git_con_autor()` call in `+page.svelte` with `<GitIdentityDialog>` triggered on project creation
- [ ] 5.2 Add ⚠️ toolbar indicator — show when `push_enabled=false` AND remote was configured; retry/reconfigure mini-dialog
- [ ] 5.3 Wire `crear_checkpoint` push-failure warnings to toast notification
- [ ] 5.4 Add ~12 i18n keys in `src/lib/i18n.svelte.ts` — `git.identity*`, `git.remote*`, `git.push*`, `git.toolbar*` in ES and EN
