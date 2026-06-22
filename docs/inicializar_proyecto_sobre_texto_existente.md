---
name: inicializar-proyecto-sobre-texto-existente
description: "Trigger: inicializar proyecto, crear proyecto desde texto, poblar cronista, importar cuento, importar novela, convertir texto a proyecto cronista. Crea un proyecto Cronista completo y autónomo a partir de un texto literario existente, sin depender del código fuente de Cronista."
license: Apache-2.0
metadata:
  author: "galejan"
  version: "1.0"
  standalone: true
  dependencies: []
---

# Skill: Inicializar Proyecto Cronista desde Texto Existente

> **Standalone** — no require el repo de Cronista. Cualquier agente AI con acceso a sistema de archivos puede ejecutarla.

## Activation Contract

Activar cuando el usuario pide importar un texto literario existente (cuento, novela, relato) como proyecto Cronista. El objetivo es crear la estructura completa del proyecto en disco, analizar el texto, y poblarlo con capítulos, personajes, timeline y notas — sin necesidad de tener el código fuente de Cronista instalado.

## Hard Rules

1. **NO modificar el texto literario original.** El texto del autor se preserva intacto. Solo se organiza, estructura y cataloga.
2. **Estructura de directorios idéntica a Cronista.** El proyecto debe ser 100% compatible con la app Cronista.
3. **Codificación UTF-8 en todos los archivos.**
4. **Respetar el formato exacto de los archivos JSON y MD.** La app Cronista los parsea con schemas específicos.
5. **Los archivos de capítulos usan HTML** (TipTap/ProseMirror output), no Markdown puro.
6. **Mantener nivel de análisis literario**, no técnico.

## Execution Steps

### Fase 0: Verificación Previa

Antes de empezar, verificar:

- [ ] El texto literario existe y se puede leer
- [ ] El directorio de destino NO contiene ya un proyecto Cronista (tiene `.config/metadata.json`)
- [ ] Git está disponible (`git --version`) — opcional, pero recomendado
- [ ] Hay espacio en disco suficiente (un texto típico pesa < 10 MB)

Si el directorio destino YA es un proyecto Cronista, preguntar si quiere sobrescribir o continuar. Nunca destruir datos sin confirmación.

### Fase 1: Lectura y Análisis del Texto

Leer el texto completo y analizarlo:

1. **Tono general**: lírico, épico, íntimo, oscuro, humorístico
2. **Mundo narrativo**: realista, fantástico, histórico, ciencia ficción
3. **Voz del narrador**: primera persona, tercera omnisciente, tercera limitada
4. **Estructura**: ¿tiene divisiones preexistentes (capítulos, partes, actos)?
5. **Personajes**: identificar todos, presentes y mencionados
6. **Línea temporal**: eventos del presente y flashbacks/leyendas
7. **Objetos simbólicos**: elementos recurrentes con carga narrativa

### Fase 2: División en Capítulos

Si el texto ya tiene divisiones (capítulos numerados, saltos `---`, etc.), usarlas. Si no, dividir usando criterios narrativos:

- **Cambio de foco**: salto de perspectiva entre personajes
- **Cambio temporal**: elipsis clara ("A la mañana siguiente...")
- **Arco emocional**: cada capítulo con su propio clímax o revelación
- **Tamaño recomendado**: 800–2500 palabras por capítulo
- **Nombres**: descriptivos, en español (o el idioma del texto), que inviten a leer

Generar N capítulos. Cada capítulo tendrá:
- Un **filename**: `{número}_nombre_descriptivo.md` (ej: `0001_el_encuentro_en_el_rio.md`)
- Un **título** para mostrar
- El **contenido** del texto convertido a HTML de TipTap

#### Formato HTML para capítulos (TipTap)

El HTML debe usar solo estos tags permitidos:

```html
<h1>Título del capítulo</h1>
<h2>Subtítulo</h2>
<h3>Sección</h3>
<p>Párrafo de texto normal.</p>
<p>—Diálogo con guion largo.—</p>
```

No incluir: negritas, cursivas, enlaces, listas, blockquotes, código, imágenes.

Regla de conversión texto → HTML:
- Líneas que empiezan con `# ` → `<h1>`
- Líneas que empiezan con `## ` → `<h2>`
- Líneas que empiezan con `### ` → `<h3>`
- Líneas en blanco separan párrafos → `</p><p>`
- Texto normal envuelto en `<p>...</p>`
- Diálogos con guion largo (`—`) van dentro de `<p>`

### Fase 3: Extracción de Personajes

Para cada personaje (presente o mencionado), extraer:

| Campo | Descripción | Obligatorio |
|-------|-------------|-------------|
| `id` | Slug único: nombre en minúsculas, sin acentos, guiones (ej: `yoshio`) | Sí |
| `name` | Nombre completo como aparece en el texto | Sí |
| `physicalDescription` | Rasgos físicos, edad, vestimenta, forma de moverse | No |
| `personality` | Motivaciones, forma de hablar, contradicciones | No |
| `traumas` | Heridas del pasado que explican su comportamiento | No |
| `relationships` | Array de relaciones con otros personajes | No |

Formato de relaciones:

```json
{
  "targetName": "Nombre del otro personaje",
  "type": "hermano, amigo, enemigo, padre, mentor...",
  "notes": "Notas sobre la relación (ej: unilateral, conflictiva)"
}
```

Incluir personajes presentes en la acción Y personajes evocados (familiares ausentes, figuras legendarias) que influyen en la trama. El hechicero de Hammet es un buen ejemplo de personaje evocado pero estructuralmente esencial.

### Fase 4: Línea de Tiempo

Extraer todos los eventos con relevancia narrativa. Dos escalas:

1. **Eventos del presente** del relato (con la mayor granularidad temporal disponible: día/hora)
2. **Eventos del pasado** mencionados (flashbacks, leyendas, recuerdos)

Cada evento debe vincularse a:
- Personajes relacionados (array de `id` de personajes)
- Capítulos relacionados (array de filenames de capítulos)

Usar el lenguaje temporal del propio relato para las fechas (ej: "~300 cosechas atrás", "Día 1 — Mañana", "La Época Oscura").

### Fase 5: Notas de Análisis

Generar 2–4 notas de análisis literario. Cada nota tiene:

- `id`: slug único
- `title`: título descriptivo (ej: "Simbolismo del pétalo de flor")
- `content`: HTML de TipTap con el análisis

Temas sugeridos para notas:
- Simbolismo de objetos o lugares recurrentes
- Estructura narrativa (circular, lineal, en espiral, marco)
- Temas subyacentes
- Posibles continuaciones o derivaciones
- Relaciones entre personajes complejas
- Paralelismos con otras obras o mitos

## Fase 6: Creación del Proyecto en Disco

Una vez completado el análisis, crear la estructura de directorios y archivos.

### 6.1. Estructura de Directorios

```
{project-path}/
├── .config/
│   ├── metadata.json
│   └── timeline.json
├── capitulos/
│   ├── 0001_nombre_capitulo.md
│   ├── 0002_nombre_capitulo.md
│   └── ...
├── personajes/
│   ├── index.json
│   ├── {id-personaje-1}.json
│   ├── {id-personaje-2}.json
│   └── ...
├── notas/
│   ├── index.json
│   ├── {id-nota-1}.md
│   ├── {id-nota-2}.md
│   └── ...
└── exportaciones/          ← se crea al exportar, no inicial
```

### 6.2. metadata.json

```json
{
  "project_name": "{Nombre del proyecto}",
  "last_modified": "{ISO 8601 timestamp}",
  "chapters_order": [
    "0001_nombre_capitulo.md",
    "0002_nombre_capitulo.md"
  ],
  "characters_index": [
    { "id": "yoshio", "file": "yoshio.json", "name": "Yoshio" },
    { "id": "hammet", "file": "hammet.json", "name": "Hammet" }
  ],
  "font_family": "serif"
}
```

**font_family**: Usar `"serif"` para narrativa literaria (es la que más se acerca a un libro impreso). Opciones válidas: `"monospace"`, `"serif"`, `"sans-serif"`.

**last_modified**: Usar formato ISO 8601: `new Date().toISOString()` o equivalente.

### 6.3. timeline.json

Array de eventos, ordenados cronológicamente:

```json
[
  {
    "id": "evt-{timestamp}",
    "date": "Día 1 — Mañana",
    "title": "Yoshio encuentra a Hammet",
    "description": "Yoshio baja al río cantando y encuentra a Hammet silbando la canción de Ailiana.",
    "relatedCharacters": ["yoshio", "hammet"],
    "relatedChapters": ["0001_el_encuentro_en_el_rio.md"]
  }
]
```

**id**: Generar con timestamp (`Date.now()` o equivalente). Prefijo `evt-`.

### 6.4. personajes/index.json

```json
[
  { "id": "yoshio", "name": "Yoshio" },
  { "id": "hammet", "name": "Hammet" }
]
```

### 6.5. personajes/{id}.json

```json
{
  "id": "yoshio",
  "name": "Yoshio",
  "physicalDescription": "Alta para sus 11 cosechas, ágil, ojos expresivos, rebosante de vida",
  "personality": "Curiosa, justa, leal, dispersa en el buen sentido",
  "traumas": "Abandono parental — sus padres se fueron a trabajar y nunca volvieron",
  "relationships": [
    {
      "targetName": "Abuela",
      "type": "nieta",
      "notes": "Cuidado mutuo, la cría con canciones y valores"
    },
    {
      "targetName": "Hammet",
      "type": "conexión paterna",
      "notes": "Inmediata, lo defiende sin conocerlo"
    }
  ]
}
```

### 6.6. notas/index.json

```json
[
  { "id": "simbolismo-petalo", "title": "Simbolismo del pétalo de flor" },
  { "id": "estructura-narrativa", "title": "Estructura circular del relato" }
]
```

### 6.7. notas/{id}.md

Archivo HTML de TipTap (mismos tags que los capítulos):

```html
<h1>Simbolismo del pétalo de flor</h1>
<p>Un tallado tosco y desgastado que conecta a tres generaciones. Cada vez que el pétalo cambia de manos, alguien llora.</p>
```

## Fase 7: Inicialización Git (Opcional)

Si Git está disponible, ofrecer inicializar el repositorio:

```bash
cd "{project-path}"
git init
git add .
git commit -m "Primera piedra — {Nombre del proyecto}"
```

Si no, informar que el proyecto funciona igual pero sin control de versiones.

## Output Contract

Al finalizar, presentar:

1. **Resumen del proyecto creado**: ruta, nombre, cantidad de capítulos, personajes, eventos de timeline, notas
2. **Decisiones de análisis**: por qué se dividió así, qué personajes se incluyeron, criterios usados
3. **Estado de Git**: inicializado o no
4. **Próximos pasos**: cómo abrir el proyecto en Cronista, qué mejorar

## Verificación

Después de crear el proyecto, verificar:

- [ ] La estructura de directorios existe y es correcta
- [ ] `metadata.json` tiene JSON válido con todos los campos requeridos
- [ ] `chapters_order` existe y sus valores coinciden con archivos reales en `capitulos/`
- [ ] `characters_index` existe y sus valores coinciden con archivos reales en `personajes/`
- [ ] Cada capítulo es HTML de TipTap válido (solo h1/h2/h3/p)
- [ ] Cada personaje tiene al menos `id` y `name` poblados
- [ ] `timeline.json` existe y es un array válido
- [ ] `notas/index.json` existe (incluso vacío = `[]`)
- [ ] Todos los archivos son UTF-8 válidos
- [ ] NO hay archivos residuales fuera de la estructura esperada

## Referencia: Ejemplo "Hammet"

Ver `/home/alex/Documentos/Hammet/analisis-ejemplo-completo.md` para un ejemplo completo de análisis literario que respeta este formato.

### Datos del ejemplo:
- **Texto**: Hammet (relato fantástico, ~4000 palabras)
- **Capítulos**: 4 (El encuentro en el río, El pétalo de Keisha, La noche y la mañana, La leyenda)
- **Personajes**: 6 (Yoshio, Hammet, Abuela, Keisha, Ailiana, El Hechicero)
- **Timeline**: 10 eventos (2 escalas temporales)
- **Notas**: 3 (simbolismo, estructura circular, ser justo vs cumplir promesas, continuación)

## Flujo Autónomo (sin Cronista)

Cuando un agente AI ejecuta esta skill sin tener acceso al código fuente de Cronista:

```
1. LEER texto literario (ruta proporcionada por el usuario)
2. ANALIZAR estructura narrativa, personajes, timeline
3. PREGUNTAR al usuario: nombre del proyecto, ruta de destino, ¿inicializar Git?
4. CREAR directorios y archivos según los formatos documentados arriba
5. INICIALIZAR Git si corresponde
6. VERIFICAR integridad del proyecto
7. INFORMAR resultado

El agente NO necesita importar nada de Cronista. Solo sigue estas
instrucciones y escribe archivos directamente en disco.
```

El proyecto resultante es 100% compatible con la app Cronista: se puede abrir desde la interfaz "Abrir proyecto" seleccionando la carpeta raíz.
