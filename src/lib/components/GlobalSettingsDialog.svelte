<script lang="ts">
  import { t, setLang, lang } from "$lib/i18n.svelte";
  import { themeManager, type AppTheme } from "$lib/theme.svelte";
  import Gear from "phosphor-svelte/lib/Gear";
  import X from "phosphor-svelte/lib/X";

  let {
    open = $bindable(false),
  } = $props();

  type Tab = "language" | "theme";
  let activeTab = $state<Tab>("language");

  // ── Theme definitions with metadata for preview cards ────────

  interface ThemeDef {
    id: AppTheme;
    name: string;
    bg: string;
    text: string;
    title: string;
    border: string;
    accent: string;
  }

  const themes: ThemeDef[] = [
    { id: "dark-nordic",    name: "Oscuro Nórdico", bg: "#0f172a", text: "#e2e8f0", title: "#f1f5f9", border: "#1e293b", accent: "#3b82f6" },
    { id: "dark-amethyst",  name: "Oscuro Amatista",bg: "#130f1c", text: "#e4def2", title: "#f3effc", border: "#241c33", accent: "#a855f7" },
    { id: "light-nordic",   name: "Claro Nórdico",  bg: "#e2e8f0", text: "#1e293b", title: "#0f172a", border: "#cbd5e1", accent: "#3b82f6" },
    { id: "light-sepia",    name: "Claro Sepia",    bg: "#f3efe9", text: "#2d2824", title: "#1a1614", border: "#dcd5cb", accent: "#b45309" },
  ];

  // ── Handlers ─────────────────────────────────────────────────

  function closeDialog() {
    open = false;
  }

  function handleOverlayClick() {
    closeDialog();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      closeDialog();
    }
  }

  function selectLang(l: "es" | "en") {
    setLang(l);
  }

  function selectTheme(theme: AppTheme) {
    themeManager.setTheme(theme);
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    tabindex="-1"
    aria-label={t("globalSettings.title")}
    onclick={handleOverlayClick}
    onkeydown={handleKeydown}
  >
    <div
      class="modal-panel"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <!-- Header -->
      <div class="modal-header">
        <h2><Gear size={18} weight="light" /> {t("globalSettings.title")}</h2>
        <button
          class="close-btn"
          onclick={closeDialog}
          aria-label={t("globalSettings.close")}
        ><X size={18} weight="light" /></button>
      </div>

      <!-- Tabs -->
      <nav class="settings-tabs">
        <button
          class="settings-tab"
          class:active={activeTab === "language"}
          onclick={() => activeTab = "language"}
        >{t("globalSettings.language")}</button>
        <button
          class="settings-tab"
          class:active={activeTab === "theme"}
          onclick={() => activeTab = "theme"}
        >{t("globalSettings.theme")}</button>
      </nav>

      <div class="panel-body">
        <!-- ═══ Language Panel ═══ -->
        {#if activeTab === "language"}
          <div class="language-panel">
            <div class="lang-options">
              <button
                class="lang-btn"
                class:active={lang.current === "es"}
                onclick={() => selectLang("es")}
                title="Español"
              >ES</button>
              <button
                class="lang-btn"
                class:active={lang.current === "en"}
                onclick={() => selectLang("en")}
                title="English"
              >EN</button>
            </div>
          </div>

        <!-- ═══ Theme Panel ═══ -->
        {:else}
          <div class="theme-panel">
            <div class="theme-grid">
              {#each themes as theme}
                {@const selected = themeManager.current === theme.id}
                <button
                  class="theme-card"
                  class:selected
                  onclick={() => selectTheme(theme.id)}
                  title={theme.name}
                >
                  <div
                    class="theme-preview"
                    style="background:{theme.bg}; color:{theme.text}; border-color:{theme.border}"
                  >
                    <div class="preview-title" style="color:{theme.title}">
                      {lang.current === "es" ? "Título de ejemplo" : "Sample Title"}
                    </div>
                    <div class="preview-body" style="color:{theme.text}">
                      {lang.current === "es"
                        ? "El texto del cuerpo se adapta al tema seleccionado."
                        : "The body text adapts to the selected theme."}
                    </div>
                  </div>
                  <span class="theme-name">{theme.name}</span>
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>

      <!-- Actions -->
      <div class="modal-actions">
        <button class="btn-primary" onclick={closeDialog}>
          {t("globalSettings.close")}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* ── Modal overlay (matches ProjectSettingsDialog) ──── */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fadeIn 120ms ease;
  }

  .modal-panel {
    background: var(--bg-app, #ffffff);
    border-radius: 0.75rem;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
    max-width: 480px;
    width: 90vw;
    padding: 0;
    overflow: hidden;
  }

  :global(.dark) .modal-panel {
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }

  /* ── Header ─────────────────────────────────── */
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1.25rem 1.5rem 0.5rem;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--text-main, #1e293b);
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border: none;
    background: transparent;
    color: var(--text-muted, #64748b);
    cursor: pointer;
    border-radius: 0.375rem;
    transition: background 150ms;
    padding: 0;
  }

  .close-btn:hover {
    background: var(--bg-editor, #f1f5f9);
  }

  /* ── Tabs ────────────────────────────────────── */
  .settings-tabs {
    display: flex;
    border-bottom: 1px solid var(--border-color, #e2e8f0);
    padding: 0 1.5rem;
    gap: 0;
  }

  .settings-tab {
    padding: 0.625rem 1rem;
    border: none;
    background: transparent;
    color: var(--text-muted, #64748b);
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: color 150ms, border-color 150ms;
  }

  .settings-tab:hover {
    color: var(--text-main, #475569);
  }

  .settings-tab.active {
    color: var(--accent, #3b82f6);
    border-bottom-color: var(--accent, #3b82f6);
  }

  /* ── Panel body ──────────────────────────────── */
  .panel-body {
    padding: 1.25rem 1.5rem 1.5rem;
  }

  /* ── Language panel ──────────────────────────── */
  .lang-options {
    display: flex;
    gap: 0.75rem;
    justify-content: center;
  }

  .lang-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 4rem;
    height: 3rem;
    border: 2px solid var(--border-color, #e2e8f0);
    border-radius: 0.5rem;
    background: var(--bg-editor, #f8fafc);
    color: var(--text-main, #1e293b);
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 150ms;
  }

  .lang-btn:hover {
    border-color: var(--accent, #3b82f6);
  }

  .lang-btn.active {
    border-color: var(--accent, #3b82f6);
    background: var(--accent, #3b82f6);
    color: #ffffff;
  }

  /* ── Theme panel ─────────────────────────────── */
  .theme-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .theme-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem;
    border: 2px solid var(--border-color, #e2e8f0);
    border-radius: 0.5rem;
    background: var(--bg-editor, #f8fafc);
    cursor: pointer;
    transition: all 150ms;
    text-align: center;
  }

  .theme-card:hover {
    border-color: var(--text-muted, #94a3b8);
  }

  .theme-card.selected {
    border-color: var(--accent, #3b82f6);
    box-shadow: 0 0 0 1px var(--accent, #3b82f6);
  }

  .theme-preview {
    width: 100%;
    padding: 0.625rem;
    border-radius: 0.375rem;
    border: 1px solid;
    font-size: 0.75rem;
    line-height: 1.5;
  }

  .preview-title {
    font-size: 0.8125rem;
    font-weight: 700;
    margin-bottom: 0.25rem;
  }

  .preview-body {
    font-size: 0.6875rem;
    opacity: 0.85;
  }

  .theme-name {
    font-size: 0.6875rem;
    font-weight: 500;
    color: var(--text-main, #1e293b);
  }

  /* ── Fade-in animation ───────────────────────── */
  @keyframes fadeIn {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    padding: 0 1.5rem 1.25rem;
  }

  .btn-primary {
    padding: 0.5rem 1.5rem;
    border: none;
    border-radius: 0.375rem;
    background: var(--accent);
    color: #fff;
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 120ms;
  }

  .btn-primary:hover {
    background: var(--accent-hover);
  }
</style>
