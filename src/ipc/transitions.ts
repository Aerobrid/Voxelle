import type { SessionState } from "./types";

// UI-side mirror of the core state machine's guards (src-tauri/src/session/state.rs).
// Used to enable/disable controls. The core is always the authority and re-checks every action.

export const TERMINAL: readonly SessionState[] = ["completed", "cancelled", "failed"];

export function isTerminal(s: SessionState): boolean {
  return TERMINAL.includes(s);
}

/** A new dictation may start only from idle or a terminal state. */
export function canStart(s: SessionState): boolean {
  return s === "idle" || isTerminal(s);
}

/** Stop applies only while recording. */
export function canStop(s: SessionState): boolean {
  return s === "recording";
}

/** Cancel applies while actively recording or processing. */
export function canCancel(s: SessionState): boolean {
  return s === "recording" || s === "processing";
}
