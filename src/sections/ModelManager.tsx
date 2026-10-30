import { useState } from "react";
import { core } from "../ipc/bridge";
import type { ModelInfo } from "../ipc/types";
import { errorText, useCore } from "../state/useCore";
import { Gate } from "../components/Gate";
import { toast } from "../state/store";
import { cpuAdvice, speedText } from "../state/derive";

function fmtSize(bytes: number): string {
  const mb = bytes / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb.toFixed(0)} MB`;
}

function formatModelMarkdown(m: ModelInfo): string {
  const fileName = m.path
    ? m.path.split(/[/\\]/).pop()
    : m.userAdded
      ? m.id.replace(/^file:/, "")
      : m.name;
  const lines: string[] = [
    `Name:        ${m.name}`,
    `File:        ${fileName}`,
    `Size:        ${fmtSize(m.sizeBytes)} (${m.sizeBytes.toLocaleString()} bytes)`,
  ];
  if (m.memoryMb > 0) {
    lines.push(`Memory:      ~${m.memoryMb} MB while loaded`);
  }
  if (m.languages.length > 0) {
    lines.push(`Language:    ${m.languages.join(", ")}`);
  }
  if (m.measuredRtf != null) {
    lines.push(`Benchmark:   ${speedText(m.measuredRtf)} (${m.measuredBackend ?? "CPU"})`);
  }
  if (m.userAdded) {
    lines.push(`Format:      whisper.cpp GGML (.bin)`);
    lines.push(`Source:      Custom (user-imported file)`);
  } else {
    lines.push(`License:     ${m.license}`);
    if (m.sha256) {
      lines.push(`SHA-256:     ${m.sha256}`);
    }
  }
  if (m.path) {
    lines.push(`Path:        ${m.path}`);
  }
  return lines.join("\n");
}

function ModelCard({ m, onChange }: { m: ModelInfo; onChange: () => void }) {
  const [busy, setBusy] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const select = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (m.selected || busy != null) return;
    setBusy("select");
    core.selectModel(m.id).then(
      () => {
        setBusy(null);
        toast(`Now using ${m.name}.`);
        onChange();
      },
      (e) => {
        setBusy(null);
        toast(errorText(e), "error");
      },
    );
  };

  const specMarkdown = formatModelMarkdown(m);

  const copyDetails = (e: React.MouseEvent) => {
    e.stopPropagation();
    core.copyText(specMarkdown);
    setCopied(true);
    toast("Model details copied to clipboard.");
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div
      className={`card${m.selected ? " card--selected" : " card--selectable"}`}
      onClick={!m.selected ? select : undefined}
    >
      <div className="row-between" style={{ alignItems: "center" }}>
        <div>
          <h2 style={{ margin: 0, fontSize: "16px", fontWeight: 600 }}>{m.name}</h2>
          {m.measuredRtf != null && (
            <p className="small muted" style={{ margin: "4px 0 0" }}>
              Speed: {speedText(m.measuredRtf)} ({m.measuredBackend ?? "CPU"})
            </p>
          )}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {m.selected ? (
            <span className="tag tag--accent">In use</span>
          ) : (
            <button
              className="btn btn--primary btn--sm"
              disabled={busy != null}
              onClick={select}
            >
              {busy === "select" ? "Selecting…" : "Use this model"}
            </button>
          )}
        </div>
      </div>

      <details className="details" style={{ marginTop: 12 }}>
        <summary style={{ cursor: "pointer", fontSize: "12px", color: "var(--text-3)", userSelect: "none" }}>
          File details
        </summary>
        <div className="model-spec-box">
          <div className="model-spec-box__header">
            <span>model-spec.md</span>
            <button
              type="button"
              className="btn btn--ghost btn--xs"
              onClick={copyDetails}
              title="Copy markdown specifications"
            >
              {copied ? "Copied!" : "Copy"}
            </button>
          </div>
          <pre className="model-spec-box__code">
            {specMarkdown}
          </pre>
        </div>
      </details>
    </div>
  );
}

export function ModelManager() {
  const [q, reload] = useCore(() => core.listModels(), []);
  const [caps] = useCore(() => core.asrCapabilities(), []);
  const [dir] = useCore(() => core.getModelDir(), []);
  const [settings] = useCore(() => core.getSettings(), []);
  const selected = q.status === "ok" ? q.data.find((m) => m.selected) : undefined;
  const advice =
    selected && caps.status === "ok" && settings.status === "ok"
      ? cpuAdvice({
          modelName: selected.name,
          memoryMb: selected.memoryMb,
          measuredCpuRtf: selected.measuredBackend === "CPU" ? selected.measuredRtf : null,
          gpuActive: caps.data.gpuActive,
          gpuCompiled: caps.data.gpuCompiled,
          preferGpu: settings.data.preferGpu,
        })
      : null;
  const openFolder = () => core.openModelFolder().catch((e) => toast(errorText(e), "error"));

  return (
    <div className="page">
      <div className="page__head">
        <h1>Models</h1>
        <p className="sub">Speech models run locally on this computer.</p>
      </div>
      <div className="tiles" style={{ marginBottom: 18 }}>
        <div className="tile">
          <div className="tile__label">Acceleration</div>
          <div className="tile__value">
            {caps.status === "ok"
              ? caps.data.gpuCompiled
                ? `GPU (${caps.data.gpuBackend ?? "Vulkan"}) · ${caps.data.gpuActive ? "in use" : "ready"}`
                : "CPU"
              : "—"}
          </div>
        </div>
        <div className="tile">
          <div className="tile__label">Model folder</div>
          <div className="path">{dir.status === "ok" ? dir.data : "—"}</div>
          <div className="actions" style={{ marginTop: 8 }}>
            <button className="btn btn--ghost btn--sm" onClick={openFolder}>
              Open folder
            </button>
            <button className="btn btn--ghost btn--sm" onClick={reload}>
              Rescan
            </button>
          </div>
        </div>
      </div>
      {advice && <div className="notice notice--warn">{advice}</div>}
      <Gate q={q}>
        {(models) => {
          const installed = models.filter((m) => m.installed);
          if (installed.length === 0) {
            return (
              <div className="card" style={{ textAlign: "center", padding: "36px 20px" }}>
                <p style={{ margin: "0 0 8px", fontSize: "15px", color: "var(--text-1)", fontWeight: 500 }}>
                  No speech models installed yet
                </p>
                <p className="small muted" style={{ margin: "0 0 16px", maxWidth: "48ch", marginLeft: "auto", marginRight: "auto" }}>
                  Add a whisper.cpp model file (<code>.bin</code>) to your model folder, then click Rescan to use it.
                </p>
                <button className="btn btn--primary" onClick={openFolder}>
                  Open model folder
                </button>
              </div>
            );
          }

          return (
            <div>
              {installed.map((m) => (
                <ModelCard key={m.id} m={m} onChange={reload} />
              ))}
              <p className="small muted" style={{ marginTop: 20, textAlign: "center" }}>
                To add more models, drop any whisper.cpp <span className="mono">.bin</span> file into your model folder and click Rescan.
              </p>
            </div>
          );
        }}
      </Gate>
    </div>
  );
}
