# Design: Replace Emoji/Unicode Icons with Phosphor Icons

## Technical Approach

Direct component substitution: each emoji/unicode character in `+page.svelte` becomes a Phosphor Svelte component imported individually from `phosphor-svelte/lib/{Name}`. Help title strings in `i18n.svelte.ts` are stripped of emoji prefixes; icons render in the template via a `helpIcons` mapping. The toast is refactored to accept an optional `icon` Svelte component prop, eliminating `"✅ " + t(...)` string concatenation. All icons use `weight="light" size={16} color="currentColor"` wrapped in `inline-flex items-center gap-1`.

## Architecture Decisions

| Decision | Options | Choice | Rationale |
|----------|---------|--------|-----------|
| Import strategy | Barrel (`phosphor-svelte`) vs direct path (`phosphor-svelte/lib/{Name}`) | Direct path | Tree-shaking with barrel is unreliable in Vite; direct path guarantees only imported SVGs land in bundle |
| Icon prop on toast | Boolean `success` flag vs component ref vs Phosphor name string | Component ref (`icon?: ComponentType`) | Most flexible — lets callers pass any Phosphor component; no string-to-component mapping overhead |
| Toast type mapping | Auto-icon from `type` vs explicit icon param | Explicit icon param; default `WarningCircle` for warnings, `XCircle` for errors | Call sites that were success toasts using `type: "warning"` pass `CheckCircle` explicitly; avoids breaking the type enum |
| i18n split | Strip emoji + hardcode mapping per key vs keep emoji | Strip from `i18n.svelte.ts`, render in template via `iconMap: Record<string, ComponentType>` in `+page.svelte` | Translations become clean, locale-switching only re-renders text, icons are theme-aware via `currentColor` |
| Central icon map | Single `iconMap` file vs inline at call site | No central file; 33 imports directly in `+page.svelte` | No abstract mapping layer needed — every icon appears exactly once at its usage point; avoids indirection |

## Data Flow: Toast with Icon

```
showToast("Importado", "warning", action, CheckCircle)
    │
    ├─ toast = { message, type, action, icon: CheckCircle }
    │
    ▼
{#if toast}                          ┌─ toast.message
  <div class="toast">               │
    <CheckCircle size={16} />  ◄────┤ toast.icon (rendered if present)
    {toast.message}            ◄────┘
    ...
```

Backend `⚠️` check (`ckResult.includes("⚠️")` at line 382) is NOT touched — it parses Rust response strings. The `⚠️` in the remote-warning UI span (line 1990) IS replaced with `Warning`.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `package.json` | Modify | Add `"phosphor-svelte": "^3.1.0"` to dependencies |
| `pnpm-lock.yaml` | Modify | Lockfile updated by pnpm install |
| `src/routes/+page.svelte` | Modify | 33 emoji→Phosphor substitutions; toast refactor; help title icon mapping; sidebar tab icons; create-button icons |
| `src/lib/i18n.svelte.ts` | Modify | Strip emoji from 9 help title keys (`editorTitle`–`shortcutsTitle`) in both `es` and `en` dictionaries |

**No new files.** No component extraction — `+page.svelte` handles all changes in-place.

## Interfaces / Contracts

```ts
// Modified toast state (line 228)
let toast = $state<{
  message: string;
  type: "warning" | "error";
  action?: { label: string; onClick: () => void };
  icon?: typeof CheckCircle;  // Svelte component type — NEW
} | null>(null);

// Modified showToast (line 498)
function showToast(
  message: string,
  type: "warning" | "error" = "warning",
  action?: { label: string; onClick: () => void },
  icon?: typeof CheckCircle,  // NEW
) { ... }
```

## Icon Inventory (33 total)

| Emoji | Phosphor | Location |
|-------|----------|----------|
| × (delete) | `X` | 4 places: chapter/character/note/timeline delete |
| × (toast) | `X` | Toast close button |
| 📝 | `NotePencil` | Toolbar "New Chapter" |
| ✨ | `Sparkle` | Toolbar "New Project" |
| 📂 | `FolderOpen` | Toolbar "Open Project", Help chapters title |
| 📥 | `DownloadSimple` | Toolbar "Import" |
| 🗜️ | `FileZip` | Export modal + footer |
| 📄 | `FileText` | Share/export md, git log badges |
| ✕ | `XCircle` | Close project, dock close |
| 💾 | `FloppyDisk` | Save button |
| → | `ArrowRight` | View sessions, next chapter |
| ← | `CaretLeft` | Previous chapter |
| ▶/▼ | `CaretRight`/`CaretDown` | Timeline toggle, event expand |
| ▼/▲ | `CaretDown`/`CaretUp` | Footer toggle |
| ✎ | `PencilSimple` | Timeline edit |
| 📌 | `PushPin` | Docked character header |
| 📜 | `Scroll` | Git sessions title |
| ⚠️ (UI only) | `Warning` | Remote warning indicator |
| ? | `Question` | Help button |
| — (none) | `Article`/`Users`/`StickyNote` | Sidebar tabs (NEW) |
| — (none) | `UserPlus`/`NotePencil` | Create buttons in Characters/Notes tabs (NEW) |
| — (none) | `CheckCircle`/`WarningCircle` | Toast icons (NEW) |
| 📖/👤/⏳/📦/💬/⌨️/🟢 | `BookOpen`/`User`/`Clock`/`Package`/`ChatText`/`Keyboard`/`GitBranch` | Help section titles (via iconMap in template) |

**Omitted (per proposal):** `🌙`/`☀️` theme toggle, `🟢`/`🟠`/`🔴` git status dots, backend `⚠️` in `includes("⚠️")`.

## Testing Strategy

| Layer | What | Approach |
|-------|------|----------|
| Build | `pnpm build` succeeds | Verify no import errors, tree-shaking output |
| Visual | 33 icons render in both themes | Manual check on Linux (primary platform); verify `currentColor` inheritance |
| Behavior | Toast shows with `CheckCircle` icon | Call `showToast("msg", "warning", undefined, CheckCircle)` → verify icon renders left of message |
| i18n | `t("help.editorTitle")` returns `"Editor"` | Check both `es`/`en` dictionaries have no emoji in 9 help title keys |
| Bundle | Chunk size < +20 KB gzipped | `pnpm build && gzip -c build/*.js` — compare before/after |

## Bundle Size Estimate

Each Phosphor icon is ~300–700 B of SVG path data (uncompressed JS). At 33 icons: ~16 KB uncompressed, ~6–8 KB gzipped. Well within the 20 KB gzipped budget. Direct path imports ensure Vite includes only the 33 imported icons — the full `phosphor-svelte` library (~1500 icons, ~400 KB) is never bundled.

## Open Questions

None.
