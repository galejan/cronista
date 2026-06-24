# Especificación de editor-integration

## Propósito

Define la integración del editor TipTap en la zona del 60%, el pipeline de guardado con debounce de 2 segundos y el flujo de creación/carga de capítulos. Todo el contenido se persiste mediante comandos Tauri vía IPC.

## Requisitos

### Requisito: Renderizado del Componente Editor

El sistema DEBE renderizar un editor TipTap dentro de la zona central (60%) al montar `Editor.svelte`. El editor DEBE inicializarse en `onMount` y destruirse en `onDestroy`.

#### Escenario: El editor se monta y muestra contenido inicial

- DADO un capítulo activo con contenido HTML
- CUANDO `Editor.svelte` se monta
- ENTONCES TipTap se inicializa y muestra el contenido provisto
- Y el área de edición ocupa el 100% de la zona central

#### Escenario: El editor se desmonta limpiamente

- DADO un editor TipTap activo con listeners de `onUpdate` registrados
- CUANDO el componente se desmonta (cambio de capítulo o navegación)
- ENTONCES la instancia de TipTap se destruye sin errores
- Y los listeners pendientes se cancelan

### Requisito: Menú Flotante de Formato

El sistema DEBE mostrar un Bubble Menu con controles de encabezado (h1, h2, h3) y selector de familia tipográfica al seleccionar texto. No DEBE incluir negrita, cursiva ni enlaces en este alcance.

#### Escenario: El menú flotante aparece al seleccionar texto

- DADO un editor TipTap con contenido
- CUANDO el usuario selecciona una porción de texto
- ENTONCES aparece el Bubble Menu con opciones h1, h2, h3 y font-family
- Y el menú se posiciona cerca de la selección

#### Escenario: Aplicar encabezado desde el menú flotante

- DADO el Bubble Menu visible sobre texto seleccionado
- CUANDO el usuario hace clic en "h2"
- ENTONCES el texto seleccionado se convierte en encabezado de nivel 2
- Y el menú flotante se oculta

### Requisito: Guardado con Debounce

El sistema DEBE persistir el contenido del editor mediante `guardar_capitulo` tras 2 segundos de inactividad desde la última pulsación. El debounce DEBE cancelarse al desmontar el componente o cambiar de capítulo.

#### Escenario: El contenido se guarda tras 2 segundos de inactividad

- DADO un editor con contenido modificado y un capítulo activo
- CUANDO el usuario deja de escribir durante 2 segundos
- ENTONCES `guardar_capitulo` se invoca con la ruta del proyecto, el nombre del archivo y el HTML actual

#### Escenario: Escritura continua reinicia el temporizador

- DADO un temporizador de debounce en curso (1.5s transcurridos)
- CUANDO el usuario pulsa una tecla
- ENTONCES el temporizador se reinicia a 2 segundos
- Y no se invoca `guardar_capitulo` hasta que transcurran otros 2s sin actividad

#### Escenario: El debounce se cancela al cambiar de capítulo

- DADO un temporizador de debounce pendiente para el capítulo A
- CUANDO el usuario selecciona el capítulo B
- ENTONCES el temporizador pendiente se cancela
- Y el contenido del capítulo A no se guarda en el archivo del capítulo B

### Requisito: Flujo de Creación de Capítulo

El sistema DEBE permitir crear un nuevo capítulo invocando `crear_capitulo`, que crea el archivo .md y actualiza `metadata.json`. Tras la creación, el editor DEBE cargar el nuevo capítulo para edición.

#### Escenario: Se crea un nuevo capítulo y se abre en el editor

- DADO un proyecto existente con `capitulos/` y `metadata.json`
- CUANDO el usuario crea un nuevo capítulo con nombre "0003_capitulo_3.md" y contenido inicial "# Capítulo 3\n\n"
- ENTONCES el archivo se crea con el contenido dado
- Y `metadata.json` incluye "0003_capitulo_3.md" en `chapters_order`
- Y el editor carga el contenido del nuevo capítulo

#### Escenario: Error al crear capítulo con nombre duplicado

- DADO un capítulo "0001_prologo.md" que ya existe
- CUANDO se invoca `crear_capitulo` con el mismo nombre
- ENTONCES el comando retorna `Err(String)` indicando que el archivo ya existe

### Requisito: Flujo de Carga de Capítulo

El sistema DEBE cargar el contenido de un capítulo mediante `cargar_capitulo`. El contenido retornado DEBE inyectarse en el editor vía `editor.commands.setContent()`.

#### Escenario: Se carga un capítulo existente

- DADO un proyecto con `capitulos/0001_prologo.md` que contiene "# Prólogo\n\nTexto..."
- CUANDO el usuario selecciona "0001_prologo.md" en la barra lateral
- ENTONCES `cargar_capitulo` retorna el contenido del archivo
- Y el editor muestra el contenido cargado

#### Escenario: Error al cargar capítulo inexistente

- DADO un proyecto sin el archivo "0001_prologo.md"
- CUANDO se invoca `cargar_capitulo` para ese archivo
- ENTONCES el comando retorna `Err(String)` indicando archivo no encontrado

### Requisito: Menú Contextual "Agregar a Lugar"

El menú contextual DEBE mostrar una opción "Agregar a lugar" cuando el usuario selecciona texto en el editor. Seleccionar esta opción DEBE pedir al usuario que elija un lugar existente o cree uno nuevo. El texto seleccionado DEBE agregarse al campo `description` del lugar elegido mediante `actualizar_lugar`.

#### Escenario: Agregar texto seleccionado a un lugar existente

- DADO que el usuario ha seleccionado "la torre del vigía" en el editor
- CUANDO hace clic derecho y elige "Agregar a lugar" → selecciona "Torre Norte"
- ENTONCES se invoca `actualizar_lugar` con la descripción actual del lugar + "\n" + el texto seleccionado
- Y un toast de éxito confirma la actualización

#### Escenario: Crear nuevo lugar desde el menú contextual

- DADO que el usuario ha seleccionado texto y hace clic derecho
- CUANDO elige "Agregar a lugar" → "Crear nuevo"
- ENTONCES un prompt recoge el nombre y una descripción inicial opcional
- Y se invoca `crear_lugar` con el texto seleccionado como descripción
- Y el nuevo lugar aparece en la pestaña Lugares

#### Escenario: Opción oculta cuando no hay texto seleccionado

- DADO que el cursor está en el editor sin una selección activa
- CUANDO se abre el menú contextual
- ENTONCES "Agregar a lugar" NO aparece en el menú

#### Escenario: Cancelar cierra el prompt sin efectos

- DADO que el prompt "Agregar a lugar" está abierto
- CUANDO el usuario cancela o hace clic fuera
- ENTONCES no se invoca ningún comando IPC
- Y el estado del editor no cambia
