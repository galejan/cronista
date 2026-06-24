# Delta for user-interface

## MODIFIED Requirements

### Requirement: Sidebar Tab Icons

Sidebar tab buttons MUST display Phosphor icons: `Notebook` (Chapters), `User` (Characters), `Notepad` (Notes), `Clock` (Timeline), `MapTrifold` (Places). Five tabs SHALL render inside a container with `role="tablist"`. Each tab button MUST expose `role="tab"`, `aria-selected`, and `aria-expanded`.  
(Previously: 3 tabs with Article, Users, StickyNote)

#### Scenario: Five tabs render with correct icons

- GIVEN the sidebar is visible
- WHEN tabs mount
- THEN all 5 tabs display their assigned Phosphor icon + translated label via `inline-flex items-center gap-1`
- AND the Timeline tab replaces the former fixed collapsible footer section

#### Scenario: ARIA attributes reflect tab state

- GIVEN the sidebar tablist is rendered
- WHEN the user selects tab "Lugares"
- THEN that tab sets `aria-selected="true"` and all others set `aria-selected="false"`
- AND inactive tabs set `aria-expanded="false"`

## ADDED Requirements

### Requirement: Five-Tab Keyboard Navigation

The system MUST cycle through all 5 tabs when the user presses Ctrl+T. The cycle order SHALL be: Chapters → Characters → Notes → Timeline → Places → Chapters. The Ctrl+L shortcut MUST be removed.

#### Scenario: Ctrl+T advances to next tab

- GIVEN the active tab is "Notas" (3rd)
- WHEN the user presses Ctrl+T
- THEN the active tab becomes "Timeline" (4th)
- AND the panel content switches accordingly

#### Scenario: Ctrl+T wraps from last to first

- GIVEN the active tab is "Lugares" (5th)
- WHEN the user presses Ctrl+T
- THEN the active tab becomes "Capítulos" (1st)

#### Scenario: Ctrl+L has no effect

- GIVEN the sidebar is visible
- WHEN the user presses Ctrl+L
- THEN no navigation action occurs

### Requirement: Timeline as Peer Tab

The timeline section MUST migrate from a fixed collapsible footer to the 4th tab panel. Its content (event list, drag-and-drop, badge count) SHALL render identically within the tab panel markup. The sidebar footer SHALL NOT contain a separate timeline section.

#### Scenario: Timeline renders inside its tab panel

- GIVEN the user selects the Timeline tab (4th)
- WHEN the tab panel mounts
- THEN events render with create, edit, delete, and drag-and-drop actions unchanged
- AND the event-count badge is visible on the tab button

#### Scenario: No timeline section in sidebar footer

- GIVEN the sidebar is rendered
- WHEN inspecting the footer area
- THEN no `.timeline-section` collapsible element exists

### Requirement: Places Tab Panel

The system MUST provide a 5th tab "Lugares" with inline CRUD form identical in pattern to the Characters tab. It SHALL list places with edit and delete actions, show a creation form, and persist changes via IPC commands to `lugares/`.

#### Scenario: Places panel lists existing places

- GIVEN a project with places in `lugares/`
- WHEN the user selects the Places tab
- THEN a scrollable list of places renders with name + truncated description
- AND each item shows edit and delete buttons

#### Scenario: Inline form creates a new place

- GIVEN the Places tab is active
- WHEN the user fills name and description in the inline form and submits
- THEN `crear_lugar` is invoked via IPC
- AND the new place appears in the list without page reload

### Requirement: Help Panel Updates

The help panel MUST reflect the new 5-tab navigation: Ctrl+T description updated to mention 5 tabs, Ctrl+L removed from shortcuts list, and a new "Lugares" section added with its keybindings and usage.

#### Scenario: Help panel shows updated shortcuts

- GIVEN the help panel is open
- WHEN the user reads the shortcuts section
- THEN Ctrl+T is described as cycling 5 tabs
- AND Ctrl+L does not appear

#### Scenario: Help panel includes Places section

- GIVEN the help panel is open
- WHEN the user scrolls to entity sections
- THEN a "Lugares" section exists with its keybindings
