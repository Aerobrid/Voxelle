import { useState, type ReactNode } from "react";
import { NAV_ITEMS, useAppStore, type SectionId } from "../state/store";
import { IconChevron, IconHistory, IconHome, IconModels, IconSettings, IconStats } from "./Icons";
import { PrivacyChip } from "./PrivacyChip";

const ICONS: Partial<Record<SectionId, ReactNode>> = {
  dashboard: <IconHome />,
  history: <IconHistory />,
  stats: <IconStats />,
  models: <IconModels />,
  settings: <IconSettings />,
};

export function Sidebar() {
  const section = useAppStore((s) => s.section);
  const setSection = useAppStore((s) => s.setSection);
  const live = NAV_ITEMS.filter((i) => !i.planned);
  const planned = NAV_ITEMS.filter((i) => i.planned);
  const [soonOpen, setSoonOpen] = useState(planned.some((p) => p.id === section));

  return (
    <nav className="side" aria-label="Sections">
      {live.map((item) => (
        <button
          key={item.id}
          className={`side__item${section === item.id ? " side__item--active" : ""}`}
          aria-current={section === item.id ? "page" : undefined}
          onClick={() => setSection(item.id)}
        >
          {ICONS[item.id]}
          {item.label}
        </button>
      ))}

      <button className="side__group" aria-expanded={soonOpen} onClick={() => setSoonOpen(!soonOpen)}>
        Coming soon
        <IconChevron open={soonOpen} />
      </button>
      {soonOpen &&
        planned.map((item) => (
          <button
            key={item.id}
            className={`side__soon${section === item.id ? " side__soon--active" : ""}`}
            onClick={() => setSection(item.id)}
          >
            {item.label}
            <span className="tag tag--soon">planned</span>
          </button>
        ))}

      <div className="side__spacer" />
      <div className="side__footer">
        <PrivacyChip locality="local" />
        <span>Offline · nothing leaves this device</span>
      </div>
    </nav>
  );
}
