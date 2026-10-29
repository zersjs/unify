import { useI18n } from "@/i18n";
import { useClipboardDemoViewModel } from "@/viewmodels/useClipboardDemoViewModel";
import { useSectionReveal } from "@/viewmodels/useSectionReveal";
import { PressButton } from "./PressElements";

export function ClipboardSection() {
  const { t } = useI18n();
  const revealRef = useSectionReveal<HTMLElement>();
  const vm = useClipboardDemoViewModel();

  const macIsSource = vm.direction === "mac-to-win";
  const activeSnippet = macIsSource ? t.clipboard.snippetMacSource : t.clipboard.snippetWinSource;
  const sourceCopied = vm.step === "copied" || vm.step === "transmitting" || vm.step === "pasted";
  const destinationPasted = vm.step === "pasted";

  return (
    <section id="clipboard" ref={revealRef} className="section-band" aria-labelledby="clipboard-heading">
      <div className="container" ref={vm.sectionRef}>
        <div className="clipboard-header-grid">
          <div>
            <p className="eyebrow">{t.clipboard.eyebrow}</p>
            <h2 id="clipboard-heading" className="section-title">
              <span>{t.clipboard.headlineLine1}</span>
              <span>{t.clipboard.headlineLine2}</span>
            </h2>
          </div>

          <div className="clipboard-copy-stack">
            <p className="section-body">{t.clipboard.body}</p>
            <div className="clipboard-toolbar">
              <div className="segmented-bar" role="group" aria-label={t.clipboard.eyebrow}>
                {(["mac-to-win", "win-to-mac"] as const).map((dir) => (
                  <PressButton
                    key={dir}
                    className="segment-btn"
                    data-active={vm.direction === dir ? "true" : "false"}
                    aria-pressed={vm.direction === dir}
                    onPointerDown={() => vm.selectDirection(dir)}
                    onClick={(event) => {
                      if (event.detail === 0) {
                        vm.selectDirection(dir);
                      }
                    }}
                  >
                    {t.clipboard.directions[dir]}
                  </PressButton>
                ))}
              </div>

              <PressButton
                className="btn-quiet"
                onPointerDown={() => vm.replay()}
                onClick={(event) => {
                  if (event.detail === 0) {
                    vm.replay();
                  }
                }}
              >
                {t.clipboard.replayButton}
              </PressButton>
            </div>
          </div>
        </div>

        <div className="clipboard-stage" data-step={vm.step} data-direction={vm.direction}>
          <div className="os-pane os-pane-mac" data-role={macIsSource ? "source" : "target"}>
            <div className="os-pane-bar">
              <div className="mac-window-dots" aria-hidden="true">
                <span />
                <span />
                <span />
              </div>
              <span className="os-pane-title">{t.clipboard.macAppTitle}</span>
              <span className="os-keycap mono">
                {macIsSource ? t.clipboard.macShortcut : "⌘V"}
              </span>
            </div>
            <div className="os-pane-body">
              <code
                className="os-snippet mono selectable"
                data-highlighted={macIsSource && sourceCopied ? "true" : undefined}
                data-visible={!macIsSource && !destinationPasted ? "false" : "true"}
              >
                {activeSnippet}
              </code>
              <span className="os-state-tag">
                {macIsSource
                  ? sourceCopied
                    ? t.clipboard.copiedBadge
                    : t.clipboard.stepLabels.idle
                  : destinationPasted
                    ? t.clipboard.pastedBadge
                    : t.clipboard.waitingBadge}
              </span>
            </div>
          </div>

          <div className="clipboard-wire" aria-live="polite">
            <div className="clipboard-wire-track" aria-hidden="true">
              <span className="clipboard-wire-pulse" data-active={vm.step === "transmitting" ? "true" : "false"} />
            </div>
            <p className="clipboard-wire-status mono">{t.clipboard.stepLabels[vm.step]}</p>
          </div>

          <div className="os-pane os-pane-win" data-role={!macIsSource ? "source" : "target"}>
            <div className="os-pane-bar">
              <span className="os-pane-title">{t.clipboard.winAppTitle}</span>
              <div className="os-win-right">
                <span className="os-keycap mono">
                  {!macIsSource ? "Ctrl + C" : t.clipboard.winShortcut}
                </span>
                <span className="win-caption-glyphs" aria-hidden="true">
                  — □ ×
                </span>
              </div>
            </div>
            <div className="os-pane-body">
              <code
                className="os-snippet mono selectable"
                data-highlighted={!macIsSource && sourceCopied ? "true" : undefined}
                data-visible={macIsSource && !destinationPasted ? "false" : "true"}
              >
                {activeSnippet}
              </code>
              <span className="os-state-tag">
                {!macIsSource
                  ? sourceCopied
                    ? t.clipboard.copiedBadge
                    : t.clipboard.stepLabels.idle
                  : destinationPasted
                    ? t.clipboard.pastedBadge
                    : t.clipboard.waitingBadge}
              </span>
            </div>
          </div>
        </div>

        <div className="clipboard-footnotes">
          <p className="footnote-item">{t.clipboard.scopeNotice}</p>
          <p className="footnote-item">{t.clipboard.secretNotice}</p>
        </div>
      </div>
    </section>
  );
}
