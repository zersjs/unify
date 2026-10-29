import { useEffect } from "react";
import { ClipboardSection } from "@/components/ClipboardSection";
import { EarlyAccessSection } from "@/components/EarlyAccessSection";
import { LandingErrorBoundary } from "@/components/ErrorBoundary";
import { Footer } from "@/components/Footer";
import { HeroSection } from "@/components/HeroWorkspaceStage";
import { Navbar } from "@/components/Navbar";
import { OpenSourceSection } from "@/components/OpenSourceSection";
import { PlatformsSection } from "@/components/PlatformsSection";
import { SeamSection } from "@/components/SeamSection";
import { SecuritySection } from "@/components/SecuritySection";
import { siteConfig } from "@/config/site";
import { I18nProvider, useI18n, type Locale } from "@/i18n";

function LandingShell() {
  const { t } = useI18n();

  useEffect(() => {
    if (!siteConfig.canonicalUrl) {
      return;
    }
    let link = document.querySelector<HTMLLinkElement>('link[rel="canonical"]');
    if (!link) {
      link = document.createElement("link");
      link.rel = "canonical";
      document.head.appendChild(link);
    }
    link.href = siteConfig.canonicalUrl;

    const ogImage = document.querySelector<HTMLMetaElement>('meta[property="og:image"]');
    if (ogImage) {
      ogImage.content = `${siteConfig.canonicalUrl}/og-card.svg`;
    }
  }, []);

  return (
    <LandingErrorBoundary fallbackTitle={t.errorBoundary.title} retryLabel={t.errorBoundary.retry}>
      <a href="#main-content" className="skip-link">
        {t.brand.skipToContent}
      </a>
      <Navbar />
      <main id="main-content">
        <HeroSection />
        <SeamSection />
        <ClipboardSection />
        <SecuritySection />
        <OpenSourceSection />
        <PlatformsSection />
        <EarlyAccessSection />
      </main>
      <Footer />
    </LandingErrorBoundary>
  );
}

export function App({ initialLocale = "en" }: { initialLocale?: Locale }) {
  return (
    <I18nProvider initialLocale={initialLocale}>
      <LandingShell />
    </I18nProvider>
  );
}
