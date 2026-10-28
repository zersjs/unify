# Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| Stuck on **Waiting for connection** | Unify closed on the other computer; firewall; different networks; mDNS blocked | Start Unify on the other computer; follow [network-and-firewall.md](network-and-firewall.md); enter the address manually |
| "The other computer does not recognise this device" | Pairing removed or identity reset on the other side | Remove the pairing on both sides and pair again |
| "Code could not be verified" | Wrong code or wrong computer | Check code and address; after 5 failures create a new code |
| "Permission required" on the Mac | Accessibility / Input Monitoring missing or lost after a rebuild | [permissions.md](permissions.md); remove and re-add the entry |
| Pointer does not cross the edge | Edge switching off; wrong position; mouse button held; corner guard | Check the display layout page; switching happens only with no button held |
| Pointer bounces back and forth | Positions conflict on the two computers | Drag the other computer to the correct side on the Display layout page; the setting propagates to the other side automatically |
| Control stuck on the other computer | Network outage, frozen app on the other side | Shortcut: Windows `Ctrl+Alt+Shift+Space`, Mac `Control+Option+Shift+Space`. After 5 s without a reply control returns automatically |
| Some Windows windows ignore typing | Elevated window, UAC or lock screen | OS limitation ([permissions.md](permissions.md)) |
| Wrong characters | Different keyboard layouts; Mac ISO/ANSI detection | Select Turkish Q on both computers; check the Mac keyboard type setting |
| AltGr characters such as `@`, `€` missing | Left Option used on the Mac | Use right Option on the Mac (maps to AltGr on Windows) |
| Clipboard not arriving | Sync off; content marked secret; above 16 MB; not text | See "Last skipped content" on the clipboard page |
| No reconnect after sleep | Network interface came up late | Wait a few seconds or press "Reconnect now" |

## Logs

**Logs & diagnostics → Open log folder.** Files rotate daily; the last 7 days are kept:

- Windows: `%LOCALAPPDATA%\dev.unify.desktop\logs\`
- macOS: `~/Library/Logs/dev.unify.desktop/`

Logs contain no clipboard content, keystrokes or keys. "Generate report" produces only status, settings and counters; attach it when reporting a bug.
