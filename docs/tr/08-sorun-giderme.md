# Sorun giderme

| Belirti | Olası neden | Çözüm |
|---|---|---|
| "Bağlantı bekleniyor" sürekli kalıyor | Diğer bilgisayarda Unify kapalı; güvenlik duvarı; farklı ağlar; mDNS engelli | Diğer bilgisayarda Unify'ı açın; [07](07-ag-ve-guvenlik-duvari.md) adımlarını uygulayın; elle adres girin |
| "Diğer bilgisayar bu cihazı tanımıyor" | Karşı tarafta eşleştirme kaldırılmış veya kimlik yenilenmiş | İki tarafta da eşleştirmeyi kaldırıp yeniden eşleştirin |
| "Kod doğrulanamadı" | Yanlış kod veya başka bir bilgisayara bağlanıldı | Kodu ve adresi kontrol edin; 5 hatadan sonra yeni kod oluşturun |
| Anahtarlar açık olduğu hâlde Mac'te "İzin gerekiyor" | Anahtarlar eski bir sürüme ait | **Güvenlik ve izinler → İzinleri sıfırla**, Unify'ı yeniden açın, **Unify'ı yeniden başlat** ([05-izinler.md](05-izinler.md)) |
| Windows diğer bilgisayarda izin eksik diyor, imleç kenarda kalıyor | Mac giriş üretemediği için kontrolü reddetti | Mac'te aynı çözüm |
| Fare kenarda geçmiyor | Kenar geçişi kapalı; yanlış konum ayarı; fare düğmesi basılı; köşe koruması | Ekran düzeni sayfasını kontrol edin; geçiş fare düğmesi bırakıldığında yapılır |
| Fare sürekli gidip geliyor | Konum iki bilgisayarda çelişiyor (eski sürüm) | Ekran düzeninde konumu yeniden seçin; ayar diğer tarafa otomatik uygulanır |
| Kontrol diğer bilgisayarda takıldı | Ağ kesintisi, karşı tarafta uygulama donması | Kısayol: Windows `Ctrl+Alt+Shift+Boşluk`, Mac `Control+Option+Shift+Boşluk`. Bağlantı 5 sn yanıt vermezse kontrol kendiliğinden geri döner |
| Windows'ta bazı pencerelere yazılamıyor | Yönetici penceresi, UAC veya kilit ekranı | İşletim sistemi sınırıdır ([05](05-izinler.md)) |
| Yanlış karakterler çıkıyor | İki bilgisayarda farklı klavye düzeni; Mac ISO/ANSI algısı | İki bilgisayarda da Türkçe Q seçin; Klavye ve fare → Mac klavye tipi |
| `@`, `€` gibi AltGr karakterleri çıkmıyor | Mac'te sol Option kullanılıyor | Mac'te sağ Option tuşunu kullanın (Windows'ta AltGr'ye karşılık gelir) |
| Pano gelmiyor | Pano eşitleme kapalı; içerik gizli işaretli; 16 MB üstü; metin değil | Pano eşitleme sayfasında "Son atlanan içerik" satırına bakın |
| Uyku sonrası bağlanmıyor | Ağ arayüzü geç açıldı | Birkaç saniye bekleyin veya "Şimdi yeniden bağlan" |

## Günlükler

**Günlükler ve tanılama → Günlük klasörünü aç.** Dosyalar günlük döndürülür, son 7 gün tutulur:

- Windows: `%LOCALAPPDATA%\dev.unify.desktop\logs\`
- macOS: `~/Library/Logs/dev.unify.desktop/`

Günlükler pano içeriği, tuş vuruşu veya anahtar içermez. "Rapor oluştur" yalnızca durum, ayar ve sayaç bilgisi üretir; hata bildirirken bu raporu paylaşın.
