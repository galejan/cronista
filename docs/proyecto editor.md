DOCUMENTO DE DISEÑO DE SOFTWARE (SDD) & GUÍA DE EJECUCIÓN

Proyecto: Editor de Texto Literario Minimalista (Local-First)

PARTE 1: DOCUMENTO DE DISEÑO DE SOFTWARE (SDD)

1. Control de Versiones del Documento

Versión

Fecha

Descripción

Autor

v1.1.0

17/06/2026

Refinamiento de esquemas: IDs, relaciones estructuradas, guardado en dos niveles, testing y dependencias de sistema.

Arquitecto de Software

2. Visión General del Sistema

Este sistema es un editor de texto enriquecido diseñado específicamente para la escritura de novelas largas. Se rige por los principios de minimalismo visual, privacidad local (Local-First) y automatización de la estructura.

A diferencia de editores genéricos (como Obsidian), el usuario experimenta un entorno cerrado e integrado donde los capítulos, las fichas de personajes, las notas y la línea de tiempo coexisten sin requerir la gestión manual de archivos o enlaces de forma directa.

Objetivos Clave:

Zona de escritura sagrada: Al menos el 60% del espacio de la pantalla está libre de distracciones.

Barra lateral integrada (40%): Control jerárquico de capítulos, fichas y línea de tiempo mediante drag-and-drop.

Almacenamiento transparente: Archivos .md puros para el texto y .json para índices y metadatos rápidos.

Historial indestructible: Sistema Git embebido e invisible para el usuario bajo nomenclatura literaria.

3. Decisión de Arquitectura y Compilación

Para cumplir con el requisito de compilar para Windows y Linux nativamente, se selecciona Tauri + Rust + Frontend Web (Svelte/React).

+-------------------------------------------------------------+
|                     INTERFAZ (Frontend)                      |
|          Svelte / React + Tailwind CSS + TipTap Editor      |
+-------------------------------------------------------------+
                             |  IPC (Inter-Process Comm)
                             v
+-------------------------------------------------------------+
|                      NÚCLEO (Backend)                       |
|           Tauri (Rust) - Gestión de Disco y Git             |
+-------------------------------------------------------------+
                             |  Acceso Nativo
                             v
+-------------------------------------------------------------+
|                     SISTEMA DE ARCHIVOS                     |
|           Carpeta del Proyecto (.md, .json, .git)           |
+-------------------------------------------------------------+


Justificación del Stack técnico:

Seguridad y Empaquetado: Tauri genera un único ejecutable nativo (.exe / .deb / .AppImage) de menos de 15MB, evitando que el usuario deba instalar entornos como Node.js en su ordenador.

Rendimiento en Disco: Rust maneja la concurrencia y la sincronización con Git de forma ultraeficiente, previniendo corrupción de datos en archivos extensos.

Portabilidad futura: El frontend web desarrollado puede reconvertirse en una aplicación web tradicional o servidor local si las necesidades del proyecto cambian.

4. Estructura del Almacenamiento (Esquema de Directorios)

Cada "Proyecto" es una carpeta física en el disco del usuario con la siguiente estructura interna automatizada:

[Nombre del Proyecto]/
├── .config/
│   ├── metadata.json       <-- Índice global, orden de capítulos y enlaces de personajes.
│   └── timeline.json       <-- Array ordenado de eventos cronológicos.
├── .git/                   <-- Repositorio oculto para control de versiones invisible.
├── capitulos/
│   ├── 0001_prologo.md     <-- Contenido textual indexado numéricamente.
│   └── 0002_capitulo_1.md
├── personajes/
│   ├── p_elena_vance.md    <-- Ficha de personaje con Frontmatter.
│   └── p_john_doe.md
└── notas/
    └── n_ideas_final.md    <-- Notas libres del autor.


Convención de Nombrado e Identificadores (IDs)

Para garantizar la integridad referencial entre metadata.json y timeline.json, se define el siguiente esquema de IDs:

- Capítulos: El ID de un capítulo es su nombre de archivo sin extensión (ej: "0001_prologo"). El prefijo numérico es auto-incremental y se reasigna al reordenar mediante drag-and-drop en la barra lateral. Los números no se reutilizan tras eliminación — los nuevos capítulos toman el siguiente número disponible en la secuencia para evitar ambigüedad en el historial de Git.

- Personajes: El ID es el valor del campo `id` en el frontmatter del archivo .md del personaje, que debe coincidir con el nombre del archivo sin el prefijo `p_` (ej: archivo `p_elena_vance.md` → id `"elena_vance"`).

- Eventos de timeline: Cada evento en timeline.json tiene un `id` único generado como UUID v4 al momento de su creación. El campo `capitulo_asociado_id` referencia el ID del capítulo (nombre de archivo sin extensión). Si un capítulo se renombra, el sistema debe actualizar todas las referencias en timeline.json automáticamente.

- Relaciones entre personajes: El campo `character_id` dentro de `relations` en el frontmatter referencia el ID de otro personaje. Esto permite construir un grafo de relaciones consultable desde la UI.

Esta convención asegura que el timeline sobreviva a reorganizaciones de capítulos y que las relaciones entre personajes sean trazables.

Especificación de Esquemas de Datos

A. .config/metadata.json

{
  "project_name": "Mi Novela",
  "last_modified": "2026-06-17T21:00:00Z",
  "chapters_order": [
    "0001_prologo.md",
    "0002_capitulo_1.md"
  ],
  "characters_index": [
    {"id": "elena_vance", "file": "p_elena_vance.md", "name": "Elena Vance"}
  ]
}


B. personajes/p_elena_vance.md (Estructura con Frontmatter)

---
id: "elena_vance"
name: "Elena Vance"
role: "Protagonista"
age: 34
relations:
  - character_id: "john_doe"
    type: "Enemigo"
---
Aquí comienza la descripción libre, rasgos físicos y trasfondo psicológico que el usuario edita de forma abierta...

El campo `relations` usa un array de objetos estructurados (en lugar de strings planos como `"john_doe:Enemigo"`) para permitir consultas por tipo de relación y construir un grafo de personajes consultable desde la interfaz.

C. .config/timeline.json

[
  {
    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "titulo": "Elena descubre la carta",
    "descripcion_corta": "En el ático de la casa familiar, Elena encuentra una carta sellada de su padre.",
    "capitulo_asociado_id": "0002_capitulo_1",
    "orden": 0
  },
  {
    "id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
    "titulo": "Primer encuentro con John",
    "descripcion_corta": "John intercepta a Elena en la biblioteca municipal.",
    "capitulo_asociado_id": "0003_capitulo_2",
    "orden": 1
  }
]

Nota: El campo `capitulo_asociado_id` referencia el ID del capítulo (nombre de archivo sin extensión). Si el evento no está asociado a ningún capítulo, el campo puede ser `null`. El array se ordena por el campo `orden`, que se recalcula automáticamente al reordenar mediante drag-and-drop.


5. Diseño de Interfaz y Módulos del Sistema

La interfaz se divide estrictamente en proporción estática 40% Lateral (Modulable) / 60% Central (Editor). El panel del 40% puede ocultarse por completo mediante el atajo de teclado global Ctrl + B.

Módulo A: El Editor Central (60%)

Motor: Basado en TipTap / ProseMirror.

Comportamiento: WYSIWYG minimalista (Texto enriquecido basado en Markdown). No muestra barras de herramientas fijas. Al seleccionar texto, emerge un menú contextual flotante para negritas, cursivas o enlaces.

Rendimiento: Solo carga un capítulo a la vez (Lazy Loading). Al cambiar de capítulo en la barra lateral, se guarda el estado actual y se monta el JSON parsed del nuevo capítulo.

Módulo B: Barra Lateral Multifunción (40%)

Funciona mediante pestañas verticales o un sistema de acordeón colapsable:

Navegador de Capítulos: Árbol visual que lee metadata.json. Permite arrastrar y soltar elementos para reordenar los capítulos físicos (reordenando el array en el JSON).

Gestor de Personajes y Notas: Al hacer clic en un personaje, su ficha .md se abre en un panel secundario dentro de la misma barra lateral. El usuario puede consultar datos de su personaje sin perder el foco del 60% central.

Módulo C: Línea de Tiempo (Timeline Vertical Intercalable)

Mecánica: Lista vertical de tarjetas que representa eventos de la historia.

Intercalación: Permite arrastrar una tarjeta de evento y soltarla entre dos eventos existentes.

Persistencia: La acción altera únicamente el orden de índices en .config/timeline.json. Cada tarjeta puede enlazarse a un ID de capítulo concreto mediante metadatos para permitir saltos rápidos.

6. Mecanismo de Control de Versiones Invisible (Abstracción de Git)

Para evitar copias de seguridad redundantes, la aplicación ejecuta comandos binarios de Git en segundo plano a través de Rust de forma totalmente transparente para el usuario.

Inicialización: Al crear un proyecto, la app ejecuta git init dentro de la raíz de forma silenciosa.

Guardado Automático (Dos Niveles):

Nivel 1 — Guardado a Disco (Inmediato): Cada cambio en el editor se persiste al archivo .md mediante debounce de 2 segundos tras la última pulsación. Esto garantiza que el contenido nunca se pierda por un cierre inesperado o corte de energía. No se ejecuta commit de Git en este nivel.

Nivel 2 — Checkpoint en Git (Diferido): Tras un umbral de inactividad de 30 minutos y habiendo detectado cambios significativos (+100 palabras acumuladas desde el último checkpoint), la app ejecuta:
git add .
git commit -m "Progreso automático: [Fecha] - [Recuento Palabras]"

Historial Literario (Línea de Vida): La interfaz muestra los commits analizando el git log, traduciéndolo a una interfaz limpia con marcas de tiempo y estadísticas de escritura. El usuario puede seleccionar un punto anterior y restaurar el texto (git checkout).

Líneas Temporales Alternativas (Ramas): Se mapea el concepto de git branch para permitir al escritor crear experimentos narrativos alternativos sin duplicar archivos.

7. Estrategia de Testing

Dado que la aplicación manipula archivos del usuario de forma directa, la prevención de corrupción de datos es prioridad absoluta. Se definen tres niveles de testing:

A. Tests Unitarios (Rust — Backend)

- Cada comando Tauri (#[tauri::command]) debe tener tests unitarios que validen:
  - Creación correcta de la estructura de directorios del proyecto.
  - Escritura y lectura de archivos .md con caracteres especiales y UTF-8.
  - Parsing y serialización de JSON (metadata.json, timeline.json) con validación de esquema.
  - Manejo de errores de E/S: permisos denegados, disco lleno, paths inválidos.
  - Verificación de disponibilidad de Git en el PATH antes de ejecutar comandos.

- Framework: tests nativos de Rust (#[cfg(test)]) con crate `tempfile` para directorios temporales aislados.

B. Tests de Integración (Rust — Tauri Commands)

- Tests end-to-end de los comandos Tauri invocándolos programáticamente:
  - Flujo completo: crear_proyecto → inicializar_git → guardar_capitulo → cargar_indice.
  - Simulación de fallos: Git no instalado, archivo bloqueado por otro proceso.
  - Verificación de que el guardado a disco (Nivel 1) no genera commits de Git.

C. Tests de Frontend (Svelte/TS)

- Tests de componentes con Vitest + Testing Library para Svelte:
  - Renderizado del layout 60/40 y toggle de la barra lateral.
  - Comportamiento del Bubble Menu de TipTap al seleccionar texto.
  - Funcionalidad de drag-and-drop en la barra lateral y timeline.
  - Debounce de guardado: verificar que no se disparan múltiples llamadas al backend en rápida sucesión.

D. Tests de Regresión Manual (Checklist)

- Crear un proyecto con nombre con espacios y caracteres especiales (ej: "Mi Novela — Edición Final").
- Escribir +5000 palabras en un capítulo y verificar que no hay degradación de rendimiento.
- Cerrar la aplicación abruptamente (kill del proceso) y verificar que el contenido se recupera del último guardado a disco.
- Probar la app en Windows y Linux con Git instalado y sin Git instalado.

Ejecución de tests en CI:

```bash
# Backend
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend
pnpm test
```

PARTE 2: GUÍA DE PROTOTIPADO Y INSTRUCCIONES PARA LA IA

1. Preparación del Entorno de Desarrollo

Para compilar de forma nativa en Windows y Linux, necesitaremos configurar el entorno de Tauri.

Requisitos Previos:

Node.js (v18 o superior) y pnpm/npm.

Rust y Cargo (Instalado mediante rustup).

Git instalado en el sistema operativo.

Dependencias de sistema específicas por plataforma:

Linux (Debian/Ubuntu):
```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Linux (Fedora):
```bash
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libxdo-devel libappindicator-gtk3-devel librsvg2-devel
```

Windows:
- Microsoft Visual Studio C++ Build Tools (requerido para compilar Rust en Windows).
- WebView2 Runtime (incluido por defecto en Windows 10/11; en Windows 7/8 debe instalarse manualmente).
- Git para Windows (verificar que esté en el PATH del sistema; el instalador oficial ofrece esta opción).

Nota: En Windows, Git no suele estar en el PATH del sistema por defecto a menos que el usuario lo configure explícitamente durante la instalación. El backend en Rust debe manejar este caso detectando la ruta típica de instalación (`C:\Program Files\Git\bin\git.exe`) como fallback.

Inicialización del Proyecto:

Ejecuta el siguiente comando en tu terminal para crear el scaffolding inicial:

pnpm create tauri-app --template svelte-ts


2. Estructura de Prompts para la IA (Desarrollo por Módulos)

Para construir la aplicación sin errores de contexto, proporciona estos requerimientos a tu asistente de IA en fases estrictas:

📜 PROMPT 1: Configuración del Backend de Archivos (Rust / Tauri Commands)

Contexto: Estoy desarrollando un editor literario Local-First con Tauri y Svelte. Necesito la lógica del backend en Rust para gestionar la estructura de un "Proyecto".

Requerimientos:
Crea las siguientes funciones de Tauri (#[tauri::command]) en Rust:

crear_proyecto(path: String, nombre: String): Debe generar la carpeta raíz, la subcarpeta oculta .config/, y las carpetas capitulos/, personajes/ y notas/. Debe inicializar un archivo .config/metadata.json vacío y un archivo .config/timeline.json vacío.

inicializar_git(path: String): Debe ejecutar de forma silenciosa el comando git init dentro de la carpeta del proyecto.

guardar_capitulo(proyecto_path: String, filename: String, contenido: String): Guarda el archivo .md del capítulo en disco (Nivel 1 — inmediato, sin commit de Git). El checkpoint en Git (Nivel 2) se gestiona por separado mediante un temporizador de inactividad global.

crear_checkpoint(proyecto_path: String): Ejecuta git add . && git commit -m "Progreso automático: [Fecha] - [Recuento Palabras]" cuando el temporizador de inactividad lo dispare. Esta separación asegura que el guardado a disco nunca falle por ausencia de Git.

cargar_indice(proyecto_path: String) -> String: Lee y devuelve el contenido de .config/metadata.json.

Provee el código estructurado para src-tauri/src/main.rs. Asegúrate de manejar correctamente los errores de E/S de disco devolviendo un Result<String, String>.

🎨 PROMPT 2: Interfaz 60/40 y Motor del Editor (Frontend - Svelte/React)

Contexto: Tengo el backend de Tauri listo. Ahora necesito diseñar el layout de la interfaz respetando un enfoque minimalista y el motor de edición enriquecida.

Requerimientos:

Diseña un contenedor principal utilizando CSS para el layout.

El layout debe dividirse estrictamente en:

Barra Lateral Izquierda (40% de ancho fijo o dinámico).

Zona de Escritura Central (60% de la pantalla).

Añade un atajo de teclado global (Ctrl + B) que oculte/muestre la barra lateral instantáneamente mediante una transición suave.

En la zona de escritura (60%), integra el editor TipTap. Configúralo en modo minimalista: sin barras de herramientas superiores fijas. Implementa un menú flotante (Bubble Menu) que aparezca únicamente cuando el usuario selecciona una porción de texto para aplicar Negrita, Cursiva o convertir el texto en un encabezado H1/H2.

El texto del editor debe actualizarse mediante debounce (esperar 2 segundos tras parar de escribir) llamando al comando de guardado del backend para no saturar el disco.

⏳ PROMPT 3: Lógica de la Línea de Tiempo Intercalable (JSON Mapping)

Contexto: Necesito desarrollar el módulo de la línea de tiempo en la columna lateral (40%). El orden cronológico es independiente del orden de los capítulos.

Requerimientos:

Diseña un componente de línea de tiempo vertical para la barra lateral.

El componente debe leer los eventos desde un archivo .config/timeline.json que contiene un array ordenado de objetos. Cada objeto tiene: id, titulo, descripcion_corta, y capitulo_asociado_id.

Implementa una funcionalidad nativa de arrastrar y soltar (Drag and Drop) usando la API nativa de HTML5 o una librería ligera de Svelte/React.

Al arrastrar un evento e intercalarlo entre otros dos, el array en el JSON debe reordenarse inmediatamente reflejando su nueva posición en el índice y guardarse en el disco a través de Tauri.

Cada tarjeta de evento debe incluir un botón minimalista que, al hacer clic, dispare un evento para abrir el capítulo asociado en la zona del editor del 60%.

3. Pipeline de Compilación

Utiliza los siguientes comandos en tu terminal de producción para generar los instaladores nativos listos para distribución:

Para Windows (Genera un archivo .exe ejecutable y un instalador .msi):

pnpm tauri build --target x86_64-pc-windows-msvc


Para Linux (Genera paquetes .deb y .AppImage):

pnpm tauri build --target x86_64-unknown-linux-gnu

OJO. 
Nota crítica de desarrollo:
Al usar comandos nativos de Git en Rust mediante Tauri, asegúrate de añadir un control de errores en caso de que el usuario no tenga Git instalado en su sistema operativo al ejecutar la aplicación por primera vez. De lo contrario, la aplicación lanzará un error silencioso en el backend al intentar inicializar el proyecto. Puedes capturar esto en Rust evaluando si el comando git está disponible en el PATH antes de lanzar git init.

En Windows, implementar un fallback que busque Git en las rutas de instalación típicas (`C:\Program Files\Git\bin\git.exe` y `C:\Program Files (x86)\Git\bin\git.exe`) si no se encuentra en el PATH. En Linux/macOS, `which git` es suficiente. Si Git no está disponible en ninguna ubicación, la app debe mostrar una notificación clara al usuario en la interfaz (no un error silencioso) indicando que el control de versiones no estará activo hasta que se instale Git. La funcionalidad de guardado a disco (Nivel 1) debe seguir funcionando normalmente sin Git.
