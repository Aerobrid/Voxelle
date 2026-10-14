import { core } from "../ipc/bridge";
import { useCore } from "../state/useCore";
import { Gate } from "../components/Gate";

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", padding: "6px 0" }}>
      <span className="muted">{label}</span>
      <span className="mono">{value}</span>
    </div>
  );
}

export function Settings() {
  const q = useCore(() => core.getSettings(), []);
  return (
    <div className="main">
      <h1>Settings</h1>
      <p className="sub">Every setting here maps to real behavior. Planned options are labeled.</p>
      <Gate q={q}>
        {(s) => (
          <>
            <div className="card">
              <strong>Hotkeys</strong>
              <Row label="Toggle" value={s.toggleHotkey} />
              <Row label="Push-to-talk" value={s.pushToTalkHotkey ?? "none"} />
              <Row label="Cancel" value={s.cancelHotkey ?? "none"} />
            </div>
            <div className="card">
              <strong>Transcription</strong>
              <Row label="Language" value={s.language} />
              <Row label="Punctuation" value={String(s.punctuation)} />
              <Row label="Auto-capitalize" value={String(s.autoCapitalize)} />
              <Row label="Remove filler words (cleanup)" value={String(s.removeFillerWords)} />
              <Row label="Prefer GPU" value={String(s.preferGpu)} />
              <Row label="Selected speech model" value={s.selectedSpeechModelId ?? "none"} />
            </div>
            <div className="card">
              <strong>Privacy &amp; storage</strong>
              <Row label="Store transcripts" value={String(s.storeTranscripts)} />
              <Row
                label="Retention"
                value={s.retentionDays == null ? "keep forever" : `${s.retentionDays} days`}
              />
              <Row label="Offline mode (block optional network)" value={String(s.offlineMode)} />
              <Row label="Model directory" value={s.modelDir ?? "default"} />
            </div>
          </>
        )}
      </Gate>
    </div>
  );
}
