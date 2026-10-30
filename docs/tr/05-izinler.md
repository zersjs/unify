# İşletim sistemi izinleri

## macOS

| İzin | Neden | Yoksa ne olur |
|---|---|---|
| **Erişilebilirlik** | Diğer bilgisayardan gelen tuş ve fare olaylarını üretmek (`CGEventPost`) ve kontrol diğer bilgisayardayken yerel olayları yutmak (filtreleyen `CGEventTap`) | Mac kontrol edilemez; Mac'ten Windows'a geçişte yerel olaylar engellenemez |
| **Giriş İzleme** | Klavye ve fare olaylarını dinlemek | Mac'in klavye/faresiyle Windows kontrol edilemez |
| **Yerel Ağ** (macOS 15+) | Aynı ağdaki Windows bilgisayara bağlanmak ve mDNS ile bulmak | Bağlantı kurulamaz; sistem ilk bağlantıda izin sorar |

Adımlar:

1. Unify → **Güvenlik ve izinler → İzin iste**. macOS, Giriş İzleme ve Erişilebilirlik için sistem diyaloglarını gösterir.
2. **Sistem Ayarları → Gizlilik ve Güvenlik → Erişilebilirlik** altında Unify'ı açın.
3. **Giriş İzleme** altında Unify'ı açın. macOS bu izinden sonra uygulamayı yeniden başlatmanızı isteyebilir.
4. İlk bağlantıda çıkan "yerel ağdaki aygıtları bulmak" sorusuna **İzin Ver** deyin.

Unify izinleri 3 saniyede bir yeniden kontrol eder; izin verildiğinde giriş yakalamayı kendiliğinden başlatır. İzin yokken arayüz "İzin gerekiyor" uyarısı gösterir ve klavye/fare kontrolü yapılabildiğini iddia etmez.

İmzasız derlemelerde kod imzası her derlemede değiştiği için macOS, anahtar açık görünse bile eski izni yeni derlemeye uygulamaz. **Güvenlik ve izinler → İzinleri sıfırla** ile sıfırlayın, Sistem Ayarları'nda Unify'ı yeniden açın ve **Unify'ı yeniden başlat** düğmesine basın. Elle yapmak için listedeki Unify kaydını **–** ile kaldırıp yeniden ekleyin.

Anahtar Zinciri: ilk açılışta macOS, Unify'ın `dev.unify.desktop` kaydına erişmesi için onay isteyebilir; **Her Zaman İzin Ver** seçin.

## Windows 11

- Yönetici yetkisi **gerekmez**. Düşük seviye klavye/fare kancaları ve `SendInput` normal kullanıcı haklarıyla çalışır.
- **Güvenlik duvarı**: ilk dinlemede izin sorulur. Yalnızca Özel ağ profiline izin verin.
- **Sınırlar (UIPI ve güvenli masaüstü):**
  - Yönetici olarak çalışan pencerelere (ör. yönetici terminali, Görev Yöneticisi) normal bir uygulama giriş gönderemez. Bu pencereler öndeyken Mac'ten yazılanlar iletilmez. Windows bu engeli `SendInput` dönüş değeriyle bildirmez.
  - Kilit ekranı, oturum açma ekranı ve UAC onay penceresi (güvenli masaüstü) uzaktan kontrol edilemez. Bu ekranlar açıkken kendi klavyenizi kullanın.
  - Unify'ı yönetici olarak çalıştırmak yönetici pencerelerini kontrol etmeyi mümkün kılar ancak önerilmez; güvenli masaüstü yine kontrol edilemez.
