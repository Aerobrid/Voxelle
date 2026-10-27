import { describe, expect, it } from "vitest";
import { formatShortcutFromCodes, keyNameFromCode } from "./shortcutUtils";

describe("shortcutUtils", () => {
  it("formats single letter with Ctrl and Shift in canonical order", () => {
    const res = formatShortcutFromCodes(["ControlLeft", "ShiftLeft", "KeyA"]);
    expect(res.shortcut).toBe("Ctrl+Shift+A");
  });

  it("formats Space with Ctrl and Shift", () => {
    const res = formatShortcutFromCodes(["ControlLeft", "ShiftLeft", "Space"]);
    expect(res.shortcut).toBe("Ctrl+Shift+Space");
  });

  it("formats Win modifier chord in canonical order (Ctrl+Win)", () => {
    const res = formatShortcutFromCodes(["ControlLeft", "MetaLeft"]);
    expect(res.shortcut).toBe("Ctrl+Win");
  });

  it("preserves side for single modifier alone", () => {
    expect(formatShortcutFromCodes(["ControlRight"]).shortcut).toBe("RightCtrl");
    expect(formatShortcutFromCodes(["AltRight"]).shortcut).toBe("RightAlt");
    expect(formatShortcutFromCodes(["ShiftRight"]).shortcut).toBe("RightShift");
    expect(formatShortcutFromCodes(["ControlLeft"]).shortcut).toBe("LeftCtrl");
  });

  it("handles function keys alone or combined", () => {
    expect(formatShortcutFromCodes(["F9"]).shortcut).toBe("F9");
    expect(formatShortcutFromCodes(["ControlLeft", "F9"]).shortcut).toBe("Ctrl+F9");
  });

  it("rejects reserved keys like Enter, Tab, Backspace", () => {
    const enterRes = formatShortcutFromCodes(["ControlLeft", "Enter"]);
    expect(enterRes.shortcut).toBeNull();
    expect(enterRes.error).toContain("Enter can't be part of a shortcut");

    const tabRes = formatShortcutFromCodes(["ControlLeft", "Tab"]);
    expect(tabRes.shortcut).toBeNull();
    expect(tabRes.error).toContain("Tab can't be part of a shortcut");
  });

  it("rejects combinations with two regular keys", () => {
    const res = formatShortcutFromCodes(["ControlLeft", "KeyA", "KeyB"]);
    expect(res.shortcut).toBeNull();
    expect(res.error).toContain("Use one key plus modifiers");
  });

  it("maps navigation and punctuation keys correctly", () => {
    expect(keyNameFromCode("ArrowLeft")).toBe("Left");
    expect(keyNameFromCode("ArrowUp")).toBe("Up");
    expect(keyNameFromCode("Backquote")).toBe("Backquote");
    expect(keyNameFromCode("Minus")).toBe("Minus");
    expect(keyNameFromCode("Equal")).toBe("Equal");
    expect(keyNameFromCode("Escape")).toBe("Escape");
  });
});
