// Short notice in the lower corner of the window, fades out on its own
// (toast::show in Widgets.cpp). Call toast() from anywhere.
import { useSyncExternalStore, type CSSProperties } from "react";
import { Icon, type IconName } from "./Icon";

export type ToastKind = "info" | "success" | "warning" | "error";

interface ToastItem {
  id: number;
  text: string;
  kind: ToastKind;
  leaving: boolean;
}

let items: ToastItem[] = [];
let nextId = 1;
const listeners = new Set<() => void>();
const notify = () => {
  items = [...items];
  for (const fn of listeners) fn();
};

export function toast(text: string, kind: ToastKind = "info", milliseconds = 4200): void {
  const id = nextId++;
  items.push({ id, text, kind, leaving: false });
  // Keep the stack short; the oldest go first.
  while (items.length > 5) items.shift();
  notify();
  setTimeout(() => {
    const item = items.find((t) => t.id === id);
    if (!item) return;
    item.leaving = true;
    notify();
    setTimeout(() => {
      items = items.filter((t) => t.id !== id);
      notify();
    }, 260);
  }, milliseconds);
}

const LOOK: Record<ToastKind, { icon: IconName; tone: string }> = {
  info: { icon: "info", tone: "var(--info)" },
  success: { icon: "check-circle", tone: "var(--good)" },
  warning: { icon: "alert", tone: "var(--warn)" },
  error: { icon: "x-circle", tone: "var(--bad)" },
};

export function Toaster() {
  const list = useSyncExternalStore(
    (fn) => {
      listeners.add(fn);
      return () => listeners.delete(fn);
    },
    () => items,
  );
  return (
    <div className="toaster" aria-live="polite">
      {list.map((t) => (
        <div
          key={t.id}
          className={`toast${t.leaving ? " leaving" : ""}`}
          style={{ "--tone": LOOK[t.kind].tone } as CSSProperties}
          onClick={() => {
            items = items.filter((x) => x.id !== t.id);
            notify();
          }}
        >
          <Icon name={LOOK[t.kind].icon} size={18} className="toast-icon" />
          <span className="toast-text">{t.text}</span>
        </div>
      ))}
    </div>
  );
}
