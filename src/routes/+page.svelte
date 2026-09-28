<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { listen } from '@tauri-apps/api/event';
  import Icon from '$lib/Icon.svelte';
  import Audition from '$lib/Audition.svelte';
  import {
    api,
    native,
    time,
    bytes,
    type Library,
    type AudioFile,
    type Collection,
    type SmartQuery,
    type DuplicateLocation,
    type DuplicateSummary,
    type Cursor,
    type Player,
    type Progress,
    type Control
  } from '$lib/api';

  let desktop = $state(false);
  let data = $state<Library>({
    roots: [],
    files: [],
    collections: [],
    smartQueries: [],
    total: 0,
    favorites: 0,
    duplicates: 0,
    matched: 0,
    next: null,
    scanning: false
  });
  let search = $state('');
  let searchInput: HTMLInputElement;
  let view = $state<'all' | 'favorites' | 'tray' | 'collection' | 'duplicates'>(
    'all'
  );
  let selectedDuplicates = $state<DuplicateLocation[]>([]);
  let duplicateSummary = $state<DuplicateSummary | null>(null);
  let scanningDuplicates = $state(false);
  let rootId = $state<string | null>(null);
  let activeCollectionId = $state<string | null>(null);
  let format = $state('');
  let minRating = $state(0);
  let selected = $state<AudioFile | null>(null);
  let playingFile = $state<AudioFile | null>(null);
  let player = $state<Player>({
    fileId: null,
    playing: false,
    position: 0,
    duration: 0,
    volume: 0.6,
    loopRegion: null,
    underrunFrames: 0,
    error: null
  });
  let tray = $state<AudioFile[]>([]);
  let busy = $state(false);
  let saving = $state(false);
  let loading = $state(false);
  let error = $state('');
  let notice = $state('');
  let tags = $state('');
  let notes = $state('');
  let rating = $state(0);
  let progress = $state<Progress | null>(null);
  let cursor = $state<Cursor | null>(null);
  let history = $state<(Cursor | null)[]>([]);
  let request = 0;
  let timer: ReturnType<typeof setTimeout>;
  let polling = false;

  // Estados de interfaz para Colecciones y Smart Queries
  let creatingCollection = $state(false);
  let newCollectionName = $state('');
  let selectedCollectionToAdd = $state('');
  let savingQuery = $state(false);
  let newQueryName = $state('');

  function focusOnMount(node: HTMLElement) {
    node.focus();
  }

  const dirty = $derived(
    selected !== null &&
      (tags !== selected.tags.join(', ') ||
        notes !== selected.notes ||
        rating !== selected.rating)
  );
  const files = $derived(
    view === 'tray'
      ? tray.filter(
          (f) =>
            `${f.name} ${f.tags.join(' ')} ${f.notes}`
              .toLowerCase()
              .includes(search.toLowerCase()) &&
            (!format || f.format === format) &&
            (!minRating || f.rating >= minRating)
        )
      : data.files
  );
  const title = $derived(
    view === 'favorites'
      ? 'Tus favoritos'
      : view === 'tray'
        ? 'Bandeja de sesión'
        : view === 'duplicates'
          ? 'Archivos duplicados'
          : view === 'collection'
            ? (data.collections.find((c) => c.id === activeCollectionId)
                ?.name ?? 'Colección')
            : (data.roots.find((r) => r.id === rootId)?.name ??
              'Todos los sonidos')
  );

  async function refresh(after: Cursor | null = cursor) {
    if (!desktop) return;
    const token = ++request;
    loading = true;
    try {
      const result = await api.library({
        text: search,
        rootId: view === 'collection' || view === 'duplicates' ? null : rootId,
        favorites: view === 'favorites',
        format: format || null,
        collectionId: view === 'collection' ? activeCollectionId : null,
        minRating: minRating > 0 ? minRating : null,
        duplicatesOnly: view === 'duplicates',
        after
      });
      if (token === request) {
        data = result;
        if (selected) {
          const current = result.files.find((f) => f.id === selected?.id);
          if (current && !dirty) {
            selected = current;
            rating = current.rating;
          }
        }
      }
    } catch (e) {
      if (token === request) error = String(e);
    } finally {
      if (token === request) loading = false;
    }
  }
  function searchChanged() {
    clearTimeout(timer);
    cursor = null;
    history = [];
    timer = setTimeout(() => void refresh(), 180);
  }
  function navigate(
    next: typeof view,
    root: string | null = null,
    collectionId: string | null = null
  ) {
    view = next;
    rootId = root;
    activeCollectionId = collectionId;
    cursor = null;
    history = [];
    if (next === 'duplicates') {
      void loadDuplicateSummary();
    }
    void refresh();
  }
  async function loadDuplicateSummary() {
    if (!desktop) return;
    try {
      duplicateSummary = await api.duplicateSummary();
    } catch (e) {
      console.error(e);
    }
  }
  async function triggerScanDuplicates() {
    if (!desktop) return;
    scanningDuplicates = true;
    try {
      duplicateSummary = await api.scanDuplicates();
      await refresh(null);
      notice = 'Análisis de duplicados completado.';
    } catch (e) {
      error = String(e);
    } finally {
      scanningDuplicates = false;
    }
  }
  async function loadFileDuplicates(fileId: string) {
    try {
      selectedDuplicates = await api.getFileDuplicates(fileId);
    } catch (e) {
      console.error(e);
    }
  }
  async function run(action: () => Promise<unknown>) {
    error = '';
    try {
      await action();
    } catch (e) {
      error = String(e);
    }
  }
  async function addFolder() {
    busy = true;
    await run(async () => {
      const id = await api.chooseRoot();
      if (id) {
        rootId = id;
        activeCollectionId = null;
        view = 'all';
        cursor = null;
        history = [];
        await refresh();
      }
    });
    busy = false;
  }
  function select(file: AudioFile) {
    if (dirty && selected?.id !== file.id) {
      error =
        'Guarda o descarta los cambios del inspector antes de cambiar de sonido.';
      return;
    }
    selected = file;
    tags = file.tags.join(', ');
    notes = file.notes;
    rating = file.rating;
    selectedDuplicates = [];
    if (file.duplicateCount > 0) {
      void loadFileDuplicates(file.id);
    }
  }
  async function save(favorite = selected?.favorite ?? false) {
    if (!selected || saving) return;
    saving = true;
    const file = selected;
    const annotation = {
      favorite,
      tags: tags
        .split(',')
        .map((t) => t.trim())
        .filter(Boolean),
      notes,
      rating,
      status: file.userStatus
    };
    await run(async () => {
      await api.annotate(file.id, annotation);
      const updated = { ...file, ...annotation, userStatus: file.userStatus };
      if (selected?.id === file.id) {
        selected = updated;
        tags = updated.tags.join(', ');
        notes = updated.notes;
        rating = updated.rating;
      }
      tray = tray.map((f) => (f.id === file.id ? updated : f));
      notice = 'Anotaciones guardadas en tu catálogo.';
      await refresh();
    });
    saving = false;
  }
  async function setRating(stars: number) {
    if (!selected) return;
    rating = rating === stars ? 0 : stars;
    await save();
  }
  async function createCollection() {
    const name = newCollectionName.trim();
    if (!name) return;
    await run(async () => {
      const col = await api.createCollection(name);
      newCollectionName = '';
      creatingCollection = false;
      notice = `Colección "${col.name}" creada.`;
      await refresh();
      navigate('collection', null, col.id);
    });
  }
  async function deleteCollection(id: string, name: string) {
    if (
      !confirm(
        `¿Eliminar la colección "${name}"? Los archivos de audio no se borrarán.`
      )
    )
      return;
    await run(async () => {
      await api.deleteCollection(id);
      notice = 'Colección eliminada.';
      if (activeCollectionId === id) {
        navigate('all');
      } else {
        await refresh();
      }
    });
  }
  async function addSelectedToCollection() {
    if (!selected || !selectedCollectionToAdd) return;
    const targetCol = data.collections.find(
      (c) => c.id === selectedCollectionToAdd
    );
    await run(async () => {
      await api.addToCollection(selectedCollectionToAdd, [selected!.id]);
      notice = `Sonido añadido a "${targetCol?.name ?? 'la colección'}".`;
      selectedCollectionToAdd = '';
      await refresh();
    });
  }
  async function removeSelectedFromCurrentCollection(fileId: string) {
    if (!activeCollectionId) return;
    await run(async () => {
      await api.removeFromCollection(activeCollectionId!, [fileId]);
      notice = 'Sonido quitado de la colección.';
      await refresh();
    });
  }
  async function saveCurrentQuery() {
    const name = newQueryName.trim();
    if (!name) return;
    const filter = {
      text: search,
      format: format || null,
      minRating: minRating > 0 ? minRating : null,
      rootId: view === 'collection' ? null : rootId
    };
    await run(async () => {
      await api.saveSmartQuery(name, JSON.stringify(filter));
      newQueryName = '';
      savingQuery = false;
      notice = `Filtro "${name}" guardado.`;
      await refresh();
    });
  }
  function applySmartQuery(sq: SmartQuery) {
    try {
      const filter = JSON.parse(sq.filterJson);
      search = filter.text ?? '';
      format = filter.format ?? '';
      minRating = filter.minRating ?? 0;
      rootId = filter.rootId ?? null;
      activeCollectionId = null;
      view = 'all';
      cursor = null;
      history = [];
      notice = `Filtro "${sq.name}" aplicado.`;
      void refresh();
    } catch {
      error = 'No se pudo aplicar la consulta guardada.';
    }
  }
  async function deleteSmartQuery(id: string, name: string) {
    if (!confirm(`¿Eliminar la consulta guardada "${name}"?`)) return;
    await run(async () => {
      await api.deleteSmartQuery(id);
      notice = 'Consulta eliminada.';
      await refresh();
    });
  }
  function discard() {
    if (selected) {
      tags = selected.tags.join(', ');
      notes = selected.notes;
      rating = selected.rating;
      error = '';
    }
  }
  async function play(file = selected) {
    if (!file) return;
    await run(async () => {
      await api.play(file.id);
      playingFile = file;
      player = await api.player();
    });
  }
  async function control(command: Control) {
    error = '';
    try {
      await api.control(command);
      player = await api.player();
      return true;
    } catch (e) {
      error = String(e);
      return false;
    }
  }
  async function toggle() {
    if (selected && selected.id !== player.fileId) await play(selected);
    else if (player.fileId && player.playing)
      await control({ action: 'pause' });
    else if (player.fileId) await control({ action: 'resume' });
    else await play(selected ?? playingFile);
  }
  function step(direction: number) {
    const index = files.findIndex(
      (f) => f.id === (selected?.id ?? player.fileId)
    );
    const file =
      files[Math.max(0, Math.min(files.length - 1, index + direction))];
    if (file) {
      select(file);
      if (selected?.id === file.id) void play(file);
    }
  }
  function addToTray(file: AudioFile) {
    if (!tray.some((f) => f.id === file.id)) tray = [...tray, file];
    notice = 'Sonido añadido a la bandeja de esta sesión.';
  }
  async function exportTray() {
    busy = true;
    await run(async () => {
      const result = await api.export(tray.map((f) => f.id));
      if (result)
        notice = `${result.files.length} copias verificadas en ${result.directory}`;
    });
    busy = false;
  }
  function keyboard(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key === 'f') {
      event.preventDefault();
      searchInput?.focus();
      return;
    }
    if (
      event.target instanceof HTMLElement &&
      event.target.closest(
        'input, textarea, select, button, [contenteditable="true"]'
      )
    )
      return;
    if (event.key === ' ' && desktop) {
      event.preventDefault();
      void toggle();
    }
    if (
      (event.key === 'ArrowDown' || event.key === 'ArrowUp') &&
      files.length
    ) {
      event.preventDefault();
      const index = files.findIndex((f) => f.id === selected?.id);
      select(
        files[
          Math.max(
            0,
            Math.min(
              files.length - 1,
              index + (event.key === 'ArrowDown' ? 1 : -1)
            )
          )
        ]
      );
    }
  }
  onMount(() => {
    desktop = native();
    if (!desktop) return;
    let disposed = false;
    const cleanups: (() => void)[] = [];
    async function setup() {
      for (const [name, handler] of [
        [
          'scan-progress',
          (payload: unknown) => {
            progress = payload as Progress;
          }
        ],
        [
          'scan-error',
          (payload: unknown) => {
            error = String(payload);
          }
        ],
        [
          'scan-finished',
          () => {
            void refresh();
          }
        ]
      ] as const) {
        const unlisten = await listen(name, (e) => handler(e.payload));
        if (disposed) unlisten();
        else cleanups.push(unlisten);
      }
      if (!disposed) await refresh();
    }
    void setup().catch((e) => {
      error = String(e);
    });
    const poll = setInterval(async () => {
      if (polling || disposed) return;
      polling = true;
      try {
        if (player.fileId) {
          player = await api.player();
          if (player.error) error = player.error;
        }
        if (data.scanning) await refresh();
      } catch (e) {
        error = String(e);
      } finally {
        polling = false;
      }
    }, 600);
    return () => {
      disposed = true;
      clearInterval(poll);
      clearTimeout(timer);
      cleanups.forEach((fn) => fn());
    };
  });
</script>

<svelte:head
  ><title>Audio Atlas · Tu biblioteca de sonidos</title><meta
    name="description"
    content="Explora, escucha y organiza tu biblioteca de audio local."
  /></svelte:head
>
<svelte:window onkeydown={keyboard} />
<a class="skip-link" href="#library">Ir a la biblioteca</a>
<div class="app-shell">
  <aside class="sidebar" aria-label="Biblioteca y fuentes">
    <a class="brand" href={resolve('/')} aria-label="Audio Atlas, inicio"
      ><span class="brand-mark" aria-hidden="true"
        ><i></i><i></i><i></i><i></i><i></i></span
      ><span
        >audio<span class="brand-light">atlas</span><small
          >UN LUGAR PARA TUS SONIDOS</small
        ></span
      ></a
    >
    <div class="workspace-label">
      <span class="status-dot"></span> Espacio personal
      <span class="local-badge">LOCAL</span>
    </div>
    <p class="nav-label">BIBLIOTECA</p>
    <nav aria-label="Vistas de biblioteca">
      <button
        class:active={view === 'all' && !rootId}
        onclick={() => navigate('all')}
        ><Icon name="library" />Todos los sonidos<span class="nav-count"
          >{data.total}</span
        ></button
      >
      <button
        class:active={view === 'favorites'}
        onclick={() => navigate('favorites')}
        ><Icon name="heart" />Favoritos<span class="nav-count"
          >{data.favorites}</span
        ></button
      >
      <button class:active={view === 'tray'} onclick={() => navigate('tray')}
        ><Icon name="tray" />Bandeja de sesión<span class="nav-count"
          >{tray.length}</span
        ></button
      >
      <button
        class:active={view === 'duplicates'}
        onclick={() => navigate('duplicates')}
        ><Icon name="copy" />Duplicados<span class="nav-count"
          >{data.duplicates}</span
        ></button
      >
    </nav>

    <div class="section-heading">
      <p class="nav-label">COLECCIONES</p>
      <button
        class="icon-button"
        aria-label="Nueva colección"
        title="Crear nueva colección"
        onclick={() => (creatingCollection = !creatingCollection)}
        disabled={!desktop || busy}><Icon name="plus" size={15} /></button
      >
    </div>
    {#if creatingCollection}
      <form
        class="inline-create-box"
        onsubmit={(e) => {
          e.preventDefault();
          void createCollection();
        }}
      >
        <input
          bind:value={newCollectionName}
          placeholder="Nombre de la colección…"
          use:focusOnMount
        />
        <div class="inline-create-actions">
          <button
            type="button"
            class="text-button"
            onclick={() => {
              creatingCollection = false;
              newCollectionName = '';
            }}>Cancelar</button
          >
          <button
            type="submit"
            class="primary"
            disabled={!newCollectionName.trim()}>Crear</button
          >
        </div>
      </form>
    {/if}
    <nav class="collections" aria-label="Colecciones de usuario">
      {#each data.collections as col (col.id)}
        <div class="sidebar-item-row">
          <button
            class:active={view === 'collection' &&
              activeCollectionId === col.id}
            onclick={() => navigate('collection', null, col.id)}
            title={col.name}
          >
            <Icon name="tag" size={16} /><span class="source-name"
              >{col.name}</span
            ><span class="nav-count">{col.count}</span>
          </button>
          <button
            class="icon-button item-del-btn"
            title="Eliminar colección"
            aria-label="Eliminar colección"
            onclick={(e) => {
              e.stopPropagation();
              void deleteCollection(col.id, col.name);
            }}
          >
            <Icon name="trash" size={13} />
          </button>
        </div>
      {:else}
        <p class="source-hint">Organiza listas personalizadas de sonidos.</p>
      {/each}
    </nav>

    {#if data.smartQueries.length}
      <div class="section-heading">
        <p class="nav-label">CONSULTAS GUARDADAS</p>
      </div>
      <nav class="smart-queries" aria-label="Consultas guardadas">
        {#each data.smartQueries as sq (sq.id)}
          <div class="sidebar-item-row">
            <button
              onclick={() => applySmartQuery(sq)}
              title="Aplicar consulta guardada"
            >
              <Icon name="bookmark" size={15} /><span class="query-name"
                >{sq.name}</span
              >
            </button>
            <button
              class="icon-button item-del-btn"
              title="Eliminar consulta"
              aria-label="Eliminar consulta"
              onclick={(e) => {
                e.stopPropagation();
                void deleteSmartQuery(sq.id, sq.name);
              }}
            >
              <Icon name="trash" size={13} />
            </button>
          </div>
        {/each}
      </nav>
    {/if}

    <div class="section-heading">
      <p class="nav-label">TUS FUENTES</p>
      <button
        class="icon-button"
        aria-label="Añadir fuente"
        onclick={addFolder}
        disabled={!desktop || busy || data.scanning}
        ><Icon name="plus" size={15} /></button
      >
    </div>
    <nav class="sources" aria-label="Carpetas de audio">
      {#each data.roots as root (root.id)}
        <button
          class:active={rootId === root.id && view === 'all'}
          onclick={() => navigate('all', root.id)}
          title={root.path}
          ><Icon name="folder" /><span class="source-name"
            >{root.name}<small
              >{root.online ? `${root.count} archivos` : 'Desconectada'}</small
            ></span
          ></button
        >
      {:else}
        <p class="source-hint">Conecta las carpetas donde viven tus sonidos.</p>
      {/each}
    </nav>
    <button
      class="add-source"
      onclick={addFolder}
      disabled={!desktop || busy || data.scanning}
      ><Icon name="plus" size={16} />Añadir carpeta</button
    >
    <div class="sidebar-bottom">
      <div class="privacy-icon"><Icon name="folder" /></div>
      <strong>Tu biblioteca. En tu equipo.</strong>
      <p>
        Los originales permanecen en su sitio. Tú decides cómo organizarlos.
      </p>
      <span class="version">AUDIO ATLAS <span>0.1 · PRIMERA VERSIÓN</span></span
      >
    </div>
  </aside>

  <main id="library" tabindex="-1">
    <header class="topbar">
      <div class="breadcrumb">
        Biblioteca <span>/</span> <strong>{title}</strong>
      </div>
      <span class="connection"
        ><span class="status-dot"></span>{desktop
          ? 'Catálogo local'
          : 'Vista previa web'}</span
      >
    </header>
    <div class="page-heading">
      <div>
        <p class="eyebrow">REDESCUBRE TU COLECCIÓN</p>
        <h1>
          {title}<span class="heading-count"
            >{view === 'tray' ? tray.length : data.matched}</span
          >
        </h1>
        <p>
          {view === 'tray'
            ? 'Reúne los sonidos que quieres llevar a tu próxima sesión.'
            : 'Encuentra ese sonido. Dale una nueva vida.'}
        </p>
      </div>
      <button
        class="primary"
        onclick={addFolder}
        disabled={!desktop || busy || data.scanning}
        ><Icon name="plus" size={17} />Añadir carpeta</button
      >
    </div>
    {#if !desktop}<div class="preview-notice">
        <Icon name="info" size={16} />Vista previa de la interfaz. Abre la
        aplicación de escritorio para conectar tus carpetas y escuchar audio.
      </div>{/if}
    <div class="toolbar">
      <div class="search">
        <Icon name="search" /><label class="visually-hidden" for="search"
          >Buscar sonidos, etiquetas o notas</label
        ><input
          bind:this={searchInput}
          id="search"
          bind:value={search}
          oninput={searchChanged}
          placeholder="Buscar sonidos, etiquetas o notas…"
          autocomplete="off"
        /><kbd>⌘ F</kbd>
      </div>
      <label class="visually-hidden" for="format">Filtrar por formato</label
      ><select id="format" bind:value={format} onchange={searchChanged}
        ><option value="">Todos los formatos</option
        >{#each ['WAV', 'AIFF', 'AIF', 'FLAC', 'MP3', 'M4A', 'AAC', 'OGG'] as item (item)}<option
            >{item}</option
          >{/each}</select
      >
      <label class="visually-hidden" for="rating-filter"
        >Filtrar por valoración</label
      ><select
        id="rating-filter"
        bind:value={minRating}
        onchange={searchChanged}
      >
        <option value={0}>Cualquier puntuación</option>
        <option value={5}>★★★★★ (5 estrellas)</option>
        <option value={4}>★★★★☆ (4+ estrellas)</option>
        <option value={3}>★★★☆☆ (3+ estrellas)</option>
        <option value={2}>★★☆☆☆ (2+ estrellas)</option>
        <option value={1}>★☆☆☆☆ (1+ estrella)</option>
      </select>
      {#if search || format || minRating > 0}
        <button
          class="secondary save-query-btn"
          title="Guardar esta búsqueda como consulta inteligente"
          onclick={() => (savingQuery = !savingQuery)}
        >
          <Icon name="bookmark" size={14} />Guardar búsqueda
        </button>
      {/if}
      {#if rootId}<button
          class="icon-button refresh"
          title="Volver a escanear esta fuente"
          aria-label="Volver a escanear esta fuente"
          disabled={data.scanning}
          onclick={() =>
            run(async () => {
              await api.rescan(rootId!);
              await refresh();
            })}><Icon name="refresh" /></button
        >{/if}
    </div>
    {#if savingQuery}
      <form
        class="inline-create-box"
        style="margin: -8px 0 16px 0;"
        onsubmit={(e) => {
          e.preventDefault();
          void saveCurrentQuery();
        }}
      >
        <input
          bind:value={newQueryName}
          placeholder="Nombre del filtro guardado…"
          use:focusOnMount
        />
        <div class="inline-create-actions">
          <button
            type="button"
            class="text-button"
            onclick={() => {
              savingQuery = false;
              newQueryName = '';
            }}>Cancelar</button
          >
          <button type="submit" class="primary" disabled={!newQueryName.trim()}
            >Guardar filtro</button
          >
        </div>
      </form>
    {/if}
    {#if view === 'duplicates' && duplicateSummary}
      <div class="duplicate-summary-bar">
        <div class="duplicate-summary-stats">
          <span
            ><strong>{duplicateSummary.duplicateFilesCount}</strong> archivos duplicados</span
          >
          <span>•</span>
          <span
            ><strong>{duplicateSummary.duplicateGroupsCount}</strong> grupos de contenido</span
          >
          <span>•</span>
          <span
            ><strong>{bytes(duplicateSummary.wastedBytes)}</strong> de espacio duplicado</span
          >
        </div>
        <button
          type="button"
          class="secondary"
          style="padding: 4px 10px; font-size: 0.78rem;"
          onclick={triggerScanDuplicates}
          disabled={scanningDuplicates}
        >
          <Icon name="refresh" size={13} />
          {scanningDuplicates ? 'Analizando…' : 'Reanalizar duplicados'}
        </button>
      </div>
    {/if}
    {#if error}<div class="message error" role="alert">
        <span>{error}</span><button
          class="icon-button"
          aria-label="Cerrar error"
          onclick={() => (error = '')}><Icon name="close" size={16} /></button
        >
      </div>{/if}
    <div class="notice" role="status">{notice}</div>
    {#if data.scanning}<div class="scan-status">
        <span class="status-dot"></span><span
          >Indexando · {progress?.indexed ?? 0} archivos {progress?.errors
            ? `· ${progress.errors} incidencias`
            : ''}</span
        ><button onclick={() => run(api.cancel)}>Detener</button>
      </div>{/if}
    <div class="library-layout">
      <section class="file-area" aria-label="Sonidos" aria-busy={loading}>
        <div class="list-caption">
          <span
            >{view === 'tray'
              ? 'SELECCIÓN DE LA SESIÓN'
              : 'ARCHIVOS DE AUDIO'}</span
          ><span
            >{files.length
              ? `${files.length} en esta página`
              : 'Tu próximo hallazgo empieza aquí'}</span
          >
        </div>
        {#if files.length}
          <div class="table-scroll">
            <table>
              <caption class="visually-hidden"
                >Archivos de audio de tu biblioteca</caption
              ><thead
                ><tr
                  ><th scope="col">Nombre</th><th scope="col">Formato</th><th
                    scope="col">Duración</th
                  ><th scope="col" class="channels-column">Canales</th><th
                    scope="col"
                    ><span class="visually-hidden">Añadir a sesión</span></th
                  ></tr
                ></thead
              ><tbody>
                {#each files as file (file.id)}
                  <tr class:selected={selected?.id === file.id}
                    ><td
                      ><button
                        class="file-select"
                        onclick={() => select(file)}
                        ondblclick={() => play(file)}
                        ><span
                          class="file-icon"
                          class:is-playing={player.fileId === file.id &&
                            player.playing}
                          ><Icon
                            name={player.fileId === file.id && player.playing
                              ? 'volume'
                              : 'file'}
                          /></span
                        ><span
                          ><strong>{file.name}</strong
                          >{#if file.rating > 0}<span
                              class="rating-badge"
                              title="{file.rating} estrellas"
                              >{'★'.repeat(file.rating)}</span
                            >{/if}{#if file.duplicateCount > 0}<span
                              class="badge-duplicate"
                              title="{file.duplicateCount +
                                1} copias idénticas en la biblioteca"
                              ><Icon name="copy" size={11} />
                              {file.duplicateCount + 1}</span
                            >{/if}<small
                            >{file.status !== 'ready'
                              ? ({
                                  offline: 'Fuente desconectada',
                                  missing: 'Archivo no encontrado',
                                  unsupported: 'No compatible'
                                }[file.status] ?? file.status)
                              : file.tags.length
                                ? file.tags.join(' · ')
                                : file.relativePath}</small
                          ></span
                        >{#if file.favorite}<span
                            class="favorite-dot"
                            aria-label="Favorito">♥</span
                          >{/if}</button
                      ></td
                    ><td><span class="format-badge">{file.format}</span></td><td
                      class="mono">{time(file.duration)}</td
                    ><td class="channels-column muted"
                      >{file.channels === 1
                        ? 'Mono'
                        : file.channels === 2
                          ? 'Estéreo'
                          : (file.channels ?? '—')}</td
                    ><td
                      ><button
                        class="icon-button"
                        aria-label={view === 'tray'
                          ? `Quitar ${file.name} de la bandeja`
                          : `Añadir ${file.name} a la bandeja`}
                        onclick={() =>
                          view === 'tray'
                            ? (tray = tray.filter((f) => f.id !== file.id))
                            : addToTray(file)}
                        ><Icon
                          name={view === 'tray'
                            ? 'close'
                            : tray.some((f) => f.id === file.id)
                              ? 'check'
                              : 'plus'}
                          size={16}
                        /></button
                      ></td
                    ></tr
                  >
                {/each}
              </tbody>
            </table>
          </div>
          {#if view !== 'tray'}<div class="pagination">
              <button
                disabled={!history.length}
                onclick={() => {
                  cursor = history[history.length - 1];
                  history = history.slice(0, -1);
                  void refresh();
                }}>Anterior</button
              ><span>Página {history.length + 1}</span><button
                disabled={!data.next}
                onclick={() => {
                  history = [...history, cursor];
                  cursor = data.next;
                  void refresh();
                }}>Siguiente</button
              >
            </div>{/if}
        {:else}
          <div class="empty-state">
            <div class="atlas-art" aria-hidden="true">
              <div class="orbit orbit-one"></div>
              <div class="orbit orbit-two"></div>
              <div class="orbit orbit-three"></div>
              <span class="orbit-point"></span>
              <div class="audio-symbol">
                <span></span><span></span><span></span><span></span><span
                ></span><span></span><span></span>
              </div>
            </div>
            <p class="eyebrow">
              {data.total || view !== 'all'
                ? 'ESPACIO PARA DESCUBRIR'
                : 'TODO EMPIEZA CON UN SONIDO'}
            </p>
            <h2>
              {search || format || minRating
                ? 'No encontramos coincidencias'
                : view === 'favorites'
                  ? 'Los sonidos que quieras volver a escuchar'
                  : view === 'tray'
                    ? 'Prepara tu próxima sesión'
                    : view === 'duplicates'
                      ? 'No se han detectado duplicados'
                      : view === 'collection'
                        ? 'Colección vacía'
                        : 'Tu universo sonoro, en un solo lugar.'}
            </h2>
            <p>
              {search || format || minRating
                ? 'Prueba otra búsqueda o cambia los filtros.'
                : view === 'favorites'
                  ? 'Marca un sonido con el corazón del inspector para guardarlo aquí.'
                  : view === 'tray'
                    ? 'Añade sonidos desde la biblioteca y exporta una selección de copias.'
                    : view === 'duplicates'
                      ? 'Todos los archivos analizados en tu biblioteca tienen contenidos únicos.'
                      : view === 'collection'
                        ? 'Añade sonidos a esta colección desde el inspector lateral.'
                        : 'Conecta una carpeta para explorar, escuchar y organizar tus audios, sin mover los originales.'}
            </p>
            {#if !search && !format && !minRating && view === 'all'}<button
                class="primary"
                onclick={addFolder}
                disabled={!desktop || busy || data.scanning}
                ><Icon name="folder" size={17} />Conectar mi primera carpeta</button
              ><span class="supported"
                >WAV · AIFF · FLAC · MP3 · M4A · AAC · OGG*</span
              ><small class="compatibility"
                >* Compatibilidad según el códec del archivo.</small
              >{:else if search || format || minRating}<button
                class="secondary"
                onclick={() => {
                  search = '';
                  format = '';
                  minRating = 0;
                  searchChanged();
                }}>Limpiar filtros</button
              >{/if}
          </div>
        {/if}
      </section>
      <aside class="inspector" aria-label="Inspector del sonido">
        <div class="inspector-title">
          INSPECTOR <Icon name="info" size={15} />
        </div>
        {#if selected}<div class="selected-icon">
            <Icon name="file" size={28} />
          </div>
          <h2>{selected.name}</h2>
          <p class="file-path">{selected.relativePath}</p>
          <div class="inspector-actions">
            <button
              class="secondary"
              onclick={() => play(selected)}
              disabled={selected.status !== 'ready'}
              ><Icon name="play" size={15} />Escuchar</button
            ><button
              class="icon-button"
              class:favorite={selected.favorite}
              aria-label="Favorito"
              aria-pressed={selected.favorite}
              disabled={saving}
              onclick={() => save(!selected!.favorite)}
              ><Icon name="heart" /></button
            >
          </div>
          <div class="rating-selector" role="group" aria-label="Puntuación">
            <span class="rating-label">Puntuación</span>
            <div class="stars">
              {#each [1, 2, 3, 4, 5] as star}
                <button
                  type="button"
                  class="star-btn"
                  class:filled={star <= rating}
                  aria-label="{star} estrellas"
                  onclick={() => setRating(star)}
                >
                  ★
                </button>
              {/each}
              {#if rating > 0}
                <button
                  type="button"
                  class="text-button clear-rating"
                  onclick={() => setRating(0)}
                >
                  Quitar
                </button>
              {/if}
            </div>
          </div>
          <dl>
            <div>
              <dt>Formato</dt>
              <dd>{selected.format} / {selected.codec ?? '—'}</dd>
            </div>
            <div>
              <dt>Duración</dt>
              <dd>{time(selected.duration)}</dd>
            </div>
            <div>
              <dt>Frecuencia</dt>
              <dd>
                {selected.sampleRate
                  ? `${selected.sampleRate / 1000} kHz`
                  : '—'}
              </dd>
            </div>
            <div>
              <dt>Profundidad</dt>
              <dd>
                {selected.bitDepth
                  ? `${selected.bitDepth} bit`
                  : 'No disponible'}
              </dd>
            </div>
            <div>
              <dt>Tamaño</dt>
              <dd>{bytes(selected.size)}</dd>
            </div>
          </dl>
          {#if selected.error}<p class="decoder-error">{selected.error}</p>{/if}
          <form
            onsubmit={(e) => {
              e.preventDefault();
              void save();
            }}
          >
            <label for="tags">Etiquetas</label><input
              id="tags"
              bind:value={tags}
              placeholder="ambiente, oscuro, textura"
            />
            <p class="input-hint">Separadas por comas.</p>
            <label for="notes">Notas</label><textarea
              id="notes"
              bind:value={notes}
              rows="3"
              placeholder="Una idea para este sonido…"></textarea><button
              class="secondary save-button"
              disabled={!dirty || saving}
              >{saving ? 'Guardando…' : 'Guardar cambios'}</button
            >{#if dirty}<button
                type="button"
                class="text-button"
                onclick={discard}>Descartar cambios</button
              >{/if}
          </form>
          {#if data.collections.length}
            <div class="add-to-collection-block">
              <label for="collection-select">Colección</label>
              <div class="collection-add-row">
                <select
                  id="collection-select"
                  bind:value={selectedCollectionToAdd}
                >
                  <option value="">Añadir a una colección…</option>
                  {#each data.collections as col (col.id)}
                    <option value={col.id}>{col.name}</option>
                  {/each}
                </select>
                <button
                  type="button"
                  class="secondary"
                  disabled={!selectedCollectionToAdd}
                  onclick={addSelectedToCollection}
                >
                  Añadir
                </button>
              </div>
            </div>
          {/if}
          {#if view === 'collection' && activeCollectionId}
            <button
              type="button"
              class="text-button remove-col-btn"
              onclick={() => removeSelectedFromCurrentCollection(selected!.id)}
            >
              <Icon name="close" size={13} />Quitar de esta colección
            </button>
          {/if}
          {#if selectedDuplicates.length > 0}
            <div class="duplicate-box">
              <div class="duplicate-box-header">
                <span>Ubicaciones idénticas</span>
                <span class="badge"
                  >{selectedDuplicates.length}
                  {selectedDuplicates.length === 1 ? 'copia' : 'copias'}</span
                >
              </div>
              <div class="duplicate-list">
                {#each selectedDuplicates as dup (dup.fileId)}
                  <div class="duplicate-item">
                    <div class="duplicate-item-info">
                      <span class="duplicate-item-path" title={dup.fullPath}
                        >{dup.relativePath}</span
                      >
                      <span class="duplicate-item-root"
                        >Fuente: {dup.rootName} ({bytes(dup.size)})</span
                      >
                    </div>
                    <button
                      type="button"
                      class="icon-button"
                      title="Mostrar copia en Finder"
                      onclick={() => run(() => api.reveal(dup.fileId))}
                    >
                      <Icon name="external" size={13} />
                    </button>
                  </div>
                {/each}
              </div>
              <p class="duplicate-notice">
                Hash SHA-256 verificado en streaming. Los archivos originales
                son de sólo lectura y no se modifican.
              </p>
            </div>
          {/if}
          <button
            class="text-button reveal"
            onclick={() => run(() => api.reveal(selected!.id))}
            ><Icon name="external" size={14} />Mostrar en Finder</button
          >{:else}<div class="inspector-empty">
            <div class="outline-icon"><Icon name="file" size={26} /></div>
            <h2>Cada sonido tiene una historia</h2>
            <p>
              Selecciona un archivo para ver sus detalles, añadir etiquetas y
              guardar tus ideas.
            </p>
            <div class="inspector-tip">
              <span>UN PEQUEÑO CONSEJO</span>
              <p>Una etiqueta hoy puede ser una gran idea mañana.</p>
            </div>
          </div>{/if}
      </aside>
    </div>
    {#if playingFile}
      {#key playingFile.id}
        <Audition file={playingFile} {player} oncontrol={control} />
      {/key}
    {/if}
    <div class="session-bar">
      <Icon name="tray" /><strong>Bandeja de sesión</strong><span
        >{tray.length
          ? `${tray.length} sonidos preparados`
          : 'Guarda aquí los sonidos para tu próximo proyecto'}</span
      ><button
        class="secondary"
        disabled={!tray.length || busy}
        onclick={exportTray}
        ><Icon name="arrow" size={15} />{busy
          ? 'Espera…'
          : 'Exportar copias'}</button
      >
    </div>
  </main>
  <footer class="player" aria-label="Reproductor de audio">
    <div class="now-playing">
      <span class="player-art"><Icon name="file" size={23} /></span>
      <div>
        <strong>{playingFile?.name ?? 'Un mundo por escuchar'}</strong><span
          >{playingFile
            ? `${playingFile.format} · Preescucha original`
            : 'Selecciona un sonido para empezar'}</span
        >
      </div>
    </div>
    <div class="transport">
      <div class="transport-buttons">
        <button
          class="icon-button"
          disabled={!files.length || !desktop}
          aria-label="Sonido anterior"
          onclick={() => step(-1)}><Icon name="previous" size={17} /></button
        ><button
          class="play-button"
          disabled={(!selected && !playingFile) || !desktop}
          aria-label={player.playing ? 'Pausar' : 'Reproducir'}
          onclick={toggle}
          ><Icon name={player.playing ? 'pause' : 'play'} size={20} /></button
        ><button
          class="icon-button"
          disabled={!files.length || !desktop}
          aria-label="Sonido siguiente"
          onclick={() => step(1)}><Icon name="next" size={17} /></button
        >
      </div>
      <div class="seek">
        <span>{time(player.position)}</span><label
          class="visually-hidden"
          for="seek">Posición de reproducción</label
        ><input
          id="seek"
          type="range"
          min="0"
          max={player.duration || 1}
          step="0.01"
          value={player.position}
          disabled={!player.fileId || !player.duration}
          onchange={(e) =>
            control({ action: 'seek', value: +e.currentTarget.value })}
        /><span>{time(player.duration)}</span>
      </div>
    </div>
    <div class="volume">
      <Icon name="volume" size={17} /><label
        class="visually-hidden"
        for="volume">Volumen de preescucha</label
      ><input
        id="volume"
        type="range"
        min="0"
        max="1"
        step="0.01"
        value={player.volume}
        disabled={!desktop}
        onchange={(e) =>
          control({ action: 'volume', value: +e.currentTarget.value })}
      /><span>{Math.round(player.volume * 100)}%</span>
    </div>
  </footer>
</div>
