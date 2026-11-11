// Small building blocks shared by every page: the page frame and header,
// cards, buttons, chips, form controls, tabs, status lines and empty states.
// They are thin wrappers over the classes in styles/base.css.
import { useId, type CSSProperties, type ReactNode, type ButtonHTMLAttributes, type SelectHTMLAttributes, type InputHTMLAttributes } from "react";
import { Icon, type IconName } from "./Icon";

const cx = (...parts: (string | false | null | undefined)[]) => parts.filter(Boolean).join(" ");

// --- Page frame ------------------------------------------------------------------

// The scrolling body of a page with the standard margins.
export function Page({ children, className, wide }: { children: ReactNode; className?: string; wide?: boolean }) {
  return (
    <div className={cx("page", wide && "page-wide", className)}>
      <div className="page-inner">{children}</div>
    </div>
  );
}

// Eyebrow, title and an optional subtitle stacked the way every page starts
// (makePageHeader in Widgets.cpp). Actions go to the right.
export function PageHeader({
  eyebrow,
  title,
  subtitle,
  actions,
}: {
  eyebrow?: string;
  title: string;
  subtitle?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <header className="page-header">
      <div className="page-header-text">
        {eyebrow ? <div className="eyebrow">{eyebrow}</div> : null}
        <h1 className="page-title">{title}</h1>
        {subtitle ? <p className="subtle page-subtitle">{subtitle}</p> : null}
      </div>
      {actions ? <div className="page-header-actions">{actions}</div> : null}
    </header>
  );
}

// --- Cards ------------------------------------------------------------------------

export function Card({
  children,
  className,
  flat,
  title,
  icon,
  actions,
  style,
}: {
  children?: ReactNode;
  className?: string;
  flat?: boolean;
  title?: ReactNode;
  icon?: IconName;
  actions?: ReactNode;
  style?: CSSProperties;
}) {
  return (
    <section className={cx("card", flat && "flat", className)} style={style}>
      {title || actions ? (
        <div className="card-head">
          {icon ? <Icon name={icon} size={18} className="muted" /> : null}
          {title ? <h2 className="section-title">{title}</h2> : <span className="spacer" />}
          {actions}
        </div>
      ) : null}
      {children}
    </section>
  );
}

// --- Buttons ------------------------------------------------------------------------

export type ButtonVariant = "default" | "primary" | "danger" | "ghost";

export function Button({
  variant = "default",
  icon,
  small,
  children,
  className,
  busy,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  icon?: IconName;
  small?: boolean;
  busy?: boolean;
}) {
  // A button with only an icon is named by its tooltip for screen readers.
  const label = rest["aria-label"] ?? (!children && typeof rest.title === "string" ? rest.title : undefined);
  return (
    <button
      type="button"
      className={cx("btn", variant !== "default" && variant, small && "small", !children && "icon-only", className)}
      {...rest}
      aria-label={label}
      aria-busy={busy || undefined}
    >
      {busy ? <Spinner size={14} /> : icon ? <Icon name={icon} size={small ? 14 : 16} /> : null}
      {children}
    </button>
  );
}

export function Chip({
  on,
  children,
  onClick,
  title,
  disabled,
}: {
  on?: boolean;
  children: ReactNode;
  onClick?: () => void;
  title?: string;
  disabled?: boolean;
}) {
  return (
    <button type="button" className={cx("chip", on && "on")} aria-pressed={!!on} onClick={onClick} title={title} disabled={disabled}>
      {children}
    </button>
  );
}

// A small coloured label; tone is a CSS colour, usually a var(--...).
export function Pill({ tone, children, title }: { tone?: string; children: ReactNode; title?: string }) {
  return (
    <span className="pill" style={tone ? ({ "--tone": tone } as CSSProperties) : undefined} title={title}>
      {children}
    </span>
  );
}

// --- Status -----------------------------------------------------------------------------

export function Dot({ tone, glow }: { tone?: string; glow?: boolean }) {
  return <span className={cx("dot", glow && "glow")} style={tone ? ({ "--tone": tone } as CSSProperties) : undefined} />;
}

// Severity or state colour as a small dot with text, e.g. "5 bereit".
export function StatusLine({ text, tone }: { text: ReactNode; tone?: string }) {
  return (
    <span className="status-line">
      <Dot tone={tone} />
      {text}
    </span>
  );
}

export function Note({ tone, title, children, icon }: { tone?: string; title?: ReactNode; children?: ReactNode; icon?: IconName }) {
  return (
    <div className="note" style={tone ? ({ "--tone": tone } as CSSProperties) : undefined}>
      <div className="row top">
        {icon ? <Icon name={icon} size={18} style={{ color: tone ?? "var(--info)", flex: "none", marginTop: 1 }} /> : null}
        <div className="grow">
          {title ? <div className="note-title">{title}</div> : null}
          {children ? <div className="wrap">{children}</div> : null}
        </div>
      </div>
    </div>
  );
}

export function EmptyState({
  icon = "info",
  title,
  text,
  children,
}: {
  icon?: IconName;
  title?: ReactNode;
  text?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <div className="empty fade-in">
      <div className="empty-icon">
        <Icon name={icon} size={26} />
      </div>
      {title ? <div className="empty-title">{title}</div> : null}
      {text ? <div className="empty-text">{text}</div> : null}
      {children}
    </div>
  );
}

// --- Progress and spinner ------------------------------------------------------------------

export function Progress({ value, indeterminate }: { value?: number; indeterminate?: boolean }) {
  const pct = Math.max(0, Math.min(100, value ?? 0));
  return (
    <div className={cx("progress", indeterminate && "indeterminate")} role="progressbar" aria-valuenow={indeterminate ? undefined : pct}>
      <span style={indeterminate ? undefined : { width: `${pct}%` }} />
    </div>
  );
}

// Arc with the gradient that turns while something runs.
export function Spinner({ size = 18 }: { size?: number }) {
  const w = Math.max(2, size * 0.12);
  const r = size / 2 - w;
  const c = 2 * Math.PI * r;
  const id = useId();
  return (
    <svg className="spinner" width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden>
      <defs>
        <linearGradient id={id} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="var(--sky)" />
          <stop offset="1" stopColor="var(--accent)" />
        </linearGradient>
      </defs>
      <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--border)" strokeWidth={w} />
      <circle
        cx={size / 2}
        cy={size / 2}
        r={r}
        fill="none"
        stroke={`url(#${id})`}
        strokeWidth={w}
        strokeLinecap="round"
        strokeDasharray={`${c * 0.3} ${c}`}
      />
    </svg>
  );
}

// --- Form controls ---------------------------------------------------------------------------

export function Check({
  checked,
  onChange,
  children,
  disabled,
  title,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  children?: ReactNode;
  disabled?: boolean;
  title?: string;
}) {
  return (
    <label className="check" title={title}>
      <input type="checkbox" checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} />
      {children ? <span>{children}</span> : null}
    </label>
  );
}

export interface Option<T extends string | number> {
  value: T;
  label: string;
}

export function Select<T extends string | number>({
  value,
  options,
  onChange,
  className,
  ...rest
}: Omit<SelectHTMLAttributes<HTMLSelectElement>, "value" | "onChange"> & {
  value: T;
  options: readonly Option<T>[];
  onChange: (value: T) => void;
}) {
  return (
    <select
      className={cx("select", className)}
      value={String(value)}
      onChange={(e) => {
        const hit = options.find((o) => String(o.value) === e.target.value);
        if (hit) onChange(hit.value);
      }}
      {...rest}
    >
      {options.map((o) => (
        <option key={String(o.value)} value={String(o.value)}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

export function TextInput({
  value,
  onChange,
  className,
  ...rest
}: Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "onChange"> & {
  value: string;
  onChange: (value: string) => void;
}) {
  return <input className={cx("input", className)} value={value} onChange={(e) => onChange(e.target.value)} {...rest} />;
}

// Number field like QSpinBox: clamps to min and max on blur.
export function NumberInput({
  value,
  onChange,
  min,
  max,
  step = 1,
  suffix,
  className,
  disabled,
  decimals = 0,
}: {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  suffix?: string;
  className?: string;
  disabled?: boolean;
  decimals?: number;
}) {
  const clamp = (v: number) => Math.min(max ?? Infinity, Math.max(min ?? -Infinity, v));
  return (
    <span className={cx("number-input", className)}>
      <input
        className="input num"
        type="number"
        value={Number.isFinite(value) ? Number(value.toFixed(decimals)) : ""}
        min={min}
        max={max}
        step={step}
        disabled={disabled}
        onChange={(e) => {
          const v = Number(e.target.value);
          if (!Number.isNaN(v) && e.target.value !== "") onChange(v);
        }}
        onBlur={(e) => {
          const v = Number(e.target.value);
          onChange(clamp(Number.isNaN(v) ? (min ?? 0) : v));
        }}
      />
      {suffix ? <span className="number-suffix faint">{suffix}</span> : null}
    </span>
  );
}

export function Tabs<T extends string | number>({
  value,
  tabs,
  onChange,
}: {
  value: T;
  tabs: readonly { value: T; label: ReactNode }[];
  onChange: (value: T) => void;
}) {
  return (
    <div className="tabs" role="tablist">
      {tabs.map((tab) => (
        <button
          key={String(tab.value)}
          type="button"
          role="tab"
          aria-selected={tab.value === value}
          className={cx("tab", tab.value === value && "on")}
          onClick={() => onChange(tab.value)}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}

// A segmented choice of a few options, like a row of checkable chips.
export function Segmented<T extends string | number>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: readonly { value: T; label: ReactNode; icon?: IconName }[];
  onChange: (value: T) => void;
}) {
  return (
    <div className="segmented" role="radiogroup">
      {options.map((o) => (
        <button
          key={String(o.value)}
          type="button"
          role="radio"
          aria-checked={o.value === value}
          className={cx("segment", o.value === value && "on")}
          onClick={() => onChange(o.value)}
        >
          {o.icon ? <Icon name={o.icon} size={16} /> : null}
          {o.label}
        </button>
      ))}
    </div>
  );
}

export { cx };
