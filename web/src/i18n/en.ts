export type CrateId =
  | "unify-proto"
  | "unify-core"
  | "unify-security"
  | "unify-net"
  | "unify-platform"
  | "unify-runtime"
  | "unify-cli";

export type SeamPhase = "armed" | "dwell" | "crossed" | "cooldown";

export type ClipboardDirection = "mac-to-win" | "win-to-mac";

export const en = {
  meta: {
    title: "Unify — One Keyboard. Two Computers.",
    description:
      "Open-source, local-first desktop utility for sharing one keyboard, mouse, and plain-text clipboard between macOS and Windows 11 over a direct TLS 1.3 connection.",
  },
  brand: {
    wordmark: "unify",
    name: "Unify",
    skipToContent: "Skip to main content",
  },
  nav: {
    ariaLabel: "Primary navigation",
    overview: "Overview",
    security: "Security",
    openSource: "Open source",
    docs: "Docs",
    github: "GitHub",
    menuOpen: "Menu",
    menuClose: "Close",
  },
  hero: {
    eyebrow: "Open source · Local first",
    headlineLine1: "One workspace.",
    headlineLine2: "Two computers.",
    supportingCopy:
      "Use one keyboard and mouse across your Mac and Windows PC. Move between screens naturally and share text through a direct, encrypted local connection.",
    primaryCta: "Explore on GitHub",
    secondaryCta: "See how it works",
    specsLine: "macOS (Apple Silicon) · Windows 11 · Direct LAN over TLS 1.3",
    stage: {
      ariaLabel:
        "Workspace demonstration showing a MacBook Pro and a Windows 11 desktop monitor side by side as a single pointer crosses the screen boundary",
      macLabel: "MacBook Pro · macOS",
      winLabel: "Studio PC · Windows 11",
      macMenuFinder: "Finder",
      macMenuFile: "File",
      macMenuEdit: "Edit",
      macMenuView: "View",
      macWindowOneTitle: "Terminal — unify-workspace",
      macWindowOneLine1: "$ cargo test --workspace",
      macWindowOneLine2: "running 74 tests across 7 crates",
      macWindowOneLine3: "test result: ok. 74 passed; 0 failed",
      macWindowTwoTitle: "Notes — Architecture",
      macWindowTwoLine1: "Direct peer session over local network",
      macWindowTwoLine2: "Certificate pinned · SHA-256",
      winWindowOneTitle: "crates\\unify-core\\src\\switcher.rs",
      winWindowOneLine1: "pub enum EdgeDecision {",
      winWindowOneLine2: "    Stay,",
      winWindowOneLine3: "    Switch { ratio: u16 },",
      winWindowOneLine4: "}",
      winWindowTwoTitle: "Explorer — unify",
      winWindowTwoLine1: "crates / unify-proto",
      winWindowTwoLine2: "crates / unify-security",
      activeControlLabel: "Active input",
      handoffLabel: "Boundary ratio",
      transportLabel: "Local TLS 1.3",
      switchToMac: "Focus Mac",
      switchToWin: "Focus Windows 11",
      resumeCycle: "Auto cycle",
    },
  },
  seam: {
    eyebrow: "Continuous input",
    headlineLine1: "Move between screens.",
    headlineLine2: "Not between setups.",
    body:
      "Guide the pointer to the configured screen edge and control transfers straight to your other computer. Vertical entry position maps proportionally between Apple Retina points and Windows physical pixels, turning two independent machines into one continuous desk.",
    mechanicsTitle: "Boundary mechanics in unify-core",
    mechanics: [
      {
        term: "Proportional entry ratio",
        detail: "Crossing at 62% height on macOS enters at 62% height on Windows 11.",
      },
      {
        term: "Configurable edge dwell",
        detail: "Brief pointer contact stays local so stray movements do not trigger an unintended switch.",
      },
      {
        term: "Corner guard zones",
        detail: "Screen corners remain reserved for native macOS Hot Corners and Windows window controls.",
      },
      {
        term: "250 ms return cooldown",
        detail: "Returning across the seam disarms the edge until the pointer moves away, preventing boundary bounce.",
      },
    ],
    phasesLabel: "Edge state machine",
    phases: {
      armed: {
        label: "Armed",
        summary: "Pointer moves freely on the local display outside the corner guard.",
      },
      dwell: {
        label: "Edge dwell",
        summary: "Pointer rests against the shared boundary until dwell_ms elapses.",
      },
      crossed: {
        label: "Controlling peer",
        summary: "Enter(Edge, ratio) transfers relative HID input over the prioritized queue.",
      },
      cooldown: {
        label: "Return & re-arm",
        summary: "Leave restores local position with a 250 ms cooldown before the next switch.",
      },
    } satisfies Record<SeamPhase, { label: string; summary: string }>,
    diagramMac: "macOS display",
    diagramWin: "Windows 11 display",
    diagramCornerGuard: "Corner guard",
    diagramSeam: "Shared edge",
  },
  clipboard: {
    eyebrow: "Bidirectional clipboard",
    headlineLine1: "Copy here.",
    headlineLine2: "Paste there.",
    body:
      "Copy a snippet on one computer and paste it immediately on the other. Unify synchronizes plain-text clipboard changes in both directions over the same authenticated local session, verifying every transfer with SHA-256 before writing to the destination clipboard.",
    scopeNotice:
      "Plain text only (UTF-8 up to 16 MiB). Image, file, and rich-text clipboard sharing are not currently supported.",
    secretNotice:
      "Items flagged as concealed or transient by supported password managers are filtered locally and never leave the source machine.",
    directions: {
      "mac-to-win": "macOS → Windows 11",
      "win-to-mac": "Windows 11 → macOS",
    } satisfies Record<ClipboardDirection, string>,
    replayButton: "Run transfer",
    stepLabels: {
      idle: "Ready on local session",
      copied: "Copied to source pasteboard",
      transmitting: "16 KiB low-priority frame · SHA-256 verified",
      pasted: "Written to peer clipboard",
    },
    macAppTitle: "macOS · Terminal note",
    macShortcut: "⌘C",
    winAppTitle: "Windows 11 · Editor",
    winShortcut: "Ctrl + V",
    snippetMacSource: "cargo check -p unify-platform --target aarch64-apple-darwin",
    snippetWinSource: "cargo test --workspace --locked",
    copiedBadge: "Copied",
    pastedBadge: "Pasted",
    waitingBadge: "Awaiting paste",
  },
  security: {
    eyebrow: "Private by design",
    headlineLine1: "Your network.",
    headlineLine2: "Your machines. Your data.",
    body:
      "Keyboard input and clipboard text never transit external relays. Every session is established directly between your two computers on the local network, authenticated with pinned device certificates.",
    diagramAria: "Direct local network architecture connecting a Mac and a Windows 11 PC over mutual TLS 1.3",
    diagramLeftTitle: "Mac (Apple Silicon)",
    diagramLeftMeta: "P-256 key in macOS Keychain",
    diagramCenterTop: "Direct LAN · TLS 1.3 mutual authentication",
    diagramCenterBottom: "One-time pairing via 6-digit SPAKE2 + TLS exporter binding",
    diagramRightTitle: "Windows 11 PC",
    diagramRightMeta: "P-256 key in Credential Manager",
    verifiedHeading: "Implemented mechanisms",
    verifiedItems: [
      {
        title: "Direct communication over the local network",
        detail: "Peers connect socket-to-socket over LAN without external signaling servers or relays.",
      },
      {
        title: "No cloud service required",
        detail: "Discovery, pairing, input forwarding, and clipboard sync operate entirely offline on your local subnet.",
      },
      {
        title: "No user account required",
        detail: "Zero sign-up, zero telemetry endpoints, and zero persistent history of keystrokes or clipboard text.",
      },
      {
        title: "TLS 1.3 with mutual authentication",
        detail: "Built on rustls and ring with 0-RTT and session tickets disabled; both peers present client and server certificates.",
      },
      {
        title: "SPAKE2 password-authenticated key exchange",
        detail: "The 6-digit pairing code never crosses the wire, expires after 120 seconds, and binds confirmation MACs to the TLS session.",
      },
      {
        title: "Paired certificates pinned by SHA-256 fingerprint",
        detail: "Unpinned certificates and pre-authentication input frames close the connection immediately.",
      },
      {
        title: "Password-manager secret exclusion",
        detail: "Clipboard entries marked with ConcealedType on macOS or ExcludeClipboardContentFromMonitorProcessing on Windows are skipped.",
      },
    ],
    validationHeading: "Verification status & scope",
    validationBody:
      "Unify has not undergone an independent third-party security audit. Protocol framing, SPAKE2 confirmation binding, certificate pinning, and secret clipboard exclusion are implemented in Rust and covered by automated workspace tests. Installers are currently unsigned, and end-to-end validation on physical Apple Silicon hardware is still in progress.",
  },
  openSource: {
    eyebrow: "Open source · No black box",
    headline: "Built in the open.",
    body:
      "Unify is an evolving open-source project built around transparency, local control, and a straightforward goal: make working across two computers feel natural.",
    cta: "View source on GitHub",
    dependencyRule: "Dependency flow: proto ← core ← {security, net, platform} ← runtime ← cli",
    cratesHeading: "Rust workspace architecture",
    cratesAria: "Select a Rust workspace crate to inspect its responsibility and source excerpt",
    earlyDevNote:
      "Unify is in early development. Developers are invited to read the crates, run the test suite on real hardware, report issues, and contribute focused pull requests.",
    crates: {
      "unify-proto": {
        role: "Wire messages, length-prefixed postcard framing, bounds validation, and USB HID key tables.",
        file: "crates/unify-proto/src/message.rs",
      },
      "unify-core": {
        role: "Display geometry, edge-switch state machine, remote cursor clamping, and clipboard chunk assembly.",
        file: "crates/unify-core/src/switcher.rs",
      },
      "unify-security": {
        role: "P-256 device identity, OS secret store integration, TLS 1.3 pinning, and SPAKE2 pairing.",
        file: "crates/unify-security/src/pairing.rs",
      },
      "unify-net": {
        role: "TLS transport, prioritized writer draining input ahead of clipboard chunks, and mDNS discovery.",
        file: "crates/unify-net/src/transport.rs",
      },
      "unify-platform": {
        role: "Native OS hooks: CGEventTap and NSPasteboard on macOS; WH_KEYBOARD_LL, SendInput, and WM_CLIPBOARDUPDATE on Windows.",
        file: "crates/unify-platform/src/lib.rs",
      },
      "unify-runtime": {
        role: "Single-task Tokio actor coordinating session lifecycle, pairing, control handoff, and key release on disconnect.",
        file: "crates/unify-runtime/src/control.rs",
      },
      "unify-cli": {
        role: "Command-line prototype entry point running the full Unify runtime directly from a terminal.",
        file: "crates/unify-cli/src/main.rs",
      },
    } satisfies Record<CrateId, { role: string; file: string }>,
  },
  platforms: {
    eyebrow: "Native integration",
    headlineLine1: "Mac meets Windows.",
    headlineLine2: "Finally, naturally.",
    body:
      "Input hooks run directly against native operating-system APIs in Rust with zero browser engine in the input path. Each machine keeps its own conventions while sharing a single keyboard and mouse.",
    macTitle: "macOS · Apple Silicon",
    macItems: [
      "Low-level pointer and key capture via CoreGraphics CGEventTap and CGEventPost",
      "Clipboard monitoring through NSPasteboard with ConcealedType filtering",
      "Private key stored in macOS Keychain; Local Network, Accessibility, and Input Monitoring permissions",
      "Emergency control release: Control + Option + Shift + Space",
    ],
    winTitle: "Windows 11",
    winItems: [
      "User-session hooks via WH_KEYBOARD_LL, WH_MOUSE_LL, and SendInput without elevation",
      "Event-driven WM_CLIPBOARDUPDATE listener with password-manager exclusion formats",
      "Private key stored in Windows Credential Manager; respects UAC and lock-screen OS boundaries",
      "Emergency control release: Ctrl + Alt + Shift + Space",
    ],
    sharedHeading: "Shared session capabilities",
    sharedItems: [
      {
        label: "Automatic mDNS discovery",
        detail: "Finds paired peers on the local subnet automatically, with manual IP and port connection when multicast is blocked.",
      },
      {
        label: "Keyboard & mouse forwarding",
        detail: "Forwards USB HID physical key positions with optional Command ↔ Ctrl mapping and releases held keys on disconnect.",
      },
      {
        label: "Two-way plain-text clipboard",
        detail: "Synchronizes UTF-8 text in both directions over low-priority frames so pointer motion stays responsive.",
      },
      {
        label: "Prototype entry point",
        detail: "Runs today via the unify-cli command-line prototype while the Tauri desktop shell and real-Mac hardware validation continue.",
      },
    ],
  },
  earlyAccess: {
    eyebrow: "Project status",
    headline: "Help shape what comes next.",
    body:
      "Unify is in early development. The core libraries are being tested, macOS code compiles for Apple Silicon, and real-hardware validation is still ongoing.",
    primaryCta: "Explore the project",
    secondaryLink: "Read the documentation",
    honestyNote:
      "No prebuilt public release is published yet. Inspect the source repository, review the threat model, or build from Cargo.",
  },
  footer: {
    description:
      "Open-source keyboard, mouse, and plain-text clipboard sharing between macOS and Windows 11 over your local network.",
    github: "GitHub",
    documentation: "Documentation",
    license: "MIT License",
    platforms: "macOS · Windows 11",
    statusNote: "Early development · Source repository",
    languageLabel: "Language",
  },
  errorBoundary: {
    title: "The page encountered a rendering error.",
    retry: "Reload view",
  },
};

export type Messages = typeof en;
