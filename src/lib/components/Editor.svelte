<script lang="ts">
  import { onMount } from "svelte";
  import { Editor } from "@tiptap/core";
  import { StarterKit } from "@tiptap/starter-kit";
  import { TextStyle, FontFamily } from "@tiptap/extension-text-style";
  import { t } from "$lib/i18n";

  interface Props {
    /** Initial HTML content to load into the editor. */
    content: string;
    /** Called on every editor update with the current HTML. */
    onUpdate: (html: string) => void;
  }

  const { content = "", onUpdate }: Props = $props();

  let editorContainer: HTMLDivElement;

  // The editor instance — $state so template bindings react.
  let editor = $state<Editor | null>(null);

  /** Exposed to the parent via bind:this. */
  export function setContent(html: string): void {
    editor?.commands.setContent(html, { emitUpdate: true });
  }

  /** Toggle heading level (1–3) at cursor. Called by keyboard shortcuts. */
  export function toggleHeading(level: 1 | 2 | 3): void {
    editor?.chain().focus().toggleHeading({ level }).run();
  }

  onMount(() => {
    const ed = new Editor({
      element: editorContainer,
      extensions: [
        StarterKit.configure({
          bold: false,
          italic: false,
          strike: false,
          underline: false,
          code: false,
          codeBlock: false,
          blockquote: false,
          bulletList: false,
          orderedList: false,
          listItem: false,
          listKeymap: false,
          link: false,
          horizontalRule: false,
          hardBreak: false,
        }),
        TextStyle,
        FontFamily,
      ],
      content,
      onUpdate: ({ editor: ed }) => {
        onUpdate(ed.getHTML());
      },
    });

    editor = ed;

    return () => {
      ed.destroy();
    };
  });
</script>

<div class="editor-wrapper">
  <!-- Fixed formatting toolbar -->
  <div class="formatting-bar">
    <button
      type="button"
      class:active={editor?.isActive("heading", { level: 1 })}
      onclick={() => editor?.chain().focus().toggleHeading({ level: 1 }).run()}
      title={t("editor.heading1")}
    >H1</button>
    <button
      type="button"
      class:active={editor?.isActive("heading", { level: 2 })}
      onclick={() => editor?.chain().focus().toggleHeading({ level: 2 }).run()}
      title={t("editor.heading2")}
    >H2</button>
    <button
      type="button"
      class:active={editor?.isActive("heading", { level: 3 })}
      onclick={() => editor?.chain().focus().toggleHeading({ level: 3 }).run()}
      title={t("editor.heading3")}
    >H3</button>
    <button
      type="button"
      class:active={editor?.isActive("paragraph")}
      onclick={() => editor?.chain().focus().setParagraph().run()}
      title={t("editor.paragraph")}
    >¶</button>

    <span class="bar-divider" aria-hidden="true"></span>

    <select
      aria-label={t("editor.fontFamily")}
      onchange={(e) =>
        editor
          ?.chain()
          .focus()
          .setFontFamily(e.currentTarget.value)
          .run()}
    >
      <option value="">{t("editor.fontDefault")}</option>
      <option value="Georgia, 'Times New Roman', serif">{t("editor.fontSerif")}</option>
      <option value="Arial, Helvetica, sans-serif">{t("editor.fontSans")}</option>
      <option value="'Courier New', Courier, monospace">{t("editor.fontMono")}</option>
    </select>
  </div>

  <!-- TipTap mounts here -->
  <div bind:this={editorContainer}></div>
</div>

<style>
  .editor-wrapper {
    width: 100%;
    height: 100%;
    position: relative;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* The div where TipTap mounts — fills remaining space */
  .editor-wrapper > div:last-child {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  /* ── Formatting bar ───────────────────────────────────────── */

  .formatting-bar {
    display: flex;
    align-items: center;
    gap: 0.125rem;
    padding: 0.25rem 0.75rem;
    background: #f8fafc;
    border-bottom: 1px solid #e2e8f0;
    flex-shrink: 0;
  }

  :global(.dark) .formatting-bar {
    background: #0f172a;
    border-bottom-color: #334155;
  }

  .formatting-bar button {
    padding: 0.2rem 0.45rem;
    border: none;
    background: transparent;
    border-radius: 0.25rem;
    cursor: pointer;
    font-size: 0.75rem;
    font-weight: 600;
    color: #64748b;
    line-height: 1.4;
    transition: background 120ms, color 120ms;
  }

  .formatting-bar button:hover {
    background: #e2e8f0;
    color: #1e293b;
  }

  .formatting-bar button.active {
    background: #3b82f6;
    color: #ffffff;
  }

  :global(.dark) .formatting-bar button {
    color: #94a3b8;
  }

  :global(.dark) .formatting-bar button:hover {
    background: #334155;
    color: #e2e8f0;
  }

  :global(.dark) .formatting-bar button.active {
    background: #3b82f6;
    color: #ffffff;
  }

  .bar-divider {
    display: inline-block;
    width: 1px;
    height: 1.25rem;
    background: #e2e8f0;
    margin: 0 0.25rem;
  }

  :global(.dark) .bar-divider {
    background: #334155;
  }

  .formatting-bar select {
    padding: 0.15rem 0.3rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    color: #475569;
    background: #ffffff;
    cursor: pointer;
    outline: none;
    appearance: none;
    -webkit-appearance: none;
  }

  .formatting-bar select:focus {
    border-color: #3b82f6;
  }

  .formatting-bar select option {
    background: #ffffff;
    color: #475569;
  }

  :global(.dark) .formatting-bar select {
    background: #1e293b;
    border-color: #334155;
    color: #94a3b8;
    color-scheme: dark;
  }

  :global(.dark) .formatting-bar select option {
    background: #1e293b;
    color: #94a3b8;
  }

  :global(.dark) .formatting-bar select:focus {
    border-color: #60a5fa;
  }
</style>
