# git-abstraction Specification

## Purpose

Defines the invisible Git version control layer for Cronista projects. Handles Git binary detection across platforms, silent repository initialization, and checkpoint creation. Guarantees graceful degradation when Git is unavailable — the user is always informed, and disk save (Nivel 1) never fails due to missing Git.

## Requirements

### Requirement: Git Binary Detection

The system MUST locate the `git` executable before any Git operation.

Detection strategy:
- **Linux**: Search `PATH` via `which git`
- **Windows**: Search `PATH`, then fallback to `C:\Program Files\Git\bin\git.exe`
- On failure, return a structured error so callers can degrade gracefully

#### Scenario: Git found on Linux PATH

- GIVEN Git is installed at `/usr/bin/git`
- WHEN `find_git()` is called on Linux
- THEN the function returns `Ok("/usr/bin/git")`

#### Scenario: Git not found on Linux

- GIVEN Git is not installed or not in PATH
- WHEN `find_git()` is called on Linux
- THEN the function returns `Err("Git no encontrado...")`

#### Scenario: Git found via Windows fallback path

- GIVEN Git is installed at `C:\Program Files\Git\bin\git.exe` but not in PATH
- WHEN `find_git()` is called on Windows
- THEN the function returns `Ok("C:\\Program Files\\Git\\bin\\git.exe")`

### Requirement: Silent Git Initialization

The system SHALL initialize a Git repository silently when a new project is created.

If Git is available, `inicializar_git` runs `git init` in the project root. If Git is unavailable, the function returns an error with a clear user-facing message — disk operations (Nivel 1) continue unaffected.

#### Scenario: Git init succeeds

- GIVEN a new project directory at `/tmp/proj` with Git available
- WHEN `inicializar_git("/tmp/proj")` is called
- THEN a `.git/` directory exists under `/tmp/proj`
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

### Requirement: Checkpoint Creation

The system SHALL create Git commits as versioned snapshots (Nivel 2 — deferred, not per-keystroke).

`crear_checkpoint` runs `git add .` followed by `git commit` with a descriptive message. It is triggered by a frontend inactivity timer (≥30 min idle, ≥100 words accumulated since last checkpoint). The timer logic is a frontend concern; this command only executes the commit.

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
