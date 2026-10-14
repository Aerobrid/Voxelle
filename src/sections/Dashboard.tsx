import { useAppStore } from "../state/store";

export function Dashboard() {
  const snapshot = useAppStore((s) => s.snapshot);
  const coreConnected = useAppStore((s) => s.coreConnected);

  return (
    <div className="main">
      <h1>Dashboard</h1>
      <p className="sub">Local, offline dictation. Nothing leaves this device.</p>

      <div className="card">
        <strong>How to dictate</strong>
        <ol>
          <li>Install a speech model in Model Manager (one-time, verified by checksum).</li>
          <li>
            Focus any text field, then press your toggle hotkey to start and stop, or hold your
            push-to-talk key.
          </li>
          <li>Your words are transcribed locally and inserted at the cursor.</li>
        </ol>
        <p className="muted">
          Current state: <span className="mono">{snapshot.state}</span>
          {snapshot.message ? ` — ${snapshot.message}` : ""}
        </p>
      </div>

      {!coreConnected && (
        <div className="notice">
          Running in browser preview — the desktop core is not connected, so dictation and live data
          are unavailable here.
        </div>
      )}
    </div>
  );
}
