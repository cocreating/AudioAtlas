# Estado del desarrollo · 27 de septiembre de 2026

## Entrega actual

Primera vertical funcional, con aplicación macOS arm64 de desarrollo compilada. **La fase 1 completa sigue en desarrollo.**

| Función                                                       | Estado                                                                      |
| ------------------------------------------------------------- | --------------------------------------------------------------------------- |
| Tauri 2 + SvelteKit + TypeScript + CSS nativo                 | Implementado y compilado                                                    |
| Diálogo de carpetas, escaneo recursivo, catálogo SQLite       | Implementado; carpeta real observada en UI con 2.341 registros              |
| Persistencia, metadatos y FTS5                                | Verificado con fixtures WAV y reapertura de base de datos                   |
| Favoritos, etiquetas y notas                                  | Implementado y verificado en backend                                        |
| Búsqueda, fuente, favorito y formato                          | Implementado; cursor de 100 elementos y debounce                            |
| Preescucha nativa, pausa, seek, volumen                       | Motor verificado con salida real a volumen cero                             |
| Navegación anterior/siguiente y atajos locales                | Implementado; revisión manual exhaustiva pendiente                          |
| Exportación de copias y manifiesto SHA-256                    | Verificado con fixtures; sin cambios en fuentes ni sobrescrituras           |
| Fuentes offline, errores de decoder y rutas Unicode           | Pruebas automatizadas satisfactorias                                        |
| Mostrar en Finder                                             | Implementado; validación de la acción manual pendiente                      |
| Empaquetado macOS                                             | Bundle debug local generado, sin Developer ID ni notarización               |
| Reinicio completo de UI y recuperación de anotaciones         | Reapertura de SQLite probada; ciclo manual de cierre/arranque aún pendiente |
| FFmpeg/ffprobe distribuidos                                   | Pendiente; decoder provisional Rust autocontenido                           |
| Waveform, loop A/B, fades, dispositivo de salida              | Pendiente                                                                   |
| Watcher, exclusiones, retirar fuentes y volumen UUID          | Pendiente                                                                   |
| Cola persistente, pausa real, reintentos, timeout del decoder | Pendiente; cancelación entre archivos y reescaneo manual disponibles        |
| Colecciones, consultas guardadas, rating, bandeja persistente | Pendiente                                                                   |
| Duplicados exactos                                            | Pendiente; hashes sólo en exportación y pruebas                             |
| Backup/restauración consistente                               | Pendiente                                                                   |
| Arrastre nativo a Ableton Live                                | Pendiente de implementación y prueba; Live está instalado                   |
| Benchmarks de 100.000 registros y p95                         | Pendiente; no se declara alcanzado ningún objetivo de rendimiento           |

## Pruebas ejecutadas

Hardware/plataforma: macOS 26.6.2 (25G83), arm64. Node 25.6.0, npm 11.8.0, Rust/Cargo 1.95.

- `npm run check`: 0 errores, 0 advertencias.
- `npm run build`: SPA estática generada correctamente.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`: revisión sin advertencias.
- Analizador oficial de Svelte: sin problemas en los tres componentes; sugerencia opcional sobre `bind:this` conservado para Cmd+F.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 3 pruebas significativas satisfactorias: persistencia/corruptos/exportaciones/offline, autorización de rutas y solapamiento, y paginación/búsqueda con caracteres especiales.
- `cargo run --manifest-path src-tauri/Cargo.toml --example verify_vertical -- test-fixtures/sonidos --audio`: 7 archivos, incluido uno corrupto, anotaciones recuperadas tras reapertura, SHA-256 idéntico antes/después de exportar. Audio nativo iniciado, seek observado a 60,16 s, pausa/reanudación/detención correctos. Volumen cero: no valida calidad audible ni latencia.
- `npm run tauri -- build --debug --bundles app`: bundle arm64 generado (aprox. 38 MiB).
- Apertura real de la app, lectura del árbol de accesibilidad y captura visual. Diálogo de carpetas operativo. El usuario seleccionó su propia fuente; la UI mostró 2.341 archivos y metadatos. No se intervino en esa selección ni se reinició su aplicación.
- Auditoría npm de dependencias de ejecución: sin vulnerabilidades informadas. El primer install indicó avisos de severidad baja en herramientas de desarrollo; no equivale a una auditoría completa de Rust ni a una certificación de seguridad.

Las pruebas automáticas usan catálogos temporales y archivos propios. La bandeja del usuario y su catálogo no se usan para las pruebas de anotaciones/exportación. No se ha realizado una escucha humana, prueba dentro de Live, benchmark formal ni validación en un Mac limpio.

## Próximo incremento

1. Confirmar el motor de audio con escucha, decodificación desacoplada, dispositivos y formatos comprimidos; cerrar la estrategia FFmpeg.
2. Añadir waveform regenerable y loop A/B, conservando memoria acotada.
3. Cola persistente, detalle de incidencias, fuentes/exclusiones e identificación de volúmenes.
4. Colecciones y consultas guardadas; duplicados exactos por hash completo.
5. Backup/restauración, arrastre nativo probado en Live y mediciones antes de aceptar fase 1.
