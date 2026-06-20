/**
 * Cronista i18n — lightweight Spanish/English translation system.
 *
 * Uses a Svelte writable store (works in plain .ts files).
 * In templates, use the $lang auto-subscription: {$lang === "en" ? "🇬🇧" : "🇪🇸"}
 * In JS functions, use get(lang) to read the current value.
 *
 * Usage in components:
 *   import { t, setLang, lang } from "$lib/i18n";
 *   <button onclick={() => setLang("en")}>🇬🇧</button>
 *   <p>{t("common.cancel")}</p>
 */

import { writable, get } from "svelte/store";

export type Lang = "es" | "en";

/** Reactive language store — use $lang in templates, get(lang) in JS. */
export const lang = writable<Lang>(
  (typeof localStorage !== "undefined"
    ? (localStorage.getItem("cronista-lang") as Lang | null)
    : null) ?? "es",
);

/** Translate a key to the current language. Reactive in templates. */
export function t(key: string): string {
  return translations[get(lang)]?.[key] ?? key;
}

/** Change the active language and persist the choice. */
export function setLang(l: Lang): void {
  lang.set(l);
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("cronista-lang", l);
  }
}

// ── Translations dictionary ──────────────────────────────────

const translations: Record<Lang, Record<string, string>> = {
  es: {
    // ── Setup screen ───────────────────────────────────────
    "setup.selectFolder": "Seleccioná una carpeta de proyecto para comenzar",
    "setup.newProject": "+ Nuevo proyecto",
    "setup.openProject": "Abrir proyecto",
    "setup.reopening": "Reabriendo último proyecto…",

    // ── Toolbar ────────────────────────────────────────────
    "toolbar.newChapter": "+ Nuevo capítulo",
    "toolbar.newChapterTitle": "Nuevo capítulo (Ctrl+N)",
    "toolbar.save": "Guardar",
    "toolbar.saveTitle": "Guardar ahora (Ctrl+S)",
    "toolbar.helpTitle": "Ayuda (F1)",
    "toolbar.darkMode": "Activar tema oscuro",
    "toolbar.lightMode": "Activar tema claro",
    "toolbar.saving": "Guardando…",
    "toolbar.saved": "Guardado",
    "toolbar.unsaved": "Sin guardar",
    "toolbar.openProjectTitle": "Abrir otro proyecto (Ctrl+O)",
    "toolbar.closeProjectTitle": "Cerrar proyecto",

    // ── Git status ──────────────────────────────────────────
    "git.active": "Versionado activo",
    "git.activeTitle": "Git inicializado — los cambios se registran automáticamente",
    "git.notInit": "Versionado no disponible (inicializar)",
    "git.notInitTitle": "Inicializar control de versiones para este proyecto",
    "git.unavailable": "Versionado no disponible (falta Git)",
    "git.unavailableTitle": "Git no encontrado en el sistema",
    "git.defaultName": "Miguel de Cervantes",
    "git.defaultEmail": "manco@lepanto.org",

    "git.initModalTitle": "Inicializar control de versiones",
    "git.initModalDesc":
      "Los datos que se indican a continuación se guardan únicamente en la configuración local del proyecto y nunca son compartidos por Cronista.",
    "git.initModalName": "Nombre para los commits:",
    "git.initModalEmail": "Correo electrónico:",
    "git.initButton": "Inicializar repositorio",
    "git.initSuccess": "Repositorio Git inicializado correctamente. ¡Primer commit creado!",
    "git.initError": "Error al inicializar Git:",

    "git.helpTitle": "Control de versiones con Git",
    "git.helpWhy": "¿Por qué usar versionado?",
    "git.helpWhyDesc":
      "Cada vez que guardás, Cronista crea un punto de control (commit) automático. Esto te permite volver atrás en el tiempo, recuperar versiones anteriores de tu texto y tener un historial completo de tu proceso creativo. Como máquina del tiempo para tu novela.",
    "git.helpInstall": "Instalar Git",
    "git.helpInstallDesc":
      "Git es gratuito y está disponible para Windows, macOS y Linux. Instalalo desde <a href='https://git-scm.com/downloads'>git-scm.com</a>. Una vez instalado, reiniciá Cronista y volvé a abrir tu proyecto.",
    "git.helpClose": "Entendido",
    "git.viewSessions": "Ver últimas sesiones",
    "git.sessionsTitle": "Últimas sesiones",
    "git.sessionsDesc":
      "Cada entrada representa un punto de guardado automático al cerrar la aplicación. Para ver cambios detallados, usá un gestor Git como <a href='https://www.sourcetreeapp.com/'>Sourcetree</a> o <a href='https://desktop.github.com/'>GitHub Desktop</a>.",
    "git.sessionsClose": "Cerrar",
    "git.sessionsEmpty": "No hay sesiones guardadas todavía.",

    // ── Sidebar tabs ───────────────────────────────────────
    "tabs.chapters": "Capítulos",
    "tabs.characters": "Personajes",
    "tabs.notes": "Notas",

    // ── Chapters ───────────────────────────────────────────
    "chapters.label": "Capítulos:",
    "chapters.empty": "Sin capítulos aún.",
    "chapters.load": "Cargar capítulo",
    "chapters.loadPrompt":
      "Nombre del archivo a cargar (ej: 0001_prologo.md):",
    "chapters.confirmDelete": "¿Eliminar?",
    "chapters.confirmDeleteTitle": "Confirmar eliminación",
    "chapters.deleteTitle": "Eliminar capítulo",
    "chapters.newFilePrompt": "Nombre del archivo (ej: 0001_prologo.md):",
    "chapters.untitled": "Sin título",
    "chapters.deleteError": "Error al eliminar capítulo:",
    "chapters.createError": "Error al crear capítulo:",

    // ── Characters ─────────────────────────────────────────
    "characters.empty": "Sin personajes aún.",
    "characters.new": "+ Nuevo personaje",
    "characters.namePlaceholder": "Nombre del personaje",
    "characters.nameRequired": "El nombre del personaje es obligatorio.",
    "characters.create": "Crear",
    "characters.name": "Nombre",
    "characters.physicalDescription": "Descripción física",
    "characters.personality": "Personalidad",
    "characters.traumas": "Traumas",
    "characters.relationships": "Relaciones",
    "characters.addRelationship": "+ Añadir relación",
    "characters.relName": "Nombre",
    "characters.relType": "Tipo (hermano, amigo…)",
    "characters.relNotes": "Notas",
    "characters.save": "Guardar",
    "characters.delete": "Eliminar",
    "characters.deleteConfirm": "¿Eliminar este personaje?",
    "characters.createError": "Error al crear personaje:",
    "characters.saveError": "Error al guardar personaje:",
    "characters.deleteError": "Error al eliminar personaje:",

    // ── Notes ──────────────────────────────────────────────
    "notes.empty": "Sin notas aún.",
    "notes.titleLabel": "Título de la nota",
    "notes.titlePrompt": "Título de la nota:",
    "notes.save": "Guardar",
    "notes.close": "Cerrar",
    "notes.deleteTitle": "Eliminar nota",
    "notes.new": "+ Nueva nota",
    "notes.deleteConfirm": "¿Eliminar esta nota?",
    "notes.createError": "Error al crear nota:",
    "notes.deleteError": "Error al eliminar nota:",

    // ── Timeline ───────────────────────────────────────────
    "timeline.title": "Línea de tiempo",
    "timeline.empty": "Sin eventos en la línea de tiempo.",
    "timeline.deleteTitle": "Eliminar evento",
    "timeline.deleteConfirm": "¿Eliminar este evento?",
    "timeline.date": "Fecha",
    "timeline.eventTitle": "Título",
    "timeline.titlePlaceholder": "¿Qué pasó?",
    "timeline.description": "Descripción",
    "timeline.descriptionPlaceholder": "Detalles del evento…",
    "timeline.relatedCharacters": "Personajes relacionados",
    "timeline.relatedChapters": "Capítulos relacionados",
    "timeline.add": "Agregar",
    "timeline.newEvent": "+ Nuevo evento",
    "timeline.requiredFields": "Fecha y título son obligatorios.",
    "timeline.addError": "Error al agregar evento:",
    "timeline.deleteError": "Error al eliminar evento:",

    // ── Help panel ─────────────────────────────────────────
    "help.ariaLabel": "Ayuda de Cronista",
    "help.createdBy": "creado por",
    "help.editorTitle": "📖 Editor",
    "help.editorDesc":
      "Escribí en la zona central. El texto se guarda automáticamente tras 2 segundos de inactividad. Usá el menú flotante para dar formato al seleccionar texto.",
    "help.chaptersTitle": "📂 Capítulos",
    "help.chaptersDesc":
      "Creá, cargá y eliminá capítulos desde la pestaña Capítulos o con el botón + Nuevo capítulo. Doble clic en × para eliminar con confirmación.",
    "help.charactersTitle": "👤 Personajes",
    "help.charactersDesc":
      "Fichas con descripción física, personalidad, traumas y relaciones. Las relaciones pueden ser unilaterales (ej.: A está enamorado de B, pero no al revés).",
    "help.notesTitle": "📝 Notas",
    "help.notesDesc":
      "Ideas, recordatorios y análisis. Al hacer clic en una nota, su contenido se carga en el editor principal para trabajar con formato.",
    "help.timelineTitle": "⏳ Línea de tiempo",
    "help.timelineDesc":
      "Línea temporal al final del panel lateral. Añadí eventos con fecha, descripción y vinculalos a personajes y capítulos.",
    "help.shortcutsTitle": "⌨️ Atajos de teclado",
    "help.shortcuts.toggleSidebar": "Colapsar / restaurar panel lateral",
    "help.shortcuts.resizeSidebar":
      "Reducir / ampliar panel lateral (5 % por paso)",
    "help.shortcuts.saveNow": "Guardar ahora",
    "help.shortcuts.newChapter": "Nuevo capítulo",
    "help.shortcuts.openProject": "Abrir otro proyecto (cierra el actual)",
    "help.shortcuts.newProject": "Nuevo proyecto (cierra el actual)",
    "help.shortcuts.applyHeading": "Subir / bajar nivel de título (H1 ↔ H2 ↔ normal)",
    "help.shortcuts.fullscreen": "Pantalla completa",
    "help.shortcuts.toggleHelp": "Mostrar / ocultar esta ayuda",

    // ── Dialogs (prompts and confirmations) ────────────────
    "dialog.selectCreateFolder":
      "Seleccioná la carpeta donde crear el proyecto",
    "dialog.projectName": "Nombre del proyecto (ej: Mi Novela):",
    "dialog.projectNameDefault": "Mi Novela",
    "dialog.fontPrompt":
      "Tipo de letra:\n1. Monoespaciada (recomendado)\n2. Con serifa (Serif)\n3. Sin serifa (Sans-serif)\n\nElegí 1, 2 o 3:",
    "dialog.projectFolderPath": "Ruta de la carpeta del proyecto:",
    "dialog.selectProjectFolder": "Seleccioná la carpeta del proyecto",
    "dialog.createProjectError": "Error al crear proyecto:",
    "dialog.openProjectError":
      "No se pudo abrir el proyecto. ¿La carpeta contiene .config/metadata.json?",
    "dialog.filename": "Nombre del archivo:",

    // ── Common ─────────────────────────────────────────────
    "common.error": "Error",
    "common.cancel": "Cancelar",
    "common.delete": "Eliminar",

    // ── Git ────────────────────────────────────────────────
    "git.notInstalled": "Git no está instalado",
    "git.notInstalledDesc": "El control de versiones automático no estará disponible.",
    "git.installInstructions": "Instalar Git:\n• Linux: sudo pacman -S git (Arch) / sudo apt install git (Debian/Ubuntu)\n• Windows: https://git-scm.com/download/win\n• macOS: brew install git",
    "git.continueWithout": "¿Continuar sin Git?",
    "git.autoCommit": "Cierre de sesión",

    // ── Formatting toolbar (Editor.svelte) ──────────────────
    "editor.heading1": "Título 1 (Ctrl+Alt+1)",
    "editor.heading2": "Título 2 (Ctrl+Alt+2)",
    "editor.heading3": "Título 3 (Ctrl+Alt+3)",
    "editor.paragraph": "Párrafo",
    "editor.fontFamily": "Tipografía",
    "editor.fontDefault": "Por defecto",
    "editor.fontSerif": "Serif",
    "editor.fontSans": "Sans-serif",
    "editor.fontMono": "Monospace",
  },

  en: {
    // ── Setup screen ───────────────────────────────────────
    "setup.selectFolder": "Select a project folder to get started",
    "setup.newProject": "+ New Project",
    "setup.openProject": "Open Project",
    "setup.reopening": "Reopening last project…",

    // ── Toolbar ────────────────────────────────────────────
    "toolbar.newChapter": "+ New Chapter",
    "toolbar.newChapterTitle": "New chapter (Ctrl+N)",
    "toolbar.save": "Save",
    "toolbar.saveTitle": "Save now (Ctrl+S)",
    "toolbar.helpTitle": "Help (F1)",
    "toolbar.darkMode": "Enable dark theme",
    "toolbar.lightMode": "Enable light theme",
    "toolbar.saving": "Saving…",
    "toolbar.saved": "Saved",
    "toolbar.unsaved": "Unsaved",
    "toolbar.openProjectTitle": "Open another project (Ctrl+O)",
    "toolbar.closeProjectTitle": "Close project",

    // ── Git status ──────────────────────────────────────────
    "git.active": "Versioning active",
    "git.activeTitle": "Git initialized — changes are tracked automatically",
    "git.notInit": "Versioning unavailable (initialize)",
    "git.notInitTitle": "Initialize version control for this project",
    "git.unavailable": "Versioning unavailable (Git missing)",
    "git.unavailableTitle": "Git not found on this system",
    "git.defaultName": "William Shakespeare",
    "git.defaultEmail": "bard@avon.org",

    "git.initModalTitle": "Initialize version control",
    "git.initModalDesc":
      "The name and email below are stored only in the local project configuration and are never shared by Cronista.",
    "git.initModalName": "Name for commits:",
    "git.initModalEmail": "Email:",
    "git.initButton": "Initialize repository",
    "git.initSuccess": "Git repository initialized. First commit created!",
    "git.initError": "Error initializing Git:",

    "git.helpTitle": "Version control with Git",
    "git.helpWhy": "Why use version control?",
    "git.helpWhyDesc":
      "Every time you save, Cronista creates an automatic checkpoint (commit). This lets you travel back in time, recover previous versions of your text, and keep a complete history of your creative process. Like a time machine for your novel.",
    "git.helpInstall": "Install Git",
    "git.helpInstallDesc":
      "Git is free and available for Windows, macOS, and Linux. Install it from <a href='https://git-scm.com/downloads'>git-scm.com</a>. Once installed, restart Cronista and reopen your project.",
    "git.helpClose": "Got it",
    "git.viewSessions": "View recent sessions",
    "git.sessionsTitle": "Recent sessions",
    "git.sessionsDesc":
      "Each entry is an automatic save point created when closing the app. For detailed diffs, use a Git client like <a href='https://www.sourcetreeapp.com/'>Sourcetree</a> or <a href='https://desktop.github.com/'>GitHub Desktop</a>.",
    "git.sessionsClose": "Close",
    "git.sessionsEmpty": "No saved sessions yet.",

    // ── Sidebar tabs ───────────────────────────────────────
    "tabs.chapters": "Chapters",
    "tabs.characters": "Characters",
    "tabs.notes": "Notes",

    // ── Chapters ───────────────────────────────────────────
    "chapters.label": "Chapters:",
    "chapters.empty": "No chapters yet.",
    "chapters.load": "Load chapter",
    "chapters.loadPrompt": "Filename to load (e.g. 0001_prologue.md):",
    "chapters.confirmDelete": "Delete?",
    "chapters.confirmDeleteTitle": "Confirm deletion",
    "chapters.deleteTitle": "Delete chapter",
    "chapters.newFilePrompt": "Filename (e.g. 0001_prologue.md):",
    "chapters.untitled": "Untitled",
    "chapters.deleteError": "Error deleting chapter:",
    "chapters.createError": "Error creating chapter:",

    // ── Characters ─────────────────────────────────────────
    "characters.empty": "No characters yet.",
    "characters.new": "+ New Character",
    "characters.namePlaceholder": "Character name",
    "characters.nameRequired": "Character name is required.",
    "characters.create": "Create",
    "characters.name": "Name",
    "characters.physicalDescription": "Physical description",
    "characters.personality": "Personality",
    "characters.traumas": "Traumas",
    "characters.relationships": "Relationships",
    "characters.addRelationship": "+ Add relationship",
    "characters.relName": "Name",
    "characters.relType": "Type (brother, friend…)",
    "characters.relNotes": "Notes",
    "characters.save": "Save",
    "characters.delete": "Delete",
    "characters.deleteConfirm": "Delete this character?",
    "characters.createError": "Error creating character:",
    "characters.saveError": "Error saving character:",
    "characters.deleteError": "Error deleting character:",

    // ── Notes ──────────────────────────────────────────────
    "notes.empty": "No notes yet.",
    "notes.titleLabel": "Note title",
    "notes.titlePrompt": "Note title:",
    "notes.save": "Save",
    "notes.close": "Close",
    "notes.deleteTitle": "Delete note",
    "notes.new": "+ New Note",
    "notes.deleteConfirm": "Delete this note?",
    "notes.createError": "Error creating note:",
    "notes.deleteError": "Error deleting note:",

    // ── Timeline ───────────────────────────────────────────
    "timeline.title": "Timeline",
    "timeline.empty": "No events in the timeline.",
    "timeline.deleteTitle": "Delete event",
    "timeline.deleteConfirm": "Delete this event?",
    "timeline.date": "Date",
    "timeline.eventTitle": "Title",
    "timeline.titlePlaceholder": "What happened?",
    "timeline.description": "Description",
    "timeline.descriptionPlaceholder": "Event details…",
    "timeline.relatedCharacters": "Related characters",
    "timeline.relatedChapters": "Related chapters",
    "timeline.add": "Add",
    "timeline.newEvent": "+ New Event",
    "timeline.requiredFields": "Date and title are required.",
    "timeline.addError": "Error adding event:",
    "timeline.deleteError": "Error deleting event:",

    // ── Help panel ─────────────────────────────────────────
    "help.ariaLabel": "Cronista Help",
    "help.createdBy": "created by",
    "help.editorTitle": "📖 Editor",
    "help.editorDesc":
      "Write in the central area. Text is auto-saved after 2 seconds of inactivity. Use the floating menu to format selected text.",
    "help.chaptersTitle": "📂 Chapters",
    "help.chaptersDesc":
      "Create, load, and delete chapters from the Chapters tab or using the + New Chapter button. Double-click × to delete with confirmation.",
    "help.charactersTitle": "👤 Characters",
    "help.charactersDesc":
      "Character sheets with physical description, personality, traumas, and relationships. Relationships can be one-sided (e.g., A loves B, but not the other way around).",
    "help.notesTitle": "📝 Notes",
    "help.notesDesc":
      "Ideas, reminders, and analysis. Clicking a note loads its content into the main editor so you can work with formatting.",
    "help.timelineTitle": "⏳ Timeline",
    "help.timelineDesc":
      "Timeline at the bottom of the sidebar. Add events with date, description, and link them to characters and chapters.",
    "help.shortcutsTitle": "⌨️ Keyboard Shortcuts",
    "help.shortcuts.toggleSidebar": "Toggle sidebar collapse",
    "help.shortcuts.resizeSidebar": "Shrink / grow sidebar (5% per step)",
    "help.shortcuts.saveNow": "Save now",
    "help.shortcuts.newChapter": "New chapter",
    "help.shortcuts.openProject": "Open another project (closes current)",
    "help.shortcuts.newProject": "New project (closes current)",
    "help.shortcuts.applyHeading": "Increase / decrease heading level (H1 ↔ H2 ↔ normal)",
    "help.shortcuts.fullscreen": "Full screen",
    "help.shortcuts.toggleHelp": "Show / hide this help",

    // ── Dialogs (prompts and confirmations) ────────────────
    "dialog.selectCreateFolder":
      "Select the folder where to create the project",
    "dialog.projectName": "Project name (e.g. My Novel):",
    "dialog.projectNameDefault": "My Novel",
    "dialog.fontPrompt":
      "Font:\n1. Monospace (recommended)\n2. Serif\n3. Sans-serif\n\nChoose 1, 2, or 3:",
    "dialog.projectFolderPath": "Project folder path:",
    "dialog.selectProjectFolder": "Select the project folder",
    "dialog.createProjectError": "Error creating project:",
    "dialog.openProjectError":
      "Could not open the project. Does the folder contain .config/metadata.json?",
    "dialog.filename": "Filename:",

    // ── Common ─────────────────────────────────────────────
    "common.error": "Error",
    "common.cancel": "Cancel",
    "common.delete": "Delete",

    // ── Git ────────────────────────────────────────────────
    "git.notInstalled": "Git is not installed",
    "git.notInstalledDesc": "Automatic version control will not be available.",
    "git.installInstructions": "Install Git:\n• Linux: sudo pacman -S git (Arch) / sudo apt install git (Debian/Ubuntu)\n• Windows: https://git-scm.com/download/win\n• macOS: brew install git",
    "git.continueWithout": "Continue without Git?",
    "git.autoCommit": "Session closed",

    // ── Formatting toolbar (Editor.svelte) ──────────────────
    "editor.heading1": "Heading 1 (Ctrl+Alt+1)",
    "editor.heading2": "Heading 2 (Ctrl+Alt+2)",
    "editor.heading3": "Heading 3 (Ctrl+Alt+3)",
    "editor.paragraph": "Paragraph",
    "editor.fontFamily": "Font family",
    "editor.fontDefault": "Default",
    "editor.fontSerif": "Serif",
    "editor.fontSans": "Sans-serif",
    "editor.fontMono": "Monospace",
  },
};

/**
 * Translate a key to the current language.
 *
 * Usage: `t("common.cancel")` → "Cancelar" (es) or "Cancel" (en)
 *
 * Falls back to the key itself if translation is missing.
 */
