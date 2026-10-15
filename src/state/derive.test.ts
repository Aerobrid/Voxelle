import { describe, expect, it } from "vitest";
import type { HistoryEntry, SessionSnapshot } from "../ipc/types";
import { dayLabel, groupByDay, matchesQuery, stateLabel, todayStats, wordCount } from "./derive";

const entry = (id: string, createdAt: string, raw: string, edited: string | null = null): HistoryEntry => ({
  id,
  createdAt,
  rawTranscript: raw,
  editedText: edited,
  durationMs: 2000,
  language: "en",
});

const snap = (state: SessionSnapshot["state"], micReady = true): SessionSnapshot => ({
  state,
  level: 0,
  message: null,
  elapsedMs: 0,
  micReady,
  generation: 1,
});

describe("stateLabel", () => {
  it("never claims to listen before the mic delivers audio", () => {
    expect(stateLabel(snap("recording", false), true).text).toBe("Starting mic…");
    expect(stateLabel(snap("recording", true), true).text).toBe("Listening");
  });
  it("says so when the core is not connected", () => {
    expect(stateLabel(snap("idle"), false).text).toMatch(/not connected/);
    expect(stateLabel(snap("idle"), true).text).toBe("Ready");
  });
});

describe("todayStats", () => {
  const now = new Date(2026, 9, 15, 15, 0);
  it("counts only today's entries and prefers the edited text", () => {
    const entries = [
      entry("a", new Date(2026, 9, 15, 9, 0).toISOString(), "one two three"),
      entry("b", new Date(2026, 9, 15, 10, 0).toISOString(), "uh hello", "hello world again"),
      entry("c", new Date(2026, 9, 14, 23, 0).toISOString(), "yesterday words here"),
    ];
    expect(todayStats(entries, now)).toEqual({ dictations: 2, words: 6, audioMs: 4000 });
  });
  it("is zero with no history", () => {
    expect(todayStats([], now)).toEqual({ dictations: 0, words: 0, audioMs: 0 });
  });
});

describe("history grouping", () => {
  const now = new Date(2026, 9, 15, 15, 0);
  it("labels today and yesterday, keeps order", () => {
    const entries = [
      entry("a", new Date(2026, 9, 15, 9, 0).toISOString(), "x"),
      entry("b", new Date(2026, 9, 15, 8, 0).toISOString(), "y"),
      entry("c", new Date(2026, 9, 14, 8, 0).toISOString(), "z"),
      entry("d", new Date(2026, 9, 10, 8, 0).toISOString(), "w"),
    ];
    const g = groupByDay(entries, now);
    expect(g.map((x) => x.label).slice(0, 2)).toEqual(["Today", "Yesterday"]);
    expect(g[0].entries.map((e) => e.id)).toEqual(["a", "b"]);
    expect(g).toHaveLength(3);
    expect(dayLabel(new Date(2026, 9, 10), now)).not.toMatch(/Today|Yesterday/);
  });
  it("searches raw and edited text", () => {
    const e = entry("a", now.toISOString(), "Ship the smaller model", "Ship the small model");
    expect(matchesQuery(e, "SMALLER")).toBe(true);
    expect(matchesQuery(e, "small model")).toBe(true);
    expect(matchesQuery(e, "latency")).toBe(false);
    expect(matchesQuery(e, "  ")).toBe(true);
  });
  it("counts words", () => {
    expect(wordCount("  hello   there ")).toBe(2);
    expect(wordCount("")).toBe(0);
  });
});
