import { invoke } from "@tauri-apps/api/core";

export async function crearProyecto(
  path: string,
  nombre: string,
  fontFamily?: string,
): Promise<string> {
  return invoke("crear_proyecto", { path, nombre, fontFamily });
}

/** Set the project folder icon (best-effort, gvfs). Call after crearProyecto. */
export async function marcarProyectoCronInsta(
  path: string,
): Promise<void> {
  return invoke("marcar_proyecto_cron_insta", { path });
}

export async function exportarProyectoZip(
  proyectoPath: string,
): Promise<string> {
  return invoke("exportar_proyecto_zip", { proyectoPath });
}

export async function exportarProyectoMd(
  proyectoPath: string,
): Promise<string> {
  return invoke("exportar_proyecto_md", { proyectoPath });
}

export async function inicializarGit(path: string): Promise<string> {
  return invoke("inicializar_git", { path });
}

export async function inicializarGitConAutor(
  path: string,
  nombre: string,
  email: string,
): Promise<string> {
  return invoke("inicializar_git_con_autor", { path, nombre, email });
}

export async function verificarGitInicializado(
  path: string,
): Promise<boolean> {
  return invoke("verificar_git_inicializado", { path });
}

export interface GitLogEntry {
  hash: string;
  date: string;
  message: string;
  words: string;
  files: string[];
}

export async function obtenerGitLog(
  path: string,
  limit: number,
): Promise<GitLogEntry[]> {
  const raw: string = await invoke("obtener_git_log", { path, limit });
  return JSON.parse(raw);
}

export async function detectarGit(): Promise<boolean> {
  return invoke("detectar_git");
}

export async function guardarCapitulo(
  proyectoPath: string,
  filename: string,
  contenido: string,
): Promise<string> {
  return invoke("guardar_capitulo", { proyectoPath, filename, contenido });
}

export async function crearCheckpoint(
  proyectoPath: string,
): Promise<string> {
  return invoke("crear_checkpoint", { proyectoPath });
}

/** Tell the Rust backend which project is open (for close-time checkpoint). */
export async function setActiveProject(
  path: string | null,
): Promise<void> {
  return invoke("set_active_project", { path });
}

export async function cargarIndice(proyectoPath: string): Promise<string> {
  return invoke("cargar_indice", { proyectoPath });
}

export async function cargarCapitulo(
  proyectoPath: string,
  filename: string,
): Promise<string> {
  return invoke("cargar_capitulo", { proyectoPath, filename });
}

export async function crearCapitulo(
  proyectoPath: string,
  filename: string,
  contenido: string,
): Promise<string> {
  return invoke("crear_capitulo", { proyectoPath, filename, contenido });
}

export async function eliminarCapitulo(
  proyectoPath: string,
  filename: string,
): Promise<string> {
  return invoke("eliminar_capitulo", { proyectoPath, filename });
}

// ── Characters ────────────────────────────────────────────────

export async function listarPersonajes(
  proyectoPath: string,
): Promise<string> {
  return invoke("listar_personajes", { proyectoPath });
}

export async function crearPersonaje(
  proyectoPath: string,
  personajeJson: string,
): Promise<string> {
  return invoke("crear_personaje", { proyectoPath, personajeJson });
}

export async function cargarPersonaje(
  proyectoPath: string,
  id: string,
): Promise<string> {
  return invoke("cargar_personaje", { proyectoPath, id });
}

export async function actualizarPersonaje(
  proyectoPath: string,
  id: string,
  personajeJson: string,
): Promise<string> {
  return invoke("actualizar_personaje", {
    proyectoPath,
    id,
    personajeJson,
  });
}

export async function eliminarPersonaje(
  proyectoPath: string,
  id: string,
): Promise<string> {
  return invoke("eliminar_personaje", { proyectoPath, id });
}

// ── Notes ─────────────────────────────────────────────────────

export async function listarNotas(
  proyectoPath: string,
): Promise<string> {
  return invoke("listar_notas", { proyectoPath });
}

export async function crearNota(
  proyectoPath: string,
  id: string,
  titulo: string,
  contenido: string,
): Promise<string> {
  return invoke("crear_nota", { proyectoPath, id, titulo, contenido });
}

export async function cargarNota(
  proyectoPath: string,
  id: string,
): Promise<string> {
  return invoke("cargar_nota", { proyectoPath, id });
}

export async function eliminarNota(
  proyectoPath: string,
  id: string,
): Promise<string> {
  return invoke("eliminar_nota", { proyectoPath, id });
}

// ── Timeline ──────────────────────────────────────────────────

export async function cargarTimeline(
  proyectoPath: string,
): Promise<string> {
  return invoke("cargar_timeline", { proyectoPath });
}

export async function agregarEventoTimeline(
  proyectoPath: string,
  eventoJson: string,
): Promise<string> {
  return invoke("agregar_evento_timeline", { proyectoPath, eventoJson });
}

export async function actualizarEventoTimeline(
  proyectoPath: string,
  eventoJson: string,
): Promise<string> {
  return invoke("actualizar_evento_timeline", { proyectoPath, eventoJson });
}

export async function eliminarEventoTimeline(
  proyectoPath: string,
  id: string,
): Promise<string> {
  return invoke("eliminar_evento_timeline", { proyectoPath, id });
}

export async function reordenarTimeline(
  proyectoPath: string,
  ids: string[],
): Promise<string> {
  return invoke("reordenar_timeline", { proyectoPath, idsJson: JSON.stringify(ids) });
}

// ── Places ────────────────────────────────────────────────────

export async function listarLugares(
  proyectoPath: string,
): Promise<string> {
  return invoke("listar_lugares", { proyectoPath });
}

export async function crearLugar(
  proyectoPath: string,
  lugarJson: string,
): Promise<string> {
  return invoke("crear_lugar", { proyectoPath, lugarJson });
}

export async function cargarLugar(
  proyectoPath: string,
  id: string,
): Promise<string> {
  return invoke("cargar_lugar", { proyectoPath, id });
}

export async function actualizarLugar(
  proyectoPath: string,
  id: string,
  lugarJson: string,
): Promise<string> {
  return invoke("actualizar_lugar", {
    proyectoPath,
    id,
    lugarJson,
  });
}

export async function eliminarLugar(
  proyectoPath: string,
  id: string,
): Promise<string> {
  return invoke("eliminar_lugar", { proyectoPath, id });
}

// ── Font ──────────────────────────────────────────────────────

export async function actualizarFuenteProyecto(
  projectPath: string,
  fontFamily: string,
): Promise<string> {
  return invoke("actualizar_fuente_proyecto", { projectPath, fontFamily });
}

// ── Git Identity & Remote ─────────────────────────────────────

export async function cargarIdentidadGit(): Promise<{name: string, email: string, github_user?: string} | null> {
  const result = await invoke<string>("cargar_identidad_git");
  if (result === "null") return null;
  return JSON.parse(result);
}

export async function guardarIdentidadGit(name: string, email: string, githubUser?: string): Promise<string> {
  return invoke("guardar_identidad_git", { name, email, githubUser });
}

export async function cargarConfigRemoto(projectPath: string): Promise<{push_enabled: boolean, consecutive_failures: number, url: string | null} | null> {
  const result = await invoke<string>("cargar_config_remoto", { proyectoPath: projectPath });
  if (result === "null") return null;
  return JSON.parse(result);
}

export async function guardarConfigRemoto(projectPath: string, url: string, pushEnabled: boolean): Promise<string> {
  return invoke("guardar_config_remoto", { proyectoPath: projectPath, url, pushEnabled });
}

export async function configurarRemoto(path: string, url: string): Promise<string> {
  return invoke("configurar_remoto", { path, url });
}

export async function reintentarPush(path: string): Promise<string> {
  return invoke("reintentar_push", { path });
}

export async function sincronizarRemoto(path: string): Promise<string> {
  return invoke("sincronizar_remoto", { path });
}

export async function importarProyecto(zipPath: string, destino: string): Promise<string> {
  return invoke("importar_proyecto", { zipPath, destino });
}

export async function eliminarDirectorioGit(path: string): Promise<void> {
  return invoke("eliminar_directorio_git", { path });
}
