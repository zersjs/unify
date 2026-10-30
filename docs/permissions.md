# Operating system permissions

## macOS

| Permission | Why | Without it |
|---|---|---|
| **Accessibility** | Generate key and mouse events received from the other computer (`CGEventPost`) and swallow local events while control is remote (filtering `CGEventTap`) | The Mac cannot be controlled; local events cannot be blocked when controlling Windows |
| **Input Monitoring** | Listen to keyboard and mouse events | The Mac's keyboard and mouse cannot control Windows |
| **Local Network** (macOS 15+) | Connect to the Windows PC on the same network and find it via mDNS | No connection; the system asks on first connection |

Steps:

1. Unify → **Security & permissions → Request permission**. macOS shows dialogs for Input Monitoring and Accessibility.
2. Enable Unify under **System Settings → Privacy & Security → Accessibility**.
3. Enable Unify under **Input Monitoring**. macOS may ask you to restart the app.
4. Answer **Allow** to the "find devices on local networks" prompt at the first connection.

Unify re-checks permissions every 3 seconds and starts input capture automatically once granted. Without permission the UI shows a warning and does not claim keyboard/mouse control works.

With unsigned builds the code signature changes on every build, so macOS does not apply an earlier grant to a new build even though the switch still looks on. Use **Security & permissions → Reset permissions**, turn Unify on again in System Settings, then press **Restart Unify**. Doing the same by hand: remove the Unify entry with **–** and add it again.

Keychain: on first launch macOS may ask to let Unify access the `dev.unify.desktop` item; choose **Always Allow**.

## Windows 11

- Elevation is **not** required. Low-level keyboard/mouse hooks and `SendInput` work with normal user rights.
- **Firewall**: Windows prompts on first listen. Allow only the Private profile.
- **Limits (UIPI and secure desktop):**
  - A normal application cannot send input to elevated windows (an admin terminal, Task Manager). While such a window is focused, typing from the Mac is not delivered. Windows does not report this through the `SendInput` return value.
  - The lock screen, sign-in screen and UAC prompt (secure desktop) cannot be controlled remotely. Use the local keyboard there.
  - Running Unify elevated allows controlling elevated windows but is not recommended; the secure desktop remains unreachable.
