# Proposal: Replace Emoji/Unicode Icons with Phosphor Icons

## Intent

Cronista currently uses raw emoji and unicode characters (📝, ×, ▶, ←, ✅, etc.) as UI icons.
These render inconsistently across platforms (Windows/Linux/macOS), lack a unified visual
weight, and look amateurish next to a professional desktop writing app. Phosphor Icons provide
a cohesive, vector-based icon set with consistent stroke weight across all ~25 replacements.

## Scope

### In Scope

- Replace ~25 emoji/unicode icons in `src/routes/+page.svelte` with Phosphor Svelte components
- Replace ~7 emoji icons in `src/lib/i18n.svelte.ts` help section titles
- Add sidebar tab icons (`Article`, `Users`, `StickyNote`) — currently no icons
- Add content-area create-button icons (`UserPlus`, `NotePencil`) — currently no icons
- Install `phosphor-svelte` v3.1.0 via pnpm (`weight="light"`, `size={16}`, `color="currentColor"`)
- Use direct import pattern: `import { IconName } from "phosphor-svelte/lib/IconName"` for tree-shaking
- Replace ✅ in JS string concatenations (toast messages) with component-based approach

### Out of Scope

- Changing icon positioning, layout, or button structure
- Modifying UI behavior, event handlers, or component logic
- Replacing git status indicators (🟢🟠🔴) — these are colored semantic dots, not icons
- Replacing theme toggle (🌙/☀️) — these are contextual symbols, not structural icons
- The ⚠️ used in JS logic (`ckResult.includes("⚠️")`) — backend string check, must not change

## Capabilities

### New Capabilities

None — this is a pure visual/icon replacement, not a new user-facing capability.

### Modified Capabilities

None — no spec-level behavior changes. Existing specs (editor-integration, git-abstraction,
project-file-management, git-identity-config, git-remote-sync) are unaffected.

## Approach

**Direct component substitution** — each emoji/unicode character is replaced with its
corresponding Phosphor Svelte component using consistent props. The import pattern uses
individual path imports (`phosphor-svelte/lib/IconName`) for maximum tree-shaking.

**i18n titles**: Help section headers currently embed emojis in translation strings
(e.g., `"help.editorTitle": "📖 Editor"`). These are split — icon component renders
separately from the translated label — so translations remain clean and the icon
is a Svelte component in the template.

**Toast messages**: ✅ currently appears in JS string concatenation (`"✅ " + t(...)`).
The toast component is refactored to accept an optional icon prop (Phosphor component)
instead of embedding the emoji in the message string.

**Tailwind alignment**: `inline-flex items-center gap-1` classes ensure icons align
vertically with adjacent text, matching the existing visual rhythm.

### Additional Unicode Icons Discovered

Beyond the confirmed mapping, these were found and are included:
- ✨ (New project, line 1925) → `Sparkle`
- → (View sessions link, line 1988) → `ArrowRight`
- 📜 (Git sessions title, line 2486) → `Scroll`

These bring the total to ~28 replacements + 5 new icon additions = ~33 icons.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src/routes/+page.svelte` | Modified | All emoji/unicode icons in template (~25 locations) replaced |
| `src/lib/i18n.svelte.ts` | Modified | Emojis removed from ~7 help title strings; icons rendered in template |
| `src/lib/components/` | New | `Toast.svelte` extracted (or inline refactored) to accept icon prop |
| `package.json` | Modified | Add `phosphor-svelte` dependency |
| `pnpm-lock.yaml` | Modified | Lockfile updated |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Bundle size increase from icon library | Low | Direct path imports enable tree-shaking; only imported icons are bundled |
| Visual regression (different size/weight) | Med | Consistent `weight="light" size={16}` props; Tailwind alignment classes |
| Phosphor icon name mismatch with code | Low | Each icon verified against Phosphor v3.1.0 catalog before commit |
| Breaking JS string logic with ⚠️ replacement | Low | ⚠️ in backend response parsing (`includes("⚠️")`) explicitly excluded |

## Rollback Plan

1. `pnpm remove phosphor-svelte`
2. `git revert` the commit(s) — all changes are isolated to icon replacement
3. No data migration, no backend changes, no persisted state affected

## Dependencies

- `phosphor-svelte` v3.1.0 (npm package, installed via pnpm)
- No backend (Rust) changes required

## Success Criteria

- [ ] All ~28 emoji/unicode icons in +page.svelte replaced with Phosphor components
- [ ] All 7 help-section emojis in i18n.svelte.ts removed; icons rendered in template
- [ ] 5 new icons added (sidebar tabs + create buttons)
- [ ] `pnpm build` succeeds with no errors
- [ ] Icons render consistently on Linux (primary dev platform)
- [ ] Dark/light themes work via `currentColor` inheritance
- [ ] Toast messages render ✅ checkmark via component, not JS string
- [ ] No visual layout shifts in sidebar, footer, toolbar, or editor
