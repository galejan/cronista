<script lang="ts">
  import { onMount } from "svelte";
  import { Editor } from "@tiptap/core";
  import { StarterKit } from "@tiptap/starter-kit";
  import { BubbleMenu } from "@tiptap/extension-bubble-menu";
  import { TextStyle, FontFamily } from "@tiptap/extension-text-style";

  interface Props {
    /** Initial HTML content to load into the editor. */
    content: string;
    /** Called on every editor update with the current HTML. */
    onUpdate: (html: string) => void;
  }

  const { content = "", onUpdate }: Props = $props();

  // DOM refs set via bind:this during mount.
  let editorContainer: HTMLDivElement;
  let bubbleMenuEl: HTMLDivElement;

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
          // Disable formatting not in scope for the literary editor MVP.
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
          // Kept: heading, paragraph, document, text,
          //        undoRedo, dropcursor, gapcursor, trailingNode.
        }),
        TextStyle,
        FontFamily,
        BubbleMenu.configure({
          element: bubbleMenuEl,
        }),
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
  <!--
    Bubble menu — always rendered in the DOM.
    The BubbleMenu extension manages positioning and visibility.
    Optional-chaining keeps buttons inert when the editor hasn't been created yet.
  -->
  <div bind:this={bubbleMenuEl} class="bubble-menu">
    <button
      type="button"
      title="Título 1 (Ctrl+Alt+1)"
      onclick={() =>
        editor?.chain().focus().toggleHeading({ level: 1 }).run()}
      class:active={editor?.isActive("heading", { level: 1 })}
    >
      H1
    </button>
    <button
      type="button"
      title="Título 2 (Ctrl+Alt+2)"
      onclick={() =>
        editor?.chain().focus().toggleHeading({ level: 2 }).run()}
      class:active={editor?.isActive("heading", { level: 2 })}
    >
      H2
    </button>
    <button
      type="button"
      title="Título 3 (Ctrl+Alt+3)"
      onclick={() =>
        editor?.chain().focus().toggleHeading({ level: 3 }).run()}
      class:active={editor?.isActive("heading", { level: 3 })}
    >
      H3
    </button>
    <button
      type="button"
      onclick={() => editor?.chain().focus().setParagraph().run()}
      class:active={editor?.isActive("paragraph")}
    >
      ¶
    </button>

    <span class="menu-divider" aria-hidden="true"></span>

    <select
      aria-label="Font family"
      onchange={(e) =>
        editor
          ?.chain()
          .focus()
          .setFontFamily(e.currentTarget.value)
          .run()}
    >
      <option value="">Default</option>
      <option value="Georgia, 'Times New Roman', serif">Serif</option>
      <option value="Arial, Helvetica, sans-serif">Sans-serif</option>
      <option value="'Courier New', Courier, monospace">Monospace</option>
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
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  /* ── Bubble Menu ───────────────────────────────────────────── */

  .bubble-menu {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.375rem 0.5rem;
    background: #ffffff;
    border: 1px solid #e2e8f0;
    border-radius: 0.5rem;
    box-shadow:
      0 4px 12px rgba(0, 0, 0, 0.08),
      0 1px 3px rgba(0, 0, 0, 0.06);
    z-index: 50;
  }

  .bubble-menu button {
    padding: 0.25rem 0.5rem;
    border: none;
    background: transparent;
    border-radius: 0.25rem;
    cursor: pointer;
    font-size: 0.8125rem;
    font-weight: 500;
    color: #475569;
    line-height: 1.4;
    transition: background 120ms, color 120ms;
  }

  .bubble-menu button:hover {
    background: #f1f5f9;
  }

  .bubble-menu button.active {
    background: #3b82f6;
    color: #ffffff;
  }

  .menu-divider {
    display: inline-block;
    width: 1px;
    height: 1.25rem;
    background: #e2e8f0;
    margin: 0 0.25rem;
  }

  .bubble-menu select {
    padding: 0.25rem 0.375rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.25rem;
    font-size: 0.8125rem;
    color: #475569;
    background: #ffffff;
    cursor: pointer;
    outline: none;
  }

  .bubble-menu select:focus {
    border-color: #3b82f6;
  }

  /* ── Dark mode ─────────────────────────────────────────────── */

  :global(.dark) .bubble-menu {
    background: #1e293b;
    border-color: #334155;
  }

  :global(.dark) .bubble-menu button {
    color: #94a3b8;
  }

  :global(.dark) .bubble-menu button:hover {
    background: #334155;
  }

  :global(.dark) .bubble-menu button.active {
    background: #3b82f6;
    color: #ffffff;
  }

  :global(.dark) .menu-divider {
    background: #334155;
  }

  :global(.dark) .bubble-menu select {
    background: #1e293b;
    border-color: #334155;
    color: #94a3b8;
  }

  :global(.dark) .bubble-menu select:focus {
    border-color: #60a5fa;
  }
</style>
