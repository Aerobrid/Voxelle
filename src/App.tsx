import { useEffect } from "react";
import { NavRail } from "./components/NavRail";
import { StatusStrip } from "./components/StatusStrip";
import { useAppStore } from "./state/store";
import { onSessionUpdate } from "./ipc/bridge";
import { Dashboard } from "./sections/Dashboard";
import { History } from "./sections/History";
import { ModelManager } from "./sections/ModelManager";
import { Settings } from "./sections/Settings";
import { Privacy } from "./sections/Privacy";
import { Planned } from "./sections/Planned";

const PLANNED_NOTES: Record<string, string> = {
  vocabulary: "A local speech/vocabulary coach.",
  assistant: "A local voice assistant with a deny-by-default policy engine.",
  notes: "Local notes with search.",
  "audio-import": "Transcribe imported audio files locally.",
};

export default function App() {
  const section = useAppStore((s) => s.section);
  const setSnapshot = useAppStore((s) => s.setSnapshot);

  useEffect(() => {
    let unsub = () => {};
    onSessionUpdate(setSnapshot).then((fn) => {
      unsub = fn;
    });
    return () => unsub();
  }, [setSnapshot]);

  return (
    <div className="app">
      <NavRail />
      {section === "dashboard" && <Dashboard />}
      {section === "history" && <History />}
      {section === "models" && <ModelManager />}
      {section === "settings" && <Settings />}
      {section === "privacy" && <Privacy />}
      {section === "vocabulary" && <Planned title="Vocabulary Coach" note={PLANNED_NOTES.vocabulary} />}
      {section === "assistant" && <Planned title="AI Assistant" note={PLANNED_NOTES.assistant} />}
      {section === "notes" && <Planned title="Notes" note={PLANNED_NOTES.notes} />}
      {section === "audio-import" && (
        <Planned title="Audio Import" note={PLANNED_NOTES["audio-import"]} />
      )}
      <StatusStrip />
    </div>
  );
}
