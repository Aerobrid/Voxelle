import { useState } from "react";
import { core } from "../ipc/bridge";
import type { ModelInfo } from "../ipc/types";
import { errorText, useCore } from "../state/useCore";
import { Gate } from "../components/Gate";
import { PrivacyChip } from "../components/PrivacyChip";

function fmtSize(bytes: number): string {
  const mb = bytes / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb.toFixed(0)} MB`;
}

function ModelCard({ m, onChange }: { m: ModelInfo; onChange: () => void }) {
  const [busy, setBusy] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);

  const act = (label: string, p: () => Promise<unknown>, done: (v: unknown) => string | null) => {
    setBusy(label);
    setNote(null);
    p().then(
      (v) => {
        setBusy(null);
        setNote(done(v));
        onChange();
      },
      (e) => {
        setBusy(null);
        setNote(errorText(e));
      },
    );
  };

  return (
    <div className={`card${m.selected ? " card--selected" : ""}`}>
      <div className="row-between">
        <h2 style={{ margin: 0 }}>{m.name}</h2>
        <div className="model__status">
          {m.selected && <span className="tag tag--accent">In use</span>}
          {m.installed ? (
            <span className={m.integrityVerified ? "tag tag--ok" : "tag"}>
              {m.integrityVerified ? "Installed · verified" : "Installed · not verified yet"}
            </span>
          ) : (
            <span className="tag">Not installed</span>
          )}
        </div>
      </div>
      <div className="model__meta">
        <span>{fmtSize(m.sizeBytes)} download</span>
        <span>~{m.memoryMb} MB while loaded</span>
        <span>{m.languages.length > 3 ? `${m.languages.length} languages` : m.languages.join(", ")}</span>
        <span>{m.license}</span>
        <PrivacyChip locality={m.locality} />
      </div>
      <p className="small muted" style={{ margin: "0 0 12px" }}>
        {m.measuredRtf != null
          ? `Measured on this computer: ${m.measuredRtf.toFixed(2)}× real time (${m.measuredBackend ?? "?"}).`
          : "Speed on this computer: not measured yet — dictate once with it."}
      </p>
      <div className="actions">
        {m.installed ? (
          <>
            <button
              className={m.selected ? "btn" : "btn btn--primary"}
              disabled={m.selected || busy != null}
              onClick={() => act("select", () => core.selectModel(m.id), () => null)}
            >
              {m.selected ? "In use" : "Use this model"}
            </button>
            <button
              className="btn btn--ghost"
              disabled={busy != null}
              onClick={() =>
                act("verify", () => core.verifyModel(m.id), (ok) =>
                  ok ? "Checksum matches the pinned value." : "Checksum MISMATCH — remove and reinstall this file.",
                )
              }
            >
              {busy === "verify" ? "Verifying…" : "Verify checksum"}
            </button>
          </>
        ) : (
          <span className="small muted">
            In-app download is <span className="tag tag--soon">planned</span>. For now, put{" "}
            <span className="mono">{m.source.split("/").pop()}</span> in the model folder; it is checked
            against its pinned SHA-256 before first use.
          </span>
        )}
        {note && <span className="muted small">{note}</span>}
      </div>
      <details className="details">
        <summary>File details</summary>
        <p className="path" style={{ margin: "8px 0 0" }}>Source: {m.source}</p>
        <p className="path" style={{ margin: "4px 0 0" }}>SHA-256: {m.sha256}</p>
        {m.path && <p className="path" style={{ margin: "4px 0 0" }}>Path: {m.path}</p>}
      </details>
    </div>
  );
}

export function ModelManager() {
  const [q, reload] = useCore(() => core.listModels(), []);
  const [caps] = useCore(() => core.asrCapabilities(), []);
  const [dir] = useCore(() => core.getModelDir(), []);
  return (
    <div className="page">
      <div className="page__head">
        <h1>Models</h1>
        <p className="sub">Speech models run on this computer. Each file is checked against a pinned checksum before it loads.</p>
      </div>
      <div className="tiles" style={{ marginBottom: 18 }}>
        <div className="tile">
          <div className="tile__label">Acceleration</div>
          <div className="tile__value">
            {caps.status === "ok"
              ? caps.data.gpuCompiled
                ? `GPU (Vulkan) · ${caps.data.gpuActive ? "in use" : "not in use"}`
                : "CPU only in this build"
              : "—"}
          </div>
        </div>
        <div className="tile">
          <div className="tile__label">Model folder</div>
          <div className="path">{dir.status === "ok" ? dir.data : "—"}</div>
        </div>
      </div>
      <Gate q={q}>
        {(models) => (
          <div>
            {models.every((m) => !m.installed) && <div className="notice notice--warn">No speech model installed yet.</div>}
            {[...models]
              .sort((a, b) => Number(b.installed) - Number(a.installed))
              .map((m) => (
                <ModelCard key={m.id} m={m} onChange={reload} />
              ))}
          </div>
        )}
      </Gate>
    </div>
  );
}
