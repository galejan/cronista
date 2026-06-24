---
name: analisis-creativo
description: "Trigger: analizar cuento, analizar relato, partir capítulos, fichas de personajes, línea de tiempo narrativa, creative writing analysis. Analiza una obra literaria sin modificar el texto original: divide en capítulos, extrae personajes con relaciones, construye línea de tiempo y genera notas de análisis."
license: Apache-2.0
metadata:
  author: "galejan"
  version: "1.0"
---

# Skill: Análisis Creativo Literario

## Activation Contract

Activar cuando el usuario pide analizar un cuento, relato o novela para poblarlo en Cron-Insta. El foco es organización y análisis, NUNCA edición del texto original.

## Hard Rules

1. **NO EDITAR EL TEXTO ORIGINAL.** El texto literario es sagrado. Solo se organiza, cataloga y analiza.
2. Respetar el tono y estilo del autor al escribir descripciones de personajes, eventos y notas.
3. Las fechas en la línea de tiempo deben usar el lenguaje del propio relato (ej: "~300 cosechas atrás", "La Época Oscura", "Día 1 — Mañana"), no formatos cronológicos ajenos a la obra.
4. Mantener un nivel de análisis literario, no técnico. Esto es para un escritor, no para un ingeniero.

## Execution Steps

### 1. Lectura completa
Leer el texto completo una vez sin intervenir. Identificar:
- Tono general (lírico, épico, íntimo, oscuro)
- Mundo narrativo (realista, fantástico, histórico)
- Voz del narrador

### 2. División en capítulos
Partir el relato en unidades narrativas con identidad propia. Criterios:
- **Cambio de foco**: cuando la perspectiva salta de un personaje a otro
- **Cambio temporal**: cuando hay una elipsis clara ("La mañana siguiente...")
- **Arco emocional**: cada capítulo debería tener su propio clímax o revelación
- **Tamaño**: idealmente entre 800 y 2500 palabras por capítulo
- **Nombres**: descriptivos y evocadores, en español, que inviten a leer

### 3. Extracción de personajes
Para cada personaje nombrar:
- **Físico**: rasgos distintivos, edad aparente, forma de moverse
- **Personalidad**: motivaciones, forma de hablar, contradicciones internas
- **Traumas**: heridas del pasado que explican su comportamiento presente
- **Relaciones**: con otros personajes, especificando tipo de vínculo y notas

Incluir tanto personajes presentes en la acción como personajes mencionados que influyen en la trama (familiares ausentes, figuras legendarias, etc.).

Las relaciones deben indicar si son bidireccionales o unilaterales. Ejemplo de relación unilateral: "A está enamorado de B (B no lo sabe)".

### 4. Línea de tiempo
Extraer todos los eventos con relevancia narrativa:
- Eventos del presente del relato (con granularidad de día/hora si aplica)
- Eventos del pasado que se mencionan (flashbacks, leyendas, recuerdos)
- Vincular cada evento con los personajes y capítulos relacionados

### 5. Notas de análisis
Generar 2-4 notas breves con observaciones literarias:
- Simbolismo de objetos o lugares recurrentes
- Estructura narrativa (circular, lineal, en espiral)
- Temas subyacentes
- Posibles continuaciones o derivaciones

Las notas deben ser sugerentes, no dogmáticas. Usar preguntas abiertas cuando corresponda.

## Output Contract

Al terminar, presentar:
1. Lista de capítulos con títulos y breve descripción
2. Fichas completas de personajes (físico, personalidad, traumas, relaciones)
3. Timeline con 5-10 eventos clave
4. 2-4 notas de análisis
5. Resumen de decisiones tomadas (por qué se partió así, qué quedó fuera)

Todo en español, con el mismo tono cálido y respetuoso hacia la obra que se usó en el análisis de "Hammet".

## Example: Análisis de "Hammet"

### Capítulos (4)
1. **El encuentro en el río** — Yoshio descubre a Hammet, la canción de Ailiana, primeras sospechas
2. **El pétalo de Keisha** — Flashback de Hammet, el intercambio, el abrazo que lo rompe
3. **La noche y la mañana** — Dos noches, la despedida, el pez muerto
4. **La leyenda** — La abuela revela la verdad: la peste que pasa

### Personajes (6)
Yoshio, Hammet, Abuela, Keisha, Ailiana, El Hechicero — cada uno con ficha completa de físico, personalidad, traumas y relaciones cruzadas.

### Timeline (8 eventos)
Desde "~300 cosechas atrás — Nacimiento de Keisha" hasta "Día 2 — Tarde — La abuela revela la leyenda". Dos escalas temporales: el pasado legendario y el presente de 48 horas.

### Notas (3)
- Simbolismo del pétalo de flor
- Estructura circular del relato
- Posible continuación: ¿vuelve Hammet?
