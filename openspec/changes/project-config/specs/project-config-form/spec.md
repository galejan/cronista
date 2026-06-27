# project-config-form Specification

## Purpose

Defines the unified `ProjectConfigForm` Svelte component used for both new-project creation and editing existing project config. Replaces inline wizard modals and `ProjectSettingsDialog` font/tabs/interval panels.

## Requirements

### Requirement: Dual-Mode Form

The component MUST operate in two modes: `"new"` and `"edit"`. In `"new"` mode all fields are editable; in `"edit"` mode the project name SHALL be read-only. Both modes SHALL expose: font family, visible tabs, auto-save interval.

#### Scenario: New project mode

- GIVEN mode is `"new"`
- WHEN the form mounts
- THEN project name input is editable
- AND all fields use defaults (monospace, all tabs on, 5 min)

#### Scenario: Edit mode

- GIVEN mode is `"edit"` with existing project data
- WHEN the form mounts
- THEN project name is displayed read-only
- AND fields pre-fill from metadata values

### Requirement: Step-by-Step Wizard Navigation

The form MUST present a step-by-step wizard: Project Info → Sidebar Tabs → Auto-Save → Review. Each step SHALL have a descriptive header explaining what is being configured. The user MAY skip any step; skipped steps SHALL retain defaults. A progress indicator SHALL show current/total steps.

#### Scenario: Navigate forward through steps

- GIVEN the form is on Step 1 (Project Info)
- WHEN the user clicks "Next"
- THEN Step 2 (Sidebar Tabs) displays with its header and controls

#### Scenario: Skip preserves defaults

- GIVEN the user is on Step 2 and clicks "Skip"
- WHEN the wizard advances to Step 3
- THEN all toggles remain at defaults (all tabs on)

#### Scenario: Back preserves changes

- GIVEN the user moved to Step 3 after changing interval to 10 min
- WHEN the user clicks "Back"
- THEN Step 2 displays with the 10 min selection preserved

### Requirement: Visible Tabs Step

The Sidebar Tabs step MUST display checkboxes for: Characters, Places, Timeline, Notes. The Chapters checkbox SHALL be displayed as checked and disabled (always `true`). Each checkbox MUST be labeled clearly. A descriptive header SHALL explain the purpose of hiding unused tabs.

#### Scenario: Chapters checkbox disabled

- GIVEN the Sidebar Tabs step is displayed
- WHEN the user interacts with the Chapters checkbox
- THEN it cannot be toggled — it stays checked

#### Scenario: Toggle character tab off

- GIVEN the user unchecks Characters
- WHEN the user advances to Review
- THEN the review shows Characters as disabled

### Requirement: Auto-Save Step

The Auto-Save step MUST present a radio group with options: 1 minute, 5 minutes, 10 minutes. The default SHALL be 5 minutes. A descriptive header SHALL explain the tradeoffs. The step SHALL show the selected interval clearly.

#### Scenario: Select 1 minute

- GIVEN the Auto-Save step is displayed with 5 selected
- WHEN the user selects 1 minute
- THEN the review step reflects 1 minute

### Requirement: Review and Confirm Step

The final step MUST display a summary of all selections: font, visible tabs (showing disabled ones), and auto-save interval. It SHALL provide "Confirm" and "Back" actions. On confirm, the form MUST emit the config payload via event dispatch or callback.

#### Scenario: Confirm emits payload

- GIVEN the Review step shows font=serif, tabs: [chapters, notes], interval=10
- WHEN the user clicks "Confirm"
- THEN the form emits `{ font_family: "serif", visible_tabs: {chapters:true,characters:false,places:false,timeline:false,notes:true}, auto_save_interval_minutes: 10 }`

#### Scenario: Back from review

- GIVEN the Review step is displayed
- WHEN the user clicks "Back"
- THEN the wizard returns to the Auto-Save step with selections intact
