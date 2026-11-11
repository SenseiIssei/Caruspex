// "Was soll ich jetzt tun?": faults, readiness, health check findings and
// live values as one list in three steps (crate caruspex-advice). Each point
// says why, roughly what it costs and what to tell the workshop. Two things
// OBD cannot see (a flashing or a red lamp) the driver ticks here.
import { useState, type CSSProperties } from "react";
import { call } from "../api";
import type { AdviceItem, AdviceReport, Urgency } from "../api";
import { Icon, type IconName } from "../components/Icon";
import { ProHint } from "../components/ProHint";
import { toast } from "../components/Toast";
import { Button, Card, Check, Page, PageHeader, Pill } from "../components/ui";
import { translator } from "../i18n";
import { navigate } from "../shell/nav";
import { useAdvice, workshopNote, type Observations } from "../state/advice";
import { useSession } from "../state/session";
import "./WhatToDoPage.css";

const tr = translator("caruspex::WhatToDoPage");

export const TIER_TONE: Record<Urgency, string> = { 0: "var(--bad)", 1: "var(--warn)", 2: "var(--good)" };
const TIER_ICON: Record<Urgency, IconName> = { 0: "x-circle", 1: "wrench", 2: "clock" };

export function tierTitle(u: Urgency): string {
  return u === 0 ? tr("Sofort anhalten") : u === 1 ? tr("Bald in die Werkstatt") : tr("Kann warten");
}

function tierText(u: Urgency): string {
  return u === 0
    ? tr("Nicht weiterfahren, Motor aus, Hilfe holen.")
    : u === 1
      ? tr("Weiterfahren geht, aber in den nächsten Wochen ansehen lassen.")
      : tr("Bis zur nächsten Inspektion oder es erledigt sich von selbst.");
}

// The tone of the whole list: its most urgent step.
export function reportTone(report: AdviceReport | null): string {
  if (!report || report.nothingRead) return "var(--accent)";
  const worst = report.items[0]?.urgency;
  return worst === undefined ? "var(--good)" : TIER_TONE[worst];
}

async function copy(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    toast(tr("In die Zwischenablage kopiert."), "success");
  } catch {
    toast(tr("Kopieren hat nicht geklappt."), "error");
  }
}

function costIcon(cost: number): IconName {
  return cost === 1 ? "check-circle" : "fuel";
}

function ItemCard({ item, index }: { item: AdviceItem; index: number }) {
  const tone = TIER_TONE[item.urgency];
  return (
    <div className="todo-item fade-in" style={{ "--tone": tone, animationDelay: `${index * 50}ms` } as CSSProperties}>
      <div className="todo-item-head">
        {item.code ? <Pill tone={tone}>{item.code}</Pill> : null}
        <div className="todo-item-title">{item.title}</div>
      </div>
      <div className="todo-item-why">{item.why}</div>
      {item.evidence.length > 0 ? (
        <div className="todo-item-evidence subtle">
          <Icon name="activity" size={14} />
          <span>{tr("Die Messwerte passen dazu: %1", item.evidence.join("; "))}</span>
        </div>
      ) : null}
      <div className="todo-item-cost">
        <Icon name={costIcon(item.cost)} size={14} />
        <span>{item.costText}</span>
      </div>
      <div className="todo-workshop">
        <div className="todo-workshop-head">
          <Icon name="wrench" size={14} />
          <span className="grow">{tr("Das sagst du der Werkstatt")}</span>
          <Button small variant="ghost" icon="file" onClick={() => void copy(item.workshop)}>
            {tr("Kopieren")}
          </Button>
        </div>
        <div className="todo-workshop-text selectable">{item.workshop}</div>
      </div>
      {item.source === "lamp" ? (
        <ProHint text={tr("Alle Steuergeräte lesen, auch Airbag, ABS und Getriebe, kann der Profimodus.")} page="ecus" />
      ) : null}
      {item.source === "readiness" ? (
        <ProHint text={tr("Die Messwerte der Selbsttests zeigt die Bauteilprüfung im Profimodus.")} page="monitor" />
      ) : null}
      {item.page ? (
        <div className="todo-item-actions">
          <Button small variant="ghost" icon="chevron-right" onClick={() => navigate(item.page as "faults")}>
            {item.page === "faults" ? tr("Zum Fehlerspeicher") : item.page === "battery" ? tr("Zur Batterie") : tr("Zum Gesundheitscheck")}
          </Button>
        </div>
      ) : null}
    </div>
  );
}

function Tier({ urgency, items }: { urgency: Urgency; items: AdviceItem[] }) {
  const tone = TIER_TONE[urgency];
  return (
    <Card className="todo-tier" style={{ "--tone": tone } as CSSProperties}>
      <div className="todo-tier-head">
        <div className="todo-tier-icon">
          <Icon name={TIER_ICON[urgency]} size={18} />
        </div>
        <div className="grow">
          <h2 className="section-title">{tierTitle(urgency)}</h2>
          <div className="subtle">{tierText(urgency)}</div>
        </div>
        <span className="todo-tier-count">{items.length}</span>
      </div>
      {items.length === 0 ? (
        <div className="todo-tier-empty subtle">{tr("Nichts in dieser Stufe.")}</div>
      ) : (
        <div className="todo-tier-list">
          {items.map((item, i) => (
            <ItemCard key={`${item.source}-${item.code}-${item.title}`} item={item} index={i} />
          ))}
        </div>
      )}
    </Card>
  );
}

export default function WhatToDoPage() {
  const connected = useSession((s) => s.connected);
  const busy = useSession((s) => s.busy);
  const [observations, setObservations] = useState<Observations>({ milFlashing: false, redWarningLamp: false });
  const report = useAdvice(observations);
  const tone = reportTone(report);

  const check = () => {
    if (!connected) {
      navigate("connect");
      return;
    }
    void call("read_faults");
  };

  return (
    <Page className="todo-page">
      <PageHeader
        eyebrow={tr("DIAGNOSE")}
        title={tr("Was soll ich jetzt tun?")}
        subtitle={tr(
          "Alles, was Caruspex gelesen hat, in drei Stufen: was sofort zählt, was bald in die Werkstatt gehört und was warten kann.",
        )}
      />

      <Card className="todo-hero" style={{ "--tone": tone } as CSSProperties}>
        <div className="todo-hero-row">
          <div className="todo-hero-text">
            <div className="todo-headline">
              <span className="todo-headline-dot" />
              {report?.headline ?? ""}
            </div>
            <div className="subtle">{report?.summary ?? ""}</div>
            <div className="todo-hero-buttons">
              <Button variant="primary" icon={connected ? "sparkles" : "plug"} busy={busy && connected} onClick={check}>
                {connected ? tr("Neu lesen") : tr("Verbinden")}
              </Button>
              <Button
                icon="file"
                disabled={!report || report.items.length === 0}
                onClick={() => report && void copy(workshopNote(report, tr("Für die Werkstatt")))}
              >
                {tr("Zettel für die Werkstatt kopieren")}
              </Button>
            </div>
          </div>
          <div className="todo-observe">
            <div className="todo-observe-title">{tr("Was siehst du im Kombiinstrument?")}</div>
            <div className="hint">{tr("Das kann Caruspex über OBD nicht sehen. Kreuze an, was gerade leuchtet.")}</div>
            <Check checked={observations.milFlashing} onChange={(v) => setObservations((o) => ({ ...o, milFlashing: v }))}>
              {tr("Die gelbe Motorkontrollleuchte blinkt")}
            </Check>
            <Check checked={observations.redWarningLamp} onChange={(v) => setObservations((o) => ({ ...o, redWarningLamp: v }))}>
              {tr("Eine rote Warnlampe leuchtet")}
            </Check>
          </div>
        </div>
      </Card>

      {report && !report.nothingRead ? (
        <div className="todo-tiers">
          {([0, 1, 2] as Urgency[]).map((u) => (
            <Tier key={u} urgency={u} items={report.items.filter((i) => i.urgency === u)} />
          ))}
        </div>
      ) : null}

      <div className="hint todo-footnote">
        {tr(
          "Die Kosten sind grobe Richtwerte für eine Werkstatt, Teile und Arbeit zusammen; je nach Auto und Region kann es mehr oder weniger sein. Caruspex ersetzt keinen Blick in der Werkstatt.",
        )}
      </div>
    </Page>
  );
}
