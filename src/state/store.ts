import { create } from "zustand";
import type { SessionSnapshot } from "../ipc/types";
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
  { id: "dashboard", label: "Dashboard", planned: false },
  { id: "history", label: "History", planned: false },
  { id: "models", label: "Model Manager", planned: false },
  { id: "settings", label: "Settings", planned: false },
  { id: "privacy", label: "Privacy", planned: false },
  { id: "vocabulary", label: "Vocabulary Coach", planned: true },
  { id: "assistant", label: "AI Assistant", planned: true },
  { id: "notes", label: "Notes", planned: true },
  { id: "audio-import", label: "Audio Import", planned: true },
];

const IDLE: SessionSnapshot = { state: "idle", level: 0, message: null, elapsedMs: 0 };

interface AppState {
  section: SectionId;
  coreConnected: boolean;
  snapshot: SessionSnapshot;
  setSection: (s: SectionId) => void;
  setSnapshot: (s: SessionSnapshot) => void;
}

export const useAppStore = create<AppState>((set) => ({
  section: "dashboard",
  coreConnected: isTauri(),
  snapshot: IDLE,
  setSection: (section) => set({ section }),
  setSnapshot: (snapshot) => set({ snapshot }),
}));
