import { create } from "zustand";
import type { DictationResult, SessionSnapshot } from "../ipc/types";
import { isTauri } from "../ipc/bridge";

export type SectionId =
  | "dashboard"
  | "history"
  | "stats"
  | "vocabulary"
  | "assistant"
  | "notes"
  | "audio-import"
  | "models"
  | "settings"

export interface NavItem {
  id: SectionId;
  label: string;
  /** P0 sections are live; others are visible but labeled planned (never faked). */
  planned: boolean;
}

export const NAV_ITEMS: NavItem[] = [
  { id: "dashboard", label: "Home", planned: false },
  { id: "history", label: "History", planned: false },
  { id: "stats", label: "Stats", planned: false },
  { id: "models", label: "Models", planned: false },
  { id: "settings", label: "Settings", planned: false },
  { id: "vocabulary", label: "Vocabulary Coach", planned: true },
  { id: "assistant", label: "AI Assistant", planned: true },
  { id: "notes", label: "Notes", planned: true },
  { id: "audio-import", label: "Audio Import", planned: true },
];

const IDLE: SessionSnapshot = { state: "idle", level: 0, message: null, elapsedMs: 0, micReady: false, generation: 0 };

/** Dictation timings are kept in memory for the Stats page; this many, newest first. */
export const RECENT_RESULTS_MAX = 25;

export type ToastTone = "ok" | "info" | "error";

export interface Toast {
  id: number;
  text: string;
  tone: ToastTone;
}

interface AppState {
  section: SectionId;
  coreConnected: boolean;
  snapshot: SessionSnapshot;
  /** Most recent finished dictation (shown on the dashboard for correction). */
  lastResult: DictationResult | null;
  /** This session's dictations, newest first (timings are not stored on disk). */
  recentResults: DictationResult[];
  toasts: Toast[];
  setLastResult: (r: DictationResult) => void;
  setSection: (s: SectionId) => void;
  setSnapshot: (s: SessionSnapshot) => void;
  pushToast: (text: string, tone?: ToastTone) => void;
  dismissToast: (id: number) => void;
}

let nextToastId = 1;
const TOAST_MS = 2600;

export const useAppStore = create<AppState>((set, get) => ({
  section: "dashboard",
  coreConnected: isTauri(),
  snapshot: IDLE,
  lastResult: null,
  recentResults: [],
  toasts: [],
  setLastResult: (lastResult) =>
    set((s) => ({ lastResult, recentResults: [lastResult, ...s.recentResults].slice(0, RECENT_RESULTS_MAX) })),
  setSection: (section) => set({ section }),
  setSnapshot: (snapshot) => set({ snapshot }),
  pushToast: (text, tone = "ok") => {
    const id = nextToastId++;
    // One message at a time reads better than a stack; the newest replaces the rest.
    set({ toasts: [{ id, text, tone }] });
    setTimeout(() => get().dismissToast(id), tone === "error" ? TOAST_MS * 2 : TOAST_MS);
  },
  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

/** Show a short confirmation ("Copied") from anywhere. */
export const toast = (text: string, tone?: ToastTone) => useAppStore.getState().pushToast(text, tone);
