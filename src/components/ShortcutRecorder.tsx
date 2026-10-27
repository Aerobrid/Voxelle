// Shortcut field. Clicking it opens a small dialog that listens for keys:
// It captures keys via DOM keyboard events in the focused window AND via the core's keyboard hook,
// so all shortcuts (Ctrl+Shift+A, Space, F-keys, and the Win key) work reliably.
// The shortcut is checked when every key is released. Accepted → saved and confirmed with a toast;
// refused → the dialog says why.
import { useEffect, useRef, useState } from "react";
import { core, onShortcutCapture } from "../ipc/bridge";
import type { ShortcutRole } from "../ipc/types";
import { errorText } from "../state/useCore";
import { toast } from "../state/store";
import { Modal } from "./Modal";
import { IconAlert, IconClose, IconPencil } from "./Icons";
import { formatShortcutFromCodes, keyNameFromCode } from "./shortcutUtils";

export function KeyChips({ shortcut, size }: { shortcut: string; size?: "lg" }) {
  return (
    <span className={`keys${size === "lg" ? " keys--lg" : ""}`}>
      {shortcut.split("+").map((k, i) => (
        <kbd key={i}>{k}</kbd>
      ))}
    </span>
  );
}

/** "Ctrl+Win" → "Ctrl + Win" for sentences. */
export const spacedKeys = (s: string) => s.split("+").join(" + ");

type Phase =
  | { kind: "closed" }
  | { kind: "listening"; live: string }
  | { kind: "saving"; shortcut: string }
  | { kind: "refused"; reason: string; keys: string | null }
  | { kind: "paused" };

export function ShortcutRecorder({
  role,
  label,
  value,
  onSave,
  allowClear,
}: {
  role: ShortcutRole;
  /** Name of the shortcut, e.g. "Push to talk". */
  label: string;
  value: string | null;
  /** Persist the shortcut; rejects with the reason when the core refuses it. */
  onSave: (v: string | null) => Promise<unknown>;
  allowClear: boolean;
}) {
  const [phase, setPhase] = useState<Phase>({ kind: "closed" });
  const unlisten = useRef<(() => void) | null>(null);
  const lastKeys = useRef<string | null>(null);
  const heldCodes = useRef<Set<string>>(new Set());
  const peakCodes = useRef<string[]>([]);

  const stopListening = () => {
    unlisten.current?.();
    unlisten.current = null;
    heldCodes.current.clear();
    peakCodes.current = [];
  };

  // Never leave the recorder swallowing keys if this field goes away mid-recording.
  useEffect(
    () => () => {
      if (unlisten.current) {
        stopListening();
        core.cancelShortcutCapture().catch(() => {});
      }
    },
    [],
  );

  const save = (shortcut: string) => {
    if (shortcut === value) {
      setPhase({ kind: "closed" });
      toast(`${label} is still ${spacedKeys(shortcut)}.`, "info");
      return;
    }
    setPhase({ kind: "saving", shortcut });
    onSave(shortcut).then(
      () => {
        setPhase({ kind: "closed" });
        toast(`${label} is now ${spacedKeys(shortcut)}.`);
      },
      (e) => setPhase({ kind: "refused", reason: errorText(e), keys: shortcut }),
    );
  };

  const close = () => {
    if (unlisten.current || phase.kind !== "closed") {
      stopListening();
      core.cancelShortcutCapture().catch(() => {});
    }
    setPhase({ kind: "closed" });
  };

  // Listen for physical keyboard events in the DOM while recording
  useEffect(() => {
    if (phase.kind !== "listening") {
      heldCodes.current.clear();
      peakCodes.current = [];
      return;
    }

    const onKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Escape on its own cancels recording (unless Cancel shortcut itself is being recorded)
      if (e.code === "Escape" && role !== "cancel" && heldCodes.current.size === 0) {
        close();
        return;
      }

      if (heldCodes.current.has(e.code)) {
        return; // auto-repeat
      }

      heldCodes.current.add(e.code);
      const current = Array.from(heldCodes.current);
      if (current.length >= peakCodes.current.length) {
        peakCodes.current = current;
      }

      const formatted = formatShortcutFromCodes(current);
      const display =
        formatted.shortcut ??
        current.map((c) => keyNameFromCode(c) ?? c).join("+");
      lastKeys.current = display;
      setPhase({ kind: "listening", live: display });
    };

    const onKeyUp = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const wasHeld = heldCodes.current.has(e.code);
      heldCodes.current.delete(e.code);

      if (wasHeld && heldCodes.current.size === 0 && peakCodes.current.length > 0) {
        const peak = peakCodes.current;
        peakCodes.current = [];
        const res = formatShortcutFromCodes(peak);
        if (res.error) {
          stopListening();
          core.cancelShortcutCapture().catch(() => {});
          setPhase({ kind: "refused", reason: res.error, keys: lastKeys.current });
          return;
        }
        if (res.shortcut) {
          stopListening();
          core.cancelShortcutCapture().catch(() => {});
          save(res.shortcut);
        }
      }
    };

    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);

    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("keyup", onKeyUp, true);
    };
  }, [phase.kind, role]);

  const listen = async () => {
    stopListening();
    lastKeys.current = null;
    heldCodes.current.clear();
    peakCodes.current = [];
    setPhase({ kind: "listening", live: "" });
    const off = await onShortcutCapture((u) => {
      switch (u.kind) {
        case "progress":
          lastKeys.current = u.keys;
          setPhase({ kind: "listening", live: u.keys });
          return;
        case "done":
          stopListening();
          save(u.shortcut);
          return;
        case "rejected":
          stopListening();
          setPhase({ kind: "refused", reason: u.reason, keys: lastKeys.current });
          return;
        case "ended":
          stopListening();
          setPhase({ kind: "paused" });
          return;
        case "dismissed":
          stopListening();
          setPhase({ kind: "closed" });
          return;
      }
    });
    unlisten.current = off;
    try {
      await core.startShortcutCapture(role);
    } catch (e) {
      console.warn("Global shortcut capture init:", e);
    }
  };

  const turnOff = () =>
    onSave(null).then(
      () => toast(`${label} shortcut turned off.`),
      (e) => toast(errorText(e), "error"),
    );

  return (
    <span className="shortcut">
      <button
        type="button"
        className={`keyfield${value ? "" : " keyfield--empty"}`}
        onClick={listen}
        aria-label={value ? `${label}: ${spacedKeys(value)}. Change shortcut` : `${label}: off. Set a shortcut`}
      >
        {value ? <KeyChips shortcut={value} /> : <span className="keyfield__placeholder">Not set</span>}
        <span className="keyfield__edit" aria-hidden>
          <IconPencil />
        </span>
      </button>
      {allowClear && value ? (
        <button type="button" className="icon-btn" title="Turn off" aria-label={`Turn off ${label}`} onClick={turnOff}>
          <IconClose />
        </button>
      ) : (
        <span className="icon-btn icon-btn--spacer" aria-hidden /> // keeps the fields lined up
      )}

      {phase.kind !== "closed" && (
        <Modal title={`${label} shortcut`} onClose={close} closeOnEscape={phase.kind !== "listening"}>
          {phase.kind === "listening" && (
            <>
              <div className="capture capture--live" aria-live="polite">
                {phase.live ? <KeyChips shortcut={phase.live} size="lg" /> : <span className="capture__prompt">Press your shortcut</span>}
              </div>
              <p className="modal__hint">
                Hold the keys together, then let go. Two modifiers like <KeyChips shortcut="Ctrl+Win" /> work well.
                {role !== "cancel" && " Press Esc to keep the current one."}
              </p>
              <div className="modal__actions">
                <button className="btn btn--ghost" onClick={close}>
                  Cancel
                </button>
              </div>
            </>
          )}
          {phase.kind === "saving" && (
            <div className="capture">
              <KeyChips shortcut={phase.shortcut} size="lg" />
            </div>
          )}
          {phase.kind === "refused" && (
            <>
              <div className="capture capture--refused">
                {phase.keys && <KeyChips shortcut={phase.keys} size="lg" />}
              </div>
              <p className="refusal">
                <IconAlert />
                <span>
                  <strong>That shortcut can't be used.</strong> {phase.reason}
                </span>
              </p>
              <div className="modal__actions">
                <button className="btn btn--ghost" onClick={close}>
                  Keep current
                </button>
                <button className="btn btn--primary" onClick={listen}>
                  Try again
                </button>
              </div>
            </>
          )}
          {phase.kind === "paused" && (
            <>
              <div className="capture">
                <span className="capture__prompt">Stopped listening</span>
              </div>
              <p className="modal__hint">No keys were pressed for a while, so your shortcut is unchanged.</p>
              <div className="modal__actions">
                <button className="btn btn--ghost" onClick={close}>
                  Close
                </button>
                <button className="btn btn--primary" onClick={listen}>
                  Listen again
                </button>
              </div>
            </>
          )}
        </Modal>
      )}
    </span>
  );
}
