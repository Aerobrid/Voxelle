// The Home screen's focal control: a round button whose waveform bars follow the real microphone
// level while recording (no decorative fake motion), ringed by the brand gradient.
import { useEffect, useRef, useState } from "react";
import type { SessionSnapshot } from "../ipc/types";

const BARS = 5;
/** Resting silhouette: the logo's symmetric waveform. */
const REST = [0.34, 0.62, 0.9, 0.62, 0.34];

export function VoiceOrb({
  snapshot,
  disabled,
  onClick,
  label,
}: {
  snapshot: SessionSnapshot;
  disabled: boolean;
  onClick: () => void;
  label: string;
}) {
  const recording = snapshot.state === "recording";
  const levelRef = useRef(0);
  levelRef.current = recording && snapshot.micReady ? snapshot.level : 0;
  const [bars, setBars] = useState(REST);

  useEffect(() => {
    if (!recording) {
      setBars(REST);
      return;
    }
    // Each bar follows the live level with its own weight, so speech reads as a waveform.
    const id = window.setInterval(() => {
      const l = levelRef.current;
      // Variation scales with the level: silence holds still.
      const t = Date.now() / 140;
      setBars(REST.map((r, i) => Math.max(0.16, Math.min(1, r * (0.35 + l * 1.6) * (1 + l * 0.6 * Math.sin(t + i * 1.7))))));
    }, 70);
    return () => window.clearInterval(id);
  }, [recording]);

  const mod =
    snapshot.state === "recording"
      ? snapshot.micReady
        ? " orb--recording"
        : " orb--recording orb--warming"
      : snapshot.state === "processing" || snapshot.state === "inserting"
        ? " orb--processing"
        : "";

  return (
    <button className={`orb${mod}`} disabled={disabled} onClick={onClick} aria-label={label} title={label}>
      <span className="orb__bars" aria-hidden>
        {bars.slice(0, BARS).map((h, i) => (
          <span key={i} className="orb__bar" style={{ transform: `scaleY(${h})` }} />
        ))}
      </span>
    </button>
  );
}
