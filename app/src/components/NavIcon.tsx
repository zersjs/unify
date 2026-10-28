import type { ReactElement } from "react";

export type IconName = "status" | "general" | "devices" | "layout" | "input" | "clipboard" | "network" | "security" | "diagnostics" | "about";

const paths: Record<IconName, ReactElement> = {
  status: (
    <>
      <circle cx="12" cy="12" r="8" />
      <circle cx="12" cy="12" r="3" fill="currentColor" stroke="none" />
    </>
  ),
  general: (
    <>
      <path d="M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1" />
      <circle cx="15" cy="6" r="2" />
      <circle cx="9" cy="12" r="2" />
      <circle cx="17" cy="18" r="2" />
    </>
  ),
  devices: (
    <>
      <path d="M10 14a4 4 0 0 0 5.66 0l3-3a4 4 0 0 0-5.66-5.66l-1 1" />
      <path d="M14 10a4 4 0 0 0-5.66 0l-3 3a4 4 0 0 0 5.66 5.66l1-1" />
    </>
  ),
  layout: (
    <>
      <rect x="2.5" y="6" width="8.5" height="7" rx="1.5" />
      <rect x="13" y="6" width="8.5" height="7" rx="1.5" />
      <path d="M7 17h10" />
    </>
  ),
  input: (
    <>
      <rect x="3" y="6" width="18" height="12" rx="2" />
      <path d="M7 10h1M11 10h1M15 10h2M7 14h10" />
    </>
  ),
  clipboard: (
    <>
      <rect x="6" y="4.5" width="12" height="16" rx="2" />
      <rect x="9" y="3" width="6" height="3" rx="1" />
      <path d="M9 11h6M9 15h4" />
    </>
  ),
  network: (
    <>
      <path d="M4.5 10.5a10.5 10.5 0 0 1 15 0" />
      <path d="M8 14a5.5 5.5 0 0 1 8 0" />
      <circle cx="12" cy="17.5" r="1" fill="currentColor" stroke="none" />
    </>
  ),
  security: <path d="M12 3l7 3v5c0 4.5-3 8-7 10-4-2-7-5.5-7-10V6z" />,
  diagnostics: <path d="M3 12h4l2.5-6 4 12 2.5-6H21" />,
  about: (
    <>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v5" />
      <circle cx="12" cy="8" r="0.9" fill="currentColor" stroke="none" />
    </>
  ),
};

export function NavIcon({ name }: { name: IconName }) {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      {paths[name]}
    </svg>
  );
}
