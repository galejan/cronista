# user-interface Specification

## Purpose

Cron-Insta's visual icon system. All UI icons MUST use Phosphor Icons v3.1.0 (SVG, `weight="light" size={16} color="currentColor"`) instead of platform-dependent emoji/unicode. Import via direct path (`phosphor-svelte/lib/IconName`) for tree-shaking.

## Requirements

### Requirement: Icon Component Substitution

All emoji/unicode icons in `src/routes/+page.svelte` and help title strings in `src/lib/i18n.svelte.ts` MUST be replaced with Phosphor Svelte components. The ⚠️ character in JS string parsing (`ckResult.includes("⚠️")`) SHALL NOT be touched — it is backend response logic, not a UI icon. Full mapping (~33 icons) in `openspec/changes/phosphor-icons/proposal.md`.

#### Scenario: Icons render identically cross-platform

- GIVEN the app runs on any supported OS
- WHEN any icon mounts
- THEN it renders as SVG with `weight="light"` and `size={16}`
- AND inherits text color via `currentColor` in both light and dark themes

#### Scenario: Backend ⚠️ parsing is immune

- GIVEN checkpoint returns `"⚠️ push failed"`
- WHEN `ckResult.includes("⚠️")` executes
- THEN the warning is detected — no Phosphor code touches this path

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

### Requirement: Content-Area Create-Button Icons

"New Chapter" and "New Note" create buttons in sidebar panels MUST display a `NotePencil` icon before the label.

#### Scenario: Create button shows icon

- GIVEN the Chapters or Notes panel is active
- WHEN its create button renders
- THEN `NotePencil` appears before the label

### Requirement: Toast Icon Support

The toast component MUST accept an optional Phosphor icon prop. Success SHALL render `CheckCircle`; warning/error SHALL render `WarningCircle`. Message strings MUST NOT embed emoji — all `"✅ " + t(...)` concatenations are replaced with the icon prop.

#### Scenario: Success toast with icon

- GIVEN `showToast` is called for success
- WHEN the toast mounts
- THEN `CheckCircle` renders left of the message — no ✅ in the string

#### Scenario: Auto-dismiss preserved

- GIVEN a toast is displayed
- WHEN 5 seconds elapse
- THEN it dismisses (existing behavior unchanged)

### Requirement: i18n Icon/Label Separation

Translation strings MUST NOT contain emoji. Icons MUST render as components in the template, separate from `t()`. Nine help title keys (editorTitle through shortcutsTitle) SHALL have emojis stripped from their values.

#### Scenario: Help section renders icon independently

- GIVEN the help panel is open
- WHEN a title renders (e.g., editorTitle)
- THEN its Phosphor icon sits left of the text
- AND `t("help.editorTitle")` returns `"Editor"` (es/en) with no emoji

#### Scenario: Language switch preserves icons

- GIVEN `setLang("en")` is called with help panel open
- WHEN sections re-render
- THEN icons stay unchanged — only the label text translates

### Requirement: Bundle Size Constraint

Direct path imports MUST be used for tree-shaking. Bundle increase SHOULD remain under 20 KB gzipped.

#### Scenario: Only imported icons bundled

- GIVEN ~33 icons imported via `phosphor-svelte/lib/{Name}`
- WHEN `pnpm build` completes
- THEN only those icons appear in output — no unused Phosphor icons bundled

### Requirement: Visual Consistency

All icons MUST use `weight="light"`, `size={16}`, `color="currentColor"`. No deviation SHALL occur. Icons align with text via `inline-flex items-center gap-1`.

#### Scenario: Uniform visual weight

- GIVEN icons in sidebar, footer, toolbar, modals, and toasts
- WHEN rendered
- THEN all share identical stroke weight and base size, aligned with adjacent text

### Requirement: Accessibility on Icon Buttons

Icon-only buttons (close, delete, navigation) MUST have `title` or `aria-label`. The SVG icon SHALL use `aria-hidden="true"`.

#### Scenario: Screen reader identifies button

- GIVEN a screen reader encounters a close button
- WHEN the button focuses
- THEN it announces its title and the icon is hidden from accessibility tree

#### Scenario: Focus ring visible

- GIVEN keyboard navigation reaches an icon button
- WHEN the button receives focus
- THEN a visible focus ring appears

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
