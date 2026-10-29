import { fileURLToPath } from "node:url";
import { defineConfig } from "vitepress";

const webResolveAnchor = fileURLToPath(new URL("../package.json", import.meta.url));

const rawBase = process.env.DOCS_BASE?.trim() || "/docs/";
const normalizedBase = rawBase.startsWith("/")
  ? rawBase.endsWith("/")
    ? rawBase
    : `${rawBase}/`
  : `/${rawBase.replace(/\/?$/, "/")}`;

const repoUrl = process.env.VITE_REPO_URL?.trim() || "https://github.com/zersjs/unify";

export default defineConfig({
  srcDir: "../docs",
  outDir: "./dist/docs",
  cacheDir: "./.vitepress/cache",
  title: "unify",
  titleTemplate: ":title — Unify Docs",
  base: normalizedBase,
  srcExclude: ["tr/README.md"],
  vite: {
    plugins: [
      {
        name: "unify-docs-module-resolver",
        async resolveId(source, importer, options) {
          if (
            !importer ||
            source.startsWith(".") ||
            source.startsWith("/") ||
            source.startsWith("\0") ||
            /^[a-zA-Z]:/.test(source)
          ) {
            return null;
          }
          const direct = await this.resolve(source, importer, { ...options, skipSelf: true });
          if (direct) {
            return direct;
          }
          return this.resolve(source, webResolveAnchor, { ...options, skipSelf: true });
        },
      },
    ],
  },
  head: [
    ["link", { rel: "icon", type: "image/svg+xml", href: `${normalizedBase}favicon.svg` }],
  ],
  themeConfig: {
    logo: {
      light: "/logo-light.svg",
      dark: "/logo-dark.svg",
      alt: "Unify",
    },
    siteTitle: "unify",
    socialLinks: [{ icon: "github", link: repoUrl }],
    search: {
      provider: "local",
      options: {
        locales: {
          tr: {
            translations: {
              button: {
                buttonText: "Belgelerde ara",
                buttonAriaLabel: "Belgelerde ara",
              },
              modal: {
                displayDetails: "Ayrıntılı listeyi göster",
                resetButtonTitle: "Aramayı temizle",
                backButtonTitle: "Kapat",
                noResultsText: "Sonuç bulunamadı",
                footer: {
                  selectText: "seç",
                  selectKeyAriaLabel: "giriş",
                  navigateText: "gezin",
                  navigateUpKeyAriaLabel: "yukarı ok",
                  navigateDownKeyAriaLabel: "aşağı ok",
                  closeText: "kapat",
                  closeKeyAriaLabel: "esc",
                },
              },
            },
          },
        },
      },
    },
  },
  locales: {
    root: {
      label: "English",
      lang: "en-US",
      description:
        "Open-source keyboard, mouse, and plain-text clipboard sharing between macOS and Windows 11 over TLS 1.3.",
      themeConfig: {
        nav: [
          { text: "Overview", link: "/" },
          { text: "Quick start", link: "/quick-start" },
          { text: "CLI guide", link: "/cli-guide" },
          { text: "Crate reference", link: "/crate-reference" },
          { text: "Product page", link: "../", target: "_self", noIcon: true },
        ],
        sidebar: [
          {
            text: "Getting started",
            items: [
              { text: "Overview", link: "/" },
              { text: "Quick start", link: "/quick-start" },
              { text: "Installation and build", link: "/installation-and-build" },
              { text: "OS permissions", link: "/permissions" },
              { text: "Device pairing", link: "/pairing" },
            ],
          },
          {
            text: "Operations and security",
            items: [
              { text: "Network and firewall", link: "/network-and-firewall" },
              { text: "Threat model", link: "/threat-model" },
              { text: "Troubleshooting", link: "/troubleshooting" },
              { text: "Known limitations", link: "/known-limitations" },
            ],
          },
          {
            text: "Engineering reference",
            items: [
              { text: "CLI prototype (unify-cli)", link: "/cli-guide" },
              { text: "Architecture and protocol", link: "/architecture-and-protocol" },
              { text: "Crate and config reference", link: "/crate-reference" },
              { text: "Technical assessment", link: "/technical-assessment" },
              { text: "Test plan", link: "/test-plan" },
            ],
          },
        ],
        outline: {
          level: [2, 3],
          label: "On this page",
        },
        docFooter: {
          prev: "Previous page",
          next: "Next page",
        },
        footer: {
          message: "MIT License · Early development",
          copyright: "Unify open-source project",
        },
      },
    },
    tr: {
      label: "Türkçe",
      lang: "tr-TR",
      link: "/tr/",
      titleTemplate: ":title — Unify Belgeleri",
      description:
        "macOS ve Windows 11 arasında TLS 1.3 üzerinden açık kaynaklı klavye, fare ve düz metin panosu paylaşımı.",
      themeConfig: {
        nav: [
          { text: "Genel bakış", link: "/tr/" },
          { text: "Hızlı başlangıç", link: "/tr/hizli-baslangic" },
          { text: "CLI rehberi", link: "/tr/cli-rehberi" },
          { text: "Crate referansı", link: "/tr/crate-referansi" },
          { text: "Ürün sayfası", link: "../../", target: "_self", noIcon: true },
        ],
        sidebar: [
          {
            text: "Başlangıç",
            items: [
              { text: "Genel bakış", link: "/tr/" },
              { text: "Hızlı başlangıç", link: "/tr/hizli-baslangic" },
              { text: "Kurulum ve derleme", link: "/tr/04-kurulum-ve-derleme" },
              { text: "İşletim sistemi izinleri", link: "/tr/05-izinler" },
              { text: "Cihaz eşleştirme", link: "/tr/06-eslestirme" },
            ],
          },
          {
            text: "Ağ, güvenlik ve işletim",
            items: [
              { text: "Ağ ve güvenlik duvarı", link: "/tr/07-ag-ve-guvenlik-duvari" },
              { text: "Tehdit modeli", link: "/tr/03-tehdit-modeli" },
              { text: "Sorun giderme", link: "/tr/08-sorun-giderme" },
              { text: "Bilinen sınırlamalar", link: "/tr/10-bilinen-sinirlamalar" },
            ],
          },
          {
            text: "Mühendislik referansı",
            items: [
              { text: "CLI prototipi (unify-cli)", link: "/tr/cli-rehberi" },
              { text: "Mimari ve protokol", link: "/tr/02-mimari-ve-protokol" },
              { text: "Crate ve yapılandırma referansı", link: "/tr/crate-referansi" },
              { text: "Teknik değerlendirme", link: "/tr/01-teknik-degerlendirme" },
              { text: "Test planı", link: "/tr/09-test-plani" },
            ],
          },
        ],
        outline: {
          level: [2, 3],
          label: "Bu sayfada",
        },
        docFooter: {
          prev: "Önceki sayfa",
          next: "Sonraki sayfa",
        },
        darkModeSwitchLabel: "Görünüm",
        lightModeSwitchTitle: "Açık görünüme geç",
        darkModeSwitchTitle: "Koyu görünüme geç",
        sidebarMenuLabel: "Menü",
        returnToTopLabel: "Yukarı dön",
        langMenuLabel: "Dil seçimi",
        footer: {
          message: "MIT Lisansı · Erken geliştirme",
          copyright: "Unify açık kaynak projesi",
        },
      },
    },
  },
});
