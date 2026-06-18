import { invoke } from "@tauri-apps/api/core";

export async function crearProyecto(
  path: string,
  nombre: string,
): Promise<string> {
  return invoke("crear_proyecto", { path, nombre });
}

export async function inicializarGit(path: string): Promise<string> {
  return invoke("inicializar_git", { path });
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
