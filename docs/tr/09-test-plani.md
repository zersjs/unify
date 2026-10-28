# Test planı

## 1. Otomatik testler (Windows 11 geliştirme makinesinde çalıştırıldı)

| Paket | Test sayısı | Kapsam |
|---|---|---|
| `unify-proto` | 19 | Tüm mesaj tiplerinin serileştirilmesi, parçalı okuma, sıfır/aşırı uzunluk, bozuk ve eksik paket, alan sınırları, HID ↔ Windows ↔ macOS tablolarının çift yönlü tutarlılığı, Türkçe Q tuş konumları, ISO tuş takası |
| `unify-core` | 56 | Kenar geçiş durum makinesi (silahlanma, bekleme, köşe koruması, tekrar geçiş engeli), uzak imleç ve dönüş kenarı, çoklu ekran geometrisi, pano döngü önleme ve tekrar gönderim, parça birleştirme (sıra, taşma, özet, UTF-8), modifier eşleme, kısayol filtresi, takılı tuş bırakma, yapılandırma yükleme/bozuk dosya kurtarma, adres doğrulama, yeniden deneme zamanlayıcısı |
| `unify-security` | 14 | Kimlik üretimi ve saklama, anahtar-sertifika uyuşmazlığı, kimlik yenileme, SPAKE2 doğru/yanlış kod, kanal bağlama, yansıtma ve parmak izi takası saldırıları, kod penceresi süresi ve deneme sınırı |
| `unify-net` | 10 | Öncelikli yazıcı (giriş olayı pano parçalarının önüne geçer), fare hareketi birleştirme, kuyruk taşması, bozuk çerçeve, mDNS kayıt doğrulama |
| `unify-platform` | 8 + 1 | macOS saf mantığı (modifier bayrakları, kaydırma dönüşümü, çift tıklama), Windows mutlak koordinat ve tarama kodu üretimi; **gerçek Windows panosu** üzerinde Türkçe/CRLF/emoji gidiş-dönüş ve `WM_CLIPBOARDUPDATE` bildirimi (`--include-ignored` ile) |
| `unify-runtime` | 17 | İki runtime arasında gerçek TLS: onaylı eşleştirme, yanlış kod, yerel ret, pano iki yön (400 KB dahil) ve döngüsüzlük, gizli içerik filtresi, pano kapatma, kenar geçişi + Command→Ctrl, izinsiz giriş reddi, kopmada tuş bırakma, kısayolla dönüş, paylaşımı kapalı eşin reddi, yeniden başlatma sonrası bağlanma, eşleştirme kaldırma, yetkisiz sertifika ve kimlik doğrulama öncesi mesaj reddi, RTT ölçümü |
| `app` (Vitest) | 29 | Yay animasyon motoru (yerleşme, tek yönlü hareket, azaltılmış hareket), İngilizce ve Türkçe durum ekranı, ayar yaması, eşleştirme düğmesi, izin uyarısı, kontrolü geri alma, kod girişi, İngilizce/Türkçe anahtar eşliği, ekran düzeni sürükle-bırak (ok tuşları, en yakın kenara bırakma, iptal), yay, momentum izdüşümü, kenar esnemesi ve bırakma hızı |

Komutlar: `cargo test --workspace`, `cargo test -p unify-platform -- --include-ignored`, `npm --prefix app test`.

Ek doğrulamalar:

- macOS katmanı `cargo check -p unify-platform --target aarch64-apple-darwin` ile tip denetiminden geçti. **Mac'te çalıştırılmadı.**
- İki gerçek `unify-cli` süreci bu Windows makinesinde gerçek Windows Kimlik Bilgisi Yöneticisi, TLS ve mDNS ile eşleştirildi ve bağlandı (loopback RTT 0–1 ms).
- Windows kurulum paketi (`Unify_0.1.0_x64-setup.exe`, 3,39 MB) üretildi; release uygulaması çalıştırılıp arayüz görüntülendi.

## 2. Ölçülen ve hedeflenen performans

| Ölçüt | Hedef | Ölçülen | Ortam |
|---|---|---|---|
| Ping/pong RTT | Wi-Fi'de p50 < 8 ms, kablolu < 2 ms | 0–1 ms | Aynı makinede loopback; **Wi-Fi ölçümü yapılmadı** |
| `unify.exe` boşta bellek | < 60 MB çalışma kümesi | 42,1 MB çalışma kümesi, 9,9 MB özel | Windows 11, release, pencere açık |
| `unify.exe` boşta CPU | < %1 tek çekirdek | 20 sn ölçümde hiçbir iş parçacığı 20 ms'yi aşmadı | Windows 11, release, fare hareketsiz |
| WebView2 bellek | — | 6 süreç, ~358 MB çalışma kümesi toplamı (paylaşılan sayfalar dahil, fazla sayılır) | Pencere açıkken; pencere kapatılınca süreçler bellekte kalır |
| Fare hareketi gecikmesi | Fark edilmeyecek düzey | **Ölçülmedi**; gerçek donanımda ölçülecek | — |

## 3. Gerçek donanımda manuel kabul testleri

Ortam: Apple M2 MacBook (macOS sürümünü not edin), Windows 11 bilgisayar, aynı Wi-Fi veya Ethernet. Her testte sonucu ve sürümleri kaydedin. Simülasyon sonuçları bu tabloya yazılmaz.

| No | Adımlar | Beklenen | MVP ölçütü |
|---|---|---|---|
| M1 | Mac'te DMG'den kur, aç | Gatekeeper uyarısı, Yine de Aç ile açılır; menü çubuğunda simge | 1 |
| M2 | Windows'ta kurulum paketini çalıştır | Yönetici istenmez; güvenlik duvarı sorusu; tepside simge | 2 |
| M3 | Mac'te izin verilmeden durum ekranı | "İzin gerekiyor" uyarısı; klavye/fare paylaşımı çalışıyor gibi gösterilmez | 10 |
| M4 | Erişilebilirlik ve Giriş İzleme ver | 3 sn içinde uyarı kalkar, yakalama "çalışıyor" | 10 |
| M5 | Windows'ta kod oluştur, Mac'te mDNS listesinden seç ve kodu gir | Windows'ta onay penceresi; parmak izleri eşleşir; onay sonrası iki tarafta "Bağlandı" | 3 |
| M6 | Yanlış kodla dene | "Kod doğrulanamadı"; eşleştirme olmaz | 3 |
| M7 | Mac'te metin kopyala (Türkçe karakterler, çok satır), Windows'ta Ctrl+V | Birebir aynı metin | 4, 8 |
| M8 | Windows'ta kopyala, Mac'te Command+V | Birebir aynı metin | 5, 8 |
| M9 | 5 MB metin kopyala ve aynı anda fareyi hareket ettir | Fare akıcı kalır; metin tamamen gelir | 4 |
| M10 | Parola yöneticisinden parola kopyala | Diğer bilgisayara gitmez; "Son atlanan içerik: Gizli" | Güvenlik |
| M11 | Mac solda, Windows sağda; Mac faresini sağ kenara götür | İmleç Windows'un sol kenarında aynı yükseklikte belirir | 6, 7 |
| M12 | Windows'ta Not Defteri'nde Türkçe Q ile `ğüşıöçİ` ve `Shift` ile büyük harf yaz | Doğru karakterler | 8 |
| M13 | Mac klavyesiyle Command+C / Command+V / Command+Z | Windows'ta Ctrl+C / Ctrl+V / Ctrl+Z | 8 |
| M14 | Sağ Option + Q (Windows TR-Q'da `@`) | `@` | 8 |
| M15 | Sol, sağ, orta tık, çift tık, sürükle-bırak (aynı ekran içinde), tekerlek, yatay kaydırma, geri/ileri tuşları | Hepsi doğru; çift tık seçimi çalışır | 6 |
| M16 | Fareyi Windows'un sol kenarından geri getir | Kontrol Mac'e döner; kenarda takılma veya gidip gelme yok | 7 |
| M17 | Ters yön: Windows faresi ve klavyesiyle Mac'i kontrol et (M11–M16'nın aynası) | Aynı sonuçlar; Mac'te çift tık çalışır | 6 |
| M18 | Kontrol Windows'tayken Shift basılı tut ve Mac'te Wi-Fi'yi kapat | ≤ 5 sn içinde Windows'ta Shift bırakılır; Mac kontrolü geri alır | 9 |
| M19 | Wi-Fi'yi yeniden aç | Kullanıcı müdahalesi olmadan yeniden bağlanır | 9 |
| M20 | Mac'i uyut, 2 dk sonra uyandır | Yeniden bağlanır | 9 |
| M21 | Windows'u yeniden başlat (otomatik başlatma açık) | Windows oturum açınca Unify arka planda başlar ve bağlanır | 9 |
| M22 | Kontrol Windows'tayken Mac'te `Control+Option+Shift+Boşluk` | Kontrol hemen Mac'e döner | 9 |
| M23 | Windows'ta yönetici olarak açılmış terminale yazmayı dene | Yazılmaz; belgelenmiş sınır | 10 |
| M24 | Windows'u kilitle (Win+L) ve Mac'ten yazmayı dene | Kilit ekranı kontrol edilemez; belgelenmiş sınır | 10 |
| M25 | Mac'e ikinci monitör bağla, ekran düzeni sayfasını kontrol et | Ekranlar listelenir; geçiş en dış kenardan yapılır | 7 |
| M26 | Windows'ta ölçeklemeyi %150 yap | Geçiş noktaları doğru konumda | 7 |
| M27 | Bir taraftan eşleştirmeyi kaldır | Bağlantı kesilir ve yeniden kurulmaz | Güvenlik |
| M28 | Koyu/açık tema, klavyeyle gezinme, VoiceOver/Narrator ile ana ekran | Okunaklı, tüm kontroller erişilebilir | 11 |
| M29 | Fare hareketi gecikmesini algısal olarak değerlendir, Ağ ve bağlantı sayfasındaki ölçülen gecikmeyi not et | Fark edilmeyecek düzey; RTT değeri kaydedilir | Performans |
| M30 | Ekran düzeni sayfasında diğer bilgisayarı fare ve trackpad ile her kenara sürükle, fırlat, ok tuşlarını kullan | Kutu imleci birebir izler, sıçramadan en yakın kenara yerleşir; diğer bilgisayarda ayna konum görünür | 7, 11 |
| M31 | Genel → Dil ayarını English ve Türkçe arasında değiştir | Pencere ve tepsi menüsü hemen dil değiştirir; ayar yeniden başlatmada korunur | 11 |

## 4. Test edilemeyen durumlar

- macOS üzerindeki tüm çalışma zamanı davranışı (izin akışı, `CGEventTap`, olay üretimi, `NSPasteboard`, Retina koordinatları, ISO algılama) bu ortamda **çalıştırılmadı**; yalnızca derleyici tip denetimi yapıldı.
- Gerçek Windows'ta düşük seviye kancaların ve `SendInput`'un uçtan uca davranışı otomatik testte kullanılmadı (kullanıcı girişini etkileyeceği için); M11–M17 ile doğrulanmalıdır.
- Wi-Fi gecikmesi, uyku/uyanma ve ağ değişimi gerçek donanımda ölçülmelidir.
