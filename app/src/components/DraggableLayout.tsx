import type { Side } from "@/bridge";
import { useI18n } from "@/i18n";
import { SIDES } from "@/viewmodels/layoutPhysics";
import { useLayoutDragViewModel } from "@/viewmodels/useLayoutDragViewModel";

const centred = "translate(-50%, -50%)";

export function DraggableLayout({
  side,
  localName,
  peerName,
  onChange,
}: {
  side: Side;
  localName: string;
  peerName: string;
  onChange: (side: Side) => void;
}) {
  const { t } = useI18n();
  const vm = useLayoutDragViewModel(side, onChange);
  const description = t.settings.dragPeer(peerName, t.sideOf[vm.candidate]);

  return (
    <div ref={vm.containerRef} className={`layout-canvas${vm.dragging ? " is-dragging" : ""}`}>
      {vm.dragging
        ? SIDES.map((slot) => (
            <div
              key={slot}
              aria-hidden="true"
              className={`drop-slot${vm.candidate === slot ? " drop-slot-active" : ""}`}
              style={{ transform: `${centred} translate(${vm.slots[slot].x}px, ${vm.slots[slot].y}px)` }}
            />
          ))
        : null}
      <div className="layout-node" style={{ transform: centred }}>
        <div className="screen screen-active">
          <span className="screen-name">{localName}</span>
        </div>
      </div>
      <div
        ref={vm.peerRef}
        className={`layout-node draggable${vm.pressed ? " is-pressed" : ""}`}
        role="button"
        tabIndex={0}
        aria-label={description}
        aria-keyshortcuts="ArrowLeft ArrowRight ArrowUp ArrowDown"
        {...vm.handlers}
      >
        <div className="screen screen-peer screen-face">
          <span className="screen-name">{peerName}</span>
        </div>
      </div>
      <p className="visually-hidden" aria-live="polite">
        {t.status.diagram(peerName, t.sideOf[side], t.control.local)}
      </p>
    </div>
  );
}
