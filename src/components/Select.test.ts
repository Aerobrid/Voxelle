import { describe, expect, it } from "vitest";
import { typeAhead } from "./Select";

const opts = ["Off", "For 1 minute", "For 5 minutes", "Until Voxelle quits"].map((label, i) => ({ value: String(i), label }));

describe("typeAhead", () => {
  it("jumps to the next option starting with the letter, wrapping", () => {
    expect(typeAhead(opts, 0, "f")).toBe(1);
    expect(typeAhead(opts, 1, "F")).toBe(2);
    expect(typeAhead(opts, 2, "f")).toBe(1);
    expect(typeAhead(opts, 3, "o")).toBe(0);
  });
  it("stays put when nothing matches", () => {
    expect(typeAhead(opts, 2, "z")).toBe(2);
  });
});
