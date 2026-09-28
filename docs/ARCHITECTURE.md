# Arquitectura · incremento 05

## Estructura

```text
src/
  lib/api.ts               Contrato de IPC y tipos de la interfaz
  lib/Audition.svelte      Forma de onda, seek y controles de loop A/B
  lib/Icon.svelte          Iconos locales
  routes/+page.svelte      Biblioteca, colecciones, inspector, duplicados, bandeja y transporte
  routes/+layout.ts        SPA sin SSR
  app.css                  CSS nativo
src-tauri/
  src/lib.rs               Arranque y estado administrado
  src/commands.rs          Comandos IPC y diálogos de autorización
  src/catalog.rs           SQLite, escaneo, consultas, colecciones, smart queries, duplicados y copias
  src/playback.rs          Servicio de audio con canal de mensajes
  src/audio_stream.rs      Decoder dedicado y FIFO de tamaño fijo
  src/waveform.rs          Envolvente multinivel y caché por contenido
  migrations/001_catalog.sql
  migrations/002_phase1.sql
  capabilities/main.json
scripts/create-fixtures.py Audios originales de prueba
scripts/create-codec-fixtures.py Corpus opcional de siete formatos
```

## Decisiones

- Se respeta Tauri 2 + SvelteKit 2 + Svelte 5 (runes exclusivos `$state`, `$derived`, `$props`) + adapter-static + CSS nativo de la propuesta. La UI no incorpora servicios, contenido remoto ni telemetría.
- SQLite bundled, migraciones secuenciales versionadas (`001_catalog.sql` -> `002_phase1.sql`), foreign keys, WAL y FTS5. Cada operación abre su conexión; el escaneo no mantiene una transacción sobre todo el lote y las búsquedas pueden ejecutarse mientras avanza.
- Consultas parametrizadas con palabras FTS escapadas y prefijos; sin concatenar texto de usuario a SQL. Debounce de 180 ms y rechazo de respuestas antiguas en la interfaz. Paginación acotada por cursor `(name, id)` con 100 registros por página.
- Una fuente se autoriza mediante diálogo iniciado en Rust. No hay comandos que acepten rutas arbitrarias para leer audio o exportar. Reproducción, Finder y copias resuelven IDs del catálogo y comprueban la ruta canónica dentro de la fuente.
- Las raíces solapadas se rechazan y el recorrido no sigue enlaces simbólicos. Se ignoran entradas ocultas y placeholders macOS `UF_DATALESS`.
- Escaneo en un hilo dedicado con un guard para impedir escaneos simultáneos; publicación de progreso y reescaneo manual incremental por tamaño/mtime. Al finalizar el escaneo, se indexan automáticamente los candidatos a duplicados exactos.
- Los originales se abren en lectura. Un cambio de tamaño/mtime durante la inspección invalida el resultado y se recoge como incidencia; requiere reescaneo manual. La falta de una raíz produce estado offline al consultar; no borra anotaciones.
- Colecciones manuales y consultas inteligentes: tablas `collections`, `collection_items` (con orden y borrado en cascada) y `smart_queries` (filtros JSON guardados). Valoraciones (0–5) en SQLite asociadas al catálogo.
- Duplicados exactos: tablas `contents` y `file_contents`. Algoritmo de dos fases (prefiltrado por tamaño común y cálculo criptográfico SHA-256 en bloques streaming de 64 KiB). Un cambio de tamaño/mtime o desaparición invalida el vínculo de hash; antes de reanalizar se comprueban los vínculos existentes y la estabilidad del archivo durante el hash. Inmutabilidad estricta: los archivos jamás se borran ni modifican; se exponen sus ubicaciones físicas para revelarlas en Finder.

## Motor de audio provisional

Rodio 0.21.1 + Symphonia 0.5.5 prueban salida nativa y seek en la primera vertical. Se fijan en Cargo.lock. El servicio posee la salida y Sink en su propio hilo; el frontend sólo solicita operaciones y consulta estado. Cada fuente decodifica en un worker y entrega frames mono/estéreo a un FIFO SPSC de ringbuf 0.4.8: 16.384 frames, 256 KiB más los buffers del decoder. El consumidor de audio sólo lee el FIFO y actualiza atómicos: no decodifica, accede al disco, espera ni toma mutex propios. Si falta audio emite silencio, conserva la posición y cuenta underruns visibles en la UI. No se precarga el archivo completo ni se transporta por JSON.

El worker vuelve a A al llegar a B, sin acumular en memoria el intervalo. Seek y cambios de loop reconstruyen la fuente preservando pausa y volumen; activar un loop fuera del intervalo coloca la posición en A. Los comandos de loop incluyen fileId para rechazar cambios atrasados de otro archivo. Detener elimina la fuente y el loop. La preparación espera hasta 8 segundos, y su cancelación es cooperativa: no interrumpe una lectura bloqueada por el sistema. La posición publicada se limita a la duración conocida, evitando errores al activar un loop después del final.

**Esta prueba no sustituye la decisión final sobre FFmpeg/ffprobe de la propuesta.** Su distribución con binarios por arquitectura, checksums, flags de compilación y licencias está pendiente. No se reutiliza el FFmpeg de Homebrew del desarrollador dentro del producto. Antes de cerrar el motor, medir latencia, precisión del seek de formatos comprimidos, recuperación del dispositivo y empaquetado en un Mac limpio.

Por ahora se rechaza más de dos canales; no se promete un downmix. No se eligen dispositivos ni se recuperan fallos de salida. No hay procesamiento de volumen perceptual, tempo ni transposición.

## Forma de onda

El análisis se solicita explícitamente para el archivo en preescucha y trabaja fuera de la UI. Calcula SHA-256 completo en bloques de 64 KiB y decodifica PCM por paquetes para obtener extremos de todos los canales. El acumulador conserva como máximo 2.048 bins y los fusiona al crecer, generando niveles progresivamente más gruesos; no guarda todo el PCM. La vista usa un nivel de hasta 512 bins, seek accesible y resaltado del intervalo A/B. No es una vista de zoom a nivel de muestra.

La caché vive en el directorio de caché de Tauri, subcarpeta `waveforms`, con versión y hash de contenido en el nombre. Los temporales se publican de forma atómica; una caché inválida se regenera. Se verifican tamaño, tiempos e inode del original durante el análisis y se conservan hasta 128 entradas propias. Cada actualización vuelve a leer el original para calcular el hash.

Sólo se ejecuta un análisis pesado simultáneo; la nueva solicitud invalida la anterior. Cancelación y límite de 120 s se comprueban entre lecturas y paquetes, sin prometer interrupción inmediata de E/S bloqueada. Cambiar de archivo descarta respuestas antiguas. Las regiones de loop son temporales y no alteran archivos ni exportaciones. Aún no hay fades ni garantía de transiciones sin clics.

## Respaldo y restauración

`rusqlite` usa la API de backup de SQLite para crear una copia consistente del catálogo con transacciones ya confirmadas en WAL. Se genera un temporal en la carpeta elegida, se valida versión 2, esquema y `PRAGMA quick_check`, y se publica sin sobrescribir con un nombre UUID. Los audios originales no forman parte del respaldo.

La restauración se prepara desde un archivo elegido mediante diálogo nativo: se verifica y se copia a `catalog-restore-pending.sqlite` dentro del directorio de datos. Al siguiente arranque, antes de abrir el catálogo, se guarda allí una copia de recuperación del catálogo actual y se aplica el respaldo mediante la API de restore de SQLite. Si falla, se intenta recuperar el catálogo previo, se aparta el archivo pendiente y la UI muestra el error de arranque. No se modifica ningún audio. La restauración nativa con el catálogo real del usuario no se ha ejecutado; el ciclo completo se probó con catálogos temporales.

## Exportación

Un diálogo elige destino; se crea una subcarpeta UUID persistente. Cada archivo se lee en bloques de 64 KiB, se escribe en un temporal propio, se calcula SHA-256, se verifica la copia, se revisa el estado de la fuente y se publica sin sobrescribir con `persist_noclobber`. El manifiesto se finaliza de la misma forma. Un fallo parcial deja las copias verificadas y un `export-error.json`; no se promete atomicidad de todo el lote. Se maneja un error de disco lleno, pero aún no hay comprobación previa de espacio libre ni progreso/cancelación de exportación.

El manifiesto contiene la procedencia local y parámetros vacíos de transformación. Se entrega la copia original; la preescucha no altera la exportación. No se automatiza Ableton ni se implementa arrastre HTML simulando arrastre nativo.
