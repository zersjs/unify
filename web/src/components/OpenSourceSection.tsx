import { siteConfig } from "@/config/site";
import { useI18n } from "@/i18n";
import { useCrateExplorerViewModel } from "@/viewmodels/useCrateExplorerViewModel";
import { useSectionReveal } from "@/viewmodels/useSectionReveal";
import { ArrowUpRightIcon } from "./Icons";
import { PressButton, PressLink } from "./PressElements";

export function OpenSourceSection() {
  const { t } = useI18n();
  const revealRef = useSectionReveal<HTMLElement>();
  const vm = useCrateExplorerViewModel();
  const activeMeta = t.openSource.crates[vm.selectedCrate];

  return (
    <section id="open-source" ref={revealRef} className="section-band" aria-labelledby="opensource-heading">
      <div className="container">
        <div className="opensource-header-grid">
          <div>
            <p className="eyebrow">{t.openSource.eyebrow}</p>
            <h2 id="opensource-heading" className="section-title">
              {t.openSource.headline}
            </h2>
          </div>

          <div className="opensource-intro-col">
            <p className="section-body">{t.openSource.body}</p>
            <div className="opensource-cta-row">
              <PressLink
                href={siteConfig.repositoryUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="btn-primary"
              >
                <span>{t.openSource.cta}</span>
                <ArrowUpRightIcon size={15} />
              </PressLink>
            </div>
          </div>
        </div>

        <p className="dependency-flow mono selectable">{t.openSource.dependencyRule}</p>

        <div className="workspace-explorer">
          <div className="crate-list-col" role="tablist" aria-label={t.openSource.cratesAria}>
            {vm.crates.map((crateId) => {
              const selected = vm.selectedCrate === crateId;
              return (
                <PressButton
                  key={crateId}
                  role="tab"
                  aria-selected={selected}
                  className="crate-row"
                  data-active={selected ? "true" : "false"}
                  onPointerDown={() => vm.selectCrate(crateId)}
                  onClick={(event) => {
                    if (event.detail === 0) {
                      vm.selectCrate(crateId);
                    }
                  }}
                >
                  <span className="crate-name mono">{crateId}</span>
                  <span className="crate-role">{t.openSource.crates[crateId].role}</span>
                </PressButton>
              );
            })}
          </div>

          <div className="source-sheet" role="tabpanel" aria-label={vm.selectedCrate}>
            <div className="source-sheet-header">
              <span className="source-path mono selectable">{activeMeta.file}</span>
              <span className="source-lang mono">Rust · edition 2021</span>
            </div>
            <pre className="source-code mono selectable">
              <code>{vm.excerpt}</code>
            </pre>
          </div>
        </div>

        <p className="opensource-note">{t.openSource.earlyDevNote}</p>
      </div>
    </section>
  );
}
