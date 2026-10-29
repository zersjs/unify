import { useI18n } from "@/i18n";
import { useSectionReveal } from "@/viewmodels/useSectionReveal";

export function PlatformsSection() {
  const { t } = useI18n();
  const revealRef = useSectionReveal<HTMLElement>();

  return (
    <section id="platforms" ref={revealRef} className="section-band section-subtle" aria-labelledby="platforms-heading">
      <div className="container">
        <div className="platforms-header">
          <p className="eyebrow">{t.platforms.eyebrow}</p>
          <h2 id="platforms-heading" className="section-title">
            <span>{t.platforms.headlineLine1}</span>
            <span>{t.platforms.headlineLine2}</span>
          </h2>
          <p className="section-body">{t.platforms.body}</p>
        </div>

        <div className="platforms-split">
          <div className="platform-column">
            <h3 className="platform-col-title">{t.platforms.macTitle}</h3>
            <ul className="platform-spec-list">
              {t.platforms.macItems.map((item) => (
                <li key={item} className="platform-spec-item">
                  {item}
                </li>
              ))}
            </ul>
          </div>

          <div className="platform-column">
            <h3 className="platform-col-title">{t.platforms.winTitle}</h3>
            <ul className="platform-spec-list">
              {t.platforms.winItems.map((item) => (
                <li key={item} className="platform-spec-item">
                  {item}
                </li>
              ))}
            </ul>
          </div>
        </div>

        <div className="shared-capabilities" aria-label={t.platforms.sharedHeading}>
          {t.platforms.sharedItems.map((item) => (
            <div key={item.label} className="shared-cap-cell">
              <h4 className="shared-cap-title">{item.label}</h4>
              <p className="shared-cap-detail">{item.detail}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
