import type { ControlState, Side } from "@/bridge";

const placement: Record<Side, { direction: string; peerFirst: boolean }> = {
  Left: { direction: "row", peerFirst: true },
  Right: { direction: "row", peerFirst: false },
  Top: { direction: "column", peerFirst: true },
  Bottom: { direction: "column", peerFirst: false },
};

export function LayoutDiagram({
  side,
  localName,
  peerName,
  control,
  description,
}: {
  side: Side;
  localName: string;
  peerName: string | null;
  control: ControlState;
  description: string;
}) {
  const layout = placement[side];
  const localActive = control !== "controlling";
  const local = (
    <div className={`screen ${localActive ? "screen-active" : ""}`} key="local">
      <span className="screen-name">{localName}</span>
    </div>
  );
  const peer = (
    <div className={`screen screen-peer ${!localActive ? "screen-active" : ""} ${peerName ? "" : "screen-empty"}`} key="peer">
      <span className="screen-name">{peerName ?? "—"}</span>
    </div>
  );
  return (
    <div className="layout-diagram" role="img" aria-label={description} data-direction={layout.direction}>
      {layout.peerFirst ? [peer, local] : [local, peer]}
    </div>
  );
}
