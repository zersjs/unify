import type { ControlState, Side } from "@/bridge";

const placement: Record<Side, { direction: "row" | "column"; peerFirst: boolean }> = {
  Left: { direction: "row", peerFirst: true },
  Right: { direction: "row", peerFirst: false },
  Top: { direction: "column", peerFirst: true },
  Bottom: { direction: "column", peerFirst: false },
};

function Display({ name, caption, active, empty }: { name: string; caption: string; active: boolean; empty?: boolean }) {
  return (
    <div className={`display${active ? " display-active" : ""}${empty ? " display-empty" : ""}`}>
      <div className="display-panel">
        <span className="display-name">{name}</span>
      </div>
      <span className="display-caption">{caption}</span>
    </div>
  );
}

export function LayoutDiagram({
  side,
  localName,
  localCaption,
  peerName,
  peerCaption,
  control,
  connected,
  description,
}: {
  side: Side;
  localName: string;
  localCaption: string;
  peerName: string | null;
  peerCaption: string;
  control: ControlState;
  connected: boolean;
  description: string;
}) {
  const layout = placement[side];
  const localActive = control !== "controlling";
  const local = <Display key="local" name={localName} caption={localCaption} active={localActive} />;
  const peer = <Display key="peer" name={peerName ?? "—"} caption={peerCaption} active={!localActive} empty={!peerName} />;
  const link = <span key="link" className={`link${connected ? " link-connected" : ""}`} aria-hidden="true" />;
  return (
    <div className="stage" role="img" aria-label={description}>
      <div className="stage-row" data-direction={layout.direction}>
        {layout.peerFirst ? [peer, link, local] : [local, link, peer]}
      </div>
    </div>
  );
}
