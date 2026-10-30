# Network and firewall requirements

| Traffic | Protocol / port | Direction | Scope |
|---|---|---|---|
| Unify session | TCP 47823 (configurable on the network settings page) | Inbound and outbound | Local network only; TLS 1.3 |
| Screen view | UDP, random port chosen per session on the viewing computer | Inbound to the viewer | Local network only; AES-256-GCM with a per-session key |
| Device discovery | UDP 5353, multicast 224.0.0.251 / ff02::fb (mDNS) | Inbound and outbound | Local network only; optional |

No internet access is required. Unify never connects to external servers.

## Windows Firewall

In the first-run prompt tick **Private networks** only. If your network is classified as Public, inbound connections are blocked: **Settings → Network & internet → Wi-Fi/Ethernet → Network profile type → Private**.

To add the rule manually (elevated PowerShell):

```powershell
New-NetFirewallRule -DisplayName "Unify" -Direction Inbound -Protocol TCP -LocalPort 47823 -Profile Private -Action Allow
```

## macOS

- If the Application Firewall is on, macOS asks to allow incoming connections; choose **Allow**.
- macOS 15 and later require the Local Network privacy permission ([permissions.md](permissions.md)).

## When discovery does not work

Some routers and corporate networks block mDNS. In that case:

1. Find the Windows IP address: **Settings → Network & internet → Properties → IPv4 address**.
2. On the Mac open Unify → **Network → Other computer's address**, enter `IP:47823` and save.
3. Both computers attempt to connect; as long as one can reach the other, the session is established.

Guest Wi-Fi networks usually block client-to-client traffic ("AP isolation"); connect both computers to the main network.
