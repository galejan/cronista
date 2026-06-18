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
