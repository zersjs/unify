import { siteConfig } from "@/config/site";
import { useI18n } from "@/i18n";
import { prefersReducedMotion } from "@/viewmodels/motion";
import { useWorkspaceStageViewModel } from "@/viewmodels/useWorkspaceStageViewModel";
import { ArrowDownIcon, ArrowUpRightIcon } from "./Icons";
import { UNIFY_LOGO_PATH } from "./Logo";
import { PressButton, PressLink } from "./PressElements";

function PointerArrow({ x, y, variant }: { x: number; y: number; variant: "macos" | "windows" }) {
  const fill = variant === "macos" ? "var(--text-primary)" : "var(--bg-elevated)";
  const stroke = variant === "macos" ? "var(--bg-elevated)" : "var(--text-primary)";
  return (
    <g transform={`translate(${x.toFixed(2)} ${y.toFixed(2)})`}>
      <path
        d="M0 0L0 17.5L4.6 13.4L7.7 20.2L10.3 19.0L7.2 12.3L13.0 12.3Z"
        fill={fill}
        stroke={stroke}
        strokeWidth="1.35"
        strokeLinejoin="round"
      />
    </g>
  );
}

export function HeroSection() {
  const { t } = useI18n();
  const stage = useWorkspaceStageViewModel();

  const macDisplayLeft = 88;
  const macDisplayRight = 516;
  const macDisplayTop = 146;
  const macDisplayHeight = 274;

  const winDisplayLeft = 604;
  const winDisplayRight = 1100;
  const winDisplayTop = 92;
  const winDisplayHeight = 298;

  const normV = (stage.pose.v - 0.22) / (0.8 - 0.22);
  const clampedNormV = Math.min(1, Math.max(0, normV));

  const macY = macDisplayTop + 34 + clampedNormV * (macDisplayHeight - 62);
  const winY = winDisplayTop + 34 + clampedNormV * (winDisplayHeight - 68);

  const macProgress = (stage.pose.u - 0.08) / (0.5 - 0.08);
  const macCursorX = macDisplayLeft + 24 + macProgress * (macDisplayRight - macDisplayLeft - 24);

  const winProgress = (stage.pose.u - 0.5) / (0.92 - 0.5);
  const winCursorX = winDisplayLeft + winProgress * (winDisplayRight - winDisplayLeft - 32);

  const macFocused = stage.activeMachine === "macos";
  const winFocused = stage.activeMachine === "windows";
  const seamOpacity = Math.min(1, Math.max(0.14, stage.seamGlow));

  const scrollToOverview = () => {
    const target = document.getElementById("overview");
    if (target) {
      target.scrollIntoView({
        behavior: prefersReducedMotion() ? "auto" : "smooth",
        block: "start",
      });
    }
  };

  return (
    <section id="top" className="hero-section" aria-labelledby="hero-heading">
      <div className="container">
        <div className="hero-editorial-grid">
          <div className="hero-headline-col">
            <p className="eyebrow">{t.hero.eyebrow}</p>
            <h1 id="hero-heading" className="hero-title">
              <span>{t.hero.headlineLine1}</span>
              <span>{t.hero.headlineLine2}</span>
            </h1>
          </div>

          <div className="hero-copy-col">
            <p className="hero-lead">{t.hero.supportingCopy}</p>
            <div className="hero-actions">
              <PressLink
                href={siteConfig.repositoryUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="btn-primary"
              >
                <span>{t.hero.primaryCta}</span>
                <ArrowUpRightIcon size={15} />
              </PressLink>
              <PressLink
                href="#overview"
                className="btn-secondary"
                onClick={(event) => {
                  event.preventDefault();
                  scrollToOverview();
                }}
              >
                <span>{t.hero.secondaryCta}</span>
                <ArrowDownIcon size={15} />
              </PressLink>
            </div>
            <p className="hero-specs-note">{t.hero.specsLine}</p>
          </div>
        </div>

        <div className="hero-stage-shell">
          <div
            ref={stage.stageRef}
            className="hero-stage-canvas"
            data-dragging={stage.dragging ? "true" : "false"}
            role="region"
            tabIndex={0}
            aria-label={t.hero.stage.ariaLabel}
            onPointerDown={stage.onStagePointerDown}
            onPointerMove={stage.onStagePointerMove}
            onPointerUp={stage.onStagePointerUp}
            onPointerCancel={stage.onStagePointerCancel}
            onKeyDown={stage.onStageKeyDown}
          >
            <svg viewBox="0 0 1160 486" className="hero-stage-svg" aria-hidden="true">
              <defs>
                <clipPath id="hero-mac-screen">
                  <rect x={macDisplayLeft} y={macDisplayTop} width={428} height={macDisplayHeight} rx="5" />
                </clipPath>
                <clipPath id="hero-win-screen">
                  <rect x={winDisplayLeft} y={winDisplayTop} width={496} height={winDisplayHeight} rx="3" />
                </clipPath>
              </defs>

              <rect x="0" y="0" width="1160" height="442" fill="var(--stage-wall)" />
              <rect x="0" y="442" width="1160" height="44" fill="var(--stage-desk)" />
              <line x1="0" y1="442" x2="1160" y2="442" stroke="var(--stage-horizon)" strokeWidth="1" />

              <ellipse cx="302" cy="443" rx="236" ry="5.5" fill="var(--stage-contact-shadow)" />
              <ellipse cx="852" cy="443" rx="108" ry="4.5" fill="var(--stage-contact-shadow)" />

              <rect
                x="80"
                y="138"
                width="444"
                height="292"
                rx="12"
                fill="var(--stage-mac-shell)"
                stroke="var(--stage-mac-rim)"
                strokeWidth="1"
              />

              <g clipPath="url(#hero-mac-screen)">
                <rect x="88" y="146" width="428" height="274" fill="var(--stage-mac-wallpaper)" />
                <rect x="88" y="146" width="428" height="19" fill="var(--stage-mac-menubar)" />
                <line x1="88" y1="165" x2="516" y2="165" stroke="var(--stage-mac-menubar-line)" strokeWidth="1" />

                <circle cx="101" cy="155.5" r="3.2" fill="var(--text-primary)" />
                <text x="113" y="159" className="svg-os-menu svg-os-menu-strong">
                  {t.hero.stage.macMenuFinder}
                </text>
                <text x="153" y="159" className="svg-os-menu">
                  {t.hero.stage.macMenuFile}
                </text>
                <text x="179" y="159" className="svg-os-menu">
                  {t.hero.stage.macMenuEdit}
                </text>
                <text x="206" y="159" className="svg-os-menu">
                  {t.hero.stage.macMenuView}
                </text>

                <g transform="translate(458 150) scale(0.0115) translate(-413 -106)">
                  <path fill="var(--text-primary)" d={UNIFY_LOGO_PATH} />
                </g>
                <text x="478" y="159" className="svg-os-menu">
                  14:32
                </text>

                <rect x="278" y="146" width="48" height="9" rx="3" fill="var(--stage-mac-shell)" />

                <g opacity={macFocused ? 0.94 : 0.76}>
                  <rect
                    x="108"
                    y="180"
                    width="244"
                    height="122"
                    rx="6"
                    fill="var(--stage-mac-win-bg)"
                    stroke="var(--stage-mac-win-border)"
                    strokeWidth="1"
                  />
                  <line x1="108" y1="201" x2="352" y2="201" stroke="var(--stage-mac-win-rule)" strokeWidth="1" />
                  <circle cx="120" cy="190.5" r="3" fill="var(--stage-mac-dot-idle)" />
                  <circle cx="130" cy="190.5" r="3" fill="var(--stage-mac-dot-idle)" />
                  <circle cx="140" cy="190.5" r="3" fill="var(--stage-mac-dot-idle)" />
                  <text x="154" y="194" className="svg-win-caption">
                    {t.hero.stage.macWindowTwoTitle}
                  </text>
                  <text x="122" y="226" className="svg-doc-line">
                    {t.hero.stage.macWindowTwoLine1}
                  </text>
                  <text x="122" y="246" className="svg-doc-muted">
                    {t.hero.stage.macWindowTwoLine2}
                  </text>
                </g>

                <g>
                  <rect
                    x="154"
                    y="224"
                    width="342"
                    height="156"
                    rx="6"
                    fill="var(--bg-elevated)"
                    stroke={macFocused ? "var(--accent-primary)" : "var(--stage-horizon)"}
                    strokeWidth={macFocused ? "1.4" : "1"}
                  />
                  <rect
                    x="154"
                    y="224"
                    width="342"
                    height="23"
                    rx="6"
                    fill={macFocused ? "var(--stage-mac-header-active)" : "var(--stage-mac-header-idle)"}
                  />
                  <line x1="154" y1="247" x2="496" y2="247" stroke="var(--border-hairline)" strokeWidth="1" />
                  <circle
                    cx="167"
                    cy="235.5"
                    r="3.2"
                    fill={macFocused ? "var(--text-primary)" : "var(--stage-mac-dot-idle)"}
                  />
                  <circle
                    cx="177"
                    cy="235.5"
                    r="3.2"
                    fill={macFocused ? "var(--stage-mac-dot-mid)" : "var(--stage-mac-dot-idle)"}
                  />
                  <circle
                    cx="187"
                    cy="235.5"
                    r="3.2"
                    fill={macFocused ? "var(--stage-mac-dot-soft)" : "var(--stage-mac-dot-idle)"}
                  />
                  <text x="204" y="239" className="svg-win-caption">
                    {t.hero.stage.macWindowOneTitle}
                  </text>
                  <text x="170" y="274" className="svg-mono-line svg-mono-strong">
                    {t.hero.stage.macWindowOneLine1}
                  </text>
                  <text x="170" y="297" className="svg-mono-line">
                    {t.hero.stage.macWindowOneLine2}
                  </text>
                  <text x="170" y="320" className="svg-mono-accent">
                    {t.hero.stage.macWindowOneLine3}
                  </text>
                  {macFocused ? (
                    <rect x="170" y="334" width="6.5" height="12" fill="var(--accent-primary)" opacity="0.85" />
                  ) : null}
                </g>

                <rect
                  x="230"
                  y="398"
                  width="144"
                  height="15"
                  rx="4.5"
                  fill="var(--stage-mac-dock)"
                  stroke="var(--stage-horizon)"
                  strokeWidth="0.8"
                />
                <rect x="246" y="401.5" width="8" height="8" rx="2" fill="var(--stage-mac-dot-mid)" />
                <rect x="262" y="401.5" width="8" height="8" rx="2" fill="var(--stage-mac-lip)" />
                <rect x="278" y="401.5" width="8" height="8" rx="2" fill="var(--accent-primary)" />
                <rect x="294" y="401.5" width="8" height="8" rx="2" fill="var(--stage-mac-lip)" />
                <rect x="310" y="401.5" width="8" height="8" rx="2" fill="var(--stage-mac-dot-mid)" />

                {stage.pose.u < 0.54 ? <PointerArrow x={macCursorX} y={macY} variant="macos" /> : null}
              </g>

              <rect
                x="54"
                y="428"
                width="496"
                height="12"
                rx="2.5"
                fill="var(--stage-mac-deck)"
                stroke="var(--stage-mac-deck-stroke)"
                strokeWidth="0.8"
              />
              <rect x="266" y="428" width="72" height="3.5" rx="1.5" fill="var(--stage-mac-lip)" />
              <rect x="86" y="440" width="24" height="2" rx="1" fill="var(--stage-mac-foot)" />
              <rect x="494" y="440" width="24" height="2" rx="1" fill="var(--stage-mac-foot)" />

              <rect
                x="835"
                y="398"
                width="34"
                height="37"
                fill="var(--stage-win-stand)"
                stroke="var(--stage-win-stand-stroke)"
                strokeWidth="0.8"
              />
              <rect
                x="776"
                y="434"
                width="152"
                height="7"
                rx="1.5"
                fill="var(--stage-win-base)"
                stroke="var(--stage-mac-deck-stroke)"
                strokeWidth="0.8"
              />
              <rect x="786" y="441" width="132" height="1" fill="var(--stage-mac-foot)" />

              <rect
                x="596"
                y="84"
                width="512"
                height="316"
                rx="7"
                fill="var(--stage-win-shell)"
                stroke="var(--stage-win-rim)"
                strokeWidth="1"
              />

              <g clipPath="url(#hero-win-screen)">
                <rect x="604" y="92" width="496" height="298" fill="var(--stage-win-wallpaper)" />

                <g opacity={winFocused ? 0.92 : 0.74}>
                  <rect
                    x="626"
                    y="114"
                    width="256"
                    height="148"
                    rx="4"
                    fill="var(--stage-win-bg-muted)"
                    stroke="var(--stage-win-border)"
                    strokeWidth="1"
                  />
                  <line x1="626" y1="136" x2="882" y2="136" stroke="var(--stage-win-rule)" strokeWidth="1" />
                  <text x="638" y="129" className="svg-win-caption">
                    {t.hero.stage.winWindowTwoTitle}
                  </text>
                  <line x1="836" y1="125" x2="843" y2="125" stroke="var(--stage-win-glyph)" strokeWidth="1" />
                  <rect x="851" y="121.5" width="6.5" height="6.5" fill="none" stroke="var(--stage-win-glyph)" strokeWidth="1" />
                  <path d="M866 121.5L872.5 128M872.5 121.5L866 128" stroke="var(--stage-win-glyph)" strokeWidth="1" />
                  <text x="640" y="162" className="svg-mono-line">
                    {t.hero.stage.winWindowTwoLine1}
                  </text>
                  <text x="640" y="184" className="svg-mono-line">
                    {t.hero.stage.winWindowTwoLine2}
                  </text>
                </g>

                <g>
                  <rect
                    x="686"
                    y="150"
                    width="390"
                    height="194"
                    rx="4"
                    fill="var(--bg-elevated)"
                    stroke={winFocused ? "var(--accent-primary)" : "var(--stage-win-border)"}
                    strokeWidth={winFocused ? "1.4" : "1"}
                  />
                  <rect
                    x="686"
                    y="150"
                    width="390"
                    height="25"
                    rx="4"
                    fill={winFocused ? "var(--stage-win-header-active)" : "var(--stage-win-header-idle)"}
                  />
                  <line x1="686" y1="175" x2="1076" y2="175" stroke="var(--stage-win-rule)" strokeWidth="1" />
                  <text x="700" y="166" className="svg-win-caption">
                    {t.hero.stage.winWindowOneTitle}
                  </text>
                  <line x1="1024" y1="162.5" x2="1031" y2="162.5" stroke="var(--stage-win-glyph)" strokeWidth="1" />
                  <rect
                    x="1040"
                    y="159"
                    width="6.5"
                    height="6.5"
                    fill="none"
                    stroke="var(--stage-win-glyph)"
                    strokeWidth="1"
                  />
                  <path d="M1056 159L1062.5 165.5M1062.5 159L1056 165.5" stroke="var(--stage-win-glyph)" strokeWidth="1" />

                  <text x="702" y="204" className="svg-line-num">
                    15
                  </text>
                  <text x="728" y="204" className="svg-mono-line svg-mono-strong">
                    {t.hero.stage.winWindowOneLine1}
                  </text>
                  <text x="702" y="228" className="svg-line-num">
                    16
                  </text>
                  <text x="728" y="228" className="svg-mono-line">
                    {t.hero.stage.winWindowOneLine2}
                  </text>
                  <text x="702" y="252" className="svg-line-num">
                    17
                  </text>
                  <text x="728" y="252" className="svg-mono-accent">
                    {t.hero.stage.winWindowOneLine3}
                  </text>
                  <text x="702" y="276" className="svg-line-num">
                    18
                  </text>
                  <text x="728" y="276" className="svg-mono-line">
                    {t.hero.stage.winWindowOneLine4}
                  </text>

                  {winFocused ? <rect x="894" y="241" width="1.8" height="14" fill="var(--accent-primary)" /> : null}
                </g>

                <rect x="604" y="366" width="496" height="24" fill="var(--stage-win-taskbar)" />
                <line x1="604" y1="366" x2="1100" y2="366" stroke="var(--stage-win-border)" strokeWidth="1" />
                <g transform="translate(830 373)">
                  <rect x="0" y="0" width="4.2" height="4.2" fill="var(--stage-win-start)" />
                  <rect x="5.4" y="0" width="4.2" height="4.2" fill="var(--stage-win-start)" />
                  <rect x="0" y="5.4" width="4.2" height="4.2" fill="var(--stage-win-start)" />
                  <rect x="5.4" y="5.4" width="4.2" height="4.2" fill="var(--stage-win-start)" />
                </g>
                <rect x="850" y="373.5" width="9" height="9" rx="1.5" fill="var(--stage-win-tray)" />
                <rect x="867" y="373.5" width="9" height="9" rx="1.5" fill="var(--accent-primary)" />
                <text x="1038" y="381.5" className="svg-os-menu">
                  ENG · 14:32
                </text>

                {stage.pose.u >= 0.46 ? <PointerArrow x={winCursorX} y={winY} variant="windows" /> : null}
              </g>

              <g opacity={seamOpacity}>
                <rect x="514.5" y={macY - 14} width="2.5" height="32" rx="1.2" fill="var(--accent-primary)" />
                <rect x="604" y={winY - 14} width="2.5" height="32" rx="1.2" fill="var(--accent-primary)" />
                <path
                  d={`M 517 ${macY + 2} C 552 ${macY + 2}, 568 ${winY + 2}, 604 ${winY + 2}`}
                  fill="none"
                  stroke="var(--accent-primary)"
                  strokeWidth="1.25"
                  strokeDasharray="3 3"
                />
              </g>
            </svg>
          </div>

          <div className="hero-stage-footer">
            <div className="stage-readout-group">
              <span className="stage-readout-item">
                <span className="stage-status-dot" data-machine={stage.activeMachine} />
                <span className="stage-readout-label">{t.hero.stage.activeControlLabel}:</span>
                <strong className="stage-readout-value">
                  {stage.activeMachine === "macos" ? t.hero.stage.macLabel : t.hero.stage.winLabel}
                </strong>
              </span>
              <span className="stage-readout-sep" aria-hidden="true">
                ·
              </span>
              <span className="stage-readout-item mono">
                {t.hero.stage.handoffLabel} {stage.ratioDisplay}
              </span>
              <span className="stage-readout-sep" aria-hidden="true">
                ·
              </span>
              <span className="stage-readout-item mono">{t.hero.stage.transportLabel}</span>
            </div>

            <div className="stage-controls" role="group" aria-label={t.hero.stage.activeControlLabel}>
              <PressButton
                className="stage-chip"
                data-active={stage.activeMachine === "macos" ? "true" : "false"}
                onPointerDown={() => stage.focusMachine("macos")}
                onClick={(event) => {
                  if (event.detail === 0) {
                    stage.focusMachine("macos");
                  }
                }}
              >
                {t.hero.stage.switchToMac}
              </PressButton>
              <PressButton
                className="stage-chip"
                data-active={stage.activeMachine === "windows" ? "true" : "false"}
                onPointerDown={() => stage.focusMachine("windows")}
                onClick={(event) => {
                  if (event.detail === 0) {
                    stage.focusMachine("windows");
                  }
                }}
              >
                {t.hero.stage.switchToWin}
              </PressButton>
              {stage.mode === "manual" ? (
                <PressButton
                  className="stage-chip stage-chip-quiet"
                  onPointerDown={() => stage.resumeAutoCycle()}
                  onClick={(event) => {
                    if (event.detail === 0) {
                      stage.resumeAutoCycle();
                    }
                  }}
                >
                  {t.hero.stage.resumeCycle}
                </PressButton>
              ) : null}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
