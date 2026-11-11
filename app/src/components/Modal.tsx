// Dialogs: a modal frame for page dialogs (adding a service, a fuel entry,
// the report preview) and confirm(), the QMessageBox::question of the Qt
// version, as a promise.
import { useEffect, useId, useRef, useSyncExternalStore, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { t } from "../i18n";
import { Icon, type IconName } from "./Icon";
import { Button } from "./ui";

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), iframe, [tabindex]:not([tabindex="-1"])';

// The elements Tab can reach inside a dialog, in order.
export function focusables(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((el) => !el.hidden && el.getClientRects().length > 0);
}

// Keeps Tab and Shift+Tab inside the dialog (the focus trap).
export function trapTab(root: HTMLElement, e: KeyboardEvent): void {
  const items = focusables(root);
  if (items.length === 0) {
    e.preventDefault();
    root.focus();
    return;
  }
  const first = items[0]!;
  const last = items[items.length - 1]!;
  const active = document.activeElement;
  if (e.shiftKey && (active === first || !root.contains(active))) {
    e.preventDefault();
    last.focus();
  } else if (!e.shiftKey && (active === last || !root.contains(active))) {
    e.preventDefault();
    first.focus();
  }
}

// The last dialog in the document is the one on top.
function isTopmost(el: HTMLElement): boolean {
  const all = document.querySelectorAll('[role="dialog"][aria-modal]');
  return all[all.length - 1] === el;
}

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
  const box = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      // Only the dialog on top answers, when two are open.
      if (box.current && !isTopmost(box.current)) return;
      if (e.key === "Escape") closeRef.current();
      else if (e.key === "Tab" && box.current) trapTab(box.current, e);
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);
  // Focus moves into the dialog and back to where it came from on close,
  // so keyboard users keep their place.
  useEffect(() => {
    const before = document.activeElement as HTMLElement | null;
    const el = box.current;
    if (el && !el.contains(document.activeElement)) {
      const first = focusables(el).find((f) => !f.classList.contains("modal-close")) ?? el;
      first.focus({ preventScroll: true });
    }
    return () => {
      if (before && document.contains(before)) before.focus({ preventScroll: true });
    };
  }, []);
  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div
        ref={box}
        className={`modal${small ? " small" : ""}`}
        style={wide ? { width: "min(1180px, 100%)" } : undefined}
        role="dialog"
        aria-modal
        aria-labelledby={titleId}
        tabIndex={-1}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="modal-head">
          {icon ? <Icon name={icon} size={18} className="muted" /> : null}
          <h2 className="section-title grow" id={titleId}>
            {title}
          </h2>
          <Button variant="ghost" small icon="x" className="modal-close" title={t("caruspex::TitleBar", "Schließen")} onClick={onClose} />
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
