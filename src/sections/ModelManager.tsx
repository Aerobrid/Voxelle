import { core } from "../ipc/bridge";
import { useCore } from "../state/useCore";
import { Gate } from "../components/Gate";
import { PrivacyChip } from "../components/PrivacyChip";

function fmtSize(bytes: number | null): string {
  if (bytes == null) return "—";
  const mb = bytes / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb.toFixed(0)} MB`;
}

export function ModelManager() {
  const q = useCore(() => core.listModels(), []);
  return (
    <div className="main">
      <h1>Model Manager</h1>
      <p className="sub">
        Models are stored locally. Downloads need your consent and are verified by SHA-256.
      </p>
      <Gate q={q}>
        {(models) =>
          models.length === 0 ? (
            <div className="notice">No models yet. Install a speech model to start dictating.</div>
          ) : (
            <div>
              {models.map((m) => (
                <div className="card" key={m.id}>
                  <div style={{ display: "flex", justifyContent: "space-between", gap: 12 }}>
                    <strong>{m.name}</strong>
                    <PrivacyChip locality={m.locality} />
                  </div>
                  <div className="muted">
                    {m.purpose} · {fmtSize(m.sizeBytes)} · {m.languages.join(", ") || "multi"} ·{" "}
                    {m.license}
                  </div>
                  <div className="muted">
                    {m.installed ? "installed" : "not installed"}
                    {m.selected ? " · selected" : ""} ·{" "}
                    {m.measuredRtf != null
                      ? `measured RTF ${m.measuredRtf.toFixed(2)}×`
                      : "speed not benchmarked yet"}
                    {m.installed ? (m.integrityVerified ? " · verified" : " · unverified") : ""}
                  </div>
                </div>
              ))}
            </div>
          )
        }
      </Gate>
    </div>
  );
}
