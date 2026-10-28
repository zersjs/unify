# Bilinen sınırlamalar

| Konu | Durum | Neden / alternatif |
|---|---|---|
| macOS çalışma zamanı | Gerçek Mac'te henüz çalıştırılmadı | Geliştirme ortamı Windows; [test planı](09-test-plani.md) M1–M29 ile doğrulanmalı |
| İmza ve notarization | Paketler imzasız (macOS ad-hoc) | Apple Developer ve Authenticode sertifikası gerekir |
| Kilit ekranı, UAC, yönetici pencereleri | Kontrol edilemez | İşletim sistemi güvenlik sınırı; aşılmaya çalışılmaz |
| Kontrol başka bilgisayardayken imleç | Windows'ta birincil ekranın ortasında, Mac'te çıkılan kenarda görünür kalır | Arka plandaki bir uygulamanın imleci gizlemesi için genel API yok |
| Klavye düzeni | Fiziksel tuş konumu aktarılır; iki tarafta aynı düzen beklenir | Karakter tabanlı çeviri Aşama 5'te |
| Mac'te AltGr | Sağ Option kullanılmalı | Sol Option, Windows'ta Alt olarak gider (Alt+Tab vb. korunur) |
| Caps Lock ile Mac'e yazma | Büyük harf durumu Mac'te uygulamaya yansır, sistem Caps Lock ışığı değişmeyebilir | macOS sentetik Caps Lock'u tam desteklemez |
| Medya tuşları (ses, oynat, parlaklık) | Desteklenmiyor; yalnızca Windows → Mac yönünde ses tuşları çalışabilir | macOS bu tuşları klavye olayı olarak değil sistem olayı olarak üretir; HID Consumer sayfası desteği yok |
| Pano | Yalnızca düz metin, ≤ 16 MB | Resim, dosya, zengin metin Aşama 5 |
| Ekranlar arası sürükle-bırak | Desteklenmiyor | Fare düğmesi basılıyken geçiş yapılmaz |
| Fare hızı | Kontrol eden bilgisayarın hız ayarı geçerlidir; Retina'da Windows piksel hareketi Mac noktasına birebir uygulanır | Hız ölçekleme ayarı eklenebilir |
| Doğal kaydırma | Mac'in doğal kaydırma yönü Windows'a aynen gider | "Kaydırma yönünü ters çevir" ayarı |
| Bilgisayar sayısı | İki bilgisayar | Tasarım gereği |
| mDNS | Bazı ağlarda engelli | Elle adres |
| Pencere kapatıldığında WebView2 | Süreçler bellekte kalır | Tepsiden "Çık" tamamen kapatır |
| Arayüz dilleri | İngilizce (varsayılan) ve Türkçe | Yeni diller `app/src/i18n/` altında `en.ts` tipine göre eklenir |
