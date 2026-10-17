// Shared types mirrored between the React UI and the Rust core.
// Keep in sync with src-tauri/src/{session,insert,dictation,models,storage,audio,asr}.

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
  /** While recording: true once the mic has delivered audio (Bluetooth headsets can take 1-3 s). */
  micReady: boolean;
  /** Session id; increments on every start. */
  generation: number;
}

/** Result of a text-insertion attempt. Never "inserted" when only copied. */
export type InsertOutcome = "inserted" | "copied-only" | "failed";

export interface InsertResult {
  outcome: InsertOutcome;
  /** Adapter that produced the outcome: "typed" | "clipboard-paste" | "clipboard-copy" | "none". */
  method: string;
  message: string | null;
}

export interface DictationMetrics {
  startLatencyMs: number;
  /** Recording start → first samples from the device; speech before this is lost. */
  firstAudioMs: number | null;
  /** The mic was still open from the previous dictation ("keep microphone ready"). */
  micWasReady: boolean;
  audioMs: number;
  modelLoadMs: number;
  transcribeMs: number;
  stopToTranscriptMs: number;
  stopToInsertMs: number;
  realTimeFactor: number;
  gpu: boolean;
}

export interface DictationResult {
  historyId: string | null;
  /** Recognizer output exactly as produced (never AI-edited). */
  transcript: string;
  language: string | null;
  insert: InsertResult;
  metrics: DictationMetrics;
}

export type DataLocality = "local" | "network";

export interface ModelInfo {
  id: string;
  name: string;
  purpose: "speech" | "cleanup" | "vocabulary" | "embeddings" | "translation" | "diarization";
  locality: DataLocality;
  sizeBytes: number;
  memoryMb: number;
  languages: string[];
  license: string;
  source: string;
  sha256: string;
  installed: boolean;
  path: string | null;
  selected: boolean;
  integrityVerified: boolean;
  /** Measured speed on this machine (real-time factor), null until benchmarked. */
  measuredRtf: number | null;
  measuredBackend: string | null;
  /** A file the user put in the model folder: no pinned checksum, format-checked only. */
  userAdded: boolean;
}

export interface AsrCapabilities {
  gpuCompiled: boolean;
  /** GPU API in this build ("Vulkan" / "CUDA"), null for CPU-only builds. */
  gpuBackend: string | null;
  gpuActive: boolean;
  streaming: boolean;
  multilingual: boolean;
}

export interface InputDevice {
  id: string;
  name: string;
  isDefault: boolean;
}

export interface HistoryEntry {
  id: string;
  createdAt: string; // ISO8601
  /** Raw recognizer output — never overwritten by AI edits. */
  rawTranscript: string;
  /** User- or AI-edited version, stored separately from the raw transcript. */
  editedText: string | null;
  durationMs: number;
  language: string | null;
}

export interface Settings {
  language: string; // "auto" or a whisper language code
  punctuation: boolean;
  autoCapitalize: boolean;
  removeFillerWords: boolean; // cleanup off by default
  storeTranscripts: boolean; // false disables history storage entirely
  retentionDays: number | null; // null = keep forever
  preferGpu: boolean;
  inputDeviceId: string | null; // null = system default microphone
  keepMicReadySecs: number; // 0 = release mic after each dictation; 4294967295 = until quit
  closeToTray: boolean; // closing the window keeps Voxelle running in the tray
  modelDir: string | null;
  selectedSpeechModelId: string | null;
  toggleHotkey: string;
  pushToTalkHotkey: string | null;
  cancelHotkey: string | null;
  offlineMode: boolean; // true = block all optional network
}

export interface StorageInfo {
  /** false = database could not be opened; nothing is written to disk this session. */
  persistent: boolean;
  dataDir: string;
  modelDir: string;
}

/** Which shortcut is being recorded (src-tauri/src/hotkeys/mod.rs `Role`). */
export type ShortcutRole = "toggle" | "pushToTalk" | "cancel";

/** Shortcut recorder updates (src-tauri/src/hotkeys/capture.rs `CaptureUpdate`). */
export type ShortcutCaptureUpdate =
  | { kind: "progress"; keys: string }
  | { kind: "done"; shortcut: string }
  | { kind: "rejected"; reason: string }
  /** No key for a while: listening stopped quietly; nothing changed. */
  | { kind: "ended" }
  /** Esc on its own: the user backed out. */
  | { kind: "dismissed" };
