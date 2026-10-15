// Original line icons for Voxelle (24×24 grid, 1.7 stroke, round caps). Drawn for this app.
import type { ReactNode } from "react";

function Icon({ children }: { children: ReactNode }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      {children}
    </svg>
  );
}

/** Home = the brand waveform (three bars). */
export const IconHome = () => (
  <Icon>
    <path d="M7 9v6M12 5v14M17 9v6" />
  </Icon>
);

export const IconHistory = () => (
  <Icon>
    <path d="M4 12a8 8 0 1 0 2.4-5.7" />
    <path d="M4 4v4h4" />
    <path d="M12 8v4l3 2" />
  </Icon>
);

export const IconModels = () => (
  <Icon>
    <rect x="6" y="6" width="12" height="12" rx="3" />
    <path d="M10 2.5V6M14 2.5V6M10 18v3.5M14 18v3.5M2.5 10H6M2.5 14H6M18 10h3.5M18 14h3.5" />
  </Icon>
);

export const IconSettings = () => (
  <Icon>
    <path d="M4 7h9M17 7h3M4 17h3M11 17h9" />
    <circle cx="15" cy="7" r="2" />
    <circle cx="9" cy="17" r="2" />
  </Icon>
);

export const IconPrivacy = () => (
  <Icon>
    <path d="M12 3l7 3v5c0 4.4-3 8.3-7 10-4-1.7-7-5.6-7-10V6l7-3z" />
    <path d="M9 12l2 2 4-4" />
  </Icon>
);

export const IconCopy = () => (
  <Icon>
    <rect x="9" y="9" width="11" height="11" rx="2.5" />
    <path d="M5 15V6.5A2.5 2.5 0 0 1 7.5 4H15" />
  </Icon>
);

export const IconTrash = () => (
  <Icon>
    <path d="M4 7h16M9 7V4.5h6V7M6.5 7l1 12.5h9l1-12.5" />
  </Icon>
);

export const IconChevron = ({ open }: { open: boolean }) => (
  <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden
    style={{ transform: open ? "rotate(90deg)" : "none", transition: "transform 120ms" }}>
    <path d="M9 6l6 6-6 6" />
  </svg>
);

// Window buttons: thin 10×10 glyphs.
export const GlyphMinimize = () => (
  <svg viewBox="0 0 10 10" aria-hidden>
    <path d="M0 5h10" stroke="currentColor" strokeWidth="1" />
  </svg>
);

export const GlyphMaximize = () => (
  <svg viewBox="0 0 10 10" aria-hidden>
    <rect x="0.5" y="0.5" width="9" height="9" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1" />
  </svg>
);

export const GlyphRestore = () => (
  <svg viewBox="0 0 10 10" aria-hidden>
    <rect x="0.5" y="2.5" width="7" height="7" rx="1.2" fill="none" stroke="currentColor" strokeWidth="1" />
    <path d="M2.5 2.5V1.7c0-.7.5-1.2 1.2-1.2h4.6c.7 0 1.2.5 1.2 1.2v4.6c0 .7-.5 1.2-1.2 1.2H7.5" fill="none" stroke="currentColor" strokeWidth="1" />
  </svg>
);

export const GlyphClose = () => (
  <svg viewBox="0 0 10 10" aria-hidden>
    <path d="M0.5 0.5l9 9M9.5 0.5l-9 9" stroke="currentColor" strokeWidth="1" />
  </svg>
);
