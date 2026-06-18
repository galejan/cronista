# Delta para project-file-management

## ADDED Requirements

### Requisito: Lectura de Archivo de Capítulo

El sistema DEBE leer el contenido de un único archivo .md de capítulo y retornarlo como string UTF-8. El comando `cargar_capitulo` recibe la ruta del proyecto y el nombre del archivo. El parseo y validación del contenido son responsabilidad del frontend.

#### Escenario: Lee un capítulo existente

- DADO un proyecto en `/tmp/proj` con `capitulos/0001_prologo.md` que contiene "# Prólogo\n\nEra una noche..."
- CUANDO `cargar_capitulo("/tmp/proj", "0001_prologo.md")` es invocado
- ENTONCES retorna `Ok("# Prólogo\n\nEra una noche...")`
- Y el contenido preserva codificación UTF-8

#### Escenario: Retorna error para archivo inexistente

- DADO un proyecto donde `capitulos/9999_fantasma.md` no existe
- CUANDO `cargar_capitulo("/tmp/proj", "9999_fantasma.md")` es invocado
- ENTONCES retorna `Err(String)` indicando que el archivo no fue encontrado

#### Escenario: Retorna error para ruta inválida

- DADO un proyecto_path vacío
- CUANDO `cargar_capitulo("", "test.md")` es invocado
- ENTONCES retorna `Err(String)` indicando ruta inválida

### Requisito: Creación de Capítulo con Registro en Metadatos

El sistema DEBE crear un nuevo archivo .md de capítulo y registrar su entrada en `metadata.json` dentro de `chapters_order`. El comando `crear_capitulo` recibe ruta del proyecto, nombre del archivo y contenido inicial. La operación DEBE escribir primero el archivo y luego actualizar los metadatos para minimizar corrupción por crash.

#### Escenario: Crea capítulo y actualiza metadatos

- DADO un proyecto en `/tmp/proj` con `metadata.json` que tiene `chapters_order: ["0001_prologo.md"]`
- CUANDO `crear_capitulo("/tmp/proj", "0002_capitulo_1.md", "# Capítulo 1\n\n")` es invocado
- ENTONCES `capitulos/0002_capitulo_1.md` existe con el contenido provisto
- Y `metadata.json` contiene `chapters_order: ["0001_prologo.md", "0002_capitulo_1.md"]`
- Y `last_modified` se actualiza a la fecha/hora actual en ISO 8601

#### Escenario: Retorna error para capítulo duplicado

- DADO un proyecto donde "0001_prologo.md" ya existe en `capitulos/`
- CUANDO `crear_capitulo` es invocado con el mismo nombre de archivo
- ENTONCES retorna `Err(String)` indicando que el capítulo ya existe
- Y `metadata.json` no se modifica

#### Escenario: Maneja contenido Unicode

- DADO un proyecto con directorio `capitulos/`
- CUANDO el contenido incluye ñ, áéíóú, emojis y caracteres CJK
- ENTONCES el archivo se crea con codificación UTF-8 correcta
- Y la lectura posterior retorna contenido idéntico
