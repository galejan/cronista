<script lang="ts">
  import { t, lang } from "$lib/i18n.svelte";
  import {
    cargarIdentidadGit,
    guardarIdentidadGit,
    cargarConfigRemoto,
    guardarConfigRemoto,
    configurarRemoto,
  } from "$lib/tauri";

  let {
    open = $bindable(false),
    projectPath = "",
    projectName = "",
    onComplete = (_ctx: { remoteConfigured: boolean; remoteUrl: string }) => {},
  } = $props();

  // Form state
  let name = $state("");
  let email = $state("");
  let githubUser = $state("");
  let wantsRemote = $state(false);
  let remoteUrl = $state("");
  let loading = $state(false);
  let error = $state("");
  let step = $state<"identity" | "remote">("identity");

  // Load identity on mount (when dialog opens)
  $effect(() => {
    if (open) {
      loadIdentity();
    }
  });

  async function loadIdentity() {
    loading = true;
    error = "";
    try {
      const identity = await cargarIdentidadGit();
      if (identity) {
        name = identity.name;
        email = identity.email;
        githubUser = identity.github_user || "";
      } else {
        // Pre-fill with language-aware presets
        if (lang.current === "es") {
          name = "Miguel de Cervantes";
          email = "cervantes@literatura.es";
        } else {
          name = "William Shakespeare";
          email = "shakespeare@literature.com";
        }
      }
      // Only load existing remote config for reconfiguration (not new projects)
      if (!projectName) {
        const remote = await cargarConfigRemoto();
        if (remote && remote.url) {
          remoteUrl = remote.url;
          wantsRemote = true;
        }
      }
    } catch (e) {
      console.error("[GitIdentityDialog] Failed to load identity:", e);
    } finally {
      loading = false;
    }
  }

  async function saveIdentityAndContinue() {
    loading = true;
    error = "";
    try {
      await guardarIdentidadGit(name, email, githubUser || undefined);
      step = "remote";
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function saveIdentityAndComplete() {
    loading = true;
    error = "";
    try {
      await guardarIdentidadGit(name, email, githubUser || undefined);
      // Explicitly set no remote
      await guardarConfigRemoto("", false);
      close({ remoteConfigured: false, remoteUrl: "" });
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function saveRemoteAndComplete() {
    loading = true;
    error = "";
    try {
      await guardarIdentidadGit(name, email, githubUser || undefined);
      if (wantsRemote && remoteUrl.trim()) {
        await guardarConfigRemoto(remoteUrl.trim(), true);
        // configurar_remoto needs the project to have git initialized.
        // We save the config now; the page handles git init + remote setup.
        close({ remoteConfigured: true, remoteUrl: remoteUrl.trim() });
      } else {
        await guardarConfigRemoto("", false);
        close({ remoteConfigured: false, remoteUrl: "" });
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function skipRemote() {
    close({ remoteConfigured: false, remoteUrl: "" });
  }

  function close(ctx: { remoteConfigured: boolean; remoteUrl: string }) {
    open = false;
    onComplete(ctx);
  }

  function onOverlayClick() {
    // Don't close by clicking overlay — must use buttons
  }

  function resetOnClose() {
    if (!open) {
      step = "identity";
      error = "";
      wantsRemote = false;
      remoteUrl = "";
      githubUser = "";
    }
  }

  $effect(() => {
    resetOnClose();
  });

  // Auto-fill remote URL from githubUser + projectName when entering step 2
  $effect(() => {
    if (step === "remote" && githubUser && !remoteUrl && projectName) {
      const slug = projectName
        .toLowerCase()
        .replace(/\s+/g, "-")
        .replace(/[^a-z0-9-]/g, "")
        .replace(/-+/g, "-")
        .replace(/^-+|-+$/g, "");
      if (slug) {
        remoteUrl = "git@github.com:" + githubUser + "/" + slug + ".git";
      }
    }
  });
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    tabindex="-1"
    aria-label={t("git.identityTitle")}
    onclick={onOverlayClick}
    onkeydown={(e) => e.key === "Escape" && step === "identity" && skipRemote()}
  >
    <div
      class="modal-panel"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      {#if step === "identity"}
        <!-- Step 1: Identity -->
        <h2>{t("git.identityTitle")}</h2>
        <p class="modal-desc">{t("git.identityDesc")}</p>

        <label class="modal-field">
          {t("git.nameLabel")}
          <input
            type="text"
            bind:value={name}
            class="modal-input"
            disabled={loading}
          />
        </label>

        <label class="modal-field">
          {t("git.emailLabel")}
          <input
            type="email"
            bind:value={email}
            class="modal-input"
            disabled={loading}
          />
        </label>

        <label class="modal-field">
          {t("git.githubUserLabel")}
          <input
            type="text"
            bind:value={githubUser}
            class="modal-input"
            placeholder={t("git.githubUserPlaceholder")}
            disabled={loading}
          />
        </label>

        {#if error}
          <p class="error-msg">{error}</p>
        {/if}

        <div class="modal-actions">
          <button
            class="btn-secondary"
            onclick={saveIdentityAndComplete}
            disabled={loading || !name.trim() || !email.trim()}
          >
            {t("git.identityUseThese")}
          </button>
          <button
            class="btn-primary"
            onclick={saveIdentityAndContinue}
            disabled={loading || !name.trim() || !email.trim()}
          >
            {loading ? t("git.processing") : t("git.identityContinue")}
          </button>
        </div>
      {:else}
        <!-- Step 2: Remote (optional) -->
        <h2>{t("git.remoteTitle")}</h2>

        <div class="info-box">
          <p>{t("git.remoteInfoBox")}</p>
        </div>

        <label class="checkbox-field">
          <input type="checkbox" bind:checked={wantsRemote} disabled={loading} />
          <span>{t("git.remoteCheckbox")}</span>
        </label>

        {#if wantsRemote}
          <label class="modal-field">
            {t("git.remoteUrlLabel")}
            <input
              type="text"
              bind:value={remoteUrl}
              class="modal-input"
              placeholder="git@github.com:user/repo.git"
              disabled={loading}
            />
          </label>
        {/if}

        {#if error}
          <p class="error-msg">{error}</p>
        {/if}

        <div class="modal-actions">
          <button
            class="btn-secondary"
            onclick={skipRemote}
            disabled={loading}
          >
            {t("git.remoteSkip")}
          </button>
          <button
            class="btn-primary"
            onclick={saveRemoteAndComplete}
            disabled={loading || (wantsRemote && !remoteUrl.trim())}
          >
            {loading ? t("git.processing") : t("git.remoteFinish")}
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  /* ── Modal overlay (matches existing patterns in +page.svelte) ──── */
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
    max-width: 460px;
    width: 90vw;
    padding: 1.5rem;
  }

  :global(.dark) .modal-panel {
    background: #1e293b;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }

  .modal-panel h2 {
    margin: 0 0 0.75rem;
    font-size: 1.1rem;
    font-weight: 700;
    color: #1e293b;
  }

  :global(.dark) .modal-panel h2 {
    color: #f1f5f9;
  }

  .modal-desc {
    margin: 0 0 1rem;
    font-size: 0.8125rem;
    color: #64748b;
    line-height: 1.5;
  }

  :global(.dark) .modal-desc {
    color: #94a3b8;
  }

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

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1.25rem;
  }

  /* ── Buttons (matching +page.svelte styles) ────────────────────── */
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

  /* ── Info box ──────────────────────────────────────────────────── */
  .info-box {
    background: #f0f9ff;
    border: 1px solid #bae6fd;
    border-radius: 0.5rem;
    padding: 0.75rem 1rem;
    margin-bottom: 1rem;
    font-size: 0.75rem;
    color: #0369a1;
    line-height: 1.5;
    white-space: pre-line;
  }

  :global(.dark) .info-box {
    background: #0c2d48;
    border-color: #1e5a7a;
    color: #7dd3fc;
  }

  /* ── Checkbox field ────────────────────────────────────────────── */
  .checkbox-field {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
    font-size: 0.8125rem;
    color: #475569;
    cursor: pointer;
  }

  :global(.dark) .checkbox-field {
    color: #cbd5e1;
  }

  .checkbox-field input[type="checkbox"] {
    width: 1rem;
    height: 1rem;
    accent-color: #3b82f6;
    cursor: pointer;
  }

  .checkbox-field input[type="checkbox"]:disabled {
    cursor: not-allowed;
  }

  /* ── Error ─────────────────────────────────────────────────────── */
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

  /* ── Fade-in animation ─────────────────────────────────────────── */
  @keyframes fadeIn {
    from { opacity: 0; }
    to   { opacity: 1; }
  }
</style>
