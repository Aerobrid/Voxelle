// Pure view helpers (unit-tested): human state labels, today's stats from real history, day groups.
import type { HistoryEntry, SessionSnapshot } from "../ipc/types";

export function stateLabel(s: SessionSnapshot, coreConnected: boolean): { text: string; dot: string } {
  if (!coreConnected) return { text: "Preview — core not connected", dot: "" };
  switch (s.state) {
    case "recording":
      return s.micReady ? { text: "Listening", dot: "dot--recording" } : { text: "Starting mic…", dot: "dot--warming" };
    case "processing":
      return { text: "Transcribing", dot: "dot--processing" };
    case "inserting":
      return { text: "Inserting", dot: "dot--processing" };
    case "failed":
      return { text: "Something went wrong", dot: "dot--failed" };
    default:
      return { text: "Ready", dot: "dot--ready" };
  }
}

export function wordCount(text: string): number {
  return text.trim() === "" ? 0 : text.trim().split(/\s+/).length;
}

function sameLocalDay(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

export interface TodayStats {
  dictations: number;
  words: number;
  /** Total recorded audio today, ms. */
  audioMs: number;
}

/** Counts today's dictations from local history (the text the user kept, edited or raw). */
export function todayStats(entries: HistoryEntry[], now: Date = new Date()): TodayStats {
  const today = entries.filter((e) => sameLocalDay(new Date(e.createdAt), now));
  return {
    dictations: today.length,
    words: today.reduce((n, e) => n + wordCount(e.editedText ?? e.rawTranscript), 0),
    audioMs: today.reduce((n, e) => n + e.durationMs, 0),
  };
}

export function dayLabel(d: Date, now: Date = new Date()): string {
  if (sameLocalDay(d, now)) return "Today";
  const y = new Date(now);
  y.setDate(now.getDate() - 1);
  if (sameLocalDay(d, y)) return "Yesterday";
  return d.toLocaleDateString(undefined, {
    weekday: "long",
    month: "long",
    day: "numeric",
    ...(d.getFullYear() !== now.getFullYear() ? { year: "numeric" } : {}),
  });
}

/** Group entries (already newest-first) into consecutive day buckets, preserving order. */
export function groupByDay(entries: HistoryEntry[], now: Date = new Date()): Array<{ label: string; entries: HistoryEntry[] }> {
  const groups: Array<{ label: string; entries: HistoryEntry[] }> = [];
  for (const e of entries) {
    const label = dayLabel(new Date(e.createdAt), now);
    const last = groups[groups.length - 1];
    if (last && last.label === label) last.entries.push(e);
    else groups.push({ label, entries: [e] });
  }
  return groups;
}

/** Case-insensitive match against the kept text and the original transcript. */
export function matchesQuery(e: HistoryEntry, q: string): boolean {
  const needle = q.trim().toLowerCase();
  if (!needle) return true;
  return e.rawTranscript.toLowerCase().includes(needle) || (e.editedText ?? "").toLowerCase().includes(needle);
}
