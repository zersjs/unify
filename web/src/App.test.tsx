import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";

describe("Unify landing page", () => {
  beforeEach(() => {
    vi.stubGlobal("matchMedia", (query: string) => ({
      matches: query.includes("prefers-reduced-motion"),
      media: query,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    }));
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("renders the editorial hero, navigation with integrated docs link, and zero SaaS account or fake download controls", () => {
    render(<App />);

    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("One workspace.Two computers.");
    const nav = screen.getByRole("navigation", { name: "Primary navigation" });
    expect(within(nav).getByText("Overview")).toBeTruthy();
    expect(within(nav).getByText("Security")).toBeTruthy();
    expect(within(nav).getByText("Open source")).toBeTruthy();
    const docsNavLink = within(nav).getByRole("link", { name: "Docs" });
    expect(docsNavLink.getAttribute("href")).toBe("/docs/");
    expect(within(nav).getByText("GitHub")).toBeTruthy();

    const readDocsCta = screen.getByRole("link", { name: "Read the documentation" });
    expect(readDocsCta.getAttribute("href")).toBe("/docs/");

    expect(screen.queryByText(/pricing/i)).toBeNull();
    expect(screen.queryByText(/sign up/i)).toBeNull();
    expect(screen.queryByText(/log in/i)).toBeNull();
    expect(screen.queryByText(/download for mac/i)).toBeNull();
    expect(screen.queryByText(/download for windows/i)).toBeNull();
  });

  it("transfers active machine focus across the workspace boundary via controls and keyboard", () => {
    render(<App />);

    expect(screen.getByText("MacBook Pro · macOS")).toBeTruthy();
    const winButton = screen.getByRole("button", { name: "Focus Windows 11" });
    fireEvent.pointerDown(winButton, { button: 0 });
    expect(screen.getByText("Studio PC · Windows 11")).toBeTruthy();

    const stageRegion = screen.getByRole("region", {
      name: /Workspace demonstration showing a MacBook Pro and a Windows 11 desktop monitor/i,
    });
    for (let i = 0; i < 6; i += 1) {
      fireEvent.keyDown(stageRegion, { key: "ArrowLeft" });
    }
    expect(screen.getByText("MacBook Pro · macOS")).toBeTruthy();
  });

  it("documents plain-text-only clipboard scope, verified security mechanisms, and audit status", () => {
    render(<App />);

    expect(screen.getByText(/Image, file, and rich-text clipboard sharing are not currently supported/i)).toBeTruthy();
    expect(screen.getByText(/Unify has not undergone an independent third-party security audit/i)).toBeTruthy();
    expect(screen.getByText("TLS 1.3 with mutual authentication")).toBeTruthy();
    expect(screen.getByText("SPAKE2 password-authenticated key exchange")).toBeTruthy();

    const reverseDir = screen.getByRole("button", { name: "Windows 11 → macOS" });
    fireEvent.pointerDown(reverseDir, { button: 0 });
    expect(screen.getAllByText("cargo test --workspace --locked").length).toBeGreaterThan(0);
  });

  it("lists all seven actual Rust crates and switches verbatim source excerpts", () => {
    render(<App />);

    const expectedCrates = [
      "unify-proto",
      "unify-core",
      "unify-security",
      "unify-net",
      "unify-platform",
      "unify-runtime",
      "unify-cli",
    ];
    for (const crateName of expectedCrates) {
      expect(screen.getByRole("tab", { name: new RegExp(crateName) })).toBeTruthy();
    }

    const securityTab = screen.getByRole("tab", { name: /unify-security/i });
    fireEvent.pointerDown(securityTab, { button: 0 });
    expect(screen.getByText("crates/unify-security/src/pairing.rs")).toBeTruthy();
    expect(screen.getByText(/Spake2::<Ed25519Group>::start_symmetric/)).toBeTruthy();
  });

  it("opens and dismisses the mobile menu with Escape and switches locale + docs links to Turkish cleanly", () => {
    render(<App />);

    const menuToggle = screen.getByRole("button", { name: "Menu" });
    fireEvent.pointerDown(menuToggle, { button: 0 });
    expect(menuToggle.getAttribute("aria-expanded")).toBe("true");

    fireEvent.keyDown(window, { key: "Escape" });
    expect(menuToggle.getAttribute("aria-expanded")).toBe("false");

    const trButton = screen.getByRole("button", { name: "Türkçe" });
    fireEvent.pointerDown(trButton, { button: 0 });
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Tek çalışma alanı.İki bilgisayar.");

    const trReadDocsCta = screen.getByRole("link", { name: "Dokümantasyonu oku" });
    expect(trReadDocsCta.getAttribute("href")).toBe("/docs/tr/");
  });
});
