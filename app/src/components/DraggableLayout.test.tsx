import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { I18nProvider } from "@/i18n";
import { DraggableLayout } from "./DraggableLayout";

describe("DraggableLayout", () => {
  beforeEach(() => {
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      left: 0,
      top: 0,
      right: 600,
      bottom: 320,
      width: 600,
      height: 320,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    });
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("moves the peer with arrow keys", () => {
    const onChange = vi.fn();
    render(<DraggableLayout side="Right" localName="MacBook" peerName="Masaüstü PC" onChange={onChange} />);
    const peer = screen.getByRole("button", { name: /Masaüstü PC, to the right of this computer/ });
    fireEvent.keyDown(peer, { key: "ArrowLeft" });
    expect(onChange).toHaveBeenCalledWith("Left");
    fireEvent.keyDown(peer, { key: "ArrowLeft" });
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("drops the peer on the side nearest to where it is released", async () => {
    const onChange = vi.fn();
    render(<DraggableLayout side="Right" localName="MacBook" peerName="PC" onChange={onChange} />);
    const peer = screen.getByRole("button", { name: /PC, to the right of/ });
    fireEvent.pointerDown(peer, { pointerId: 1, button: 0, clientX: 460, clientY: 160 });
    fireEvent.pointerMove(peer, { pointerId: 1, clientX: 300, clientY: 200 });
    fireEvent.pointerMove(peer, { pointerId: 1, clientX: 300, clientY: 270 });
    await new Promise((resolve) => setTimeout(resolve, 150));
    fireEvent.pointerUp(peer, { pointerId: 1, clientX: 300, clientY: 270 });
    expect(onChange).toHaveBeenCalledWith("Bottom");
  });

  it("returns to the committed side when the gesture is cancelled", () => {
    const onChange = vi.fn();
    render(<DraggableLayout side="Right" localName="MacBook" peerName="PC" onChange={onChange} />);
    const peer = screen.getByRole("button", { name: /PC, to the right of/ });
    fireEvent.pointerDown(peer, { pointerId: 1, button: 0, clientX: 460, clientY: 160 });
    fireEvent.pointerMove(peer, { pointerId: 1, clientX: 100, clientY: 160 });
    fireEvent.pointerCancel(peer, { pointerId: 1 });
    expect(onChange).not.toHaveBeenCalled();
  });

  it("describes the position in Turkish when the language is Turkish", () => {
    render(
      <I18nProvider language="tr">
        <DraggableLayout side="Left" localName="MacBook" peerName="PC" onChange={() => undefined} />
      </I18nProvider>,
    );
    expect(screen.getByRole("button", { name: /PC, bu bilgisayarın solunda/ })).toBeTruthy();
  });
});
