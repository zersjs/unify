import { isExternalHttpUrl, localizedDocsUrl, siteConfig } from "@/config/site";
import { useI18n } from "@/i18n";
import { useNavigationViewModel } from "@/viewmodels/useNavigationViewModel";
import { ArrowUpRightIcon, MenuIcon } from "./Icons";
import { Logo } from "./Logo";
import { PressLink } from "./PressElements";

export function Navbar() {
  const { locale, t } = useI18n();
  const nav = useNavigationViewModel();
  const docsHref = localizedDocsUrl(siteConfig.documentationUrl, locale);
  const docsExternal = isExternalHttpUrl(docsHref);

  return (
    <header className="site-header" data-scrolled={nav.scrolled ? "true" : "false"}>
      <div className="container nav-inner">
        <PressLink
          href="#top"
          className="brand-link"
          onClick={(event) => {
            event.preventDefault();
            nav.navigateToSection("top");
          }}
        >
          <Logo size={20} />
          <span className="brand-wordmark">{t.brand.wordmark}</span>
        </PressLink>

        <nav className="nav-desktop" aria-label={t.nav.ariaLabel}>
          <PressLink
            href="#overview"
            className="nav-link"
            onClick={(event) => {
              event.preventDefault();
              nav.navigateToSection("overview");
            }}
          >
            {t.nav.overview}
          </PressLink>
          <PressLink
            href="#security"
            className="nav-link"
            onClick={(event) => {
              event.preventDefault();
              nav.navigateToSection("security");
            }}
          >
            {t.nav.security}
          </PressLink>
          <PressLink
            href="#open-source"
            className="nav-link"
            onClick={(event) => {
              event.preventDefault();
              nav.navigateToSection("open-source");
            }}
          >
            {t.nav.openSource}
          </PressLink>
          <PressLink
            href={docsHref}
            target={docsExternal ? "_blank" : undefined}
            rel={docsExternal ? "noopener noreferrer" : undefined}
            className={docsExternal ? "nav-link nav-link-external" : "nav-link"}
          >
            <span>{t.nav.docs}</span>
            {docsExternal ? <ArrowUpRightIcon size={13} /> : null}
          </PressLink>
          <PressLink
            href={siteConfig.repositoryUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="nav-link nav-link-external"
          >
            <span>{t.nav.github}</span>
            <ArrowUpRightIcon size={13} />
          </PressLink>
        </nav>

        <button
          ref={nav.menuButtonRef}
          type="button"
          className="nav-mobile-toggle"
          aria-expanded={nav.mobileOpen}
          aria-controls="mobile-nav-panel"
          aria-label={nav.mobileOpen ? t.nav.menuClose : t.nav.menuOpen}
          onPointerDown={(event) => {
            if (event.button === 0) {
              event.preventDefault();
              nav.toggleMobileMenu();
            }
          }}
          onClick={(event) => {
            if (event.detail === 0) {
              nav.toggleMobileMenu();
            }
          }}
        >
          <MenuIcon open={nav.mobileOpen} />
        </button>
      </div>

      {nav.mobileOpen ? (
        <nav id="mobile-nav-panel" className="nav-mobile-panel" aria-label={t.nav.ariaLabel}>
          <div className="container nav-mobile-links">
            <a
              href="#overview"
              className="nav-mobile-link"
              onClick={(event) => {
                event.preventDefault();
                nav.navigateToSection("overview");
              }}
            >
              {t.nav.overview}
            </a>
            <a
              href="#security"
              className="nav-mobile-link"
              onClick={(event) => {
                event.preventDefault();
                nav.navigateToSection("security");
              }}
            >
              {t.nav.security}
            </a>
            <a
              href="#open-source"
              className="nav-mobile-link"
              onClick={(event) => {
                event.preventDefault();
                nav.navigateToSection("open-source");
              }}
            >
              {t.nav.openSource}
            </a>
            <a
              href={docsHref}
              target={docsExternal ? "_blank" : undefined}
              rel={docsExternal ? "noopener noreferrer" : undefined}
              className={docsExternal ? "nav-mobile-link nav-link-external" : "nav-mobile-link"}
              onClick={() => nav.closeMobileMenu()}
            >
              <span>{t.nav.docs}</span>
              {docsExternal ? <ArrowUpRightIcon size={14} /> : null}
            </a>
            <a
              href={siteConfig.repositoryUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="nav-mobile-link nav-link-external"
              onClick={() => nav.closeMobileMenu()}
            >
              <span>{t.nav.github}</span>
              <ArrowUpRightIcon size={14} />
            </a>
          </div>
        </nav>
      ) : null}
    </header>
  );
}
