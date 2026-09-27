# Contexto del proyecto

Actualizado: 27 de septiembre de 2026.

## Producto y alcance

Audio Atlas es una aplicación de escritorio local para catalogar, buscar, escuchar, etiquetar y reunir audios para composición y trabajo con Ableton Live. Indexa archivos donde están y guarda organización virtual en su catálogo. No mueve ni modifica originales.

La implementación actual es el incremento 01: una primera vertical ejecutable. La fase 1 completa de la propuesta todavía no está aceptada. No ampliar a análisis musical, IA, similitud, Link ni automatización del DAW antes de completar y verificar los fundamentos.

## Preferencias y decisiones vigentes

- Tauri 2 y Rust; SvelteKit 2 + Svelte 5 + TypeScript con adapter-static y SPA sin SSR.
- La propuesta del proyecto elige Svelte para esta interfaz. No introducir React ni frameworks CSS. Estilos propios en CSS nativo.
- macOS Apple Silicon como primer objetivo. Se compiló en macOS 26.6.2 arm64. macOS 13 es sólo el mínimo configurado; no se ha validado allí ni en Intel/Windows.
- SQLite bundled con WAL, FTS5 y migración inicial transaccional. El catálogo reside en el directorio de datos de la aplicación.
- Preescucha provisional: Rodio 0.21.1 y Symphonia 0.5.5, fijados mediante Cargo.lock. Lectura streaming; el frontend recibe estado, no PCM. Sólo mono/estéreo y salida predeterminada.
- FFmpeg/ffprobe empaquetados siguen pendientes. El decoder Rust permite probar la vertical; no supone que se haya cerrado o sustituido el requisito de distribución de la propuesta.
- Sin cuenta, servicios cloud, telemetría ni contenido web remoto. Vite sólo sirve durante el desarrollo; la aplicación empaquetada no levanta un servidor HTTP.

## Mapa del código

| Ruta | Responsabilidad |
|---|---|
| `src/routes/+page.svelte` | Biblioteca, búsqueda, inspector, bandeja y reproductor |
| `src/lib/api.ts` | Contrato IPC y tipos del frontend |
| `src/app.css` | Diseño propio, oscuro y adaptable |
| `src-tauri/src/lib.rs` | Arranque, registro de comandos y estado |
| `src-tauri/src/commands.rs` | IPC, diálogos y coordinación de tareas |
| `src-tauri/src/catalog.rs` | Persistencia, escaneo, consultas, anotaciones, exportación y pruebas |
| `src-tauri/src/playback.rs` | Motor nativo controlado por canal de mensajes |
| `src-tauri/migrations/001_catalog.sql` | roots, files, annotations, FTS y exports |
| `src-tauri/examples/verify_vertical.rs` | Comprobación aislada del catálogo y salida de audio |
| `scripts/create-fixtures.py` | Generación reproducible de audios propios para pruebas |

## Invariantes que deben conservarse

- Fuentes autorizadas mediante diálogo nativo. Los comandos trabajan con IDs y validan la ruta canónica dentro de su raíz.
- No seguir enlaces simbólicos durante el escaneo ni aceptar raíces solapadas. Se omiten entradas ocultas y placeholders macOS UF_DATALESS; otros proveedores cloud aún no están validados.
- Un volumen desconectado conserva registros y anotaciones. No confundir falta de acceso con autorización para borrar datos.
- Anotaciones sólo en SQLite. No escribir en audio, archivos .asd o .als.
- Exportar copias originales a una carpeta duradera elegida por el usuario; temporales propios, verificación SHA-256 y publicación sin sobrescritura. Los errores de lote pueden dejar copias parciales verificadas y un informe.
- No fingir waveform, BPM, tonalidad, similitud ni arrastre nativo. La columna Formato usa la extensión y el inspector muestra además el códec detectado.
- No declarar una prueba silenciosa como escucha validada, una reapertura de SQLite como reinicio completo de la UI, ni un bundle local como distribución notarizada.

## Desarrollo y comprobaciones

Entorno usado: Node 25.6.0, npm 11.8.0, Rust/Cargo 1.95 y Command Line Tools de Xcode.

```sh
npm ci
npm run desktop
npm run check
npm run build
npm run test:native
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run fixtures
cargo run --manifest-path src-tauri/Cargo.toml --example verify_vertical -- test-fixtures/sonidos --audio
npm run tauri -- build --debug --bundles app
```

`--audio` abre la salida real a volumen cero. La prueba usa un catálogo temporal. `npm run dev` muestra sólo la interfaz web; no tiene operaciones de escritorio. `npm run format` aplica Prettier y rustfmt.

Aplicación generada: `src-tauri/target/debug/bundle/macos/Audio Atlas.app`.
Catálogo del usuario: `~/Library/Application Support/studio.audioatlas.desktop/catalog.sqlite`.

SQLite usa WAL: no respaldar sólo el archivo principal mientras la app está abierta. No borrar ni reinicializar el catálogo del usuario para probar código. Usar fixtures y catálogos temporales. No interrumpir su sesión nativa si está interactuando con la aplicación.

## Control de versiones

Repositorio Git local inicializado en `main`. El primer commit reúne la vertical y su documentación. No se ha configurado remoto ni publicado en un servicio externo.

Versionar código, documentación, migraciones, iconos y los dos lockfiles. Excluir dependencias instaladas, builds, target, esquemas generados, fixtures de audio regenerables, artefactos locales, logs, variables privadas y bases de datos. El generador de fixtures sí se versiona.
