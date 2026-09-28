<script lang="ts">
  let { name, size = 18 }: { name: string; size?: number } = $props();
  const paths: Record<string, string> = {
    library: 'M4 4h6v16H4z M14 4h6v16h-6z M4 8h6 M14 16h6',
    folder: 'M3 7V5h6l2 2h10v13H3z',
    plus: 'M12 5v14 M5 12h14',
    search: 'M21 21l-5-5 M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0',
    heart:
      'M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8L12 21l8.8-8.6a5.5 5.5 0 0 0 0-7.8z',
    tray: 'M4 4h16l2 12v4H2v-4z M2 16h6l2 2h4l2-2h6',
    arrow: 'M12 4v12 M7 11l5 5 5-5 M4 16v5h16v-5',
    play: 'M8 4l13 8-13 8z',
    pause: 'M8 4v16 M16 4v16',
    next: 'M5 5l10 7-10 7z M19 5v14',
    previous: 'M19 5L9 12l10 7z M5 5v14',
    volume: 'M11 4L5 9H2v6h3l6 5z M15 8a6 6 0 0 1 0 8 M18 4a11 11 0 0 1 0 16',
    refresh:
      'M20 7v5h-5 M4 17v-5h5 M5 7a8 8 0 0 1 13-2l2 3 M4 16l2 3a8 8 0 0 0 13-2',
    close: 'M6 6l12 12 M18 6L6 18',
    file: 'M5 2h9l5 5v15H5z M14 2v6h5 M8 14v4 M12 11v9 M16 14v4',
    check: 'M5 12l4 4L20 5',
    external: 'M14 3h7v7 M21 3L10 14 M10 3H3v18h18v-7',
    info: 'M12 11v6 M12 7h.01 M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0',
    star: 'M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z',
    bookmark: 'M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z',
    trash:
      'M3 6h18 M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2',
    tag: 'M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z M7 7h.01',
    copy: 'M8 4h10a2 2 0 0 1 2 2v10 M4 8h10a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V10a2 2 0 0 1 2-2z'
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"><path d={paths[name] ?? paths.file} /></svg
>
