# Known limitations

| Topic | State | Reason / alternative |
|---|---|---|
| macOS runtime | Not yet run on a real Mac | Development environment is Windows; verify with [test plan](test-plan.md) M1–M29 |
| Signing and notarization | Packages are unsigned (ad-hoc on macOS) | Requires Apple Developer and Authenticode certificates |
| Lock screen, UAC, elevated windows | Cannot be controlled | OS security boundary; not bypassed |
| Pointer while control is remote | Stays visible at the centre of the primary display on Windows and at the exit edge on the Mac | No public API lets a background app hide the system pointer |
| Keyboard layout | Physical key positions are forwarded; the same layout is expected on both sides | Character-based translation planned for phase 5 |
| AltGr from the Mac | Use right Option | Left Option is sent as Alt so Alt+Tab etc. keep working |
| Caps Lock towards the Mac | Uppercase reaches apps; the system Caps Lock light may not change | macOS does not fully support synthetic Caps Lock |
| Media keys (volume, play, brightness) | Not supported; volume keys may work only from Windows to the Mac | macOS emits these as system-defined events, not key events; no HID consumer page support |
| Clipboard | Plain text only, ≤ 16 MB | Images, files and rich text planned for phase 5 |
| Drag and drop between screens | Not supported | Switching is blocked while a mouse button is held |
| Pointer speed | The controller's speed setting applies; on Retina, Windows pixel motion maps 1:1 to Mac points | A speed scaling setting can be added |
| Natural scrolling | The Mac's natural direction is forwarded to Windows unchanged | "Invert scroll direction" setting |
| Number of computers | Two | By design |
| mDNS | Blocked on some networks | Manual address |
| WebView2 when the window is closed | Processes stay in memory | "Quit" from the tray exits completely |
| UI languages | English (default) and Turkish | New locales are added as a table in `app/src/i18n/` typed against `en.ts` |
| Theme | Light theme only | The interface follows the white brand theme regardless of the system dark mode |
| Screen view | Mac to Windows only, main display, H.264, no audio; pointer shape does not follow the Mac | First version; real end-to-end latency must be measured on your network |
