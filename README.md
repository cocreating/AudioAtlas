# Audio Atlas

Biblioteca de audio local para macOS. Primera vertical ejecutable de la [propuesta](.agents/docs/Audio-Atlas-propuesta-y-prompt-tecnico.md), no fase 1 completa.

Para retomar el desarrollo, empieza por el [contexto y avance en `.agents/docs`](.agents/docs/README.md).

## Desarrollo

Probado en macOS 26.6.2, Apple Silicon arm64, Node 25.6.0, npm 11.8.0 y Rust 1.95. Requiere Command Line Tools de Xcode. La versión mínima configurada es macOS 13; aún no se ha probado en esa versión, Intel ni Windows.

```sh
npm ci
npm run desktop
```

`npm run dev` abre únicamente la interfaz web en `http://127.0.0.1:1420`: las acciones nativas están desactivadas. No hay una biblioteca ficticia ni un backend web. El servidor de Vite es exclusivo del desarrollo.

```sh
npm run check
npm run build
npm run test:native
npm run fixtures
npm run tauri -- build --debug --bundles app
```

La aplicación de desarrollo queda en `src-tauri/target/debug/bundle/macos/Audio Atlas.app`. El bundle contiene la interfaz, SQLite y el decoder Rust. No necesita Node, Python ni FFmpeg instalado para las funciones actuales. No está firmado con identidad Developer ID ni notarizado; no es un paquete de distribución.

## Primer uso

1. Abre la aplicación de escritorio y pulsa **Añadir carpeta**. Puedes elegir `test-fixtures/sonidos` después de ejecutar `npm run fixtures`.
2. Selecciona un archivo, pulsa **Escuchar** y mueve el control de posición. Doble clic también reproduce. El reproductor permite pausa, volumen y navegación anterior/siguiente.
3. Escribe etiquetas separadas por comas y notas; pulsa **Guardar cambios**. El corazón guarda el favorito y las anotaciones del inspector.
4. Busca por nombre, ruta relativa, etiquetas o notas. Combina fuente, favoritos y formato. Los resultados se consultan en páginas de 100 mediante cursor.
5. Añade archivos con **+** a la bandeja. **Exportar copias** crea una carpeta de sesión duradera, con SHA-256 y `manifest.json`, dentro del destino que elijas. Nunca sobrescribe archivos existentes. La bandeja es temporal y se vacía al cerrar.
6. Cierra y abre la app: fuentes, archivos, favoritos, etiquetas y notas permanecen en SQLite. Una fuente desconectada conserva sus entradas. El botón de reescaneo aparece al seleccionar una fuente.

Atajos: **Cmd+F** busca; **Espacio** reproduce/pausa cuando el foco no está en un control; flechas arriba/abajo seleccionan cuando el foco está fuera de los controles. No se capturan letras ni espacio durante la edición.

## Datos y límites

El catálogo está en `~/Library/Application Support/studio.audioatlas.desktop/catalog.sqlite`. La app no modifica audios ni escribe etiquetas dentro de ellos. SQLite usa WAL: no copies sólo el archivo principal con la app abierta. El backup/restauración coordinados todavía no están implementados.

Se prueban archivos WAV reales generados localmente. Symphonia detecta el contenido y obtiene el códec; WAV, AIFF, FLAC, MP3, AAC, M4A y OGG son candidatos por extensión y dependen de los códecs admitidos. La columna **Formato** refleja la extensión; el inspector muestra además el códec detectado. La validación completa de cada combinación de contenedor y códec está pendiente. No se analiza BPM ni tonalidad.

El motor inicial Rodio/Symphonia lee el archivo en streaming, sin enviar PCM al frontend. Se limita a mono/estéreo; el audio multicanal se rechaza hasta definir el downmix. Usa la salida predeterminada del sistema. Waveform, loop A/B, fades, cambio de dispositivo y recuperación de desconexiones siguen pendientes. La cancelación del escaneo se comprueba entre archivos, no interrumpe una llamada al decoder.

Consulta [estado y validación](docs/STATUS.md), [arquitectura](docs/ARCHITECTURE.md) y [dependencias y licencias](docs/DEPENDENCIES.md).
