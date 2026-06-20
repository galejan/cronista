# git-remote-sync Specification

## Purpose

Manages optional SSH-only Git remote configuration and checkpoint auto-push. Includes failure tracking with a 3-strike auto-disable rule and a toolbar re-enable flow.

## Requirements

### Requirement: SSH-Only Remote Validation

The system MUST reject remote URLs that do not use the SSH protocol.

Valid URLs SHALL match the pattern `git@` (implicit SSH). HTTPS and http:// URLs MUST be rejected with a clear user-facing message in the current UI language.

#### Scenario: Valid SSH URL
- GIVEN the user provides `git@github.com:user/repo.git`
- WHEN the URL is validated
- THEN validation SHALL pass and the remote SHALL be configured

#### Scenario: HTTPS URL rejected
- GIVEN the user provides `https://github.com/user/repo.git`
- WHEN the URL is validated
- THEN validation MUST fail with message: "Only SSH remotes are supported (e.g. git@github.com:user/repo.git)"

### Requirement: Remote Configuration and Initial Push

The system SHALL configure the Git remote and perform an initial push when the user provides a valid SSH URL during project creation.

On valid URL: `git remote add origin <url>` followed by `git push -u origin main`. Success sets `push_enabled: true` in global config.

#### Scenario: Remote added and pushed successfully
- GIVEN a new project with valid SSH URL and accessible remote
- WHEN the user confirms identity and remote in the dialog
- THEN `origin` remote SHALL be configured
- AND an initial push SHALL succeed
- AND `push_enabled` SHALL be set to `true`

#### Scenario: Remote not accessible on first push
- GIVEN a valid SSH URL but the remote is unreachable
- WHEN the initial push is attempted
- THEN the commit SHALL remain local
- AND the user SHALL receive a non-blocking warning
- AND `push_enabled` SHALL remain `true` (first attempt is not a strike)

### Requirement: Checkpoint Auto-Push

When `push_enabled: true`, `crear_checkpoint` SHALL attempt `git push origin main` after each successful commit.

Push failures SHALL increment a consecutive-failure counter. After 3 consecutive failures, the system MUST set `push_enabled: false` and notify the user. A successful push SHALL reset the counter to 0.

#### Scenario: Push succeeds silently
- GIVEN `push_enabled: true` and a successful checkpoint commit
- WHEN the auto-push executes
- THEN no user notification SHALL appear

#### Scenario: First push failure (strike 1)
- GIVEN `push_enabled: true` and remote inaccessible
- WHEN auto-push fails after a checkpoint
- THEN a warning toast SHALL appear
- AND the failure counter SHALL increment to 1

#### Scenario: Third consecutive failure → disable
- GIVEN `push_enabled: true` and 2 prior consecutive push failures
- WHEN auto-push fails a third time
- THEN `push_enabled` SHALL be set to `false`
- AND the user SHALL be notified: "Remote push has been disabled after 3 failures."

#### Scenario: Success resets counter
- GIVEN 1 or 2 prior consecutive push failures
- WHEN the next auto-push succeeds
- THEN the failure counter SHALL reset to 0

### Requirement: Toolbar Warning and Re-enable Flow

The system SHALL display a warning indicator (⚠️ icon) in the toolbar only when a remote WAS previously configured but `push_enabled` is `false`.

Users who never configured a remote SHALL see no warning. Re-enabling SHALL offer retry sync or remote reconfigure; on re-enable, the failure counter MUST reset to 0.

#### Scenario: Warning shown for disabled remote
- GIVEN a project with a configured remote and `push_enabled: false`
- WHEN the toolbar renders
- THEN a ⚠️ icon SHALL be visible

#### Scenario: No warning without remote config
- GIVEN a project where no remote was ever configured
- WHEN the toolbar renders
- THEN no warning icon SHALL appear

#### Scenario: User re-enables push
- GIVEN `push_enabled: false` after 3 consecutive failures
- WHEN the user clicks the warning icon and chooses "Retry Sync"
- THEN `push_enabled` SHALL be set to `true`
- AND the failure counter SHALL reset to 0
