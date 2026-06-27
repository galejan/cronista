# Delta for user-interface

## MODIFIED Requirements

### Requirement: Sidebar Tab Icons

Sidebar tab buttons MUST display Phosphor icons: `Notebook` (Chapters), `User` (Characters), `Notepad` (Notes), `Clock` (Timeline), `MapTrifold` (Places). Tabs SHALL render inside a container with `role="tablist"`. Each tab button MUST expose `role="tab"`, `aria-selected`, and `aria-expanded`.

Tab visibility is governed by `visible_tabs` from `metadata.json`: a tab whose `visible_tabs[key]` is `false` MUST NOT render in the sidebar. The Chapters tab SHALL render unconditionally regardless of `visible_tabs.chapters` value. Hidden tabs MUST also hide their corresponding panel content.

(Previously: all five tabs always visible, no conditional rendering)

#### Scenario: Hidden tab not rendered

- GIVEN `visible_tabs.characters` is `false`
- WHEN the sidebar renders
- THEN the Characters button is absent from the tablist
- AND the Characters panel does not render

#### Scenario: Chapters always visible

- GIVEN `visible_tabs` data somehow has `chapters: false`
- WHEN the sidebar renders
- THEN the Chapters button still renders
- AND the Chapters panel is accessible

#### Scenario: All tabs visible (old project default)

- GIVEN `visible_tabs` is absent from metadata (old project)
- WHEN `#[serde(default)]` fills all `true`
- THEN all five tabs render as before this change

### Requirement: Five-Tab Keyboard Navigation

The system MUST cycle through VISIBLE tabs when the user presses Ctrl+T. The cycle SHALL skip tabs where `visible_tabs[key]` is `false`. Cycle order SHALL be the fixed canonical order: Chapters → Characters → Notes → Timeline → Places → Chapters. The Ctrl+L shortcut MUST be removed.
(Previously: always cycled all 5 tabs; no visibility filtering)

#### Scenario: Ctrl+T skips hidden tab

- GIVEN Characters is hidden (`visible_tabs.characters = false`)
- AND the active tab is Chapters (1st visible)
- WHEN the user presses Ctrl+T
- THEN the active tab becomes Notes (next visible)

#### Scenario: Ctrl+T wraps from last visible

- GIVEN Timeline and Places are hidden; only Chapters, Characters, Notes visible
- AND the active tab is Notes (last visible)
- WHEN the user presses Ctrl+T
- THEN the active tab wraps to Chapters (first visible)

## ADDED Requirements

### Requirement: Auto-Save Timer Configuration

The auto-save timer MUST derive its interval from `auto_save_interval_minutes` in metadata instead of a hardcoded value. On project load, the system SHALL read the interval and initialize the debounced save function with `interval * 60_000` milliseconds. When `auto_save_interval_minutes` changes at runtime (via config form save), the system MUST cancel the existing timer and create a new debounce instance with the updated interval. The manual "Save" button and "Guardar y subir" behavior SHALL remain unchanged.

#### Scenario: Timer uses metadata interval on load

- GIVEN a project with `auto_save_interval_minutes: 10`
- WHEN the project opens
- THEN the debounce timer is initialized with 600,000 ms (10 minutes)

#### Scenario: Timer recreates on runtime interval change

- GIVEN the timer is running with a 5-minute interval
- WHEN the user saves config with `auto_save_interval_minutes: 1`
- THEN the old timer is cancelled
- AND a new debounce instance is created with 60,000 ms (1 minute)
- AND the save status resets to idle

#### Scenario: Manual save unaffected

- GIVEN the timer interval has been changed
- WHEN the user clicks the manual Save button or "Guardar y subir"
- THEN the save executes immediately without debounce (existing behavior preserved)
