# Ağ ve güvenlik duvarı gereksinimleri

| Trafik | Protokol / port | Yön | Kapsam |
|---|---|---|---|
| Unify oturumu | TCP 47823 (Ağ ve bağlantı ayarından değiştirilebilir) | Gelen ve giden | Yalnızca yerel ağ; TLS 1.3 |
| Cihaz keşfi | UDP 5353, çok noktaya yayın 224.0.0.251 / ff02::fb (mDNS) | Gelen ve giden | Yalnızca yerel ağ; isteğe bağlı |

İnternet bağlantısı gerekmez. Unify dışarıya hiçbir sunucuya bağlanmaz.

## Windows Güvenlik Duvarı

İlk çalıştırmada çıkan diyalogda yalnızca **Özel ağlar** kutusunu işaretleyin. Ağınız "Ortak" olarak tanımlıysa gelen bağlantılar engellenir: **Ayarlar → Ağ ve İnternet → Wi-Fi/Ethernet → Ağ profili türü → Özel**.

Kuralı elle eklemek için (yönetici PowerShell):

```powershell
New-NetFirewallRule -DisplayName "Unify" -Direction Inbound -Protocol TCP -LocalPort 47823 -Profile Private -Action Allow
```

## macOS

- Uygulama Güvenlik Duvarı açıksa gelen bağlantı izni sorulur; **İzin Ver** seçin.
- macOS 15 ve sonrasında Yerel Ağ gizlilik izni gerekir ([05-izinler.md](05-izinler.md)).

## Keşif çalışmıyorsa

Bazı yönlendiriciler ve kurumsal ağlar mDNS'i engeller. Bu durumda:

1. Windows'ta IP adresini öğrenin: **Ayarlar → Ağ ve İnternet → Özellikler → IPv4 adresi**.
2. Mac'te Unify → **Ağ ve bağlantı → Diğer bilgisayarın adresi (elle)** alanına `IP:47823` yazıp kaydedin.
3. İki bilgisayar da bağlantı kurmayı dener; biri ulaşabildiği sürece bağlantı oluşur.

Misafir Wi-Fi ağları istemciler arası trafiği engeller ("AP isolation"); iki bilgisayarı aynı ana ağa bağlayın.
