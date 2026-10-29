# Crate ve yapılandırma referansı

Unify, `crates/` altındaki yedi Rust crate'inden oluşan bir Cargo çalışma alanı ve `app/` altındaki Tauri 2 masaüstü kabuğu olarak yapılandırılmıştır. İşletim sistemine özgü FFI çağrıları yalnızca `unify-platform` içinde bulunur; diğer tüm crate'ler platformdan bağımsız Rust kodudur.

## Bağımlılık grafiği

```text
unify-proto
    ↑
unify-core
    ↑
    ├── unify-security
    ├── unify-net
    └── unify-platform
            ↑
      unify-runtime
        ↑       ↑
  unify-cli   app/src-tauri
```

## Çalışma alanı crate'leri

### `crates/unify-proto`

Ağ mesaj tipleri, ikili çerçeveleme, alan sınır doğrulamaları ve klavye tarama kodu tabloları. Ağ G/Ç veya işletim sistemi çağrısı içermez.

- **Modüller:** `frame`, `keymap`, `message`.
- **Çerçeve yapısı:** 4 bayt big-endian `u32` uzunluk başlığı + `postcard` ile kodlanmış `Message`.
- **Temel protokol sabitleri:**
  - `PROTOCOL_VERSION = 1`
  - `FRAME_HEADER_LEN = 4` bayt
  - `MAX_FRAME_LEN = 65_536` bayt (`64 KiB`)
  - `CLIPBOARD_CHUNK_LEN = 16_384` bayt (`16 KiB`)
  - `MAX_CLIPBOARD_TEXT_LEN = 16_777_216` bayt (`16 MiB`)
  - `MAX_DEVICE_NAME_CHARS = 64`
  - `MAX_DEVICE_ID_LEN = 64` ASCII onaltılık karakter
  - `RATIO_SCALE = 10_000` (`0.0..=1.0` kenar oranını `0..=10_000` aralığına eşler)
  - `SPAKE_MESSAGE_LEN = 33` bayt
- **Doğrulama (`Message::validate`):** uzun cihaz adlarını, onaltılık olmayan cihaz kimliklerini, `10_000` üstü kenar oranlarını, `2^20` sınırını aşan fare/tekerlek deltalarını, `16 MiB` üstü pano aktarımlarını, boş veya `16 KiB` üstü pano parçalarını ve hatalı SPAKE2 paketlerini reddeder.

### `crates/unify-core`

Soketlerden ve yerel kancalardan bağımsız olarak test edilen durum makineleri ve yapılandırma katmanı.

- **Modüller:** `backoff`, `capture`, `clipboard`, `config`, `geometry`, `keys`, `switcher`.
- **Zamanlama sabitleri:**
  - `PING_INTERVAL_MS = 1_000` (`1 sn` kalp atışı)
  - `PEER_TIMEOUT_MS = 5_000` (`5 sn` sessizlikte oturum kapatılır)
  - `SLEEP_GAP_MS = 6_000` (`> 6 sn` zamanlayıcı sıçraması uyku dönüşü sayılır ve anında yeniden bağlanılır)
  - `CONNECT_TIMEOUT_MS = 3_000`
  - `HANDSHAKE_TIMEOUT_MS = 5_000`
- **`switcher`:** köşe koruma bölgelerini, isteğe bağlı kenar bekleme süresini (`0..=1000 ms`), 250 ms dönüş beklemesini ve yeniden kurma (re-arm) kuralını uygulayan kenar geçiş durum makinesi.
- **`keys`:** yerel ve uzak basılı USB HID kodlarını izler, açık olduğunda `Command ↔ Control` dönüşümü yapar, Windows `AltGr` kaynaklı sahte sol Ctrl olayını yutar ve oturum koptuğunda basılı tuşlar için bırakma olayı üretir.
- **`clipboard`:** giden UTF-8 metni `16 KiB` parçalara böler, gelen parçaları birleştirip SHA-256 özetini doğrular ve son gönderilen/alınan özetleri tutarak yankı döngüsünü engeller.

### `crates/unify-security`

Kriptografik cihaz kimliği, işletim sistemi anahtar kasası, karşılıklı TLS 1.3 yapılandırması ve SPAKE2 eşleştirme.

- **Modüller:** `identity`, `keystore`, `pairing`, `tls`.
- **Cihaz kimliği:** ECDSA P-256 anahtar çifti (`rcgen`) ve kendinden imzalı X.509 sertifikası üretir; SHA-256 `Fingerprint` hesaplar.
- **Anahtar saklama (`SecretStore`):** `OsSecretStore` özel anahtarı macOS Anahtar Zinciri veya Windows Kimlik Bilgisi Yöneticisi (`keyring`) içinde tutar; `MemorySecretStore` testlerde kullanılır.
- **TLS 1.3 (`rustls` + `ring`):** yalnızca TLS 1.3 kabul edilir, 0-RTT ve oturum biletleri kapalıdır; iki taraf da sabitlenmiş SHA-256 parmak izine göre sertifika doğrular.
- **Eşleştirme sabitleri:**
  - `PAIRING_CODE_DIGITS = 6`
  - `PAIRING_CODE_TTL_MS = 120_000` (`120 sn`)
  - `MAX_PAIRING_FAILURES = 5`
  - `PAIRING_CONFIRM_TIMEOUT_MS = 60_000` (`60 sn` kullanıcı onay süresi)
  - Onay HMAC-SHA256 değeri hem SPAKE2 ortak çıktısını hem RFC 5705 TLS dışa aktarıcısını (`PAIRING_EXPORTER_LABEL`) bağlar.

### `crates/unify-net`

Eşzamansız Tokio taşıma katmanı ve yerel ağ keşfi.

- **Modüller:** `discovery`, `transport`.
- **Öncelikli kuyruklar:** `spawn_writer`, yüksek öncelikli kanalı (`HIGH_PRIORITY_CAPACITY = 8_192`; `Input`, `Enter`, `Leave`, `Ping`, `Pong`) düşük öncelikli pano kanalından (`LOW_PRIORITY_CAPACITY = 64`; `ClipboardBegin`, `ClipboardChunk`) önce boşaltır ve soket yavaşladığında ardışık `MouseMove` deltalarını birleştirir.
- **Keşif:** `mdns-sd` üzerinden `_unify._tcp.local.` hizmetini yayınlar ve dinler (`DiscoveryEvent::Found`, `DiscoveryEvent::Lost`).

### `crates/unify-platform`

Tüm `unsafe` FFI çağrılarını ve işletim sistemi olay döngülerini yalıtan soyutlama sınırı (`Platform`, `Injector`, `StopHandle`).

- **`windows` (`#[cfg(windows)]`):** ayrı Win32 mesaj iş parçacığında `WH_KEYBOARD_LL` ve `WH_MOUSE_LL`, Unify'ın kendi ürettiği olayları tanıması için `INJECTION_MARKER = 0x5546_5949` işaretli `SendInput`, `AddClipboardFormatListener` (`WM_CLIPBOARDUPDATE`) ve `ExcludeClipboardContentFromMonitorProcessing` / `CanIncludeInClipboardHistory` / `Clipboard Viewer Ignore` filtreleri.
- **`macos` (`#[cfg(target_os = "macos")]`) ve `mac_logic`:** `CFRunLoop` iş parçacığında `CGEventTap` (`TapDisabledByTimeout` durumunda otomatik yeniden etkinleştirme), ayrı enjeksiyon iş parçacığında `CGEventPost`, 250 ms aralıklı `NSPasteboard.changeCount` kontrolü, `org.nspasteboard.ConcealedType` / `TransientType` filtreleme, Apple ISO tuş takası ve Erişilebilirlik / Giriş İzleme sorguları.
- **`fake`:** `unify-runtime` tümleştirme testlerinde kullanılan bellek içi deterministik `Platform` uygulaması.

### `crates/unify-runtime`

Oturum döngüsünü, aynı anda bağlanma çakışmasını (küçük `device_id` kazanır), üstel geri çekilmeli yeniden bağlanmayı, eşleştirme adımlarını, kontrol devrini ve `watch::Receiver<StatusSnapshot>` yayınını yöneten tek görevli Tokio aktörü (`Actor::spawn` → `RuntimeHandle`).

### `crates/unify-cli` ve `app/`

- **`crates/unify-cli`:** `RuntimeHandle` komutlarını `stdin`/`stdout` üzerinden çalıştıran terminal ikilisi ([cli-rehberi.md](cli-rehberi.md)).
- **`app/` (`src-tauri` + `src`):** `RuntimeHandle` durum anlık görüntülerini ve `SettingsPatch` güncellemelerini sıkı CSP (`connect-src ipc: http://ipc.localhost`) altında IPC üzerinden sunan Tauri 2 uygulaması (`dev.unify.desktop`).

## `config.json` şema referansı

Yapılandırma dosyası atomik olarak yazılır (`config.json.tmp` + `sync_all` + yeniden adlandırma). `config.json` bozulmuşsa `ConfigStore` dosyayı `config.json.corrupt` adıyla yedekler ve varsayılan değerlerle yeni dosya oluşturur.

Üst düzey `ConfigFile` (`CONFIG_SCHEMA_VERSION = 1`):

| Alan | Tip | Açıklama |
|---|---|---|
| `schema_version` | `u32` | Güncel sürüm `1`. |
| `settings` | `Settings` | Kullanıcı ayarları (aşağıdaki tablo). |
| `peer` | `PairedPeer \| null` | Sabitlenmiş eş cihaz kaydı (`device_id`, `device_name`, `os`, PEM `certificate`, SHA-256 `fingerprint`, `paired_at`, isteğe bağlı `manual_address` ve `last_address`). |

### `Settings` alanları

| Anahtar | Tip | Varsayılan | Sınırlar ve normalleştirme |
|---|---|---|---|
| `device_name` | `string` | Sistem bilgisayar adı veya `"Unify"` | Kontrol karakterleri temizlenir, kırpılır, en fazla `64` karakter tutulur; boş kalırsa `"Unify"` atanır. |
| `listen_port` | `u16` | `47823` | `1024` altındaki değerler `47823` değerine döndürülür. |
| `peer_side` | `"Left" \| "Right" \| "Top" \| "Bottom"` | `"Right"` | `SettingsPatch` ile değiştiğinde `layout_revision` artırılır. |
| `layout_revision` | `u64` | `0` | İki cihaz arasındaki ekran düzeni uyuşmazlıklarını çözen artan sayaç/zaman damgası. |
| `input_sharing` | `bool` | `true` | Etkin oturumda klavye ve fare aktarımını açar. |
| `clipboard_sync` | `bool` | `true` | Çift yönlü düz metin pano eşitlemesini açar. |
| `edge_switch` | `bool` | `true` | İmleç ekran kenarına geldiğinde geçiş yapılmasını açar. |
| `edge_dwell_ms` | `u32` | `0` | `0..=1000` aralığına (`MAX_DWELL_MS`) sınırlandırılır. Geçişten önce kenarda beklenecek milisaniye. |
| `corner_guard_px` | `u32` | `0` | `0..=200` aralığına (`MAX_CORNER_GUARD_PX`) sınırlandırılır. Her köşede geçişin kapatıldığı piksel mesafesi. |
| `switch_hotkey` | `bool` | `true` | `Ctrl+Alt+Shift+Space` (`Control+Option+Shift+Space`) kısayolunu açar. |
| `swap_command_control` | `bool` | `true` | Platformlar arası kontrolde gönderen tarafta `Command ↔ Control` dönüşümü uygular. |
| `invert_scroll` | `bool` | `false` | Uzak bilgisayara enjekte edilen dikey ve yatay tekerlek yönünü ters çevirir. |
| `mac_keyboard_layout` | `"auto" \| "iso" \| "ansi"` | `"auto"` | Apple ISO sol üst / sol Shift tuş takası algısını belirler. |
| `discovery` | `bool` | `true` | Yerel mDNS ilanını ve taramasını açar. |
| `autostart` | `bool` | `false` | Masaüstü uygulamasında oturum açılışında `--background` ile başlatmayı kaydeder. |
| `log_level` | `"error" \| "warn" \| "info" \| "debug"` | `"info"` | Çalışma zamanı günlük ayrıntı düzeyi. |
| `language` | `"en" \| "tr"` | `"en"` | Masaüstü arayüz dili (`en` veya `tr`). |
