// Shortcut field: click "Change", then press the keys. The core's keyboard hook records them (so
// the Win key works and doesn't open Start) and the shortcut is saved when every key is released.
import { useEffect, useRef, useState } from "react";
import { core, onShortcutCapture } from "../ipc/bridge";
import { errorText } from "../state/useCore";

export function KeyChips({ shortcut }: { shortcut: string }) {
  return (
    <span className="keys">
      {shortcut.split("+").map((k, i) => (
        <kbd key={i}>{k}</kbd>
      ))}
    </span>
  );
}

export function ShortcutRecorder({
  value,
  onSave,
  allowClear,
}: {
  value: string | null;
  onSave: (v: string | null) => void;
  allowClear: boolean;
}) {
  const [recording, setRecording] = useState(false);
  const [live, setLive] = useState("");
  const [error, setError] = useState<string | null>(null);
  const unlisten = useRef<() => void>(() => {});

  const stopListening = () => {
    unlisten.current();
    unlisten.current = () => {};
    setRecording(false);
    setLive("");
  };

  // Never leave the recorder swallowing keys if this field goes away.
  useEffect(
    () => () => {
      if (unlisten.current) {
        unlisten.current();
        core.cancelShortcutCapture().catch(() => {});
      }
    },
    [],
  );

  const start = async () => {
    setError(null);
    unlisten.current = await onShortcutCapture((u) => {
      if (u.kind === "progress") setLive(u.keys);
      else if (u.kind === "done") {
        stopListening();
        if (u.shortcut !== value) onSave(u.shortcut);
      } else {
        stopListening();
        setError(u.reason);
      }
    });
    try {
      await core.startShortcutCapture();
      setRecording(true);
    } catch (e) {
      stopListening();
      setError(errorText(e));
    }
  };

  const cancel = () => {
    core.cancelShortcutCapture().catch(() => {});
    stopListening();
  };

  return (
    <span className="shortcut">
      {recording ? (
        <span className="shortcut__live" aria-live="polite">
          {live ? <KeyChips shortcut={live} /> : <span className="muted">Press the keys…</span>}
        </span>
      ) : value ? (
        <KeyChips shortcut={value} />
      ) : (
        <span className="muted">Off</span>
      )}
      {recording ? (
        <button className="btn btn--ghost btn--sm" onClick={cancel}>
          Cancel
        </button>
      ) : (
        <>
          <button className="btn btn--sm" onClick={start}>
            Change
          </button>
          {allowClear && value && (
            <button className="btn btn--ghost btn--sm" onClick={() => onSave(null)}>
              Turn off
            </button>
          )}
        </>
      )}
      {error && <span className="error small shortcut__error">{error}</span>}
    </span>
  );
}
