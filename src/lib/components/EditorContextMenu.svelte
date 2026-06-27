<script lang="ts">
  import { t } from "$lib/i18n.svelte";

  interface Props {
    x: number;
    y: number;
    open: boolean;
    selectedText: string;
    onClose: () => void;
    onCut: () => void;
    onSaveAsNote: () => void;
    onSaveAsTrait: () => void;
    onNewChapter: () => void;
    onAddAsEvent: () => void;
    onAddToPlace: () => void;
  }

  let {
    x = 0,
    y = 0,
    open = $bindable(false),
    selectedText = "",
    onClose = () => {},
    onCut = () => {},
    onSaveAsNote = () => {},
    onSaveAsTrait = () => {},
    onNewChapter = () => {},
    onAddAsEvent = () => {},
    onAddToPlace = () => {},
  }: Props = $props();

  let menuEl = $state<HTMLDivElement>();
  let menuX = $state(0);
  let menuY = $state(0);

  // Position correction on mount — wait a tick for browser layout
  $effect(() => {
    if (!open || !menuEl) return;

    // Read back dimensions (set via CSS, post-render)
    requestAnimationFrame(() => {
      if (!menuEl) return;
      const rect = menuEl.getBoundingClientRect();
      const { innerWidth, innerHeight } = window;

      let adjustedX = x;
      let adjustedY = y;

      if (adjustedX + rect.width > innerWidth) {
        adjustedX = x - rect.width;
      }
      if (adjustedY + rect.height > innerHeight) {
        adjustedY = y - rect.height;
      }

      // Clamp so it never goes off-screen on the opposite side
      adjustedX = Math.max(4, Math.min(adjustedX, innerWidth - rect.width - 4));
      adjustedY = Math.max(4, Math.min(adjustedY, innerHeight - rect.height - 4));

      menuX = adjustedX;
      menuY = adjustedY;
    });
  });

  function handleAction(fn: () => void) {
    fn();
    close();
  }

  function close() {
    open = false;
    onClose();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  }

  function onOverlayClick() {
    close();
  }

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      // Fallback for environments without clipboard API
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
    }
  }

  async function handleCopy() {
    await copyToClipboard(selectedText);
    close();
  }

  async function handleCut() {
    await copyToClipboard(selectedText);
    onCut();
    close();
  }

  async function handlePaste() {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        // Paste at cursor — the page will handle this via insertion
        // For now we just close; the page can use the `paste` event hook
        document.execCommand("insertText", false, text);
      }
    } catch {
      // Clipboard read may fail
    }
    close();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="ctx-overlay"
    onclick={onOverlayClick}
    onkeydown={onKeydown}
  >
    <div
      bind:this={menuEl}
      class="ctx-menu"
      style:left={menuX + "px"}
      style:top={menuY + "px"}
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="menu"
      tabindex="-1"
    >
      <!-- Always visible: clipboard actions -->
      <button class="ctx-item" role="menuitem" onclick={() => handleCopy()}>
        {t("context.copy")}
      </button>
      <button
        class="ctx-item"
        role="menuitem"
        disabled={!selectedText}
        onclick={() => handleCut()}
      >
        {t("context.cut")}
      </button>
      <div class="ctx-separator"></div>
      <button class="ctx-item" role="menuitem" onclick={() => handlePaste()}>
        {t("context.paste")}
      </button>

      {#if selectedText}
        <div class="ctx-separator"></div>

        <button
          class="ctx-item"
          role="menuitem"
          onclick={() => handleAction(onSaveAsNote)}
        >
          {t("context.saveAsNote")}
        </button>
        <button
          class="ctx-item"
          role="menuitem"
          onclick={() => handleAction(onSaveAsTrait)}
        >
          {t("context.saveAsTrait")}
        </button>
        <button
          class="ctx-item"
          role="menuitem"
          onclick={() => handleAction(onNewChapter)}
        >
          {t("context.newChapter")}
        </button>
        <button
          class="ctx-item"
          role="menuitem"
          onclick={() => handleAction(onAddAsEvent)}
        >
          {t("context.addAsEvent")}
        </button>
        <button
          class="ctx-item"
          role="menuitem"
          onclick={() => handleAction(onAddToPlace)}
        >
          {t("context.addToPlace")}
        </button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .ctx-overlay {
    position: fixed;
    inset: 0;
    z-index: 250;
    /* Invisible — just catches clicks */
  }

  .ctx-menu {
    position: fixed;
    min-width: 220px;
    background: var(--bg-app);
    border: 1px solid var(--border-color);
    border-radius: 0.625rem;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.18);
    padding: 0.375rem;
    display: flex;
    flex-direction: column;
    animation: ctxFadeIn 100ms ease;
    z-index: 251;
  }

  :global(.dark) .ctx-menu {
    background: #1e293b;
    border-color: #334155;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  }

  .ctx-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.5rem 0.75rem;
    border: none;
    border-radius: 0.375rem;
    background: transparent;
    font-size: 0.8125rem;
    color: var(--text-main);
    cursor: pointer;
    transition: background 100ms;
    line-height: 1.4;
  }

  .ctx-item:hover {
    background: var(--bg-active-tab);
  }

  .ctx-item:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .ctx-item:disabled:hover {
    background: transparent;
  }

  :global(.dark) .ctx-item {
    color: #e2e8f0;
  }

  :global(.dark) .ctx-item:hover {
    background: #334155;
  }

  :global(.dark) .ctx-item:disabled:hover {
    background: transparent;
  }

  .ctx-separator {
    height: 1px;
    background: #e2e8f0;
    margin: 0.25rem 0.5rem;
  }

  :global(.dark) .ctx-separator {
    background: #334155;
  }

  @keyframes ctxFadeIn {
    from { opacity: 0; transform: scale(0.96); }
    to   { opacity: 1; transform: scale(1); }
  }
</style>
