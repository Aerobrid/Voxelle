// Typed bridge to the Rust core over Tauri IPC.
// The renderer has no OS/Node access; every capability goes through a validated core command.
// In a plain browser (vite preview / vitest) there is no core: calls reject with CoreNotConnected,
// which the UI renders as an honest "core not connected" state rather than faking data.

import type {
  HistoryEntry,
  InsertResult,
  ModelInfo,
  SessionSnapshot,
  Settings,
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

export const core = {
  getSessionSnapshot: () => call<SessionSnapshot>("get_session_snapshot"),
  startDictation: () => call<void>("start_dictation"),
  stopDictation: () => call<InsertResult>("stop_dictation"),
  cancelDictation: () => call<void>("cancel_dictation"),

  getSettings: () => call<Settings>("get_settings"),
  updateSettings: (patch: Partial<Settings>) => call<Settings>("update_settings", { patch }),

  listModels: () => call<ModelInfo[]>("list_models"),
  selectModel: (id: string) => call<void>("select_model", { id }),

  getHistory: () => call<HistoryEntry[]>("get_history"),
  deleteHistoryEntry: (id: string) => call<void>("delete_history_entry", { id }),
  clearHistory: () => call<void>("clear_history"),

  listInputDevices: () => call<string[]>("list_input_devices"),
};
