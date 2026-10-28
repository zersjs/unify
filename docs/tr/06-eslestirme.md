# Güvenli cihaz eşleştirme

1. Her iki bilgisayarda Unify'ı açın.
2. Bilgisayar A'da (örneğin Windows): **Durum → Eşleştir → Kod oluştur**. 6 haneli kod 120 saniye görünür.
3. Bilgisayar B'de (örneğin Mac): **Eşleştir → Koda katıl**. "Ağda bulunan bilgisayarlar" listesinden A'yı seçin veya adresini girin (`192.168.1.20` ya da `192.168.1.20:47823`), kodu yazın, **Bağlan**.
4. A'da "Eşleştirme isteği" penceresi açılır ve B'nin parmak izini gösterir. B'de aynı parmak izi "Bu bilgisayarın parmak izi" olarak görünür. İkisi aynıysa **Eşleştir**.
5. İki bilgisayar birbirinin sertifikasını kaydeder ve birkaç saniye içinde "Bağlandı" durumuna geçer.

## Arka planda ne olur

- Kod ağda gönderilmez. İki taraf SPAKE2 ile koddan ortak bir anahtar türetir; kodu bilmeyen biri her bağlantıda yalnızca bir tahmin yapabilir.
- Onay mesajları TLS oturumuna ve iki sertifika parmak izine bağlıdır; araya giren biri doğrulamayı geçemez.
- 5 hatalı denemede kod geçersiz olur; yeni kod oluşturmanız gerekir.
- Sonraki bağlantılarda kod gerekmez; yalnızca kayıtlı sertifikalar kabul edilir.

## Eşleştirmeyi kaldırma ve anahtarları geçersiz kılma

- **Eşleştirilmiş cihaz → Eşleştirmeyi kaldır**: bağlantı kesilir, karşı sertifika silinir; o cihaz artık bu bilgisayara bağlanamaz.
- **Güvenlik ve izinler → Cihaz kimliğini yenile**: bu bilgisayarın anahtarı ve sertifikası yeniden üretilir. Daha önce eşleştirilmiş her cihaz bu bilgisayarı tanımaz. Bir cihazın ele geçirildiğinden şüphelenirseniz kullanın.
- Yeni bir cihaz eşleştirmek mevcut eşleştirmenin yerine geçer (Unify iki bilgisayar için tasarlanmıştır).
