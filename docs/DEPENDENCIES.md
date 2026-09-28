# Dependencias y distribución

El proyecto fija las resoluciones exactas en `package-lock.json` y `src-tauri/Cargo.lock`. Este es un inventario inicial de dependencias directas; no constituye todavía una revisión de todas las obligaciones transitivas ni autorización de distribución.

| Dependencia                               | Uso                                        | Licencia declarada por el proyecto |
| ----------------------------------------- | ------------------------------------------ | ---------------------------------- |
| Tauri / tauri-build / tauri-plugin-dialog | Ventana, IPC, empaquetado, diálogos        | MIT o Apache-2.0                   |
| Svelte / SvelteKit / adapter-static       | Interfaz SPA                               | MIT                                |
| Vite                                      | Desarrollo y build                         | MIT                                |
| TypeScript                                | Tipos en desarrollo                        | Apache-2.0                         |
| rusqlite (`backup`)                       | SQLite y respaldo/restauración consistente | MIT                                |
| SQLite bundled                            | Catálogo y FTS5                            | Dominio público                    |
| Rodio 0.21.1 / CPAL                       | Preescucha nativa                          | MIT o Apache-2.0                   |
| ringbuf 0.4.8                             | FIFO de audio entre worker y salida        | MIT o Apache-2.0                   |
| Symphonia 0.5.5                           | Inspección y decodificación                | MPL-2.0                            |
| serde / serde_json                        | IPC y manifiestos                          | MIT o Apache-2.0                   |
| uuid                                      | Identificadores locales                    | MIT o Apache-2.0                   |
| walkdir                                   | Recorrido de carpetas                      | Unlicense o MIT                    |
| sha2                                      | SHA-256                                    | MIT o Apache-2.0                   |
| tempfile                                  | Temporales y publicación sin sobrescritura | MIT o Apache-2.0                   |
| hound                                     | WAV de pruebas                             | Apache-2.0                         |

Los iconos SVG y los audios de prueba se crean en el proyecto. Sin fuentes, fotos, modelos ni sonidos descargados. No se añaden Essentia, CLAP, Python distribuido ni modelos de IA.

El corpus opcional de códecs se generó con FFmpeg 8.1.2 instalado en el entorno de desarrollo. `scripts/create-codec-fixtures.py` sólo convierte audios propios de prueba; no se distribuye FFmpeg ni se invoca desde la aplicación.

Antes de distribución: generar inventario transitivo/SBOM y avisos, revisar MPL y componentes/códecs, seleccionar FFmpeg/ffprobe y compilar por arquitectura, registrar sus checksums y flags, documentar Developer ID/notarización, y validar la app sin herramientas de desarrollo. La configuración macOS 13 es provisional y no demuestra compatibilidad con ese sistema.

Referencias de implementación consultadas:

- https://v2.tauri.app/start/frontend/sveltekit/
- https://docs.rs/rodio/0.21.1/rodio/
- https://docs.rs/ringbuf/0.4.8/ringbuf/
- https://svelte.dev/docs/kit/adapter-static
- https://github.com/pdeljanov/Symphonia
