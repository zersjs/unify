import { useI18n } from "@/i18n";
import { useSeamInspectorViewModel } from "@/viewmodels/useSeamInspectorViewModel";
import { useSectionReveal } from "@/viewmodels/useSectionReveal";
import { PressButton } from "./PressElements";

export function SeamSection() {
  const { t } = useI18n();
  const revealRef = useSectionReveal<HTMLElement>();
  const vm = useSeamInspectorViewModel();

  const cursorX = 56 + vm.visual.cursorU * 408;
  const cursorY = 48 + vm.visual.cursorV * 196;
  const onPeer = vm.visual.cursorU >= 0.5;

  return (
    <section id="overview" ref={revealRef} className="section-band section-subtle" aria-labelledby="seam-heading">
      <div className="container seam-layout" ref={vm.containerRef}>
        <div className="seam-narrative">
          <p className="eyebrow">{t.seam.eyebrow}</p>
          <h2 id="seam-heading" className="section-title">
            <span>{t.seam.headlineLine1}</span>
            <span>{t.seam.headlineLine2}</span>
          </h2>
          <p className="section-body">{t.seam.body}</p>

          <dl className="hairline-specs" aria-label={t.seam.mechanicsTitle}>
            {t.seam.mechanics.map((item) => (
              <div key={item.term} className="hairline-spec-row">
                <dt className="hairline-spec-term">{item.term}</dt>
                <dd className="hairline-spec-detail">{item.detail}</dd>
              </div>
            ))}
          </dl>
        </div>

        <div className="seam-visual-col">
          <div className="seam-diagram-frame">
            <svg viewBox="0 0 520 292" className="seam-svg" role="img" aria-label={t.seam.phasesLabel}>
              <rect x="0" y="0" width="520" height="292" fill="var(--bg-canvas)" />

              <rect
                x="28"
                y="28"
                width="224"
                height="236"
                rx="4"
                fill={!onPeer ? "var(--bg-elevated)" : "var(--bg-subtle)"}
                stroke={!onPeer ? "var(--text-primary)" : "var(--stage-horizon)"}
                strokeWidth={!onPeer ? "1.25" : "1"}
              />
              <rect
                x="268"
                y="28"
                width="224"
                height="236"
                rx="4"
                fill={onPeer ? "var(--bg-elevated)" : "var(--bg-subtle)"}
                stroke={onPeer ? "var(--text-primary)" : "var(--stage-horizon)"}
                strokeWidth={onPeer ? "1.25" : "1"}
              />

              <text x="42" y="50" className="svg-diagram-label">
                {t.seam.diagramMac}
              </text>
              <text x="282" y="50" className="svg-diagram-label">
                {t.seam.diagramWin}
              </text>

              <rect x="246" y="28" width="6" height="36" fill="var(--border-hairline)" />
              <rect x="246" y="228" width="6" height="36" fill="var(--border-hairline)" />
              <text x="238" y="49" textAnchor="end" className="svg-diagram-micro">
                {t.seam.diagramCornerGuard}
              </text>

              <line x1="252" y1="64" x2="252" y2="228" stroke="var(--accent-primary)" strokeWidth="2" strokeDasharray="4 3" />
              <line x1="268" y1="64" x2="268" y2="228" stroke="var(--accent-primary)" strokeWidth="2" strokeDasharray="4 3" />

              <rect
                x="258"
                y={64 + (1 - vm.visual.dwellFill) * 164}
                width="4"
                height={Math.max(0, vm.visual.dwellFill * 164)}
                rx="2"
                fill="var(--accent-primary)"
                opacity="0.85"
              />

              <line
                x1="44"
                y1={cursorY}
                x2="476"
                y2={cursorY}
                stroke="var(--border-hairline)"
                strokeWidth="1"
                strokeDasharray="2 4"
              />
              <text x="476" y={cursorY - 6} textAnchor="end" className="svg-mono-micro">
                ratio 0.62
              </text>

              <g transform={`translate(${cursorX.toFixed(1)} ${cursorY.toFixed(1)})`}>
                <circle cx="0" cy="0" r="14" fill="var(--stage-cursor-halo)" />
                <path
                  d="M-3 -7L-3 9L1.2 5.2L4.1 11.4L6.4 10.3L3.5 4.2L8.8 4.2Z"
                  fill={onPeer ? "var(--bg-elevated)" : "var(--text-primary)"}
                  stroke={onPeer ? "var(--text-primary)" : "var(--bg-elevated)"}
                  strokeWidth="1.25"
                  strokeLinejoin="round"
                />
              </g>
            </svg>

            <div className="seam-phase-strip" role="tablist" aria-label={t.seam.phasesLabel}>
              {vm.phases.map((phase) => {
                const active = vm.activePhase === phase;
                return (
                  <PressButton
                    key={phase}
                    role="tab"
                    aria-selected={active}
                    className="seam-phase-tab"
                    data-active={active ? "true" : "false"}
                    onPointerDown={() => vm.selectPhase(phase)}
                    onClick={(event) => {
                      if (event.detail === 0) {
                        vm.selectPhase(phase);
                      }
                    }}
                  >
                    <span className="seam-phase-name">{t.seam.phases[phase].label}</span>
                  </PressButton>
                );
              })}
            </div>

            <p className="seam-phase-summary" aria-live="polite">
              {t.seam.phases[vm.activePhase].summary}
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
