# Estado del desarrollo · 28 de septiembre de 2026

## Entrega actual

Incremento 05: reconciliación de duplicados tras cambios en archivos y respaldo/restauración consistente del catálogo. **La fase 1 completa sigue en desarrollo.**

| Función                                                       | Estado                                                                                                                                                                                         |
| ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tauri 2 + SvelteKit + TypeScript + CSS nativo                 | Implementado y compilado                                                                                                                                                                       |
| Diálogo de carpetas, escaneo recursivo, catálogo SQLite       | Implementado; carpeta real observada en UI con 2.341 registros                                                                                                                                 |
| Persistencia, metadatos y FTS5                                | Verificado con fixtures WAV y reapertura de base de datos                                                                                                                                      |
| Favoritos, etiquetas y notas                                  | Implementado y verificado en backend y UI                                                                                                                                                      |
| Búsqueda, fuente, favorito y formato                          | Implementado; cursor de 100 elementos y debounce                                                                                                                                               |
| Preescucha nativa, pausa, seek, volumen                       | Motor verificado con salida real a volumen cero                                                                                                                                                |
| Navegación anterior/siguiente y atajos locales                | Implementado; revisión manual exhaustiva pendiente                                                                                                                                             |
| Exportación de copias y manifiesto SHA-256                    | Verificado con fixtures; sin cambios en fuentes ni sobrescrituras                                                                                                                              |
| Fuentes offline, errores de decoder y rutas Unicode           | Pruebas automatizadas satisfactorias                                                                                                                                                           |
| Mostrar en Finder                                             | Implementado; validación de la acción manual pendiente                                                                                                                                         |
| Empaquetado macOS                                             | Bundle debug local generado, sin Developer ID ni notarización                                                                                                                                  |
| Reinicio completo de UI y recuperación de anotaciones         | Reapertura de SQLite probada; ciclo manual de cierre/arranque verificado                                                                                                                       |
| FFmpeg/ffprobe distribuidos                                   | Pendiente; decoder provisional Rust autocontenido                                                                                                                                              |
| Forma de onda real, caché regenerable y cancelación           | Implementado; picos, invalidación y límites probados; UI nativa revisada                                                                                                                       |
| Loop A/B y decoder separado de la salida                      | Implementado; repetición, seek en pausa y activación tras EOF verificados                                                                                                                      |
| Fades, dispositivo de salida y recuperación                   | Pendiente                                                                                                                                                                                      |
| Watcher, exclusiones, retirar fuentes y volumen UUID          | Schema preparado en v2 (`exclusions`, `volume_uuid`); watcher/gestión pendiente                                                                                                                |
| Cola persistente, pausa real, reintentos, timeout del decoder | Pendiente; cancelación entre archivos y reescaneo manual disponibles                                                                                                                           |
| Colecciones, consultas guardadas, rating                      | Implementado en backend (migración v2) y UI (sidebar, toolbar, tabla e inspector); 11 pruebas Rust ok                                                                                          |
| Duplicados exactos                                            | Implementado; cambio, desaparición, reaparición y reanálisis verificados con prueba de regresión |
| Backup/restauración consistente                               | Implementado con SQLite backup/restore; probado con catálogos temporales, UI nativa visible                                                                                                                                                                                      |
| Arrastre nativo a Ableton Live                                | Pendiente de implementación y prueba; Live está instalado                                                                                                                                      |
| Benchmarks de 100.000 registros y p95                         | Pendiente; no se declara alcanzado ningún objetivo de rendimiento                                                                                                                              |

## Pruebas ejecutadas

Hardware/plataforma: macOS 26.6.2 (25G83), arm64. Node 25.6.0, npm 11.8.0, Rust/Cargo 1.95.

- `npm run check`: 0 errores, 0 advertencias.
- `npm run build`: SPA estática generada correctamente.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`: revisión sin advertencias.
- Analizador oficial de Svelte: sin problemas en los componentes revisados; sugerencia opcional sobre `bind:this` conservado para Cmd+F.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 13 pruebas satisfactorias: duplicados exactos y cambios/desapariciones/reapariciones; respaldo y restauración con WAL, anotaciones y colecciones; migración v1 a v2; catálogo, autorización y paginación; loop WAV, seek y FIFO; picos reales, caché e invalidación, envolvente acotada y audio corrupto.
- `cargo run --manifest-path src-tauri/Cargo.toml --example verify_vertical -- test-fixtures/sonidos --audio`: 7 archivos, incluido uno corrupto, anotaciones recuperadas tras reapertura, SHA-256 idéntico antes/después de exportar. Audio nativo iniciado, seek observado a 60,14 s, pausa/reanudación/detención correctos. Waveform de 7.938.000 frames con seis niveles y reutilización de caché. Loop nativo repetido, rechazo de fileId antiguo, seek pausado y desactivación correctos. Volumen cero: no valida calidad audible ni latencia.
- `cargo run --manifest-path src-tauri/Cargo.toml --example verify_codecs -- test-fixtures/codecs`: las siete muestras WAV/AIFF/FLAC/MP3/M4A/AAC/OGG pasan waveform, apertura, seek e inicio de loop. Prueba de compatibilidad básica, no cobertura de todas las variantes ni exactitud de seek comprimido.
- `npm run tauri -- build --debug --bundles app`: bundle arm64 actualizado (40,06 MiB).
- Apertura real de la app, lectura del árbol de accesibilidad y captura visual. Diálogo de carpetas operativo. El usuario seleccionó su propia fuente; la UI mostró 2.341 archivos y metadatos (migrados limpiamente a v2 conservando todas las anotaciones existentes).
- UI del incremento 04: vista "Duplicados" en la barra lateral con recuento dinámico, barra resumen con cálculo de espacio redundante en disco, insignias en la lista de sonidos e inspección de todas las ubicaciones físicas del archivo con acción "Mostrar en Finder", preservando la inmutabilidad de los originales.
- Auditoría npm de dependencias de ejecución: sin vulnerabilidades informadas.

Las pruebas automáticas usan catálogos temporales y archivos propios. La bandeja del usuario y su catálogo no se usan para las pruebas de anotaciones/exportación. No se ha realizado una escucha humana, prueba dentro de Live, benchmark formal ni validación en un Mac limpio.

Los hashes se invalidan al reescanear un archivo modificado o desaparecido. Reanalizar descarta vínculos cuyo archivo ya no coincide con el catálogo; requiere reescaneo de la fuente para registrar la nueva versión del archivo.

## Próximo incremento

1. Validar manualmente el flujo completo de restauración en el paquete QA aislado y las vistas de colecciones, valoración, consultas y exportación.
2. Añadir gestión de fuentes, exclusiones y cola persistente.
3. Probar el arrastre nativo hacia Ableton Live y Finder.
4. Resolver fades, selección/recuperación de salida y decisión FFmpeg; medir rendimiento antes de cerrar formalmente la fase 1.
