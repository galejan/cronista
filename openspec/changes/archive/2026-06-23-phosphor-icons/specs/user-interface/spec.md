# user-interface Specification

## Purpose

Cronista's visual icon system. All UI icons MUST use Phosphor Icons v3.1.0 (SVG, `weight="light" size={16} color="currentColor"`) instead of platform-dependent emoji/unicode. Import via direct path (`phosphor-svelte/lib/IconName`) for tree-shaking.

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

Sidebar tab buttons MUST display a Phosphor icon: `Article` (Chapters), `Users` (Characters), `StickyNote` (Notes).

#### Scenario: Tabs render with icon

- GIVEN the sidebar is visible
- WHEN tabs mount
- THEN each shows icon + translated label aligned via `inline-flex items-center gap-1`

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
