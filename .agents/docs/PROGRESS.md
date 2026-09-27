# Avance y continuidad

Actualizado: 27 de septiembre de 2026.

## Incremento 01 · primera vertical

Implementado:

- Base Tauri 2, Rust, SvelteKit y CSS nativo; interfaz en español, sin datos ficticios.
- Diálogo para autorizar fuentes, recorrido recursivo y reescaneo manual incremental por tamaño/mtime. Cancelación entre archivos y progreso visible.
- Catálogo SQLite persistente, metadatos, FTS5, búsqueda por nombre/ruta relativa/etiquetas/notas, filtros por fuente/formato/favoritos y páginas de 100 registros por cursor.
- Favoritos, etiquetas y notas persistentes. Protección de cambios sin guardar al cambiar de sonido.
- Preescucha nativa, pausa, seek, volumen, anterior/siguiente y atajos locales.
- Bandeja temporal de sesión y exportación de copias verificadas con manifiesto JSON; acción Mostrar en Finder.
- Bundle macOS arm64 local de desarrollo y documentación de arquitectura, dependencias y límites.

## Evidencia de validación

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
- Faltan waveform, loop A/B, fades, selección/recuperación de salida y downmix multicanal.
- Faltan colecciones, consultas guardadas, rating, persistencia de bandeja, duplicados exactos y backup/restauración.
- La exportación todavía no comprueba espacio antes del lote ni muestra progreso/cancelación.
- La distribución de FFmpeg/ffprobe, el inventario transitivo de licencias, la firma/notarización y el arrastre nativo a Live siguen pendientes.

## Próximo trabajo recomendado

1. Completar la validación manual de la vertical: escucha, seek, guardar anotaciones, reiniciar UI, recuperarlas y exportar desde la interfaz. Usar material de prueba para no alterar anotaciones del usuario.
2. Confirmar el motor con formatos comprimidos, salida/dispositivos y decodificación desacoplada; decidir y documentar la estrategia de FFmpeg.
3. Implementar waveform regenerable y loop A/B manteniendo memoria acotada.
4. Añadir cola persistente, incidencias detalladas y gestión de fuentes/volúmenes.
5. Continuar con colecciones, duplicados, backup/restauración y arrastre nativo probado en Live. Medir rendimiento antes de aceptar fase 1.

## Preparación del primer commit

Repositorio local creado en `main`, usando la identidad Git ya configurada. Se añaden esta documentación de continuidad y reglas de exclusión para datos locales y variables privadas. El commit inicial no publica la aplicación ni crea un remoto.

El detalle técnico y la matriz completa se mantienen en [docs/STATUS.md](../../docs/STATUS.md) y [docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md). La [propuesta original](Audio-Atlas-propuesta-y-prompt-tecnico.md) sigue siendo la referencia de alcance.
