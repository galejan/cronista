# Proposal: Git Identity & Remote Sync

## Intent

Replace the silent, hardcoded Git identity (Cronista / cronista@local) and per-project author config with a user-friendly, persistent identity flow — plus optional SSH-only remote sync with auto-push on checkpoints. Eliminate manual `inicializar_git_con_autor()` calls by integrating identity into project creation.

## Scope

### In Scope
- Single-dialog identity collection on project creation with global config persistence (`~/.config/cronista/git-config.json`)
- Literary fallback presets by UI language (Cervantes ES / Shakespeare EN)
- Optional SSH-only remote sync within same dialog (reject HTTPS)
- Auto-push on checkpoints when `push_enabled: true`
- 3-strike push failure tracking → auto-disable
- Toolbar warning indicator (⚠️) for disabled-but-configured remotes with retry/reconfigure

### Out of Scope
- HTTPS remote support
- SSH key management (user must configure externally)
- Multi-step wizard UI
- `inicializar_git_con_autor()` — deprecated/replaced by new flow

## Capabilities

### New Capabilities
- `git-identity-config`: Global Git identity management via platform app config dir with language-aware presets
- `git-remote-sync`: SSH-only remote configuration, checkpoint auto-push, failure tracking, and toolbar re-enable flow

### Modified Capabilities
- `git-abstraction`: `crear_checkpoint()` gains conditional auto-push; `inicializar_git()` gains identity detection and dialog trigger

## Approach

1. **Backend**: Add `save_git_identity()` / `load_git_identity()` / `save_remote_config()` / `load_remote_config()` commands using Tauri's `app.path().app_config_dir()` → `cronista/git-config.json`. Add `git_push()` command wrapping `git push -u origin main`. Extend `crear_checkpoint()` with post-commit push logic and 3-strike counter.
2. **Frontend**: Replace `inicializar_git_con_autor()` callsite in `+page.svelte` with a new `GitIdentityDialog.svelte` component. Add toolbar indicator to existing Git history button. Wire `crear_checkpoint` to surface push-failure warnings via toast/notification.
3. **i18n**: Extend `src/lib/i18n.svelte.ts` with dialog strings, error messages, and toolbar labels.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/lib.rs` | Modified | New commands: identity/remote config CRUD, `git_push`. Extend `crear_checkpoint` with auto-push + failure tracking. |
| `src/lib/components/GitIdentityDialog.svelte` | New | Single-dialog component: identity + optional remote section |
| `src/routes/+page.svelte` | Modified | Replace `inicializar_git_con_autor()` call, add toolbar warning indicator |
| `src/lib/i18n.svelte.ts` | Modified | New translation keys for dialog, errors, toolbar |
| `~/.config/cronista/git-config.json` | New | Persistent global config (platform-path via Tauri API) |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Push failures cause UX noise on every checkpoint | Med | 3-strike rule + non-blocking toast; silent success |
| SSH not configured → push always fails | High | Clear pre-condition messaging in dialog; graceful degradation |
| Legacy `git-config.json` format conflicts | Low | Schema version field; migrate on read |

## Rollback Plan

Revert to `inicializar_git_con_autor()` per-project flow: remove `GitIdentityDialog`, delete global config read, and strip auto-push from `crear_checkpoint`. The global config file is purely additive — deleting it restores old behavior.

## Dependencies

- Tauri `app.path()` API (already available)
- `tauri-plugin-dialog` (already registered)
- No new Rust crates or npm packages required

## Success Criteria

- [ ] New project creation shows unified identity dialog with language-aware presets
- [ ] Identity and remote config persist across projects via global config
- [ ] Checkpoints auto-push when remote is configured and accessible
- [ ] 3 consecutive push failures disable push with user notification
- [ ] Toolbar shows warning only when remote WAS configured (not for local-only users)
- [ ] `cargo test` — all 50 existing Rust tests still pass; new tests cover identity/remote config and push failure logic
