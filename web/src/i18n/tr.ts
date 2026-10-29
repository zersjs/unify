import type { Messages } from "./en";

export const tr: Messages = {
  meta: {
    title: "Unify — Tek Klavye. İki Bilgisayar.",
    description:
      "macOS ve Windows 11 arasında yerel ağ üzerinden doğrudan TLS 1.3 bağlantısıyla tek klavye, fare ve düz metin panosu paylaşan açık kaynaklı masaüstü aracı.",
  },
  brand: {
    wordmark: "unify",
    name: "Unify",
    skipToContent: "Ana içeriğe geç",
  },
  nav: {
    ariaLabel: "Ana gezinme",
    overview: "Genel bakış",
    security: "Güvenlik",
    openSource: "Açık kaynak",
    docs: "Belgeler",
    github: "GitHub",
    menuOpen: "Menü",
    menuClose: "Kapat",
  },
  hero: {
    eyebrow: "Açık kaynak · Yerel ağ öncelikli",
    headlineLine1: "Tek çalışma alanı.",
    headlineLine2: "İki bilgisayar.",
    supportingCopy:
      "Mac ve Windows bilgisayarınızda tek klavye ve fare kullanın. Ekranlar arasında doğal biçimde geçiş yapın ve doğrudan, şifreli yerel bağlantı üzerinden metin paylaşın.",
    primaryCta: "GitHub'da incele",
    secondaryCta: "Nasıl çalıştığını gör",
    specsLine: "macOS (Apple Silicon) · Windows 11 · TLS 1.3 ile doğrudan yerel ağ",
    stage: {
      ariaLabel:
        "Yan yana duran MacBook Pro ve Windows 11 monitör arasında tek bir imlecin ekran sınırından geçişini gösteren çalışma alanı sunumu",
      macLabel: "MacBook Pro · macOS",
      winLabel: "Masaüstü PC · Windows 11",
      macMenuFinder: "Finder",
      macMenuFile: "Dosya",
      macMenuEdit: "Düzen",
      macMenuView: "Görünüm",
      macWindowOneTitle: "Terminal — unify-workspace",
      macWindowOneLine1: "$ cargo test --workspace",
      macWindowOneLine2: "7 crate içinde 74 test çalıştırılıyor",
      macWindowOneLine3: "test sonucu: ok. 74 başarılı; 0 hatalı",
      macWindowTwoTitle: "Notlar — Mimari",
      macWindowTwoLine1: "Yerel ağ üzerinden doğrudan eş oturumu",
      macWindowTwoLine2: "Sertifika sabitlendi · SHA-256",
      winWindowOneTitle: "crates\\unify-core\\src\\switcher.rs",
      winWindowOneLine1: "pub enum EdgeDecision {",
      winWindowOneLine2: "    Stay,",
      winWindowOneLine3: "    Switch { ratio: u16 },",
      winWindowOneLine4: "}",
      winWindowTwoTitle: "Gezgin — unify",
      winWindowTwoLine1: "crates / unify-proto",
      winWindowTwoLine2: "crates / unify-security",
      activeControlLabel: "Etkin giriş",
      handoffLabel: "Kenar oranı",
      transportLabel: "Yerel TLS 1.3",
      switchToMac: "Mac'e geç",
      switchToWin: "Windows 11'e geç",
      resumeCycle: "Otomatik akış",
    },
  },
  seam: {
    eyebrow: "Kesintisiz giriş",
    headlineLine1: "Ekranlar arasında geçin.",
    headlineLine2: "Kurulumlar arasında değil.",
    body:
      "İmleci tanımlı ekran kenarına getirdiğinizde kontrol doğrudan diğer bilgisayara geçer. Dikey giriş noktası Apple Retina birimleri ile Windows fiziksel pikselleri arasında orantılı eşlenir; iki ayrı makine tek bir masa gibi çalışır.",
    mechanicsTitle: "unify-core kenar geçiş kuralları",
    mechanics: [
      {
        term: "Orantılı giriş yüksekliği",
        detail: "macOS ekranında %62 yükseklikten çıkan imleç, Windows 11 ekranına %62 yükseklikten girer.",
      },
      {
        term: "Ayarlanabilir kenar bekleme süresi",
        detail: "Kısa temaslar yerel ekranda kalır; yanlışlıkla kenara değmek geçiş tetiklemez.",
      },
      {
        term: "Köşe koruma bölgeleri",
        detail: "Ekran köşeleri macOS Sıcak Köşeler ve Windows pencere kontrolleri için korunur.",
      },
      {
        term: "250 ms dönüş beklemesi",
        detail: "Geri dönüşten sonra imleç kenardan ayrılana kadar geçiş kilidi açılmaz; sınırda gidip gelme engellenir.",
      },
    ],
    phasesLabel: "Kenar durum makinesi",
    phases: {
      armed: {
        label: "Hazır",
        summary: "İmleç köşe koruması dışında yerel ekranda serbestçe hareket eder.",
      },
      dwell: {
        label: "Kenar beklemesi",
        summary: "İmleç ortak sınırda dwell_ms süresi dolana kadar bekler.",
      },
      crossed: {
        label: "Karşı bilgisayarda",
        summary: "Enter(Edge, ratio) ile bağıl HID girişleri öncelikli kuyruktan iletilir.",
      },
      cooldown: {
        label: "Dönüş ve kurma",
        summary: "Leave yerel konumu geri yükler ve yeni geçişten önce 250 ms bekler.",
      },
    },
    diagramMac: "macOS ekranı",
    diagramWin: "Windows 11 ekranı",
    diagramCornerGuard: "Köşe koruması",
    diagramSeam: "Ortak kenar",
  },
  clipboard: {
    eyebrow: "Çift yönlü pano",
    headlineLine1: "Burada kopyala.",
    headlineLine2: "Orada yapıştır.",
    body:
      "Bir bilgisayarda kopyaladığınız metni diğerinde hemen yapıştırın. Unify, düz metin pano değişikliklerini aynı kimliği doğrulanmış yerel oturum üzerinden iki yönlü eşitler ve hedef panoya yazmadan önce her aktarımı SHA-256 ile doğrular.",
    scopeNotice:
      "Yalnızca düz metin desteklenir (16 MiB'a kadar UTF-8). Görsel, dosya ve zengin metin pano paylaşımı şu anda desteklenmez.",
    secretNotice:
      "Desteklenen parola yöneticilerinin gizli veya geçici olarak işaretlediği içerikler yerel olarak filtrelenir ve kaynak bilgisayardan dışarı çıkmaz.",
    directions: {
      "mac-to-win": "macOS → Windows 11",
      "win-to-mac": "Windows 11 → macOS",
    },
    replayButton: "Aktarımı çalıştır",
    stepLabels: {
      idle: "Yerel oturumda hazır",
      copied: "Kaynak panoya kopyalandı",
      transmitting: "16 KiB düşük öncelikli çerçeve · SHA-256 doğrulandı",
      pasted: "Karşı panoya yazıldı",
    },
    macAppTitle: "macOS · Terminal notu",
    macShortcut: "⌘C",
    winAppTitle: "Windows 11 · Düzenleyici",
    winShortcut: "Ctrl + V",
    snippetMacSource: "cargo check -p unify-platform --target aarch64-apple-darwin",
    snippetWinSource: "cargo test --workspace --locked",
    copiedBadge: "Kopyalandı",
    pastedBadge: "Yapıştırıldı",
    waitingBadge: "Yapıştırma bekleniyor",
  },
  security: {
    eyebrow: "Tasarımdan gizli",
    headlineLine1: "Sizin ağınız.",
    headlineLine2: "Sizin bilgisayarlarınız. Sizin veriniz.",
    body:
      "Klavye girişleri ve pano metinleri dış sunuculardan geçmez. Her oturum doğrudan yerel ağdaki iki bilgisayarınız arasında kurulur ve sabitlenmiş cihaz sertifikalarıyla doğrulanır.",
    diagramAria: "Mac ile Windows 11 bilgisayarı karşılıklı TLS 1.3 üzerinden bağlayan doğrudan yerel ağ mimarisi",
    diagramLeftTitle: "Mac (Apple Silicon)",
    diagramLeftMeta: "macOS Anahtar Zinciri'nde P-256 anahtarı",
    diagramCenterTop: "Doğrudan yerel ağ · TLS 1.3 karşılıklı kimlik doğrulama",
    diagramCenterBottom: "6 haneli SPAKE2 + TLS dışa aktarıcı bağlamasıyla tek seferlik eşleştirme",
    diagramRightTitle: "Windows 11 PC",
    diagramRightMeta: "Kimlik Bilgisi Yöneticisi'nde P-256 anahtarı",
    verifiedHeading: "Uygulanan güvenlik mekanizmaları",
    verifiedItems: [
      {
        title: "Yerel ağ üzerinden doğrudan iletişim",
        detail: "Cihazlar dış sinyal sunucusu veya aktarıcı kullanmadan doğrudan soket bağlantısı kurar.",
      },
      {
        title: "Bulut servisi gerekmez",
        detail: "Keşif, eşleştirme, giriş aktarımı ve pano eşitleme internet olmadan yerel alt ağda çalışır.",
      },
      {
        title: "Kullanıcı hesabı gerekmez",
        detail: "Kayıt adımı, telemetri uç noktası veya tuş ve pano geçmişi kaydı yoktur.",
      },
      {
        title: "Karşılıklı kimlik doğrulamalı TLS 1.3",
        detail: "rustls ve ring üzerine kuruludur; 0-RTT ve oturum biletleri kapalıdır, iki taraf da sertifika sunar.",
      },
      {
        title: "SPAKE2 tabanlı parola doğrulamalı anahtar değişimi",
        detail: "6 haneli kod ağdan gönderilmez, 120 saniyede sona erer ve onay MAC değerleri TLS oturumuna bağlanır.",
      },
      {
        title: "SHA-256 parmak iziyle sabitlenen cihaz sertifikaları",
        detail: "Sabitlenmemiş sertifikalar ve kimlik doğrulama öncesi giriş çerçeveleri bağlantıyı anında kapatır.",
      },
      {
        title: "Parola yöneticisi gizli metin filtreleme",
        detail: "macOS ConcealedType ve Windows ExcludeClipboardContentFromMonitorProcessing işaretli veriler gönderilmez.",
      },
    ],
    validationHeading: "Doğrulama durumu ve kapsam",
    validationBody:
      "Unify bağımsız bir üçüncü taraf güvenlik denetiminden geçmemiştir. Protokol çerçeveleme, SPAKE2 kanal bağlama, sertifika sabitleme ve gizli pano filtreleme Rust ile yazılmış ve otomatik çalışma alanı testleriyle doğrulanmıştır. Kurulum paketleri henüz kod imzalı değildir ve gerçek Apple Silicon donanım doğrulaması sürmektedir.",
  },
  openSource: {
    eyebrow: "Açık kaynak · Kapalı kutu yok",
    headline: "Açıkta geliştiriliyor.",
    body:
      "Unify; şeffaflık, yerel kontrol ve net bir hedef etrafında gelişen açık kaynaklı bir projedir: iki bilgisayar arasında çalışmayı doğal hale getirmek.",
    cta: "Kaynağı GitHub'da görüntüle",
    dependencyRule: "Bağımlılık yönü: proto ← core ← {security, net, platform} ← runtime ← cli",
    cratesHeading: "Rust çalışma alanı mimarisi",
    cratesAria: "Sorumluluğunu ve kaynak kodu kesitini incelemek için bir Rust crate seçin",
    earlyDevNote:
      "Unify erken geliştirme aşamasındadır. Geliştiriciler crate'leri inceleyebilir, gerçek donanımda test çalıştırabilir, hata bildirebilir ve katkıda bulunabilir.",
    crates: {
      "unify-proto": {
        role: "Ağ mesajları, uzunluk önekli postcard çerçeveleme, sınır doğrulamaları ve USB HID tuş tabloları.",
        file: "crates/unify-proto/src/message.rs",
      },
      "unify-core": {
        role: "Ekran geometrisi, kenar geçiş durum makinesi, uzak imleç sınırlama ve pano parça birleştirme.",
        file: "crates/unify-core/src/switcher.rs",
      },
      "unify-security": {
        role: "P-256 cihaz kimliği, işletim sistemi anahtar kasası, TLS 1.3 sabitleme ve SPAKE2 eşleştirme.",
        file: "crates/unify-security/src/pairing.rs",
      },
      "unify-net": {
        role: "TLS taşıma katmanı, giriş olaylarını pano parçalarından önce ileten öncelikli yazıcı ve mDNS keşfi.",
        file: "crates/unify-net/src/transport.rs",
      },
      "unify-platform": {
        role: "Yerel API'ler: macOS'ta CGEventTap ve NSPasteboard; Windows'ta WH_KEYBOARD_LL, SendInput ve WM_CLIPBOARDUPDATE.",
        file: "crates/unify-platform/src/lib.rs",
      },
      "unify-runtime": {
        role: "Oturum döngüsü, eşleştirme, kontrol devri ve kopmada basılı tuş bırakmayı yöneten tek görevli Tokio aktörü.",
        file: "crates/unify-runtime/src/control.rs",
      },
      "unify-cli": {
        role: "Tüm Unify çalışma zamanını doğrudan terminalden başlatan komut satırı prototipi.",
        file: "crates/unify-cli/src/main.rs",
      },
    },
  },
  platforms: {
    eyebrow: "Yerel entegrasyon",
    headlineLine1: "Mac ile Windows buluşuyor.",
    headlineLine2: "Sonunda, doğal biçimde.",
    body:
      "Giriş kancaları Rust içinden doğrudan işletim sistemi API'lerini çağırır; giriş yolunda tarayıcı motoru bulunmaz. Her iki bilgisayar kendi alışkanlıklarını korurken tek klavye ve fareyi paylaşır.",
    macTitle: "macOS · Apple Silicon",
    macItems: [
      "CoreGraphics CGEventTap ve CGEventPost ile düşük seviyeli imleç ve tuş yakalama",
      "ConcealedType filtrelemeli NSPasteboard pano izleme",
      "macOS Anahtar Zinciri'nde özel anahtar; Yerel Ağ, Erişilebilirlik ve Giriş İzleme izinleri",
      "Acil kontrol bırakma kısayolu: Control + Option + Shift + Space",
    ],
    winTitle: "Windows 11",
    winItems: [
      "Yönetici yetkisi gerektirmeyen WH_KEYBOARD_LL, WH_MOUSE_LL ve SendInput kancaları",
      "Parola yöneticisi istisna biçimlerini tanıyan olay tabanlı WM_CLIPBOARDUPDATE dinleyicisi",
      "Windows Kimlik Bilgisi Yöneticisi'nde özel anahtar; UAC ve kilit ekranı sınırlarına uyum",
      "Acil kontrol bırakma kısayolu: Ctrl + Alt + Shift + Space",
    ],
    sharedHeading: "Ortak oturum yetenekleri",
    sharedItems: [
      {
        label: "Otomatik mDNS keşfi",
        detail: "Yerel alt ağdaki eşleşmiş cihazları otomatik bulur; çoklu yayının engellendiği ağlarda elle IP ve port bağlantısı sunar.",
      },
      {
        label: "Klavye ve fare aktarımı",
        detail: "USB HID fiziksel tuş konumlarını isteğe bağlı Command ↔ Ctrl eşlemesiyle iletir ve kopmada basılı tuşları bırakır.",
      },
      {
        label: "Çift yönlü düz metin panosu",
        detail: "İmleç hareketinin gecikmemesi için UTF-8 metinleri düşük öncelikli çerçevelerle iki yönlü eşitler.",
      },
      {
        label: "Prototip giriş noktası",
        detail: "Tauri masaüstü kabuğu ve gerçek Mac donanım doğrulaması sürerken bugün unify-cli komut satırı prototipiyle çalışır.",
      },
    ],
  },
  earlyAccess: {
    eyebrow: "Proje durumu",
    headline: "Sıradaki adımı birlikte şekillendirin.",
    body:
      "Unify erken geliştirme aşamasındadır. Çekirdek kütüphaneler test ediliyor, macOS kodu Apple Silicon için derleniyor ve gerçek donanım doğrulaması devam ediyor.",
    primaryCta: "Projeyi incele",
    secondaryLink: "Dokümantasyonu oku",
    honestyNote:
      "Henüz yayınlanmış imzalı bir kurulum paketi yoktur. Kaynak depoyu inceleyebilir, tehdit modelini okuyabilir veya Cargo ile derleyebilirsiniz.",
  },
  footer: {
    description:
      "macOS ve Windows 11 arasında yerel ağ üzerinden açık kaynaklı klavye, fare ve düz metin panosu paylaşımı.",
    github: "GitHub",
    documentation: "Dokümantasyon",
    license: "MIT Lisansı",
    platforms: "macOS · Windows 11",
    statusNote: "Erken geliştirme · Kaynak depo",
    languageLabel: "Dil",
  },
  errorBoundary: {
    title: "Sayfa oluşturulurken bir hata oluştu.",
    retry: "Görünümü yenile",
  },
};
