import { invoke, isTauri } from '@tauri-apps/api/core';

export interface Root {
  id: string;
  name: string;
  path: string;
  online: boolean;
  count: number;
}

export interface Collection {
  id: string;
  name: string;
  color: string | null;
  count: number;
  createdAt: string;
  updatedAt: string;
}

export interface SmartQuery {
  id: string;
  name: string;
  filterJson: string;
  createdAt: string;
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
  rating: number;
  userStatus: string;
  duplicateCount: number;
}

export interface DuplicateLocation {
  fileId: string;
  rootId: string;
  rootName: string;
  relativePath: string;
  fullPath: string;
  size: number;
}

export interface DuplicateSummary {
  duplicateFilesCount: number;
  duplicateGroupsCount: number;
  wastedBytes: number;
}

export interface Cursor {
  name: string;
  id: string;
}

export interface Library {
  roots: Root[];
  files: AudioFile[];
  collections: Collection[];
  smartQueries: SmartQuery[];
  total: number;
  favorites: number;
  duplicates: number;
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
  rating?: number;
  status?: string;
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
    collectionId?: string | null;
    minRating?: number | null;
    duplicatesOnly?: boolean;
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
  reveal: (id: string) => invoke<void>('reveal', { id }),

  // Colecciones y Smart Queries
  createCollection: (name: string, color?: string | null) =>
    invoke<Collection>('create_collection', { name, color }),
  renameCollection: (id: string, name: string, color?: string | null) =>
    invoke<void>('rename_collection', { id, name, color }),
  deleteCollection: (id: string) => invoke<void>('delete_collection', { id }),
  addToCollection: (collectionId: string, fileIds: string[]) =>
    invoke<number>('add_to_collection', { collectionId, fileIds }),
  removeFromCollection: (collectionId: string, fileIds: string[]) =>
    invoke<void>('remove_from_collection', { collectionId, fileIds }),
  saveSmartQuery: (name: string, filterJson: string) =>
    invoke<SmartQuery>('save_smart_query', { name, filterJson }),
  deleteSmartQuery: (id: string) => invoke<void>('delete_smart_query', { id }),

  // Duplicados exactos
  scanDuplicates: () => invoke<DuplicateSummary>('scan_duplicates'),
  getFileDuplicates: (fileId: string) =>
    invoke<DuplicateLocation[]>('get_file_duplicates', { fileId }),
  duplicateSummary: () => invoke<DuplicateSummary>('duplicate_summary')
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
