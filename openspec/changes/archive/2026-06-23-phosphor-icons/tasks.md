# Tasks: Replace Emoji/Unicode Icons with Phosphor Icons

## Review Workload Forecast

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

Estimated changed lines: ~130 (single PR, well under 400-line cap).

## Phase 1: Foundation — Package & Toast Refactor

- [x] 1.1 Add `"phosphor-svelte": "^3.1.0"` to `package.json` deps; run `pnpm install`
- [x] 1.2 Extend toast state (line 228) and `showToast` signature (line 498) in `src/routes/+page.svelte` with optional `icon?: Component`
- [x] 1.3 Update toast template (lines 2367–2376): render `toast.icon` before message; replace `×` close with `<X>` component
- [x] 1.4 Import `CheckCircle`; pass `icon: CheckCircle` on 5 `"✅ "` showToast call sites (lines 581, 768, 2029, 2334, 2353)

## Phase 2: i18n Help Title Cleanup

- [x] 2.1 Strip emoji prefix from 9 help title keys in ES dict (`src/lib/i18n.svelte.ts` lines 208–232)
- [x] 2.2 Strip emoji prefix from same 9 keys in EN dict (lines 496–520)

## Phase 3: Core Icon Replacement — +page.svelte

- [x] 3.1 Import 34 Phosphor components via direct path (includes substitutes: `Note` for missing `StickyNote`, plus `CheckCircle`, `Notebook`, `Plus`, `Package`, `ChatText`, `GitBranch`)
- [x] 3.2 Replace toolbar emojis: 📝→`NotePencil`, ✨→`Sparkle`, 📂→`FolderOpen`, 📥→`DownloadSimple`, 🗜️→`FileZip`, 📄→`FileText`, 💾→`FloppyDisk`
- [x] 3.3 Replace nav icons: →→`ArrowRight`, ←→`CaretLeft`, ▶/▼→`CaretRight`/`CaretDown`, ▼/▲ footer toggle
- [x] 3.4 Replace ×→`X` (4 sidebar deletes + toast close), ✕→`XCircle` (close project, dock close)
- [x] 3.5 Replace misc: 📌→`PushPin`, 📜→`Scroll`, ✎→`PencilSimple`, ⚠️(UI-only)→`Warning`, ?→`Question`
- [x] 3.6 Add sidebar tab icons: `Article` (Chapters), `Users` (Characters), `Note` (Notes — `StickyNote` not available in phosphor v3.1.0)
- [x] 3.7 Add create-button icons: `UserPlus` before `characters.new`, `NotePencil` before `notes.new`, `Plus` before `timeline.newEvent`
- [x] 3.8 Update 9 help `<h3>` elements to render explicit Phosphor icons before `{t(key)}`
- [x] 3.9 Apply `weight="light" size={16}` (`size={18}` for toolbar) `color="currentColor"` to every icon; add `display: flex/inline-flex; align-items: center; gap` CSS to `.btn-add` and `.footer-btn`

## Phase 4: Accessibility

- [x] 4.1 Icon-only buttons already had `title` attributes; toast close and help close added `title`
- [x] 4.2 `aria-hidden="true"` added to decorative SVG icons rendered alongside text labels (help H3, chapter nav, docked character, export modal, git sessions)

## Phase 5: Verification

- [x] 5.1 `pnpm build` — zero errors, all direct-path imports resolve
- [x] 5.2 Bundle check: main chunk ~637KB (184.75KB gzipped) — phosphor icons add ~6-8KB gzipped, within budget
- [x] 5.3 Visual: 34 icons use `currentColor` for theme inheritance; CSS `flex` alignment prevents layout shifts
- [x] 5.4 Backend ⚠️ immunity: `ckResult.includes("⚠️")` at line 420 NOT modified
- [x] 5.5 Toast: `CheckCircle` on success; auto-dismiss at 5s preserved
- [x] 5.6 i18n: `t("help.editorTitle")` returns emoji-free label; language switch preserves icons

## Dependencies

- Phase 1 → Phase 3 (imports needed before icon replacement)
- Phase 2 → Phase 3.8 (clean i18n strings needed for help template)
- Phase 4 (accessibility) independent of Phase 2; can parallelize
- Phase 5 after all others
