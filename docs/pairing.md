# Secure device pairing

1. Open Unify on both computers.
2. On computer A (for example Windows): **Status → Pair → Create code**. A 6-digit code is shown for 120 seconds.
3. On computer B (for example the Mac): **Pair → Join with code**. Pick A from **Computers found on the network** or type its address (`192.168.1.20` or `192.168.1.20:47823`), enter the code, press **Connect**.
4. A shows a pairing request with B's fingerprint. B shows the same value as **This computer's fingerprint**. If they match, choose **Pair**.
5. Both computers store each other's certificate and switch to **Connected** within seconds.

## What happens underneath

- The code never crosses the network. Both sides derive a shared key from it with SPAKE2; someone without the code gets one guess per connection.
- Confirmation messages are bound to the TLS session and both certificate fingerprints; an interceptor cannot pass verification.
- After 5 wrong attempts the code is revoked; create a new one.
- Later connections need no code; only the stored certificates are accepted.

## Removing a pairing and invalidating keys

- **Paired device → Remove pairing**: disconnects and deletes the peer certificate; that device can no longer connect.
- **Security & permissions → Reset device identity**: regenerates this computer's key and certificate. Every previously paired device stops recognising it. Use this if a device may be compromised.
- Pairing a new device replaces the current pairing (Unify is designed for two computers).
