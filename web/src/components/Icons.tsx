export function ArrowUpRightIcon({ size = 14 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" aria-hidden="true" className="inline-icon">
      <path
        d="M4.5 11.5L11.5 4.5M11.5 4.5H5.75M11.5 4.5V10.25"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function ArrowRightIcon({ size = 14 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" aria-hidden="true" className="inline-icon">
      <path
        d="M3.25 8H12.75M12.75 8L9 4.25M12.75 8L9 11.75"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function ArrowDownIcon({ size = 14 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" aria-hidden="true" className="inline-icon">
      <path
        d="M8 3.25V12.75M8 12.75L4.25 9M8 12.75L11.75 9"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function MenuIcon({ open }: { open: boolean }) {
  return (
    <svg width={18} height={18} viewBox="0 0 18 18" fill="none" aria-hidden="true">
      {open ? (
        <path d="M4.5 4.5L13.5 13.5M13.5 4.5L4.5 13.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
      ) : (
        <path d="M3 6H15M3 12H15" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
      )}
    </svg>
  );
}
