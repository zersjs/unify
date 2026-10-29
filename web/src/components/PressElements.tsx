import type { AnchorHTMLAttributes, ButtonHTMLAttributes, ReactNode } from "react";
import { usePressFeedback } from "@/viewmodels/usePressFeedback";

interface PressLinkProps extends AnchorHTMLAttributes<HTMLAnchorElement> {
  readonly children: ReactNode;
}

export function PressLink({ children, onPointerDown, onPointerMove, onPointerUp, onPointerCancel, onPointerLeave, onBlur, ...rest }: PressLinkProps) {
  const press = usePressFeedback();
  return (
    <a
      {...rest}
      data-pressed={press.pressed ? "true" : undefined}
      onPointerDown={(event) => {
        press.onPointerDown(event);
        onPointerDown?.(event);
      }}
      onPointerMove={(event) => {
        press.onPointerMove(event);
        onPointerMove?.(event);
      }}
      onPointerUp={(event) => {
        press.onPointerUp();
        onPointerUp?.(event);
      }}
      onPointerCancel={(event) => {
        press.onPointerCancel();
        onPointerCancel?.(event);
      }}
      onPointerLeave={(event) => {
        press.onPointerLeave();
        onPointerLeave?.(event);
      }}
      onBlur={(event) => {
        press.onBlur();
        onBlur?.(event);
      }}
    >
      {children}
    </a>
  );
}

interface PressButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  readonly children: ReactNode;
}

export function PressButton({ children, onPointerDown, onPointerMove, onPointerUp, onPointerCancel, onPointerLeave, onBlur, ...rest }: PressButtonProps) {
  const press = usePressFeedback();
  return (
    <button
      type="button"
      {...rest}
      data-pressed={press.pressed ? "true" : undefined}
      onPointerDown={(event) => {
        press.onPointerDown(event);
        onPointerDown?.(event);
      }}
      onPointerMove={(event) => {
        press.onPointerMove(event);
        onPointerMove?.(event);
      }}
      onPointerUp={(event) => {
        press.onPointerUp();
        onPointerUp?.(event);
      }}
      onPointerCancel={(event) => {
        press.onPointerCancel();
        onPointerCancel?.(event);
      }}
      onPointerLeave={(event) => {
        press.onPointerLeave();
        onPointerLeave?.(event);
      }}
      onBlur={(event) => {
        press.onBlur();
        onBlur?.(event);
      }}
    >
      {children}
    </button>
  );
}
