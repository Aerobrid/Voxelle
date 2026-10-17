import { useState, type ReactNode } from "react";
import { core } from "../ipc/bridge";
import type { Settings as SettingsT } from "../ipc/types";
import { errorText, useCore } from "../state/useCore";
import { Gate } from "../components/Gate";
import { ShortcutRecorder } from "../components/ShortcutRecorder";
import { Switch } from "../components/Switch";
import { Select } from "../components/Select";
import { toast } from "../state/store";

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
  const [devices] = useCore(() => core.listInputDevices(), []);

  /** Shortcut saves: the recorder dialog shows a refusal itself. */
  const saveShortcut = (patch: Partial<SettingsT>) => core.updateSettings(patch).then(setS);
  const save = (patch: Partial<SettingsT>) => {
    core.updateSettings(patch).then(setS, (e) => toast(`Not saved: ${errorText(e)}`, "error"));
  };

  return (
    <>

      <div className="group__title">Shortcuts</div>
      <p className="group__intro">
        Click a shortcut, then press the keys you want, including the Windows key. Shortcuts work in
        every app.
      </p>
      <div className="group">
        <Row label="Push to talk" hint="Hold to dictate, release to insert">
          <ShortcutRecorder role="pushToTalk" label="Push to talk" value={s.pushToTalkHotkey} allowClear onSave={(v) => saveShortcut({ pushToTalkHotkey: v })} />
        </Row>
        <Row label="Hands-free" hint="Press once to start, again to stop">
          <ShortcutRecorder role="toggle" label="Hands-free" value={s.toggleHotkey} allowClear={false} onSave={(v) => saveShortcut({ toggleHotkey: v ?? s.toggleHotkey })} />
        </Row>
        <Row label="Cancel" hint="Only while dictating — otherwise the key works as usual">
          <ShortcutRecorder role="cancel" label="Cancel" value={s.cancelHotkey} allowClear onSave={(v) => saveShortcut({ cancelHotkey: v })} />
        </Row>
      </div>

      <div className="group__title">Microphone</div>
      <div className="group">
        <Row label="Input device" hint={devices.status === "error" ? <span className="error">{devices.error}</span> : undefined}>
          <Select
            label="Input device"
            value={s.inputDeviceId ?? ""}
            onChange={(v) => save({ inputDeviceId: v || null })}
            options={[
              { value: "", label: "System default" },
              ...(devices.status === "ok"
                ? devices.data.map((d) => ({ value: d.id, label: d.name + (d.isDefault ? " (default)" : "") }))
                : []),
            ]}
          />
        </Row>
        <Row
          label="Keep microphone ready"
          hint="Use this if your first words get cut off (common with Bluetooth headsets). While ready, Windows shows the mic as in use and Bluetooth audio drops to call quality. Nothing is recorded until you dictate."
        >
          <Select
            label="Keep microphone ready"
            value={String(s.keepMicReadySecs)}
            onChange={(v) => save({ keepMicReadySecs: Number(v) })}
            options={[
              { value: "0", label: "Off" },
              { value: "60", label: "For 1 minute" },
              { value: "300", label: "For 5 minutes" },
              { value: "1800", label: "For 30 minutes" },
              { value: String(KEEP_UNTIL_QUIT), label: "Until Voxelle quits" },
            ]}
          />
        </Row>
      </div>

      <div className="group__title">Transcription</div>
      <div className="group">
        <Row label="Language">
          <Select
            label="Language"
            value={s.language}
            onChange={(v) => save({ language: v })}
            options={LANGUAGES.map(([value, label]) => ({ value, label }))}
          />
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
          <Select
            label="Delete history automatically"
            value={s.retentionDays == null ? "" : String(s.retentionDays)}
            onChange={(v) => save({ retentionDays: v === "" ? null : Number(v) })}
            options={[
              { value: "", label: "Never" },
              { value: "1", label: "After 1 day" },
              { value: "7", label: "After 7 days" },
              { value: "30", label: "After 30 days" },
              { value: "90", label: "After 90 days" },
            ]}
          />
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
