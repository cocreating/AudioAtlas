# Audio Atlas — propuesta funcional y prompt técnico

Fecha: 27 de septiembre de 2026. Nombre provisional.

## 1. Objetivo y supuestos

Aplicación de escritorio para convertir una colección dispersa de audios en una biblioteca consultable, útil para composición, diseño sonoro y trabajo con Ableton Live. Principio central: indexar los archivos donde están y organizar virtualmente mediante metadatos, sin cambiar los originales.

Hipótesis iniciales: macOS como primera plataforma, Apple Silicon como primer objetivo de compilación, funcionamiento local sin cuenta ni servicios externos obligatorios. Confirmar antes de distribuir la arquitectura del Mac, versión de macOS y versión/edición de Live. No se presupone que el ordenador del usuario sea Apple Silicon ni que disponga de Suite. Windows e Intel son objetivos posteriores sujetos a pruebas específicas.

Casos de uso: muestras de librerías, loops, golpes individuales, grabaciones de campo, voces, stems, pistas de referencia, renders de sintetizadores, exportaciones de Live y versiones de composiciones propias. Los archivos de proyecto y presets se catalogan como contexto; no se tratan como audio reproducible.

Éxito del producto: reducir el tiempo entre buscar un sonido y probarlo dentro de una composición; recuperar material olvidado; conservar la procedencia y evitar perder referencias de proyectos.

## 2. Funcionalidades propuestas

| Área | Funcionalidad | Utilidad | Fase |
|---|---|---|---|
| Inventario | Selección de carpetas, volúmenes externos y exclusiones | Unificar el catálogo sin mover archivos | 1 |
| Escaneo | Indexación incremental, cola persistente y modo de bajo consumo | Seguir trabajando mientras se cataloga | 1 |
| Metadatos | Formato, duración, canales, frecuencia, profundidad cuando proceda, tamaño, fecha y etiquetas existentes | Filtrar por características reales | 1 |
| Preescucha | Waveform, búsqueda temporal, loop A/B, volumen y atajos | Evaluar sonidos con rapidez | 1 |
| Clasificación | Etiquetas, favoritos, valoración, notas, estado pendiente/revisado | Construir una taxonomía personal | 1 |
| Colecciones | Listas manuales y colecciones basadas en filtros | Preparar una sesión o encontrar material pendiente | 1 |
| Duplicados | Igualdad de bytes mediante hash completo | Localizar copias exactas | 1 |
| Entrega a Live | Arrastre nativo, revelar en Finder y carpeta de exportación persistente | Llevar una selección a Live | 1 |
| Ritmo y tono | Estimación de BPM, candidatos x2/÷2, tonalidad y nota dominante cuando corresponda | Buscar materiales musicalmente compatibles | 2 |
| Descriptores | Ataque, densidad de transitorios, brillo, RMS, picos, silencios y sonoridad cuando sea apropiado | Encontrar texturas por características medibles | 2 |
| Clasificación asistida | Instrumento, loop/one-shot, voz, ambiente, efecto, textura | Reducir etiquetado manual | 2 |
| Similitud | Buscar sonidos parecidos a uno seleccionado y consultar por descripción | Explorar sin conocer nombres de archivo | 2 |
| Fragmentos | Marcadores y regiones dentro de grabaciones largas | Recuperar un instante útil sin duplicar el original | 2 |
| Variantes | Agrupar stems, tomas, renders y versiones con revisión humana | Ordenar composiciones y exportaciones | 2 |
| Audición contextual | Tempo objetivo, transposición y comparación de sonoridad | Probar compatibilidad antes de exportar | 2 |
| Integración | Link para sincronía; Link Audio para enviar preescucha | Escuchar en contexto dentro de Live | 3 |
| Proyectos | Relacionar sets y muestras con cobertura de análisis visible | Entender dependencias conocidas | 3 |
| Automatización | Adaptadores opcionales Max for Live / Extensions SDK | Acciones concretas dentro del DAW | 3 |
| Gestión física | Plan previo de copia/renombrado/movimiento, historial y recuperación | Ordenar archivos cuando sea realmente necesario | 3 |
| Exploración | Mapa visual de similitud, selección aleatoria filtrada, material poco usado | Favorecer descubrimiento y composición | 3 |

No incluir en el MVP un editor multipista, hosting de plugins, separación de stems, generación de música, sincronización cloud ni un asistente conversacional general. La separación de stems puede integrarse después mediante una herramienta externa, preservando procedencia y parámetros.

## 3. Taxonomía y flujos para el usuario

Separar dimensiones en vez de una sola jerarquía rígida:

- Tipo: one-shot, loop, stem, grabación, canción, fragmento.
- Fuente: kick, snare, hats, percusión, bajo, sintetizador, guitarra, piano, voz, ambiente.
- Carácter: seco, oscuro, granular, metálico, orgánico, saturado, espacial.
- Función: groove, transición, fill, impacto, fondo, textura, hook.
- Estado: sin escuchar, candidato, favorito, descartado, usado en una selección.
- Procedencia: grabación propia, librería, sintetizador, resampling, exportación de IA, otra fuente.
- Contexto: composición, álbum, sesión, versión, autor, notas y enlace a información de licencia aportada por el usuario.

Un archivo puede tener varias etiquetas. La aplicación no puede deducir de su sonido los derechos de uso ni demostrar que fue generado por IA. No asignar esas categorías automáticamente como hechos.

Flujo A: añadir carpeta → consultar resultados mientras continúa el escaneo → filtrar → escuchar → etiquetar → añadir a una bandeja → exportar o arrastrar a Live.

Flujo B: elegir un sample → encontrar similares → restringir duración y carácter → comparar A/B → guardar candidatos. La similitud tímbrica no garantiza compatibilidad armónica o rítmica.

Flujo C: filtrar material sin escuchar → audición rápida → guardar marcadores interesantes → crear colección para una sesión. Contar como escuchado sólo tras un umbral documentado; permitir corrección manual.

Flujo D: agrupar exportaciones relacionadas → comparar mezclas → etiquetar favorita → reunir stems asociados. La coincidencia de nombres genera una sugerencia, no una relación confirmada.

Ejemplos: «voces susurradas cortas», «texturas granulares oscuras», «percusión seca entre 120 y 128 BPM», «ambientes propios sin escuchar» y «renders del proyecto Soft Dispossession». Las búsquedas semánticas se introducirán después de medir su calidad, también en español.

## 4. Interfaz propuesta

Ventana principal con barra lateral de fuentes y colecciones; buscador y filtros superiores; tabla central virtualizada; inspector lateral; reproductor inferior persistente. La bandeja de sesión puede ocupar un panel plegable.

Columnas configurables: nombre, tipo, duración, BPM, tonalidad, etiquetas, valoración, disponibilidad y procedencia. BPM y tonalidad mostrarán si proceden del nombre, de análisis o de una corrección manual.

Diseño sobrio, contraste suficiente, modo oscuro, controles reconocibles, foco visible, navegación por teclado y etiquetas accesibles. Reducir animaciones y actualizaciones visuales durante reproducción. Adaptar la ventana a tamaños pequeños sin exigir un diseño de teléfono. CSS mobile-first, con breakpoints min-width en em, en orden ascendente y sólo los necesarios.

Atajos iniciales: espacio para reproducir/pausar, flechas para selección, Cmd+F para buscar, tecla configurable para favorito. No capturar espacio o letras mientras se escribe en un campo. Los atajos son locales a la app salvo una opción global explícita futura.

## 5. Arquitectura recomendada

| Componente | Propuesta | Razón y límite |
|---|---|---|
| Aplicación | Tauri 2 + Rust | Coordinar archivos, persistencia, trabajos y servicios nativos |
| Interfaz | SvelteKit + TypeScript, adapter-static, SPA sin SSR en producción | Aprovechar el entorno web del usuario sin servidor de aplicación |
| Estilos | CSS nativo, componentes pequeños | Evitar Tailwind y dependencias de presentación innecesarias |
| Datos | SQLite local + FTS5 | Catálogo, filtros e índice textual; no confundir búsqueda textual con vectorial |
| Decodificación | ffprobe y FFmpeg empaquetados, versión fijada y build documentada | Inspección, lectura, proxies y exportaciones explícitas |
| Reproducción | Servicio nativo, decodificación por bloques y salida de audio desacoplada | Archivos largos sin cargar todo en memoria ni transportar PCM como JSON |
| Análisis musical | Worker separado, inicialmente Python empaquetado si la prueba de distribución lo valida | Prototipar algoritmos; evitar dependencia del Python instalado por el usuario |
| IA opcional | Adaptador local de embeddings audio/texto | Validar calidad, licencia de pesos, tamaño y coste antes de elegir modelo |

El motor nativo de reproducción es una decisión que debe probarse en una primera vertical. Validar salida de audio, seek, cambio de dispositivo y empaquetado antes de ampliar el catálogo. Una capa de abstracción permite cambiar su implementación sin rehacer la interfaz.

Essentia es una candidata para análisis musical, no una dependencia aprobada de antemano: su licencia y las de sus componentes/modelos necesitan revisión para el modo de distribución elegido. Librosa es otra candidata para prototipado; no resuelve por sí sola todas las tareas de clasificación o tonalidad. CLAP es una familia candidata para similitud audio/texto, no una garantía de calidad para kicks o loops cortos. Un runtime como ONNX sólo se incorporará si se demuestra que ejecuta el modelo concreto con resultados y rendimiento aceptables.

No arrancar servidores HTTP ni servicios cloud en el MVP. Comunicar frontend y backend mediante IPC tipado, y workers mediante un protocolo local versionado con request_id, cancelación, progreso y errores estructurados.

## 6. Límites de integración con Ableton

1. Base universal: archivos físicos disponibles, exportaciones duraderas, Finder y arrastre nativo probado en Live. Un gesto drag de HTML no demuestra transferencia de archivos entre aplicaciones.
2. Carpeta de intercambio: copias seleccionadas que el usuario pueda añadir a Places. No generar formatos de proyecto propietarios ni un .alp simulando que es un Pack oficial.
3. Link: sincronización musical de tempo, pulso y fase. Cambiar tempo de sesión requiere una acción explícita; conectarse no debe publicar automáticamente el BPM de un sample.
4. Link Audio: opción posterior para transmitir la preescucha a Live; la documentación indica soporte desde Live 12.4 y funcionamiento en localhost. Medir latencia y evitar monitorizar dos veces. No sustituye la exportación del archivo original.
5. Max for Live: posible puente a funciones expuestas por la Live API. Comprobar cada operación y la disponibilidad en la instalación del usuario; no prometer acceso universal al DAW.
6. Extensions SDK: opción posterior para tareas discretas sobre sets. Según la documentación consultada, disponible en Live 12 Suite Beta 12.4.5 o posterior; no diseñado para procesamiento continuo de audio ni sincronía de tiempo real.
7. Dependencias de proyectos: preferir APIs documentadas donde resuelvan el caso. Si se analiza .als directamente, aislar el parser, probar versiones y trabajar en sólo lectura. Una extracción parcial nunca demuestra que un sample no se usa en ningún proyecto.

Ableton documenta que usar un archivo externo no implica copiarlo dentro del proyecto; Collect All and Save reúne los archivos referenciados. Un catálogo externo no sustituye ese procedimiento ni puede garantizar que los proyectos toleren un movimiento de sus samples.

## 7. Prompt técnico completo, listo para entregar a un agente de desarrollo

--- INICIO DEL PROMPT ---

Actúa como arquitecto de software y desarrollador senior de aplicaciones de escritorio para audio. Desarrolla Audio Atlas, una herramienta local para catalogar, buscar, escuchar, clasificar y preparar archivos de audio para trabajar con Ableton Live.

### A. Resultado y alcance

Entrega código ejecutable y una primera versión funcional con datos reales. Implementa únicamente la fase 1 descrita aquí; prepara interfaces de extensión para fases 2 y 3 sin simular funciones inexistentes. Si el encargo actual se realiza en un repositorio, inspecciona su estructura e instrucciones antes de modificarlo. No sustituyas código existente ni cambies el stack sin explicar una incompatibilidad concreta.

Primera plataforma objetivo: macOS. Usa Apple Silicon como objetivo inicial provisional, documentando arquitectura y versión mínima soportadas. No declares soporte Intel o Windows sin compilar y verificar. La versión y edición de Ableton son parámetros de compatibilidad, no supuestos ocultos.

La app debe funcionar sin cuenta, sin conexión y sin Python, Node o FFmpeg preinstalados en el ordenador del usuario final. Las herramientas de desarrollo pueden requerirlos. Los análisis pesados deben poder detenerse para trabajar con Live. No enviar audio, nombres de archivos ni telemetría a terceros.

### B. Stack y límites

- Tauri 2, Rust, SvelteKit, TypeScript y CSS nativo. Sin React, Tailwind, jQuery, Electron ni Docker salvo incompatibilidad demostrada y decisión documentada.
- SvelteKit con adapter-static y modo SPA; ninguna dependencia de endpoints de servidor en el binario distribuido.
- SQLite local con migraciones, FTS5 e índices para los filtros de la interfaz. Base de datos en el directorio de datos de la app, no en un disco externo ni una carpeta sincronizada por defecto.
- FFmpeg/ffprobe como ejecutables empaquetados con arquitectura, versión, checksums y opciones de compilación documentadas. Inventario de licencias antes de distribución.
- Audio mediante un servicio nativo sustituible que lea y decodifique por bloques. Selecciona dependencias concretas tras una prueba pequeña de reproducción y empaquetado.
- No cargar PCM completo de grabaciones largas en el frontend. No transportar audio por eventos IPC JSON. La interfaz consume estado, posición y niveles de waveform precomputados.

### C. Primera vertical obligatoria

Antes de implementar todas las pantallas, demuestra: abrir carpeta → listar un archivo real → leer metadatos → reproducir y hacer seek → persistir una etiqueta → reiniciar y recuperarla → exportar una copia → probar arrastre nativo a Live cuando el entorno disponga de Live.

Registra lo probado y lo pendiente. Si no hay macOS o Live disponibles, no declares verificadas su reproducción, distribución o integración. Mantén como alternativa funcional «revelar en Finder» y exportación a carpeta. No uses esta alternativa para ocultar que el arrastre queda pendiente.

### D. Indexación

Permitir añadir y retirar raíces autorizadas, exclusiones y subcarpetas. Retirar una raíz del catálogo no elimina archivos del disco. Escanear almacenamiento montado y permitido por el usuario, no prometer acceso a todo el ordenador sin permisos.

Formatos iniciales: WAV, AIFF, FLAC, MP3, M4A/AAC y OGG/Vorbis cuando el decoder empaquetado los soporte. Detectar contenedor y codec; no confiar sólo en la extensión. Registrar «no compatible» sin bloquear la cola. No inventar profundidad de bits para formatos donde no procede.

Guardar metadatos técnicos y etiquetas existentes. Separar inventario rápido, waveform, hash completo y análisis musical. El catálogo debe ser utilizable antes de acabar las tareas pesadas.

Escaneo incremental por eventos del sistema y reconciliación periódica/manual. Evitar loops de enlaces simbólicos y raíces solapadas. Manejar permisos denegados, rutas Unicode, archivos corruptos, nombres largos, discos desmontados y archivos todavía en escritura. No descargar automáticamente placeholders de servicios cloud.

Un disco desconectado se marca offline; sus registros, etiquetas y colecciones permanecen. Reconciliar por identidad de volumen y ruta relativa. Usar identificadores del sistema cuando sean útiles, sin asumir que sobreviven a una copia. Si cambian tamaño o fecha durante un análisis, descartar el resultado y reencolar tras estabilización.

### E. Modelo de datos

Diseña migraciones para:

- roots: identidad de volumen, ruta autorizada, exclusiones, estado y último escaneo completo.
- files: UUID estable del catálogo, root_id, ruta relativa, identidad del sistema opcional, tamaño, mtime, estado y metadatos.
- contents: hash completo, algoritmo y relación con una o más ubicaciones. No fusionar ubicaciones ni borrar archivos al detectar coincidencias.
- analyses: file/content_id, algoritmo/modelo, versión, parámetros, ámbito temporal, fecha, resultado, estado y evidencia/calidad si existe.
- annotations: valores del usuario, notas, favoritos, valoración y procedencia. Conservar datos detectados aunque una corrección manual tenga prioridad.
- tags y file_tags: clasificación multietiqueta con origen manual, nombre de archivo o algoritmo.
- collections y collection_items: listas ordenadas; smart_queries: filtros guardados versionados.
- regions: intervalos referidos a frames del audio fuente, sample rate de referencia, nombre y anotaciones.
- jobs: tipo, objetivo, prioridad, progreso, intentos, estado, cancelación y error.
- exports: destino, elementos, hashes, parámetros, origen y resultado por elemento.

Reservar extensiones posteriores para relaciones entre versiones/stems, embeddings y referencias de proyectos. No almacenar vectores de modelos diferentes como si fueran comparables.

### F. Búsqueda y clasificación

Buscar nombre, ruta, etiquetas y notas. Combinar filtros por duración, formato, canales, favorito, valoración, disponibilidad, colección y estado de análisis. Añadir campos manuales BPM/tonalidad y extracción opcional desde nombres, siempre etiquetando su procedencia.

En el MVP no fingir detección musical si sólo se ha leído un nombre. Soportar desconocido y no aplicable. Guardar consultas como colecciones inteligentes. Paginación por cursor y tabla virtualizada; no leer todo el catálogo para cada pulsación. Consulta cancelable, parametrizada y con debounce breve.

### G. Reproducción

Play/pause, seek, loop A/B, volumen, silencio y siguiente/anterior. Sólo una preescucha simultánea por defecto. Fades cortos al cambiar de archivo para reducir clics. Waveform multinivel almacenada como caché regenerable; no es necesario decodificar entero cada archivo al seleccionarlo.

Permitir salida de audio seleccionable si el motor lo soporta y probar recuperación ante desconexión. Respetar las capacidades del dispositivo; no asumir soporte multicanal. Documentar la política de downmix de preescucha, preservando el original.

La igualación perceptual de volumen, time-stretch y transposición pertenecen a fase 2. Cuando se implementen, serán ajustes de preescucha independientes y sólo se aplicarán a una exportación mediante una opción explícita.

### H. Duplicados y seguridad de archivos

Agrupar primero por tamaño y después calcular un hash criptográfico completo en streaming. Un hash parcial sólo sirve para seleccionar candidatos. Antes de cualquier futura eliminación, revalidar el estado y verificar igualdad de bytes cuando sea necesario.

El MVP únicamente informa de copias idénticas y permite revelar sus ubicaciones. No detecta como idénticos WAV y MP3 de una misma toma. La equivalencia de PCM y la similitud perceptual serán clases separadas en fases posteriores.

No modificar, mover, renombrar, normalizar ni eliminar originales durante escaneo, clasificación o preescucha. No escribir etiquetas dentro del audio ni modificar archivos .asd o .als en el MVP. Las etiquetas residen en la base de datos y pueden exportarse a JSON/CSV.

### I. Exportación e integración inicial

Una bandeja de sesión reúne archivos seleccionados y notas. Exportar copias a un destino elegido, sin sobrescribir silenciosamente ni cambiar formato por defecto. Resolver colisiones, comprobar espacio y verificar la copia. Escribir primero a temporales propios y finalizar por renombrado donde el sistema lo permita; no afirmar atomicidad de todo un lote.

Crear un manifiesto JSON con IDs de origen, hashes, nombres de destino y parámetros de transformación, si los hubiera. Una carpeta exportada debe sobrevivir a la limpieza de caché. No entregar a Live rutas temporales que puedan desaparecer.

Arrastre al exterior mediante APIs nativas de archivos; validar el mecanismo en la plataforma objetivo y dentro de Live. En el MVP no automatizar la inserción en pistas ni sincronizar etiquetas con la base de datos interna de Ableton.

### J. Trabajos, concurrencia y rendimiento

Rust coordina trabajos y escrituras de base de datos. Limitar trabajadores; evitar competencia con reproducción y permitir perfiles «trabajar con Live» y «analizar biblioteca». Cola persistente, pausa, cancelación, reintentos limitados y reanudación tras cierre inesperado. No marcar un análisis incompleto como terminado.

Invalidar cachés por contenido y versión de algoritmo. En procesos de audio no realizar consultas SQL ni operaciones bloqueantes. Limitar caché y mostrar espacio consumido. No garantizar carga de CPU constante: medir en el hardware de referencia.

Objetivos iniciales, sujetos a benchmark: 100.000 registros; búsqueda estructurada/textual caliente con p95 inferior a 200 ms; inicio de preescucha de WAV local preparado con p95 inferior a 300 ms; consumo de memoria sin crecimiento proporcional a la duración de un archivo. Documentar hardware, corpus, codec, caché y percentiles. Una prueba con metadatos sintéticos no valida rendimiento de decodificación.

### K. Seguridad, datos y distribución

Limitar acceso a raíces y destinos autorizados. Validar rutas también en comandos Rust personalizados: los permisos del frontend no sustituyen esa validación. Lanzar procesos con arrays de argumentos, sin shell ni interpolación de nombres de archivo. Decoders con timeout, cancelación y errores estructurados. No cargar contenido web remoto privilegiado.

Usar transacciones y migraciones versionadas. Crear copias consistentes de SQLite con su mecanismo de backup o tras cierre coordinado, no copiando sólo el archivo principal cuando haya WAL activo. Probar restauración de etiquetas y colecciones. Los respaldos del catálogo no son respaldos de los audios.

Empaquetar binarios, dependencias y recursos por arquitectura. Documentar firma y notarización de macOS, requisitos de permisos y prueba en un equipo limpio. No publicar, comprar certificados ni inventar una firma si faltan credenciales.

### L. Fases posteriores

Fase 2: estimar BPM y tonalidad con algoritmos verificables, permitir corrección manual y no mostrar probabilidades inventadas. En material arrítmico o atonal permitir no aplicable; exponer candidatos de tempo mitad/doble. No inferir compás sólo a partir de duración y BPM. Distinguir nota de un one-shot y tonalidad de una composición.

Añadir clasificación asistida, regiones, análisis de sonoridad adecuado al material, variantes y similitud con un modelo seleccionado tras medirlo. Mostrar candidatos de clasificación, no hechos incontrovertibles. En consultas españolas verificar soporte del modelo o documentar traducción local; no presuponer multilingüismo.

Fase 3: Link, Link Audio y puentes a Max for Live/Extensions según capacidades de la versión real. Link aporta sincronización; Link Audio añade streaming. Extensions es para tareas discretas, no un motor de procesamiento continuo. Mantener estas integraciones opcionales. El parser de proyectos, si existe, sólo puede afirmar referencias encontradas y alcance del escaneo; nunca asegurar ausencia universal de uso.

### M. Pruebas y aceptación

Pruebas significativas con fixtures generados o redistribuibles: audio corto, largo, silencioso, corrupto, mono/estéreo, Unicode, copias idénticas con distinto nombre, mismo nombre con audio distinto, disco desconectado y archivo modificado durante análisis.

Aceptar fase 1 únicamente si:

1. Se indexa una carpeta real y se recupera el catálogo tras reiniciar.
2. La aplicación permite buscar y escuchar mientras quedan trabajos pendientes.
3. Favoritos, etiquetas y consultas sobreviven al cierre.
4. Se distinguen duplicados exactos de simples coincidencias de nombre.
5. Desconectar un volumen no borra sus anotaciones.
6. Un error de decoder no bloquea el lote ni la interfaz.
7. Una exportación no sobrescribe originales ni cambia muestras sin petición.
8. Se verifica, mediante hashes, que escaneo y etiquetado no han cambiado los archivos de prueba.
9. La restauración del catálogo conserva etiquetas y colecciones.
10. Se documentan rendimiento medido y pruebas manuales de macOS/Live efectivamente realizadas.

### N. Entregables y forma de trabajar

Entrega arquitectura resumida, decisiones y límites, árbol del proyecto, código, migraciones, instrucciones de desarrollo y empaquetado, pruebas, inventario de licencias y tabla de funciones implementadas/pendientes.

Trabaja en incrementos ejecutables: vertical inicial, indexación persistente, búsqueda/etiquetado, preescucha completa, duplicados/exportación y validación. No declares el producto acabado por haber creado una interfaz con datos ficticios. No amplíes el MVP hacia las fases posteriores hasta que los criterios esenciales estén verificados. Si una integración está bloqueada por falta de plataforma o API, documenta el límite y completa las demás funciones autorizadas.

--- FIN DEL PROMPT ---

## 8. Fuentes primarias consultadas

Las funcionalidades y prioridades son propuestas de diseño. Las siguientes fuentes fundamentan las restricciones tecnológicas; deben revisarse de nuevo al comenzar la implementación.

- [Ableton: gestión de archivos y sets](https://www.ableton.com/en/manual/managing-files-and-sets/).
- [Ableton: Collect All and Save](https://help.ableton.com/hc/en-us/articles/209775645-Collect-All-and-Save).
- [Tauri: configuración con SvelteKit](https://v2.tauri.app/start/frontend/sveltekit/).
- [Tauri: capabilities](https://tauri.app/security/capabilities/).
- [FFmpeg: licencias y configuración de componentes](https://www.ffmpeg.org/legal.html).
- [Essentia: licencias](https://essentia.upf.edu/licensing_information.html).
- [Essentia: Music Extractor](https://essentia.upf.edu/streaming_extractor_music.html).
- [Librosa: implementación y documentación del análisis rítmico](https://librosa.org/doc/0.11.0/_modules/librosa/beat.html).
- [LAION: repositorio CLAP](https://github.com/LAION-AI/CLAP).
- [Ableton: Link y Link Audio, documentación para desarrolladores](https://ableton.github.io/link/).
- [Ableton: Link Audio FAQ](https://help.ableton.com/hc/en-us/articles/25425913328924-Link-Audio-FAQ).
- [Cycling '74: Live API](https://docs.cycling74.com/userguide/m4l/live_api_overview/).
- [Ableton: Extensions FAQ y requisitos](https://help.ableton.com/hc/en-us/articles/27303428331420-Ableton-Extensions-FAQ).
- [Ableton: presentación de Extensions SDK](https://www.ableton.com/en/blog/introducing-extensions-sdk/).
