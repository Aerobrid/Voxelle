// Always-on-top recording indicator. It never takes focus (the core creates its window
// non-focusable and click-through), so the app being dictated into keeps the cursor.
import { useEffect, useRef, useState } from "react";
import { onSessionUpdate } from "../ipc/bridge";
import type { SessionSnapshot } from "../ipc/types";
import { overlayLabel, BAR_COUNT, nextBars } from "./bars";

export function Overlay() {
  const [snap, setSnap] = useState<SessionSnapshot | null>(null);
  const [bars, setBars] = useState<number[]>(() => Array(BAR_COUNT).fill(0.1));
  const levelRef = useRef(0);

  useEffect(() => {
    let unsub = () => {};
    onSessionUpdate((s) => {
      levelRef.current = s.state === "recording" ? s.level : 0;
      setSnap(s);
    }).then((fn) => (unsub = fn));
    return () => unsub();
  }, []);

  // Scroll a short level history across the bars: the waveform motif from the logo, driven by
  // the real microphone level (no decorative fake motion).
  useEffect(() => {
    const id = window.setInterval(() => setBars((b) => nextBars(b, levelRef.current)), 60);
    return () => window.clearInterval(id);
  }, []);

  if (!snap) return null;
  const label = overlayLabel(snap);
  return (
    <div className={`pill pill--${snap.state}`} role="status" aria-live="polite">
      <span className="pill__bars" aria-hidden>
        {bars.map((h, i) => (
          <span key={i} className="pill__bar" style={{ transform: `scaleY(${h})` }} />
        ))}
      </span>
      <span className="pill__label">{label}</span>
      {snap.state === "recording" && (
        <span className="pill__time">{(snap.elapsedMs / 1000).toFixed(0)}s</span>
      )}
    </div>
  );
}
