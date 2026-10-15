// Typed bridge to the Rust core over Tauri IPC.
// The renderer has no OS/Node access; every capability goes through a validated core command.
// In a plain browser (vite preview / vitest) there is no core: calls reject with CoreNotConnected,
// which the UI renders as an honest "core not connected" state rather than faking data.

import type {
  AsrCapabilities,
  DictationResult,
  InputDevice,
  HistoryEntry,
  ModelInfo,
  SessionSnapshot,
  Settings,
  ShortcutCaptureUpdate,
  StorageInfo,
} from "./types";

export class CoreNotConnected extends Error {
  constructor() {
    super("Voxelle core not connected (running outside the desktop app).");
    this.name = "CoreNotConnected";
  }
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) throw new CoreNotConnected();
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

/** Subscribe to session snapshots pushed by the core. Returns an unsubscribe fn. */
export async function onSessionUpdate(
  handler: (snap: SessionSnapshot) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<SessionSnapshot>("session_update", (e) => handler(e.payload));
  return unlisten;
}

/** Subscribe to finished dictations (transcript + insertion outcome + timings). */
export async function onDictationResult(
  handler: (r: DictationResult) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<DictationResult>("dictation_result", (e) => handler(e.payload));
}

/** Updates from the shortcut recorder (see `core.startShortcutCapture`). */
export async function onShortcutCapture(
  handler: (u: ShortcutCaptureUpdate) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<ShortcutCaptureUpdate>("shortcut_capture", (e) => handler(e.payload));
}

export const core = {
  getSessionSnapshot: () => call<SessionSnapshot>("get_session_snapshot"),
  startDictation: () => call<void>("start_dictation"),
  stopDictation: () => call<DictationResult>("stop_dictation"),
  cancelDictation: () => call<void>("cancel_dictation"),

  getSettings: () => call<Settings>("get_settings"),
  updateSettings: (patch: Partial<Settings>) => call<Settings>("update_settings", { patch }),

  listModels: () => call<ModelInfo[]>("list_models"),
  selectModel: (id: string) => call<void>("select_model", { id }),
  verifyModel: (id: string) => call<boolean>("verify_model", { id }),
  asrCapabilities: () => call<AsrCapabilities>("asr_capabilities"),
  getModelDir: () => call<string>("get_model_dir"),

  getHistory: () => call<HistoryEntry[]>("get_history"),
  deleteHistoryEntry: (id: string) => call<void>("delete_history_entry", { id }),
  clearHistory: () => call<number>("clear_history"),
  exportHistory: () => call<string>("export_history"),
  storageInfo: () => call<StorageInfo>("storage_info"),

  listInputDevices: () => call<InputDevice[]>("list_input_devices"),
  copyText: (text: string) => call<void>("copy_text", { text }),
  setHistoryEdit: (id: string, editedText: string | null) =>
    call<HistoryEntry>("set_history_edit", { id, editedText }),

  /** Record the next keys pressed as a shortcut; updates arrive via onShortcutCapture. */
  startShortcutCapture: () => call<void>("start_shortcut_capture"),
  cancelShortcutCapture: () => call<void>("cancel_shortcut_capture"),
};
