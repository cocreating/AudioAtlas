<script lang="ts">
  import { onMount } from 'svelte';
  import {
    api,
    time,
    type AudioFile,
    type Control,
    type Player,
    type Waveform
  } from './api';
  let {
    file,
    player,
    oncontrol
  }: {
    file: AudioFile;
    player: Player;
    oncontrol: (control: Control) => Promise<boolean>;
  } = $props();
  let waveform = $state.raw<Waveform | null>(null);
  let loading = $state(false);
  let message = $state('');
  let start = $state(0);
  let end = $state(0);
  let applying = $state(false);
  let request = 0;
  let disposed = false;
  const active = $derived(player.fileId === file.id);
  const duration = $derived(player.duration || file.duration || 0);
  const valid = $derived(
    Number.isFinite(start) &&
      Number.isFinite(end) &&
      start >= 0 &&
      end - start >= 0.05 &&
      end <= duration
  );
  const region = $derived(active ? player.loopRegion : null);
  const level = $derived(
    waveform?.levels.find((l) => l.peaks.length <= 512) ?? waveform?.levels[0]
  );
  const seconds = (value: number) => Number(value.toFixed(2));
  const svgPath = $derived.by(() => {
    if (!level || !waveform) return '';
    const totalFrames = waveform.frames;
    return level.peaks
      .map(([low, high], i) => {
        const x = (((i + 0.5) * level.framesPerBin) / totalFrames) * 1000;
        return `M${Math.min(1000, x).toFixed(2)},${(40 - Math.min(1, high) * 35).toFixed(2)}V${(40 - Math.max(-1, low) * 35).toFixed(2)}`;
      })
      .join(' ');
  });
  async function generate() {
    const token = ++request;
    loading = true;
    message = '';
    try {
      const result = await api.waveform(file.id);
      if (!disposed && token === request) waveform = result;
    } catch (e) {
      if (!disposed && token === request) message = String(e);
    } finally {
      if (!disposed && token === request) loading = false;
    }
  }
  async function cancel() {
    ++request;
    try {
      await api.cancelWaveform();
      message = 'Análisis detenido. Puedes volver a generarlo.';
    } catch (e) {
      message = String(e);
    }
    loading = false;
  }
  async function apply(enabled: boolean) {
    applying = true;
    await oncontrol({
      action: 'setLoop',
      value: { fileId: file.id, region: enabled ? { start, end } : null }
    });
    applying = false;
  }
  onMount(() => {
    start = player.loopRegion?.start ?? 0;
    end = player.loopRegion?.end ?? Math.min(4, duration);
    return () => {
      disposed = true;
      ++request;
      if (loading) void api.cancelWaveform().catch(() => {});
    };
  });
</script>

<section class="audition" aria-label="Forma de onda y loop de preescucha">
  <div class="audition-heading">
    <h2>Preescucha <span>{file.name}</span></h2>
    <div>
      {#if loading}<span role="status">Analizando el audio…</span><button
          class="secondary"
          onclick={cancel}>Detener</button
        >{:else}<button class="secondary" onclick={generate}
          >{waveform
            ? 'Actualizar forma de onda'
            : 'Generar forma de onda'}</button
        >{/if}
    </div>
  </div>
  <div class="waveform" class:has-waveform={waveform !== null}>
    <svg viewBox="0 0 1000 80" preserveAspectRatio="none" aria-hidden="true">
      <path class="baseline" d="M0 40H1000" />
      {#if region && duration}<rect
          class="loop-region"
          x={(region.start / duration) * 1000}
          y="0"
          width={((region.end - region.start) / duration) * 1000}
          height="80"
        />{/if}
      <path class="peaks" d={svgPath} />
      {#if active && duration}<path
          class="playhead"
          d={`M${(player.position / duration) * 1000} 0V80`}
        />{/if}
    </svg>
    {#if !waveform}<span class="waveform-empty"
        >{loading
          ? 'Lectura por bloques · puedes seguir escuchando'
          : 'Genera la vista del audio para explorar sus detalles'}</span
      >{/if}
    <label class="visually-hidden" for="waveform-seek"
      >Buscar posición en la forma de onda</label
    >
    <input
      id="waveform-seek"
      type="range"
      min="0"
      max={duration || 1}
      step="0.01"
      value={active ? player.position : 0}
      disabled={!active || !duration}
      aria-valuetext={`${seconds(active ? player.position : 0)} segundos`}
      onchange={(e) =>
        oncontrol({ action: 'seek', value: +e.currentTarget.value })}
    />
  </div>
  <div class="waveform-scale">
    <span>0:00</span><span
      >{waveform
        ? `${waveform.channels === 1 ? 'Mono' : `${waveform.channels} canales`} · ${waveform.sampleRate / 1000} kHz`
        : 'Vista completa'}</span
    ><span>{time(duration || null)}</span>
  </div>
  <form
    class="loop-controls"
    onsubmit={(e) => {
      e.preventDefault();
      if (valid) void apply(true);
    }}
  >
    <span class="loop-label">LOOP A/B</span>
    <label for="loop-start"
      >A <input
        id="loop-start"
        type="number"
        min="0"
        max={duration}
        step="0.01"
        bind:value={start}
        disabled={!active || applying}
      /></label
    >
    <button
      type="button"
      class="marker"
      disabled={!active || applying}
      onclick={() => (start = seconds(player.position))}>Marcar A</button
    >
    <label for="loop-end"
      >B <input
        id="loop-end"
        type="number"
        min="0.05"
        max={duration}
        step="0.01"
        bind:value={end}
        disabled={!active || applying}
      /></label
    >
    <button
      type="button"
      class="marker"
      disabled={!active || applying}
      onclick={() => (end = seconds(player.position))}>Marcar B</button
    >
    <button class="secondary" disabled={!active || !valid || applying}
      >{region ? 'Aplicar intervalo' : 'Activar loop'}</button
    >
    {#if region}<button
        type="button"
        class="secondary"
        disabled={applying}
        onclick={() => apply(false)}>Desactivar loop</button
      >{/if}
    <span class="loop-status"
      >{region
        ? `${region.start.toFixed(2)}–${region.end.toFixed(2)} s · activo`
        : valid
          ? 'Segundos · sólo preescucha'
          : 'B debe superar A al menos 0,05 s'}</span
    >
  </form>
  {#if message}<p class="waveform-message" role="status">{message}</p>{/if}
  {#if player.underrunFrames > 0}<p class="waveform-message">
      La lectura no ha seguido el ritmo de la salida en algún momento. Prueba a
      detener el análisis o usar un disco local.
    </p>{/if}
</section>

<style>
  .audition {
    flex-shrink: 0;
    border-top: 1px solid var(--line);
    padding: 12px 26px;
    background: #181d16;
  }
  .audition-heading,
  .audition-heading > div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  h2 {
    font-size: 0.76rem;
    font-weight: 500;
    min-width: 0;
  }
  h2 span {
    font-weight: 400;
    color: #a5b398;
    display: inline-block;
    max-width: 30ch;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    vertical-align: bottom;
    margin-left: 10px;
  }
  .audition-heading > div > span {
    font-size: 0.7rem;
    color: var(--muted);
  }
  .audition-heading button {
    min-height: 28px;
    padding: 4px 9px;
  }
  .waveform {
    height: 64px;
    position: relative;
    margin-top: 10px;
    border: 1px solid #46513a;
    border-radius: 4px;
    background: #141912;
  }
  svg {
    width: 100%;
    height: 100%;
    display: block;
  }
  .baseline {
    stroke: #48543d;
    stroke-width: 1;
  }
  .peaks {
    stroke: #b6cc96;
    stroke-width: 1.6;
    vector-effect: non-scaling-stroke;
  }
  .playhead {
    stroke: #f1f4e9;
    stroke-width: 1.5;
  }
  .loop-region {
    fill: #89b558;
    fill-opacity: 0.17;
    stroke: #9ac471;
    stroke-width: 1;
  }
  .waveform-empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 0.72rem;
    color: #a6b797;
    pointer-events: none;
  }
  .waveform input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    margin: 0;
    cursor: crosshair;
  }
  .waveform:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .waveform-scale {
    display: flex;
    justify-content: space-between;
    color: #9cac8d;
    font-size: 0.6rem;
    margin: 4px 0 8px;
    font-variant-numeric: tabular-nums;
  }
  .loop-controls {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 0.7rem;
  }
  .loop-label {
    letter-spacing: 1px;
    color: #c0d3a8;
    margin-right: 5px;
    font-size: 0.6rem;
  }
  label {
    display: inline-flex;
    gap: 5px;
    align-items: center;
  }
  input[type='number'] {
    width: 74px;
    padding: 4px 6px;
    min-height: 29px;
    font-variant-numeric: tabular-nums;
  }
  .loop-controls button {
    min-height: 29px;
    padding: 4px 8px;
    font-size: 0.7rem;
  }
  .marker {
    border: 1px solid #48583b;
  }
  .loop-status {
    color: #adbf9b;
    font-size: 0.64rem;
  }
  .waveform-message {
    color: #d8b997;
    font-size: 0.72rem;
    margin-top: 8px;
  }
</style>
