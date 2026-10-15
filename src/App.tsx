import { useEffect } from "react";
import { TitleBar } from "./components/TitleBar";
import { Sidebar } from "./components/Sidebar";
import { useAppStore } from "./state/store";
import { onDictationResult, onSessionUpdate } from "./ipc/bridge";
import { Dashboard } from "./sections/Dashboard";
import { History } from "./sections/History";
import { ModelManager } from "./sections/ModelManager";
import { Settings } from "./sections/Settings";
import { Privacy } from "./sections/Privacy";
import { Planned } from "./sections/Planned";

const PLANNED_NOTES: Record<string, string> = {
  vocabulary: "A local coach for your speech and vocabulary.",
  assistant: "A local voice assistant with a deny-by-default policy engine.",
  notes: "Local notes with search.",
  "audio-import": "Transcribe audio files on this computer.",
};

export default function App() {
  const section = useAppStore((s) => s.section);
  const setSnapshot = useAppStore((s) => s.setSnapshot);
  const setLastResult = useAppStore((s) => s.setLastResult);

  useEffect(() => {
    const unsubs: Array<() => void> = [];
    onSessionUpdate(setSnapshot).then((fn) => unsubs.push(fn));
    onDictationResult(setLastResult).then((fn) => unsubs.push(fn));
    return () => unsubs.forEach((u) => u());
  }, [setSnapshot, setLastResult]);

  return (
    <div className="app">
      <TitleBar />
      <Sidebar />
      <main className="main">
        {section === "dashboard" && <Dashboard />}
        {section === "history" && <History />}
        {section === "models" && <ModelManager />}
        {section === "settings" && <Settings />}
        {section === "privacy" && <Privacy />}
        {section === "vocabulary" && <Planned title="Vocabulary Coach" note={PLANNED_NOTES.vocabulary} />}
        {section === "assistant" && <Planned title="AI Assistant" note={PLANNED_NOTES.assistant} />}
        {section === "notes" && <Planned title="Notes" note={PLANNED_NOTES.notes} />}
        {section === "audio-import" && <Planned title="Audio Import" note={PLANNED_NOTES["audio-import"]} />}
      </main>
    </div>
  );
}
