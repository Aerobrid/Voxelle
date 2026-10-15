import { create } from "zustand";
import type { DictationResult, SessionSnapshot } from "../ipc/types";
import { isTauri } from "../ipc/bridge";

export type SectionId =
  | "dashboard"
  | "history"
  | "vocabulary"
  | "assistant"
  | "notes"
  | "audio-import"
  | "models"
  | "settings"
  | "privacy";

export interface NavItem {
  id: SectionId;
  label: string;
  /** P0 sections are live; others are visible but labeled planned (never faked). */
  planned: boolean;
}

export const NAV_ITEMS: NavItem[] = [
  { id: "dashboard", label: "Home", planned: false },
  { id: "history", label: "History", planned: false },
  { id: "models", label: "Models", planned: false },
  { id: "settings", label: "Settings", planned: false },
  { id: "privacy", label: "Privacy", planned: false },
  { id: "vocabulary", label: "Vocabulary Coach", planned: true },
  { id: "assistant", label: "AI Assistant", planned: true },
  { id: "notes", label: "Notes", planned: true },
  { id: "audio-import", label: "Audio Import", planned: true },
];

const IDLE: SessionSnapshot = { state: "idle", level: 0, message: null, elapsedMs: 0, micReady: false, generation: 0 };

interface AppState {
  section: SectionId;
  coreConnected: boolean;
  snapshot: SessionSnapshot;
  /** Most recent finished dictation (shown on the dashboard for correction). */
  lastResult: DictationResult | null;
  setLastResult: (r: DictationResult) => void;
  setSection: (s: SectionId) => void;
  setSnapshot: (s: SessionSnapshot) => void;
}

export const useAppStore = create<AppState>((set) => ({
  section: "dashboard",
  coreConnected: isTauri(),
  snapshot: IDLE,
  lastResult: null,
  setLastResult: (lastResult) => set({ lastResult }),
  setSection: (section) => set({ section }),
  setSnapshot: (snapshot) => set({ snapshot }),
}));
