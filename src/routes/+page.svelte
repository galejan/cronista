<script lang="ts">
  import { untrack } from "svelte";
  import Editor from "$lib/components/Editor.svelte";
  import { debounce } from "$lib/debounce";
  import { t, setLang, lang } from "$lib/i18n";
  import {
    actualizarPersonaje,
    agregarEventoTimeline,
    cargarCapitulo,
    cargarIndice,
    cargarNota,
    cargarPersonaje,
    cargarTimeline,
    crearCapitulo,
    crearCheckpoint,
    crearNota,
    crearPersonaje,
    crearProyecto,
    detectarGit,
    eliminarCapitulo,
    eliminarEventoTimeline,
    exportarProyectoMd,
    exportarProyectoZip,
    eliminarNota,
    eliminarPersonaje,
    guardarCapitulo,
    inicializarGitConAutor,
    listarNotas,
    listarPersonajes,
    marcarProyectoCronista,
    obtenerGitLog,
    reordenarTimeline,
    setActiveProject,
    verificarGitInicializado,
  } from "$lib/tauri";
  import type { GitLogEntry } from "$lib/tauri";
  import { documentDir } from "@tauri-apps/api/path";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { PhysicalSize, PhysicalPosition } from "@tauri-apps/api/dpi";
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

  // ── Restore zoom level ────────────────────────────────────────
  $effect(() => {
    const stored = localStorage.getItem("cronista-zoom");
    if (stored) zoomLevel = Math.min(2, Math.max(0, Number(stored)));
  });

  // ── Apply zoom to the whole UI ─────────────────────────────────
  $effect(() => {
    const scales = [1, 1.15, 1.3];
    document.body.style.zoom = String(scales[zoomLevel] ?? 1);
  });
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

  // ── Window state persistence (Tauri only, fails silently elsewhere) ──

  async function saveWindowState(): Promise<void> {
    try {
      const win = getCurrentWindow();
      const [isMax, size, pos] = await Promise.all([
        win.isMaximized(),
        win.outerSize(),
        win.outerPosition(),
      ]);
      localStorage.setItem("cronista-window-maximized", String(isMax));
      localStorage.setItem(
        "cronista-window-size",
        JSON.stringify({ width: size.width, height: size.height }),
      );
      localStorage.setItem(
        "cronista-window-position",
        JSON.stringify({ x: pos.x, y: pos.y }),
      );
    } catch {
      // Not running inside Tauri (e.g. svelte-check).
    }
  }

  async function restoreWindowState(): Promise<void> {
    try {
      const win = getCurrentWindow();
      const storedMax = localStorage.getItem("cronista-window-maximized");
      if (storedMax === "true") {
        await win.maximize();
        return;
      }
      const storedSize = localStorage.getItem("cronista-window-size");
      const storedPos = localStorage.getItem("cronista-window-position");
      if (storedSize) {
        const { width, height } = JSON.parse(storedSize);
        await win.setSize(new PhysicalSize(width, height));
      }
      if (storedPos) {
        const { x, y } = JSON.parse(storedPos);
        await win.setPosition(new PhysicalPosition(x, y));
      }
    } catch {
      // Not running inside Tauri.
    }
  }

  // ── Restore sidebar width and window state on mount ───────────
  $effect(() => {
    // Restore sidebar width
    const savedPct = localStorage.getItem("cronista-sidebar-pct");
    if (savedPct) {
      const val = Number(savedPct);
      if (val >= 20 && val <= 60) {
        sidebarPct = val;
        sidebarSaved = val;
      }
    }

    // Restore window size/position (Tauri)
    restoreWindowState();

    // Keep saved window state current on resize/move
    try {
      const win = getCurrentWindow();
      let saveTimer: ReturnType<typeof setTimeout> | null = null;
      const debouncedSave = () => {
        if (saveTimer) clearTimeout(saveTimer);
        saveTimer = setTimeout(saveWindowState, 500);
      };
      win.onResized(debouncedSave);
      win.onMoved(debouncedSave);
    } catch {
      // Not in Tauri.
    }

    // Last-resort save on close
    window.addEventListener("beforeunload", () => {
      saveWindowState();
    });
  });

  // ── Persist sidebar width on every change ─────────────────────
  $effect(() => {
    if (sidebarPct > 0) {
      localStorage.setItem("cronista-sidebar-pct", String(sidebarPct));
    }
  });

  // ── Editor & project state ──────────────────────────────────
  let projectPath = $state("");
  let gitEnabled = $state(false);
  let gitStatus = $state<"active" | "unavailable" | "not-initialized" | "unknown">("unknown");
  let gitInitModal = $state(false);
  let gitInitNombre = $state("");
  let gitInitEmail = $state("");
  let gitHelpModal = $state(false);
  let gitLogVisible = $state(false);
  let gitLogEntries = $state<GitLogEntry[]>([]);
  let footerExpanded = $state(true);
  let zoomLevel = $state(0); // 0=normal, 1=medium, 2=large
  let exportModal = $state(false);
  let chapters = $state<string[]>([]);
  let pendingDelete = $state<string | null>(null);
  let activeChapter = $state("");
  let editorContent = $state("");
  let fontFamily = $state("monospace");
  let fontPickerOpen = $state(false);
  let fontPickerFont = $state("monospace");
  let fontPickerResolve = $state<((v: string) => void) | null>(null);
  let saveStatus = $state<"" | "saved" | "unsaved" | "saving">("");

  // ── Auto-commit on close (Tauri window event) ─────────────────
  // The Rust backend (on_window_event) handles the git checkpoint.
  // JS only shows the closing overlay and calls destroy().
  // No IPC calls from within the handler — avoids deadlock.
  let closing = $state(false);
  let closeStep = $state("");

  $effect(() => {
    let unlisten: (() => void) | undefined;

    try {
      const w = getCurrentWindow();
      console.log("[cronista:close] Registering onCloseRequested handler");

      w.onCloseRequested(async (event) => {
        const path = untrack(() => projectPath);
        const gitOk = untrack(() => gitEnabled);

        console.log("[cronista:close] ── Close requested ──");
        console.log("[cronista:close]   projectPath:", path || "(none)");
        console.log("[cronista:close]   gitEnabled:", gitOk);

        if (!path || !gitOk) {
          console.log("[cronista:close] → no project or no git, letting close through");
          return; // No preventDefault — window closes normally
        }

        if (untrack(() => closing)) {
          console.log("[cronista:close] → already closing, letting through");
          return;
        }

        closing = true;
        closeStep = "Cerrando aplicación...";
        console.log("[cronista:close] → showing overlay, Rust handles checkpoint");

        event.preventDefault(); // Keep window alive while overlay shows

        // Brief pause so user sees the overlay
        await new Promise(r => setTimeout(r, 500));

        // Force-close. Rust's on_window_event already did the checkpoint
        // (or is about to) on its own thread.
        console.log("[cronista:close] → destroying window");
        try {
          getCurrentWindow().destroy();
        } catch (e) {
          console.error("[cronista:close]   destroy FAILED:", e);
        }
      }).then((fn) => {
        unlisten = fn;
        console.log("[cronista:close] Handler registered successfully");
      }).catch((err) => {
        console.error("[cronista:close] Failed to register handler:", err);
      });
    } catch (err) {
      console.error("[cronista:close] Not in Tauri:", err);
    }

    return () => {
      console.log("[cronista:close] Effect cleanup — unregistering handler");
      unlisten?.();
    };
  });

  /** Editor component reference — exposes setContent(html) + heading control. */
  let editorRef = $state<{
    setContent(html: string): void;
    increaseHeading(): void;
    decreaseHeading(): void;
    insertPair(open: string, close: string): void;
    insertText(text: string): void;
    isFocused(): boolean;
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
  }, 20_000);

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
        alert(`${t("chapters.deleteError")} ${e}`);
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
        title: t("dialog.selectCreateFolder"),
        defaultPath: docsDir,
      });
      if (!selected) return;

      const gitDisponible = await detectarGit();
      if (!gitDisponible) {
        const continuar = confirm(
          t("git.notInstalled") + "\n\n" +
          t("git.notInstalledDesc") + "\n\n" +
          t("git.installInstructions") + "\n\n" +
          t("git.continueWithout")
        );
        if (!continuar) return;
      }

      const name = prompt(t("dialog.projectName"), t("dialog.projectNameDefault"));
      if (!name) return;

      // Font selection — show picker modal
      fontFamily = await pickFont();

      const path = `${selected}/${name.trim()}`;

      console.log("[cronista] Creating project:", { path, name });
      try {
        const msg = await crearProyecto(path, name.trim(), fontFamily);
        console.log("[cronista] Project created:", msg);
        projectPath = path;
        setActiveProject(path);
        marcarProyectoCronista(path); // fire-and-forget: set folder icon
        await actualizarGitStatus(path);
        await refreshChapters();
      } catch (e) {
        console.error("[cronista] Failed to create project:", e);
        alert(`${t("dialog.createProjectError")} ${e}`);
        return;
      }
    }

    const filename = prompt(t("chapters.newFilePrompt"), "0001_prologue.md");
    if (!filename) return;

    // Use the chapter filename (without .md) as the initial title
    const titulo = filename.replace(/\.md$/, "")
      .replace(/^[\d_]+/, "")
      .replace(/_/g, " ")
      .trim() || t("chapters.untitled");
    const initialHTML = `<h1>${titulo}</h1><p></p>`;

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
      alert(`${t("chapters.createError")} ${e}`);
    }
  }

  /** Open an existing project by loading its metadata.json. */
  async function abrirProyecto(): Promise<void> {
    // Close current project first if one is open
    if (projectPath) {
      await cerrarProyecto();
    }

    const docsDir = await documentDir();
    const selected = await open({
      directory: true,
      multiple: false,
      title: t("dialog.selectProjectFolder"),
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
      setActiveProject(path);
      fontFamily = meta.font_family || "monospace";
      await actualizarGitStatus(path);
      chapters = meta.chapters_order ?? [];
      console.log("[cronista] Project opened:", meta.project_name, chapters);

      // Warn if git is unavailable
      if (gitStatus === "unavailable") {
        console.warn("[cronista] Git not detected — automatic version control disabled");
        alert(t("git.notInstalled") + "\n\n" + t("git.notInstalledDesc"));
      }

      // Auto-load first chapter if there is one
      if (chapters.length > 0) {
        await cargarCapituloActual(chapters[0]);
      }
    } catch (e) {
      console.error("[cronista] Failed to open project:", e);
      alert(t("dialog.openProjectError") + `\n\n${e}`);
    }
  }

  /** Close current project and start the new-project setup flow. */
  async function nuevoProyectoHandler(): Promise<void> {
    if (projectPath) await cerrarProyecto();
    localStorage.removeItem("cronista-last-project");
    crearCapituloNuevo();
  }
  async function cerrarProyecto(): Promise<void> {
    if (!projectPath) return;

    // Save current chapter before closing
    if (activeChapter && editorContent) {
      try {
        await guardarCapitulo(projectPath, activeChapter, editorContent);
      } catch { /* silent */ }
    }

    // Tell Rust to stop tracking this project
    setActiveProject(null);

    // Clear all frontend state
    projectPath = "";
    chapters = [];
    activeChapter = "";
    editorContent = "";
    saveStatus = "";
    personajes = [];
    personajeExpandido = null;
    personajeEditando = null;
    notas = [];
    activeNote = "";
    timeline = [];
    timelineVisible = false;
    gitEnabled = false;
    gitStatus = "unknown";

    // Keep last project for auto-reopen on next launch
    // (deliberately NOT removing from localStorage)
  }

  /** Determine git status: active / unavailable / not-initialized */
  async function actualizarGitStatus(path: string): Promise<void> {
    try {
      const gitOk = await detectarGit();
      if (!gitOk) {
        gitStatus = "unavailable";
        gitEnabled = false;
        return;
      }
      const initialized = await verificarGitInicializado(path);
      if (initialized) {
        gitStatus = "active";
        gitEnabled = true;
      } else {
        gitStatus = "not-initialized";
        gitEnabled = true; // Binary exists, just needs init
      }
    } catch {
      gitStatus = "unknown";
    }
  }

  /** Load git log for the sessions panel. */
  async function cargarGitLog(): Promise<void> {
    if (!projectPath || gitStatus !== "active") return;
    try {
      gitLogEntries = await obtenerGitLog(projectPath, 5);
      gitLogVisible = true;
    } catch (e) {
      console.error("[cronista] Failed to load git log:", e);
    }
  }

  /** Show font picker modal and return selected font. */
  function pickFont(): Promise<string> {
    return new Promise((resolve) => {
      fontPickerFont = fontFamily;
      fontPickerOpen = true;
      fontPickerResolve = resolve;
    });
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
      alert(t("characters.nameRequired"));
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
      alert(`${t("characters.createError")} ${e}`);
    }
  }

  async function seleccionarPersonaje(id: string): Promise<void> {
    save.trigger(); // save current work first
    // Toggle: if already expanded, collapse; otherwise load and expand
    if (personajeExpandido === id) {
      personajeExpandido = null;
      personajeEditando = null;
      return;
    }
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
      alert(`${t("characters.saveError")} ${e}`);
    }
  }

  async function eliminarPersonajeHandler(id: string): Promise<void> {
    if (!confirm(t("characters.deleteConfirm"))) return;
    try {
      await eliminarPersonaje(projectPath, id);
      personajeExpandido = null;
      personajeEditando = null;
      await refreshPersonajes();
      await refreshTimeline();
    } catch (e) {
      console.error("[cronista] Delete character failed:", e);
      alert(`${t("characters.deleteError")} ${e}`);
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
    const title = prompt(t("notes.titlePrompt"));
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
      alert(`${t("notes.createError")} ${e}`);
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
    if (!confirm(t("notes.deleteConfirm"))) return;
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
      alert(`${t("notes.deleteError")} ${e}`);
    }
  }

  // ── Timeline CRUD ───────────────────────────────────────────

  async function refreshTimeline(): Promise<void> {
    if (!projectPath) return;
    try {
      const raw = await cargarTimeline(projectPath);
      timeline = JSON.parse(raw);
    } catch (e) {
      console.error("[cronista] Failed to load timeline:", e);
      timeline = [];
    }
  }

  async function agregarEventoHandler(): Promise<void> {
    if (!nuevoEventoFecha || !nuevoEventoTitulo) {
      alert(t("timeline.requiredFields"));
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
      alert(`${t("timeline.addError")} ${e}`);
    }
  }

  async function eliminarEventoHandler(id: string): Promise<void> {
    if (!confirm(t("timeline.deleteConfirm"))) return;
    try {
      await eliminarEventoTimeline(projectPath, id);
      await refreshTimeline();
    } catch (e) {
      console.error("[cronista] Delete timeline event failed:", e);
      alert(`${t("timeline.deleteError")} ${e}`);
    }
  }

  // ── Drag-and-drop reorder ────────────────────────────────────
  let dragId = $state<string | null>(null);

  function handleDragStart(e: DragEvent, id: string) {
    dragId = id;
    e.dataTransfer!.effectAllowed = "move";
    (e.currentTarget as HTMLElement).classList.add("dragging");
  }

  function handleDragEnd(e: DragEvent) {
    (e.currentTarget as HTMLElement).classList.remove("dragging");
    // Clean up all drag-over highlights
    document.querySelectorAll(".timeline-event.drag-over").forEach(el => {
      el.classList.remove("drag-over");
    });
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    (e.currentTarget as HTMLElement).classList.add("drag-over");
  }

  function handleDragLeave(e: DragEvent) {
    (e.currentTarget as HTMLElement).classList.remove("drag-over");
  }

  function handleDrop(e: DragEvent, targetId: string) {
    e.preventDefault();
    if (!dragId || dragId === targetId) return;

    const fromIdx = timeline.findIndex(evt => evt.id === dragId);
    const toIdx = timeline.findIndex(evt => evt.id === targetId);
    if (fromIdx === -1 || toIdx === -1) return;

    const reordered = [...timeline];
    const [moved] = reordered.splice(fromIdx, 1);
    reordered.splice(toIdx, 0, moved);
    timeline = reordered;
    dragId = null;

    // Save to backend
    const ids = reordered.map(evt => evt.id);
    reordenarTimeline(projectPath, ids).catch(e => console.error("Reorder failed:", e));
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

  // ── Persist current project path ─────────────────────────────
  $effect(() => {
    if (projectPath) {
      localStorage.setItem("cronista-last-project", projectPath);
    }
  });

  // ── Auto-reopen last project on startup ──────────────────────
  let reopeningStatus = $state("");
  $effect(() => {
    const lastPath = localStorage.getItem("cronista-last-project");
    if (!lastPath) return;

    reopeningStatus = t("setup.reopening");
    console.log("[cronista] Trying to reopen last project:", lastPath);

    cargarIndice(lastPath)
      .then(async (raw) => {
        const meta = JSON.parse(raw);
        projectPath = lastPath;
        setActiveProject(lastPath);
        fontFamily = meta.font_family || "monospace";
        await actualizarGitStatus(lastPath);
        chapters = meta.chapters_order ?? [];
        console.log("[cronista] Project reopened:", meta.project_name, chapters);

        if (chapters.length > 0) {
          return cargarCapituloActual(chapters[0]);
        }
      })
      .catch((e) => {
        console.error("[cronista] Failed to reopen last project:", e);
        localStorage.removeItem("cronista-last-project");
      })
      .finally(() => {
        reopeningStatus = "";
      });
  });

  // ── Keyboard shortcuts ──────────────────────────────────────
  function handleKeydown(e: KeyboardEvent) {
    // Ctrl+Shift+Left — collapse sidebar
    if (e.ctrlKey && e.shiftKey && e.key === "ArrowLeft") {
      e.preventDefault();
      sidebarSaved = sidebarPct;
      sidebarPct = 0;
      sidebarCollapsed = true;
      return;
    }

    // Ctrl+Shift+Right — restore sidebar
    if (e.ctrlKey && e.shiftKey && e.key === "ArrowRight") {
      e.preventDefault();
      sidebarCollapsed = false;
      sidebarPct = sidebarSaved || 40;
      return;
    }

    // Ctrl+Left — shrink sidebar by 5 % (min 20 %)
    if (e.ctrlKey && !e.shiftKey && e.key === "ArrowLeft") {
      e.preventDefault();
      sidebarCollapsed = false;
      sidebarPct = Math.max(20, sidebarPct - 5);
      sidebarSaved = sidebarPct;
      return;
    }

    // Ctrl+Right — grow sidebar by 5 % (max 60 %)
    if (e.ctrlKey && !e.shiftKey && e.key === "ArrowRight") {
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

    // Ctrl+Shift+N — new project (close current, then new)
    if (e.ctrlKey && e.shiftKey && e.key === "N") {
      e.preventDefault();
      cerrarProyecto().then(() => {
        localStorage.removeItem("cronista-last-project");
        crearCapituloNuevo();
      });
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

    // Ctrl+P — toggle footer panel
    if (e.ctrlKey && !e.shiftKey && e.key === "p") {
      e.preventDefault();
      footerExpanded = !footerExpanded;
      return;
    }

    // Ctrl+= (Ctrl++) — zoom in
    if (e.ctrlKey && !e.shiftKey && (e.key === "=" || e.key === "+")) {
      e.preventDefault();
      zoomLevel = Math.min(2, zoomLevel + 1);
      localStorage.setItem("cronista-zoom", String(zoomLevel));
      return;
    }

    // Ctrl+- — zoom out
    if (e.ctrlKey && !e.shiftKey && e.key === "-") {
      e.preventDefault();
      zoomLevel = Math.max(0, zoomLevel - 1);
      localStorage.setItem("cronista-zoom", String(zoomLevel));
      return;
    }

    // ? — help toggle (without shift, plain key)
    if (!e.ctrlKey && !e.altKey && !e.metaKey && e.key === "?") {
      e.preventDefault();
      helpMode = !helpMode;
      return;
    }

    // Ctrl+Up — increase heading level (paragraph → H2 → H1)
    if (e.ctrlKey && !e.shiftKey && e.key === "ArrowUp") {
      e.preventDefault();
      editorRef?.increaseHeading();
      return;
    }

    // Ctrl+Down — decrease heading level (H1 → H2 → paragraph)
    if (e.ctrlKey && !e.shiftKey && e.key === "ArrowDown") {
      e.preventDefault();
      editorRef?.decreaseHeading();
      return;
    }

    // ── Editor inserts ─────────────────────────────────────────
    // Ctrl+D → dialogue dash pair: —|— (cursor between dashes)
    if (e.ctrlKey && !e.shiftKey && (e.key === "d" || e.key === "D")) {
      if (editorRef?.isFocused()) {
        e.preventDefault();
        editorRef.insertPair("—", "—");
        return;
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="app-layout"
  class:sidebar-collapsed={sidebarCollapsed}
  style:grid-template-columns={sidebarCollapsed ? "0 100%" : `${sidebarPct}% ${100 - sidebarPct}%`}
>
  <!-- Sidebar (40 % when visible) — placeholder, not modified per spec -->
  <aside class="sidebar">
    <nav class="tabs">
      <button
        class="tab"
        class:active={activeTab === "capitulos"}
        onclick={() => { pendingDelete = null; activeTab = "capitulos"; activeNote = ""; }}
      >{t("tabs.chapters")}</button>
      <button
        class="tab"
        class:active={activeTab === "personajes"}
        onclick={() => { pendingDelete = null; activeTab = "personajes"; }}
      >{t("tabs.characters")}</button>
      <button
        class="tab"
        class:active={activeTab === "notas"}
        onclick={() => { pendingDelete = null; activeTab = "notas"; }}
      >{t("tabs.notes")}</button>
    </nav>

    <div class="sidebar-content">
      <!-- ═══ Capítulos tab ═══ -->
      {#if activeTab === "capitulos"}
        <div class="tab-panel">
          <button class="btn-add" onclick={() => crearCapituloNuevo()}>
            {t("toolbar.newChapter")}
          </button>
          {#if chapters.length > 0}
            <p class="chapter-list-label">{t("chapters.label")}</p>
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
                      title={t("chapters.confirmDeleteTitle")}
                      onclick={() => eliminarCapituloHandler(ch)}
                    >{t("chapters.confirmDelete")}</button>
                  {:else}
                    <button
                      class="item-delete"
                      title={t("chapters.deleteTitle")}
                      onclick={() => eliminarCapituloHandler(ch)}
                    >×</button>
                  {/if}
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-hint">{t("chapters.empty")}</p>
          {/if}
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
                      <label class="field-label" for="char-name-{p.id}">{t("characters.name")}</label>
                      <input
                        id="char-name-{p.id}"
                        class="field-input"
                        type="text"
                        bind:value={personajeEditando.name}
                      />

                      <label class="field-label" for="char-desc-{p.id}">{t("characters.physicalDescription")}</label>
                      <textarea
                        id="char-desc-{p.id}"
                        class="field-textarea"
                        bind:value={personajeEditando.physicalDescription}
                        rows="2"
                      ></textarea>

                      <label class="field-label" for="char-pers-{p.id}">{t("characters.personality")}</label>
                      <textarea
                        id="char-pers-{p.id}"
                        class="field-textarea"
                        bind:value={personajeEditando.personality}
                        rows="2"
                      ></textarea>

                      <label class="field-label" for="char-trau-{p.id}">{t("characters.traumas")}</label>
                      <textarea
                        id="char-trau-{p.id}"
                        class="field-textarea"
                        bind:value={personajeEditando.traumas}
                        rows="2"
                      ></textarea>

                      <label class="field-label" for="char-rel-{p.id}">{t("characters.relationships")}</label>
                      {#if personajeEditando.relationships?.length > 0}
                        {#each personajeEditando.relationships as rel, ri}
                          <div class="relationship-row">
                            <input
                              class="field-input small"
                              type="text"
                              placeholder={t("characters.relName")}
                              bind:value={rel.targetName}
                            />
                            <input
                              class="field-input small"
                              type="text"
                              placeholder={t("characters.relType")}
                              bind:value={rel.type}
                            />
                            <input
                              class="field-input small"
                              type="text"
                              placeholder={t("characters.relNotes")}
                              bind:value={rel.notes}
                            />
                            <button
                              class="btn-sm btn-danger"
                              onclick={() => eliminarRelacionPersonaje(ri)}
                            >×</button>
                          </div>
                        {/each}
{/if}

{#if exportModal}
  <!-- Export modal -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" role="dialog" tabindex="-1"
    aria-label={t("export.title")}
    onclick={() => (exportModal = false)}
    onkeydown={(e) => e.key === "Escape" && (exportModal = false)}>
    <div class="modal-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2>📦 {t("export.title")}</h2>
      <p class="modal-desc">{t("export.desc")}</p>

      <div class="export-options">
        <button class="export-option" onclick={async () => {
          try {
            const result = await exportarProyectoZip(projectPath);
            exportModal = false;
            alert(t("export.zipSuccess") + "\n" + result);
          } catch (e) {
            alert(t("export.error") + " " + e);
          }
        }}>
          <span class="export-option-icon">🗜️</span>
          <span class="export-option-title">{t("export.zipTitle")}</span>
          <span class="export-option-hint">{t("export.zipHint")}</span>
        </button>

        <button class="export-option" onclick={async () => {
          try {
            const result = await exportarProyectoMd(projectPath);
            exportModal = false;
            alert(t("export.mdSuccess") + "\n" + result);
          } catch (e) {
            alert(t("export.error") + " " + e);
          }
        }}>
          <span class="export-option-icon">📄</span>
          <span class="export-option-title">{t("export.mdTitle")}</span>
          <span class="export-option-hint">{t("export.mdHint")}</span>
        </button>
      </div>

      <div class="modal-actions">
        <button class="btn-secondary" onclick={() => (exportModal = false)}>
          {t("common.cancel")}
        </button>
      </div>
    </div>
  </div>
{/if}
                      <button class="btn-sm" onclick={agregarRelacionPersonaje}>
                        {t("characters.addRelationship")}
                      </button>

                      <div class="form-actions">
                        <button class="btn-sm btn-primary" onclick={guardarPersonaje}>{t("characters.save")}</button>
                        <button class="btn-sm btn-danger" onclick={() => eliminarPersonajeHandler(p.id)}>{t("characters.delete")}</button>
                      </div>
                    </div>
                  {/if}
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-hint">{t("characters.empty")}</p>
          {/if}

          {#if personajeFormVisible}
            <div class="inline-form">
              <input
                class="field-input"
                type="text"
                placeholder={t("characters.namePlaceholder")}
                bind:value={personajeNuevoNombre}
                onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter") crearPersonajeHandler(); }}
              />
              <div class="form-actions">
                <button class="btn-sm btn-primary" onclick={crearPersonajeHandler}>{t("characters.create")}</button>
                <button class="btn-sm" onclick={() => personajeFormVisible = false}>{t("common.cancel")}</button>
              </div>
            </div>
          {:else}
            <button class="btn-add" onclick={() => personajeFormVisible = true}>
              {t("characters.new")}
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
                    title={t("notes.deleteTitle")}
                    onclick={() => eliminarNotaHandler(n.id)}
                  >×</button>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-hint">{t("notes.empty")}</p>
          {/if}

          {#if activeNote}
            <div class="inline-form">
              <label class="field-label" for="note-title">{t("notes.titleLabel")}</label>
              <input
                id="note-title"
                class="field-input"
                type="text"
                bind:value={notaTitulo}
              />
              <div class="form-actions">
                <button class="btn-sm btn-primary" onclick={guardarNotaActual}>{t("notes.save")}</button>
                <button
                  class="btn-sm"
                  onclick={() => { activeNote = ""; notaTitulo = ""; }}
                >{t("notes.close")}</button>
              </div>
            </div>
          {/if}

          <button class="btn-add" onclick={() => crearNotaHandler()}>
            {t("notes.new")}
          </button>
        </div>
      {/if}

      <!-- ═══ Timeline — collapsible section at bottom ═══ -->
      <div class="timeline-section">
        <button
          class="timeline-toggle"
          onclick={() => { timelineVisible = !timelineVisible; if (timelineVisible) refreshTimeline(); }}
        >
          {timelineVisible ? "▼" : "▶"} {t("timeline.title")}
          {#if timeline.length > 0}
            <span class="timeline-badge">{timeline.length}</span>
          {/if}
        </button>

        {#if timelineVisible}
          <div class="timeline-content">
            {#if timeline.length > 0}
              <ul class="timeline-list">
                {#each timeline as evt}
                  <li
                    class="timeline-event"
                    draggable="true"
                    ondragstart={(e) => handleDragStart(e, evt.id)}
                    ondragend={(e) => handleDragEnd(e)}
                    ondragover={(e) => handleDragOver(e)}
                    ondragleave={(e) => handleDragLeave(e)}
                    ondrop={(e) => handleDrop(e, evt.id)}
                  >
                    <span class="event-date">{evt.date}</span>
                    <span class="event-title">{evt.title}</span>
                    <button
                      class="item-delete"
                      title={t("timeline.deleteTitle")}
                      onclick={() => eliminarEventoHandler(evt.id)}
                    >×</button>
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="empty-hint">{t("timeline.empty")}</p>
            {/if}

            {#if eventoFormVisible}
              <div class="inline-form">
                <label class="field-label" for="evt-date">{t("timeline.date")}</label>
                <input
                  id="evt-date"
                  class="field-input"
                  type="date"
                  bind:value={nuevoEventoFecha}
                />
                <label class="field-label" for="evt-title">{t("timeline.eventTitle")}</label>
                <input
                  id="evt-title"
                  class="field-input"
                  type="text"
                  bind:value={nuevoEventoTitulo}
                  placeholder={t("timeline.titlePlaceholder")}
                />
                <label class="field-label" for="evt-desc">{t("timeline.description")}</label>
                <textarea
                  id="evt-desc"
                  class="field-textarea"
                  bind:value={nuevoEventoDescripcion}
                  rows="2"
                  placeholder={t("timeline.descriptionPlaceholder")}
                ></textarea>

                {#if personajes.length > 0}
                  <span class="field-label">{t("timeline.relatedCharacters")}</span>
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
                  <span class="field-label">{t("timeline.relatedChapters")}</span>
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
                  <button class="btn-sm btn-primary" onclick={agregarEventoHandler}>{t("timeline.add")}</button>
                  <button class="btn-sm" onclick={() => eventoFormVisible = false}>{t("common.cancel")}</button>
                </div>
              </div>
            {:else}
              <button class="btn-add" onclick={() => eventoFormVisible = true}>
                {t("timeline.newEvent")}
              </button>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <!-- Sidebar footer — tools + git, collapsible -->
    <div class="sidebar-footer">
      <button
        class="footer-toggle"
        onclick={() => (footerExpanded = !footerExpanded)}
        title={footerExpanded ? t("toolbar.collapseFooter") : t("toolbar.expandFooter")}
      >
        {footerExpanded ? "▼" : "▲"}
      </button>

      {#if footerExpanded}
        <div class="footer-rows">
          <!-- Row 1: language + theme -->
          <div class="footer-row">
            <button
              class="footer-btn footer-lang"
              class:active={$lang === "es"}
              onclick={() => setLang("es")}
              title="Español"
            >ES</button>
            <button
              class="footer-btn footer-lang"
              class:active={$lang === "en"}
              onclick={() => setLang("en")}
              title="English"
            >EN</button>
            <span class="footer-sep"></span>
            <button
              class="footer-btn"
              onclick={() => (theme = theme === "light" ? "dark" : "light")}
              title={theme === "light" ? t("toolbar.darkMode") : t("toolbar.lightMode")}
            >{theme === "light" ? "🌙" : "☀️"}</button>
          </div>

          <!-- Row 2: project management -->
          <div class="footer-row">
            <button class="footer-btn" onclick={() => abrirProyecto()} title={t("toolbar.openProjectTitle")}>
              📂 {t("toolbar.openProject")}
            </button>
            <button class="footer-btn" onclick={nuevoProyectoHandler} title={t("toolbar.newProjectTitle")}>
              ✨ {t("toolbar.newProject")}
            </button>
            <button class="footer-btn" onclick={() => cerrarProyecto()} title={t("toolbar.closeProjectTitle")}>
              ✕ {t("toolbar.closeProject")}
            </button>
            <span class="footer-sep"></span>
            <button class="footer-btn" onclick={() => (exportModal = true)} title={t("export.title")}>
              📦 {t("export.export")}
            </button>
          </div>

          <!-- Row 3: save -->
          <div class="footer-row">
            <button
              class="footer-btn"
              onclick={() => { saveStatus = "saving"; save.trigger(); }}
              title={t("toolbar.saveTitle")}
            >💾 {t("toolbar.save")}</button>
            <span class="save-indicator"
              class:saving={saveStatus === "saving"}
              class:saved={saveStatus === "saved"}
              class:unsaved={saveStatus === "unsaved"}
            >
              {saveStatus === "saving"
                ? t("toolbar.saving")
                : saveStatus === "saved"
                  ? t("toolbar.saved")
                  : saveStatus === "unsaved"
                    ? t("toolbar.unsaved")
                    : ""}
            </span>
          </div>

          <!-- Row 4: versioning -->
          {#if gitStatus !== "unknown"}
            <div class="footer-row">
              {#if gitStatus === "active"}
                <span class="git-indicator git-active" title={t("git.activeTitle")}>🟢 {t("git.active")}</span>
                <button class="git-log-link" onclick={cargarGitLog}>{t("git.viewSessions")} →</button>
              {:else if gitStatus === "not-initialized"}
                <button class="git-indicator git-warn"
                  onclick={() => { gitInitNombre = t("git.defaultName"); gitInitEmail = t("git.defaultEmail"); gitInitModal = true; }}
                  title={t("git.notInitTitle")}>🟠 {t("git.notInit")}</button>
              {:else if gitStatus === "unavailable"}
                <button class="git-indicator git-off"
                  onclick={() => (gitHelpModal = true)}
                  title={t("git.unavailableTitle")}>🔴 {t("git.unavailable")}</button>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </aside>

  <!-- Editor area (60 % when visible, 100 % when sidebar collapsed) -->
  <main class="editor">
    {#if !projectPath}
      {#if reopeningStatus}
        <div class="setup-prompt">
          <p class="setup-text">{reopeningStatus}</p>
        </div>
      {:else}
      <!-- First launch: prompt for project path -->
      <div class="setup-prompt">
        <p class="setup-text">{t("setup.selectFolder")}</p>
        <div class="setup-actions">
          <button
            class="btn-primary"
            onclick={() => crearCapituloNuevo()}
          >
            {t("setup.newProject")}
          </button>
          <button
            class="btn-secondary"
            onclick={() => abrirProyecto()}
          >
            {t("setup.openProject")}
          </button>
        </div>
      </div>
      {/if}
    {:else}
      <!-- Toolbar + Editor -->
      <div class="editor-pane">
        <div class="editor-toolbar">
          <span class="project-label" title={projectPath}>
            {projectPath.split("/").pop() || projectPath}
          </span>
          {#if activeChapter}
            <span class="chapter-label">{activeChapter}</span>
          {:else}
            <span></span>
          {/if}
          <button
            class="help-btn"
            onclick={() => (helpMode = !helpMode)}
            title={t("toolbar.helpTitle")}
          >?</button>
        </div>

        <div class="editor-body">
          <Editor
            bind:this={editorRef}
            content={editorContent}
            {fontFamily}
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
    tabindex="-1"
    aria-label={t("help.ariaLabel")}
    onclick={() => (helpMode = false)}
    onkeydown={(e) => e.key === "Escape" && (helpMode = false)}
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="help-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <div class="help-header">
        <h2>Cronista</h2>
        <span class="help-version">v0.1.1</span>
        <button class="help-close" onclick={() => (helpMode = false)}>✕</button>
      </div>

      <p class="help-creator">{t("help.createdBy")} <a href="mailto:galejan@gmail.com">galejan@gmail.com</a></p>

      <div class="help-section">
        <h3>{t("help.editorTitle")}</h3>
        <p>{t("help.editorDesc")}</p>
      </div>

      <div class="help-section">
        <h3>{t("help.chaptersTitle")}</h3>
        <p>{@html t("help.chaptersDesc")}</p>
      </div>

      <div class="help-section">
        <h3>{t("help.charactersTitle")}</h3>
        <p>{t("help.charactersDesc")}</p>
      </div>

      <div class="help-section">
        <h3>{t("help.notesTitle")}</h3>
        <p>{t("help.notesDesc")}</p>
      </div>

      <div class="help-section">
        <h3>{t("help.timelineTitle")}</h3>
        <p>{t("help.timelineDesc")}</p>
      </div>

      <div class="help-section">
        <h3>{t("help.shortcutsTitle")}</h3>
        <table class="help-shortcuts">
          <tbody>
          <tr><td><kbd>Ctrl+Shift+←</kbd></td><td>{t("help.shortcuts.toggleSidebar")}</td></tr>
          <tr><td><kbd>Ctrl+Shift+→</kbd></td><td>{t("help.shortcuts.restoreSidebar")}</td></tr>
          <tr><td><kbd>Ctrl+←</kbd> / <kbd>Ctrl+→</kbd></td><td>{t("help.shortcuts.resizeSidebar")}</td></tr>
          <tr><td><kbd>Ctrl+P</kbd></td><td>{t("help.shortcuts.toggleFooter")}</td></tr>
          <tr><td><kbd>Ctrl+S</kbd></td><td>{t("help.shortcuts.saveNow")}</td></tr>
          <tr><td><kbd>Ctrl+N</kbd></td><td>{t("help.shortcuts.newChapter")}</td></tr>
          <tr><td><kbd>Ctrl+O</kbd></td><td>{t("help.shortcuts.openProject")}</td></tr>
          <tr><td><kbd>Ctrl+Shift+N</kbd></td><td>{t("help.shortcuts.newProject")}</td></tr>
          <tr><td><kbd>Ctrl+↑</kbd> / <kbd>Ctrl+↓</kbd></td><td>{t("help.shortcuts.applyHeading")}</td></tr>
          <tr><td><kbd>Ctrl+=</kbd> / <kbd>Ctrl+-</kbd></td><td>{t("help.shortcuts.zoomIn")} / {t("help.shortcuts.zoomOut")}</td></tr>
          <tr><td><kbd>F11</kbd></td><td>{t("help.shortcuts.fullscreen")}</td></tr>
          <tr><td><kbd>F1</kbd> o <kbd>?</kbd></td><td>{t("help.shortcuts.toggleHelp")}</td></tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
{/if}

{#if gitInitModal}
  <!-- Git init modal — configure author and initialize -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    tabindex="-1"
    aria-label={t("git.initModalTitle")}
    onkeydown={(e) => e.key === "Escape" && (gitInitModal = false)}
  >
    <div class="modal-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2>{t("git.initModalTitle")}</h2>
      <p class="modal-desc">{t("git.initModalDesc")}</p>

      <label class="modal-field">
        {t("git.initModalName")}
        <input
          type="text"
          bind:value={gitInitNombre}
          class="modal-input"
        />
      </label>

      <label class="modal-field">
        {t("git.initModalEmail")}
        <input
          type="email"
          bind:value={gitInitEmail}
          class="modal-input"
        />
      </label>

      <div class="modal-actions">
        <button class="btn-secondary" onclick={() => (gitInitModal = false)}>
          {t("common.cancel")}
        </button>
        <button
          class="btn-primary"
          onclick={async () => {
            try {
              await inicializarGitConAutor(projectPath, gitInitNombre, gitInitEmail);
              await actualizarGitStatus(projectPath);
              gitInitModal = false;
              alert(t("git.initSuccess"));
            } catch (e) {
              alert(t("git.initError") + " " + e);
            }
          }}
        >
          {t("git.initButton")}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if gitHelpModal}
  <!-- Git help panel — shown when git is not installed -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    tabindex="-1"
    aria-label={t("git.helpTitle")}
    onclick={() => (gitHelpModal = false)}
    onkeydown={(e) => e.key === "Escape" && (gitHelpModal = false)}
  >
    <div class="modal-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2>{t("git.helpTitle")}</h2>

      <div class="help-section">
        <h3>{t("git.helpWhy")}</h3>
        <p>{t("git.helpWhyDesc")}</p>
      </div>

      <div class="help-section">
        <h3>{t("git.helpInstall")}</h3>
        <p>{@html t("git.helpInstallDesc")}</p>
      </div>

      <div class="modal-actions">
        <button class="btn-primary" onclick={() => (gitHelpModal = false)}>
          {t("git.helpClose")}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if fontPickerOpen}
  <!-- Font picker modal with live preview -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    tabindex="-1"
    aria-label="Elegir tipo de letra"
    onkeydown={(e) => e.key === "Escape" && (fontPickerOpen = false)}
  >
    <div class="modal-panel font-picker-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2>{t("dialog.fontTitle")}</h2>
      <p class="modal-desc">{t("dialog.fontDesc")}</p>

      <div class="font-picker-layout">
        <div class="font-picker-options">
          <label class="font-option" class:selected={fontPickerFont === "monospace"}>
            <input type="radio" bind:group={fontPickerFont} value="monospace" />
            <span class="font-option-name">{t("dialog.fontMono")}</span>
            <span class="font-option-hint">{t("dialog.fontMonoHint")}</span>
          </label>
          <label class="font-option" class:selected={fontPickerFont === "serif"}>
            <input type="radio" bind:group={fontPickerFont} value="serif" />
            <span class="font-option-name">{t("dialog.fontSerif")}</span>
            <span class="font-option-hint">{t("dialog.fontSerifHint")}</span>
          </label>
          <label class="font-option" class:selected={fontPickerFont === "sans-serif"}>
            <input type="radio" bind:group={fontPickerFont} value="sans-serif" />
            <span class="font-option-name">{t("dialog.fontSans")}</span>
            <span class="font-option-hint">{t("dialog.fontSansHint")}</span>
          </label>
        </div>

        <div class="font-preview" style:font-family={fontPickerFont === "monospace"
            ? "'Courier New', 'Fira Code', monospace"
            : fontPickerFont === "serif"
              ? "Georgia, 'Times New Roman', serif"
              : "'Inter', 'Segoe UI', sans-serif"}>
          <h3 class="font-preview-h1">El comienzo</h3>
          <p class="font-preview-text">
            La niebla se alzaba perezosa sobre los tejados de París cuando
            el reloj de Notre-Dame dio las siete. El inspector dejó la taza
            de café sobre el escritorio y se asomó a la ventana.
          </p>
        </div>
      </div>

      <div class="modal-actions">
        <button class="btn-secondary" onclick={() => { fontPickerOpen = false; fontPickerResolve?.("monospace"); }}>
          {t("common.cancel")}
        </button>
        <button class="btn-primary" onclick={() => {
          const chosen = fontPickerFont;
          fontPickerOpen = false;
          fontPickerResolve?.(chosen);
        }}>
          {t("dialog.fontConfirm")}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if gitLogVisible}
  <!-- Git log sessions panel -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    tabindex="-1"
    aria-label={t("git.sessionsTitle")}
    onclick={() => (gitLogVisible = false)}
    onkeydown={(e) => e.key === "Escape" && (gitLogVisible = false)}
  >
    <div class="modal-panel git-log-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2>📜 {t("git.sessionsTitle")}</h2>
      <p class="modal-desc">{@html t("git.sessionsDesc")}</p>

      {#if gitLogEntries.length === 0}
        <p class="git-log-empty">{t("git.sessionsEmpty")}</p>
      {:else}
        <div class="git-log-list">
          {#each gitLogEntries as entry}
            <div class="git-log-entry">
              <div class="git-log-header">
                <code class="git-log-hash">{entry.hash}</code>
                <span class="git-log-words">{entry.words}</span>
              </div>
              <p class="git-log-message">{entry.message}</p>
              {#if entry.files.length > 0}
                <div class="git-log-files">
                  {#each entry.files as file}
                    <span class="git-log-file-badge">📄 {file}</span>
                  {/each}
                </div>
              {/if}
              <time class="git-log-date">{entry.date}</time>
            </div>
          {/each}
        </div>
      {/if}

      <div class="modal-actions">
        <button class="btn-primary" onclick={() => (gitLogVisible = false)}>
          {t("git.sessionsClose")}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if closing}
  <!-- Closing overlay — shown while saving + checkpoint before destroy -->
  <div class="closing-overlay" role="alertdialog" aria-label="Cerrando aplicación">
    <div class="closing-panel">
      <div class="closing-spinner"></div>
      <p class="closing-message">{closeStep}</p>
      <p class="closing-sub">Cronista v0.1.1</p>
    </div>
  </div>
{/if}

<style>
  /* ── Prevent overscroll bounce on the whole window ──────────── */
  :global(html),
  :global(body) {
    overflow: hidden;
    overscroll-behavior: none;
    height: 100%;
  }

  /* ── Layout ────────────────────────────────────────────────── */
  .app-layout {
    display: grid;
    height: 100vh;
    overflow: hidden;
    overscroll-behavior: none;
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
    height: 2.5rem;
    box-shadow: inset 0 -1px 0 0 #e2e8f0;
  }

  :global(.dark) .tabs {
    box-shadow: inset 0 -1px 0 0 #334155;
  }

  .tab {
    flex: 1;
    padding: 0 0.5rem;
    font-size: 0.8125rem;
    font-weight: 500;
    color: #64748b;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    transition: color 150ms, border-color 150ms;
    display: flex;
    align-items: center;
    justify-content: center;
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
    overscroll-behavior: contain;
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
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    padding: 0 1rem;
    border-bottom: 1px solid #e2e8f0;
    background: #f8fafc;
    flex-shrink: 0;
    height: 2.5rem;
  }

  :global(.dark) .editor-toolbar {
    background: #0f172a;
    border-bottom-color: #334155;
  }

  .project-label {
    font-size: 0.8125rem;
    font-weight: 600;
    color: #1e293b;
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    justify-self: start;
  }

  :global(.dark) .project-label {
    color: #e2e8f0;
  }

  .chapter-label {
    font-size: 0.75rem;
    color: #64748b;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.dark) .chapter-label {
    color: #94a3b8;
  }

  .help-btn {
    justify-self: end;
    width: 1.5rem;
    height: 1.5rem;
    padding: 0;
    border: 1px solid #e2e8f0;
    border-radius: 50%;
    background: transparent;
    font-size: 0.75rem;
    font-weight: 700;
    color: #64748b;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 120ms;
  }

  .help-btn:hover {
    background: #e2e8f0;
  }

  :global(.dark) .help-btn {
    border-color: #334155;
    color: #94a3b8;
  }

  :global(.dark) .help-btn:hover {
    background: #334155;
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
    overflow-y: auto;
    overscroll-behavior: contain;
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
    cursor: grab;
  }
  .timeline-event:global(.dragging) { opacity: 0.4; }
  .timeline-event:global(.drag-over) { border-top: 2px solid #3b82f6; }

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

  /* ── Closing overlay ────────────────────────────────────────── */
  .closing-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fadeIn 120ms ease;
    backdrop-filter: blur(4px);
  }

  .closing-panel {
    background: #ffffff;
    border-radius: 1rem;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
    padding: 2.5rem 3rem;
    text-align: center;
    min-width: 280px;
  }

  :global(.dark) .closing-panel {
    background: #1e293b;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }

  .closing-spinner {
    width: 40px;
    height: 40px;
    margin: 0 auto 1.25rem;
    border: 3px solid #e2e8f0;
    border-top-color: #3b82f6;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  :global(.dark) .closing-spinner {
    border-color: #334155;
    border-top-color: #60a5fa;
  }

  .closing-message {
    margin: 0 0 0.5rem;
    font-size: 1rem;
    font-weight: 600;
    color: #1e293b;
  }

  :global(.dark) .closing-message {
    color: #f1f5f9;
  }

  .closing-sub {
    margin: 0;
    font-size: 0.75rem;
    color: #94a3b8;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  /* ── Git indicator ──────────────────────────────────────────── */
  .git-indicator {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.7rem;
    font-weight: 600;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    white-space: nowrap;
    border: 1px solid transparent;
    background: none;
    cursor: default;
  }

  button.git-indicator {
    cursor: pointer;
  }

  button.git-indicator:hover {
    filter: brightness(0.95);
  }

  .git-active {
    background: #dcfce7;
    color: #166534;
    border-color: #86efac;
  }

  :global(.dark) .git-active {
    background: #052e16;
    color: #86efac;
    border-color: #166534;
  }

  .git-warn {
    background: #ffedd5;
    color: #9a3412;
    border-color: #fdba74;
  }

  :global(.dark) .git-warn {
    background: #451a03;
    color: #fdba74;
    border-color: #9a3412;
  }

  .git-off {
    background: #fee2e2;
    color: #991b1b;
    border-color: #fca5a5;
  }

  :global(.dark) .git-off {
    background: #450a0a;
    color: #fca5a5;
    border-color: #991b1b;
  }

  /* ── Generic modal overlay ──────────────────────────────────── */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 150;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fadeIn 120ms ease;
  }

  .modal-panel {
    background: #ffffff;
    border-radius: 0.75rem;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
    max-width: 440px;
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

  /* ── Font picker ─────────────────────────────────────────────── */
  .font-picker-panel {
    max-width: 580px;
  }

  .font-picker-layout {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin: 1rem 0;
  }

  .font-picker-options {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .font-option {
    display: flex;
    flex-direction: column;
    padding: 0.6rem 0.75rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.5rem;
    cursor: pointer;
    transition: border-color 120ms, background 120ms;
  }

  .font-option.selected {
    border-color: #3b82f6;
    background: #eff6ff;
  }

  :global(.dark) .font-option.selected {
    border-color: #60a5fa;
    background: #1e3a5f;
  }

  .font-option input { display: none; }

  .font-option-name {
    font-size: 0.875rem;
    font-weight: 600;
    color: #1e293b;
  }

  :global(.dark) .font-option-name {
    color: #f1f5f9;
  }

  .font-option-hint {
    font-size: 0.6875rem;
    color: #94a3b8;
    margin-top: 0.15rem;
  }

  .font-preview {
    border: 1px solid #e2e8f0;
    border-radius: 0.5rem;
    padding: 1rem;
    background: #fafafa;
    overflow: hidden;
  }

  :global(.dark) .font-preview {
    background: #0f172a;
    border-color: #334155;
  }

  .font-preview-h1 {
    font-size: 1.3rem;
    font-weight: 700;
    margin: 0 0 0.5rem;
    color: #1e293b;
  }

  :global(.dark) .font-preview-h1 {
    color: #f1f5f9;
  }

  .font-preview-text {
    font-size: 0.75rem;
    line-height: 1.6;
    color: #475569;
    margin: 0;
  }

  :global(.dark)   .font-preview-text {
    color: #94a3b8;
  }

  /* ── Export options ──────────────────────────────────────────── */
  .export-options {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin: 0.5rem 0;
  }

  .export-option {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    padding: 0.75rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.5rem;
    background: transparent;
    cursor: pointer;
    text-align: left;
    transition: border-color 120ms, background 120ms;
  }

  .export-option:hover {
    border-color: #3b82f6;
    background: #eff6ff;
  }

  :global(.dark) .export-option:hover {
    border-color: #60a5fa;
    background: #1e3a5f;
  }

  .export-option-icon {
    font-size: 1.2rem;
    flex-shrink: 0;
    margin-top: 0.1rem;
  }

  .export-option-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: #1e293b;
  }

  :global(.dark) .export-option-title {
    color: #f1f5f9;
  }

  .export-option-hint {
    font-size: 0.7rem;
    color: #94a3b8;
    margin-top: 0.15rem;
  }

  /* ── Git log sessions panel ──────────────────────────────────── */
  .git-log-link {
    font-size: 0.7rem;
    font-weight: 500;
    color: #3b82f6;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .git-log-link:hover {
    color: #2563eb;
  }

  :global(.dark) .git-log-link {
    color: #60a5fa;
  }

  :global(.dark) .git-log-link:hover {
    color: #93bbfd;
  }

  .git-log-panel {
    max-width: 480px;
  }

  .git-log-empty {
    text-align: center;
    color: #94a3b8;
    font-size: 0.875rem;
    padding: 1.5rem 0;
  }

  .git-log-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    max-height: 50vh;
    overflow-y: auto;
    margin-bottom: 0.5rem;
  }

  .git-log-entry {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 0.5rem;
    padding: 0.75rem;
  }

  :global(.dark) .git-log-entry {
    background: #0f172a;
    border-color: #334155;
  }

  .git-log-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.25rem;
  }

  .git-log-hash {
    font-size: 0.6875rem;
    font-family: "SF Mono", "Fira Code", monospace;
    color: #64748b;
    background: #e2e8f0;
    padding: 0.1rem 0.35rem;
    border-radius: 0.25rem;
  }

  :global(.dark) .git-log-hash {
    color: #94a3b8;
    background: #1e293b;
  }

  .git-log-words {
    font-size: 0.6875rem;
    font-weight: 600;
    color: #16a34a;
    background: #dcfce7;
    padding: 0.1rem 0.4rem;
    border-radius: 0.25rem;
  }

  :global(.dark) .git-log-words {
    color: #86efac;
    background: #052e16;
  }

  .git-log-message {
    margin: 0.25rem 0;
    font-size: 0.8125rem;
    color: #334155;
    line-height: 1.4;
  }

  :global(.dark) .git-log-message {
    color: #e2e8f0;
  }

  .git-log-date {
    font-size: 0.6875rem;
    color: #94a3b8;
  }

  :global(.dark) .git-log-date {
    color: #64748b;
  }

  .git-log-files {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin: 0.4rem 0;
  }

  .git-log-file-badge {
    font-size: 0.65rem;
    color: #475569;
    background: #f1f5f9;
    border: 1px solid #e2e8f0;
    border-radius: 0.25rem;
    padding: 0.1rem 0.35rem;
  }

  :global(.dark) .git-log-file-badge {
    color: #cbd5e1;
    background: #1e293b;
    border-color: #334155;
  }

  /* ── Sidebar footer (utility buttons + git, collapsible) ────── */
  .sidebar-footer {
    border-top: 1px solid #e2e8f0;
    background: #f8fafc;
    flex-shrink: 0;
  }

  :global(.dark) .sidebar-footer {
    border-top-color: #334155;
    background: #0f172a;
  }

  .footer-toggle {
    width: 100%;
    padding: 0.15rem 0;
    border: none;
    background: transparent;
    cursor: pointer;
    font-size: 0.6rem;
    color: #94a3b8;
    transition: color 120ms;
  }

  .footer-toggle:hover {
    color: #64748b;
  }

  :global(.dark) .footer-toggle:hover {
    color: #cbd5e1;
  }

  .footer-rows {
    padding: 0.3rem 0.5rem 0.45rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .footer-row {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-wrap: wrap;
  }

  .footer-btn {
    padding: 0.2rem 0.4rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.25rem;
    background: transparent;
    font-size: 0.7rem;
    color: #64748b;
    cursor: pointer;
    transition: background 120ms, color 120ms;
    line-height: 1;
  }

  .footer-btn:hover {
    background: #e2e8f0;
    color: #1e293b;
  }

  :global(.dark) .footer-btn {
    border-color: #334155;
    color: #94a3b8;
  }

  :global(.dark) .footer-btn:hover {
    background: #334155;
    color: #e2e8f0;
  }

  .footer-lang {
    font-weight: 700;
    font-size: 0.6rem;
    letter-spacing: 0.05em;
    min-width: 1.5rem;
    text-align: center;
  }

  .footer-lang.active {
    border-color: #3b82f6;
    background: #eff6ff;
    color: #1e40af;
  }

  :global(.dark) .footer-lang.active {
    border-color: #60a5fa;
    background: #1e3a5f;
    color: #93c5fd;
  }

  .footer-sep {
    width: 1px;
    height: 1rem;
    background: #e2e8f0;
  }

  :global(.dark) .footer-sep {
    background: #334155;
  }
</style>
