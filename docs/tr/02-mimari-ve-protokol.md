# Mimari ve protokol

## Modüller

| Crate / klasör | Sorumluluk | Platform bağımlılığı |
|---|---|---|
| `crates/unify-proto` | Mesaj tipleri, çerçeveleme (4 bayt uzunluk + postcard), alan doğrulama, USB HID ↔ Windows tarama kodu ↔ macOS sanal tuş kodu tabloları | Yok |
| `crates/unify-core` | Yapılandırma (JSON, atomik yazma), ekran geometrisi, kenar geçiş durum makinesi, uzak imleç, pano eşitleme ve parça birleştirme, tuş yönlendirme/kısayol filtresi, Command↔Ctrl eşleme, yeniden deneme zamanlayıcısı | Yok |
| `crates/unify-security` | Cihaz kimliği (P-256 + kendinden imzalı sertifika), anahtar deposu (Keychain / Credential Manager), TLS 1.3 yapılandırmaları ve sertifika sabitleme, SPAKE2 eşleştirme | Yok |
| `crates/unify-net` | TLS bağlantı kurma/kabul, öncelikli çerçeve yazıcı (giriş olayları önce, pano parçaları sonra, ardışık fare hareketlerini birleştirme), mDNS keşfi | Yok |
| `crates/unify-platform` | `Platform` arayüzü; `windows/` (düşük seviye kancalar, `SendInput`, pano dinleyicisi, ekran listesi), `macos/` (`CGEventTap`, `CGEventPost`, `NSPasteboard`, izin sorguları), `fake` (testler için) | Evet |
| `crates/unify-runtime` | Tek iş parçacıklı aktör: bağlantı yaşam döngüsü, eşleştirme akışı, kontrol devri, pano akışı, durum yayını | Platform arayüzü üzerinden |
| `crates/unify-cli` | Teknik prototip: terminalden aynı runtime'ı çalıştırır | — |
| `app/` | Tauri 2 kabuğu (`src-tauri`) ve React arayüzü (`src`) | — |

Bağımlılık yönü: `proto ← core ← {security, net, platform} ← runtime ← {cli, app}`. Platform kodu yalnızca `unify-platform` içindedir.

## İş parçacıkları

- **Giriş kancası iş parçacığı** (Windows: mesaj döngüsü + `WH_KEYBOARD_LL`/`WH_MOUSE_LL`; macOS: `CFRunLoop` + `CGEventTap`). Geri çağrı yalnızca olayı sınıflandırır, kanala yazar ve yutma kararını verir; ağ veya disk işlemi yapmaz.
- **Pano iş parçacığı** (Windows: mesaj penceresi + `WM_CLIPBOARDUPDATE`; macOS: 250 ms `changeCount` kontrolü).
- **macOS enjeksiyon iş parçacığı**: `CGEventSource` ve modifier durumunu tek iş parçacığında tutar.
- **Tokio**: runtime aktörü, bağlantı okuyucu/yazıcı görevleri, dinleyici, keşif köprüsü.

## Protokol (sürüm 1)

Her çerçeve `u32` big-endian uzunluk + postcard ile kodlanmış `Message`. En büyük çerçeve 64 KiB; sıfır uzunluk, fazla bayt, bozuk alan veya sınır dışı değer bağlantıyı kapatır.

| Mesaj | Yön | Öncelik | Açıklama |
|---|---|---|---|
| `Hello` | iki yön | yüksek | Sürüm, cihaz kimliği, ad, işletim sistemi, dinleme portu |
| `Layout` | iki yön | yüksek | Ekran düzeni iddiası + revizyon (yenisi kazanır, eşitlikte küçük cihaz kimliği) |
| `Ping` / `Pong` | iki yön | yüksek | 1 sn aralık; 5 sn sessizlik = bağlantı ölü; RTT ölçümü |
| `Enter` | kontrol eden → kontrol edilen | yüksek | Giriş noktası (kenar + oran veya merkez) |
| `Leave` | kontrol edilen → kontrol eden | yüksek | İmleç dönüş kenarından çıktı, oran |
| `Release` | iki yön | yüksek | Kontrol bitti / reddedildi; alıcı basılı tuşları bırakır |
| `Input` | kontrol eden → kontrol edilen | yüksek | HID tuş kodu, göreli fare hareketi, düğme, tekerlek (120 = bir çentik) |
| `ClipboardBegin` / `ClipboardChunk` | iki yön | düşük | Uzunluk + SHA-256, ardından en fazla 16 KiB parçalar |
| `Goodbye` | iki yön | yüksek | Kapanış nedeni |
| `PairStart` / `PairReply` / `PairConfirm` / `PairResult` | eşleştirme | yüksek | SPAKE2 mesajları, MAC doğrulamaları, kullanıcı onayı |

### Bağlantı durum makinesi

```
TLS kuruldu ─┬─ giden oturum ── Hello gönder ── AwaitHello ── Hello doğrulandı ── Established
             ├─ gelen ───────── AwaitHello ─┬─ Hello (sabitlenmiş parmak izi) ── Established
             │                              ├─ PairStart (eşleştirme penceresi açık) ── eşleştirme akışı
             │                              └─ diğer her şey ── Goodbye(ProtocolViolation) + kapat
             └─ giden eşleştirme ── PairStart ── PairReply ── PairConfirm ⇄ ── PairResult ── kapat
```

- Kimliği doğrulanmamış bağlantıda `Input`, `Enter`, pano mesajları hiçbir koşulda işlenmez.
- İki taraf aynı anda bağlanırsa her iki bilgisayar aynı kuralı uygular: bağlantıyı başlatan cihaz kimliği küçük olan oturum kalır.
- Yeniden deneme: 0,5 sn'den başlayıp 10 sn'ye kadar üstel, ±%20 rastgele sapmalı. mDNS eşi yeniden gördüğünde beklemeden denenir.
- Uyku algılama: zamanlayıcı adımları arasında 6 sn'den büyük boşluk görülürse oturum kapatılır ve hemen yeniden bağlanılır.

### Kontrol devri

| Durum | Olay | Sonuç |
|---|---|---|
| Yerel | İmleç yapılandırılmış kenara geldi (silahlı, bekleme süresi doldu, köşe dışında, fare düğmesi basılı değil) | `Enter(kenar, oran)`, imleç park edilir, olaylar yutulur |
| Yerel | Kısayol | `Enter(Merkez)` |
| Kontrol ediyor | `Leave` | İmleç aynı orandaki kenar noktasına döner, 250 ms bekleme, yeniden silahlanma gerekir |
| Kontrol ediyor | Kısayol / bağlantı kopması / `Release` | Yerel moda dönüş |
| Kontrol ediliyor | İmleç dönüş kenarını aşmaya çalıştı | `Leave`, basılı tuşlar bırakılır |
| Kontrol ediliyor | `Release` / bağlantı kopması / yerel kısayol | Basılı tuş ve düğmeler bırakılır |
| İki taraf aynı anda `Enter` | — | Cihaz kimliği küçük olan kontrol eden olarak kalır |

Kenarda takılma ve sonsuz geçiş, "silahlanma" kuralıyla önlenir: dönüşten sonra imleç kenardan ayrılmadan yeniden geçiş yapılmaz.

### Tuş eşleme

- Tel üzerinde USB HID kullanım kodu (sayfa 7) taşınır; fiziksel konum korunur, karakteri alıcının klavye düzeni üretir.
- Command ↔ Ctrl dönüşümü gönderen tarafta, gönderenin ayarına göre yapılır.
- Kontrol başladığında yerelde basılı olan bir tuşun bırakılması yerel sisteme iletilir; kontrol eden tarafta takılı tuş kalmaz.
- Windows'ta AltGr'nin ürettiği sahte sol Ctrl olayı yutulur; karşı tarafa yalnızca sağ Alt gider.

### Pano eşitleme

- Yerel değişiklik → metin okunur → gizli işaretliyse, boşsa veya 16 MiB'ı aşıyorsa atlanır → son gönderilen veya son alınan içerikle aynıysa atlanır → gönderilir.
- Alınan içerik SHA-256 ile doğrulanır, UTF-8 kontrol edilir, panoya yazılır ve "son alınan" olarak işaretlenir; bu yazma yerel değişiklik bildirimi üretse bile geri gönderilmez.
- Bağlantı yokken yapılan son kopya bekletilir, bağlantı kurulunca gönderilir.
- Pano parçaları düşük öncelikli kuyruktadır; yazıcı her adımda önce giriş kuyruğunu boşaltır.
