import { invoke, isTauri } from '@tauri-apps/api/core';
export interface Root {
  id: string;
  name: string;
  path: string;
  online: boolean;
  count: number;
}
export interface AudioFile {
  id: string;
  rootId: string;
  name: string;
  relativePath: string;
  size: number;
  format: string;
  codec: string | null;
  duration: number | null;
  sampleRate: number | null;
  channels: number | null;
  bitDepth: number | null;
  status: string;
  error: string | null;
  favorite: boolean;
  tags: string[];
  notes: string;
}
export interface Cursor {
  name: string;
  id: string;
}
export interface Library {
  roots: Root[];
  files: AudioFile[];
  total: number;
  favorites: number;
  matched: number;
  next: Cursor | null;
  scanning: boolean;
}
export interface LoopRegion {
  start: number;
  end: number;
}
export interface Waveform {
  version: number;
  sha256: string;
  sampleRate: number;
  channels: number;
  frames: number;
  levels: { framesPerBin: number; peaks: [number, number][] }[];
}
export interface Player {
  fileId: string | null;
  playing: boolean;
  position: number;
  duration: number;
  volume: number;
  loopRegion: LoopRegion | null;
  underrunFrames: number;
  error: string | null;
}
export interface Progress {
  indexed: number;
  errors: number;
  skipped: number;
  done: boolean;
  canceled: boolean;
  current: string;
}
export interface Annotation {
  favorite: boolean;
  tags: string[];
  notes: string;
}
export interface ExportResult {
  directory: string;
  files: { sourceId: string; destinationName: string; sha256: string }[];
}
export type Control =
  | { action: 'pause' | 'resume' | 'stop' }
  | { action: 'seek' | 'volume'; value: number }
  | { action: 'setLoop'; value: { fileId: string; region: LoopRegion | null } };
export const native = isTauri;
export const api = {
  library: (query: {
    text: string;
    rootId: string | null;
    favorites: boolean;
    format: string | null;
    after: Cursor | null;
  }) => invoke<Library>('library', { query }),
  chooseRoot: () => invoke<string | null>('choose_root'),
  rescan: (rootId: string) => invoke<void>('rescan', { rootId }),
  cancel: () => invoke<void>('cancel_scan'),
  annotate: (id: string, annotation: Annotation) =>
    invoke<void>('annotate', { id, annotation }),
  play: (id: string) => invoke<void>('play', { id }),
  control: (control: Control) => invoke<void>('transport', { control }),
  player: () => invoke<Player>('player_state'),
  export: (ids: string[]) =>
    invoke<ExportResult | null>('export_files', { ids }),
  waveform: (id: string) => invoke<Waveform>('waveform', { id }),
  cancelWaveform: () => invoke<void>('cancel_waveform'),
  reveal: (id: string) => invoke<void>('reveal', { id })
};
export function time(seconds: number | null): string {
  if (seconds == null) return '—';
  return `${Math.floor(seconds / 60)}:${Math.floor(seconds % 60)
    .toString()
    .padStart(2, '0')}`;
}
export const bytes = (size: number) =>
  size < 1048576
    ? `${(size / 1024).toFixed(0)} KB`
    : `${(size / 1048576).toFixed(1)} MB`;
