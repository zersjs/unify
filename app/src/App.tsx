import { useEffect, useState } from "react";
import type { StatusSnapshot } from "@/bridge";
import { StatusDot } from "@/components/controls";
import { I18nProvider, useI18n } from "@/i18n";
import { useSettingsViewModel } from "@/viewmodels/useSettingsViewModel";
import { useStatusViewModel } from "@/viewmodels/useStatusViewModel";
import { ApprovalDialog, PairingDialog } from "@/views/PairingDialog";
import {
  AboutPage,
  ClipboardPage,
  DevicesPage,
  DiagnosticsPage,
  GeneralPage,
  InputPage,
  LayoutPage,
  NetworkPage,
  SecurityPage,
} from "@/views/SettingsPages";
import { StatusView } from "@/views/StatusView";

type PageKey = "status" | "general" | "devices" | "layout" | "input" | "clipboard" | "network" | "security" | "diagnostics" | "about";

const settingsPages: PageKey[] = ["general", "devices", "layout", "input", "clipboard", "network", "security", "diagnostics", "about"];

function platformOf(): string {
  const agent = navigator.userAgent;
  if (agent.includes("Mac")) return "macos";
  if (agent.includes("Windows")) return "windows";
  return "other";
}

function Shell({ status }: { status: StatusSnapshot }) {
  const { t } = useI18n();
  const [page, setPage] = useState<PageKey>("status");
  const [pairingOpen, setPairingOpen] = useState(false);
  const vm = useSettingsViewModel(status);
  const openPairing = () => setPairingOpen(true);
  const tone = status.connection.state === "connected" ? "positive" : status.connection.state === "not_paired" ? "warning" : "neutral";

  let content;
  switch (page) {
    case "status":
      content = <StatusView status={status} onPair={openPairing} onOpenSecurity={() => setPage("security")} />;
      break;
    case "general":
      content = <GeneralPage status={status} vm={vm} />;
      break;
    case "devices":
      content = <DevicesPage status={status} vm={vm} onPair={openPairing} />;
      break;
    case "layout":
      content = <LayoutPage status={status} vm={vm} />;
      break;
    case "input":
      content = <InputPage status={status} vm={vm} />;
      break;
    case "clipboard":
      content = <ClipboardPage status={status} vm={vm} />;
      break;
    case "network":
      content = <NetworkPage status={status} vm={vm} />;
      break;
    case "security":
      content = <SecurityPage status={status} vm={vm} />;
      break;
    case "diagnostics":
      content = <DiagnosticsPage status={status} vm={vm} />;
      break;
    case "about":
      content = <AboutPage status={status} vm={vm} />;
      break;
  }

  return (
    <div className="shell">
      <nav className="sidebar" aria-label={t.appName}>
        <button type="button" className="nav-item" aria-current={page === "status" ? "page" : undefined} onClick={() => setPage("status")}>
          <StatusDot tone={tone} />
          {t.nav.status}
        </button>
        <p className="nav-heading">{t.nav.settingsGroup}</p>
        {settingsPages.map((key) => (
          <button key={key} type="button" className="nav-item" aria-current={page === key ? "page" : undefined} onClick={() => setPage(key)}>
            {t.nav[key]}
          </button>
        ))}
      </nav>
      <main className="content">{content}</main>
      {pairingOpen && status.pairing?.kind !== "awaiting_approval" ? <PairingDialog status={status} onClose={() => setPairingOpen(false)} /> : null}
      <ApprovalDialog status={status} />
    </div>
  );
}

export function App() {
  const { t, errorMessage } = useI18n();
  const { status, loadError } = useStatusViewModel();
  useEffect(() => {
    document.documentElement.dataset.platform = platformOf();
  }, []);
  if (loadError) {
    return (
      <div className="centered" role="alert">
        {errorMessage(loadError)}
      </div>
    );
  }
  if (!status) {
    return <div className="centered skeleton" aria-busy="true" aria-label={t.appName} />;
  }
  return (
    <I18nProvider language={status.settings.language}>
      <Shell status={status} />
    </I18nProvider>
  );
}
