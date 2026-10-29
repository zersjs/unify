# Unify dokümantasyonu

Unify, bir Apple Silicon Mac ile bir Windows 11 bilgisayar arasında yerel ağ üzerinden **tek klavye, fare ve düz metin panosu** paylaşır. İmleci tanımlı ekran kenarına getirdiğinizde kontrol eşleştirilmiş bilgisayara geçer; bir bilgisayarda kopyalanan metin diğerinde yapıştırılabilir.

- Karşılıklı sertifika doğrulamalı TLS 1.3 üzerinden doğrudan eşler arası bağlantı
- TLS 1.3 oturum dışa aktarıcısına (RFC 5705) bağlanan tek kullanımlık 6 haneli SPAKE2 eşleştirme
- Bulut servisi, kullanıcı hesabı, telemetri veya pano geçmişi kaydı yoktur
- Ortak Rust çalışma alanı (`unify-proto` → `unify-runtime`), yerel işletim sistemi API kancaları, `unify-cli` prototipi ve Tauri 2 masaüstü kabuğu (`app/`)

## Proje durumu

| Bileşen | Güncel durum |
|---|---|
| Rust çekirdeği ve protokol (`crates/*`) | Uygulandı; birim ve tümleştirme testleri Windows 11 üzerinde geçiyor |
| Windows 11 yerel katmanı ve kurulum paketi | Derlendi, paketlendi (`Unify_0.1.0_x64-setup.exe`, imzasız) ve yerel ortamda çalıştırıldı |
| macOS Apple Silicon yerel katmanı | Yazıldı ve `aarch64-apple-darwin` hedefi için tip denetiminden geçti; fiziksel Mac üzerinde henüz çalıştırılmadı |
| Güvenlik incelemesi | Tehdit modeli belgelendi; bağımsız üçüncü taraf güvenlik denetimi yapılmadı |
| Donanım kabul testleri | İki fiziksel bilgisayarda doğrulama bekliyor ([09-test-plani.md](09-test-plani.md)) |

## Başlangıç

- [Hızlı başlangıç](hizli-baslangic.md) — Mac ve Windows 11 bilgisayarı beş adımda bağlama, kenar geçişini deneme ve acil kontrol bırakma kısayolunu doğrulama.
- [Kurulum, derleme ve kaldırma](04-kurulum-ve-derleme.md) — kurulum paketleri, Cargo ve Tauri ile kaynaktan derleme, kod imzalama durumu ve temiz kaldırma adımları.
- [İşletim sistemi izinleri](05-izinler.md) — macOS Erişilebilirlik, Giriş İzleme ve Yerel Ağ izinleri; Windows Güvenlik Duvarı ve UIPI sınırları.
- [Güvenli cihaz eşleştirme](06-eslestirme.md) — 6 haneli SPAKE2 anahtar değişimi, TLS kanal bağlama ve SHA-256 parmak izi sabitleme akışı.

## Ağ, güvenlik ve işletim

- [Ağ ve güvenlik duvarı](07-ag-ve-guvenlik-duvari.md) — TCP `47823` portu, UDP `5353` mDNS keşfi (`_unify._tcp.local.`), elle IP bağlantısı ve misafir Wi-Fi kısıtlamaları.
- [Tehdit modeli](03-tehdit-modeli.md) — korunan varlıklar, ortak yerel ağdaki saldırı yüzeyleri, karşı önlemler ve kabul edilen artık riskler.
- [Sorun giderme](08-sorun-giderme.md) — sık karşılaşılan belirtiler, kontrol adımları ve günlük dosyası konumları.
- [Bilinen sınırlamalar](10-bilinen-sinirlamalar.md) — iki bilgisayar sınırı, yalnızca düz metin panosu, Windows UAC/kilit ekranı sınırı ve imzasız paket notları.

## Mühendislik referansı

- [`unify-cli` komut satırı rehberi](cli-rehberi.md) — başlatma bayrakları, etkileşimli terminal komutları ve tek bilgisayarda çift profil testi.
- [Mimari ve protokol](02-mimari-ve-protokol.md) — iş parçacığı modeli, protokol sürüm 1 çerçeve yapısı, bağlantı durum makinesi ve kenar devir kuralları.
- [Crate ve yapılandırma referansı](crate-referansi.md) — yedi Rust crate modülünün sorumlulukları, protokol sabitleri ve `config.json` şeması.
- [Teknik değerlendirme ve karar kaydı](01-teknik-degerlendirme.md) — platform engelleri, mimari seçenek karşılaştırması ve MVP kapsamı.
- [Test planı](09-test-plani.md) — otomatik test katmanları ve fiziksel M2 Mac + Windows 11 kabul listesi.
