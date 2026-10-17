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

export function fmtMs(ms: number): string {
  return ms >= 1000 ? `${(ms / 1000).toFixed(2)} s` : `${Math.round(ms)} ms`;
}

/** Speaking pace in words per minute, or null with too little audio to mean anything. */
export function wordsPerMinute(words: number, audioMs: number): number | null {
  if (audioMs < 10_000 || words === 0) return null;
  return Math.round(words / (audioMs / 60_000));
}

/** Real-time factor (processing time ÷ audio length) in plain words. */
export function speedText(rtf: number): string {
  if (!(rtf > 0)) return "—";
  return rtf <= 1
    ? `${(1 / rtf).toFixed(1)}× faster than real time`
    : `${rtf.toFixed(1)}× slower than real time`;
}

export function median(values: number[]): number | null {
  if (values.length === 0) return null;
  const v = [...values].sort((a, b) => a - b);
  const mid = Math.floor(v.length / 2);
  return v.length % 2 ? v[mid] : (v[mid - 1] + v[mid]) / 2;
}

/** Catalog models at least this large are slow on a CPU (Medium, Large v3 Turbo). */
const HEAVY_MODEL_MB = 1000;

/**
 * Why dictation is slow and what to do, when a heavy model runs on the CPU; null otherwise.
 * `measuredCpuRtf` is the model's real-time factor measured here on the CPU, if known.
 */
export function cpuAdvice(o: {
  modelName: string;
  memoryMb: number;
  measuredCpuRtf: number | null;
  gpuActive: boolean;
  gpuCompiled: boolean;
  preferGpu: boolean;
}): string | null {
  if (o.gpuActive || o.memoryMb < HEAVY_MODEL_MB) return null;
  const cost =
    o.measuredCpuRtf != null && o.measuredCpuRtf > 1
      ? `about ${o.measuredCpuRtf.toFixed(1)}× as long as you spoke`
      : "several times as long as you spoke";
  const fix = !o.gpuCompiled
    ? "This build can't use the GPU, so choose Whisper Small or Base for quick results."
    : !o.preferGpu
      ? "Turn on Use the GPU in Settings, or choose Whisper Small or Base for quick results on the CPU."
      : "The GPU couldn't be used, so choose Whisper Small or Base for quick results.";
  return `${o.modelName} is running on the CPU, where transcribing takes ${cost}. ${fix}`;
}
