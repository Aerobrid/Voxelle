import { useAppStore } from "../state/store";
import { PrivacyChip } from "./PrivacyChip";

const DOT_CLASS: Record<string, string> = {
  recording: "dot--recording",
  processing: "dot--processing",
  inserting: "dot--processing",
  completed: "dot--completed",
  failed: "dot--failed",
  cancelled: "dot--failed",
};

export function StatusStrip() {
  const snapshot = useAppStore((s) => s.snapshot);
  const coreConnected = useAppStore((s) => s.coreConnected);
  const dotClass = DOT_CLASS[snapshot.state] ?? "";

  return (
    <footer className="status">
      <span className="state-badge">
        <span className={`dot ${dotClass}`} />
        {snapshot.state}
      </span>

      {snapshot.state === "recording" && (
        <span className="level" title="Input level">
          <span className="level__fill" style={{ width: `${Math.round(snapshot.level * 100)}%` }} />
        </span>
      )}

      {snapshot.message && <span className="muted">{snapshot.message}</span>}

      <span className="status__spacer" />

      {/* The P0 dictation loop is entirely local. */}
      <PrivacyChip locality="local" />
      <span className="muted">
        {coreConnected ? "Core connected" : "Core not connected (preview)"}
      </span>
    </footer>
  );
}
