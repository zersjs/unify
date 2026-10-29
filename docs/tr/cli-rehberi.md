# `unify-cli` komut satırı rehberi

`unify-cli`, Unify çalışma zamanını doğrudan terminalden başlatan komut satırı giriş noktasıdır (`crates/unify-cli/src/main.rs`). Masaüstü kabuğunun kullandığı `unify-runtime` aktörünü, yerel `unify-platform` kancalarını, mDNS keşif servisini ve TLS 1.3 taşıma katmanını birebir çalıştırır; standart girdi üzerinden satır tabanlı komutlar alır.

## Başlatma

Depo kök dizininden:

```bash
cargo run -p unify-cli -- [SEÇENEKLER]
```

Derlenmiş yayın ikilisiyle:

```bash
cargo build --release -p unify-cli
./target/release/unify-cli --log info
```

## Başlatma bayrakları

| Bayrak | Varsayılan | Açıklama |
|---|---|---|
| `--profile <AD>` | `default` | Profil kimliği. Ayrı bir yapılandırma klasörü (`profile-<AD>`) ve işletim sistemi anahtar kasasında ayrı bir özel anahtar kaydı seçer. |
| `--config-dir <YOL>` | İşletim sistemi varsayılanı | `config.json` ve `identity.json` dosyalarının tutulduğu dizini doğrudan belirler; `--profile` dizin hesabını geçersiz kılar. |
| `--port <PORT>` | `config.json` (`47823`) | Disk üzerindeki `config.json` dosyasını değiştirmeden yalnızca bu süreç için dinleme portunu belirler. |
| `--no-discovery` | `false` | UDP çoklu yayın mDNS ilanını ve taramasını (`_unify._tcp.local.`) kapatır. `address <ad:port>` ile birlikte kullanılır. |
| `--log <FİLTRE>` | `info` | `stderr` akışına yazılan `tracing_subscriber` günlük seviyesi (`error`, `warn`, `info`, `debug` veya `unify_net=debug,info`). |

### Çıkış kodları

- `0` — `quit`, `exit`, `stdin` kapanışı veya `Ctrl+C` ile temiz kapanış.
- `1` — çalışma zamanı başlatma hatası (`Failed to start: <hata_kodu>`).
- `2` — desteklenmeyen işletim sistemi (`unify_platform::native()` yalnızca Windows ve macOS destekler).

## Etkileşimli komutlar

`unify-cli` çalışırken `stdin` üzerinden satır satır komut okur ve durum değişimlerini eşzamansız olarak `stdout` akışına yazar.

| Komut | Argümanlar | İşlev |
|---|---|---|
| `help` | yok | Komut özetini yazdırır. |
| `status` | yok | Bağlantı durumu, ms cinsinden RTT gecikmesi, eş cihaz adı/işletim sistemi, kontrol durumu, yerel port ve sertifika parmak izi, anahtar kasası türü, işletim sistemi izinleri, yakalama durumu, pano durumu, mDNS ile bulunan cihazlar ve son hata kodunu gösterir. |
| `pair` | yok | Ev sahibi olarak 120 saniyelik SPAKE2 eşleştirme penceresi açar ve 6 haneli kodu (`XXX XXX`) yazdırır. |
| `join` | `<adres> <kod>` | `<adres>` (`ad:port`) üzerindeki cihaza bağlanır ve `<kod>` ile SPAKE2 doğrulamasını başlatır. |
| `approve` | yok | SPAKE2 MAC doğrulaması geçen gelen eşleştirme isteğini kabul eder. |
| `reject` | yok | Gelen eşleştirme isteğini reddeder ve oturumu kapatır. |
| `side` | `left \| right \| top \| bottom` | Eşleştirilmiş bilgisayarın bu bilgisayara göre fiziksel yönünü ayarlar ve `layout_revision` değerini artırır. |
| `address` | `[ad:port]` | Argüman verildiğinde `peer.manual_address` alanını kaydeder; boş çağrıldığında elle tanımlı adresi siler. |
| `input` | `on \| off` | Klavye ve fare paylaşımını açar veya kapatır (`settings.input_sharing`). |
| `clipboard` | `on \| off` | Düz metin pano eşitlemesini açar veya kapatır (`settings.clipboard_sync`). |
| `resync` | yok | Yerel düz metin panosunu yeniden okuyup bağlı cihaza gönderir. |
| `release` | yok | Giriş kontrolünü anında yerel bilgisayara geri alır ve basılı tuşları bırakır. |
| `permissions` | yok | Eksik işletim sistemi izinlerini ister ve macOS **Gizlilik ve Güvenlik → Erişilebilirlik** sayfasını açar. |
| `unpair` | yok | Sabitlenmiş eş kaydını (`config.json` içindeki `peer`) siler ve açık oturumu kapatır. |
| `quit` / `exit` | yok | Çalışma zamanı aktörünü durdurur, kancaları serbest bırakır ve `0` koduyla çıkar. |

## Yapılandırma ve anahtar saklama dizinleri

`--config-dir` verilmediğinde `unify-cli`, `directories::ProjectDirs::from("dev", "unify", "Unify")` yolunu kullanır:

| Platform | Varsayılan profil (`--profile default`) | İsimli profil (`--profile b`) |
|---|---|---|
| Windows 11 | `%APPDATA%\unify\Unify\config` | `%APPDATA%\unify\Unify\config\profile-b` |
| macOS | `~/Library/Application Support/dev.unify.Unify` | `~/Library/Application Support/dev.unify.Unify/profile-b` |

Tauri masaüstü uygulaması (`app/`) `dev.unify.desktop` paket kimliğini kullanır (Windows'ta `%APPDATA%\dev.unify.desktop`, macOS'ta `~/Library/Application Support/dev.unify.desktop`). Aynı dizini `unify-cli` ile açmak için `--config-dir` bayrağını verin.

## Tek bilgisayarda iki profilli yerel geri döngü (loopback) testi

`--profile`, `--port` ve `--no-discovery` bayraklarıyla tek bir bilgisayarda iki ayrı terminal açarak eşleştirme, TLS sertifika sabitleme ve oturum yenileme akışlarını doğrulayabilirsiniz. Aynı işletim sisteminde iki giriş kancasının birbirini tetiklememesi için her iki süreçte `input off` komutunu verin:

Terminal 1 (`alpha` profili, port `47823`):

```bash
cargo run -p unify-cli -- --profile alpha --port 47823 --no-discovery
```

```text
input off
pair
```

Terminal 2 (`beta` profili, port `47824`):

```bash
cargo run -p unify-cli -- --profile beta --port 47824 --no-discovery
```

```text
input off
join 127.0.0.1:47823 <alpha-terminalindeki-6-haneli-kod>
```

Terminal 1'de `approve` komutunu verin, ardından yeniden bağlanmaların doğru portlara gitmesi için elle adres atayın:

- Terminal 1: `address 127.0.0.1:47824`
- Terminal 2: `address 127.0.0.1:47823`

Her iki terminalde `status` yazarak `Connected` durumunu, ölçülen RTT süresini ve sabitlenen SHA-256 parmak izini kontrol edebilirsiniz.
