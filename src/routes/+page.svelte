<script lang="ts">
  import Editor from "$lib/components/Editor.svelte";
  import { debounce } from "$lib/debounce";
  import {
    cargarCapitulo,
    cargarIndice,
    crearCapitulo,
    crearProyecto,
    guardarCapitulo,
  } from "$lib/tauri";
  import { documentDir } from "@tauri-apps/api/path";

  let sidebarVisible = $state(true);
  let theme = $state<"light" | "dark">("light");

  // ── Persist theme in localStorage, default to system preference ──
  $effect(() => {
    const stored = localStorage.getItem("cronista-theme");
    if (stored === "light" || stored === "dark") {
      theme = stored;
    } else if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
      theme = "dark";
    }
  });

  // ── Apply dark class to <html> whenever theme changes ──────────
  $effect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    localStorage.setItem("cronista-theme", theme);
  });

  // ── Editor & project state ──────────────────────────────────
  let projectPath = $state("");
  let chapters = $state<string[]>([]);
  let activeChapter = $state("");
  let editorContent = $state("");
  let saveStatus = $state<"" | "saved" | "unsaved" | "saving">("");

  /** Editor component reference — exposes setContent(html). */
  let editorRef = $state<{ setContent(html: string): void }>();

  // ── Debounced auto-save (2 s after last keystroke) ──────────
  const save = debounce(async () => {
    if (!projectPath || !activeChapter) return;
    saveStatus = "saving";
    console.log("[cronista] Saving chapter:", activeChapter);
    try {
      await guardarCapitulo(projectPath, activeChapter, editorContent);
      saveStatus = "saved";
      console.log("[cronista] Save OK:", activeChapter);
    } catch (e) {
      console.error("[cronista] Save failed:", e);
      saveStatus = "unsaved";
    }
  }, 2_000);

  // ── Editor callbacks ────────────────────────────────────────
  function handleEditorUpdate(html: string): void {
    editorContent = html;
    saveStatus = "unsaved";
    save.trigger();
  }

  // ── Chapter operations ──────────────────────────────────────
  async function cargarCapituloActual(filename: string): Promise<void> {
    if (!projectPath) return;
    save.cancel();
    console.log("[cronista] Loading chapter:", filename);
    try {
      const content = await cargarCapitulo(projectPath, filename);
      editorRef?.setContent(content);
      activeChapter = filename;
      editorContent = content;
      saveStatus = "saved";
      console.log("[cronista] Load OK:", filename, `(${content.length} chars)`);
    } catch (e) {
      console.error("[cronista] Failed to load chapter:", e);
    }
  }

  /** Refresh the chapter list from metadata.json on disk. */
  async function refreshChapters(): Promise<void> {
    if (!projectPath) return;
    try {
      const raw = await cargarIndice(projectPath);
      const meta = JSON.parse(raw);
      chapters = meta.chapters_order ?? [];
      console.log("[cronista] Chapters refreshed:", chapters);
    } catch (e) {
      console.error("[cronista] Failed to read project index:", e);
    }
  }

  async function crearCapituloNuevo(): Promise<void> {
    // ── Initial project setup (only when no project is loaded) ─
    if (!projectPath) {
      const docsDir = await documentDir();
      const defaultPath = `${docsDir}/mi-novela`;
      const path = prompt(
        "Ruta de la carpeta del proyecto:",
        defaultPath,
      );
      if (!path) return;

      const name = prompt("Nombre del proyecto (ej: Mi Novela):", "Mi Novela");
      if (!name) return;

      console.log("[cronista] Creating project:", { path, name });
      try {
        const msg = await crearProyecto(path.trim(), name.trim());
        console.log("[cronista] Project created:", msg);
        projectPath = path.trim();
        await refreshChapters();
      } catch (e) {
        console.error("[cronista] Failed to create project:", e);
        alert(`Error al crear proyecto: ${e}`);
        return;
      }
    }

    const filename = prompt("Nombre del archivo (ej: 0001_prologo.md):", "0001_prologo.md");
    if (!filename) return;

    // Simple heading + empty paragraph so the editor isn't blank.
    const initialHTML = "<h1>Sin título</h1><p></p>";

    console.log("[cronista] Creating chapter:", filename);
    try {
      const msg = await crearCapitulo(projectPath, filename, initialHTML);
      console.log("[cronista] Chapter created:", msg);
      activeChapter = filename;
      editorRef?.setContent(initialHTML);
      editorContent = initialHTML;
      saveStatus = "saved";
      await refreshChapters();
    } catch (e) {
      console.error("[cronista] Create chapter failed:", e);
      alert(`Error al crear capítulo: ${e}`);
    }
  }

  /** Open an existing project by loading its metadata.json. */
  async function abrirProyecto(): Promise<void> {
    const docsDir = await documentDir();
    const path = prompt(
      "Ruta de la carpeta del proyecto existente:",
      docsDir,
    );
    if (!path) return;

    console.log("[cronista] Opening project:", path);
    try {
      // Verify it's a valid project by reading the index
      const raw = await cargarIndice(path.trim());
      const meta = JSON.parse(raw);
      projectPath = path.trim();
      chapters = meta.chapters_order ?? [];
      console.log("[cronista] Project opened:", meta.project_name, chapters);

      // Auto-load first chapter if there is one
      if (chapters.length > 0) {
        await cargarCapituloActual(chapters[0]);
      }
    } catch (e) {
      console.error("[cronista] Failed to open project:", e);
      alert(
        `No se pudo abrir el proyecto. ¿La carpeta contiene .config/metadata.json?\n\n${e}`,
      );
    }
  }

  // ── Keyboard shortcuts ──────────────────────────────────────
  function handleKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === "b") {
      e.preventDefault();
      sidebarVisible = !sidebarVisible;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="app-layout" class:sidebar-collapsed={!sidebarVisible}>
  <!-- Sidebar (40 % when visible) — placeholder, not modified per spec -->
  <aside class="sidebar">
    <nav class="tabs">
      <button class="tab active">Capítulos</button>
      <button class="tab">Personajes</button>
      <button class="tab">Notas</button>
    </nav>
    <div class="sidebar-content">
      <!-- Test button: load a chapter by filename (task 5.4) -->
      <div class="sidebar-toolbar">
        {#if chapters.length > 0}
          <p class="chapter-list-label">Capítulos:</p>
          <ul class="chapter-list">
            {#each chapters as ch}
              <li>
                <button
                  class="chapter-link"
                  class:active-chapter={activeChapter === ch}
                  onclick={() => cargarCapituloActual(ch)}
                >
                  {ch}
                </button>
              </li>
            {/each}
          </ul>
        {/if}

        <button class="btn-load" onclick={() => {
          const fn = prompt("Nombre del archivo a cargar (ej: 0001_prologo.md):");
          if (fn) cargarCapituloActual(fn.trim());
        }}>
          Cargar capítulo
        </button>
      </div>
    </div>
  </aside>

  <!-- Editor area (60 % when visible, 100 % when sidebar collapsed) -->
  <main class="editor">
    {#if !projectPath}
      <!-- First launch: prompt for project path -->
      <div class="setup-prompt">
        <p class="setup-text">Seleccioná una carpeta de proyecto para comenzar</p>
        <div class="setup-actions">
          <button
            class="btn-primary"
            onclick={() => crearCapituloNuevo()}
          >
            + Nuevo proyecto
          </button>
          <button
            class="btn-secondary"
            onclick={() => abrirProyecto()}
          >
            Abrir proyecto
          </button>
        </div>
      </div>
    {:else}
      <!-- Toolbar + Editor -->
      <div class="editor-pane">
        <div class="editor-toolbar">
          <div class="toolbar-left">
            <span class="project-label" title={projectPath}>
              {projectPath.split("/").pop() || projectPath}
            </span>
            <button class="toolbar-btn" onclick={crearCapituloNuevo}>
              + Nuevo capítulo
            </button>
          </div>

          <div class="toolbar-right">
            {#if activeChapter}
              <span class="chapter-label">{activeChapter}</span>
            {/if}
            <button
              class="theme-toggle"
              onclick={() => (theme = theme === "light" ? "dark" : "light")}
              title={theme === "light" ? "Activar tema oscuro" : "Activar tema claro"}
            >
              {theme === "light" ? "🌙" : "☀️"}
            </button>
            <span
              class="save-indicator"
              class:saving={saveStatus === "saving"}
              class:saved={saveStatus === "saved"}
              class:unsaved={saveStatus === "unsaved"}
            >
              {saveStatus === "saving"
                ? "Guardando…"
                : saveStatus === "saved"
                  ? "Guardado"
                  : saveStatus === "unsaved"
                    ? "Sin guardar"
                    : ""}
            </span>
          </div>
        </div>

        <div class="editor-body">
          <Editor
            bind:this={editorRef}
            content={editorContent}
            onUpdate={handleEditorUpdate}
          />
        </div>
      </div>
    {/if}
  </main>
</div>

<style>
  /* ── Layout ────────────────────────────────────────────────── */
  .app-layout {
    display: grid;
    grid-template-columns: 40% 60%;
    height: 100vh;
    transition: grid-template-columns 300ms ease;
  }

  .app-layout.sidebar-collapsed {
    grid-template-columns: 0% 100%;
  }

  /* ── Sidebar ───────────────────────────────────────────────── */
  .sidebar {
    overflow: hidden;
    border-right: 1px solid #e2e8f0;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  :global(.dark) .sidebar {
    border-right-color: #334155;
  }

  .sidebar-collapsed .sidebar {
    border-right: none;
  }

  .tabs {
    display: flex;
    border-bottom: 1px solid #e2e8f0;
  }

  :global(.dark) .tabs {
    border-bottom-color: #334155;
  }

  .tab {
    flex: 1;
    padding: 0.75rem 0.5rem;
    font-size: 0.875rem;
    font-weight: 500;
    color: #64748b;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    transition: color 150ms, border-color 150ms;
  }

  .tab:hover {
    color: #1e293b;
  }

  .tab.active {
    color: #3b82f6;
    border-bottom-color: #3b82f6;
  }

  :global(.dark) .tab {
    color: #94a3b8;
  }
  :global(.dark) .tab:hover {
    color: #e2e8f0;
  }
  :global(.dark) .tab.active {
    color: #60a5fa;
    border-bottom-color: #60a5fa;
  }

  .sidebar-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
  }

  /* ── Sidebar toolbar ──────────────────────────────────────── */
  .sidebar-toolbar {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .chapter-list-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #94a3b8;
    margin-bottom: 0.25rem;
  }

  .chapter-list {
    list-style: none;
    padding: 0;
    margin: 0 0 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .chapter-link {
    width: 100%;
    text-align: left;
    padding: 0.375rem 0.5rem;
    border: none;
    background: transparent;
    border-radius: 0.25rem;
    font-size: 0.8125rem;
    color: #475569;
    cursor: pointer;
    transition: background 120ms;
  }

  .chapter-link:hover {
    background: #f1f5f9;
  }

  .chapter-link.active-chapter {
    background: #eff6ff;
    color: #3b82f6;
    font-weight: 500;
  }

  :global(.dark) .chapter-link {
    color: #94a3b8;
  }
  :global(.dark) .chapter-link:hover {
    background: #1e293b;
  }
  :global(.dark) .chapter-link.active-chapter {
    background: #1e3a5f;
    color: #60a5fa;
  }

  .btn-load {
    padding: 0.375rem 0.75rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.375rem;
    background: #ffffff;
    font-size: 0.8125rem;
    color: #475569;
    cursor: pointer;
    transition: border-color 120ms, background 120ms;
  }

  .btn-load:hover {
    border-color: #3b82f6;
    background: #f8fafc;
  }

  :global(.dark) .btn-load {
    background: #1e293b;
    border-color: #334155;
    color: #94a3b8;
  }
  :global(.dark) .btn-load:hover {
    border-color: #60a5fa;
  }

  /* ── Editor area ───────────────────────────────────────────── */
  .editor {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  /* ── Setup prompt (no project yet) ─────────────────────────── */
  .setup-prompt {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    height: 100%;
  }

  .setup-text {
    color: #94a3b8;
    font-size: 1.125rem;
    font-style: italic;
  }

  :global(.dark) .setup-text {
    color: #64748b;
  }

  .setup-actions {
    display: flex;
    gap: 0.75rem;
  }

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

  .btn-primary:hover {
    background: #2563eb;
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

  .btn-secondary:hover {
    background: #f8fafc;
    border-color: #cbd5e1;
  }

  :global(.dark) .btn-secondary {
    background: #1e293b;
    border-color: #334155;
    color: #94a3b8;
  }
  :global(.dark) .btn-secondary:hover {
    background: #334155;
    border-color: #475569;
  }

  /* ── Editor pane ───────────────────────────────────────────── */
  .editor-pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
  }

  .editor-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 1rem;
    border-bottom: 1px solid #e2e8f0;
    background: #f8fafc;
    flex-shrink: 0;
    gap: 0.75rem;
  }

  :global(.dark) .editor-toolbar {
    background: #0f172a;
    border-bottom-color: #334155;
  }

  .toolbar-left,
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .project-label {
    font-size: 0.8125rem;
    font-weight: 500;
    color: #1e293b;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.dark) .project-label {
    color: #e2e8f0;
  }

  .chapter-label {
    font-size: 0.75rem;
    color: #64748b;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.dark) .chapter-label {
    color: #94a3b8;
  }

  .toolbar-btn {
    padding: 0.25rem 0.625rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.25rem;
    background: #ffffff;
    font-size: 0.75rem;
    color: #3b82f6;
    cursor: pointer;
    transition: background 120ms, border-color 120ms;
    white-space: nowrap;
  }

  .toolbar-btn:hover {
    background: #eff6ff;
    border-color: #3b82f6;
  }

  :global(.dark) .toolbar-btn {
    background: #1e293b;
    border-color: #334155;
    color: #60a5fa;
  }
  :global(.dark) .toolbar-btn:hover {
    background: #1e3a5f;
  }

  /* ── Theme toggle ─────────────────────────────────────────── */
  .theme-toggle {
    padding: 0.25rem 0.5rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.25rem;
    background: #ffffff;
    font-size: 1rem;
    cursor: pointer;
    line-height: 1;
    transition: background 120ms, border-color 120ms;
  }

  .theme-toggle:hover {
    background: #f1f5f9;
    border-color: #cbd5e1;
  }

  :global(.dark) .theme-toggle {
    background: #1e293b;
    border-color: #334155;
  }
  :global(.dark) .theme-toggle:hover {
    background: #334155;
    border-color: #475569;
  }

  /* ── Save indicator ────────────────────────────────────────── */
  .save-indicator {
    font-size: 0.75rem;
    color: #94a3b8;
    transition: color 200ms;
  }

  .save-indicator.saving {
    color: #f59e0b;
  }

  .save-indicator.saved {
    color: #22c55e;
  }

  .save-indicator.unsaved {
    color: #ef4444;
  }

  /* ── Editor body ───────────────────────────────────────────── */
  .editor-body {
    flex: 1;
    overflow: hidden;
    min-width: 0;
  }
</style>
