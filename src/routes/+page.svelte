<script lang="ts">
  import Editor from "$lib/components/Editor.svelte";
  import { debounce } from "$lib/debounce";
  import {
    actualizarPersonaje,
    agregarEventoTimeline,
    cargarCapitulo,
    cargarIndice,
    cargarNota,
    cargarPersonaje,
    cargarTimeline,
    crearCapitulo,
    crearNota,
    crearPersonaje,
    crearProyecto,
    eliminarCapitulo,
    eliminarEventoTimeline,
    eliminarNota,
    eliminarPersonaje,
    guardarCapitulo,
    listarNotas,
    listarPersonajes,
  } from "$lib/tauri";
  import { documentDir } from "@tauri-apps/api/path";
  import { open } from "@tauri-apps/plugin-dialog";

  let sidebarPct = $state(40);          // current sidebar width in %
  let sidebarSaved = $state(40);        // width to restore on un-collapse
  let sidebarCollapsed = $state(false); // derived for CSS class
  let theme = $state<"light" | "dark">("light");
  let helpMode = $state(false);

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
  let pendingDelete = $state<string | null>(null);
  let activeChapter = $state("");
  let editorContent = $state("");
  let saveStatus = $state<"" | "saved" | "unsaved" | "saving">("");

  /** Editor component reference — exposes setContent(html) + toggleHeading(level). */
  let editorRef = $state<{
    setContent(html: string): void;
    toggleHeading(level: 1 | 2 | 3): void;
  }>();

  // ── Sidebar tab state ───────────────────────────────────────
  let activeTab = $state<"capitulos" | "personajes" | "notas">("capitulos");

  // ── Characters state ────────────────────────────────────────
  let personajes = $state<{ id: string; name: string }[]>([]);
  let personajeFormVisible = $state(false);
  let personajeNuevoNombre = $state("");
  let personajeExpandido = $state<string | null>(null);
  let personajeEditando = $state<Record<string, any> | null>(null);

  // ── Notes state ─────────────────────────────────────────────
  let notas = $state<{ id: string; title: string }[]>([]);
  let activeNote = $state("");
  let notaTitulo = $state("");

  // ── Timeline state ──────────────────────────────────────────
  let timeline = $state<Record<string, any>[]>([]);
  let timelineVisible = $state(false);
  let eventoFormVisible = $state(false);
  let nuevoEventoFecha = $state("");
  let nuevoEventoTitulo = $state("");
  let nuevoEventoDescripcion = $state("");
  let nuevoEventoPersonajes = $state<string[]>([]);
  let nuevoEventoCapitulos = $state<string[]>([]);

  // ── Debounced auto-save (2 s after last keystroke) ──────────
  const save = debounce(async () => {
    if (!projectPath) return;
    saveStatus = "saving";

    if (activeNote) {
      console.log("[cronista] Saving note:", activeNote);
      try {
        await crearNota(projectPath, activeNote, notaTitulo, editorContent);
        saveStatus = "saved";
        console.log("[cronista] Save Note OK:", activeNote);
      } catch (e) {
        console.error("[cronista] Save note failed:", e);
        saveStatus = "unsaved";
      }
    } else if (activeChapter) {
      console.log("[cronista] Saving chapter:", activeChapter);
      try {
        await guardarCapitulo(projectPath, activeChapter, editorContent);
        saveStatus = "saved";
        console.log("[cronista] Save OK:", activeChapter);
      } catch (e) {
        console.error("[cronista] Save failed:", e);
        saveStatus = "unsaved";
      }
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

  async function eliminarCapituloHandler(filename: string): Promise<void> {
    if (pendingDelete === filename) {
      // Second click — execute deletion
      try {
        await eliminarCapitulo(projectPath, filename);
        if (activeChapter === filename) {
          activeChapter = "";
          editorRef?.setContent("");
          editorContent = "";
          saveStatus = "";
        }
        await refreshChapters();
      } catch (e) {
        console.error("[cronista] Delete chapter failed:", e);
        alert(`Error al eliminar capítulo: ${e}`);
      }
      pendingDelete = null;
    } else {
      // First click — mark for confirmation
      pendingDelete = filename;
      // Auto-reset after 3 seconds
      setTimeout(() => { pendingDelete = null; }, 3_000);
    }
  }

  async function crearCapituloNuevo(): Promise<void> {
    // ── Initial project setup (only when no project is loaded) ─
    if (!projectPath) {
      const docsDir = await documentDir();
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Seleccioná la carpeta donde crear el proyecto",
        defaultPath: docsDir,
      });
      if (!selected) return;

      const name = prompt("Nombre del proyecto (ej: Mi Novela):", "Mi Novela");
      if (!name) return;

      const path = `${selected}/${name.trim()}`;

      console.log("[cronista] Creating project:", { path, name });
      try {
        const msg = await crearProyecto(path, name.trim());
        console.log("[cronista] Project created:", msg);
        projectPath = path;
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
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Seleccioná la carpeta del proyecto",
      defaultPath: docsDir,
    });
    if (!selected) return;

    const path = selected as string;
    console.log("[cronista] Opening project:", path);
    try {
      // Verify it's a valid project by reading the index
      const raw = await cargarIndice(path);
      const meta = JSON.parse(raw);
      projectPath = path;
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

  // ── Characters CRUD ─────────────────────────────────────────

  async function refreshPersonajes(): Promise<void> {
    if (!projectPath) return;
    try {
      const raw = await listarPersonajes(projectPath);
      personajes = JSON.parse(raw);
    } catch (e) {
      console.error("[cronista] Failed to list characters:", e);
      personajes = [];
    }
  }

  async function crearPersonajeHandler(): Promise<void> {
    if (!personajeNuevoNombre.trim()) {
      alert("El nombre del personaje es obligatorio.");
      return;
    }
    const id = personajeNuevoNombre
      .trim()
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");

    const char = {
      id,
      name: personajeNuevoNombre.trim(),
      physicalDescription: "",
      personality: "",
      traumas: "",
      relationships: [],
    };
    try {
      await crearPersonaje(projectPath, JSON.stringify(char));
      personajeNuevoNombre = "";
      personajeFormVisible = false;
      await refreshPersonajes();
    } catch (e) {
      console.error("[cronista] Create character failed:", e);
      alert(`Error al crear personaje: ${e}`);
    }
  }

  async function seleccionarPersonaje(id: string): Promise<void> {
    save.trigger(); // save current work first
    try {
      const raw = await cargarPersonaje(projectPath, id);
      personajeEditando = JSON.parse(raw);
      personajeExpandido = id;
    } catch (e) {
      console.error("[cronista] Load character failed:", e);
    }
  }

  async function guardarPersonaje(): Promise<void> {
    if (!personajeEditando) return;
    try {
      await actualizarPersonaje(
        projectPath,
        personajeEditando.id,
        JSON.stringify(personajeEditando),
      );
      personajeExpandido = null;
      personajeEditando = null;
      await refreshPersonajes();
    } catch (e) {
      console.error("[cronista] Update character failed:", e);
      alert(`Error al guardar personaje: ${e}`);
    }
  }

  async function eliminarPersonajeHandler(id: string): Promise<void> {
    if (!confirm("¿Eliminar este personaje?")) return;
    try {
      await eliminarPersonaje(projectPath, id);
      personajeExpandido = null;
      personajeEditando = null;
      await refreshPersonajes();
      await refreshTimeline();
    } catch (e) {
      console.error("[cronista] Delete character failed:", e);
      alert(`Error al eliminar personaje: ${e}`);
    }
  }

  function agregarRelacionPersonaje(): void {
    if (!personajeEditando) return;
    if (!personajeEditando.relationships) personajeEditando.relationships = [];
    personajeEditando.relationships.push({
      targetName: "",
      type: "",
      notes: "",
    });
  }

  function eliminarRelacionPersonaje(idx: number): void {
    if (!personajeEditando) return;
    personajeEditando.relationships.splice(idx, 1);
  }

  // ── Notes CRUD ──────────────────────────────────────────────

  async function refreshNotas(): Promise<void> {
    if (!projectPath) return;
    try {
      const raw = await listarNotas(projectPath);
      notas = JSON.parse(raw);
    } catch (e) {
      console.error("[cronista] Failed to list notes:", e);
      notas = [];
    }
  }

  async function crearNotaHandler(): Promise<void> {
    const title = prompt("Título de la nota:");
    if (!title?.trim()) return;
    const id = title
      .trim()
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
    try {
      await crearNota(projectPath, id, title.trim(), "<p></p>");
      await refreshNotas();
    } catch (e) {
      console.error("[cronista] Create note failed:", e);
      alert(`Error al crear nota: ${e}`);
    }
  }

  async function cargarNotaHandler(id: string): Promise<void> {
    save.trigger(); // save current work first
    try {
      const raw = await cargarNota(projectPath, id);
      editorRef?.setContent(raw);
      activeNote = id;
      activeChapter = "";
      editorContent = raw;
      saveStatus = "saved";
      // Find title from index
      const found = notas.find((n) => n.id === id);
      if (found) notaTitulo = found.title;
      console.log("[cronista] Note loaded:", id);
    } catch (e) {
      console.error("[cronista] Load note failed:", e);
    }
  }

  async function guardarNotaActual(): Promise<void> {
    if (!activeNote) return;
    saveStatus = "saving";
    try {
      await crearNota(projectPath, activeNote, notaTitulo, editorContent);
      saveStatus = "saved";
      await refreshNotas();
    } catch (e) {
      console.error("[cronista] Save note failed:", e);
      saveStatus = "unsaved";
    }
  }

  async function eliminarNotaHandler(id: string): Promise<void> {
    if (!confirm("¿Eliminar esta nota?")) return;
    try {
      await eliminarNota(projectPath, id);
      if (activeNote === id) {
        activeNote = "";
        notaTitulo = "";
        editorRef?.setContent("");
      }
      await refreshNotas();
    } catch (e) {
      console.error("[cronista] Delete note failed:", e);
      alert(`Error al eliminar nota: ${e}`);
    }
  }

  // ── Timeline CRUD ───────────────────────────────────────────

  async function refreshTimeline(): Promise<void> {
    if (!projectPath) return;
    try {
      const raw = await cargarTimeline(projectPath);
      timeline = JSON.parse(raw);
      // Sort chronologically
      timeline.sort((a, b) => (a.date < b.date ? -1 : a.date > b.date ? 1 : 0));
    } catch (e) {
      console.error("[cronista] Failed to load timeline:", e);
      timeline = [];
    }
  }

  async function agregarEventoHandler(): Promise<void> {
    if (!nuevoEventoFecha || !nuevoEventoTitulo) {
      alert("Fecha y título son obligatorios.");
      return;
    }
    const evento = {
      date: nuevoEventoFecha,
      title: nuevoEventoTitulo.trim(),
      description: nuevoEventoDescripcion.trim(),
      relatedCharacters: nuevoEventoPersonajes.filter(Boolean),
      relatedChapters: nuevoEventoCapitulos.filter(Boolean),
    };
    try {
      await agregarEventoTimeline(projectPath, JSON.stringify(evento));
      nuevoEventoFecha = "";
      nuevoEventoTitulo = "";
      nuevoEventoDescripcion = "";
      nuevoEventoPersonajes = [];
      nuevoEventoCapitulos = [];
      eventoFormVisible = false;
      await refreshTimeline();
    } catch (e) {
      console.error("[cronista] Add timeline event failed:", e);
      alert(`Error al agregar evento: ${e}`);
    }
  }

  async function eliminarEventoHandler(id: string): Promise<void> {
    if (!confirm("¿Eliminar este evento?")) return;
    try {
      await eliminarEventoTimeline(projectPath, id);
      await refreshTimeline();
    } catch (e) {
      console.error("[cronista] Delete timeline event failed:", e);
      alert(`Error al eliminar evento: ${e}`);
    }
  }

  function togglePersonajeCapitulo(
    arr: string[],
    item: string,
  ): string[] {
    if (arr.includes(item)) return arr.filter((i) => i !== item);
    return [...arr, item];
  }

  // ── Refresh all sidebar data when project changes ───────────
  $effect(() => {
    if (projectPath) {
      refreshPersonajes();
      refreshNotas();
      refreshTimeline();
    }
  });

  // ── Keyboard shortcuts ──────────────────────────────────────
  function handleKeydown(e: KeyboardEvent) {
    // Ctrl+B — toggle sidebar collapse
    if (e.ctrlKey && !e.shiftKey && e.key === "b") {
      e.preventDefault();
      if (sidebarCollapsed) {
        sidebarPct = sidebarSaved;
        sidebarCollapsed = false;
      } else {
        sidebarSaved = sidebarPct;
        sidebarPct = 0;
        sidebarCollapsed = true;
      }
      return;
    }

    // Ctrl+< — shrink sidebar by 5 % (min 20 %)
    if (e.ctrlKey && e.key === "<") {
      e.preventDefault();
      sidebarCollapsed = false;
      sidebarPct = Math.max(20, sidebarPct - 5);
      sidebarSaved = sidebarPct;
      return;
    }

    // Ctrl+> — grow sidebar by 5 % (max 60 %)
    if (e.ctrlKey && e.key === ">") {
      e.preventDefault();
      sidebarCollapsed = false;
      sidebarPct = Math.min(60, sidebarPct + 5);
      sidebarSaved = sidebarPct;
      return;
    }

    // Ctrl+S — manual save
    if (e.ctrlKey && !e.shiftKey && e.key === "s") {
      e.preventDefault();
      saveStatus = "saving";
      save.trigger();
      return;
    }

    // Ctrl+N — new chapter
    if (e.ctrlKey && !e.shiftKey && e.key === "n") {
      e.preventDefault();
      crearCapituloNuevo();
      return;
    }

    // Ctrl+O — open project
    if (e.ctrlKey && !e.shiftKey && e.key === "o") {
      e.preventDefault();
      abrirProyecto();
      return;
    }

    // Ctrl+Shift+N — new project (force re-setup)
    if (e.ctrlKey && e.shiftKey && e.key === "N") {
      e.preventDefault();
      projectPath = "";
      chapters = [];
      activeChapter = "";
      editorContent = "";
      crearCapituloNuevo();
      return;
    }

    // F11 — fullscreen toggle
    if (e.key === "F11") {
      e.preventDefault();
      if (document.fullscreenElement) {
        document.exitFullscreen();
      } else {
        document.documentElement.requestFullscreen();
      }
      return;
    }

    // F1 — help toggle
    if (e.key === "F1") {
      e.preventDefault();
      helpMode = !helpMode;
      return;
    }

    // ? — help toggle (without shift, plain key)
    if (!e.ctrlKey && !e.altKey && !e.metaKey && e.key === "?") {
      e.preventDefault();
      helpMode = !helpMode;
      return;
    }

    // Ctrl+Alt+1/2/3 — heading toggle (only when editor is mounted)
    if (e.ctrlKey && e.altKey) {
      if (e.key === "1") { e.preventDefault(); editorRef?.toggleHeading(1); return; }
      if (e.key === "2") { e.preventDefault(); editorRef?.toggleHeading(2); return; }
      if (e.key === "3") { e.preventDefault(); editorRef?.toggleHeading(3); return; }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="app-layout" class:sidebar-collapsed={sidebarCollapsed} style="grid-template-columns: {sidebarCollapsed ? 0 : sidebarPct}% {sidebarCollapsed ? 100 : 100 - sidebarPct}%">
  <!-- Sidebar (40 % when visible) — placeholder, not modified per spec -->
  <aside class="sidebar">
    <nav class="tabs">
      <button
        class="tab"
        class:active={activeTab === "capitulos"}
        onclick={() => { pendingDelete = null; activeTab = "capitulos"; activeNote = ""; }}
      >Capítulos</button>
      <button
        class="tab"
        class:active={activeTab === "personajes"}
        onclick={() => { pendingDelete = null; activeTab = "personajes"; }}
      >Personajes</button>
      <button
        class="tab"
        class:active={activeTab === "notas"}
        onclick={() => { pendingDelete = null; activeTab = "notas"; }}
      >Notas</button>
    </nav>

    <div class="sidebar-content">
      <!-- ═══ Capítulos tab ═══ -->
      {#if activeTab === "capitulos"}
        <div class="tab-panel">
          {#if chapters.length > 0}
            <p class="chapter-list-label">Capítulos:</p>
            <ul class="chapter-list">
              {#each chapters as ch}
                <li class="chapter-row">
                  <button
                    class="chapter-link"
                    class:active-chapter={activeChapter === ch}
                    onclick={() => { pendingDelete = null; activeNote = ""; cargarCapituloActual(ch); }}
                  >
                    {ch}
                  </button>
                  {#if pendingDelete === ch}
                    <button
                      class="delete-confirm"
                      title="Confirmar eliminación"
                      onclick={() => eliminarCapituloHandler(ch)}
                    >¿Eliminar?</button>
                  {:else}
                    <button
                      class="item-delete"
                      title="Eliminar capítulo"
                      onclick={() => eliminarCapituloHandler(ch)}
                    >×</button>
                  {/if}
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-hint">Sin capítulos aún.</p>
          {/if}

          <button class="btn-load" onclick={() => {
            pendingDelete = null;
            const fn = prompt("Nombre del archivo a cargar (ej: 0001_prologo.md):");
            if (fn) cargarCapituloActual(fn.trim());
          }}>
            Cargar capítulo
          </button>
        </div>
      {/if}

      <!-- ═══ Personajes tab ═══ -->
      {#if activeTab === "personajes"}
        <div class="tab-panel">
          {#if personajes.length > 0}
            <ul class="chapter-list">
              {#each personajes as p}
                <li>
                  <button
                    class="chapter-link"
                    class:active-chapter={personajeExpandido === p.id}
                    onclick={() => seleccionarPersonaje(p.id)}
                  >
                    {p.name}
                  </button>

                  {#if personajeExpandido === p.id && personajeEditando}
                    <div class="inline-form">
                      <label class="field-label" for="char-name-{p.id}">Nombre</label>
                      <input
                        id="char-name-{p.id}"
                        class="field-input"
                        type="text"
                        bind:value={personajeEditando.name}
                      />

                      <label class="field-label" for="char-desc-{p.id}">Descripción física</label>
                      <textarea
                        id="char-desc-{p.id}"
                        class="field-textarea"
                        bind:value={personajeEditando.physicalDescription}
                        rows="2"
                      ></textarea>

                      <label class="field-label" for="char-pers-{p.id}">Personalidad</label>
                      <textarea
                        id="char-pers-{p.id}"
                        class="field-textarea"
                        bind:value={personajeEditando.personality}
                        rows="2"
                      ></textarea>

                      <label class="field-label" for="char-trau-{p.id}">Traumas</label>
                      <textarea
                        id="char-trau-{p.id}"
                        class="field-textarea"
                        bind:value={personajeEditando.traumas}
                        rows="2"
                      ></textarea>

                      <label class="field-label" for="char-rel-{p.id}">Relaciones</label>
                      {#if personajeEditando.relationships?.length > 0}
                        {#each personajeEditando.relationships as rel, ri}
                          <div class="relationship-row">
                            <input
                              class="field-input small"
                              type="text"
                              placeholder="Nombre"
                              bind:value={rel.targetName}
                            />
                            <input
                              class="field-input small"
                              type="text"
                              placeholder="Tipo (hermano, amigo...)"
                              bind:value={rel.type}
                            />
                            <input
                              class="field-input small"
                              type="text"
                              placeholder="Notas"
                              bind:value={rel.notes}
                            />
                            <button
                              class="btn-sm btn-danger"
                              onclick={() => eliminarRelacionPersonaje(ri)}
                            >×</button>
                          </div>
                        {/each}
                      {/if}
                      <button class="btn-sm" onclick={agregarRelacionPersonaje}>
                        + Añadir relación
                      </button>

                      <div class="form-actions">
                        <button class="btn-sm btn-primary" onclick={guardarPersonaje}>Guardar</button>
                        <button class="btn-sm btn-danger" onclick={() => eliminarPersonajeHandler(p.id)}>Eliminar</button>
                      </div>
                    </div>
                  {/if}
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-hint">Sin personajes aún.</p>
          {/if}

          {#if personajeFormVisible}
            <div class="inline-form">
              <input
                class="field-input"
                type="text"
                placeholder="Nombre del personaje"
                bind:value={personajeNuevoNombre}
                onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter") crearPersonajeHandler(); }}
              />
              <div class="form-actions">
                <button class="btn-sm btn-primary" onclick={crearPersonajeHandler}>Crear</button>
                <button class="btn-sm" onclick={() => personajeFormVisible = false}>Cancelar</button>
              </div>
            </div>
          {:else}
            <button class="btn-add" onclick={() => personajeFormVisible = true}>
              + Nuevo personaje
            </button>
          {/if}
        </div>
      {/if}

      <!-- ═══ Notas tab ═══ -->
      {#if activeTab === "notas"}
        <div class="tab-panel">
          {#if notas.length > 0}
            <ul class="chapter-list">
              {#each notas as n}
                <li class="note-row">
                  <button
                    class="chapter-link"
                    class:active-chapter={activeNote === n.id}
                    onclick={() => cargarNotaHandler(n.id)}
                  >
                    {n.title}
                  </button>
                  <button
                    class="item-delete"
                    title="Eliminar nota"
                    onclick={() => eliminarNotaHandler(n.id)}
                  >×</button>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-hint">Sin notas aún.</p>
          {/if}

          {#if activeNote}
            <div class="inline-form">
              <label class="field-label" for="note-title">Título de la nota</label>
              <input
                id="note-title"
                class="field-input"
                type="text"
                bind:value={notaTitulo}
              />
              <div class="form-actions">
                <button class="btn-sm btn-primary" onclick={guardarNotaActual}>Guardar</button>
                <button
                  class="btn-sm"
                  onclick={() => { activeNote = ""; notaTitulo = ""; }}
                >Cerrar</button>
              </div>
            </div>
          {/if}

          <button class="btn-add" onclick={() => crearNotaHandler()}>
            + Nueva nota
          </button>
        </div>
      {/if}

      <!-- ═══ Timeline — collapsible section at bottom ═══ -->
      <div class="timeline-section">
        <button
          class="timeline-toggle"
          onclick={() => { timelineVisible = !timelineVisible; if (timelineVisible) refreshTimeline(); }}
        >
          {timelineVisible ? "▼" : "▶"} Línea de tiempo
          {#if timeline.length > 0}
            <span class="timeline-badge">{timeline.length}</span>
          {/if}
        </button>

        {#if timelineVisible}
          <div class="timeline-content">
            {#if timeline.length > 0}
              <ul class="timeline-list">
                {#each timeline as evt}
                  <li class="timeline-event">
                    <span class="event-date">{evt.date}</span>
                    <span class="event-title">{evt.title}</span>
                    <button
                      class="item-delete"
                      title="Eliminar evento"
                      onclick={() => eliminarEventoHandler(evt.id)}
                    >×</button>
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="empty-hint">Sin eventos en la línea de tiempo.</p>
            {/if}

            {#if eventoFormVisible}
              <div class="inline-form">
                <label class="field-label" for="evt-date">Fecha</label>
                <input
                  id="evt-date"
                  class="field-input"
                  type="date"
                  bind:value={nuevoEventoFecha}
                />
                <label class="field-label" for="evt-title">Título</label>
                <input
                  id="evt-title"
                  class="field-input"
                  type="text"
                  bind:value={nuevoEventoTitulo}
                  placeholder="¿Qué pasó?"
                />
                <label class="field-label" for="evt-desc">Descripción</label>
                <textarea
                  id="evt-desc"
                  class="field-textarea"
                  bind:value={nuevoEventoDescripcion}
                  rows="2"
                  placeholder="Detalles del evento..."
                ></textarea>

                {#if personajes.length > 0}
                  <span class="field-label">Personajes relacionados</span>
                  <div class="checkbox-group">
                    {#each personajes as p}
                      <label class="checkbox-label">
                        <input
                          type="checkbox"
                          checked={nuevoEventoPersonajes.includes(p.id)}
                          onchange={() => nuevoEventoPersonajes = togglePersonajeCapitulo(nuevoEventoPersonajes, p.id)}
                        />
                        {p.name}
                      </label>
                    {/each}
                  </div>
                {/if}

                {#if chapters.length > 0}
                  <span class="field-label">Capítulos relacionados</span>
                  <div class="checkbox-group">
                    {#each chapters as ch}
                      <label class="checkbox-label">
                        <input
                          type="checkbox"
                          checked={nuevoEventoCapitulos.includes(ch)}
                          onchange={() => nuevoEventoCapitulos = togglePersonajeCapitulo(nuevoEventoCapitulos, ch)}
                        />
                        {ch}
                      </label>
                    {/each}
                  </div>
                {/if}

                <div class="form-actions">
                  <button class="btn-sm btn-primary" onclick={agregarEventoHandler}>Agregar</button>
                  <button class="btn-sm" onclick={() => eventoFormVisible = false}>Cancelar</button>
                </div>
              </div>
            {:else}
              <button class="btn-add" onclick={() => eventoFormVisible = true}>
                + Nuevo evento
              </button>
            {/if}
          </div>
        {/if}
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
            <button class="toolbar-btn" onclick={crearCapituloNuevo} title="Nuevo capítulo (Ctrl+N)">
              + Nuevo capítulo
            </button>
          </div>

          <div class="toolbar-right">
            {#if activeChapter}
              <span class="chapter-label">{activeChapter}</span>
            {/if}
            <button
              class="toolbar-btn"
              onclick={() => { saveStatus = "saving"; save.trigger(); }}
              title="Guardar ahora (Ctrl+S)"
            >
              Guardar
            </button>
            <button
              class="help-btn"
              onclick={() => (helpMode = !helpMode)}
              title="Ayuda (F1)"
            >
              ?
            </button>
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

{#if helpMode}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="help-overlay"
    role="dialog"
    aria-label="Ayuda de Cronista"
    onclick={() => (helpMode = false)}
    onkeydown={(e) => e.key === "Escape" && (helpMode = false)}
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="help-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <div class="help-header">
        <h2>Cronista</h2>
        <span class="help-version">v1.0</span>
        <button class="help-close" onclick={() => (helpMode = false)}>✕</button>
      </div>

      <p class="help-creator">creado por <a href="mailto:galejan@gmail.com">galejan@gmail.com</a></p>

      <div class="help-section">
        <h3>📖 Editor</h3>
        <p>Escribí en la zona central. El texto se guarda automáticamente a los 2 segundos de inactividad. Usá el menú flotante para dar formato al seleccionar texto.</p>
      </div>

      <div class="help-section">
        <h3>📂 Capítulos</h3>
        <p>Creá, cargá y eliminá capítulos desde la pestaña <strong>Capítulos</strong> o con el botón <strong>+ Nuevo capítulo</strong>. Doble clic en ✕ para eliminar con confirmación.</p>
      </div>

      <div class="help-section">
        <h3>👤 Personajes</h3>
        <p>Fichas con descripción física, personalidad, traumas y relaciones. Las relaciones pueden ser unilaterales (ej: A está enamorado de B, pero no al revés).</p>
      </div>

      <div class="help-section">
        <h3>📝 Notas</h3>
        <p>Ideas, recordatorios y análisis. Al hacer clic en una nota, su contenido se carga en el editor principal para que puedas trabajar con formato.</p>
      </div>

      <div class="help-section">
        <h3>⏳ Timeline</h3>
        <p>Línea de tiempo al final del sidebar. Agregá eventos con fecha, descripción y vinculalos a personajes y capítulos.</p>
      </div>

      <div class="help-section">
        <h3>⌨️ Atajos de teclado</h3>
        <table class="help-shortcuts">
          <tbody>
          <tr><td><kbd>Ctrl+B</kbd></td><td>Colapsar / restaurar sidebar</td></tr>
          <tr><td><kbd>Ctrl+&lt;</kbd> / <kbd>Ctrl+&gt;</kbd></td><td>Encoger / agrandar sidebar (5% por paso)</td></tr>
          <tr><td><kbd>Ctrl+S</kbd></td><td>Guardar ahora</td></tr>
          <tr><td><kbd>Ctrl+N</kbd></td><td>Nuevo capítulo</td></tr>
          <tr><td><kbd>Ctrl+O</kbd></td><td>Abrir proyecto existente</td></tr>
          <tr><td><kbd>Ctrl+Shift+N</kbd></td><td>Nuevo proyecto (reinicia)</td></tr>
          <tr><td><kbd>Ctrl+Alt+1</kbd> / <kbd>2</kbd> / <kbd>3</kbd></td><td>Aplicar Título 1 / 2 / 3</td></tr>
          <tr><td><kbd>F11</kbd></td><td>Pantalla completa</td></tr>
          <tr><td><kbd>F1</kbd> o <kbd>?</kbd></td><td>Mostrar / ocultar esta ayuda</td></tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
{/if}

<style>
  /* ── Layout ────────────────────────────────────────────────── */
  .app-layout {
    display: grid;
    height: 100vh;
    transition: grid-template-columns 200ms ease;
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

  /* ── Chapter list ──────────────────────────────────────── */
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

  /* ── Tab panels ────────────────────────────────────────────── */
  .tab-panel {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .empty-hint {
    font-size: 0.8125rem;
    color: #94a3b8;
    font-style: italic;
    padding: 0.5rem 0;
  }

  :global(.dark) .empty-hint {
    color: #64748b;
  }

  /* ── Chapter row with delete ───────────────────────────────── */
  .chapter-row,
  .note-row {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .chapter-row .chapter-link,
  .note-row .chapter-link {
    flex: 1;
  }

  .item-delete {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: #94a3b8;
    font-size: 1rem;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 120ms, color 120ms;
  }

  .item-delete:hover {
    background: #fee2e2;
    color: #ef4444;
  }

  :global(.dark) .item-delete {
    color: #64748b;
  }
  :global(.dark) .item-delete:hover {
    background: #7f1d1d33;
    color: #f87171;
  }

  .delete-confirm {
    background: #ef4444;
    color: white;
    border: none;
    border-radius: 0.25rem;
    padding: 0.125rem 0.375rem;
    font-size: 0.6875rem;
    cursor: pointer;
    animation: pulse 0.6s infinite alternate;
    flex-shrink: 0;
    white-space: nowrap;
  }

  @keyframes pulse {
    from { opacity: 1; }
    to   { opacity: 0.6; }
  }

  /* ── Inline forms (characters, notes, timeline) ────────────── */
  .inline-form {
    margin: 0.5rem 0 0.5rem 0.75rem;
    padding: 0.75rem;
    border-left: 2px solid #3b82f6;
    border-radius: 0 4px 4px 0;
    background: #f8fafc;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  :global(.dark) .inline-form {
    background: #1e293b;
    border-left-color: #60a5fa;
  }

  .field-label {
    font-size: 0.6875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #64748b;
    margin-top: 0.25rem;
  }

  :global(.dark) .field-label {
    color: #94a3b8;
  }

  .field-input {
    padding: 0.375rem 0.5rem;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    font-size: 0.8125rem;
    color: #1e293b;
    background: #fff;
    width: 100%;
    box-sizing: border-box;
  }

  .field-input:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px #3b82f633;
  }

  :global(.dark) .field-input {
    background: #0f172a;
    border-color: #334155;
    color: #e2e8f0;
  }
  :global(.dark) .field-input:focus {
    border-color: #60a5fa;
    box-shadow: 0 0 0 2px #60a5fa33;
  }

  .field-input.small {
    padding: 0.25rem 0.375rem;
    font-size: 0.75rem;
  }

  .field-textarea {
    padding: 0.375rem 0.5rem;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    font-size: 0.8125rem;
    color: #1e293b;
    background: #fff;
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    font-family: inherit;
  }

  .field-textarea:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px #3b82f633;
  }

  :global(.dark) .field-textarea {
    background: #0f172a;
    border-color: #334155;
    color: #e2e8f0;
  }
  :global(.dark) .field-textarea:focus {
    border-color: #60a5fa;
    box-shadow: 0 0 0 2px #60a5fa33;
  }

  .form-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.25rem;
  }

  /* ── Small buttons ─────────────────────────────────────────── */
  .btn-sm {
    padding: 0.25rem 0.5rem;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    background: #fff;
    font-size: 0.75rem;
    color: #475569;
    cursor: pointer;
    transition: background 120ms, border-color 120ms;
    white-space: nowrap;
  }

  .btn-sm:hover {
    background: #f1f5f9;
    border-color: #cbd5e1;
  }

  :global(.dark) .btn-sm {
    background: #1e293b;
    border-color: #334155;
    color: #94a3b8;
  }
  :global(.dark) .btn-sm:hover {
    background: #334155;
    border-color: #475569;
  }

  .btn-sm.btn-primary {
    background: #3b82f6;
    color: #fff;
    border-color: #3b82f6;
  }

  .btn-sm.btn-primary:hover {
    background: #2563eb;
    border-color: #2563eb;
  }

  :global(.dark) .btn-sm.btn-primary {
    background: #3b82f6;
    color: #fff;
  }

  .btn-sm.btn-danger {
    background: #fff;
    color: #ef4444;
    border-color: #fecaca;
  }

  .btn-sm.btn-danger:hover {
    background: #fef2f2;
    border-color: #ef4444;
  }

  :global(.dark) .btn-sm.btn-danger {
    background: #1e293b;
    color: #f87171;
    border-color: #7f1d1d;
  }
  :global(.dark) .btn-sm.btn-danger:hover {
    background: #7f1d1d33;
    border-color: #f87171;
  }

  /* ── Add button ────────────────────────────────────────────── */
  .btn-add {
    padding: 0.375rem 0.75rem;
    border: 1px dashed #cbd5e1;
    border-radius: 4px;
    background: transparent;
    font-size: 0.8125rem;
    color: #3b82f6;
    cursor: pointer;
    transition: background 120ms, border-color 120ms;
    width: 100%;
    text-align: left;
    margin-top: 0.25rem;
  }

  .btn-add:hover {
    background: #eff6ff;
    border-color: #3b82f6;
  }

  :global(.dark) .btn-add {
    border-color: #334155;
    color: #60a5fa;
  }
  :global(.dark) .btn-add:hover {
    background: #1e3a5f;
    border-color: #60a5fa;
  }

  /* ── Relationship rows ─────────────────────────────────────── */
  .relationship-row {
    display: flex;
    gap: 0.25rem;
    align-items: center;
  }

  .relationship-row .field-input {
    flex: 1;
    min-width: 0;
  }

  /* ── Timeline section ──────────────────────────────────────── */
  .timeline-section {
    margin-top: auto;
    border-top: 1px solid #e2e8f0;
    padding-top: 0.75rem;
    flex-shrink: 0;
  }

  :global(.dark) .timeline-section {
    border-top-color: #334155;
  }

  .timeline-toggle {
    width: 100%;
    text-align: left;
    padding: 0.5rem 0;
    border: none;
    background: transparent;
    font-size: 0.8125rem;
    font-weight: 600;
    color: #475569;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    transition: color 120ms;
  }

  .timeline-toggle:hover {
    color: #1e293b;
  }

  :global(.dark) .timeline-toggle {
    color: #94a3b8;
  }
  :global(.dark) .timeline-toggle:hover {
    color: #e2e8f0;
  }

  .timeline-badge {
    font-size: 0.6875rem;
    background: #e2e8f0;
    color: #475569;
    padding: 0.125rem 0.375rem;
    border-radius: 8px;
    margin-left: 0.25rem;
  }

  :global(.dark) .timeline-badge {
    background: #334155;
    color: #94a3b8;
  }

  .timeline-content {
    padding: 0.25rem 0 0.5rem 0.5rem;
  }

  .timeline-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .timeline-event {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    padding: 0.25rem 0;
    font-size: 0.75rem;
  }

  .event-date {
    flex-shrink: 0;
    color: #3b82f6;
    font-weight: 500;
    font-size: 0.6875rem;
  }

  :global(.dark) .event-date {
    color: #60a5fa;
  }

  .event-title {
    flex: 1;
    color: #475569;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.dark) .event-title {
    color: #94a3b8;
  }

  /* ── Checkbox group ────────────────────────────────────────── */
  .checkbox-group {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 0.5rem;
    max-height: 120px;
    overflow-y: auto;
  }

  .checkbox-label {
    font-size: 0.75rem;
    color: #475569;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    cursor: pointer;
  }

  :global(.dark) .checkbox-label {
    color: #94a3b8;
  }

  .checkbox-label input[type="checkbox"] {
    accent-color: #3b82f6;
  }

  /* ── Tab panel transition ──────────────────────────────────── */
  .tab-panel {
    animation: fadeIn 150ms ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateX(-4px); }
    to   { opacity: 1; transform: translateX(0); }
  }

  /* ── Help button ──────────────────────────────────────────── */
  .help-btn {
    width: 1.5rem;
    height: 1.5rem;
    padding: 0;
    border: 1px solid #e2e8f0;
    border-radius: 50%;
    background: #ffffff;
    font-size: 0.8125rem;
    font-weight: 700;
    color: #64748b;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 120ms, border-color 120ms, color 120ms;
  }

  .help-btn:hover {
    background: #f1f5f9;
    border-color: #94a3b8;
    color: #1e293b;
  }

  :global(.dark) .help-btn {
    background: #1e293b;
    border-color: #334155;
    color: #94a3b8;
  }
  :global(.dark) .help-btn:hover {
    background: #334155;
    border-color: #475569;
    color: #e2e8f0;
  }

  /* ── Help overlay ─────────────────────────────────────────── */
  .help-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fadeIn 120ms ease;
  }

  .help-panel {
    background: #ffffff;
    border-radius: 0.75rem;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.15);
    max-width: 520px;
    width: 90vw;
    max-height: 85vh;
    overflow-y: auto;
    padding: 1.5rem;
  }

  :global(.dark) .help-panel {
    background: #1e293b;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
  }

  .help-header {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    margin-bottom: 0.25rem;
  }

  .help-header h2 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
    color: #1e293b;
  }

  :global(.dark) .help-header h2 {
    color: #f1f5f9;
  }

  .help-version {
    font-size: 0.75rem;
    color: #94a3b8;
    font-weight: 500;
  }

  .help-close {
    margin-left: auto;
    background: none;
    border: none;
    font-size: 1.125rem;
    color: #94a3b8;
    cursor: pointer;
    padding: 0.25rem;
    line-height: 1;
  }

  .help-close:hover {
    color: #1e293b;
  }

  :global(.dark) .help-close:hover {
    color: #e2e8f0;
  }

  .help-creator {
    font-size: 0.8125rem;
    color: #64748b;
    margin: 0 0 1.25rem;
  }

  .help-creator a {
    color: #3b82f6;
    text-decoration: none;
  }

  .help-creator a:hover {
    text-decoration: underline;
  }

  .help-section {
    margin-bottom: 1rem;
  }

  .help-section h3 {
    font-size: 0.875rem;
    font-weight: 600;
    color: #1e293b;
    margin: 0 0 0.25rem;
  }

  :global(.dark) .help-section h3 {
    color: #e2e8f0;
  }

  .help-section p {
    font-size: 0.8125rem;
    color: #475569;
    margin: 0;
    line-height: 1.5;
  }

  :global(.dark) .help-section p {
    color: #94a3b8;
  }

  .help-shortcuts {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8125rem;
  }

  .help-shortcuts td {
    padding: 0.25rem 0;
    vertical-align: top;
    color: #475569;
  }

  .help-shortcuts td:first-child {
    white-space: nowrap;
    padding-right: 0.75rem;
  }

  :global(.dark) .help-shortcuts td {
    color: #94a3b8;
  }

  .help-shortcuts kbd {
    display: inline-block;
    padding: 0.125rem 0.375rem;
    font-size: 0.75rem;
    font-family: inherit;
    background: #f1f5f9;
    border: 1px solid #e2e8f0;
    border-radius: 0.25rem;
    color: #1e293b;
  }

  :global(.dark) .help-shortcuts kbd {
    background: #334155;
    border-color: #475569;
    color: #e2e8f0;
  }
</style>
