// Dialogs: a modal frame for page dialogs (adding a service, a fuel entry,
// the report preview) and confirm(), the QMessageBox::question of the Qt
// version, as a promise.
import { useEffect, useSyncExternalStore, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { Icon, type IconName } from "./Icon";
import { Button } from "./ui";

export function Modal({
  title,
  icon,
  children,
  footer,
  onClose,
  small,
  wide,
}: {
  title: ReactNode;
  icon?: IconName;
  children: ReactNode;
  footer?: ReactNode;
  onClose: () => void;
  small?: boolean;
  wide?: boolean;
}) {
  useEffect(() => {
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onClose]);
  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div
        className={`modal${small ? " small" : ""}`}
        style={wide ? { width: "min(1180px, 100%)" } : undefined}
        role="dialog"
        aria-modal
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="modal-head">
          {icon ? <Icon name={icon} size={18} className="muted" /> : null}
          <h2 className="section-title grow">{title}</h2>
          <Button variant="ghost" small icon="x" onClick={onClose} />
        </div>
        <div className="modal-body">{children}</div>
        {footer ? <div className="modal-foot">{footer}</div> : null}
      </div>
    </div>
  );
}

interface ConfirmOptions {
  title: string;
  text: ReactNode;
  confirm: string;
  cancel: string;
  danger?: boolean;
  icon?: IconName;
}

let pending: (ConfirmOptions & { resolve: (ok: boolean) => void }) | null = null;
const listeners = new Set<() => void>();
const notify = () => listeners.forEach((fn) => fn());

// Asks a yes/no question. Resolves true for the confirm button.
export function confirm(options: ConfirmOptions): Promise<boolean> {
  ensureHost();
  return new Promise((resolve) => {
    pending?.resolve(false);
    pending = { ...options, resolve };
    notify();
  });
}

function ConfirmHost() {
  const current = useSyncExternalStore(
    (fn) => {
      listeners.add(fn);
      return () => listeners.delete(fn);
    },
    () => pending,
  );
  if (!current) return null;
  const done = (ok: boolean) => {
    current.resolve(ok);
    pending = null;
    notify();
  };
  return (
    <Modal
      small
      title={current.title}
      icon={current.icon ?? (current.danger ? "alert" : "info")}
      onClose={() => done(false)}
      footer={
        <>
          <Button onClick={() => done(false)}>{current.cancel}</Button>
          <Button variant={current.danger ? "danger" : "primary"} onClick={() => done(true)}>
            {current.confirm}
          </Button>
        </>
      }
    >
      <div className="wrap">{current.text}</div>
    </Modal>
  );
}

let hostReady = false;
function ensureHost(): void {
  if (hostReady) return;
  hostReady = true;
  const el = document.createElement("div");
  document.body.appendChild(el);
  createRoot(el).render(<ConfirmHost />);
}
