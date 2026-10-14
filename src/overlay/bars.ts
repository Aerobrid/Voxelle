import type { SessionSnapshot } from "../ipc/types";

export const BAR_COUNT = 9;
const MIN_BAR = 0.12;

/** Shift in the newest level on the right; idle bars rest at a small minimum height. */
export function nextBars(prev: number[], level: number): number[] {
  const h = Math.max(MIN_BAR, Math.min(1, level));
  return [...prev.slice(1), h];
}

export function overlayLabel(s: SessionSnapshot): string {
  switch (s.state) {
    case "recording":
      return "Listening";
    case "processing":
      return "Transcribing…";
    case "inserting":
      return "Inserting…";
    case "completed":
      return s.message ? "Copied — press Ctrl+V" : "Done";
    case "cancelled":
      return "Cancelled";
    case "failed":
      return "Didn't work — see Voxelle";
    default:
      return "";
  }
}
