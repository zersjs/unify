export const UNIFY_LOGO_VIEWBOX = "413 106 709 858";

export const UNIFY_LOGO_PATH =
  "M708.2 149.0Q748.0 126.0 748.0 172.0L748.0 424.0Q748.0 442.0 730.0 442.0L644.0 442.0Q610.0 442.0 610.0 476.0L610.0 540.0Q610.0 574.0 644.0 574.0L715.0 574.0Q729.0 574.0 729.0 588.0L729.0 829.0Q729.0 875.0 689.2 852.0L457.2 718.0Q433.0 704.0 433.0 676.0L433.0 336.0Q433.0 308.0 457.2 294.0ZM826.8 921.0Q787.0 944.0 787.0 898.0L787.0 646.0Q787.0 628.0 805.0 628.0L891.0 628.0Q925.0 628.0 925.0 594.0L925.0 530.0Q925.0 496.0 891.0 496.0L820.0 496.0Q806.0 496.0 806.0 482.0L806.0 241.0Q806.0 195.0 845.8 218.0L1077.8 352.0Q1102.0 366.0 1102.0 394.0L1102.0 734.0Q1102.0 762.0 1077.8 776.0Z";

export function Logo({ size = 20, label }: { size?: number; label?: string }) {
  const width = (size * 709) / 858;
  return (
    <svg
      className="unify-mark"
      viewBox={UNIFY_LOGO_VIEWBOX}
      width={width}
      height={size}
      role={label ? "img" : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
    >
      <path fill="currentColor" d={UNIFY_LOGO_PATH} />
    </svg>
  );
}
