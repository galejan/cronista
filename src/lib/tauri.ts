import { invoke } from "@tauri-apps/api/core";

export async function crearProyecto(
  path: string,
  nombre: string,
  fontFamily?: string,
): Promise<string> {
  return invoke("crear_proyecto", { path, nombre, fontFamily });
}

/** Set the project folder icon (best-effort, gvfs). Call after crearProyecto. */
export async function marcarProyectoCronista(
  path: string,
): Promise<void> {
  return invoke("marcar_proyecto_cronista", { path });
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
