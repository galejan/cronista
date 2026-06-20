/**
 * Cronista i18n — lightweight Spanish/English translation system.
 *
 * Uses Svelte 5 $state runes for reactivity.
 * In templates, t("key") is reactive because it reads the $state lang.
 *
 * Usage in components:
 *   import { t, setLang, lang } from "$lib/i18n.svelte";
 *   <button onclick={() => setLang("en")}>EN</button>
 *   <p>{t("common.cancel")}</p>
 *   <span class:active={lang === "es"}>ES</span>
 */

export type Lang = "es" | "en";

/** Reactive language state. Mutate .current to trigger re-renders. */
export const lang = $state<{ current: Lang }>({
  current: (typeof localStorage !== "undefined"
    ? (localStorage.getItem("cronista-lang") as Lang | null)
    : null) ?? "es",
});

/** Translate a key to the current language. Reactive in templates. */
export function t(key: string): string {
  return translations[lang.current]?.[key] ?? key;
}

/** Change the active language and persist the choice. */
export function setLang(l: Lang): void {
  lang.current = l;
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
    "toolbar.openProject": "Abrir proyecto",
    "toolbar.newProjectTitle": "Crear un proyecto nuevo (Ctrl+Shift+N)",
    "toolbar.newProject": "Nuevo proyecto",
    "toolbar.closeProjectTitle": "Cerrar proyecto",
    "toolbar.closeProject": "Cerrar",
    "toolbar.collapseFooter": "Colapsar panel de herramientas",
    "toolbar.expandFooter": "Expandir panel de herramientas",

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

    // ── Export ──────────────────────────────────────────────
    "export.export": "Exportar",
    "export.share": "Compartir",
    "export.title": "Exportar proyecto",
    "export.desc":
      "Elegí el formato de exportación. Los archivos se guardan en la carpeta exportaciones/ dentro del proyecto.",
    "export.zipTitle": "Proyecto completo (.zip)",
    "export.zipHint": "Incluye capítulos, personajes, notas, timeline y Git. Ideal para backup.",
    "export.mdTitle": "Solo capítulos (.md)",
    "export.mdHint": "Concatena todos los capítulos en un solo archivo Markdown. Ideal para compartir.",
    "export.zipSuccess": "Proyecto exportado correctamente:",
    "export.mdSuccess": "Capítulos exportados correctamente:",
    "export.error": "Error al exportar:",

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
      "El texto se guarda automáticamente cada 20 segundos. El tipo de letra se elige al crear el proyecto y se aplica a todo el texto. Con Ctrl+↑ y Ctrl+↓ se cambia el nivel de título.",
    "help.chaptersTitle": "📂 Capítulos",
    "help.chaptersDesc":
      "Creá capítulos desde el botón «+ Nuevo capítulo» en la pestaña Capítulos. El nombre del archivo se convierte en el título H1 del editor. Para eliminar, pulsá × y luego confirmá.",
    "help.charactersTitle": "👤 Personajes",
    "help.charactersDesc":
      "Fichas con descripción física, personalidad, traumas y relaciones. Las relaciones pueden ser unilaterales (ej.: A está enamorado de B, pero no al revés).",
    "help.notesTitle": "📝 Notas",
    "help.notesDesc":
      "Ideas, recordatorios y análisis. Al hacer clic en una nota, su contenido se carga en el editor principal.",
    "help.timelineTitle": "⏳ Línea de tiempo",
    "help.timelineDesc":
      "Línea temporal al final del panel lateral. Añadí eventos con fecha, descripción y vinculalos a personajes y capítulos.",
    "help.versioningTitle": "🟢 Versionado",
    "help.versioningDesc":
      "Cronista usa Git para mantener un historial de cambios. Al cerrar la aplicación se crea un checkpoint automático. El indicador en el panel lateral muestra el estado: verde (activo), naranja (sin inicializar), rojo (Git no instalado).",
    "help.exportTitle": "📦 Exportar y compartir",
    "help.exportDesc":
      "Desde el panel de herramientas podés exportar el proyecto completo en .zip (incluye personajes, notas y Git) o compartir solo los capítulos en un archivo .md. Ambos se guardan en la carpeta exportaciones/ dentro del proyecto.",
    "help.dialogDashTitle": "💬 Guion de diálogo",
    "help.dialogDashDesc":
      "Con Ctrl+D se inserta un par de guiones largos (—) y el cursor queda en el medio, listo para escribir el diálogo.",
    "help.shortcutsTitle": "⌨️ Atajos de teclado",
    "help.shortcuts.toggleSidebar": "Colapsar panel lateral",
    "help.shortcuts.restoreSidebar": "Restaurar panel lateral",
    "help.shortcuts.resizeSidebar": "Reducir / ampliar panel lateral (5 % por paso)",
    "help.shortcuts.toggleFooter": "Mostrar / ocultar panel de herramientas",
    "help.shortcuts.zoomIn": "Aumentar tamaño de letra",
    "help.shortcuts.zoomOut": "Reducir tamaño de letra",
    "help.shortcuts.dialogDash": "Insertar guion de diálogo",
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
    "dialog.fontTitle": "Elegí el tipo de letra",
    "dialog.fontDesc":
      "Esta será la fuente para todo el proyecto. Podés cambiarla después desde la configuración.",
    "dialog.fontMono": "Monoespaciada",
    "dialog.fontMonoHint": "Ideal para escritura. Cada letra ocupa lo mismo.",
    "dialog.fontSerif": "Con serifa",
    "dialog.fontSerifHint": "Clásica, elegante. Como un libro impreso.",
    "dialog.fontSans": "Sin serifa",
    "dialog.fontSansHint": "Moderna, limpia. Fácil de leer en pantalla.",
    "dialog.fontConfirm": "Usar esta fuente",
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

    "git.identityTitle": "Configurar identidad Git",
    "git.identityDesc": "Esta identidad se usará en los commits de tus proyectos.",
    "git.nameLabel": "Nombre",
    "git.emailLabel": "Correo electrónico",
    "git.identityContinue": "Continuar",
    "git.identityUseThese": "Usar estos datos",
    "git.remoteTitle": "Sincronización remota",
    "git.remoteCheckbox": "Quiero sincronizar con un repositorio remoto",
    "git.remoteUrlLabel": "URL del repositorio (SSH)",
    "git.remoteInfoBox": "Para usar sincronización remota necesitás:\n1) Crear el repositorio en GitHub, GitLab o Bitbucket.\n2) Configurar una clave SSH en tu sistema.\nCronista solo admite conexiones SSH.",
    "git.remoteSkip": "Saltar",
    "git.remoteFinish": "Finalizar",
    "git.remoteRejectedHttps": "Solo se admiten URLs SSH. Las URLs HTTPS no son compatibles.",
    "git.pushFailed": "No se pudo sincronizar con el remoto.",
    "git.pushDisabled": "Sincronización desactivada tras varios intentos fallidos.",
    "git.toolbarRetry": "Reintentar",
    "git.toolbarReconfigure": "Reconfigurar",
    "git.processing": "Procesando...",

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
    "toolbar.openProject": "Open project",
    "toolbar.newProjectTitle": "Create a new project (Ctrl+Shift+N)",
    "toolbar.newProject": "New project",
    "toolbar.closeProjectTitle": "Close project",
    "toolbar.closeProject": "Close",
    "toolbar.collapseFooter": "Collapse tools panel",
    "toolbar.expandFooter": "Expand tools panel",

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

    // ── Export ──────────────────────────────────────────────
    "export.export": "Export",
    "export.share": "Share",
    "export.title": "Export project",
    "export.desc":
      "Choose export format. Files are saved in the exportaciones/ folder inside the project.",
    "export.zipTitle": "Full project (.zip)",
    "export.zipHint": "Includes chapters, characters, notes, timeline, and Git. Ideal for backup.",
    "export.mdTitle": "Chapters only (.md)",
    "export.mdHint": "Concatenates all chapters into a single Markdown file. Ideal for sharing.",
    "export.zipSuccess": "Project exported successfully:",
    "export.mdSuccess": "Chapters exported successfully:",
    "export.error": "Error exporting:",

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
      "Text is auto-saved every 20 seconds. The font is chosen when creating the project and applied to all text. Use Ctrl+↑ and Ctrl+↓ to change heading levels.",
    "help.chaptersTitle": "📂 Chapters",
    "help.chaptersDesc":
      "Create chapters from the «+ New Chapter» button in the Chapters tab. The filename becomes the H1 title in the editor. To delete, click × then confirm.",
    "help.charactersTitle": "👤 Characters",
    "help.charactersDesc":
      "Character sheets with physical description, personality, traumas, and relationships. Relationships can be one-sided (e.g., A loves B, but not the other way around).",
    "help.notesTitle": "📝 Notes",
    "help.notesDesc":
      "Ideas, reminders, and analysis. Clicking a note loads its content into the main editor.",
    "help.timelineTitle": "⏳ Timeline",
    "help.timelineDesc":
      "Timeline at the bottom of the sidebar. Add events with date, description, and link them to characters and chapters.",
    "help.versioningTitle": "🟢 Versioning",
    "help.versioningDesc":
      "Cronista uses Git to keep a change history. An automatic checkpoint is created when closing the app. The indicator in the sidebar shows status: green (active), orange (not initialized), red (Git not installed).",
    "help.exportTitle": "📦 Export & share",
    "help.exportDesc":
      "From the tools panel you can export the full project as .zip (includes characters, notes, and Git) or share just the chapters as a single .md file. Both are saved in the exportaciones/ folder inside the project.",
    "help.dialogDashTitle": "💬 Dialogue dash",
    "help.dialogDashDesc":
      "Ctrl+D inserts a pair of em dashes (—) with the cursor in the middle, ready to type dialogue.",
    "help.shortcutsTitle": "⌨️ Keyboard Shortcuts",
    "help.shortcuts.toggleSidebar": "Collapse sidebar",
    "help.shortcuts.restoreSidebar": "Restore sidebar",
    "help.shortcuts.resizeSidebar": "Shrink / grow sidebar (5% per step)",
    "help.shortcuts.toggleFooter": "Show / hide tools panel",
    "help.shortcuts.zoomIn": "Increase font size",
    "help.shortcuts.zoomOut": "Decrease font size",
    "help.shortcuts.dialogDash": "Insert dialogue dash",
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
    "dialog.fontTitle": "Choose a font",
    "dialog.fontDesc":
      "This will be the font for the entire project. You can change it later in settings.",
    "dialog.fontMono": "Monospace",
    "dialog.fontMonoHint": "Great for drafting. Every character takes equal space.",
    "dialog.fontSerif": "Serif",
    "dialog.fontSerifHint": "Classic, elegant. Like a printed book.",
    "dialog.fontSans": "Sans-serif",
    "dialog.fontSansHint": "Modern, clean. Easy on screen.",
    "dialog.fontConfirm": "Use this font",
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

    "git.identityTitle": "Configure Git Identity",
    "git.identityDesc": "This identity will be used in your project commits.",
    "git.nameLabel": "Name",
    "git.emailLabel": "Email",
    "git.identityContinue": "Continue",
    "git.identityUseThese": "Use these details",
    "git.remoteTitle": "Remote Sync",
    "git.remoteCheckbox": "I want to sync with a remote repository",
    "git.remoteUrlLabel": "Repository URL (SSH)",
    "git.remoteInfoBox": "To use remote sync you need:\n1) Create the repository on GitHub, GitLab, or Bitbucket.\n2) Configure an SSH key on your system.\nCronista only supports SSH connections.",
    "git.remoteSkip": "Skip",
    "git.remoteFinish": "Finish",
    "git.remoteRejectedHttps": "Only SSH URLs are supported. HTTPS URLs are not compatible.",
    "git.pushFailed": "Could not sync with remote.",
    "git.pushDisabled": "Sync disabled after several failed attempts.",
    "git.toolbarRetry": "Retry",
    "git.toolbarReconfigure": "Reconfigure",
    "git.processing": "Processing...",

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
