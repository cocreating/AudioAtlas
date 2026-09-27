# Avance y continuidad

Actualizado: 27 de septiembre de 2026.

## Incremento 03 · Fase 1: modelo de datos consolidado, colecciones y ratings

Implementado en backend y migración (Paso 1):

- Migración transaccional `002_phase1.sql`: colecciones manuales ordenadas (`collections`, `collection_items` con cascade), consultas inteligentes (`smart_queries`), campos `rating` (0–5) y `status` ('pending'/'listened'/'discarded') en `annotations`, `exclusions` y `volume_uuid` en `roots`, y `contents`/`file_contents` para duplicados exactos.
- Backend en Rust: CRUD de colecciones, adición y remoción atómica de ítems, guardado y eliminación de smart queries, filtrado por colección y por valoración mínima en `catalog.query()`.
- Exposición de comandos IPC Tauri en `commands.rs` y registro en `lib.rs`.
- Contrato tipado en `src/lib/api.ts` con nuevos modelos y métodos de biblioteca.
- Pruebas automatizadas: 10 pruebas unitarias pasando, incluyendo prueba de actualización transaccional de un catálogo existente v1 a v2, comprobación de defaults, adición a colección y query filtrada por `min_rating` y `collection_id`.

Implementado en UI (Paso 2):

- Svelte 5 con runes exclusivos (`$state`, `$derived`, sin stores de Svelte 4):
  - Barra lateral: sección "COLECCIONES" con creación inline mediante formulario accesible, lista con badge de recuento de sonidos y borrado; sección "CONSULTAS GUARDADAS" con presets de filtros guardados y eliminación.
  - Barra de herramientas: filtro dropdown por valoración mínima (⭐ Todas, ⭐ 1+, ⭐ 2+, ⭐ 3+, ⭐ 4+, ⭐ 5+) y botón "Guardar búsqueda" que almacena la consulta actual con sus filtros.
  - Lista de sonidos: insignia de estrellas en cada fila indicando la valoración asignada.
  - Panel inspector: selector interactivo de valoración de 1 a 5 estrellas (con alternancia a 0 para borrar valoración) y selector dropdown de pertenencia a colecciones para añadir o quitar el sonido seleccionado.
  - Estado vacío personalizado según la vista activa (colección vacía o búsqueda guardada sin resultados).
- Typecheck (`npm run check`): 0 errores y 0 advertencias (con acción `use:focusOnMount` en inputs inline para cumplimiento a11y).
- Formateo (`npm run format`): Prettier y `cargo fmt` ejecutados.
- Bundle de macOS (`Audio Atlas.app`, 39,15 MiB) generado y ejecutado en el sistema para prueba local.

## Incremento 02 · waveform, loop A/B y audio desacoplado (histórico)

Implementado y retomado tras el corte de la sesión:

- Decoder en worker con FIFO fijo de 16.384 frames (256 KiB), sin lectura de disco ni decodificación en el consumidor de audio. Silencio y contador visible cuando falta audio.
- Forma de onda real bajo demanda: extremos por bloques, hasta 2.048 bins por nivel, SHA-256 completo, caché versionada regenerable y máximo 128 entradas. Cancelación y límite de 120 s cooperativos.
- Panel de preescucha con seek sobre la onda, puntos A/B en segundos o desde la posición actual, activación/desactivación y región resaltada. Intervalo mínimo de 0,05 s; no modifica originales ni exportaciones.
- Loop en el worker, sin almacenar toda la región; pausa y seek preservan el intervalo. Se rechazan comandos de otro archivo. Stop limpia fuente y loop.
- Generador opcional y ejemplo de verificación para siete formatos, sin dependencia de FFmpeg en la aplicación.

Verificado en esta entrega: 9 pruebas Rust, Svelte/TypeScript sin errores ni advertencias, Clippy sin advertencias, rustfmt, build estático y bundle macOS debug de 38,27 MiB. El flujo nativo comprueba waveform de 180 s, caché y repetición/seek/pausa del loop con salida a volumen cero. Las muestras WAV, AIFF, FLAC, MP3, M4A, AAC y OGG pasan apertura, waveform, seek e inicio de loop; no demuestra exactitud ni transiciones sin clics para todos los códecs.

La UI se revisó en **Audio Atlas QA**, con catálogo aislado: onda real visible, bandeja/reproductor fijos, activación después de EOF y repetición mantenida al reanudar desde cero. La prueba detectó y corrigió dos bordes: posición del decoder ligeramente mayor que la duración al terminar y pérdida del loop al reanudar desde cero. QA queda pausada a volumen cero. La aplicación principal del usuario permanece intacta; para usar la compilación nueva debe cerrar y abrir su bundle cuando le convenga.

No se ha realizado escucha humana ni medición p95. La cancelación no interrumpe una lectura del sistema bloqueada. Fades, salida/dispositivos, decisión FFmpeg y regiones persistentes siguen pendientes.

## Incremento 01 · primera vertical (histórico)

Implementado:

- Base Tauri 2, Rust, SvelteKit y CSS nativo; interfaz en español, sin datos ficticios.
- Diálogo para autorizar fuentes, recorrido recursivo y reescaneo manual incremental por tamaño/mtime. Cancelación entre archivos y progreso visible.
- Catálogo SQLite persistente, metadatos, FTS5, búsqueda por nombre/ruta relativa/etiquetas/notas, filtros por fuente/formato/favoritos y páginas de 100 registros por cursor.
- Favoritos, etiquetas y notas persistentes. Protección de cambios sin guardar al cambiar de sonido.
- Preescucha nativa, pausa, seek, volumen, anterior/siguiente y atajos locales.
- Bandeja temporal de sesión y exportación de copias verificadas con manifiesto JSON; acción Mostrar en Finder.
- Bundle macOS arm64 local de desarrollo y documentación de arquitectura, dependencias y límites.

## Evidencia de validación del incremento 01 (histórico)

Estas comprobaciones corresponden al incremento desarrollado en esta conversación, no a una nueva ejecución durante la preparación del commit:

| Comprobación | Resultado observado |
|---|---|
| Svelte/TypeScript | 0 errores y 0 advertencias |
| Build estático y bundle macOS | Correctos; aplicación de aproximadamente 38 MiB |
| Pruebas Rust | 3 satisfactorias: persistencia/exportación/offline/corruptos, autorización de rutas y paginación/FTS |
| Clippy con advertencias como errores | Sin advertencias |
| Flujo aislado con 7 fixtures | Un corrupto aislado; etiquetas recuperadas al reabrir SQLite; hashes originales y copias iguales |
| Motor nativo a volumen cero | Inicio, seek observado a 60,16 s, pausa, reanudación y detención correctos |
| App nativa y diálogo | Apertura, árbol de accesibilidad y revisión visual realizados |
| Fuente real elegida por el usuario | La UI mostró 2.341 archivos con metadatos; dato observado en esa sesión, no un contador fijo ni un benchmark |
| Auditoría npm de ejecución | Sin vulnerabilidades informadas; avisos bajos en herramientas de desarrollo pendientes de revisión |

No se validaron escucha humana, integración con Live, todos los formatos comprimidos, recuperación ante desconexión de audio, compatibilidad macOS 13, equipo limpio ni objetivos de rendimiento. La recuperación de etiquetas se probó al reabrir SQLite; falta el ciclo manual completo de cierre/arranque de UI.

La última compilación incluye el ajuste para mantener bandeja y reproductor visibles. Se dejó la app del usuario abierta; esa instancia puede requerir cerrar y abrir para cargar el bundle actualizado. La revisión visual posterior de ese ajuste queda pendiente.

## Límites actuales

- El escaneo no tiene watcher, cola persistente, timeout/cancelación dentro del decoder, exclusiones configurables, retirada de fuentes ni identidad de volumen UUID.
- Hay paginación acotada, no tabla virtualizada ni cancelación real de SQL.
- Faltan fades, selección/recuperación de salida, downmix multicanal y regiones persistentes. Waveform y loop temporal están implementados en el incremento 02.
- Faltan colecciones, consultas guardadas, rating, persistencia de bandeja, duplicados exactos y backup/restauración.
- La exportación todavía no comprueba espacio antes del lote ni muestra progreso/cancelación.
- La distribución de FFmpeg/ffprobe, el inventario transitivo de licencias, la firma/notarización y el arrastre nativo a Live siguen pendientes.

## Próximo trabajo recomendado

1. Completar la validación manual de la vertical: escucha, seek, guardar anotaciones, reiniciar UI, recuperarlas y exportar desde la interfaz. Usar material de prueba para no alterar anotaciones del usuario.
2. Confirmar precisión de seek/loop en formatos comprimidos, escucha, fades y salida/dispositivos; decidir y documentar la estrategia de FFmpeg.
3. Ampliar casos de análisis cancelado, cambios concurrentes y lectura lenta antes de aceptar los objetivos de rendimiento.
4. Añadir cola persistente, incidencias detalladas y gestión de fuentes/volúmenes.
5. Continuar con colecciones, duplicados, backup/restauración y arrastre nativo probado en Live. Medir rendimiento antes de aceptar fase 1.

## Preparación de commits
 
Repositorio local en `main` sincronizado con su remoto en `https://github.com/cocreating/AudioAtlas`. El commit inicial `d531b40` corresponde a la primera vertical (incremento 01) y el commit `7749b7a` consolida el incremento 02 con waveform real multinivel, loop A/B y decodificación desacoplada en worker FIFO.
 
El detalle técnico y la matriz completa se mantienen en [docs/STATUS.md](../../docs/STATUS.md) y [docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md). La [propuesta original](Audio-Atlas-propuesta-y-prompt-tecnico.md) sigue siendo la referencia de alcance.
