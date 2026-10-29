# Hızlı başlangıç

Bu rehber, aynı yerel ağdaki bir Mac (Apple Silicon) ile bir Windows 11 bilgisayarı `unify-cli` terminal prototipi veya yerel olarak derlenen masaüstü uygulaması üzerinden bağlama adımlarını açıklar.

## Sistem gereksinimleri

| Gereksinim | macOS | Windows |
|---|---|---|
| İşletim sistemi | macOS 13.0 veya üstü (Apple Silicon `aarch64`) | Windows 11 (`x86_64`) |
| Ağ | İstemci izolasyonu kapalı, aynı yerel alt ağ (Wi-Fi veya Ethernet) | Windows Ayarları'nda **Özel ağ** profili |
| Gelen port | TCP `47823` (ayarlanabilir) | Özel ağlar için TCP `47823` izni |
| İsteğe bağlı keşif | UDP `5353` çoklu yayın (`_unify._tcp.local.`) | UDP `5353` çoklu yayın (`_unify._tcp.local.`) |

Hazır CI artefaktı yerine kaynak koddan derleyecekseniz önce Rust 1.80+ ve Node.js 22+ kurun; ayrıntılar [04-kurulum-ve-derleme.md](04-kurulum-ve-derleme.md) sayfasındadır.

## 1. İki bilgisayarda Unify'ı başlatın

Kaynak dizininden doğrudan komut satırı prototipini çalıştırmak için her iki bilgisayarda şu komutu verin:

```bash
cargo run -p unify-cli -- --log info
```

Bunun yerine Tauri 2 masaüstü arayüzünü geliştirme modunda açmak için:

```bash
npm --prefix app ci
npm --prefix app run tauri dev
```

İlk açılışta her bilgisayar kendi P-256 anahtar çiftini ve kendinden imzalı sertifikasını üretir. Özel anahtar macOS Anahtar Zinciri'ne veya Windows Kimlik Bilgisi Yöneticisi'ne kaydedilir; gizli olmayan ayarlar `config.json` dosyasına yazılır.

## 2. İşletim sistemi izinlerini verin

### macOS tarafında

macOS, kullanıcı açıkça izin verene kadar yapay giriş üretimini ve genel klavye dinlemesini engeller:

1. **Sistem Ayarları → Gizlilik ve Güvenlik → Erişilebilirlik** sayfasından Unify'ı (`unify-cli` kullanıyorsanız çalıştırdığınız terminal uygulamasını) etkinleştirin.
2. **Sistem Ayarları → Gizlilik ve Güvenlik → Giriş İzleme** sayfasından aynı uygulamayı etkinleştirin.
3. macOS 15 ve sonrasında Unify dinlemeye başladığında çıkan **Yerel Ağ** iznini onaylayın.

`unify-cli` içinde `permissions` komutuyla sistem penceresini açabilir, ardından `status` çıktısında `accessibility=Granted` ve `input_monitoring=Granted` değerlerini doğrulayabilirsiniz ([05-izinler.md](05-izinler.md)).

### Windows 11 tarafında

Yönetici yetkisi gerekmez. İlk açılışta Windows Güvenlik Duvarı penceresi geldiğinde yalnızca **Özel ağlar** kutusunu işaretleyin, **Ortak ağlar** kutusunu boş bırakın ([07-ag-ve-guvenlik-duvari.md](07-ag-ve-guvenlik-duvari.md)).

## 3. Bilgisayarları eşleştirin

Eşleştirme, 120 saniye geçerli tek kullanımlık 6 haneli SPAKE2 koduyla yapılır.

### `unify-cli` ile eşleştirme

1. A bilgisayarında eşleştirme penceresini açın:
   ```text
   pair
   ```
   Terminal 6 haneli bir kod üretir (örneğin `482 910`).
2. A bilgisayarının yerel IP adresini öğrenin (veya B bilgisayarında `status` komutuyla keşfedilen cihaz listesine bakın).
3. B bilgisayarında A'nın adresini ve kodu girin:
   ```text
   join 192.168.1.24:47823 482910
   ```
4. A bilgisayarında B'nin cihaz adı, işletim sistemi ve SHA-256 sertifika parmak izi görünür. Doğrulayıp onaylayın:
   ```text
   approve
   ```

### Masaüstü penceresi ile eşleştirme

1. A bilgisayarında **Eşleştir → Kod oluştur** düğmesine tıklayın.
2. B bilgisayarında listeden A bilgisayarını seçin (veya `IP:47823` adresini elle yazın), 6 haneli kodu girin ve başlatın.
3. A bilgisayarında cihaz adını kontrol edip **Onayla** düğmesine basın.

Onay tamamlandığında her iki bilgisayar karşı tarafın SHA-256 sertifika parmak izini sabitler ve karşılıklı TLS 1.3 oturumunu otomatik kurar ([06-eslestirme.md](06-eslestirme.md)).

## 4. Fiziksel ekran yönünü ayarlayın

Diğer bilgisayarın masada hangi yönde durduğunu belirtin; imleç o kenara ulaştığında kontrol devredilir.

`unify-cli` içinde (Windows bilgisayar Mac'in sağında duruyorsa Mac üzerinde çalıştırın):

```text
side right
```

Geçerli değerler: `left`, `right`, `top`, `bottom`. Yön değiştirildiğinde `layout_revision` artar ve karşı bilgisayara tamamlayıcı yön bilgisi otomatik iletilir (Mac'te `Right` seçildiğinde Windows tarafı Mac'i `Left` olarak görür).

## 5. Kenar geçişini, panoyu ve acil geri alma kısayolunu deneyin

1. **Önce acil geri alma kısayolu:** Windows'ta `Ctrl+Alt+Shift+Space`, macOS'ta `Control+Option+Shift+Space` tuşlarına basın. Kontrol karşı bilgisayardayken bu kısayol girişi anında yerel bilgisayara döndürür ve basılı kalmış tuşları bırakır.
2. **Kenar geçişi:** İmleci ayarladığınız kenara götürün. Kontrol, aynı yükseklik veya genişlik oranından diğer bilgisayara geçer; geri dönmek için karşı ekrandaki ters kenardan çıkın.
3. **Düz metin panosu:** Bir bilgisayarda UTF-8 metin kopyalayın (`Cmd+C` veya `Ctrl+C`) ve diğer bilgisayarda yapıştırın (`Ctrl+V` veya `Cmd+V`). `swap_command_control` varsayılan olarak açık olduğundan Mac klavyesinden Windows'u yönetirken `Cmd+C` tuşu karşı tarafa `Ctrl+C` olarak gider.

Kablosuz ağ çoklu yayını engelliyorsa `address 192.168.1.24:47823` komutuyla doğrudan adres tanımlayın veya [08-sorun-giderme.md](08-sorun-giderme.md) sayfasına bakın.
