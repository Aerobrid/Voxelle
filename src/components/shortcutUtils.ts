// Utilities for mapping DOM keyboard events to Voxelle's canonical shortcut strings.
// Matches the canonical format and rules enforced by the Rust core (src-tauri/src/hotkeys/mod.rs).

const CODE_TO_NAME: Record<string, string> = {
  Space: "Space",
  Escape: "Escape",
  Insert: "Insert",
  Delete: "Delete",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  ArrowUp: "Up",
  ArrowDown: "Down",
  Pause: "Pause",
  ScrollLock: "ScrollLock",
  CapsLock: "CapsLock",
  PrintScreen: "PrintScreen",
  ContextMenu: "Menu",
  Backquote: "Backquote",
  Minus: "Minus",
  Equal: "Equal",
  BracketLeft: "BracketLeft",
  BracketRight: "BracketRight",
  Backslash: "Backslash",
  Semicolon: "Semicolon",
  Quote: "Quote",
  Comma: "Comma",
  Period: "Period",
  Slash: "Slash",
  NumpadMultiply: "NumpadMultiply",
  NumpadAdd: "NumpadAdd",
  NumpadSubtract: "NumpadSubtract",
  NumpadDecimal: "NumpadDecimal",
  NumpadDivide: "NumpadDivide",
  ControlLeft: "LeftCtrl",
  ControlRight: "RightCtrl",
  ShiftLeft: "LeftShift",
  ShiftRight: "RightShift",
  AltLeft: "LeftAlt",
  AltRight: "RightAlt",
  MetaLeft: "LeftWin",
  MetaRight: "RightWin",
  Enter: "Enter",
  NumpadEnter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
};

export const RESERVED_KEYS = new Set(["Enter", "Tab", "Backspace"]);

export function isModifierCode(code: string): boolean {
  return (
    code.startsWith("Control") ||
    code.startsWith("Shift") ||
    code.startsWith("Alt") ||
    code.startsWith("Meta")
  );
}

export function keyNameFromCode(code: string): string | null {
  if (CODE_TO_NAME[code]) {
    return CODE_TO_NAME[code];
  }
  if (/^Key[A-Z]$/.test(code)) {
    return code.slice(3);
  }
  if (/^Digit[0-9]$/.test(code)) {
    return code.slice(5);
  }
  if (/^Numpad[0-9]$/.test(code)) {
    return code;
  }
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) {
    return code;
  }
  return null;
}

export interface FormattedShortcut {
  shortcut: string | null;
  error?: string;
}

/**
 * Format a set of held physical key codes into Voxelle's canonical shortcut string.
 * Order of modifiers: Ctrl, Win, Alt, Shift (matching Rust core).
 */
export function formatShortcutFromCodes(codes: string[]): FormattedShortcut {
  if (codes.length === 0) {
    return { shortcut: null, error: "No keys were pressed." };
  }

  // Check for reserved keys
  for (const c of codes) {
    const name = keyNameFromCode(c);
    if (name && RESERVED_KEYS.has(name)) {
      return {
        shortcut: null,
        error: `${name} can't be part of a shortcut (it would submit or move focus in other apps).`,
      };
    }
  }

  const modCodes = codes.filter(isModifierCode);
  const otherCodes = codes.filter((c) => !isModifierCode(c));

  // Single modifier key alone (e.g. RightCtrl for push-to-talk)
  if (modCodes.length === 1 && otherCodes.length === 0) {
    const name = keyNameFromCode(modCodes[0]);
    return { shortcut: name };
  }

  // Check others
  const otherNames: string[] = [];
  for (const c of otherCodes) {
    const name = keyNameFromCode(c);
    if (!name) {
      return { shortcut: null, error: "That key can't be used in a shortcut." };
    }
    otherNames.push(name);
  }

  if (otherNames.length > 1) {
    return {
      shortcut: null,
      error: "Use one key plus modifiers (like Ctrl+Shift+Space), or only modifiers (like Ctrl+Win).",
    };
  }

  // Active generic modifiers in Voxelle canonical order: Ctrl, Win, Alt, Shift
  const hasCtrl = modCodes.some((c) => c.startsWith("Control"));
  const hasWin = modCodes.some((c) => c.startsWith("Meta"));
  const hasAlt = modCodes.some((c) => c.startsWith("Alt"));
  const hasShift = modCodes.some((c) => c.startsWith("Shift"));

  const parts: string[] = [];
  if (hasCtrl) parts.push("Ctrl");
  if (hasWin) parts.push("Win");
  if (hasAlt) parts.push("Alt");
  if (hasShift) parts.push("Shift");

  if (otherNames.length === 1) {
    parts.push(otherNames[0]);
  }

  if (parts.length === 0) {
    return { shortcut: null, error: "No keys were pressed." };
  }

  return { shortcut: parts.join("+") };
}
