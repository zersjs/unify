import { LOGO_PATH, LOGO_VIEWBOX } from "./logoPath";

export function Logo({ size = 20, title }: { size?: number; title?: string }) {
  const width = (size * 709) / 858;
  return (
    <svg
      className="logo"
      viewBox={LOGO_VIEWBOX}
      width={width}
      height={size}
      role={title ? "img" : undefined}
      aria-hidden={title ? undefined : true}
      aria-label={title}
    >
      <path fill="currentColor" d={LOGO_PATH} />
    </svg>
  );
}
