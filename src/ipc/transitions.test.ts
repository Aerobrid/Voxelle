import { describe, it, expect } from "vitest";
import { canStart, canStop, canCancel, isTerminal } from "./transitions";
import type { SessionState } from "./types";

const ALL: SessionState[] = [
  "idle",
  "recording",
  "processing",
  "inserting",
  "completed",
  "cancelled",
  "failed",
];

describe("session transition guards (UI mirror)", () => {
  it("start is allowed only from idle or terminal states", () => {
    expect(ALL.filter(canStart).sort()).toEqual(
      ["completed", "cancelled", "failed", "idle"].sort(),
    );
  });

  it("stop is allowed only while recording", () => {
    expect(ALL.filter(canStop)).toEqual(["recording"]);
  });

  it("cancel is allowed only while recording or processing", () => {
    expect(ALL.filter(canCancel).sort()).toEqual(["processing", "recording"].sort());
  });

  it("terminal states are completed/cancelled/failed", () => {
    expect(ALL.filter(isTerminal).sort()).toEqual(["cancelled", "completed", "failed"].sort());
  });

  it("cannot start while recording/processing/inserting", () => {
    expect(canStart("recording")).toBe(false);
    expect(canStart("processing")).toBe(false);
    expect(canStart("inserting")).toBe(false);
  });
});
