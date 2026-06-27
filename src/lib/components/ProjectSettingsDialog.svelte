<script lang="ts">
  import { t, lang } from "$lib/i18n.svelte";
  import {
    actualizarConfigProyecto,
    actualizarFuenteProyecto,
    cargarIdentidadGit,
    guardarIdentidadGit,
    cargarConfigRemoto,
    guardarConfigRemoto,
    configurarRemoto,
  } from "$lib/tauri";
  import Gear from "phosphor-svelte/lib/Gear";
  import X from "phosphor-svelte/lib/X";

  let {
    open = $bindable(false),
    projectPath = "",
    currentFontFamily = "monospace",
    currentVisibleTabs = {} as Record<string, boolean>,
    currentAutoSaveInterval = 5,
    onFontSaved = (_font: string) => {},
    onConfigSaved = (_config: {visible_tabs: Record<string,boolean>; auto_save_interval_minutes: number}) => {},
  } = $props();

  // ── Tab state ──────────────────────────────────────────────
  type Tab = "font" | "tabs" | "autosave" | "identity" | "remote";
  let activeTab = $state<Tab>("font");

  // ── Font panel state ───────────────────────────────────────
  let selectedFont = $state("monospace");
  let fontSaving = $state(false);
  let fontError = $state("");
  let fontSuccess = $state(false);

  // ── Tabs panel state ───────────────────────────────────────
  let tabsConfig = $state<Record<string, boolean>>({});
  let tabsSaving = $state(false);
  let tabsError = $state("");
  let tabsSuccess = $state(false);

  // ── Autosave panel state ───────────────────────────────────
  let autoSaveInterval = $state(5);
  let autoSaveSaving = $state(false);
  let autoSaveError = $state("");
  let autoSaveSuccess = $state(false);

  // ── Identity panel state ───────────────────────────────────
  let identityName = $state("");
  let identityEmail = $state("");
  let identityGithubUser = $state("");
  let identityLoading = $state(false);
  let identitySaving = $state(false);
  let identityError = $state("");
  let identitySuccess = $state(false);

  // ── Remote panel state ─────────────────────────────────────
  let remoteUrl = $state("");
  let remoteSaving = $state(false);
  let remoteError = $state("");
  let remoteSuccess = $state(false);
  let remoteLoaded = $state(false);

  // ── Dialog open/close lifecycle ────────────────────────────
  $effect(() => {
    if (open) {
      // Reset all panels on open
      selectedFont = currentFontFamily;
      fontError = "";
      fontSuccess = false;
      fontSaving = false;

      tabsConfig = { ...currentVisibleTabs };
      tabsError = "";
      tabsSuccess = false;
      tabsSaving = false;

      autoSaveInterval = currentAutoSaveInterval;
      autoSaveError = "";
      autoSaveSuccess = false;
      autoSaveSaving = false;

      activeTab = "font";

      identityName = "";
      identityEmail = "";
      identityGithubUser = "";
      identityError = "";
      identitySuccess = false;
      identityLoading = false;
      identitySaving = false;

      remoteUrl = "";
      remoteError = "";
      remoteSuccess = false;
      remoteSaving = false;
      remoteLoaded = false;
    }
  });

  // ── Font panel ─────────────────────────────────────────────

  const fontOptions: { value: string; label: string; cssClass: string }[] = [
    { value: "monospace", label: t("editor.fontMono"), cssClass: "font-mono" },
    { value: "serif", label: t("editor.fontSerif"), cssClass: "font-serif" },
    { value: "sans-serif", label: t("editor.fontSans"), cssClass: "font-sans" },
  ];

  async function saveFont() {
    fontSaving = true;
    fontError = "";
    fontSuccess = false;
    try {
      await actualizarFuenteProyecto(projectPath, selectedFont);
      fontSuccess = true;
      onFontSaved(selectedFont);
    } catch (e) {
      fontError = String(e);
    } finally {
      fontSaving = false;
    }
  }

  function cancelFont() {
    selectedFont = currentFontFamily;
    fontError = "";
    fontSuccess = false;
  }

  // ── Tabs panel ─────────────────────────────────────────────

  async function saveTabs() {
    tabsSaving = true;
    tabsError = "";
    tabsSuccess = false;
    try {
      const result = await actualizarConfigProyecto(projectPath, { visible_tabs: tabsConfig });
      const meta = JSON.parse(result);
      tabsSuccess = true;
      onConfigSaved({
        visible_tabs: meta.visible_tabs || tabsConfig,
        auto_save_interval_minutes: meta.auto_save_interval_minutes || autoSaveInterval,
      });
    } catch (e) {
      tabsError = String(e);
    } finally {
      tabsSaving = false;
    }
  }

  // ── Autosave panel ─────────────────────────────────────────

  async function saveAutoSave() {
    autoSaveSaving = true;
    autoSaveError = "";
    autoSaveSuccess = false;
    try {
      const result = await actualizarConfigProyecto(projectPath, { auto_save_interval_minutes: autoSaveInterval });
      const meta = JSON.parse(result);
      autoSaveSuccess = true;
      onConfigSaved({
        visible_tabs: meta.visible_tabs || tabsConfig,
        auto_save_interval_minutes: meta.auto_save_interval_minutes || autoSaveInterval,
      });
    } catch (e) {
      autoSaveError = String(e);
    } finally {
      autoSaveSaving = false;
    }
  }

  // ── Identity panel ─────────────────────────────────────────

  $effect(() => {
    if (open && activeTab === "identity" && !identityLoading && !identityName) {
      loadIdentity();
    }
  });

  async function loadIdentity() {
    identityLoading = true;
    identityError = "";
    try {
      const identity = await cargarIdentidadGit();
      if (identity) {
        identityName = identity.name;
        identityEmail = identity.email;
        identityGithubUser = identity.github_user || "";
      } else {
        if (lang.current === "es") {
          identityName = "Miguel de Cervantes";
          identityEmail = "cervantes@literatura.es";
        } else {
          identityName = "William Shakespeare";
          identityEmail = "shakespeare@literature.com";
        }
        identityGithubUser = "";
      }
    } catch (e) {
      console.error("[Settings] Failed to load identity:", e);
    } finally {
      identityLoading = false;
    }
  }

  async function saveIdentity() {
    if (!identityName.trim() || !identityEmail.trim()) {
      identityError = identityName.trim()
        ? "El correo electrónico es obligatorio."
        : "El nombre es obligatorio.";
      return;
    }
    identitySaving = true;
    identityError = "";
    identitySuccess = false;
    try {
      await guardarIdentidadGit(
        identityName.trim(),
        identityEmail.trim(),
        identityGithubUser.trim() || undefined,
      );
      identitySuccess = true;
    } catch (e) {
      identityError = String(e);
    } finally {
      identitySaving = false;
    }
  }

  // ── Remote panel ───────────────────────────────────────────

  $effect(() => {
    if (open && activeTab === "remote" && !remoteLoaded) {
      loadRemote();
    }
  });

  async function loadRemote() {
    remoteError = "";
    try {
      const remote = await cargarConfigRemoto(projectPath);
      if (remote && remote.url) {
        remoteUrl = remote.url;
      }
    } catch (e) {
      console.error("[Settings] Failed to load remote config:", e);
    } finally {
      remoteLoaded = true;
    }
  }

  function isValidGitUrl(url: string): boolean {
    const trimmed = url.trim();
    if (!trimmed) return false;
    // Must look like a git URL: git@, ssh://, or https://
    return /^(git@|ssh:\/\/|https?:\/\/)/.test(trimmed) && trimmed.includes(":");
  }

  async function saveRemote() {
    if (!remoteUrl.trim()) {
      remoteError = "La URL del remoto no puede estar vacía.";
      return;
    }
    if (!isValidGitUrl(remoteUrl)) {
      remoteError =
        "La URL no parece válida. Debe comenzar con git@, ssh:// o https://.";
      return;
    }
    remoteSaving = true;
    remoteError = "";
    remoteSuccess = false;
    try {
      await configurarRemoto(projectPath, remoteUrl.trim());
      await guardarConfigRemoto(projectPath, remoteUrl.trim(), true);
      remoteSuccess = true;
    } catch (e) {
      remoteError = String(e);
    } finally {
      remoteSaving = false;
    }
  }

  // ── Dialog dismiss ─────────────────────────────────────────

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
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    tabindex="-1"
    aria-label={t("settings.title")}
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
        <h2><Gear size={18} weight="light" /> {t("settings.title")}</h2>
        <button
          class="close-btn"
          onclick={closeDialog}
          aria-label={t("common.cancel")}
        ><X size={18} weight="light" /></button>
      </div>

      <!-- Tabs -->
      <nav class="settings-tabs">
        <button
          class="settings-tab"
          class:active={activeTab === "font"}
          onclick={() => activeTab = "font"}
        >{t("settings.font")}</button>
        <button
          class="settings-tab"
          class:active={activeTab === "tabs"}
          onclick={() => activeTab = "tabs"}
        >{t("config.stepTabs")}</button>
        <button
          class="settings-tab"
          class:active={activeTab === "autosave"}
          onclick={() => activeTab = "autosave"}
        >{t("config.stepAutoSave")}</button>
        <button
          class="settings-tab"
          class:active={activeTab === "identity"}
          onclick={() => activeTab = "identity"}
        >{t("settings.identity")}</button>
        <button
          class="settings-tab"
          class:active={activeTab === "remote"}
          onclick={() => activeTab = "remote"}
        >{t("settings.remote")}</button>
      </nav>

      <div class="panel-body">
        <!-- ═══ Font Panel ═══ -->
        {#if activeTab === "font"}
          <div class="font-panel">
            <fieldset class="font-radio-group">
              <legend class="sr-only">{t("settings.fontSelectLabel")}</legend>
              {#each fontOptions as opt}
                <label class="font-radio-label">
                  <input
                    type="radio"
                    name="font-family"
                    value={opt.value}
                    bind:group={selectedFont}
                    disabled={fontSaving}
                  />
                  <span class="font-radio-text {opt.cssClass}">{opt.label}</span>
                </label>
              {/each}
            </fieldset>

            <div class="font-preview-block {selectedFont === 'monospace' ? 'font-mono' : selectedFont === 'serif' ? 'font-serif' : 'font-sans'}">
              <span class="preview-label">{t("settings.fontPreview")}</span>
              <p class="preview-text">
                El viejo coronel se desabrochó el cuello, apoyó el bastón entre las piernas y dijo: «No me parece que haya motivos para alarmarse».
              </p>
            </div>

            {#if fontError}
              <p class="error-msg">{fontError}</p>
            {/if}
            {#if fontSuccess}
              <p class="success-msg">{t("settings.saved")}</p>
            {/if}

            <div class="modal-actions">
              <button
                class="btn-secondary"
                onclick={closeDialog}
                disabled={fontSaving}
              >{t("settings.cancel")}</button>
              <button
                class="btn-primary"
                onclick={saveFont}
                disabled={fontSaving || selectedFont === currentFontFamily}
              >
                {fontSaving ? t("settings.saving") : t("settings.save")}
              </button>
            </div>
          </div>
        {:else if activeTab === "tabs"}
          <!-- ═══ Tabs Panel ═══ -->
          <div class="tabs-panel">
            <p class="step-desc">{t("config.tabsHint")}</p>
            <div class="tabs-checklist">
              <label class="tab-checkbox-label disabled">
                <input type="checkbox" checked disabled />
                <span class="tab-checkbox-text">{t("config.tabsChapters")}</span>
              </label>
              {#each [{key: "characters", label: t("config.tabsCharacters")}, {key: "places", label: t("config.tabsPlaces")}, {key: "timeline", label: t("config.tabsTimeline")}, {key: "notes", label: t("config.tabsNotes")}] as tab}
                <label class="tab-checkbox-label">
                  <input
                    type="checkbox"
                    checked={tabsConfig[tab.key] !== false}
                    onchange={() => tabsConfig[tab.key] = tabsConfig[tab.key] === false ? true : false}
                  />
                  <span class="tab-checkbox-text">{tab.label}</span>
                </label>
              {/each}
            </div>

            {#if tabsError}
              <p class="error-msg">{tabsError}</p>
            {/if}
            {#if tabsSuccess}
              <p class="success-msg">{t("settings.saved")}</p>
            {/if}

            <div class="modal-actions">
              <button class="btn-secondary" onclick={closeDialog} disabled={tabsSaving}>{t("settings.cancel")}</button>
              <button class="btn-primary" onclick={saveTabs} disabled={tabsSaving}>
                {tabsSaving ? t("settings.saving") : t("settings.save")}
              </button>
            </div>
          </div>
        {:else if activeTab === "autosave"}
          <!-- ═══ Auto-Save Panel ═══ -->
          <div class="autosave-panel">
            <p class="step-desc">{t("config.intervalHint")}</p>
            <fieldset class="interval-radio-group">
              <legend class="sr-only">{t("config.intervalLabel")}</legend>
              {#each [{value: 1, label: t("config.interval1")}, {value: 5, label: t("config.interval5")}, {value: 10, label: t("config.interval10")}] as opt}
                <label class="interval-radio-label">
                  <input
                    type="radio"
                    name="auto-save-interval"
                    value={opt.value}
                    checked={autoSaveInterval === opt.value}
                    onchange={() => autoSaveInterval = opt.value}
                  />
                  <span class="interval-radio-text">{opt.label}</span>
                </label>
              {/each}
            </fieldset>

            {#if autoSaveError}
              <p class="error-msg">{autoSaveError}</p>
            {/if}
            {#if autoSaveSuccess}
              <p class="success-msg">{t("settings.saved")}</p>
            {/if}

            <div class="modal-actions">
              <button class="btn-secondary" onclick={closeDialog} disabled={autoSaveSaving}>{t("settings.cancel")}</button>
              <button class="btn-primary" onclick={saveAutoSave} disabled={autoSaveSaving}>
                {autoSaveSaving ? t("settings.saving") : t("settings.save")}
              </button>
            </div>
          </div>
        {:else if activeTab === "identity"}
          <!-- ═══ Identity Panel ═══ -->
          <div class="identity-panel">
            {#if identityLoading}
              <p class="loading-text">{t("settings.saving")}</p>
            {:else}
              <label class="modal-field">
                {t("settings.nameLabel")}
                <input
                  type="text"
                  bind:value={identityName}
                  class="modal-input"
                  disabled={identitySaving}
                />
              </label>
              <label class="modal-field">
                {t("settings.emailLabel")}
                <input
                  type="email"
                  bind:value={identityEmail}
                  class="modal-input"
                  disabled={identitySaving}
                />
              </label>
              <label class="modal-field">
                {t("settings.githubUserLabel")}
                <input
                  type="text"
                  bind:value={identityGithubUser}
                  class="modal-input"
                  placeholder={t("git.githubUserPlaceholder")}
                  disabled={identitySaving}
                />
              </label>
            {/if}

            {#if identityError}
              <p class="error-msg">{identityError}</p>
            {/if}
            {#if identitySuccess}
              <p class="success-msg">{t("settings.saved")}</p>
            {/if}

            <div class="modal-actions">
              <button
                class="btn-secondary"
                onclick={closeDialog}
                disabled={identitySaving}
              >{t("settings.cancel")}</button>
              <button
                class="btn-primary"
                onclick={saveIdentity}
                disabled={identitySaving || identityLoading || !identityName.trim() || !identityEmail.trim()}
              >
                {identitySaving ? t("settings.saving") : t("settings.save")}
              </button>
            </div>
          </div>
        {:else}
          <!-- ═══ Remote Panel ═══ -->
          <div class="remote-panel">
            <label class="modal-field">
              {t("settings.urlLabel")}
              <input
                type="text"
                bind:value={remoteUrl}
                class="modal-input"
                placeholder="git@github.com:user/repo.git"
                disabled={remoteSaving}
              />
            </label>

            {#if remoteError}
              <p class="error-msg">{remoteError}</p>
            {/if}
            {#if remoteSuccess}
              <p class="success-msg">{t("settings.saved")}</p>
            {/if}

            <div class="modal-actions">
              <button
                class="btn-secondary"
                onclick={closeDialog}
                disabled={remoteSaving}
              >{t("settings.cancel")}</button>
              <button
                class="btn-primary"
                onclick={saveRemote}
                disabled={remoteSaving || !remoteUrl.trim()}
              >
                {remoteSaving ? t("settings.saving") : t("settings.save")}
              </button>
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  /* ── Modal overlay (matches GitIdentityDialog) ──── */
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
    background: #ffffff;
    border-radius: 0.75rem;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
    max-width: 500px;
    width: 90vw;
    padding: 0;
    overflow: hidden;
  }

  :global(.dark) .modal-panel {
    background: #1e293b;
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
    color: #1e293b;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  :global(.dark) .modal-header h2 {
    color: #f1f5f9;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border: none;
    background: transparent;
    color: #64748b;
    cursor: pointer;
    border-radius: 0.375rem;
    transition: background 150ms;
    padding: 0;
  }

  .close-btn:hover {
    background: #f1f5f9;
  }

  :global(.dark) .close-btn:hover {
    background: #334155;
  }

  /* ── Tabs ────────────────────────────────────── */
  .settings-tabs {
    display: flex;
    border-bottom: 1px solid #e2e8f0;
    padding: 0 1.5rem;
    gap: 0;
  }

  :global(.dark) .settings-tabs {
    border-bottom-color: #334155;
  }

  .settings-tab {
    padding: 0.625rem 1rem;
    border: none;
    background: transparent;
    color: #64748b;
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: color 150ms, border-color 150ms;
  }

  .settings-tab:hover {
    color: #475569;
  }

  :global(.dark) .settings-tab:hover {
    color: #cbd5e1;
  }

  .settings-tab.active {
    color: #3b82f6;
    border-bottom-color: #3b82f6;
  }

  :global(.dark) .settings-tab.active {
    color: #60a5fa;
    border-bottom-color: #60a5fa;
  }

  /* ── Panel body ──────────────────────────────── */
  .panel-body {
    padding: 1.25rem 1.5rem 1.5rem;
  }

  /* ── Font panel ──────────────────────────────── */
  .font-radio-group {
    border: none;
    padding: 0;
    margin: 0 0 1rem;
    display: flex;
    gap: 0.5rem;
  }

  .font-radio-label {
    display: flex;
    align-items: center;
    cursor: pointer;
    flex: 1;
  }

  .font-radio-label input[type="radio"] {
    accent-color: #3b82f6;
    cursor: pointer;
    margin-right: 0.375rem;
  }

  .font-radio-text {
    font-size: 0.875rem;
    color: #1e293b;
  }

  :global(.dark) .font-radio-text {
    color: #f1f5f9;
  }

  .font-mono {
    font-family: ui-monospace, "JetBrains Mono", "Fira Code", "Cascadia Code", monospace;
  }

  .font-serif {
    font-family: "Lora", "Merriweather", "Source Serif 4", "IBM Plex Serif", Georgia, serif;
  }

  .font-sans {
    font-family: "Inter", "Roboto", "Open Sans", system-ui, sans-serif;
  }

  .font-preview-block {
    border: 1px solid #e2e8f0;
    border-radius: 0.5rem;
    padding: 0.75rem;
    margin-bottom: 1rem;
    background: #f8fafc;
  }

  :global(.dark) .font-preview-block {
    background: #0f172a;
    border-color: #334155;
  }

  .preview-label {
    display: block;
    font-size: 0.6875rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #94a3b8;
    margin-bottom: 0.5rem;
  }

  .preview-text {
    margin: 0;
    font-size: 1rem;
    line-height: 1.7;
    color: #1e293b;
  }

  :global(.dark) .preview-text {
    color: #f1f5f9;
  }

  /* ── Screen-reader only ──────────────────────── */
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  /* ── Identity / Remote panels ────────────────── */
  .loading-text {
    text-align: center;
    color: #64748b;
    padding: 1rem 0;
    font-size: 0.875rem;
  }

  /* ── Shared form elements (matching GitIdentityDialog) ── */
  .modal-field {
    display: block;
    margin-bottom: 0.75rem;
    font-size: 0.8125rem;
    font-weight: 500;
    color: #475569;
  }

  :global(.dark) .modal-field {
    color: #cbd5e1;
  }

  .modal-input {
    display: block;
    width: 100%;
    margin-top: 0.25rem;
    padding: 0.5rem 0.75rem;
    font-size: 0.875rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.375rem;
    background: #ffffff;
    color: #1e293b;
    box-sizing: border-box;
  }

  .modal-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  :global(.dark) .modal-input {
    background: #0f172a;
    border-color: #334155;
    color: #f1f5f9;
  }

  .modal-input:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
  }

  /* ── Actions ─────────────────────────────────── */
  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1.25rem;
  }

  /* ── Buttons ─────────────────────────────────── */
  .btn-primary {
    padding: 0.5rem 1.25rem;
    border: none;
    border-radius: 0.375rem;
    background: #3b82f6;
    color: #ffffff;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 150ms;
  }

  .btn-primary:hover:not(:disabled) {
    background: #2563eb;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    padding: 0.5rem 1.25rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.375rem;
    background: #ffffff;
    color: #475569;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 150ms, border-color 150ms;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #f8fafc;
    border-color: #cbd5e1;
  }

  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  :global(.dark) .btn-secondary {
    background: #1e293b;
    border-color: #334155;
    color: #cbd5e1;
  }

  :global(.dark) .btn-secondary:hover:not(:disabled) {
    background: #334155;
    border-color: #475569;
  }

  /* ── Messages ────────────────────────────────── */
  .error-msg {
    color: #dc2626;
    font-size: 0.8125rem;
    margin: 0.5rem 0 0;
    padding: 0.5rem 0.75rem;
    background: #fef2f2;
    border-radius: 0.375rem;
    border: 1px solid #fecaca;
  }

  :global(.dark) .error-msg {
    color: #fca5a5;
    background: #450a0a;
    border-color: #991b1b;
  }

  .success-msg {
    color: #16a34a;
    font-size: 0.8125rem;
    margin: 0.5rem 0 0;
    padding: 0.5rem 0.75rem;
    background: #f0fdf4;
    border-radius: 0.375rem;
    border: 1px solid #bbf7d0;
  }

  :global(.dark) .success-msg {
    color: #86efac;
    background: #052e16;
    border-color: #166534;
  }

  /* ── Fade-in animation ───────────────────────── */
  @keyframes fadeIn {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  /* ── Tabs & Autosave panel styles ────────────── */
  .step-desc {
    font-size: 0.8125rem;
    color: #64748b;
    margin: 0 0 1rem;
    line-height: 1.5;
  }

  :global(.dark) .step-desc {
    color: #94a3b8;
  }

  .tabs-checklist {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .tab-checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.5rem 0.75rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.375rem;
    cursor: pointer;
  }

  :global(.dark) .tab-checkbox-label {
    border-color: #334155;
  }

  .tab-checkbox-label.disabled {
    opacity: 0.7;
    cursor: not-allowed;
    background: #f1f5f9;
  }

  :global(.dark) .tab-checkbox-label.disabled {
    background: #1e293b;
  }

  .tab-checkbox-label input[type="checkbox"] {
    accent-color: #3b82f6;
  }

  .tab-checkbox-text {
    font-size: 0.875rem;
    color: #1e293b;
  }

  :global(.dark) .tab-checkbox-text {
    color: #f1f5f9;
  }

  .interval-radio-group {
    border: none;
    padding: 0;
    margin: 0 0 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .interval-radio-label {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.625rem 0.75rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.375rem;
    cursor: pointer;
    transition: border-color 150ms;
  }

  .interval-radio-label:has(input:checked) {
    border-color: #3b82f6;
  }

  :global(.dark) .interval-radio-label {
    border-color: #334155;
  }

  :global(.dark) .interval-radio-label:has(input:checked) {
    border-color: #60a5fa;
  }

  .interval-radio-label input[type="radio"] {
    accent-color: #3b82f6;
  }

  .interval-radio-text {
    font-size: 0.9375rem;
    color: #1e293b;
  }

  :global(.dark) .interval-radio-text {
    color: #f1f5f9;
  }
</style>
