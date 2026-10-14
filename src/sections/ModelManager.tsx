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
        <strong>{m.name}</strong>
        <PrivacyChip locality={m.locality} />
      </div>
      <div className="muted">
        Speech recognition · {fmtSize(m.sizeBytes)} download · ~{m.memoryMb} MB while loaded ·{" "}
        {m.languages.join(", ")}
      </div>
      <div className="muted small">
        License: {m.license} · Source: <span className="mono">{m.source}</span>
      </div>
      <div className="muted small">
        SHA-256 <span className="mono">{m.sha256.slice(0, 16)}…</span>
      </div>
      <div className="model-status">
        <span className={m.installed ? "tag tag--ok" : "tag"}>{m.installed ? "Installed" : "Not installed"}</span>
        {m.installed && (
          <span className={m.integrityVerified ? "tag tag--ok" : "tag"}>
            {m.integrityVerified ? "Checksum verified" : "Not verified yet"}
          </span>
        )}
        {m.selected && <span className="tag tag--accent">Selected</span>}
        <span className="muted small">
          {m.measuredRtf != null
            ? `Measured here: ${m.measuredRtf.toFixed(2)}× real time on ${m.measuredBackend ?? "?"}`
            : "Speed on this machine: not measured yet (dictate once with it)"}
        </span>
      </div>
      <div className="actions">
        {m.installed ? (
          <>
            <button
              className="btn"
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
          <span className="muted small">
            In-app download: <span className="tag">planned</span> — for now, place{" "}
            <span className="mono">{m.source.split("/").pop()}</span> in the model folder; it is
            checksum-verified before first use.
          </span>
        )}
        {note && <span className="muted">{note}</span>}
      </div>
    </div>
  );
}

export function ModelManager() {
  const [q, reload] = useCore(() => core.listModels(), []);
  const [caps] = useCore(() => core.asrCapabilities(), []);
  const [dir] = useCore(() => core.getModelDir(), []);
  return (
    <div className="main">
      <h1>Model Manager</h1>
      <p className="sub">
        Models run on this device. Files are checked against pinned SHA-256 values before they are
        loaded.
      </p>
      {dir.status === "ok" && (
        <p className="muted small">
          Model folder: <span className="mono">{dir.data}</span>
        </p>
      )}
      {caps.status === "ok" && (
        <p className="muted small">
          GPU acceleration in this build: {caps.data.gpuCompiled ? "available (Vulkan)" : "not included (CPU only)"}
          {caps.data.gpuCompiled && ` · currently ${caps.data.gpuActive ? "in use" : "not in use"}`}
        </p>
      )}
      <Gate q={q}>
        {(models) => (
          <div>
            {models[0]?.path == null && models.every((m) => !m.installed) && (
              <div className="notice">No speech model installed yet.</div>
            )}
            {models.map((m) => (
              <ModelCard key={m.id} m={m} onChange={reload} />
            ))}
          </div>
        )}
      </Gate>
    </div>
  );
}
