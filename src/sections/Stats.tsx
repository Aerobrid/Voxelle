// Timings for checking speed and troubleshooting. Per-dictation timings live in memory only (since
// Voxelle started); per-model speed is what the core recorded from real dictations.
import { core } from "../ipc/bridge";
import type { DictationResult } from "../ipc/types";
import { useAppStore } from "../state/store";
import { useCore } from "../state/useCore";
import { fmtMs, median, speedText, wordCount } from "../state/derive";

/** First-audio delay worth explaining (speech before it is lost). */
const SLOW_MIC_MS = 400;

const METHOD_TEXT: Record<string, string> = {
  typed: "Typed",
  "clipboard-paste": "Pasted",
  "clipboard-copy": "Copied only",
  none: "—",
};

function LastDetails({ r }: { r: DictationResult }) {
  const m = r.metrics;
  const slowMic = !m.micWasReady && m.firstAudioMs != null && m.firstAudioMs >= SLOW_MIC_MS;
  return (
    <div className="card">
      <dl className="metrics">
        <div><dt>Audio length</dt><dd>{fmtMs(m.audioMs)}</dd></div>
        <div><dt>Stop → text</dt><dd>{fmtMs(m.stopToTranscriptMs)}</dd></div>
        <div><dt>Transcription</dt><dd>{fmtMs(m.transcribeMs)} on {m.gpu ? "GPU" : "CPU"}</dd></div>
        <div><dt>Speed</dt><dd>{speedText(m.realTimeFactor)}</dd></div>
        <div><dt>Mic open</dt><dd>{m.micWasReady ? "Already open" : fmtMs(m.startLatencyMs)}</dd></div>
        {m.firstAudioMs != null && <div><dt>First audio after</dt><dd>{fmtMs(m.firstAudioMs)}</dd></div>}
        {m.modelLoadMs > 50 && <div><dt>Model load</dt><dd>{fmtMs(m.modelLoadMs)}</dd></div>}
        <div><dt>Result</dt><dd>{METHOD_TEXT[r.insert.method] ?? r.insert.method}</dd></div>
      </dl>
      {slowMic && (
        <p className="faint small" style={{ margin: "14px 0 0" }}>
          Audio began {fmtMs(m.firstAudioMs!)} after you started. Some microphones, Bluetooth headsets
          especially, take that long to wake up. <em>Keep microphone ready</em> in Settings removes the wait.
        </p>
      )}
    </div>
  );
}

export function Stats() {
  const results = useAppStore((s) => s.recentResults);
  const [models] = useCore(() => core.listModels(), [results.length]);
  const last = results[0];
  const stopToText = median(results.map((r) => r.metrics.stopToTranscriptMs));
  const rtf = median(results.map((r) => r.metrics.realTimeFactor));
  const measured = models.status === "ok" ? models.data.filter((m) => m.measuredRtf != null) : [];

  return (
    <div className="page">
      <div className="page__head">
        <h1>Stats</h1>
        <p className="sub">Measured on this computer. Dictation timings cover the time since Voxelle started.</p>
      </div>

      <div className="stats">
        <div className="stat">
          <div className="stat__value">{results.length || "—"}</div>
          <div className="stat__label">Dictations measured</div>
        </div>
        <div className="stat">
          <div className="stat__value">{stopToText != null ? fmtMs(stopToText) : "—"}</div>
          <div className="stat__label">Typical stop → text</div>
        </div>
        <div className="stat">
          <div className="stat__value stat__value--text">{rtf != null ? speedText(rtf) : "—"}</div>
          <div className="stat__label">Typical speed</div>
        </div>
      </div>

      {last ? (
        <section className="section">
          <div className="section__head">
            <h2>Last dictation</h2>
          </div>
          <LastDetails r={last} />
        </section>
      ) : (
        <div className="empty" style={{ marginTop: 24 }}>Dictate once and its timings appear here.</div>
      )}

      {results.length > 1 && (
        <section className="section">
          <div className="section__head">
            <h2>Recent dictations</h2>
          </div>
          <table className="table">
            <thead>
              <tr>
                <th>Words</th>
                <th>Audio</th>
                <th>Stop → text</th>
                <th>On</th>
                <th>Speed</th>
              </tr>
            </thead>
            <tbody>
              {results.map((r, i) => (
                <tr key={`${r.historyId ?? "x"}-${i}`}>
                  <td>{wordCount(r.transcript)}</td>
                  <td>{fmtMs(r.metrics.audioMs)}</td>
                  <td>{fmtMs(r.metrics.stopToTranscriptMs)}</td>
                  <td>{r.metrics.gpu ? "GPU" : "CPU"}</td>
                  <td>{speedText(r.metrics.realTimeFactor)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      {measured.length > 0 && (
        <section className="section">
          <div className="section__head">
            <h2>Speech models on this computer</h2>
          </div>
          <table className="table">
            <thead>
              <tr>
                <th>Model</th>
                <th>Last measured on</th>
                <th>Speed</th>
              </tr>
            </thead>
            <tbody>
              {measured.map((m) => (
                <tr key={m.id}>
                  <td>{m.name}</td>
                  <td>{m.measuredBackend ?? "—"}</td>
                  <td>{speedText(m.measuredRtf!)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </div>
  );
}
