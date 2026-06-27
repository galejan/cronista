Instrucciones de Refactorización: Temas y Alineación de InterfazEste documento contiene las especificaciones técnicas necesarias para implementar un sistema de 4 temas dinámicos y corregir un defecto de desalineación en la cabecera del editor y el sidebar de pestañas de Cron-Insta.Parte 1: Corrección de la Desalineación SuperiorEl sidebar de pestañas (izquierda) y la barra de título del documento "fragmentos" (derecha) están visualmente desalineados debido a discrepancias en alturas de línea, paddings internos y falta de un marco de altura común rígido.Modificaciones en el Layout principal (+page.svelte o equivalente)Aplica las siguientes reglas CSS para forzar la consistencia estructural. Se recomienda definir una variable de altura de cabecera común para evitar desfases futuros::root {
  --header-height: 48px; /* Altura rígida unificada */
}

/* 1. Contenedor de pestañas (Sidebar Izquierdo) */
.tabs-navbar {
  height: var(--header-height);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center; /* Centrado vertical de iconos SVG */
  justify-content: space-around;
  padding: 0 8px;
}

/* Forzar que los items de pestaña ocupen el 100% de la altura de la caja */
.tab-item {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  flex: 1;
  cursor: pointer;
  border-bottom: 2px solid transparent;
}

/* 2. Cabecera del Editor (Columna Derecha) */
.editor-header {
  height: var(--header-height);
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-app); /* Coherencia de color de barra con el fondo general */
  display: flex;
  align-items: center; /* Alineación al píxel con el contenedor de la izquierda */
  justify-content: space-between;
  padding: 0 24px;
}
Parte 2: Refactorización y Limpieza de app.css para Multitema⚠️ ALERTA CRÍTICA DE REFACTORIZACIÓN:En el app.css original, .ProseMirror tiene el color de fuente fijo en #1a1a1a y depende de una regla .dark .ProseMirror para el modo oscuro. Esto debe eliminarse. Si no se elimina, el color negro quedará hardcodeado e impedirá que los modos claros (como el Sepia) apliquen su respectiva paleta.Sustituye la configuración base y añade los selectores de atributo en tu CSS global:/* ── Configuración de Temas mediante Variables CSS ── */

/* 1. OSCURO NÓRDICO (Por defecto) */
html, html[data-theme="dark-nordic"] {
  --bg-app: #0f172a;
  --bg-editor: #0c1424;
  --bg-sidebar: #0f172a;
  --bg-active-tab: #1e293b;
  --text-main: #e2e8f0;
  --text-title: #f1f5f9;
  --text-muted: #64748b;
  --border-color: #1e293b;
  --accent: #3b82f6;
  --scrollbar-thumb: #334155;
}

/* 2. OSCURO AMATISTA (Morado Inmersivo) */
html[data-theme="dark-amethyst"] {
  --bg-app: #130f1c;
  --bg-editor: #0e0a15;
  --bg-sidebar: #130f1c;
  --bg-active-tab: #1d172a;
  --text-main: #e4def2;
  --text-title: #f3effc;
  --text-muted: #796f8a;
  --border-color: #241c33;
  --accent: #a855f7;
  --scrollbar-thumb: #3b2d54;
}

/* 3. CLARO NÓRDICO (Técnico / Limpio) */
html[data-theme="light-nordic"] {
  --bg-app: #e2e8f0;
  --bg-editor: #f1f5f9;
  --bg-sidebar: #e2e8f0;
  --bg-active-tab: #cbd5e1;
  --text-main: #1e293b;
  --text-title: #0f172a;
  --text-muted: #64748b;
  --border-color: #cbd5e1;
  --accent: #3b82f6;
  --scrollbar-thumb: #94a3b8;
}

/* 4. CLARO SEPIA (Cálido / Tradicional) */
html[data-theme="light-sepia"] {
  --bg-app: #f3efe9;
  --bg-editor: #faf6ee;
  --bg-sidebar: #f3efe9;
  --bg-active-tab: #e7dfd5;
  --text-main: #2d2824;
  --text-title: #1a1614;
  --text-muted: #7d7268;
  --border-color: #dcd5cb;
  --accent: #b45309;
  --scrollbar-thumb: #c7bdae;
}

/* ── Aplicación Semántica (Base) ── */

html {
  background: var(--bg-app);
  color: var(--text-main);
  transition: background 0.2s ease, color 0.2s ease, border-color 0.2s ease;
}

/* ── Corrección de Tipografía en ProseMirror ── */

.ProseMirror {
  font-family: Georgia, "Times New Roman", serif;
  font-size: 1.125rem;
  line-height: 1.8;
  /* El editor hereda dinámicamente del tema activo */
  background: var(--bg-editor);
  color: var(--text-main); 
  max-width: 80ch;
  margin: 0 auto;
  padding: 2rem 2rem;
  outline: none;
  word-wrap: break-word;
  white-space: pre-wrap;
}

/* Eliminar colores fijos de encabezados y asignar variables */
.ProseMirror h1,
.ProseMirror h2,
.ProseMirror h3 {
  font-family: Georgia, "Times New Roman", serif;
  color: var(--text-title);
  font-weight: 700;
}

/* ELIMINAR COMPLETAMENTE: Las clases de soporte antiguas estilo ".dark .ProseMirror" 
   para evitar colisiones de especificidad CSS */
Parte 3: Integración del Estado en Svelte 5Dado que utilizas un sistema de traducción reactivo con runas de Svelte 5 ($state), debes seguir el mismo patrón para propagar el tema seleccionado a la raíz de la aplicación.1. Gestión del Estado (theme.svelte.ts o equivalente)Crea o añade al archivo de configuración de tu proyecto el estado reactivo del tema:// Define los tipos de tema soportados
export type AppTheme = 'dark-nordic' | 'dark-amethyst' | 'light-nordic' | 'light-sepia';

class ThemeManager {
  current = $state<AppTheme>('dark-nordic');

  constructor() {
    // Recuperar el tema de la configuración local al inicializar
    const savedTheme = localStorage.getItem('cron-insta-theme') as AppTheme;
    if (savedTheme) {
      this.setTheme(savedTheme);
    }
  }

  setTheme(theme: AppTheme) {
    this.current = theme;
    localStorage.setItem('cron-insta-theme', theme);
    // Aplicar el cambio de atributo de manera global en el DOM
    document.documentElement.setAttribute('data-theme', theme);
  }
}

export const themeManager = new ThemeManager();
2. Actualización de la UI de Configuración (Configuracion.svelte)En la pestaña de configuración de la interfaz, el componente de selección simplemente interactúa con la propiedad themeManager.current:<script lang="ts">
  import { themeManager, type AppTheme } from '../lib/theme.svelte';
</script>

<div class="config-tabs">
  <label for="theme-select">Aspecto visual:</label>
  <select 
    id="theme-select" 
    value={themeManager.current} 
    onchange={(e) => themeManager.setTheme(e.currentTarget.value as AppTheme)}
  >
    <option value="dark-nordic">Oscuro Nórdico (Original)</option>
    <option value="dark-amethyst">Oscuro Amatista</option>
    <option value="light-nordic">Claro Nórdico</option>
    <option value="light-sepia">Claro Sepia</option>
  </select>
</div>
