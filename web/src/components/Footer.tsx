import { isExternalHttpUrl, localizedDocsUrl, siteConfig } from "@/config/site";
import { useI18n } from "@/i18n";
import { Logo } from "./Logo";
import { PressButton, PressLink } from "./PressElements";

export function Footer() {
  const { locale, setLocale, t } = useI18n();
  const docsHref = localizedDocsUrl(siteConfig.documentationUrl, locale);
  const docsExternal = isExternalHttpUrl(docsHref);

  return (
    <footer className="site-footer">
      <div className="container footer-inner">
        <div className="footer-brand-block">
          <div className="footer-brand-row">
            <Logo size={18} />
            <span className="brand-wordmark">{t.brand.wordmark}</span>
          </div>
          <p className="footer-description">{t.footer.description}</p>
        </div>

        <div className="footer-meta-block">
          <div className="footer-links-row">
            <PressLink
              href={siteConfig.repositoryUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="footer-link"
            >
              {t.footer.github}
            </PressLink>
            <PressLink
              href={docsHref}
              target={docsExternal ? "_blank" : undefined}
              rel={docsExternal ? "noopener noreferrer" : undefined}
              className="footer-link"
            >
              {t.footer.documentation}
            </PressLink>
            <PressLink
              href={siteConfig.licenseUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="footer-link"
            >
              {t.footer.license}
            </PressLink>
          </div>

          <div className="footer-status-row">
            <span>{t.footer.platforms}</span>
            <span aria-hidden="true">·</span>
            <span>{t.footer.statusNote}</span>
            <span aria-hidden="true">·</span>
            <span className="footer-lang-switch" role="group" aria-label={t.footer.languageLabel}>
              <PressButton
                className="lang-btn"
                data-active={locale === "en" ? "true" : "false"}
                aria-pressed={locale === "en"}
                onPointerDown={() => setLocale("en")}
                onClick={(event) => {
                  if (event.detail === 0) {
                    setLocale("en");
                  }
                }}
              >
                English
              </PressButton>
              <span aria-hidden="true">/</span>
              <PressButton
                className="lang-btn"
                data-active={locale === "tr" ? "true" : "false"}
                aria-pressed={locale === "tr"}
                onPointerDown={() => setLocale("tr")}
                onClick={(event) => {
                  if (event.detail === 0) {
                    setLocale("tr");
                  }
                }}
              >
                Türkçe
              </PressButton>
            </span>
          </div>
        </div>
      </div>
    </footer>
  );
}
