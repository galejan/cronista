<script lang="ts">
  import { onMount } from "svelte";
  import { Editor } from "@tiptap/core";
  import { StarterKit } from "@tiptap/starter-kit";

  interface Props {
    content: string;
    onUpdate: (html: string) => void;
    fontFamily?: string;
  }

  const { content = "", onUpdate, fontFamily = "monospace" }: Props = $props();

  let editorContainer: HTMLDivElement;
  let editor = $state<Editor | null>(null);

  export function setContent(html: string): void {
    editor?.commands.setContent(html, { emitUpdate: true });
  }

  /** Cycle heading: paragraph → H2 → H1 → paragraph */
  export function increaseHeading(): void {
    editor?.chain().focus().toggleHeading({ level: 1 }).run();
  }

  /** Cycle heading: paragraph → H3 → H2 → H1 → paragraph */
  export function decreaseHeading(): void {
    editor?.chain().focus().toggleHeading({ level: 2 }).run();
  }

  /** Insert paired characters and place cursor between them. */
  export function insertPair(open: string, close: string): void {
    if (!editor) return;
    const { from } = editor.state.selection;
    editor
      .chain()
      .focus()
      .insertContent(open + close)
      .setTextSelection(from + open.length)
      .run();
  }

  /** Insert text at cursor (for em dash, etc.). */
  export function insertText(text: string): void {
    editor?.chain().focus().insertContent(text).run();
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

<div class="editor-wrapper" style:font-family={fontFamily === "monospace"
    ? "'Courier New', 'Fira Code', monospace"
    : fontFamily === "serif"
      ? "Georgia, 'Times New Roman', serif"
      : fontFamily === "sans-serif"
        ? "'Inter', 'Segoe UI', sans-serif"
        : fontFamily}>
  <div bind:this={editorContainer}></div>
</div>

<style>
  .editor-wrapper {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .editor-wrapper > div:last-child {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  /* H1 always bold — writing convention */
  :global(.editor-wrapper h1) {
    font-weight: 700 !important;
  }

  /* Clean heading spacing */
  :global(.editor-wrapper h1),
  :global(.editor-wrapper h2),
  :global(.editor-wrapper h3) {
    line-height: 1.3;
    margin: 1.2em 0 0.4em;
  }

  :global(.editor-wrapper h1:first-child),
  :global(.editor-wrapper h2:first-child),
  :global(.editor-wrapper h3:first-child) {
    margin-top: 0;
  }

  :global(.editor-wrapper p) {
    line-height: 1.75;
    margin: 0 0 0.8em;
  }
</style>
