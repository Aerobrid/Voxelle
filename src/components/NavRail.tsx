import { BrandMark } from "./BrandMark";
import { NAV_ITEMS, useAppStore } from "../state/store";

export function NavRail() {
  const section = useAppStore((s) => s.section);
  const setSection = useAppStore((s) => s.setSection);

  return (
    <nav className="rail">
      <div className="rail__brand">
        <BrandMark />
        <span>Voxelle</span>
      </div>
      {NAV_ITEMS.map((item) => (
        <button
          key={item.id}
          className={`rail__item ${section === item.id ? "rail__item--active" : ""}`}
          onClick={() => setSection(item.id)}
        >
          <span>{item.label}</span>
          {item.planned && <span className="rail__tag">planned</span>}
        </button>
      ))}
    </nav>
  );
}
