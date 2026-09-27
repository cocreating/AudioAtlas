# Arquitectura · incremento 01

## Estructura

```text
src/
  lib/api.ts               Contrato de IPC y tipos de la interfaz
  lib/Icon.svelte          Iconos locales
  routes/+page.svelte      Biblioteca, inspector, bandeja y transporte
  routes/+layout.ts        SPA sin SSR
  app.css                  CSS nativo
src-tauri/
  src/lib.rs               Arranque y estado administrado
  src/commands.rs          Comandos IPC y diálogos de autorización
  src/catalog.rs           SQLite, escaneo, consultas, anotaciones y copias
  src/playback.rs          Servicio de audio con canal de mensajes
  migrations/001_catalog.sql
  capabilities/main.json
scripts/create-fixtures.py Audios originales de prueba
```

## Decisiones

- Se respeta Tauri 2 + SvelteKit 5/TypeScript + adapter-static + CSS nativo de la propuesta. La UI no incorpora servicios, contenido remoto ni telemetría.
- SQLite bundled, migración inicial, foreign keys, WAL y FTS5. Cada operación abre su conexión; el escaneo no mantiene una transacción sobre todo el lote y las búsquedas pueden ejecutarse mientras avanza.
- Consultas parametrizadas con palabras FTS escapadas y prefijos; sin concatenar texto de usuario a SQL. Debounce de 180 ms y rechazo de respuestas antiguas en la interfaz. La cancelación real de SQL no está implementada.
- Resultados por cursor `(name,id)` y 100 filas por página. La virtualización de la tabla y los benchmarks de 100.000 registros son el siguiente incremento de rendimiento.
- Una fuente se autoriza mediante diálogo iniciado en Rust. No hay comandos que acepten rutas arbitrarias para leer audio o exportar. Reproducción, Finder y copias resuelven IDs del catálogo y comprueban la ruta canónica dentro de la fuente.
- Las raíces solapadas se rechazan y el recorrido no sigue enlaces simbólicos. Se ignoran entradas ocultas y placeholders macOS `UF_DATALESS`. Las entradas de otras nubes no están validadas todavía.
- Escaneo en un hilo dedicado con un guard para impedir escaneos simultáneos; publicación de progreso cada 10 candidatos y al finalizar. Sólo reescaneo manual incremental por tamaño/mtime. No hay aún watcher, volumen UUID, exclusiones configurables ni cola persistente.
- Los originales se abren en lectura. Un cambio de tamaño/mtime durante la inspección invalida el resultado y se recoge como incidencia; requiere reescaneo manual. La falta de una raíz produce estado offline al consultar; no borra anotaciones. Falta una vista detallada de incidencias por archivo para permisos y cambios concurrentes.
- El esquema implementa sólo roots, files, annotations, FTS y exports para la primera vertical. Tags normalizados, contents, analyses, collections, jobs, regions y smart_queries se añadirán mediante futuras migraciones, no mediante tablas vacías que aparenten funcionalidad.

## Motor de audio provisional

Rodio 0.21.1 + Symphonia 0.5.5 prueban salida nativa y seek en la primera vertical. Se fijan en Cargo.lock. El servicio posee la salida y Sink en su propio hilo; el frontend sólo solicita operaciones y consulta estado. No se precarga el audio completo ni se transporta por JSON.

**Esta prueba no sustituye la decisión final sobre FFmpeg/ffprobe de la propuesta.** Su distribución con binarios por arquitectura, checksums, flags de compilación y licencias está pendiente. No se reutiliza el FFmpeg de Homebrew del desarrollador dentro del producto. Antes de cerrar el motor, medir decodificación fuera del callback de salida, latencia, seek de formatos comprimidos, recuperación del dispositivo y empaquetado en un Mac limpio.

Por ahora se rechaza más de dos canales; no se promete un downmix. No se eligen dispositivos ni se recuperan fallos de salida. No hay procesamiento de volumen perceptual, tempo ni transposición.

## Exportación

Un diálogo elige destino; se crea una subcarpeta UUID persistente. Cada archivo se lee en bloques de 64 KiB, se escribe en un temporal propio, se calcula SHA-256, se verifica la copia, se revisa el estado de la fuente y se publica sin sobrescribir con `persist_noclobber`. El manifiesto se finaliza de la misma forma. Un fallo parcial deja las copias verificadas y un `export-error.json`; no se promete atomicidad de todo el lote. Se maneja un error de disco lleno, pero aún no hay comprobación previa de espacio libre ni progreso/cancelación de exportación.

El manifiesto contiene la procedencia local y parámetros vacíos de transformación. Se entrega la copia original; la preescucha no altera la exportación. No se automatiza Ableton ni se implementa arrastre HTML simulando arrastre nativo.
