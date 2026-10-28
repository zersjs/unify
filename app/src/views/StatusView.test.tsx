import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { statusFixture } from "@/test/fixtures";

const invoke = vi.fn<(command: string, args?: unknown) => Promise<unknown>>();

vi.mock("@tauri-apps/api/core", () => ({ invoke: (command: string, args?: unknown) => invoke(command, args) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => undefined) }));

const { StatusView } = await import("./StatusView");
const { I18nProvider } = await import("@/i18n");

describe("StatusView", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
  });

  afterEach(() => cleanup());

  it("shows connection state, measured latency and paired computer", () => {
    render(<StatusView status={statusFixture()} onPair={() => undefined} onOpenSecurity={() => undefined} />);
    expect(screen.getByRole("status").textContent).toContain("Connected");
    expect(screen.getByText("Measured latency 3 ms")).toBeTruthy();
    expect(screen.getAllByText("Masaüstü PC").length).toBeGreaterThan(0);
  });

  it("sends a settings patch when the clipboard switch is toggled", () => {
    render(<StatusView status={statusFixture()} onPair={() => undefined} onOpenSecurity={() => undefined} />);
    fireEvent.click(screen.getByRole("switch", { name: "Clipboard sync" }));
    expect(invoke).toHaveBeenCalledWith("update_settings", { patch: { clipboard_sync: false } });
  });

  it("offers pairing when no peer exists", () => {
    const onPair = vi.fn();
    render(<StatusView status={statusFixture({ peer: null, connection: { ...statusFixture().connection, state: "not_paired" } })} onPair={onPair} onOpenSecurity={() => undefined} />);
    fireEvent.click(screen.getByRole("button", { name: "Pair" }));
    expect(onPair).toHaveBeenCalledOnce();
  });

  it("shows a permission banner when capture needs permission", () => {
    render(<StatusView status={statusFixture({ capture: "permission_required" })} onPair={() => undefined} onOpenSecurity={() => undefined} />);
    expect(screen.getByRole("alert").textContent).toContain("macOS permissions");
  });

  it("offers releasing control while controlling the peer", () => {
    render(<StatusView status={statusFixture({ control: "controlling" })} onPair={() => undefined} onOpenSecurity={() => undefined} />);
    fireEvent.click(screen.getByRole("button", { name: "Take back control" }));
    expect(invoke).toHaveBeenCalledWith("release_control", undefined);
  });

  it("renders Turkish when the language setting is Turkish", () => {
    render(
      <I18nProvider language="tr">
        <StatusView status={statusFixture()} onPair={() => undefined} onOpenSecurity={() => undefined} />
      </I18nProvider>,
    );
    expect(screen.getByRole("status").textContent).toContain("Bağlandı");
    expect(screen.getByRole("switch", { name: "Pano eşitleme" })).toBeTruthy();
    expect(document.documentElement.lang).toBe("tr");
  });
});
