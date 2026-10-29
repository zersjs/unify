# Unify

[English](../../README.md) · **Türkçe**

Unify, bir macOS (Apple Silicon) bilgisayar ile bir Windows 11 bilgisayar arasında yerel ağ üzerinden **klavye, fare ve metin panosu** paylaşan kişisel bir masaüstü uygulamasıdır. Fareyi ekranın kenarından geçirdiğinizde kontrol diğer bilgisayara geçer; kopyaladığınız metin diğer bilgisayarda yapıştırılabilir.

- Uçtan uca TLS 1.3, sertifika sabitleme ve SPAKE2 ile kodla eşleştirme
- Hesap, bulut, telemetri yok; internet gerekmez
- Rust çekirdek, native giriş API'leri, Tauri 2 arayüzü

## Durum

| Bileşen | Durum |
|---|---|
| Windows 11 | Derlendi, kurulum paketi üretildi, otomatik testler geçti, uygulama çalıştırıldı |
| macOS (M2) | Kod yazıldı ve `aarch64-apple-darwin` için tip denetiminden geçti; **gerçek Mac'te henüz çalıştırılmadı** |
| Gerçek donanım kabul testleri | Bekliyor ([test planı](09-test-plani.md)) |

## Belgeler

1. [Hızlı başlangıç](hizli-baslangic.md)
2. [`unify-cli` komut satırı rehberi](cli-rehberi.md)
3. [Crate ve yapılandırma referansı](crate-referansi.md)
4. [Teknik değerlendirme ve karar kaydı](01-teknik-degerlendirme.md)
5. [Mimari ve protokol](02-mimari-ve-protokol.md)
6. [Tehdit modeli](03-tehdit-modeli.md)
7. [Kurulum, derleme ve kaldırma](04-kurulum-ve-derleme.md)
8. [İşletim sistemi izinleri](05-izinler.md)
9. [Güvenli cihaz eşleştirme](06-eslestirme.md)
10. [Ağ ve güvenlik duvarı](07-ag-ve-guvenlik-duvari.md)
11. [Sorun giderme](08-sorun-giderme.md)
12. [Test planı](09-test-plani.md)
13. [Bilinen sınırlamalar](10-bilinen-sinirlamalar.md)

## Hızlı başlangıç

Arayüz varsayılan olarak İngilizcedir. **General → Language** ayarından **Türkçe** seçildiğinde bu belgelerdeki menü adları geçerli olur.

1. Her iki bilgisayara Unify'ı kurun ([kurulum](04-kurulum-ve-derleme.md)).
2. Mac'te Erişilebilirlik ve Giriş İzleme izinlerini verin ([izinler](05-izinler.md)).
3. Bir bilgisayarda **Eşleştir → Kod oluştur**, diğerinde **Koda katıl** ([eşleştirme](06-eslestirme.md)).
4. **Ekran düzeni** sayfasında diğer bilgisayarı fiziksel olarak durduğu tarafa sürükleyin.

Acil geri alma kısayolu: Windows `Ctrl+Alt+Shift+Boşluk`, Mac `Control+Option+Shift+Boşluk`.
