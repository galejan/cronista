<script lang="ts">
  import { untrack } from "svelte";
  import type { Component } from "svelte";
  import { fly } from "svelte/transition";
  import Editor from "$lib/components/Editor.svelte";
  import EditorContextMenu from "$lib/components/EditorContextMenu.svelte";
  import GitIdentityDialog from "$lib/components/GitIdentityDialog.svelte";
  import ProjectSettingsDialog from "$lib/components/ProjectSettingsDialog.svelte";
  import GlobalSettingsDialog from "$lib/components/GlobalSettingsDialog.svelte";
  import ProjectConfigForm from "$lib/ProjectConfigForm.svelte";
  import { debounce } from "$lib/debounce";
  import { t, lang } from "$lib/i18n.svelte";
  import {
    actualizarConfigProyecto,
    actualizarFuenteProyecto,
    actualizarLugar,
    actualizarPersonaje,
    actualizarEventoTimeline,
    agregarEventoTimeline,
    cargarCapitulo,
    cargarConfigRemoto,
    cargarIdentidadGit,
    cargarIndice,
    cargarLugar,
    cargarNota,
    cargarPersonaje,
    cargarTimeline,
    configurarRemoto,
    crearCapitulo,
    crearCheckpoint,
    crearLugar,
    crearNota,
    crearPersonaje,
    crearProyecto,
    crearTrama,
    eliminarTrama,
    asignarCapituloTrama,
    detectarConfigGit,
    detectarGit,
    eliminarCapitulo,
    eliminarEventoTimeline,
    eliminarLugar,
    exportarProyectoMd,
    exportarProyectoZip,
    eliminarDirectorioGit,
    eliminarNota,
    eliminarPersonaje,
    guardarCapitulo,
    guardarConfigRemoto,
    guardarIdentidadGit,
    importarProyecto,
    inicializarGit,
    iniciarSesionEscritura,
    listarLugares,
    listarNotas,
    listarPersonajes,
    marcarProyectoCronInsta,
    obtenerGitLog,
    reintentarPush,
    pushAhora,
    verificarRemoto,
    traerCambios,
    reordenarTimeline,
    setActiveProject,
    sincronizarRemoto,
    verificarGitInicializado,
  } from "$lib/tauri";
  import type { GitLogEntry } from "$lib/tauri";
  import type { Trama, ChapterTrama } from "$lib/tauri";
  import { documentDir } from "@tauri-apps/api/path";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { PhysicalSize, PhysicalPosition } from "@tauri-apps/api/dpi";
  import { open, message, ask } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import pkg from "../../package.json";
  const APP_VERSION = pkg.version;

  // ── Phosphor Icons (direct-path imports for tree-shaking) ─────
  import ArrowRight from "phosphor-svelte/lib/ArrowRight";
  import BookOpen from "phosphor-svelte/lib/BookOpen";
  import CaretDown from "phosphor-svelte/lib/CaretDown";
  import CaretLeft from "phosphor-svelte/lib/CaretLeft";
  import CaretRight from "phosphor-svelte/lib/CaretRight";
  import CaretUp from "phosphor-svelte/lib/CaretUp";
  import ChatText from "phosphor-svelte/lib/ChatText";
  import CheckCircle from "phosphor-svelte/lib/CheckCircle";
  import Clock from "phosphor-svelte/lib/Clock";
  import DownloadSimple from "phosphor-svelte/lib/DownloadSimple";
  import Export from "phosphor-svelte/lib/Export";
  import FileText from "phosphor-svelte/lib/FileText";

  import FloppyDisk from "phosphor-svelte/lib/FloppyDisk";
  import FolderOpen from "phosphor-svelte/lib/FolderOpen";
  import Gear from "phosphor-svelte/lib/Gear";
  import GitBranch from "phosphor-svelte/lib/GitBranch";
  import Keyboard from "phosphor-svelte/lib/Keyboard";
  import MapTrifold from "phosphor-svelte/lib/MapTrifold";
  import Notebook from "phosphor-svelte/lib/Notebook";
  import NotePencil from "phosphor-svelte/lib/NotePencil";
  import Notepad from "phosphor-svelte/lib/Notepad";
  import Package from "phosphor-svelte/lib/Package";
  import PaperPlaneTilt from "phosphor-svelte/lib/PaperPlaneTilt";
  import PencilSimple from "phosphor-svelte/lib/PencilSimple";
  import PushPin from "phosphor-svelte/lib/PushPin";
  import Question from "phosphor-svelte/lib/Question";
  import Scroll from "phosphor-svelte/lib/Scroll";
  import Sparkle from "phosphor-svelte/lib/Sparkle";
  import User from "phosphor-svelte/lib/User";
  import UserPlus from "phosphor-svelte/lib/UserPlus";
  import Warning from "phosphor-svelte/lib/Warning";

  import X from "phosphor-svelte/lib/X";
  import XCircle from "phosphor-svelte/lib/XCircle";

  let sidebarPct = $state(40);          // current sidebar width in %
  let sidebarSaved = $state(40);        // width to restore on un-collapse
  let sidebarCollapsed = $state(false); // derived for CSS class
  let globalSettingsOpen = $state(false);
  let helpMode = $state(false);

  // ── Set window title (hide dev URL) ──────────────────────────
  $effect(() => {
    try {
      getCurrentWindow().setTitle("Cron-Insta");
    } catch { /* not in Tauri */ }
  });

  // ── Restore zoom level ────────────────────────────────────────
  $effect(() => {
    const stored = localStorage.getItem("cron-insta-zoom");
    if (stored) zoomLevel = Math.min(2, Math.max(0, Number(stored)));
  });

  // ── Apply zoom to the whole UI ─────────────────────────────────
  $effect(() => {
    const scales = [1, 1.15, 1.3];
    document.body.style.zoom = String(scales[zoomLevel] ?? 1);
  });
  // ── First-run detection: auto-open global settings if never launched ──
  $effect(() => {
    if (typeof localStorage !== "undefined" && !localStorage.getItem("cron-insta-has-launched")) {
      globalSettingsOpen = true;
    }
  });

  // ── Set first-run flag when dialog is dismissed ──────────────────
  $effect(() => {
    // Only set the flag after the dialog has been opened and then closed
    // (avoid setting on initial mount where globalSettingsOpen is false)
    if (!globalSettingsOpen && typeof localStorage !== "undefined") {
      if (localStorage.getItem("cron-insta-has-launched") !== "true") {
        // Dialog was opened (auto or manual) and now closed — first run complete
        localStorage.setItem("cron-insta-has-launched", "true");
      }
    }
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
      localStorage.setItem("cron-insta-window-maximized", String(isMax));
      localStorage.setItem(
        "cron-insta-window-size",
        JSON.stringify({ width: size.width, height: size.height }),
      );
      localStorage.setItem(
        "cron-insta-window-position",
        JSON.stringify({ x: pos.x, y: pos.y }),
      );
    } catch {
      // Not running inside Tauri (e.g. svelte-check).
    }
  }

  async function restoreWindowState(): Promise<void> {
    try {
      const win = getCurrentWindow();
      const storedMax = localStorage.getItem("cron-insta-window-maximized");
      if (storedMax === "true") {
        await win.maximize();
        return;
      }
      const storedSize = localStorage.getItem("cron-insta-window-size");
      const storedPos = localStorage.getItem("cron-insta-window-position");
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
    const savedPct = localStorage.getItem("cron-insta-sidebar-pct");
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
      localStorage.setItem("cron-insta-sidebar-pct", String(sidebarPct));
    }
  });

  // ── Editor & project state ──────────────────────────────────
  let projectPath = $state("");
  let projectDisplayName = $derived(
    projectPath.split(/[\\/]/).filter(Boolean).pop() || projectPath
  );
  let gitEnabled = $state(false);
  let gitStatus = $state<"active" | "unavailable" | "not-initialized" | "unknown">("unknown");
  let gitInitModal = $state(false);
  let gitInitNombre = $state("");
  let gitInitEmail = $state("");
  let gitHelpModal = $state(false);
  let gitLogVisible = $state(false);
  let gitLogEntries = $state<GitLogEntry[]>([]);

  // ── Git Identity & Remote dialog ─────────────────────────────
  let identityDialogOpen = $state(false);
  let identityDialogPath = $state("");
  let identityDialogProjectName = $state("");
  let identityDialogResolve = $state<((ctx: {remoteConfigured: boolean, remoteUrl: string}) => void) | null>(null);
  let remoteWarningVisible = $state(false);
  let remoteWarningDialog = $state(false);
  let pushReady = $state(false);
  let hasRemote = $state(false);
  let remoteBehind = $state(0);
  let pullDialogOpen = $state(false);

  // ── Project Settings dialog ──────────────────────────────────
  let settingsOpen = $state(false);

  // ── Toast notifications ─────────────────────────────────────
  let toast = $state<{message: string, type: "warning" | "error", action?: {label: string, onClick: () => void}, icon?: Component} | null>(null);

  let footerExpanded = $state(true);
  let zoomLevel = $state(0); // 0=normal, 1=medium, 2=large
  let exportModal = $state(false);
  let chapters = $state<string[]>([]);
  let tramas = $state<Trama[]>([]);
  let chapterTramas = $state<ChapterTrama[]>([]);
  let tramasCollapsed = $state<Set<string>>(new Set());
  let tramaPendingDelete = $state<string | null>(null);

  // Derived: group chapters by trama_id, ordered by chapters_order
  let chaptersByTrama = $derived.by(() => {
    const groups = new Map<string | null, string[]>();
    for (const t of tramas) {
      groups.set(t.id, []);
    }
    groups.set(null, []); // unassigned
    const lookup = new Map<string, string | null>();
    for (const ct of chapterTramas) {
      lookup.set(ct.filename, ct.trama_id ?? null);
    }
    for (const ch of chapters) {
      const tid = lookup.get(ch) ?? null;
      if (!groups.has(tid)) {
        groups.set(tid, []);
      }
      groups.get(tid)!.push(ch);
    }
    return groups;
  });

  // Derived: ordered sections for rendering
  let tramaSections = $derived.by(() => {
    const sections: Array<{ tramaId: string | null; nombre: string | null; chapters: string[] }> = [];
    for (const t of tramas) {
      const chs = chaptersByTrama.get(t.id) ?? [];
      sections.push({ tramaId: t.id, nombre: t.nombre, chapters: chs });
    }
    const unassigned = chaptersByTrama.get(null) ?? [];
    sections.push({ tramaId: null, nombre: null, chapters: unassigned });
    return sections;
  });
  let pendingDelete = $state<string | null>(null);
  let activeChapter = $state("");
  let editorContent = $state("");
  let fontFamily = $state("monospace");
  let visibleTabs = $state<Record<string, boolean>>({});
  let autoSaveInterval = $state(5);
  let configFormOpen = $state(false);
  let configFormResolve = $state<((v: {font_family: string; visible_tabs: Record<string,boolean>; auto_save_interval_minutes: number}) => void) | null>(null);
  let fontPickerOpen = $state(false);
  let fontPickerFont = $state("monospace");
  let fontPickerResolve = $state<((v: string) => void) | null>(null);

  // ── Generic text input picker (replaces native prompt()) ─────
  let textPickerOpen = $state(false);
  let textPickerMessage = $state("");
  let textPickerValue = $state("");
  let textPickerDefault = $state("");
  let textPickerResolve = $state<((v: string | null) => void) | null>(null);

  // ── Trama selector dialog ──────────────────────────────────
  let tramaSelectorOpen = $state(false);
  let tramaSelectorFilename = $state("");
  let tramaSelectorTitle = $state("");
  let tramaSelectorInitialHTML = $state("");
  let tramaSelectorResolve = $state<((v: string | null | undefined) => void) | null>(null);

  /** Show a text input modal and return the value (or null if cancelled). */
  function pickText(message: string, defaultValue?: string): Promise<string | null> {
    return new Promise((resolve) => {
      textPickerMessage = message;
      textPickerValue = defaultValue || "";
      textPickerDefault = defaultValue || "";
      textPickerOpen = true;
      textPickerResolve = resolve;
    });
  }

  /** Show the 3-option trama selector dialog. Returns trama_id, null (skip), or undefined (cancel). */
  function pickTramaForChapter(): Promise<string | null | undefined> {
    return new Promise((resolve) => {
      tramaSelectorResolve = resolve;
      tramaSelectorOpen = true;
    });
  }
  function resolveTramaSelector(value: string | null | undefined) {
    const r = tramaSelectorResolve;
    tramaSelectorOpen = false;
    tramaSelectorResolve = null;
    r?.(value);
  }

  async function handleTramaSelectExisting(): Promise<void> {
    if (tramas.length === 0) {
      await message("No hay tramas disponibles.");
      resolveTramaSelector(undefined);
      return;
    }
    tramaSelectorOpen = false; // hide dialog temporarily
    const name = await pickText(t("tramas.selectExisting"), tramas[0]?.nombre || "");
    if (!name) { resolveTramaSelector(undefined); return; }
    const found = tramas.find(t => t.nombre === name.trim());
    resolveTramaSelector(found ? found.id : null);
  }

  async function handleTramaSelectNew(): Promise<void> {
    tramaSelectorOpen = false; // hide dialog temporarily
    const name = await pickText(t("tramas.selectNewPrompt"));
    if (!name?.trim()) { resolveTramaSelector(undefined); return; }
    try {
      const trama = await crearTrama(projectPath, name.trim());
      await refreshChapters();
      resolveTramaSelector(trama.id);
    } catch (e) {
      console.error("[cron-insta] Create trama failed:", e);
      await message(`${t("tramas.createError")} ${e}`);
      resolveTramaSelector(undefined);
    }
  }

  function handleTramaSelectSkip(): void {
    resolveTramaSelector(null);
  }

  async function handleNuevaTrama(): Promise<void> {
    if (!projectPath) return;
    const nombre = await pickText(t("tramas.newPrompt"));
    if (!nombre?.trim()) return;
    try {
      await crearTrama(projectPath, nombre.trim());
      await refreshChapters();
    } catch (e) {
      console.error("[cron-insta] Create trama failed:", e);
      await message(`${t("tramas.createError")} ${e}`);
    }
  }

  async function handleEliminarTrama(id: string): Promise<void> {
    if (tramaPendingDelete === id) {
      // Second click — execute deletion
      try {
        await eliminarTrama(projectPath, id);
        await refreshChapters();
      } catch (e) {
        console.error("[cron-insta] Delete trama failed:", e);
        await message(`${t("tramas.deleteError")} ${e}`);
      }
      tramaPendingDelete = null;
    } else {
      tramaPendingDelete = id;
      setTimeout(() => { tramaPendingDelete = null; }, 3_000);
    }
  }

  async function handleDropOnTrama(e: DragEvent, tramaId: string | null): Promise<void> {
    e.preventDefault();
    if (!dragChapter) return;
    const chapterFilename = dragChapter;
    dragChapter = null;

    // Clean up drag-over classes
    document.querySelectorAll(".trama-group.drag-over").forEach(el => {
      el.classList.remove("drag-over");
    });

    const tramaName = tramaId ? tramas.find(t => t.id === tramaId)?.nombre || "" : "";
    const confirmMsg = tramaId
      ? t("tramas.moveConfirm").replace("{chapter}", chapterFilename).replace("{trama}", tramaName)
      : t("tramas.moveToUnassigned").replace("{chapter}", chapterFilename);

    const confirmed = await ask(confirmMsg);
    if (!confirmed) return;

    try {
      await asignarCapituloTrama(projectPath, chapterFilename, tramaId);
      await refreshChapters();
    } catch (e) {
      console.error("[cron-insta] Assign chapter to trama failed:", e);
      await message(`${t("tramas.assignError")} ${e}`);
    }
  }

  function toggleTramaCollapse(id: string): void {
    if (tramasCollapsed.has(id)) {
      tramasCollapsed.delete(id);
    } else {
      tramasCollapsed.add(id);
    }
    // Trigger reactivity
    tramasCollapsed = new Set(tramasCollapsed);
  }
  let saveStatus = $state<"" | "saved" | "unsaved" | "saving">("");

  // ── Context menu state ──────────────────────────────────────
  let contextMenu = $state({ open: false, x: 0, y: 0, selectedText: "" });

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
      console.log("[cron-insta:close] Registering onCloseRequested handler");

      w.onCloseRequested(async (event) => {
        const path = untrack(() => projectPath);
        const gitOk = untrack(() => gitEnabled);

        console.log("[cron-insta:close] ── Close requested ──");
        console.log("[cron-insta:close]   projectPath:", path || "(none)");
        console.log("[cron-insta:close]   gitEnabled:", gitOk);

        if (!path || !gitOk) {
          console.log("[cron-insta:close] → no project or no git, closing immediately");
          event.preventDefault();
          try {
            getCurrentWindow().destroy();
          } catch (e) {
            console.error("[cron-insta:close]   destroy FAILED:", e);
          }
          return;
        }

        if (untrack(() => closing)) {
          console.log("[cron-insta:close] → already closing, letting through");
          return;
        }

        closing = true;
        closeStep = "Cerrando aplicación...";
        console.log("[cron-insta:close] → showing overlay, Rust handles checkpoint");

        event.preventDefault(); // Keep window alive while overlay shows

        // Brief pause so user sees the overlay
        await new Promise(r => setTimeout(r, 500));

        // Force-close. Rust's on_window_event already did the checkpoint
        // (or is about to) on its own thread.
        console.log("[cron-insta:close] → destroying window");
        try {
          getCurrentWindow().destroy();
        } catch (e) {
          console.error("[cron-insta:close]   destroy FAILED:", e);
        }
      }).then((fn) => {
        unlisten = fn;
        console.log("[cron-insta:close] Handler registered successfully");
      }).catch((err) => {
        console.error("[cron-insta:close] Failed to register handler:", err);
      });
    } catch (err) {
      console.error("[cron-insta:close] Not in Tauri:", err);
    }

    return () => {
      console.log("[cron-insta:close] Effect cleanup — unregistering handler");
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
    scrollToTop(): void;
    getSelectedText(): string;
    deleteSelection(): string;
  }>();

  // ── Sidebar tab state ───────────────────────────────────────
  let activeTab = $state<"capitulos" | "personajes" | "notas" | "timeline" | "lugares">("capitulos");

  // ── Characters state ────────────────────────────────────────
  let personajes = $state<{ id: string; name: string }[]>([]);
  let personajeFormVisible = $state(false);
  let personajeNuevoNombre = $state("");
  let personajeExpandido = $state<string | null>(null);
  let personajeEditando = $state<Record<string, any> | null>(null);
  let characterDocked = $state<{
    id: string; name: string; physicalDescription: string;
    personality: string; traumas: string;
    relationships: Array<{targetName: string; type: string; notes: string}>;
  } | null>(null);

  // ── Notes state ─────────────────────────────────────────────
  let notas = $state<{ id: string; title: string }[]>([]);
  let activeNote = $state("");
  let notaTitulo = $state("");

  // ── Timeline state ──────────────────────────────────────────
  let timeline = $state<Record<string, any>[]>([]);
  let eventoFormVisible = $state(false);
  let eventoExpandido = $state<string | null>(null);
  let nuevoEventoFecha = $state("");
  let nuevoEventoTitulo = $state("");
  let nuevoEventoDescripcion = $state("");
  let nuevoEventoPersonajes = $state<string[]>([]);
  let nuevoEventoCapitulos = $state<string[]>([]);
  let nuevoEventoLugares = $state<string[]>([]);
  let eventoEditando = $state<string | null>(null); // event ID being edited, or null for new

  // ── Places state ────────────────────────────────────────────
  let lugares = $state<{ id: string; name: string }[]>([]);
  let lugarFormVisible = $state(false);
  let lugarNuevoNombre = $state("");
  let lugarNuevaDescripcion = $state("");
  let lugarExpandido = $state<string | null>(null);
  let lugarEditando = $state<Record<string, any> | null>(null);

  // ── Actual save logic (shared by debounced auto-save and manual button) ──
  async function doSave(): Promise<void> {
    if (!projectPath) return;
    saveStatus = "saving";

    if (activeNote) {
      console.log("[cron-insta] Saving note:", activeNote);
      try {
        await crearNota(projectPath, activeNote, notaTitulo, editorContent);
        saveStatus = "saved";
        console.log("[cron-insta] Save Note OK:", activeNote);
      } catch (e) {
        console.error("[cron-insta] Save note failed:", e);
        saveStatus = "unsaved";
      }
    } else if (activeChapter) {
      console.log("[cron-insta] Saving chapter:", activeChapter);
      try {
        await guardarCapitulo(projectPath, activeChapter, editorContent);
        saveStatus = "saved";
        console.log("[cron-insta] Save OK:", activeChapter);

        // Trigger checkpoint (best-effort, non-blocking)
        try {
          const ckResult = await crearCheckpoint(projectPath);
          if (ckResult.includes("⚠️")) {
            const warnPart = ckResult.split("⚠️")[1]?.trim() || "";
            showToast(warnPart || t("git.pushFailed"), "warning");
            actualizarGitStatus(projectPath);
          }
        } catch {
          // Silently ignore checkpoint errors during save
        }
      } catch (e) {
        console.error("[cron-insta] Save failed:", e);
        saveStatus = "unsaved";
      }
    }
  }

  // ── Dynamic auto-save: recreates debounce when interval changes ──
  let saveTrigger = $state<{ trigger: () => void; cancel: () => void }>({
    trigger: () => { doSave(); },
    cancel: () => {},
  });

  $effect(() => {
    const interval = autoSaveInterval * 60_000;
    const debounced = debounce(doSave, interval);
    saveTrigger = debounced;
    return () => { debounced.cancel(); };
  });

  // ── Editor callbacks ────────────────────────────────────────
  let cercaDelFinal = $state(false);

  function handleEditorUpdate(html: string): void {
    editorContent = html;
    saveStatus = "unsaved";
    saveTrigger.trigger();
  }

  // ── Chapter operations ──────────────────────────────────────
  async function cargarCapituloActual(filename: string): Promise<void> {
    if (!projectPath) return;
    saveTrigger.cancel();
    console.log("[cron-insta] Loading chapter:", filename);
    try {
      const content = await cargarCapitulo(projectPath, filename);
      editorRef?.setContent(content);
      editorRef?.scrollToTop();
      activeChapter = filename;
      editorContent = content;
      saveStatus = "saved";

      // Start writing session tracking
      try {
        await iniciarSesionEscritura(projectPath, filename);
        console.log("[cron-insta] Writing session started for:", filename);
      } catch (e) {
        console.error("[cron-insta] Failed to start writing session:", e);
      }

      console.log("[cron-insta] Load OK:", filename, `(${content.length} chars)`);
    } catch (e) {
      console.error("[cron-insta] Failed to load chapter:", e);
    }
  }

  /** Refresh the chapter list from metadata.json on disk. */
  async function refreshChapters(): Promise<void> {
    if (!projectPath) return;
    try {
      const raw = await cargarIndice(projectPath);
      const meta = JSON.parse(raw);
      chapters = meta.chapters_order ?? [];
      tramas = meta.tramas ?? [];
      chapterTramas = meta.chapter_tramas ?? [];
      console.log("[cron-insta] Chapters refreshed:", chapters);
    } catch (e) {
      console.error("[cron-insta] Failed to read project index:", e);
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
        console.error("[cron-insta] Delete chapter failed:", e);
        await message(`${t("chapters.deleteError")} ${e}`);
      }
      pendingDelete = null;
    } else {
      // First click — mark for confirmation
      pendingDelete = filename;
      // Auto-reset after 3 seconds
      setTimeout(() => { pendingDelete = null; }, 3_000);
    }
  }

  // ── Context menu action handlers ──────────────────────────

  async function handleSaveAsNote(): Promise<void> {
    if (!projectPath) return;
    const text = contextMenu.selectedText;
    if (!text.trim()) return;

    const title = await pickText(t("context.noteTitlePrompt"), text.trim().slice(0, 60));
    if (!title?.trim()) return;

    const id = "note-" + Date.now();
    try {
      await crearNota(projectPath, id, title.trim(), text);
      await refreshNotas();
    } catch (e) {
      console.error("[cron-insta] Save note from context failed:", e);
      await message(`${t("notes.createError")} ${e}`);
    }
  }

  async function handleSaveAsTrait(): Promise<void> {
    if (!projectPath) return;
    const text = contextMenu.selectedText;
    if (!text.trim()) return;

    // Prompt for character name
    const name = await pickText(t("context.characterPrompt"));
    if (!name?.trim()) return;

    // Load characters list and find match
    try {
      const raw = await listarPersonajes(projectPath);
      const chars: { id: string; name: string }[] = JSON.parse(raw);
      const found = chars.find((c) => c.name === name.trim());
      if (!found) {
        await message(t("context.characterNotFound").replace("{name}", name.trim()));
        return;
      }

      // Prompt for where to append
      const field = await pickText("¿Dónde? (personalidad / traumas)", "personalidad");
      if (!field) return;

      // Load character
      const charRaw = await cargarPersonaje(projectPath, found.id);
      const char = JSON.parse(charRaw);

      // Append to field
      if (field.toLowerCase().includes("personalidad")) {
        char.personality = (char.personality || "") + "\n" + text;
      } else if (field.toLowerCase().includes("trauma")) {
        char.traumas = (char.traumas || "") + "\n" + text;
      } else {
        // Default: personality
        char.personality = (char.personality || "") + "\n" + text;
      }

      await actualizarPersonaje(projectPath, found.id, JSON.stringify(char));
      await refreshPersonajes();
      showToast(t("context.traitSaved").replace("{name}", found.name), "warning", undefined, CheckCircle);
    } catch (e) {
      console.error("[cron-insta] Save trait from context failed:", e);
      await message(`${t("characters.saveError")} ${e}`);
    }
  }

  async function handleNewChapterFromContext(): Promise<void> {
    if (!projectPath) return;
    const text = contextMenu.selectedText;
    if (!text.trim()) return;

    const filename = await pickText(t("context.chapterNamePrompt"));
    if (!filename?.trim()) return;

    const sanitized = filename
      .trim()
      .replace(/\s+/g, "-")
      .replace(/\.md$/i, "")
      + ".md";

    // Create a simple HTML document with the selected text
    const titulo = filename.trim().replace(/\.md$/i, "")
      .replace(/^[\d_]+/, "")
      .replace(/_/g, " ")
      .trim() || t("chapters.untitled");
    const html = `<h1>${titulo}</h1><p>${text.replace(/\n/g, "</p><p>")}</p>`;

    try {
      await crearCapitulo(projectPath, sanitized, html, null);
      await refreshChapters();
      // Optionally switch to the new chapter
      await cargarCapituloActual(sanitized);
    } catch (e) {
      console.error("[cron-insta] New chapter from context failed:", e);
      await message(`${t("chapters.createError")} ${e}`);
    }
  }

  async function handleAddAsEventFromContext(): Promise<void> {
    if (!projectPath) return;
    const text = contextMenu.selectedText;
    if (!text.trim()) return;

    const title = await pickText(t("context.eventTitlePrompt"), text.trim().slice(0, 60));
    if (!title?.trim()) return;

    const evento = {
      date: "",
      title: title.trim(),
      description: text,
      relatedCharacters: [] as string[],
      relatedChapters: [] as string[],
      relatedPlaces: [] as string[],
    };

    try {
      await agregarEventoTimeline(projectPath, JSON.stringify(evento));
      await refreshTimeline();
    } catch (e) {
      console.error("[cron-insta] Add event from context failed:", e);
      await message(`${t("timeline.addError")} ${e}`);
    }
  }

  async function handleAddToPlace(): Promise<void> {
    if (!projectPath) return;
    const text = contextMenu.selectedText;
    if (!text.trim()) return;

    const name = await pickText(t("context.placePrompt"));
    if (!name?.trim()) return;

    try {
      // Search for existing place by name
      const raw = await listarLugares(projectPath);
      const places: { id: string; name: string }[] = JSON.parse(raw);
      let found = places.find((p) => p.name === name.trim());

      if (!found) {
        // Create a new place if not found
        const id = name
          .trim()
          .toLowerCase()
          .normalize("NFD")
          .replace(/[\u0300-\u036f]/g, "")
          .replace(/[^a-z0-9]+/g, "-")
          .replace(/^-+|-+$/g, "");
        const lugar = {
          id,
          name: name.trim(),
          description: text,
        };
        await crearLugar(projectPath, JSON.stringify(lugar));
        await refreshLugares();
        return;
      }

      // Append text to existing place description
      const placeRaw = await cargarLugar(projectPath, found.id);
      const place = JSON.parse(placeRaw);
      place.description = place.description
        ? place.description + "\n" + text
        : text;
      await actualizarLugar(projectPath, found.id, JSON.stringify(place));
      await refreshLugares();
    } catch (e) {
      console.error("[cron-insta] Add to place failed:", e);
      await message(`${t("places.saveError")} ${e}`);
    }
  }

  // ── Chapter navigation ──────────────────────────────────────
  function capituloAnterior(): void {
    const idx = chapters.indexOf(activeChapter);
    if (idx > 0) cargarCapituloActual(chapters[idx - 1]);
  }

  function capituloSiguiente(): void {
    const idx = chapters.indexOf(activeChapter);
    if (idx < chapters.length - 1) {
      cargarCapituloActual(chapters[idx + 1]);
    } else {
      crearCapituloNuevo();
    }
  }

  /** Show GitIdentityDialog and return the result as a promise. */
  function showIdentityDialog(path: string, projectName?: string): Promise<{remoteConfigured: boolean, remoteUrl: string}> {
    return new Promise((resolve) => {
      identityDialogPath = path;
      identityDialogProjectName = projectName || "";
      identityDialogOpen = true;
      identityDialogResolve = resolve;
    });
  }

  /** Parse the repo name from a git remote URL. */
  function extractRepoName(url: string): string | null {
    // SSH: git@github.com:user/repo.git → repo
    // SSH: git@github.com:user/repo → repo
    // HTTPS: https://github.com/user/repo.git → repo
    const match = url.match(/[:/]([^/]+?)(?:\.git)?\s*$/);
    return match ? match[1] : null;
  }

  /** Dismiss the toast after a delay. */
  function showToast(message: string, type: "warning" | "error" = "warning", action?: {label: string, onClick: () => void}, icon?: Component) {
    toast = { message, type, action, icon };
    setTimeout(() => {
      if (toast?.message === message) toast = null;
    }, 5_000);
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
        const continuar = await ask(
          t("git.notInstalled") + "\n\n" +
          t("git.notInstalledDesc") + "\n\n" +
          t("git.installInstructions") + "\n\n" +
          t("git.continueWithout")
        );
        if (!continuar) return;
      }

      const name = await pickText(t("dialog.projectName"), t("dialog.projectNameDefault"));
      if (!name) return;

      // Font + tabs + autosave via ProjectConfigForm
      const config = await showProjectConfig();
      fontFamily = config.font_family;
      visibleTabs = config.visible_tabs;
      autoSaveInterval = config.auto_save_interval_minutes;

      const path = `${selected}/${name.trim()}`;

      // If Git is available, show identity dialog BEFORE project creation
      // so identity is saved before git init reads it from global config.
      let remoteConfigured = false;
      let remoteUrl = "";
      if (gitDisponible) {
        try {
          const result = await showIdentityDialog(path, name.trim());
          remoteConfigured = result.remoteConfigured;
          remoteUrl = result.remoteUrl;
        } catch {
          // User closed dialog — proceed without identity save
        }
      }

      console.log("[cron-insta] Creating project:", { path, name });
      try {
        const visibleTabsForRust = {
          chapters: config.visible_tabs.chapters !== false,
          characters: config.visible_tabs.characters !== false,
          places: config.visible_tabs.places !== false,
          timeline: config.visible_tabs.timeline !== false,
          notes: config.visible_tabs.notes !== false,
        };
        const msg = await crearProyecto(path, name.trim(), fontFamily, visibleTabsForRust, autoSaveInterval);
        console.log("[cron-insta] Project created:", msg);
        projectPath = path;
        setActiveProject(path);
        marcarProyectoCronInsta(path); // fire-and-forget: set folder icon

        // If remote was configured, set it up
        if (remoteConfigured && remoteUrl) {
          try {
            await configurarRemoto(path, remoteUrl);
            await guardarConfigRemoto(path, remoteUrl, true);
            console.log("[cron-insta] Remote configured:", remoteUrl);
          } catch (e) {
            console.error("[cron-insta] Remote config failed:", e);
            const msg = String(e);
            if (msg.startsWith("REPO_NOT_FOUND:")) {
              const repoName = extractRepoName(remoteUrl);
              showToast(
                t("git.repoNotFound"),
                "warning",
                repoName
                  ? { label: t("git.createOnGithub"), onClick: () => openUrl(`https://github.com/new?name=${repoName}`) }
                  : undefined,
              );
            } else if (msg.startsWith("REMOTE_HAS_COMMITS:")) {
              const desc = msg.replace("REMOTE_HAS_COMMITS:", "");
              const shouldSync = await ask(desc);
              if (shouldSync) {
                try {
                  const syncMsg = await sincronizarRemoto(path);
                  showToast(syncMsg || t("git.syncSuccess"), "warning", undefined, CheckCircle);
                } catch (syncErr) {
                  showToast(String(syncErr), "error");
                }
              }
            } else {
              showToast(msg, "error");
            }
          }
        }

        await actualizarGitStatus(path);
        await refreshChapters();
      } catch (e) {
        console.error("[cron-insta] Failed to create project:", e);
        await message(`${t("dialog.createProjectError")} ${e}`);
        return;
      }
    }

    const filename = await pickText(t("chapters.newFilePrompt"), "0001_prologue.md");
    if (!filename) return;

    // Use the chapter filename (without .md) as the initial title
    const titulo = filename.replace(/\.md$/, "")
      .replace(/^[\d_]+/, "")
      .replace(/_/g, " ")
      .trim() || t("chapters.untitled");
    const initialHTML = `<h1>${titulo}</h1><p></p>`;

    // Show trama selector dialog
    const tramaId = await pickTramaForChapter();
    if (tramaId === undefined) return; // user cancelled

    console.log("[cron-insta] Creating chapter:", filename, "tramaId:", tramaId);
    try {
      const msg = await crearCapitulo(projectPath, filename, initialHTML, tramaId);
      console.log("[cron-insta] Chapter created:", msg);
      activeChapter = filename;
      editorRef?.setContent(initialHTML);
      editorContent = initialHTML;
      saveStatus = "saved";
      await refreshChapters();
    } catch (e) {
      console.error("[cron-insta] Create chapter failed:", e);
      await message(`${t("chapters.createError")} ${e}`);
    }
  }

  /**
   * Extract repo slug from remote URL.
   * "git@github.com:user/repo.git" → "user/repo"
   * "https://github.com/user/repo.git" → "user/repo"
   */
  function extraerRepoDeUrl(url: string): string {
    // SSH shortcut: git@host:user/repo.git
    if (url.includes("@") && url.includes(":")) {
      const slug = url.split(":").pop() ?? url;
      return slug.replace(/\.git$/, "");
    }
    // HTTPS / ssh:// — take last two path segments
    const clean = url.replace(/\.git$/, "");
    const parts = clean.replace(/\/+$/, "").split("/");
    if (parts.length >= 2) return parts.slice(-2).join("/");
    return clean;
  }

  /**
   * Detect git identity and remote from .git/config, merge into app config.
   *
   * Fire-and-forget: call after cargarIndice succeeds on project open.
   * Compares detected identity with stored global config; saves if different.
   * Auto-enables push when SSH origin is found (unless 3-strike rule blocks it).
   */
  async function detectarYGuardarConfigGit(path: string): Promise<void> {
    try {
      const detected = await detectarConfigGit(path);
      const hasRemote = detected.remote_url && detected.remote_url.length > 0;
      const hasIdentity = detected.name && detected.email;

      // Nothing to do
      if (!hasRemote && !hasIdentity) return;

      let identityDiferente = false;

      // ── Identity: compare with stored, save if different ───
      if (hasIdentity) {
        const stored = await cargarIdentidadGit();
        const nameDiff = !stored || stored.name !== detected.name;
        const emailDiff = !stored || stored.email !== detected.email;

        if (nameDiff || emailDiff) {
          identityDiferente = true;
          await guardarIdentidadGit(detected.name!, detected.email!);
        }
      }

      // ── Remote: SSH detection with 3-strike guard ──────────
      if (hasRemote) {
        const url = detected.remote_url!;
        const isSSH = url.startsWith("git@") || url.startsWith("ssh://");

        if (isSSH) {
          const remoteState = await cargarConfigRemoto(path);
          if (remoteState && remoteState.consecutive_failures === 0) {
            // Only update if push_enabled is currently false
            if (!remoteState.push_enabled) {
              await guardarConfigRemoto(path, url, true);
            }
          }
        }
      }

      // ── Toast ──────────────────────────────────────────────
      if (hasRemote) {
        const repo = extraerRepoDeUrl(detected.remote_url!);
        if (identityDiferente) {
          await message(
            t("git.detected")
              .replace("{repo}", repo)
              .replace("{name}", detected.name!)
              .replace("{email}", detected.email!),
          );
        } else {
          await message(t("git.detectedOrigin").replace("{repo}", repo));
        }
      }
    } catch (e) {
      // Fire-and-forget — silently ignore detection errors
      console.warn("[cron-insta] Git config detection skipped:", e);
    }
  }

  /**
   * Check if the remote has new commits and offer to pull them.
   *
   * Called after auto-detect on project open, and manually via the
   * "Traer cambios" button. If the user declines, we warn about
   * potential divergence.
   */
  async function verificarYOfrecerPull(path: string): Promise<void> {
    try {
      const status = await verificarRemoto(path);
      if (status.has_updates && status.behind_count > 0) {
        remoteBehind = status.behind_count;
        pullDialogOpen = true;
      }
    } catch {
      // Network or git error — silently skip
    }
  }

  async function ejecutarPull(): Promise<void> {
    if (!projectPath) return;
    pullDialogOpen = false;
    try {
      const msg = await traerCambios(projectPath);
      showToast(msg, "warning");
      // Reload the project to reflect pulled changes
      await abrirProyectoDesdePath(projectPath);
    } catch (e) {
      const errStr = String(e);
      if (errStr.includes("sin guardar") || errStr.includes("uncommitted")) {
        showToast(t("git.pullUncommitted"), "error");
      } else {
        showToast(t("git.pushFailed") + " " + errStr, "error");
      }
    }
  }

  function rechazarPull(): void {
    pullDialogOpen = false;
    remoteBehind = 0;
    showToast(t("git.pullDenied"), "warning");
  }

  /** Reload the currently open project from disk (used after git pull). */
  async function abrirProyectoDesdePath(path: string): Promise<void> {
    try {
      const raw = await cargarIndice(path);
      const meta = JSON.parse(raw);
      chapters = meta.chapters_order ?? [];
      tramas = meta.tramas ?? [];
      chapterTramas = meta.chapter_tramas ?? [];
      fontFamily = meta.font_family || "monospace";
      visibleTabs = meta.visible_tabs || {};
      autoSaveInterval = meta.auto_save_interval_minutes || 5;
      await actualizarGitStatus(path);

      // Reload current chapter or fallback to first
      const stillExists = activeChapter && chapters.includes(activeChapter);
      if (stillExists) {
        await cargarCapituloActual(activeChapter!);
      } else if (chapters.length > 0) {
        await cargarCapituloActual(chapters[0]);
      } else {
        editorContent = "";
        activeChapter = "";
      }
    } catch (e) {
      console.error("[cron-insta] Failed to reload project:", e);
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
    console.log("[cron-insta] Opening project:", path);
    try {
      // Verify it's a valid project by reading the index
      const raw = await cargarIndice(path);
      const meta = JSON.parse(raw);
      projectPath = path;
      setActiveProject(path);
      fontFamily = meta.font_family || "monospace";
      visibleTabs = meta.visible_tabs || {};
      autoSaveInterval = meta.auto_save_interval_minutes || 5;
      await actualizarGitStatus(path);
      detectarYGuardarConfigGit(path); // fire-and-forget
      // After a short delay (let auto-detect finish), check for remote updates
      setTimeout(() => verificarYOfrecerPull(path), 1500);
      chapters = meta.chapters_order ?? [];
      tramas = meta.tramas ?? [];
      chapterTramas = meta.chapter_tramas ?? [];
      console.log("[cron-insta] Project opened:", meta.project_name, chapters);

      // Warn if git is unavailable
      if (gitStatus === "unavailable") {
        console.warn("[cron-insta] Git not detected — automatic version control disabled");
        await message(t("git.notInstalled") + "\n\n" + t("git.notInstalledDesc"));
      }

      // Auto-load first chapter if there is one
      if (chapters.length > 0) {
        await cargarCapituloActual(chapters[0]);
      }
    } catch (e) {
      console.error("[cron-insta] Failed to open project:", e);
      await message(t("dialog.openProjectError") + `\n\n${e}`);
    }
  }

  /** Close current project and start the new-project setup flow. */
  async function nuevoProyectoHandler(): Promise<void> {
    if (projectPath) await cerrarProyecto();
    localStorage.removeItem("cron-insta-last-project");
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
    tramas = [];
    chapterTramas = [];
    tramasCollapsed = new Set();
    tramaPendingDelete = null;
    activeChapter = "";
    editorContent = "";
    saveStatus = "";
    personajes = [];
    personajeExpandido = null;
    personajeEditando = null;
    notas = [];
    activeNote = "";
    timeline = [];
    lugares = [];
    lugarExpandido = null;
    lugarEditando = null;
    characterDocked = null;
    gitEnabled = false;
    gitStatus = "unknown";

    // Keep last project for auto-reopen on next launch
    // (deliberately NOT removing from localStorage)
  }

  /** Import a Cron-Insta project from a .zip file. */
  async function importarProyectoHandler(): Promise<void> {
    if (projectPath) await cerrarProyecto();

    const docsDir = await documentDir();

    // Step 1: Choose .zip file
    const zipSelected = await open({
      multiple: false,
      title: t("import.selectZip"),
      defaultPath: docsDir,
      filters: [{ name: "ZIP", extensions: ["zip"] }],
    });
    if (!zipSelected) return;
    const zipPath = zipSelected as string;

    // Step 2: Choose destination folder
    const destSelected = await open({
      directory: true,
      multiple: false,
      title: t("import.selectDest"),
      defaultPath: docsDir,
    });
    if (!destSelected) return;
    const destDir = destSelected as string;

    // Step 3: Extract
    try {
      const importedPath = await importarProyecto(zipPath, destDir);
      console.log("[cron-insta] Project imported to:", importedPath);

      // Step 4: Handle Git history
      try {
        const gitExists = await verificarGitInicializado(importedPath);
        if (gitExists) {
          const keepHistory = await ask(t("import.gitQuestion"));
          if (!keepHistory) {
            await eliminarDirectorioGit(importedPath);
            await inicializarGit(importedPath);
          }
        }
      } catch {
        // Git not available or check failed — continue
      }

      // Step 5: Open the imported project
      projectPath = importedPath;
      setActiveProject(importedPath);
      await actualizarGitStatus(importedPath);
      const raw = await cargarIndice(importedPath);
      const meta = JSON.parse(raw);
      chapters = meta.chapters_order ?? [];
      tramas = meta.tramas ?? [];
      chapterTramas = meta.chapter_tramas ?? [];
      fontFamily = meta.font_family || "monospace";
      visibleTabs = meta.visible_tabs || {};
      autoSaveInterval = meta.auto_save_interval_minutes || 5;
      if (chapters.length > 0) {
        await cargarCapituloActual(chapters[0]);
      }
      console.log("[cron-insta] Imported project opened:", meta.project_name, chapters);
      showToast(t("import.success"), "warning", undefined, CheckCircle);
    } catch (e) {
      console.error("[cron-insta] Import failed:", e);
      await message(t("import.error") + "\n\n" + String(e));
    }
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

        // Check remote config for push-failure warning
        try {
          const remote = await cargarConfigRemoto(path);
          const wasDisabled = remoteWarningVisible;
          remoteWarningVisible = !!(remote && !remote.push_enabled && remote.url);
          pushReady = !!(remote && remote.push_enabled && remote.url);
          hasRemote = !!(remote && remote.url);
          // Show toast when push was just disabled
          if (!wasDisabled && remoteWarningVisible) {
            showToast(t("git.pushDisabled"), "warning");
          }
        } catch {
          remoteWarningVisible = false;
          pushReady = false;
          hasRemote = false;
        }
      } else {
        gitStatus = "not-initialized";
        gitEnabled = true; // Binary exists, just needs init
        remoteWarningVisible = false;
        pushReady = false;
        hasRemote = false;
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
      console.error("[cron-insta] Failed to load git log:", e);
    }
  }

  /** Show project config form modal (replaces pickFont). Returns full config. */
  function showProjectConfig(): Promise<{font_family: string; visible_tabs: Record<string,boolean>; auto_save_interval_minutes: number}> {
    return new Promise((resolve) => {
      configFormOpen = true;
      configFormResolve = resolve;
    });
  }

  /** Show the export modal. */
  function abrirExportModal(): void {
    console.log("[cron-insta] Opening export modal");
    exportModal = true;
  }

  // ── Characters CRUD ─────────────────────────────────────────

  async function refreshPersonajes(): Promise<void> {
    if (!projectPath) return;
    try {
      const raw = await listarPersonajes(projectPath);
      personajes = JSON.parse(raw);
    } catch (e) {
      console.error("[cron-insta] Failed to list characters:", e);
      personajes = [];
    }
  }

  async function crearPersonajeHandler(): Promise<void> {
    if (!personajeNuevoNombre.trim()) {
      await message(t("characters.nameRequired"));
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
      console.error("[cron-insta] Create character failed:", e);
      await message(`${t("characters.createError")} ${e}`);
    }
  }

  async function seleccionarPersonaje(id: string): Promise<void> {
    saveTrigger.trigger(); // save current work first
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
      console.error("[cron-insta] Load character failed:", e);
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
      console.error("[cron-insta] Update character failed:", e);
      await message(`${t("characters.saveError")} ${e}`);
    }
  }

  async function eliminarPersonajeHandler(id: string): Promise<void> {
    if (!await ask(t("characters.deleteConfirm"))) return;
    try {
      await eliminarPersonaje(projectPath, id);
      personajeExpandido = null;
      personajeEditando = null;
      await refreshPersonajes();
      await refreshTimeline();
    } catch (e) {
      console.error("[cron-insta] Delete character failed:", e);
      await message(`${t("characters.deleteError")} ${e}`);
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
      console.error("[cron-insta] Failed to list notes:", e);
      notas = [];
    }
  }

  async function crearNotaHandler(): Promise<void> {
    const title = await pickText(t("notes.titlePrompt"));
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
      console.error("[cron-insta] Create note failed:", e);
      await message(`${t("notes.createError")} ${e}`);
    }
  }

  async function cargarNotaHandler(id: string): Promise<void> {
    saveTrigger.trigger(); // save current work first
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
      console.log("[cron-insta] Note loaded:", id);
    } catch (e) {
      console.error("[cron-insta] Load note failed:", e);
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
      console.error("[cron-insta] Save note failed:", e);
      saveStatus = "unsaved";
    }
  }

  async function eliminarNotaHandler(id: string): Promise<void> {
    if (!await ask(t("notes.deleteConfirm"))) return;
    try {
      await eliminarNota(projectPath, id);
      if (activeNote === id) {
        activeNote = "";
        notaTitulo = "";
        editorRef?.setContent("");
      }
      await refreshNotas();
    } catch (e) {
      console.error("[cron-insta] Delete note failed:", e);
      await message(`${t("notes.deleteError")} ${e}`);
    }
  }

  // ── Timeline CRUD ───────────────────────────────────────────

  async function refreshTimeline(): Promise<void> {
    if (!projectPath) return;
    try {
      const raw = await cargarTimeline(projectPath);
      timeline = JSON.parse(raw);
    } catch (e) {
      console.error("[cron-insta] Failed to load timeline:", e);
      timeline = [];
    }
  }

  function editarEvento(evt: Record<string, any>): void {
    eventoEditando = evt.id;
    nuevoEventoFecha = evt.date ?? "";
    nuevoEventoTitulo = evt.title ?? "";
    nuevoEventoDescripcion = evt.description ?? "";
    nuevoEventoPersonajes = [...(evt.relatedCharacters ?? [])];
    nuevoEventoCapitulos = [...(evt.relatedChapters ?? [])];
    nuevoEventoLugares = [...(evt.relatedPlaces ?? [])];
    eventoExpandido = null;
    eventoFormVisible = true;
  }

  function cancelarEdicion(): void {
    eventoEditando = null;
    nuevoEventoFecha = "";
    nuevoEventoTitulo = "";
    nuevoEventoDescripcion = "";
    nuevoEventoPersonajes = [];
    nuevoEventoCapitulos = [];
    nuevoEventoLugares = [];
    eventoFormVisible = false;
  }

  async function guardarEventoHandler(): Promise<void> {
    if (!nuevoEventoTitulo) {
      await message(t("timeline.requiredFields"));
      return;
    }
    const evento: Record<string, any> = {
      date: nuevoEventoFecha.trim(),
      title: nuevoEventoTitulo.trim(),
      description: nuevoEventoDescripcion.trim(),
      relatedCharacters: nuevoEventoPersonajes.filter(Boolean),
      relatedChapters: nuevoEventoCapitulos.filter(Boolean),
      relatedPlaces: nuevoEventoLugares.filter(Boolean),
    };

    try {
      if (eventoEditando) {
        evento.id = eventoEditando;
        await actualizarEventoTimeline(projectPath, JSON.stringify(evento));
      } else {
        await agregarEventoTimeline(projectPath, JSON.stringify(evento));
      }
      cancelarEdicion();
      await refreshTimeline();
    } catch (e) {
      console.error("[cron-insta] Save timeline event failed:", e);
      await message(`${t("timeline.addError")} ${e}`);
    }
  }

  async function eliminarEventoHandler(id: string): Promise<void> {
    if (!await ask(t("timeline.deleteConfirm"))) return;
    try {
      await eliminarEventoTimeline(projectPath, id);
      await refreshTimeline();
    } catch (e) {
      console.error("[cron-insta] Delete timeline event failed:", e);
      await message(`${t("timeline.deleteError")} ${e}`);
    }
  }

  function toggleEventoExpandido(id: string): void {
    eventoExpandido = eventoExpandido === id ? null : id;
  }

  function getPersonajeName(id: string): string {
    return personajes.find(p => p.id === id)?.name ?? id;
  }

  function getLugarName(id: string): string {
    return lugares.find(l => l.id === id)?.name ?? id;
  }

  // ── Places CRUD ─────────────────────────────────────────────

  async function refreshLugares(): Promise<void> {
    if (!projectPath) return;
    try {
      const raw = await listarLugares(projectPath);
      lugares = JSON.parse(raw);
    } catch (e) {
      console.error("[cron-insta] Failed to list places:", e);
      lugares = [];
    }
  }

  async function crearLugarHandler(): Promise<void> {
    if (!lugarNuevoNombre.trim()) {
      await message(t("places.nameRequired"));
      return;
    }
    const id = lugarNuevoNombre
      .trim()
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");

    const lugar = {
      id,
      name: lugarNuevoNombre.trim(),
      description: lugarNuevaDescripcion.trim(),
    };
    try {
      await crearLugar(projectPath, JSON.stringify(lugar));
      lugarNuevoNombre = "";
      lugarNuevaDescripcion = "";
      lugarFormVisible = false;
      await refreshLugares();
    } catch (e) {
      console.error("[cron-insta] Create place failed:", e);
      await message(`${t("places.createError")} ${e}`);
    }
  }

  async function seleccionarLugar(id: string): Promise<void> {
    saveTrigger.trigger();
    if (lugarExpandido === id) {
      lugarExpandido = null;
      lugarEditando = null;
      return;
    }
    try {
      const raw = await cargarLugar(projectPath, id);
      lugarEditando = JSON.parse(raw);
      lugarExpandido = id;
    } catch (e) {
      console.error("[cron-insta] Load place failed:", e);
    }
  }

  async function guardarLugarHandler(): Promise<void> {
    if (!lugarEditando) return;
    try {
      await actualizarLugar(
        projectPath,
        lugarEditando.id,
        JSON.stringify(lugarEditando),
      );
      lugarExpandido = null;
      lugarEditando = null;
      await refreshLugares();
    } catch (e) {
      console.error("[cron-insta] Update place failed:", e);
      await message(`${t("places.saveError")} ${e}`);
    }
  }

  async function eliminarLugarHandler(id: string): Promise<void> {
    if (!await ask(t("places.deleteConfirm"))) return;
    try {
      await eliminarLugar(projectPath, id);
      lugarExpandido = null;
      lugarEditando = null;
      await refreshLugares();
    } catch (e) {
      console.error("[cron-insta] Delete place failed:", e);
      await message(`${t("places.deleteError")} ${e}`);
    }
  }

  // ── Drag-and-drop reorder ────────────────────────────────────
  let dragId = $state<string | null>(null);
  let dragChapter = $state<string | null>(null);

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
      refreshLugares();
    }
  });

  // ── Persist current project path ─────────────────────────────
  $effect(() => {
    if (projectPath) {
      localStorage.setItem("cron-insta-last-project", projectPath);
    }
  });

  // ── Auto-reopen last project on startup ──────────────────────
  let reopeningStatus = $state("");
  $effect(() => {
    const lastPath = localStorage.getItem("cron-insta-last-project");
    if (!lastPath) return;

    reopeningStatus = t("setup.reopening");
    console.log("[cron-insta] Trying to reopen last project:", lastPath);

    cargarIndice(lastPath)
      .then(async (raw) => {
        const meta = JSON.parse(raw);
        projectPath = lastPath;
        setActiveProject(lastPath);
        fontFamily = meta.font_family || "monospace";
        visibleTabs = meta.visible_tabs || {};
        autoSaveInterval = meta.auto_save_interval_minutes || 5;
        await actualizarGitStatus(lastPath);
        detectarYGuardarConfigGit(lastPath); // fire-and-forget
        setTimeout(() => verificarYOfrecerPull(lastPath), 1500);
        chapters = meta.chapters_order ?? [];
        tramas = meta.tramas ?? [];
        chapterTramas = meta.chapter_tramas ?? [];
        console.log("[cron-insta] Project reopened:", meta.project_name, chapters);

        if (chapters.length > 0) {
          return cargarCapituloActual(chapters[0]);
        }
      })
      .catch((e) => {
        console.error("[cron-insta] Failed to reopen last project:", e);
        localStorage.removeItem("cron-insta-last-project");
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

    // Ctrl+Shift+Right — expand sidebar to full width (reference mode)
    if (e.ctrlKey && e.shiftKey && e.key === "ArrowRight") {
      e.preventDefault();
      sidebarSaved = sidebarPct;
      sidebarCollapsed = false;
      sidebarPct = 100;
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

    // Ctrl+Right — grow sidebar by 5 % (max 100 %)
    if (e.ctrlKey && !e.shiftKey && e.key === "ArrowRight") {
      e.preventDefault();
      sidebarCollapsed = false;
      sidebarPct = Math.min(100, sidebarPct + 5);
      sidebarSaved = sidebarPct;
      return;
    }

    // Alt+Left — previous chapter
    if (e.altKey && !e.ctrlKey && !e.shiftKey && e.key === "ArrowLeft") {
      if (!projectPath || !activeChapter) return;
      e.preventDefault();
      capituloAnterior();
      return;
    }

    // Alt+Right — next chapter (or new chapter if at the end)
    if (e.altKey && !e.ctrlKey && !e.shiftKey && e.key === "ArrowRight") {
      if (!projectPath || !activeChapter) return;
      e.preventDefault();
      capituloSiguiente();
      return;
    }

    // Ctrl+S — manual save
    if (e.ctrlKey && !e.shiftKey && e.key === "s") {
      e.preventDefault();
      saveStatus = "saving";
      saveTrigger.trigger();
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
        localStorage.removeItem("cron-insta-last-project");
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
      localStorage.setItem("cron-insta-zoom", String(zoomLevel));
      return;
    }

    // Ctrl+- — zoom out
    if (e.ctrlKey && !e.shiftKey && e.key === "-") {
      e.preventDefault();
      zoomLevel = Math.max(0, zoomLevel - 1);
      localStorage.setItem("cron-insta-zoom", String(zoomLevel));
      return;
    }

    // ? — help toggle (without shift, plain key, only when not typing)
    if (!e.ctrlKey && !e.altKey && !e.metaKey && e.key === "?") {
      if (!editorRef?.isFocused()) {
        e.preventDefault();
        helpMode = !helpMode;
        return;
      }
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

    // Ctrl+T — cycle visible sidebar tabs (skips hidden tabs)
    if (e.ctrlKey && !e.shiftKey && (e.key === "t" || e.key === "T")) {
      e.preventDefault();
      type TabName = "capitulos" | "personajes" | "notas" | "timeline" | "lugares";
      const fullOrder: Record<TabName, TabName[]> = {
        capitulos: ["capitulos", "personajes", "notas", "timeline", "lugares"],
        personajes: ["personajes", "notas", "timeline", "lugares", "capitulos"],
        notas: ["notas", "timeline", "lugares", "capitulos", "personajes"],
        timeline: ["timeline", "lugares", "capitulos", "personajes", "notas"],
        lugares: ["lugares", "capitulos", "personajes", "notas", "timeline"],
      };
      const candidates = fullOrder[activeTab] || fullOrder["capitulos"];
      const tabKeyMap: Record<TabName, string> = {
        capitulos: "chapters",
        personajes: "characters",
        notas: "notes",
        timeline: "timeline",
        lugares: "places",
      };
      let next: TabName = activeTab;
      for (const candidate of candidates) {
        const vk = tabKeyMap[candidate];
        if (!vk || vk === "chapters" || visibleTabs[vk] !== false) {
          next = candidate;
          break;
        }
      }
      activeTab = next;
      setTimeout(() => {
        const firstBtn = document.querySelector<HTMLElement>(".sidebar-content button.chapter-link");
        firstBtn?.focus();
      }, 0);
      return;
    }

    // Ctrl+Enter — dock/undock selected character to editor panel
    if (e.ctrlKey && !e.shiftKey && e.key === "Enter") {
      if (characterDocked) {
        characterDocked = null;
        return;
      }
      if (personajeEditando && personajeExpandido) {
        e.preventDefault();
        characterDocked = {
          id: personajeExpandido,
          name: personajeEditando.name,
          physicalDescription: personajeEditando.physicalDescription,
          personality: personajeEditando.personality,
          traumas: personajeEditando.traumas,
          relationships: personajeEditando.relationships || [],
        };
        return;
      }
    }

    // Ctrl+I — import project from ZIP
    if (e.ctrlKey && !e.shiftKey && (e.key === "i" || e.key === "I")) {
      e.preventDefault();
      importarProyectoHandler();
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

  /** Arrow key navigation within sidebar lists (chapters, characters, notes). */
  function handleListKeydown(e: KeyboardEvent) {
    const list = e.currentTarget as HTMLElement;
    // Only handle vertical navigation keys
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) return;
    e.preventDefault();

    const items = list.querySelectorAll<HTMLElement>("button.chapter-link");
    if (items.length === 0) return;
    const current = document.activeElement;
    const idx = Array.from(items).indexOf(current as HTMLElement);

    let next: HTMLElement | undefined;
    if (e.key === "ArrowDown") {
      next = items[Math.min(idx + 1, items.length - 1)];
    } else if (e.key === "ArrowUp") {
      next = items[Math.max(idx - 1, 0)];
    } else if (e.key === "Home") {
      next = items[0];
    } else if (e.key === "End") {
      next = items[items.length - 1];
    }
    next?.focus();
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
      <!-- Chapters always visible -->
      <button class="tab" class:active={activeTab === "capitulos"}
        onclick={() => { pendingDelete = null; activeTab = "capitulos"; activeNote = ""; }}
        title={t("tabs.chapters")}
      ><Notebook size={18} weight="light" color="currentColor" /></button>
      {#if visibleTabs.characters !== false}
        <button class="tab" class:active={activeTab === "personajes"}
          onclick={() => { pendingDelete = null; activeTab = "personajes"; }}
          title={t("tabs.characters")}
        ><User size={18} weight="light" color="currentColor" /></button>
      {/if}
      {#if visibleTabs.notes !== false}
        <button class="tab" class:active={activeTab === "notas"}
          onclick={() => { pendingDelete = null; activeTab = "notas"; }}
          title={t("tabs.notes")}
        ><Notepad size={18} weight="light" color="currentColor" /></button>
      {/if}
      {#if visibleTabs.timeline !== false}
        <button class="tab" class:active={activeTab === "timeline"}
          onclick={() => { pendingDelete = null; activeTab = "timeline"; }}
          title={t("tabs.timeline")}
        ><Clock size={18} weight="light" color="currentColor" />
          {#if timeline.length > 0}
            <span class="timeline-badge">{timeline.length}</span>
          {/if}
        </button>
      {/if}
      {#if visibleTabs.places !== false}
        <button class="tab" class:active={activeTab === "lugares"}
          onclick={() => { pendingDelete = null; activeTab = "lugares"; }}
          title={t("tabs.places")}
        ><MapTrifold size={18} weight="light" color="currentColor" /></button>
      {/if}
    </nav>

    <div class="sidebar-content">
      <!-- ═══ Capítulos tab ═══ -->
      {#if activeTab === "capitulos"}
        <div class="tab-panel">
          {#if chapters.length > 0}
            <p class="chapter-list-label">{t("chapters.label")}</p>
            <ul class="chapter-list" role="listbox" onkeydown={handleListKeydown}>
              {#each tramaSections as section}
                {@const isUnassigned = section.tramaId === null}
                {@const tramaName = section.nombre ?? t("tramas.unassigned")}
                {@const chapterCount = section.chapters.length}
                {#if !isUnassigned || chapterCount > 0 || tramas.length === 0}
                  <li
                    class="trama-group"
                    class:drag-over={false}
                    ondragover={(e) => {
                      e.preventDefault();
                      (e.currentTarget as HTMLElement).classList.add("drag-over");
                    }}
                    ondragleave={(e) => {
                      (e.currentTarget as HTMLElement).classList.remove("drag-over");
                    }}
                    ondrop={(e) => handleDropOnTrama(e, section.tramaId)}
                  >
                    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                    <div class="trama-header"
                      class:unassigned={isUnassigned}
                      onclick={() => !isUnassigned && toggleTramaCollapse(section.tramaId!)}
                      onkeydown={(e) => { if (e.key === "Enter" && !isUnassigned) toggleTramaCollapse(section.tramaId!); }}
                      role={isUnassigned ? undefined : "button"}
                      tabindex={isUnassigned ? undefined : 0}
                    >
                      <span class="trama-caret">
                        {#if !isUnassigned}
                          {#if tramasCollapsed.has(section.tramaId!)}
                            <CaretRight size={14} weight="light" color="currentColor" />
                          {:else}
                            <CaretDown size={14} weight="light" color="currentColor" />
                          {/if}
                        {:else}
                          <span></span>
                        {/if}
                      </span>
                      <span class="trama-name">{tramaName}</span>
                      <span class="trama-count">{chapterCount}</span>
                      {#if !isUnassigned}
                        {#if tramaPendingDelete === section.tramaId}
                          <button
                            class="trama-delete-confirm"
                            title={t("tramas.deleteConfirm")}
                            onclick={(e) => { e.stopPropagation(); handleEliminarTrama(section.tramaId!); }}
                          >{t("common.delete")}</button>
                        {:else}
                          <button
                            class="trama-delete"
                            title={t("tramas.deleteConfirm")}
                            onclick={(e) => { e.stopPropagation(); handleEliminarTrama(section.tramaId!); }}
                          ><X size={14} weight="light" color="currentColor" /></button>
                        {/if}
                      {/if}
                    </div>
                    {#if isUnassigned || !tramasCollapsed.has(section.tramaId!)}
                      <ul class="chapter-list trama-chapters">
                        {#each section.chapters as ch}
                          <li class="chapter-row"
                            draggable="true"
                            ondragstart={(e) => {
                              dragChapter = ch;
                              (e.currentTarget as HTMLElement).classList.add("dragging");
                            }}
                            ondragend={(e) => {
                              (e.currentTarget as HTMLElement).classList.remove("dragging");
                            }}
                          >
                            <button
                              class="chapter-link"
                              class:active-chapter={activeChapter === ch}
                              onclick={() => { pendingDelete = null; tramaPendingDelete = null; activeNote = ""; cargarCapituloActual(ch); }}
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
                              ><X size={16} weight="light" color="currentColor" /></button>
                            {/if}
                          </li>
                        {/each}
                        {#if section.chapters.length === 0}
                          <li class="trama-empty-hint">{isUnassigned ? t("chapters.empty") : "Sin capítulos"}</li>
                        {/if}
                      </ul>
                    {/if}
                  </li>
                {/if}
              {/each}
            </ul>
          {:else}
            <p class="empty-hint">{t("chapters.empty")}</p>
          {/if}

          <button class="btn-add" onclick={() => crearCapituloNuevo()}>
            <Notebook size={16} weight="light" color="currentColor" /> {t("toolbar.newChapter")}
          </button>
          <button class="btn-add" onclick={() => handleNuevaTrama()}>
            <Notebook size={16} weight="light" color="currentColor" /> + {t("tramas.newPrompt")}
          </button>
        </div>
      {/if}

      <!-- ═══ Personajes tab ═══ -->
      {#if activeTab === "personajes"}
        <div class="tab-panel">
          {#if personajes.length > 0}
            <ul class="chapter-list" role="listbox" onkeydown={handleListKeydown}>
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
                            ><X size={16} weight="light" color="currentColor" /></button>
                          </div>
                        {/each}
{/if}

{#if exportModal}
  <!-- Export modal -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" role="dialog" tabindex="-1"
    aria-label={t("export.title")}
    onclick={() => { exportModal = false; }}
    onkeydown={(e) => { if (e.key === "Escape") exportModal = false; }}>
    <div class="modal-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2><Package size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("export.title")}</h2>
      <p class="modal-desc">{t("export.desc")}</p>

      <div class="export-options">
        <button class="export-option" onclick={async () => {
          try {
            const result = await exportarProyectoZip(projectPath);
            exportModal = false;
            await message(t("export.zipSuccess") + "\n" + result);
          } catch (e) {
            await message(t("export.error") + " " + e);
          }
        }}>
          <span class="export-option-icon"><Export size={16} weight="light" color="currentColor" /></span>
          <span class="export-option-title">{t("export.zipTitle")}</span>
          <span class="export-option-hint">{t("export.zipHint")}</span>
        </button>

        <button class="export-option" onclick={async () => {
          try {
            const result = await exportarProyectoMd(projectPath);
            exportModal = false;
            await message(t("export.mdSuccess") + "\n" + result);
          } catch (e) {
            await message(t("export.error") + " " + e);
          }
        }}>
          <span class="export-option-icon"><FileText size={16} weight="light" color="currentColor" /></span>
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
              <UserPlus size={16} weight="light" color="currentColor" /> {t("characters.new")}
            </button>
          {/if}
        </div>
      {/if}

      <!-- ═══ Notas tab ═══ -->
      {#if activeTab === "notas"}
        <div class="tab-panel">
          {#if notas.length > 0}
            <ul class="chapter-list" role="listbox" onkeydown={handleListKeydown}>
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
                  ><X size={16} weight="light" color="currentColor" /></button>
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
            <Notepad size={16} weight="light" color="currentColor" /> {t("notes.new")}
          </button>
        </div>
      {/if}

      <!-- ═══ Timeline tab ═══ -->
      {#if activeTab === "timeline"}
        <div class="tab-panel">
          {#if timeline.length > 0}
            <ul class="timeline-list">
              {#each timeline as evt}
                <li
                  class="timeline-event"
                  class:expanded={eventoExpandido === evt.id}
                  draggable="true"
                  ondragstart={(e) => handleDragStart(e, evt.id)}
                  ondragend={(e) => handleDragEnd(e)}
                  ondragover={(e) => handleDragOver(e)}
                  ondragleave={(e) => handleDragLeave(e)}
                  ondrop={(e) => handleDrop(e, evt.id)}
                >
                  <div
                    class="event-row"
                    onclick={() => toggleEventoExpandido(evt.id)}
                    onkeydown={(e) => { if (e.key === "Enter") toggleEventoExpandido(evt.id); }}
                    role="button"
                    tabindex="0"
                  >
                    {#if evt.date}
                      <span class="event-moment">{evt.date}</span>
                    {/if}
                    <span class="event-title">{evt.title}</span>
                    <span class="event-expand-icon">
                      {#if eventoExpandido === evt.id}
                        <CaretDown size={16} weight="light" color="currentColor" />
                      {:else}
                        <CaretRight size={16} weight="light" color="currentColor" />
                      {/if}
                    </span>
                    <button
                      class="item-edit"
                      title={t("timeline.editTitle")}
                      onclick={(e) => { e.stopPropagation(); editarEvento(evt); }}
                    ><PencilSimple size={16} weight="light" color="currentColor" /></button>
                    <button
                      class="item-delete"
                      title={t("timeline.deleteTitle")}
                      onclick={(e) => { e.stopPropagation(); eliminarEventoHandler(evt.id); }}
                    ><X size={16} weight="light" color="currentColor" /></button>
                  </div>
                  {#if eventoExpandido === evt.id}
                    <div class="event-details">
                      {#if evt.date}
                        <p class="event-meta"><strong>{t("timeline.date")}:</strong> {evt.date}</p>
                      {/if}
                      {#if evt.description}
                        <p class="event-description">{evt.description}</p>
                      {/if}
                      {#if evt.relatedCharacters?.length}
                        <p class="event-meta"><strong>{t("timeline.relatedCharacters")}:</strong>
                          {evt.relatedCharacters.map((id: string) => getPersonajeName(id)).join(", ")}</p>
                      {/if}
                      {#if evt.relatedChapters?.length}
                        <p class="event-meta"><strong>{t("timeline.relatedChapters")}:</strong>
                          {evt.relatedChapters.join(", ")}</p>
                      {/if}
                      {#if evt.relatedPlaces?.length}
                        <p class="event-meta"><strong>{t("timeline.relatedPlaces")}:</strong>
                          {evt.relatedPlaces.map((id: string) => getLugarName(id)).join(", ")}</p>
                      {/if}
                    </div>
                  {/if}
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-hint">{t("timeline.empty")}</p>
          {/if}

          {#if eventoFormVisible}
            <div class="inline-form">
              <label class="field-label" for="evt-moment">{t("timeline.date")}</label>
              <div class="date-input-row">
                <input
                  id="evt-moment"
                  class="field-input"
                  type="text"
                  bind:value={nuevoEventoFecha}
                  placeholder={t("timeline.datePlaceholder")}
                />
                <input
                  type="date"
                  class="date-picker-cal"
                  bind:value={nuevoEventoFecha}
                  title={t("timeline.pickDate")}
                />
              </div>
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

              {#if lugares.length > 0}
                <span class="field-label">{t("timeline.relatedPlaces")}</span>
                <div class="checkbox-group">
                  {#each lugares as l}
                    <label class="checkbox-label">
                      <input
                        type="checkbox"
                        checked={nuevoEventoLugares.includes(l.id)}
                        onchange={() => nuevoEventoLugares = togglePersonajeCapitulo(nuevoEventoLugares, l.id)}
                      />
                      {l.name}
                    </label>
                  {/each}
                </div>
              {/if}

              <div class="form-actions">
                <button class="btn-sm btn-primary" onclick={guardarEventoHandler}>
                  {eventoEditando ? t("timeline.save") : t("timeline.add")}
                </button>
                <button class="btn-sm" onclick={cancelarEdicion}>{t("common.cancel")}</button>
              </div>
            </div>
          {:else}
            <button class="btn-add" onclick={() => { cancelarEdicion(); eventoFormVisible = true; }}>
              <Clock size={16} weight="light" color="currentColor" /> {t("timeline.newEvent")}
            </button>
          {/if}
        </div>
      {/if}

      <!-- ═══ Places tab ═══ -->
      {#if activeTab === "lugares"}
        <div class="tab-panel">
          {#if lugares.length > 0}
            <ul class="chapter-list" role="listbox" onkeydown={handleListKeydown}>
              {#each lugares as l}
                <li>
                  <button
                    class="chapter-link"
                    class:active-chapter={lugarExpandido === l.id}
                    onclick={() => seleccionarLugar(l.id)}
                  >
                    {l.name}
                  </button>

                  {#if lugarExpandido === l.id && lugarEditando}
                    <div class="inline-form">
                      <label class="field-label" for="place-name-{l.id}">{t("places.namePlaceholder")}</label>
                      <input
                        id="place-name-{l.id}"
                        class="field-input"
                        type="text"
                        bind:value={lugarEditando.name}
                      />

                      <label class="field-label" for="place-desc-{l.id}">{t("places.descriptionPlaceholder")}</label>
                      <textarea
                        id="place-desc-{l.id}"
                        class="field-textarea"
                        bind:value={lugarEditando.description}
                        rows="3"
                        placeholder={t("places.descriptionPlaceholder")}
                      ></textarea>

                      <div class="form-actions">
                        <button class="btn-sm btn-primary" onclick={guardarLugarHandler}>{t("places.save")}</button>
                        <button class="btn-sm btn-danger" onclick={() => eliminarLugarHandler(l.id)}>{t("places.delete")}</button>
                      </div>
                    </div>
                  {/if}
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-hint">{t("places.empty")}</p>
          {/if}

          {#if lugarFormVisible}
            <div class="inline-form">
              <input
                class="field-input"
                type="text"
                placeholder={t("places.namePlaceholder")}
                bind:value={lugarNuevoNombre}
                onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter") crearLugarHandler(); }}
              />
              <textarea
                class="field-textarea"
                placeholder={t("places.descriptionPlaceholder")}
                bind:value={lugarNuevaDescripcion}
                rows="2"
              ></textarea>
              <div class="form-actions">
                <button class="btn-sm btn-primary" onclick={crearLugarHandler}>{t("places.create")}</button>
                <button class="btn-sm" onclick={() => lugarFormVisible = false}>{t("common.cancel")}</button>
              </div>
            </div>
          {:else}
            <button class="btn-add" onclick={() => lugarFormVisible = true}>
              <MapTrifold size={16} weight="light" color="currentColor" /> {t("places.new")}
            </button>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Sidebar footer — tools + git, collapsible -->
    <div class="sidebar-footer">
      <button
        class="footer-toggle"
        onclick={() => (footerExpanded = !footerExpanded)}
        title={footerExpanded ? t("toolbar.collapseFooter") : t("toolbar.expandFooter")}
      >
        {#if footerExpanded}
          <CaretDown size={16} weight="light" color="currentColor" />
        {:else}
          <CaretUp size={16} weight="light" color="currentColor" />
        {/if}
      </button>

      {#if footerExpanded}
        <div class="footer-rows">
          <!-- Row 1: project management (general) -->
          <div class="footer-row">
            <button class="footer-btn" onclick={nuevoProyectoHandler} title={t("toolbar.newProjectTitle")}>
              <Sparkle size={18} weight="light" color="currentColor" /> {t("toolbar.newProject")}
            </button>
            <button class="footer-btn" onclick={() => abrirProyecto()} title={t("toolbar.openProjectTitle")}>
              <FolderOpen size={18} weight="light" color="currentColor" /> {t("toolbar.openProject")}
            </button>
            <button class="footer-btn" onclick={importarProyectoHandler} title={t("import.title")}>
              <DownloadSimple size={18} weight="light" color="currentColor" /> {t("import.button")}
            </button>
          </div>

          <!-- Row 3: current project actions -->
          <div class="footer-row">
            <button class="footer-btn" onclick={() => (settingsOpen = true)} title={t("settings.settings")}>
              <Gear size={18} weight="light" color="currentColor" /> {t("settings.settings")}
            </button>
            <span class="footer-sep"></span>
            <button class="footer-btn" onclick={async () => {
              console.log("[cron-insta] Export button clicked");
              try {
                const result = await exportarProyectoZip(projectPath);
                await message(t("export.zipSuccess") + "\n" + result);
              } catch (e) {
                await message(t("export.error") + " " + e);
              }
            }} title={t("export.zipTitle")}>
              <Export size={18} weight="light" color="currentColor" /> {t("export.export")}
            </button>
            <button class="footer-btn" onclick={async () => {
              try {
                const result = await exportarProyectoMd(projectPath);
                await message(t("export.mdSuccess") + "\n" + result);
              } catch (e) {
                await message(t("export.error") + " " + e);
              }
            }} title={t("export.mdTitle")}>
              <FileText size={18} weight="light" color="currentColor" /> {t("export.share")}
            </button>
            <span class="footer-sep"></span>
            <button class="footer-btn" onclick={() => cerrarProyecto()} title={t("toolbar.closeProjectTitle")}>
              <XCircle size={18} weight="light" color="currentColor" /> {t("toolbar.closeProject")}
            </button>
          </div>

          <!-- Row 4: save -->
          <div class="footer-row">
            <button
              class="footer-btn"
              onclick={async () => { await doSave(); }}
              title={t("toolbar.saveTitle")}
            ><FloppyDisk size={18} weight="light" color="currentColor" /> {t("toolbar.save")}</button>
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

          <!-- Row 5: versioning -->
          {#if gitStatus !== "unknown"}
            <div class="footer-row">
              {#if gitStatus === "active"}
                <span class="git-indicator git-active" title={t("git.activeTitle")}>🟢 {t("git.active")}</span>
                <button class="git-log-link" onclick={cargarGitLog}>{t("git.viewSessions")} <ArrowRight size={16} weight="light" color="currentColor" /></button>
                {#if hasRemote}
                  <button
                    class="pull-now-btn"
                    onclick={() => { if (projectPath) verificarYOfrecerPull(projectPath); }}
                    title={t("git.pullNowTitle")}
                  >
                    <DownloadSimple size={16} weight="light" color="currentColor" /> {t("git.pullNow")}
                  </button>
                {/if}
                {#if pushReady}
                  <button
                    class="push-now-btn"
                    onclick={async () => {
                      if (!projectPath) return;
                      saveStatus = "saving";
                      try {
                        const result = await pushAhora(projectPath);
                        const successMsg = result.split("\n").pop() || t("git.pushSuccess");
                        showToast(successMsg, "warning");
                        // Result starts with ✅ or ⚠️ — success either way (commit went through)
                      } catch (e) {
                        showToast(t("git.pushFailed") + " " + String(e), "error");
                      }
                      saveStatus = "saved";
                      actualizarGitStatus(projectPath!);
                    }}
                    title={t("git.pushNowTitle")}
                  >
                    <PaperPlaneTilt size={16} weight="light" color="currentColor" /> {t("git.pushNow")}
                  </button>
                {/if}
                {#if remoteWarningVisible}
                  <span class="remote-warning-icon"><Warning size={16} weight="light" color="currentColor" /></span>
                  <button
                    class="remote-warning-btn"
                    onclick={() => (remoteWarningDialog = true)}
                    title={t("git.pushDisabled")}
                  >
                    {t("git.toolbarRetry")}
                  </button>
                {/if}
              {:else if gitStatus === "not-initialized"}
                <button class="git-indicator git-warn"
                  onclick={async () => {
                    let initRemoteUrl: string | undefined;
                    try {
                      const result = await showIdentityDialog(projectPath);
                      await inicializarGit(projectPath);
                      if (result.remoteConfigured && result.remoteUrl) {
                        initRemoteUrl = result.remoteUrl;
                        await configurarRemoto(projectPath, result.remoteUrl);
                        await guardarConfigRemoto(projectPath, result.remoteUrl, true);
                      }
                      await actualizarGitStatus(projectPath);
                    } catch (e) {
                      console.error("[cron-insta] Git init failed:", e);
                      const msg = String(e);
                      if (msg.startsWith("REPO_NOT_FOUND:")) {
                        const repoName = initRemoteUrl ? extractRepoName(initRemoteUrl) : null;
                        showToast(
                          t("git.repoNotFound"),
                          "warning",
                          repoName
                            ? { label: t("git.createOnGithub"), onClick: () => openUrl(`https://github.com/new?name=${repoName}`) }
                            : undefined,
                        );
                      } else if (msg.startsWith("REMOTE_HAS_COMMITS:")) {
                        const desc = msg.replace("REMOTE_HAS_COMMITS:", "");
                        const shouldSync = await ask(desc);
                        if (shouldSync) {
                          try {
                            const syncMsg = await sincronizarRemoto(projectPath);
                  showToast(syncMsg || t("git.syncSuccess"), "warning", undefined, CheckCircle);
                          } catch (syncErr) {
                            showToast(String(syncErr), "error");
                          }
                        }
                      } else {
                        await message(t("git.initError") + " " + e);
                      }
                    }
                  }}
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
          <button
            class="btn-secondary"
            onclick={importarProyectoHandler}
          >
            {t("import.button")}
          </button>
        </div>
      </div>
      {/if}
    {:else}
      <!-- Toolbar + Editor -->
      <div class="editor-pane">
        <div class="editor-toolbar">
          <span class="project-label" title={projectPath}>
            {projectDisplayName}
          </span>
          {#if activeChapter}
            <span class="chapter-label">{activeChapter}</span>
          {:else}
            <span></span>
          {/if}
          <div class="toolbar-actions">
            <button
              class="toolbar-icon-btn"
              onclick={() => (globalSettingsOpen = true)}
              title={t("settings.settings")}
            ><Gear size={16} weight="light" color="currentColor" /></button>
            <button
              class="toolbar-icon-btn"
              onclick={() => (helpMode = !helpMode)}
              title={t("toolbar.helpTitle")}
            ><Question size={16} weight="light" color="currentColor" /></button>
          </div>
        </div>

        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="editor-body"
          role="textbox"
          aria-multiline="true"
          tabindex="0"
          oncontextmenu={(e) => {
            e.preventDefault();
            const text = editorRef?.getSelectedText() || "";
            contextMenu = { open: true, x: e.clientX, y: e.clientY, selectedText: text };
          }}
        >
          <Editor
            bind:this={editorRef}
            content={editorContent}
            {fontFamily}
            onUpdate={handleEditorUpdate}
            onNearEnd={(v) => cercaDelFinal = v}
          />
        </div>

        {#if projectPath && activeChapter && chapters.length > 0}
          {@const idx = chapters.indexOf(activeChapter)}
          <div class="chapter-nav-footer" class:visible={cercaDelFinal}>
            <button
              class="nav-btn"
              disabled={idx <= 0}
              onclick={capituloAnterior}
              title={t("chapterNav.prev")}
            ><CaretLeft size={16} weight="light" color="currentColor" aria-hidden="true" /></button>
            <span class="nav-position">{idx + 1} / {chapters.length}</span>
            {#if idx < chapters.length - 1}
              <button
                class="nav-btn"
                onclick={capituloSiguiente}
                title={t("chapterNav.next")}
              ><CaretRight size={16} weight="light" color="currentColor" aria-hidden="true" /></button>
            {:else}
              <button
                class="nav-btn nav-btn-new"
                onclick={capituloSiguiente}
                title={t("chapterNav.newChapter")}
              ><Notebook size={16} weight="light" color="currentColor" aria-hidden="true" /></button>
            {/if}
          </div>
        {/if}

        {#if characterDocked}
          <div class="character-dock" transition:fly={{ x: 300, duration: 200 }}>
            <div class="character-dock-header">
              <h3><PushPin size={16} weight="light" color="currentColor" aria-hidden="true" /> {characterDocked.name}</h3>
              <button class="character-dock-close" onclick={() => characterDocked = null}
                title={t("characters.undock")}><XCircle size={16} weight="light" color="currentColor" /></button>
            </div>
            <div class="character-dock-body">
              {#if characterDocked.physicalDescription}
                <div class="char-dock-field">
                  <span class="char-dock-label">{t("characters.physicalDescription")}</span>
                  <p>{characterDocked.physicalDescription}</p>
                </div>
              {/if}
              {#if characterDocked.personality}
                <div class="char-dock-field">
                  <span class="char-dock-label">{t("characters.personality")}</span>
                  <p>{characterDocked.personality}</p>
                </div>
              {/if}
              {#if characterDocked.traumas}
                <div class="char-dock-field">
                  <span class="char-dock-label">{t("characters.traumas")}</span>
                  <p>{characterDocked.traumas}</p>
                </div>
              {/if}
              {#if characterDocked.relationships.length > 0}
                <div class="char-dock-field">
                  <span class="char-dock-label">{t("characters.relationships")}</span>
                  <ul class="char-dock-rels">
                    {#each characterDocked.relationships as rel}
                      <li>{rel.targetName}{#if rel.type} — {rel.type}{/if}{#if rel.notes}: {rel.notes}{/if}</li>
                    {/each}
                  </ul>
                </div>
              {/if}
            </div>
          </div>
        {/if}
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
        <h2>Cron-Insta</h2>
        <span class="help-version">v{APP_VERSION}</span>
        <button class="help-close" onclick={() => (helpMode = false)} title={t("common.cancel")}><X size={16} weight="light" color="currentColor" /></button>
      </div>

      <p class="help-creator">{t("help.createdBy")} <a href="mailto:galejan@gmail.com">galejan@gmail.com</a></p>

      <div class="help-section">
        <h3><BookOpen size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("help.editorTitle")}</h3>
        <p>{t("help.editorDesc")}</p>
      </div>

      <div class="help-section">
        <h3><FolderOpen size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("help.chaptersTitle")}</h3>
        <p>{@html t("help.chaptersDesc")}</p>
      </div>

      <div class="help-section">
        <h3><User size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("help.charactersTitle")}</h3>
        <p>{t("help.charactersDesc")}</p>
      </div>

      <div class="help-section">
        <h3><NotePencil size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("help.notesTitle")}</h3>
        <p>{t("help.notesDesc")}</p>
      </div>

      <div class="help-section">
        <h3><Clock size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("help.timelineTitle")}</h3>
        <p>{t("help.timelineDesc")}</p>
      </div>

      <div class="help-section">
        <h3><MapTrifold size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("help.placesTitle")}</h3>
        <p>{t("help.placesDesc")}</p>
      </div>

      <div class="help-section">
        <h3><GitBranch size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("help.versioningTitle")}</h3>
        <p>{t("help.versioningDesc")}</p>
      </div>

      <div class="help-section">
        <h3><Gear size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("help.settingsTitle")}</h3>
        <p>{t("help.settingsDesc")}</p>
      </div>

      <div class="help-section">
        <h3><Package size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("help.exportTitle")}</h3>
        <p>{t("help.exportDesc")}</p>
      </div>

      <div class="help-section">
        <h3><ChatText size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("help.dialogDashTitle")}</h3>
        <p>{t("help.dialogDashDesc")}</p>
      </div>

      <div class="help-section">
        <h3><Keyboard size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("help.shortcutsTitle")}</h3>
        <table class="help-shortcuts">
          <tbody>
          <tr><td><kbd>Ctrl+Shift+←</kbd></td><td>{t("help.shortcuts.toggleSidebar")}</td></tr>
          <tr><td><kbd>Ctrl+Shift+→</kbd></td><td>{t("help.shortcuts.fullSidebar")}</td></tr>
          <tr><td><kbd>Ctrl+←</kbd> / <kbd>Ctrl+→</kbd></td><td>{t("help.shortcuts.resizeSidebar")}</td></tr>
          <tr><td><kbd>Ctrl+P</kbd></td><td>{t("help.shortcuts.toggleFooter")}</td></tr>
          <tr><td><kbd>Ctrl+S</kbd></td><td>{t("help.shortcuts.saveNow")}</td></tr>
          <tr><td><kbd>Ctrl+N</kbd></td><td>{t("help.shortcuts.newChapter")}</td></tr>
          <tr><td><kbd>Alt+←</kbd> / <kbd>Alt+→</kbd></td><td>{t("help.shortcuts.prevNextChapter")}</td></tr>
          <tr><td><kbd>Ctrl+O</kbd></td><td>{t("help.shortcuts.openProject")}</td></tr>
          <tr><td><kbd>Ctrl+Shift+N</kbd></td><td>{t("help.shortcuts.newProject")}</td></tr>
          <tr><td><kbd>Ctrl+T</kbd></td><td>{t("help.shortcuts.cycleTabs")}</td></tr>
          <tr><td><kbd>Ctrl+Enter</kbd></td><td>{t("help.shortcuts.dockCharacter")}</td></tr>
          <tr><td><kbd>Ctrl+I</kbd></td><td>{t("help.shortcuts.importProject")}</td></tr>
          <tr><td><kbd>Ctrl+↑</kbd> / <kbd>Ctrl+↓</kbd></td><td>{t("help.shortcuts.applyHeading")}</td></tr>
          <tr><td><kbd>Ctrl+D</kbd></td><td>{t("help.shortcuts.dialogDash")}</td></tr>
          <tr><td><kbd>Ctrl++</kbd> / <kbd>Ctrl+-</kbd></td><td>{t("help.shortcuts.zoomIn")} / {t("help.shortcuts.zoomOut")}</td></tr>
          <tr><td><kbd>F11</kbd></td><td>{t("help.shortcuts.fullscreen")}</td></tr>
          <tr><td><kbd>F1</kbd> o <kbd>?</kbd></td><td>{t("help.shortcuts.toggleHelp")}</td></tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
{/if}

<!-- Git Identity dialog (replaces old git init modal) -->
<GitIdentityDialog
  bind:open={identityDialogOpen}
  projectPath={identityDialogPath}
  projectName={identityDialogProjectName}
  onComplete={(ctx: {remoteConfigured: boolean, remoteUrl: string}) => {
    identityDialogResolve?.(ctx);
    identityDialogResolve = null;
  }}
/>

<!-- Project Settings dialog -->
<ProjectSettingsDialog
  bind:open={settingsOpen}
  projectPath={projectPath}
  currentFontFamily={fontFamily}
  currentVisibleTabs={visibleTabs}
  currentAutoSaveInterval={autoSaveInterval}
  onFontSaved={(font: string) => { fontFamily = font; }}
  onConfigSaved={(config: {visible_tabs: Record<string,boolean>; auto_save_interval_minutes: number}) => {
    visibleTabs = config.visible_tabs;
    autoSaveInterval = config.auto_save_interval_minutes;
  }}
/>

<!-- Global Settings dialog -->
<GlobalSettingsDialog bind:open={globalSettingsOpen} />

<!-- Editor context menu -->
<EditorContextMenu
  x={contextMenu.x}
  y={contextMenu.y}
  bind:open={contextMenu.open}
  selectedText={contextMenu.selectedText}
  onClose={() => { contextMenu.open = false; }}
  onCut={() => { editorRef?.deleteSelection(); }}
  onSaveAsNote={handleSaveAsNote}
  onSaveAsTrait={handleSaveAsTrait}
  onNewChapter={handleNewChapterFromContext}
  onAddAsEvent={handleAddAsEventFromContext}
  onAddToPlace={handleAddToPlace}
/>

<!-- Remote sync warning mini-dialog -->
{#if remoteWarningDialog}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    tabindex="-1"
    aria-label={t("git.pushDisabled")}
    onclick={() => (remoteWarningDialog = false)}
    onkeydown={(e) => e.key === "Escape" && (remoteWarningDialog = false)}
  >
    <div class="modal-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2>{t("git.pushDisabled")}</h2>
      <p class="modal-desc">{t("git.pushFailed")}</p>
      <div class="modal-actions">
        <button
          class="btn-secondary"
          onclick={() => (remoteWarningDialog = false)}
        >
          {t("common.cancel")}
        </button>
        <button
          class="btn-secondary"
          onclick={async () => {
            remoteWarningDialog = false;
            let reconfRemoteUrl: string | undefined;
            try {
              const result = await showIdentityDialog(projectPath);
              if (result.remoteConfigured && result.remoteUrl) {
                reconfRemoteUrl = result.remoteUrl;
                await configurarRemoto(projectPath, result.remoteUrl);
                await guardarConfigRemoto(projectPath, result.remoteUrl, true);
                remoteWarningVisible = false;
                showToast(t("git.syncSuccess"), "warning", undefined, CheckCircle);
              }
              await actualizarGitStatus(projectPath);
            } catch (e) {
              const msg = String(e);
              if (msg.startsWith("REPO_NOT_FOUND:")) {
                const repoName = reconfRemoteUrl ? extractRepoName(reconfRemoteUrl) : null;
                showToast(
                  t("git.repoNotFound"),
                  "warning",
                  repoName
                    ? { label: t("git.createOnGithub"), onClick: () => openUrl(`https://github.com/new?name=${repoName}`) }
                    : undefined,
                );
              } else if (msg.startsWith("REMOTE_HAS_COMMITS:")) {
                const desc = msg.replace("REMOTE_HAS_COMMITS:", "");
                const shouldSync = await ask(desc);
                if (shouldSync) {
                  try {
                    const syncMsg = await sincronizarRemoto(projectPath);
                    showToast(syncMsg || t("git.syncSuccess"), "warning", undefined, CheckCircle);
                  } catch (syncErr) {
                    showToast(String(syncErr), "error");
                  }
                }
              } else {
                showToast(msg, "error");
              }
            }
          }}
        >
          {t("git.toolbarReconfigure")}
        </button>
        <button
          class="btn-primary"
          onclick={async () => {
            remoteWarningDialog = false;
            try {
              await reintentarPush(projectPath);
              remoteWarningVisible = false;
              showToast(t("git.syncSuccess"), "warning", undefined, CheckCircle);
              await actualizarGitStatus(projectPath);
            } catch (e) {
              showToast(String(e), "error");
            }
          }}
        >
          {t("git.toolbarRetry")}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Pull dialog: remote has new commits -->
{#if pullDialogOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    tabindex="-1"
    aria-label={t("git.pullAvailable")}
    onclick={rechazarPull}
    onkeydown={(e) => e.key === "Escape" && rechazarPull()}
  >
    <div class="modal-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2>{t("git.pullAvailable")}</h2>
      <p class="modal-desc">{t("git.pullAvailableDesc").replace("{count}", String(remoteBehind))}</p>
      <div class="modal-actions">
        <button class="btn-secondary" onclick={rechazarPull}>
          {t("common.cancel")}
        </button>
        <button class="btn-primary" onclick={ejecutarPull}>
          {t("git.pullNow")}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Text input picker modal (replaces native prompt()) -->
{#if textPickerOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    tabindex="-1"
    aria-label={textPickerMessage}
    onkeydown={(e) => e.key === "Escape" && textPickerResolve?.(null) && (textPickerOpen = false)}
  >
    <div class="modal-panel text-picker-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2>{textPickerMessage}</h2>
      <!-- svelte-ignore a11y_autofocus — modal picker, focus is intentional -->
      <input
        type="text"
        class="modal-input"
        bind:value={textPickerValue}
        onkeydown={(e) => {
          if (e.key === "Enter") { textPickerResolve?.(textPickerValue); textPickerOpen = false; }
        }}
        autofocus
      />
      <div class="modal-actions">
        <button class="btn-secondary" onclick={() => { textPickerResolve?.(null); textPickerOpen = false; }}>
          {t("common.cancel")}
        </button>
        <button class="btn-primary" onclick={() => { textPickerResolve?.(textPickerValue); textPickerOpen = false; }}>
          {t("common.accept")}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Trama selector dialog (3-option: existing, new, skip) -->
{#if tramaSelectorOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    tabindex="-1"
    aria-label={t("tramas.selectTitle")}
    onclick={() => resolveTramaSelector(undefined)}
    onkeydown={(e) => e.key === "Escape" && resolveTramaSelector(undefined)}
  >
    <div class="modal-panel trama-selector-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2>{t("tramas.selectTitle")}</h2>
      <div class="trama-selector-options">
        <button class="trama-selector-btn" onclick={handleTramaSelectExisting}>
          <Notebook size={16} weight="light" color="currentColor" />
          <span>{t("tramas.selectExisting")}</span>
        </button>
        <button class="trama-selector-btn" onclick={handleTramaSelectNew}>
          <Notebook size={16} weight="light" color="currentColor" />
          <span>{t("tramas.selectNew")}</span>
        </button>
        <button class="trama-selector-btn" onclick={handleTramaSelectSkip}>
          <Notebook size={16} weight="light" color="currentColor" />
          <span>{t("tramas.selectSkip")}</span>
        </button>
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" onclick={() => resolveTramaSelector(undefined)}>
          {t("common.cancel")}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Toast notification -->
{#if toast}
  <div class="toast" class:toast-error={toast.type === "error"}>
    {#if toast.icon}
      {@const IconComponent = toast.icon}
      <IconComponent size={16} weight="light" color="currentColor" />
    {/if}
    {toast.message}
    {#if toast.action}
      <button class="toast-action" onclick={toast.action.onClick}>{toast.action.label}</button>
    {/if}
    <button class="toast-close" onclick={() => (toast = null)} title={t("common.cancel")}>
      <X size={16} weight="light" color="currentColor" />
    </button>
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

{#if configFormOpen}
  <!-- Project config wizard modal -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" role="dialog" tabindex="-1"
    aria-label={t("config.titleNew")}
    onkeydown={(e) => e.key === "Escape" && (configFormOpen = false)}>
    <div class="modal-panel config-form-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h2>{t("config.titleNew")}</h2>
      <ProjectConfigForm mode="new"
        onComplete={(config) => {
          configFormOpen = false;
          configFormResolve?.({
            font_family: config.font_family,
            visible_tabs: config.visible_tabs as unknown as Record<string, boolean>,
            auto_save_interval_minutes: config.auto_save_interval_minutes,
          });
        }}
        onCancel={() => {
          configFormOpen = false;
          configFormResolve?.({
            font_family: "monospace",
            visible_tabs: { chapters: true, characters: true, places: true, timeline: true, notes: true },
            auto_save_interval_minutes: 5,
          });
        }}
      />
    </div>
  </div>
{:else if fontPickerOpen}
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
      <h2><Scroll size={16} weight="light" color="currentColor" aria-hidden="true" /> {t("git.sessionsTitle")}</h2>
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
                    <span class="git-log-file-badge"><FileText size={16} weight="light" color="currentColor" aria-hidden="true" /> {file}</span>
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
      <p class="closing-sub">Cron-Insta v{APP_VERSION}</p>
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

  /* Force dark scrollbars on Windows when dark theme is active */
  :global(.dark) {
    scrollbar-color: #334155 #1e293b;
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
    height: var(--header-height, 2.5rem);
    border-bottom: 1px solid #e2e8f0;
    align-items: center;
  }

  :global(.dark) .tabs {
    border-bottom-color: #334155;
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
    height: 100%;
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
    position: relative;
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
    grid-template-columns: 1fr auto auto 1fr;
    align-items: center;
    padding: 0 1rem;
    border-bottom: 1px solid #e2e8f0;
    background: #f8fafc;
    flex-shrink: 0;
    height: var(--header-height, 2.5rem);
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

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    justify-self: end;
  }

  .toolbar-icon-btn {
    width: 1.5rem;
    height: 1.5rem;
    padding: 0;
    border: 1px solid #e2e8f0;
    border-radius: 50%;
    background: transparent;
    color: #64748b;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 120ms;
  }

  .toolbar-icon-btn:hover {
    background: #e2e8f0;
  }

  :global(.dark) .toolbar-icon-btn {
    border-color: #334155;
    color: #94a3b8;
  }

  :global(.dark) .toolbar-icon-btn:hover {
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

  /* ── Chapter navigation footer ──────────────────────────────── */
  .chapter-nav-footer {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    padding: 0.375rem 1rem;
    border-top: 1px solid transparent;
    background: #f8fafc;
    max-height: 0;
    overflow: hidden;
    opacity: 0;
    transition: max-height 250ms ease, opacity 200ms ease, padding 250ms ease, border-color 250ms ease;
  }
  .chapter-nav-footer.visible {
    max-height: 2.5rem;
    opacity: 1;
    border-top-color: #e2e8f0;
  }

  :global(.dark) .chapter-nav-footer {
    background: #0f172a;
  }
  :global(.dark) .chapter-nav-footer.visible {
    border-top-color: #334155;
  }

  .nav-btn {
    background: none;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    color: #475569;
    font-size: 0.875rem;
    padding: 0.125rem 0.625rem;
    cursor: pointer;
    transition: background 120ms, color 120ms;
  }
  .nav-btn:hover:not(:disabled) {
    background: #e2e8f0;
    color: #1e293b;
  }
  .nav-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  :global(.dark) .nav-btn {
    border-color: #475569;
    color: #94a3b8;
  }
  :global(.dark) .nav-btn:hover:not(:disabled) {
    background: #1e293b;
    color: #e2e8f0;
  }

  .nav-btn-new {
    color: #3b82f6;
    font-weight: 700;
    font-size: 1rem;
  }
  :global(.dark) .nav-btn-new {
    color: #60a5fa;
  }

  .nav-position {
    font-size: 0.8125rem;
    color: #94a3b8;
    user-select: none;
    min-width: 3rem;
    text-align: center;
  }
  :global(.dark) .nav-position {
    color: #64748b;
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

  .item-edit,
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

  .item-edit:hover {
    background: #dbeafe;
    color: #3b82f6;
  }
  :global(.dark) .item-edit:hover {
    background: #1e3a5f33;
    color: #60a5fa;
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

  /* ── Trama groups ──────────────────────────────────────────── */
  .trama-group {
    margin-bottom: 0.25rem;
    border-radius: 4px;
    transition: background 120ms;
  }

  .trama-group.drag-over {
    background: #eff6ff;
    outline: 2px dashed #3b82f6;
    outline-offset: -2px;
  }

  :global(.dark) .trama-group.drag-over {
    background: #1e3a5f;
    outline-color: #60a5fa;
  }

  .trama-header {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    cursor: pointer;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    color: #475569;
    user-select: none;
    transition: background 120ms;
  }

  .trama-header:hover {
    background: #f1f5f9;
  }

  :global(.dark) .trama-header {
    color: #94a3b8;
  }
  :global(.dark) .trama-header:hover {
    background: #1e293b;
  }

  .trama-header.unassigned {
    cursor: default;
    color: #94a3b8;
  }
  :global(.dark) .trama-header.unassigned {
    color: #64748b;
  }

  .trama-caret {
    width: 14px;
    display: flex;
    align-items: center;
    color: #94a3b8;
  }

  .trama-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .trama-count {
    font-size: 0.6875rem;
    color: #94a3b8;
    background: #f1f5f9;
    padding: 0.0625rem 0.375rem;
    border-radius: 8px;
  }

  :global(.dark) .trama-count {
    color: #64748b;
    background: #334155;
  }

  .trama-delete,
  .trama-delete-confirm {
    flex-shrink: 0;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: #94a3b8;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 120ms, background 120ms, color 120ms;
  }

  .trama-header:hover .trama-delete {
    opacity: 1;
  }

  .trama-delete:hover {
    background: #fee2e2;
    color: #ef4444;
  }

  :global(.dark) .trama-delete:hover {
    background: #7f1d1d33;
    color: #f87171;
  }

  .trama-delete-confirm {
    opacity: 1;
    background: #ef4444;
    color: white;
    font-size: 0.625rem;
    width: auto;
    padding: 0.0625rem 0.375rem;
    animation: pulse 0.6s infinite alternate;
  }

  .trama-chapters {
    margin-left: 1.25rem;
    margin-bottom: 0.25rem;
  }

  .trama-empty-hint {
    font-size: 0.6875rem;
    color: #94a3b8;
    font-style: italic;
    padding: 0.125rem 0;
  }

  :global(.dark) .trama-empty-hint {
    color: #64748b;
  }

  /* ── Trama selector dialog ──────────────────────────────────── */
  .trama-selector-panel {
    max-width: 380px;
  }

  .trama-selector-options {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .trama-selector-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 0.75rem;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    background: #fff;
    color: #475569;
    font-size: 0.875rem;
    cursor: pointer;
    width: 100%;
    text-align: left;
    transition: background 120ms, border-color 120ms;
  }

  .trama-selector-btn:hover {
    background: #f8fafc;
    border-color: #3b82f6;
  }

  :global(.dark) .trama-selector-btn {
    background: #1e293b;
    border-color: #334155;
    color: #94a3b8;
  }
  :global(.dark) .trama-selector-btn:hover {
    background: #334155;
    border-color: #60a5fa;
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

  .date-input-row {
    display: flex;
    gap: 0.375rem;
    align-items: center;
  }
  .date-input-row .field-input {
    flex: 1;
  }
  .date-picker-cal {
    flex-shrink: 0;
    width: 2rem;
    height: 1.9rem;
    padding: 0;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    background: #fff;
    cursor: pointer;
  }
  .date-picker-cal::-webkit-calendar-picker-indicator {
    margin: 0;
    padding: 0.125rem;
  }
  :global(.dark) .date-picker-cal {
    background: #0f172a;
    border-color: #334155;
    color-scheme: dark;
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
    display: flex;
    align-items: center;
    gap: 0.25rem;
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
    flex-direction: column;
    padding: 0.25rem 0;
    font-size: 0.75rem;
    cursor: grab;
  }
  .timeline-event.expanded {
    cursor: default;
  }
  .timeline-event:global(.dragging) { opacity: 0.4; }
  .timeline-event:global(.drag-over) { border-top: 2px solid #3b82f6; }

  .event-row {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    cursor: pointer;
    padding: 0.125rem 0;
  }
  .event-row:hover {
    color: #1e293b;
  }
  :global(.dark) .event-row:hover {
    color: #e2e8f0;
  }

  .event-expand-icon {
    flex-shrink: 0;
    font-size: 0.5625rem;
    color: #94a3b8;
    margin-left: auto;
  }

  .event-moment {
    flex-shrink: 0;
    color: #3b82f6;
    font-weight: 500;
    font-size: 0.6875rem;
    max-width: 10rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.dark) .event-moment {
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

  .event-details {
    padding: 0.375rem 0 0.375rem 0.75rem;
    border-left: 2px solid #cbd5e1;
    margin-top: 0.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  :global(.dark) .event-details {
    border-left-color: #475569;
  }

  .event-description {
    margin: 0;
    color: #475569;
    line-height: 1.5;
    white-space: pre-wrap;
  }
  :global(.dark) .event-description {
    color: #94a3b8;
  }

  .event-meta {
    margin: 0;
    font-size: 0.6875rem;
    color: #64748b;
  }
  :global(.dark) .event-meta {
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

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1.25rem;
  }

  /* ── Font picker ─────────────────────────────────────────────── */
  .config-form-panel {
    max-width: 640px;
  }

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

  .push-now-btn {
    font-size: 0.7rem;
    font-weight: 500;
    color: #059669;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .push-now-btn:hover {
    color: #047857;
  }

  :global(.dark) .push-now-btn {
    color: #34d399;
  }

  :global(.dark) .push-now-btn:hover {
    color: #6ee7b7;
  }

  .pull-now-btn {
    font-size: 0.7rem;
    font-weight: 500;
    color: #7c3aed;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .pull-now-btn:hover {
    color: #6d28d9;
  }

  :global(.dark) .pull-now-btn {
    color: #a78bfa;
  }

  :global(.dark) .pull-now-btn:hover {
    color: #c4b5fd;
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
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
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

  .footer-sep {
    width: 1px;
    height: 1rem;
    background: #e2e8f0;
  }

  :global(.dark) .footer-sep {
    background: #334155;
  }

  /* ── Remote sync warning indicator ──────────────────────────── */
  .remote-warning-icon {
    font-size: 0.8rem;
    margin-left: 0.25rem;
    cursor: default;
  }

  .remote-warning-btn {
    font-size: 0.7rem;
    font-weight: 500;
    color: #d97706;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .remote-warning-btn:hover {
    color: #b45309;
  }

  :global(.dark) .remote-warning-btn {
    color: #fbbf24;
  }

  :global(.dark) .remote-warning-btn:hover {
    color: #f59e0b;
  }

  /* ── Toast notification ─────────────────────────────────────── */
  .toast {
    position: fixed;
    bottom: 1.5rem;
    left: 50%;
    transform: translateX(-50%);
    z-index: 300;
    padding: 0.75rem 1.25rem;
    border-radius: 0.5rem;
    background: #334155;
    color: #f1f5f9;
    font-size: 0.8125rem;
    font-weight: 500;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.25);
    display: flex;
    align-items: center;
    gap: 0.75rem;
    max-width: 90vw;
    animation: fadeIn 200ms ease;
  }

  .toast-error {
    background: #dc2626;
    color: #ffffff;
  }

  .toast-close {
    background: none;
    border: none;
    color: inherit;
    font-size: 1.1rem;
    cursor: pointer;
    padding: 0;
    line-height: 1;
    opacity: 0.7;
  }

  .toast-close:hover {
    opacity: 1;
  }

  .toast-action {
    background: rgba(255, 255, 255, 0.18);
    border: 1px solid rgba(255, 255, 255, 0.3);
    color: inherit;
    padding: 0.3rem 0.7rem;
    border-radius: 0.3rem;
    font-size: 0.8125rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 150ms;
  }

  .toast-action:hover {
    background: rgba(255, 255, 255, 0.3);
  }

  /* ── Character dock panel ───────────────────────────────────── */
  .character-dock {
    position: absolute;
    top: 1rem;
    right: 1rem;
    width: 320px;
    max-height: calc(100% - 2rem);
    background: #ffffff;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    box-shadow: 0 4px 24px rgba(0,0,0,0.12);
    z-index: 50;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  :global(.dark) .character-dock {
    background: #1e293b;
    border-color: #334155;
    box-shadow: 0 4px 24px rgba(0,0,0,0.4);
  }
  .character-dock-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #e2e8f0;
    flex-shrink: 0;
  }
  :global(.dark) .character-dock-header {
    border-bottom-color: #334155;
  }
  .character-dock-header h3 {
    margin: 0;
    font-size: 0.9375rem;
    font-weight: 600;
    color: #1e293b;
  }
  :global(.dark) .character-dock-header h3 {
    color: #f1f5f9;
  }
  .character-dock-close {
    background: none;
    border: none;
    font-size: 1.125rem;
    color: #64748b;
    cursor: pointer;
    padding: 0.25rem;
    line-height: 1;
    border-radius: 4px;
  }
  .character-dock-close:hover { color: #ef4444; }
  :global(.dark) .character-dock-close { color: #94a3b8; }
  :global(.dark) .character-dock-close:hover { color: #f87171; }
  .character-dock-body {
    padding: 1rem;
    overflow-y: auto;
    flex: 1;
  }
  .char-dock-field {
    margin-bottom: 0.75rem;
  }
  .char-dock-label {
    font-size: 0.6875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #94a3b8;
    display: block;
    margin-bottom: 0.25rem;
  }
  .char-dock-field p {
    margin: 0;
    font-size: 0.8125rem;
    line-height: 1.5;
    color: #334155;
  }
  :global(.dark) .char-dock-field p {
    color: #cbd5e1;
  }
  .char-dock-rels {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .char-dock-rels li {
    font-size: 0.8125rem;
    color: #475569;
    padding: 0.25rem 0;
    border-bottom: 1px solid #f1f5f9;
  }
  :global(.dark) .char-dock-rels li {
    color: #94a3b8;
    border-bottom-color: #334155;
  }
</style>
