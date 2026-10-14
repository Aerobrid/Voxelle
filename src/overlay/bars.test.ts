import { describe, expect, it } from "vitest";
import { BAR_COUNT, nextBars, overlayLabel } from "./bars";

describe("overlay bars", () => {
  it("shifts in the newest level and clamps it", () => {
    const start = Array(BAR_COUNT).fill(0.12);
    const next = nextBars(start, 2);
    expect(next).toHaveLength(BAR_COUNT);
    expect(next[BAR_COUNT - 1]).toBe(1);
    expect(nextBars(next, 0)[BAR_COUNT - 1]).toBe(0.12);
  });
});

describe("overlay label", () => {
  const snap = (state: any, message: string | null = null) => ({ state, level: 0, message, elapsedMs: 0, generation: 1 });
  it("never claims insertion when the text was only copied", () => {
    // A completed session with a message is the copied-only path (see dictation.rs).
    expect(overlayLabel(snap("completed", "copied because…"))).toMatch(/Copied/);
    expect(overlayLabel(snap("completed"))).toBe("Done");
  });
  it("labels each active state", () => {
    expect(overlayLabel(snap("recording"))).toBe("Listening");
    expect(overlayLabel(snap("processing"))).toMatch(/Transcribing/);
    expect(overlayLabel(snap("failed"))).toMatch(/Didn't work/);
  });
});
