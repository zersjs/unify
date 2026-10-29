import { useI18n } from "@/i18n";
import { useSectionReveal } from "@/viewmodels/useSectionReveal";

export function SecuritySection() {
  const { t } = useI18n();
  const revealRef = useSectionReveal<HTMLElement>();

  return (
    <section id="security" ref={revealRef} className="section-band section-dark" aria-labelledby="security-heading">
      <div className="container">
        <div className="security-header">
          <p className="eyebrow eyebrow-on-dark">{t.security.eyebrow}</p>
          <h2 id="security-heading" className="section-title title-on-dark">
            <span>{t.security.headlineLine1}</span>
            <span>{t.security.headlineLine2}</span>
          </h2>
          <p className="section-body body-on-dark">{t.security.body}</p>
        </div>

        <div className="lan-topology" role="figure" aria-label={t.security.diagramAria}>
          <div className="lan-node">
            <p className="lan-node-title">{t.security.diagramLeftTitle}</p>
            <p className="lan-node-meta mono">{t.security.diagramLeftMeta}</p>
          </div>

          <div className="lan-channel">
            <span className="lan-channel-primary mono">{t.security.diagramCenterTop}</span>
            <div className="lan-channel-line" aria-hidden="true">
              <span className="lan-pin-dot" />
              <span className="lan-wire-rule" />
              <span className="lan-pin-dot" />
            </div>
            <span className="lan-channel-secondary">{t.security.diagramCenterBottom}</span>
          </div>

          <div className="lan-node">
            <p className="lan-node-title">{t.security.diagramRightTitle}</p>
            <p className="lan-node-meta mono">{t.security.diagramRightMeta}</p>
          </div>
        </div>

        <div className="security-ledger-grid">
          <div className="security-properties-col">
            <h3 className="ledger-heading">{t.security.verifiedHeading}</h3>
            <ol className="security-list">
              {t.security.verifiedItems.map((item, index) => (
                <li key={item.title} className="security-list-row">
                  <span className="security-index mono">{String(index + 1).padStart(2, "0")}</span>
                  <div className="security-row-copy">
                    <p className="security-item-title">{item.title}</p>
                    <p className="security-item-detail">{item.detail}</p>
                  </div>
                </li>
              ))}
            </ol>
          </div>

          <aside className="security-scope-col" aria-labelledby="security-scope-heading">
            <h3 id="security-scope-heading" className="ledger-heading">
              {t.security.validationHeading}
            </h3>
            <p className="security-scope-body">{t.security.validationBody}</p>
          </aside>
        </div>
      </div>
    </section>
  );
}
