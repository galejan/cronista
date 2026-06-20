# Delta for git-abstraction

## ADDED Requirements

### Requirement: Checkpoint with Auto-Push

The system SHALL attempt `git push origin main` after a successful checkpoint commit when `push_enabled: true` and a remote is configured.

Push outcome SHALL be tracked via a consecutive-failure counter. After 3 consecutive failures, `push_enabled` SHALL be set to `false`. (Full failure-tracking rules in `git-remote-sync` spec.)

#### Scenario: Checkpoint with push_enabled=true and accessible remote
- GIVEN `push_enabled: true` and a reachable remote
- WHEN `crear_checkpoint` commits successfully
- THEN `git push origin main` SHALL execute and succeed silently

#### Scenario: Checkpoint with push_enabled=true but push fails
- GIVEN `push_enabled: true` and an unreachable remote
- WHEN `crear_checkpoint` commits successfully then push fails
- THEN the local commit SHALL remain intact
- AND the failure SHALL be tracked (counter incremented per git-remote-sync rules)

#### Scenario: Checkpoint with push_enabled=false
- GIVEN `push_enabled: false`
- WHEN `crear_checkpoint` commits successfully
- THEN no push SHALL be attempted

#### Scenario: Checkpoint when remote was never configured
- GIVEN no remote URL was ever configured for the project
- WHEN `crear_checkpoint` commits successfully
- THEN no push SHALL be attempted

## MODIFIED Requirements

### Requirement: Silent Git Initialization

The system SHALL initialize a Git repository silently when a new project is created.

If Git is available, `inicializar_git` runs `git init` in the project root and configures `user.name` and `user.email` from the global identity config. If no global config exists, language-aware presets SHALL be used (Cervantes ES / Shakespeare EN). If Git is unavailable, the function returns an error with a clear user-facing message — disk operations (Nivel 1) continue unaffected.

(Previously: `inicializar_git` hardcoded identity as "Cronista" / "cronista@local".)

#### Scenario: Git init succeeds
- GIVEN a new project directory at `/tmp/proj` with Git available
- WHEN `inicializar_git("/tmp/proj")` is called
- THEN a `.git/` directory exists under `/tmp/proj`
- AND `git config user.name` and `git config user.email` match the global identity config
- AND the function returns `Ok` with a success message

#### Scenario: Git unavailable — graceful degradation
- GIVEN a new project directory at `/tmp/proj` with no Git executable found
- WHEN `inicializar_git("/tmp/proj")` is called
- THEN the function returns `Err` with message: "Git no está disponible. El control de versiones permanecerá inactivo."
- AND the project directory remains intact (disk save works)
- AND no panic or crash occurs

#### Scenario: Git init on already-initialized repo
- GIVEN a project directory that already contains `.git/`
- WHEN `inicializar_git` is called
- THEN the function succeeds silently (reinit is safe)
- AND existing identity config SHALL be preserved (not overwritten)

### Requirement: Checkpoint Creation

The system SHALL create Git commits as versioned snapshots (Nivel 2 — deferred, not per-keystroke).

`crear_checkpoint` runs `git add .` followed by `git commit` with a descriptive message. When `push_enabled: true` and a remote is configured, it SHALL attempt `git push origin main` after successful commit. It is triggered by a frontend inactivity timer (≥30 min idle, ≥100 words since last checkpoint). Timer logic is a frontend concern; this command only executes commit and conditional push.

(Previously: `crear_checkpoint` did not attempt any push.)

#### Scenario: Creates a checkpoint commit
- GIVEN a project with `.git/` initialized and modified chapter files
- WHEN `crear_checkpoint("/tmp/proj")` is called
- THEN all changes are staged and committed
- AND the commit message follows format: "Progreso automático: [date] - [word count]"
- AND the function returns `Ok` with the commit hash

#### Scenario: No changes to commit
- GIVEN a clean Git repo with no modified files
- WHEN `crear_checkpoint` is called
- THEN the function returns `Ok` with a message indicating no changes were committed
- AND no empty commit is created

#### Scenario: Checkpoint when Git unavailable
- GIVEN a project where Git is not available
- WHEN `crear_checkpoint` is called
- THEN the function returns `Err` with "Git no está disponible"
- AND disk files remain unaffected
