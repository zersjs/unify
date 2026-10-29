import { isExternalHttpUrl, localizedDocsUrl, siteConfig } from "@/config/site";
import { useI18n } from "@/i18n";
import { useSectionReveal } from "@/viewmodels/useSectionReveal";
import { ArrowRightIcon, ArrowUpRightIcon } from "./Icons";
import { PressLink } from "./PressElements";

export function EarlyAccessSection() {
  const { locale, t } = useI18n();
  const revealRef = useSectionReveal<HTMLElement>();
  const docsHref = localizedDocsUrl(siteConfig.documentationUrl, locale);
  const docsExternal = isExternalHttpUrl(docsHref);

  return (
    <section id="early-access" ref={revealRef} className="section-band early-section" aria-labelledby="early-heading">
      <div className="container early-inner">
        <p className="eyebrow">{t.earlyAccess.eyebrow}</p>
        <h2 id="early-heading" className="section-title early-title">
          {t.earlyAccess.headline}
        </h2>
        <p className="section-body early-body">{t.earlyAccess.body}</p>

        <div className="early-actions">
          <PressLink
            href={siteConfig.repositoryUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="btn-primary"
          >
            <span>{t.earlyAccess.primaryCta}</span>
            <ArrowUpRightIcon size={15} />
          </PressLink>

          <PressLink
            href={docsHref}
            target={docsExternal ? "_blank" : undefined}
            rel={docsExternal ? "noopener noreferrer" : undefined}
            className="btn-secondary"
          >
            <span>{t.earlyAccess.secondaryLink}</span>
            {docsExternal ? <ArrowUpRightIcon size={15} /> : <ArrowRightIcon size={15} />}
          </PressLink>
        </div>

        <p className="early-honesty-note">{t.earlyAccess.honestyNote}</p>
      </div>
    </section>
  );
}
