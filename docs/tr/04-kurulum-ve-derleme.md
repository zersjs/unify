# Kurulum, derleme ve kaldırma

## Kullanıcı kurulumu

### Windows 11

1. `Unify_0.1.0_x64-setup.exe` dosyasını çalıştırın. Kurulum yalnızca geçerli kullanıcı için yapılır; yönetici yetkisi istemez.
2. Paket imzasız olduğu için SmartScreen "Windows bilgisayarınızı korudu" uyarısı gösterebilir: **Ek bilgi → Yine de çalıştır**. Kod imzalama sertifikası olmadan bu uyarı kaldırılamaz.
3. İlk açılışta Windows Güvenlik Duvarı izin sorar: yalnızca **Özel ağlar** kutusunu işaretleyin ([07-ag-ve-guvenlik-duvari.md](07-ag-ve-guvenlik-duvari.md)).
4. WebView2 Windows 11'de yüklüdür; eksikse kurulum Microsoft'un önyükleyicisini indirir.

### macOS (Apple Silicon)

1. `Unify_0.1.0_aarch64.dmg` dosyasını açın, Unify'ı **Uygulamalar** klasörüne sürükleyin.
2. Uygulama Apple Developer ID ile imzalanmamış ve notarize edilmemiştir (yalnızca ad-hoc imza). İlk açılışta Gatekeeper engeller: **Sistem Ayarları → Gizlilik ve Güvenlik → Yine de Aç**. macOS 15'te sağ tık → Aç kısayolu kaldırılmıştır; bu ayar sayfası kullanılır.
3. İzinleri verin ([05-izinler.md](05-izinler.md)). Unify menü çubuğunda çalışır; Dock simgesi yoktur.

### Otomatik başlatma

Varsayılan **kapalıdır**. Genel → "Oturum açıldığında başlat" ile açılır. Açıkken uygulama `--background` argümanıyla, pencere göstermeden başlar. Windows'ta kullanıcı `Run` kayıt anahtarı, macOS'ta `~/Library/LaunchAgents` altında bir LaunchAgent kullanılır.

### Güncellemeler

Unify, açılıştan 20 saniye sonra ve ardından saatte bir en yeni GitHub sürümündeki `latest.json` dosyasını kontrol eder. Eşleşmiş cihaz farklı bir protokol sürümü bildirdiğinde de kontrol yapar. Güncelleme yalnızca minisign imzası uygulamaya gömülü açık anahtarla eşleşirse kurulur.

**Genel → Otomatik güncelle** açıkken (varsayılan) Unify, kontrol yerelde ve ekran görüntüleme boştayken kurulumu yapıp yeniden başlar. Kapalıyken aynı işlem **Genel → Şimdi kontrol et** ve **Kur ve yeniden başlat** ile elle yapılır. Geliştirme derlemeleri kontrol yapmaz.

Kontrol, GitHub'a sıradan bir HTTPS isteği gönderir; GitHub IP adresini ve uygulama sürümünü görür. Ayar kapatıldığında otomatik kurulum durur; uygulama güncelleme olduğunu gösterebilsin diye saatlik kontrol sürer.

## Geliştirme ortamı

| Araç | Sürüm | Not |
|---|---|---|
| Rust | 1.80 veya üstü (geliştirme 1.98 ile yapıldı) | `rustup` |
| Node.js | 22 veya üstü | `npm` |
| Windows | Visual Studio Build Tools (MSVC, Windows SDK) | `ring` C derlemesi için |
| macOS | Xcode Command Line Tools (`xcode-select --install`) | |

```bash
cd app
npm ci
```

## Komutlar

Testler (kök dizinde):

```bash
cargo test --workspace
```

Frontend denetimi ve testleri:

```bash
npm --prefix app run typecheck
```

```bash
npm --prefix app test
```

Teknik prototip (terminal):

```bash
cargo run -p unify-cli -- --log info
```

Masaüstü uygulaması, geliştirme modu:

```bash
npm --prefix app run tauri dev
```

Windows kurulum paketi:

```bash
npm --prefix app run tauri build -- --bundles nsis
```

macOS paketi (Mac üzerinde):

```bash
npm --prefix app run tauri build -- --bundles app,dmg
```

Çıktılar `target/release/bundle/` altındadır. macOS kodu Windows'ta yalnızca tip denetiminden geçirilebilir:

```bash
cargo check -p unify-platform --target aarch64-apple-darwin
```

GitHub'a gönderildiğinde `.github/workflows/ci.yml` iki platformda lint, tip denetimi ve testleri çalıştırır, ardından imzasız `.exe` ve `.dmg` paketlerini artefakt olarak üretir. Mac'te derleme yapmadan macOS paketi almanın en kısa yolu budur.

## Sürüm yayınlama

`main` dalına yapılan push sonrası `ci` başarılı olursa `.github/workflows/release.yml`, son etiketten beri `crates/`, `app/` veya Cargo dosyaları değiştiyse `0.2.<çalıştırma numarası>` sürümünü yayınlar. Sürüm derleme sırasında yazılır, commit edilmez. Yayını zorlamak için Actions sayfasında **Run workflow** kullanılır.

Depo sırları:

| Sır | İçerik |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Güncelleme imzalama özel anahtarı |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Bu anahtarın parolası |
| `UNIFY_MAC_CERT_P12` | macOS kendinden imzalı `.p12` dosyasının Base64 hali |
| `UNIFY_MAC_CERT_PASSWORD` | Bu `.p12` dosyasının parolası |

Güncelleme anahtarı kaybolursa kurulu kopyalar artık güncelleme alamaz. macOS sertifikası değişirse macOS izinleri yeniden verilir.

## İmzalama ve notarization

| Platform | Gereken | Durum |
|---|---|---|
| macOS | Apple Developer Program üyeliği (yıllık), "Developer ID Application" sertifikası, `notarytool` ile notarization | Yapılmadı. Yayın derlemeleri, gizlilik izinleri güncellemelerden sonra korunabilsin diye sabit bir kendinden imzalı sertifika (`Unify Self-Signed`) kullanır; bu henüz Mac üzerinde doğrulanmadı. CI artefaktları ad-hoc imzalı kalır ve her derlemede izinleri kaybeder. Gatekeeper ilk elle kurulumu yine engeller. |
| Windows | Authenticode kod imzalama sertifikası (OV/EV) | Yapılmadı; SmartScreen uyarısı beklenir. |

İmza eklemek için `tauri.conf.json` içindeki `bundle.macOS.signingIdentity` ve Windows için `bundle.windows.certificateThumbprint` alanları doldurulur.

## Kaldırma ve temizlik

1. Unify'da **Eşleştirilmiş cihaz → Eşleştirmeyi kaldır** (diğer bilgisayarda da yapın).
2. Uygulamayı kaldırın: Windows **Ayarlar → Uygulamalar → Unify → Kaldır**; macOS'ta Unify'ı Çöp Sepeti'ne taşıyın.
3. Ayarlar ve kimlik dosyalarını silin:
   - Windows: `%APPDATA%\dev.unify.desktop`, `%LOCALAPPDATA%\dev.unify.desktop`
   - macOS: `~/Library/Application Support/dev.unify.desktop`, `~/Library/Logs/dev.unify.desktop`
4. Özel anahtarı silin:
   - Windows: **Kimlik Bilgisi Yöneticisi → Windows Kimlik Bilgileri** altında `device-key-default-….dev.unify.desktop`
   - macOS: **Anahtar Zinciri Erişimi** içinde `dev.unify.desktop` hizmet adlı kayıt
5. macOS: **Gizlilik ve Güvenlik** altında Erişilebilirlik, Giriş İzleme ve Yerel Ağ listelerinden Unify'ı kaldırın. Otomatik başlatma açıksa `~/Library/LaunchAgents` altındaki Unify kaydı kaldırma sırasında silinmezse elle silin.
6. Windows Güvenlik Duvarı'nda oluşan `unify.exe` kuralını **Gelişmiş ayarlar → Gelen Kuralları** altından silin.
