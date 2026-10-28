# Unify — Teknik değerlendirme ve karar kaydı

Unify, bir Apple Silicon MacBook ile bir Windows 11 bilgisayar arasında yerel ağ üzerinden klavye, fare ve metin panosu paylaşan kişisel bir masaüstü uygulamasıdır. Bulut, hesap, telemetri ve yapay zekâ bileşeni yoktur.

## 1. Teknik engeller

| Alan | macOS | Windows 11 |
|---|---|---|
| Giriş yakalama | `CGEventTap` gerekir; olayları yutmak için etkin (filtreleyen) tap ve **Erişilebilirlik** izni, yalnızca dinlemek için **Giriş İzleme** izni gerekir. Tap, geri çağrı yavaş kalırsa sistem tarafından devre dışı bırakılır; yeniden etkinleştirilmesi gerekir. | `SetWindowsHookEx(WH_KEYBOARD_LL / WH_MOUSE_LL)` yönetici yetkisi olmadan çalışır. Geri çağrı `LowLevelHooksTimeout` süresini aşarsa kanca atlanır. |
| Giriş üretme | `CGEventPost` için Erişilebilirlik izni gerekir. İmzasız/ad-hoc imzalı derlemelerde her yeniden derleme izni geçersiz kılabilir. | `SendInput`. UIPI nedeniyle yükseltilmiş (yönetici) pencerelere, UAC güvenli masaüstüne ve kilit ekranına giriş gönderilemez. Bu bir işletim sistemi sınırıdır; aşılmaya çalışılmaz. |
| Pano | Değişiklik bildirimi API'si yoktur; `NSPasteboard.changeCount` 250 ms aralıkla okunur (tek tamsayı karşılaştırması). | `AddClipboardFormatListener` ile olay tabanlı `WM_CLIPBOARDUPDATE`. |
| Yerel ağ | macOS 15+ Yerel Ağ gizlilik izni ister; `Info.plist` içinde `NSLocalNetworkUsageDescription` ve `NSBonjourServices` zorunludur. | Windows Güvenlik Duvarı ilk dinlemede izin sorar; Ortak ağ profilinde gelen bağlantılar engellenebilir. |
| İmleç | Kontrol diğer bilgisayara geçtiğinde imleç park edilir; arka plandaki bir uygulama imleci herkese açık API ile gizleyemez. | Aynı durum: imleç birincil ekranın ortasına park edilir ve görünür kalır. |
| Klavye düzeni | Fiziksel tuş konumu (USB HID kullanım kodu) aktarılır; karakteri alıcı bilgisayarın düzeni üretir. Apple ISO klavyelerde sol üst tuş ile sol Shift yanındaki tuş sanal kodları farklıdır; ISO/ANSI tespiti yapılır. | Tarama kodları (set 1, `E0` genişletilmiş) HID'e eşlenir. |

## 2. Seçilen mimari

**Ortak Rust çekirdeği + Tauri 2 kabuğu + platforma özgü native giriş/pano katmanı.**

- Ağ, protokol, eşleştirme, TLS, durum makineleri ve pano eşitleme Rust ile yazılır; iki platformda aynı koddur ve Windows üzerinde birim/entegrasyon testleriyle doğrulanır.
- Giriş yakalama/üretme ve pano erişimi doğrudan işletim sistemi API'leriyle (`windows-sys`, `core-graphics`, `objc2-app-kit`) yazılır. Arayüz teknolojisi gecikme yoluna girmez: fare olayı kancadan ağa kadar yalnızca Rust kodundan geçer.
- Arayüz Tauri 2 ile sistem web görünümünde (macOS WKWebView, Windows WebView2) çalışır. Paket boyutu Electron'a göre yaklaşık 10 kat küçüktür; ayrı bir Chromium taşınmaz.

### Değerlendirilen alternatifler

| Seçenek | Artı | Eksi | Karar |
|---|---|---|---|
| SwiftUI (Mac) + WinUI 3 (Windows) + Rust çekirdek (FFI) | En native görünüm | İki ayrı arayüz kodu, iki FFI katmanı; SwiftUI kısmı bu Windows geliştirme ortamında derlenemez ve doğrulanamaz | Elendi: bakım maliyeti ve doğrulanamayan kod oranı yüksek |
| Tauri 2 + Rust | Tek arayüz kodu, sistem web görünümü, Rust ile doğrudan entegrasyon, menü çubuğu/tepsi ve otomatik başlatma desteği | Arayüz %100 native kontrol değildir; platform hissi CSS belirteçleriyle sağlanır | **Seçildi** |
| Electron | Olgun ekosistem | ~150 MB paket, ayrı Chromium, ek bellek; giriş katmanı yine native modül gerektirir | Elendi: öncelik sırasındaki "hafiflik ve düşük kaynak" hedefiyle çelişir |

## 3. Güvenlik kararları

- **Taşıma:** TLS 1.3 (rustls, ring sağlayıcısı), karşılıklı kimlik doğrulama. Her cihaz kendi P-256 anahtarını ve kendinden imzalı sertifikasını üretir. Eşleştirme sonrası karşı cihazın sertifikası SHA-256 parmak iziyle sabitlenir (pinning). Sabitlenmemiş sertifikayla gelen bağlantı, protokol mesajı işlenmeden kapatılır.
- **Eşleştirme:** 6 haneli tek kullanımlık kod + SPAKE2 (parola doğrulamalı anahtar değişimi). Kod ağda hiçbir biçimde gönderilmez; çevrimdışı kaba kuvvet mümkün değildir. Doğrulama MAC'i TLS anahtar türetim çıktısına (RFC 5705 exporter) bağlanır; aradaki adam saldırısı iki farklı TLS oturumu üreteceği için doğrulama başarısız olur. Kod 120 saniye geçerlidir, 5 hatalı denemede iptal edilir. Kod üreten bilgisayarda kullanıcı açık onay verir.
- **Tekrar oynatma:** TLS 1.3 kayıt sıra numaraları ve oturum başına yeni anahtarlar; 0-RTT kapalıdır.
- **Anahtar saklama:** Özel anahtar macOS Anahtar Zinciri / Windows Kimlik Bilgisi Yöneticisi'nde (`keyring`). Sertifika (açık bilgi) yapılandırma klasöründe.
- **Giriş kabulü:** Uzak giriş olayları yalnızca doğrulanmış oturumda ve yalnızca bu bilgisayar kontrolü açıkça devraldığında (`Enter` sonrası) uygulanır.
- **Gizlilik:** Pano içeriği ve tuş kodları günlüğe yazılmaz; pano geçmişi tutulmaz. Parola yöneticilerinin işaretlediği gizli pano içerikleri (`ExcludeClipboardContentFromMonitorProcessing`, `org.nspasteboard.ConcealedType`) gönderilmez.

## 4. MVP kapsamı

Kesin olarak bulunacaklar: güvenli eşleştirme, TLS bağlantı, otomatik yeniden bağlanma, mDNS keşfi + elle IP, çift yönlü metin panosu, iki yönlü klavye/fare aktarımı, kenardan geçiş durum makinesi, kısayolla geçiş ve acil geri alma (`Ctrl+Alt+Shift+Space`; Mac'te `Control+Option+Shift+Space`), bağlantı kopunca basılı tuşların bırakılması, Türkçe arayüz, tepsi/menü çubuğu, otomatik başlatma (varsayılan kapalı).

Kapsam dışı (Aşama 5): resim/dosya/zengin metin panosu, ekranlar arası sürükle-bırak, medya tuşları (HID Consumer sayfası), karakter tabanlı düzen çevirisi, ikiden fazla bilgisayar.

## 5. İzinler

- macOS: Erişilebilirlik (giriş üretme ve olay yutma), Giriş İzleme (klavye dinleme), Yerel Ağ (macOS 15+). Ayrıntı: [05-izinler.md](05-izinler.md).
- Windows: yönetici yetkisi gerekmez. Güvenlik duvarı izni (Özel ağ) gerekir.

## 6. Gerçek donanımda doğrulanması gerekenler

Bu geliştirme ortamı Windows 11'dir. macOS kodu `cargo check --target aarch64-apple-darwin` ile tip denetiminden geçer ancak burada çalıştırılamaz. Aşağıdakiler yalnızca gerçek bir M2 Mac ve Windows 11 bilgisayarla doğrulanabilir; test planı [09-test-plani.md](09-test-plani.md):

1. macOS izin akışı ve `CGEventTap` ile olay yutma.
2. Mac'te üretilen klavye olaylarının Türkçe Q düzeninde doğru karakter vermesi (ISO tuş takası dahil).
3. Kaydırma yönü ve hızının iki yönde doğal hissettirmesi.
4. Wi-Fi üzerinde gerçek gecikme (uygulama, ping/pong ile ölçülen RTT'yi arayüzde gösterir).
5. Uyku/uyanma ve ağ değişimi sonrası yeniden bağlanma.
6. Retina ölçekleme ile Windows fiziksel piksel koordinatları arasındaki geçiş noktalarının doğruluğu.
