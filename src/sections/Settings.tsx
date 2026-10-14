import { useEffect, useState, type ReactNode } from "react";
import { core } from "../ipc/bridge";
import type { Settings as SettingsT } from "../ipc/types";
import { errorText, useCore } from "../state/useCore";
import { Gate } from "../components/Gate";

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

/** Text input that commits on blur/Enter so shortcuts are validated once, not per keystroke. */
function ShortcutInput({
  value,
  onCommit,
  allowEmpty,
}: {
  value: string | null;
  onCommit: (v: string | null) => void;
  allowEmpty: boolean;
}) {
  const [draft, setDraft] = useState(value ?? "");
  useEffect(() => setDraft(value ?? ""), [value]);
  const commit = () => {
    const v = draft.trim();
    if (v === (value ?? "")) return;
    onCommit(v === "" && allowEmpty ? null : v);
  };
  return (
    <input
      className="input mono"
      value={draft}
      placeholder={allowEmpty ? "off" : ""}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
    />
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
      {saved && !error && <p className="muted small">Saved. (Settings persist across restarts once storage lands — see feature status.)</p>}

      <div className="card">
        <strong>Shortcuts</strong>
        <p className="muted small">
          Examples: <kbd>Ctrl+Shift+Space</kbd>, <kbd>F9</kbd>, <kbd>RightCtrl</kbd> (hold). They work while
          other apps are focused.
        </p>
        <Field label="Toggle (press to start, again to stop)">
          <ShortcutInput value={s.toggleHotkey} allowEmpty={false} onCommit={(v) => v && save({ toggleHotkey: v })} />
        </Field>
        <Field label="Push-to-talk (hold to dictate)" hint="Leave empty to turn off">
          <ShortcutInput value={s.pushToTalkHotkey} allowEmpty onCommit={(v) => save({ pushToTalkHotkey: v })} />
        </Field>
        <Field label="Cancel (only while dictating)" hint="Leave empty to turn off">
          <ShortcutInput value={s.cancelHotkey} allowEmpty onCommit={(v) => save({ cancelHotkey: v })} />
        </Field>
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
        <Planned label="Retention period" hint="Auto-delete history after N days" />
        <Field label="Offline mode" hint="Blocks all optional network access">
          <input type="checkbox" checked={s.offlineMode} disabled title="Always on in this version" />
        </Field>
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
