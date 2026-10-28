import { useEffect, useId, useRef, type ReactNode } from "react";

export function Group({ title, footer, children }: { title?: string; footer?: ReactNode; children: ReactNode }) {
  const headingId = useId();
  return (
    <section className="group" aria-labelledby={title ? headingId : undefined}>
      {title ? (
        <h2 className="group-title" id={headingId}>
          {title}
        </h2>
      ) : null}
      <div className="group-body">{children}</div>
      {footer ? <p className="group-footer">{footer}</p> : null}
    </section>
  );
}

export function Row({ label, detail, children, htmlFor }: { label: ReactNode; detail?: ReactNode; children?: ReactNode; htmlFor?: string }) {
  return (
    <div className="row">
      <div className="row-text">
        {htmlFor ? (
          <label className="row-label" htmlFor={htmlFor}>
            {label}
          </label>
        ) : (
          <span className="row-label">{label}</span>
        )}
        {detail ? <span className="row-detail">{detail}</span> : null}
      </div>
      {children ? <div className="row-control">{children}</div> : null}
    </div>
  );
}

export function Toggle({ checked, onChange, label, disabled }: { checked: boolean; onChange: (value: boolean) => void; label: string; disabled?: boolean }) {
  return (
    <button
      type="button"
      role="switch"
      className="toggle"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    >
      <span className="toggle-thumb" aria-hidden="true" />
    </button>
  );
}

export function ToggleRow(props: { label: string; detail?: ReactNode; checked: boolean; onChange: (value: boolean) => void; disabled?: boolean }) {
  return (
    <Row label={props.label} detail={props.detail}>
      <Toggle checked={props.checked} onChange={props.onChange} label={props.label} disabled={props.disabled} />
    </Row>
  );
}

export function Button({
  children,
  onClick,
  variant = "default",
  disabled,
  type = "button",
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: "default" | "primary" | "destructive";
  disabled?: boolean;
  type?: "button" | "submit";
}) {
  return (
    <button type={type} className={`button button-${variant}`} onClick={onClick} disabled={disabled}>
      {children}
    </button>
  );
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: Array<{ value: T; label: string }>;
  onChange: (value: T) => void;
  label: string;
}) {
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={option.value === value}
          className="segment"
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

export type DotTone = "positive" | "progress" | "neutral" | "warning";

export function StatusDot({ tone }: { tone: DotTone }) {
  return <span className={`dot dot-${tone}`} aria-hidden="true" />;
}

export function Banner({ tone, children, action }: { tone: "warning" | "info"; children: ReactNode; action?: ReactNode }) {
  return (
    <div className={`banner banner-${tone}`} role={tone === "warning" ? "alert" : "status"}>
      <span className="banner-text">{children}</span>
      {action ? <span className="banner-action">{action}</span> : null}
    </div>
  );
}

export function Modal({ title, children, onClose, footer }: { title: string; children: ReactNode; onClose?: () => void; footer?: ReactNode }) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useEffect(() => {
    const dialog = dialogRef.current;
    const previous = document.activeElement as HTMLElement | null;
    if (dialog && !dialog.open) {
      dialog.showModal();
    }
    return () => {
      dialog?.close();
      previous?.focus?.();
    };
  }, []);
  return (
    <dialog
      ref={dialogRef}
      className="modal"
      aria-labelledby={titleId}
      onCancel={(event) => {
        event.preventDefault();
        onClose?.();
      }}
    >
      <h2 className="modal-title" id={titleId}>
        {title}
      </h2>
      <div className="modal-body">{children}</div>
      {footer ? <div className="modal-footer">{footer}</div> : null}
    </dialog>
  );
}

export function InlineError({ message }: { message: string | null }) {
  return (
    <p className="inline-error" role="alert" aria-live="assertive" hidden={!message}>
      {message}
    </p>
  );
}
