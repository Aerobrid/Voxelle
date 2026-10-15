import { useState, type ReactNode } from "react";
import { core } from "../ipc/bridge";
import type { Settings as SettingsT } from "../ipc/types";
import { errorText, useCore } from "../state/useCore";
import { Gate } from "../components/Gate";
import { ShortcutRecorder } from "../components/ShortcutRecorder";

/** Matches u32::MAX in the core: keep the mic open until the app quits. */
const KEEP_UNTIL_QUIT = 4294967295;

const LANGUAGES: Array<[string, string]> = [
  ["auto", "Detect automatically"],
  ["en", "English"],
  ["es", "Spanish"],
  ["fr", "French"],
  ["de", "German"],
  ["it", "Italian"],
  ["pt", "Portuguese"],
  ["nl", "Dutch"],
  ["hi", "Hindi"],
  ["gu", "Gujarati"],
  ["ja", "Japanese"],
  ["zh", "Chinese"],
  ["ko", "Korean"],
];

function Field({ label, hint, children }: { label: string; hint?: ReactNode; children: ReactNode }) {
  return (
    <label className="field">
      <span className="field__label">
        {label}
        {hint && <span className="muted small">{hint}</span>}
      </span>
      <span className="field__control">{children}</span>
    </label>
  );
}

function Planned({ label, hint }: { label: string; hint: string }) {
  return (
    <div className="field">
      <span className="field__label">
        {label}
        <span className="muted small">{hint}</span>
      </span>
      <span className="tag">planned</span>
    </div>
  );
}

/** Like Field, but not a <label>: rows with several buttons must not forward clicks. */
function Row({ label, hint, children }: { label: string; hint?: ReactNode; children: ReactNode }) {
  return (
    <div className="field">
      <span className="field__label">
        {label}
        {hint && <span className="muted small">{hint}</span>}
      </span>
      <span className="field__control">{children}</span>
    </div>
  );
}

function SettingsForm({ initial }: { initial: SettingsT }) {
  const [s, setS] = useState(initial);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [devices] = useCore(() => core.listInputDevices(), []);

  const save = (patch: Partial<SettingsT>) => {
    setError(null);
    setSaved(false);
    core.updateSettings(patch).then(
      (next) => {
        setS(next);
        setSaved(true);
      },
      (e) => setError(errorText(e)),
    );
  };

  return (
    <>
      {error && <div className="notice notice--error">Not saved: {error}</div>}
      {saved && !error && <p className="muted small">Saved.</p>}

      <div className="card">
        <strong>Shortcuts</strong>
        <p className="muted small">
          Click <em>Change</em> and press the keys you want, including the Windows key (for example
          Ctrl + Win). Shortcuts work while other apps are focused.
        </p>
        <Row label="Push-to-talk" hint="Hold to dictate, release to insert">
          <ShortcutRecorder value={s.pushToTalkHotkey} allowClear onSave={(v) => save({ pushToTalkHotkey: v })} />
        </Row>
        <Row label="Hands-free toggle" hint="Press once to start, again to stop">
          <ShortcutRecorder value={s.toggleHotkey} allowClear={false} onSave={(v) => v && save({ toggleHotkey: v })} />
        </Row>
        <Row label="Cancel" hint="Only while dictating; otherwise the key works as usual">
          <ShortcutRecorder value={s.cancelHotkey} allowClear onSave={(v) => save({ cancelHotkey: v })} />
        </Row>
      </div>

      <div className="card">
        <strong>Microphone</strong>
        <Field label="Input device">
          <select
            className="input"
            value={s.inputDeviceId ?? ""}
            onChange={(e) => save({ inputDeviceId: e.target.value || null })}
          >
            <option value="">System default</option>
            {devices.status === "ok" &&
              devices.data.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                  {d.isDefault ? " (default)" : ""}
                </option>
              ))}
          </select>
        </Field>
        <Field
          label="Keep microphone ready"
          hint="Leaves the mic open after a dictation so the next one starts instantly. Use it if your first words get cut off (common with Bluetooth headsets). While open, Windows shows the mic as in use and Bluetooth headsets stay in call-quality audio. Nothing is recorded until you dictate."
        >
          <select
            className="input"
            value={String(s.keepMicReadySecs)}
            onChange={(e) => save({ keepMicReadySecs: Number(e.target.value) })}
          >
            <option value="0">Off</option>
            <option value="60">For 1 minute</option>
            <option value="300">For 5 minutes</option>
            <option value="1800">For 30 minutes</option>
            <option value={String(KEEP_UNTIL_QUIT)}>Until Voxelle quits</option>
          </select>
        </Field>
        {devices.status === "error" && <p className="error">{devices.error}</p>}
      </div>

      <div className="card">
        <strong>Transcription</strong>
        <Field label="Language">
          <select className="input" value={s.language} onChange={(e) => save({ language: e.target.value })}>
            {LANGUAGES.map(([code, name]) => (
              <option key={code} value={code}>
                {name}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Use GPU when available" hint="Falls back to CPU automatically if the GPU fails">
          <input type="checkbox" checked={s.preferGpu} onChange={(e) => save({ preferGpu: e.target.checked })} />
        </Field>
        <Planned label="Punctuation & capitalization controls" hint="Whisper punctuates by default; overrides come later" />
        <Planned label="Remove filler words" hint="Optional cleanup, off by default" />
      </div>

      <div className="card">
        <strong>Privacy &amp; storage</strong>
        <Field label="Keep a history of transcripts" hint="Off = transcripts are never stored">
          <input
            type="checkbox"
            checked={s.storeTranscripts}
            onChange={(e) => save({ storeTranscripts: e.target.checked })}
          />
        </Field>
        <Field label="Delete history automatically" hint="Older entries are removed on startup and after each dictation">
          <select
            className="input"
            value={s.retentionDays == null ? "" : String(s.retentionDays)}
            onChange={(e) => save({ retentionDays: e.target.value === "" ? null : Number(e.target.value) })}
          >
            <option value="">Never</option>
            <option value="1">After 1 day</option>
            <option value="7">After 7 days</option>
            <option value="30">After 30 days</option>
            <option value="90">After 90 days</option>
          </select>
        </Field>
        <Field label="Offline mode" hint="Blocks all optional network access">
          <input type="checkbox" checked={s.offlineMode} disabled title="Always on in this version" />
        </Field>
      </div>

      <div className="card">
        <strong>App</strong>
        <Field
          label="Keep running in the tray when closed"
          hint="Shortcuts keep working with the window closed. Quit from the tray icon's menu."
        >
          <input type="checkbox" checked={s.closeToTray} onChange={(e) => save({ closeToTray: e.target.checked })} />
        </Field>
        <Planned label="Start Voxelle when I sign in" hint="Coming soon" />
      </div>
    </>
  );
}

export function Settings() {
  const [q] = useCore(() => core.getSettings(), []);
  return (
    <div className="main">
      <h1>Settings</h1>
      <p className="sub">Every setting here maps to real behavior. Planned options are labeled.</p>
      <Gate q={q}>{(s) => <SettingsForm initial={s} />}</Gate>
    </div>
  );
}
