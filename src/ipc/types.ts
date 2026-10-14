// Shared types mirrored between the React UI and the Rust core (core/session, core/*).
// Keep in sync with src-tauri/src/session/state.rs.

export type SessionState =
  | "idle"
  | "recording"
  | "processing"
  | "inserting"
  | "completed"
  | "cancelled"
  | "failed";

export interface SessionSnapshot {
  state: SessionState;
  /** Input level 0..1 while recording (for the meter/waveform). */
  level: number;
  /** Human-readable detail for failed/cancelled, else null. */
  message: string | null;
  /** Elapsed recording ms, when applicable. */
  elapsedMs: number;
}

/** Result of a text-insertion attempt. Never "inserted" when only copied. */
export type InsertOutcome = "inserted" | "copied-only" | "failed";

export interface InsertResult {
  outcome: InsertOutcome;
  message: string | null;
}

export type DataLocality = "local" | "network";

export interface ModelInfo {
  id: string;
  name: string;
  purpose: "speech" | "cleanup" | "vocabulary" | "embeddings" | "translation" | "diarization";
  locality: DataLocality;
  sizeBytes: number | null;
  languages: string[];
  license: string;
  source: string;
  installed: boolean;
  selected: boolean;
  /** Measured speed on this machine (real-time factor), null until benchmarked. */
  measuredRtf: number | null;
  integrityVerified: boolean;
}

export interface HistoryEntry {
  id: string;
  createdAt: string; // ISO8601
  /** Raw recognizer output — never overwritten by AI edits. */
  rawTranscript: string;
  /** Optional AI-edited version, stored separately. null in P0. */
  editedText: string | null;
  durationMs: number;
  language: string | null;
}

export interface Settings {
  language: string; // "auto" or BCP-47
  punctuation: boolean;
  autoCapitalize: boolean;
  removeFillerWords: boolean; // cleanup off by default
  storeTranscripts: boolean; // false disables history storage entirely
  retentionDays: number | null; // null = keep forever
  preferGpu: boolean;
  modelDir: string | null;
  selectedSpeechModelId: string | null;
  toggleHotkey: string;
  pushToTalkHotkey: string | null;
  cancelHotkey: string | null;
  offlineMode: boolean; // true = block all optional network
}
