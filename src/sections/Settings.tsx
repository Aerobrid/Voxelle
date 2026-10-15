import { useState, type ReactNode } from "react";
import { core } from "../ipc/bridge";
import type { Settings as SettingsT } from "../ipc/types";
import { errorText, useCore } from "../state/useCore";
import { Gate } from "../components/Gate";
import { ShortcutRecorder } from "../components/ShortcutRecorder";
import { Switch } from "../components/Switch";

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

/** One settings row: label + explanation on the left, the control on the right. Not a <label>,
 *  so rows with several buttons never forward clicks to the first one. */
function Row({ label, hint, children }: { label: string; hint?: ReactNode; children: ReactNode }) {
  return (
    <div className="field">
      <span className="field__label">
        <span>{label}</span>
        {hint && <span className="field__hint">{hint}</span>}
      </span>
      <span className="field__control">{children}</span>
    </div>
  );
}

function Planned({ label, hint }: { label: string; hint: string }) {
  return (
    <Row label={label} hint={hint}>
      <span className="tag tag--soon">planned</span>
    </Row>
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

      <div className="group__title">Shortcuts</div>
      <p className="group__intro">
        Click Change, then press the keys you want, including the Windows key (for example Ctrl + Win).
        They work in every app.
      </p>
      <div className="group">
        <Row label="Push to talk" hint="Hold to dictate, release to insert">
          <ShortcutRecorder value={s.pushToTalkHotkey} allowClear onSave={(v) => save({ pushToTalkHotkey: v })} />
        </Row>
        <Row label="Hands-free" hint="Press once to start, again to stop">
          <ShortcutRecorder value={s.toggleHotkey} allowClear={false} onSave={(v) => v && save({ toggleHotkey: v })} />
        </Row>
        <Row label="Cancel" hint="Only while dictating — otherwise the key works as usual">
          <ShortcutRecorder value={s.cancelHotkey} allowClear onSave={(v) => save({ cancelHotkey: v })} />
        </Row>
      </div>

      <div className="group__title">Microphone</div>
      <div className="group">
        <Row label="Input device" hint={devices.status === "error" ? <span className="error">{devices.error}</span> : undefined}>
          <select
            className="input"
            value={s.inputDeviceId ?? ""}
            onChange={(e) => save({ inputDeviceId: e.target.value || null })}
            aria-label="Input device"
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
        </Row>
        <Row
          label="Keep microphone ready"
          hint="Use this if your first words get cut off (common with Bluetooth headsets). While ready, Windows shows the mic as in use and Bluetooth audio drops to call quality. Nothing is recorded until you dictate."
        >
          <select
            className="input"
            value={String(s.keepMicReadySecs)}
            onChange={(e) => save({ keepMicReadySecs: Number(e.target.value) })}
            aria-label="Keep microphone ready"
          >
            <option value="0">Off</option>
            <option value="60">For 1 minute</option>
            <option value="300">For 5 minutes</option>
            <option value="1800">For 30 minutes</option>
            <option value={String(KEEP_UNTIL_QUIT)}>Until Voxelle quits</option>
          </select>
        </Row>
      </div>

      <div className="group__title">Transcription</div>
      <div className="group">
        <Row label="Language">
          <select className="input" value={s.language} onChange={(e) => save({ language: e.target.value })} aria-label="Language">
            {LANGUAGES.map(([code, name]) => (
              <option key={code} value={code}>
                {name}
              </option>
            ))}
          </select>
        </Row>
        <Row label="Use the GPU" hint="Much faster when available. Falls back to the CPU automatically.">
          <Switch label="Use the GPU" checked={s.preferGpu} onChange={(v) => save({ preferGpu: v })} />
        </Row>
        <Planned label="Punctuation & capitalization" hint="Automatic for now; controls come later" />
        <Planned label="Remove filler words" hint="Optional cleanup, off by default" />
      </div>

      <div className="group__title">Privacy & storage</div>
      <div className="group">
        <Row label="Keep a history" hint="When off, transcripts are never stored">
          <Switch label="Keep a history" checked={s.storeTranscripts} onChange={(v) => save({ storeTranscripts: v })} />
        </Row>
        <Row label="Delete history automatically" hint="Removes older entries at startup and after each dictation">
          <select
            className="input"
            value={s.retentionDays == null ? "" : String(s.retentionDays)}
            onChange={(e) => save({ retentionDays: e.target.value === "" ? null : Number(e.target.value) })}
            aria-label="Delete history automatically"
          >
            <option value="">Never</option>
            <option value="1">After 1 day</option>
            <option value="7">After 7 days</option>
            <option value="30">After 30 days</option>
            <option value="90">After 90 days</option>
          </select>
        </Row>
        <Row label="Offline mode" hint="Blocks all optional network access. Always on in this version.">
          <Switch label="Offline mode" checked={s.offlineMode} disabled />
        </Row>
      </div>

      <div className="group__title">App</div>
      <div className="group">
        <Row label="Keep running in the tray when closed" hint="Shortcuts keep working with the window closed. Quit from the tray icon.">
          <Switch label="Keep running in the tray" checked={s.closeToTray} onChange={(v) => save({ closeToTray: v })} />
        </Row>
        <Planned label="Start Voxelle when I sign in" hint="Coming soon" />
      </div>

      <p className="faint small" style={{ marginTop: 16 }} aria-live="polite">
        {saved && !error ? "Saved." : " "}
      </p>
    </>
  );
}

export function Settings() {
  const [q] = useCore(() => core.getSettings(), []);
  return (
    <div className="page">
      <div className="page__head">
        <h1>Settings</h1>
        <p className="sub">Every option here does something real; planned ones are labeled.</p>
      </div>
      <Gate q={q}>{(s) => <SettingsForm initial={s} />}</Gate>
    </div>
  );
}
