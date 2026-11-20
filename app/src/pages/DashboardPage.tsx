// Start page (DashboardPage.cpp): the connected car as a picture, its health
// as a ring with every finding listed beside it, key values as tiles and the
// live gauges.
import { useState } from "react";
import { call } from "../api";
import type { IconName } from "../components/Icon";
import { Icon } from "../components/Icon";
import { ScoreRing } from "../components/ScoreRing";
import { FindingRow } from "../components/Tiles";
import { Button, Card, Page } from "../components/ui";
import { VehicleDiagram } from "../components/VehicleDiagram";
import { t, translator } from "../i18n";
import { useAppEvent } from "../lib/hooks";
import { emitApp } from "../lib/store";
import { navigate, openReport } from "../shell/nav";
import { useProMode, useSession } from "../state/session";
import { ShareDialog } from "../report/ShareDialog";
import { sessionFacts, shareText } from "../report/share";
import { useHealthRating } from "./dashboard/useHealthRating";
import { WhatToDoCard } from "./dashboard/WhatToDoCard";
import { LiveTiles } from "./dashboard/LiveTiles";
import { GaugeCard } from "./dashboard/GaugeCard";
import "./DashboardPage.css";

const tr = translator("caruspex::DashboardPage");

export default function DashboardPage() {
  const connected = useSession((s) => s.connected);
  const busy = useSession((s) => s.busy);
  const identity = useSession((s) => s.identity);
  const adapter = useSession((s) => s.adapter);
  const transport = useSession((s) => s.transport);
  const protocol = useSession((s) => s.protocol);
  const faults = useSession((s) => s.faults);
  const haveRead = useSession((s) => s.haveFaults);
  const status = useSession((s) => s.status);
  const pro = useProMode();
  const brand = identity?.brand ?? "";
  // "Teilen": the summary as a QR code.
  const [sharing, setSharing] = useState(false);
  // --page share: the start page with the QR code open.
  useAppEvent<string>("dev:page", (name) => {
    if (name !== "share") return;
    navigate("start");
    setSharing(true);
  });

  // The score ring and the findings list (dashboard/useHealthRating.ts).
  const shown = useHealthRating({ connected, haveRead, faults, status, brand });

  // --- Hero ---------------------------------------------------------------------------
  let stateTone = "var(--accent)";
  let stateText = tr("Verbunden. Auto prüfen liest Fehler, Bereitschaft und Fahrzeugdaten.");
  if (haveRead) {
    const stored = faults.filter((f) => f.stored).length;
    if (faults.length === 0) {
      stateTone = "var(--good)";
      stateText = tr("Kein Fehler gespeichert.");
    } else {
      stateTone = stored > 0 ? "var(--bad)" : "var(--warn)";
      stateText = faults.length === 1 ? tr("1 Eintrag im Fehlerspeicher.") : tr("%1 Einträge im Fehlerspeicher.", faults.length);
    }
  }
  const subtitle = !connected
    ? tr("Adapter einstecken, Zündung an, verbinden. Oder erst einmal mit dem Demo-Auto ausprobieren.")
    : identity
      ? identity.subtitle
      : tr("%1 über %2, Protokoll %3", adapter, transport, protocol);

  const primary = () => {
    if (!connected) {
      navigate("connect");
      return;
    }
    void call("read_faults");
    void call("read_details");
  };

  return (
    <Page className="dashboard-page">
      <Card className="dash-hero">
        <div className="dash-hero-row">
          <div className="dash-hero-text">
            <div className="eyebrow">{connected ? tr("DEIN FAHRZEUG") : tr("WILLKOMMEN")}</div>
            <h1 className="headline">{connected ? identity?.title || tr("Verbundenes Fahrzeug") : tr("Noch kein Auto verbunden")}</h1>
            {subtitle ? <div className="subtle">{subtitle}</div> : null}
            {connected ? (
              <div className="dash-state fade-in" key={stateText}>
                <span className="dash-state-dot" style={{ background: stateTone }} />
                <span>{stateText}</span>
              </div>
            ) : null}
            <div className="dash-hero-buttons">
              <Button variant="primary" icon={connected ? "sparkles" : "plug"} onClick={primary}>
                {connected ? tr("Auto prüfen") : tr("Verbinden")}
              </Button>
              {pro ? (
                <Button
                  icon="activity"
                  disabled={!connected}
                  onClick={() => {
                    navigate("live");
                    emitApp("live:toggle");
                  }}
                >
                  {tr("Live-Werte")}
                </Button>
              ) : null}
              <Button icon="file" disabled={!connected} title={tr("Alles Gelesene als PDF speichern oder drucken")} onClick={openReport}>
                {tr("Prüfbericht")}
              </Button>
              <Button
                icon="qr"
                disabled={!connected}
                title={t("caruspex::Usability", "Die Zusammenfassung als QR-Code fürs Handy, ohne Konto und ohne Server")}
                onClick={() => setSharing(true)}
              >
                {t("caruspex::Usability", "Teilen")}
              </Button>
            </div>
          </div>
          <button
            type="button"
            className="dash-hero-car"
            title={t("caruspex::XrayPage", "Röntgenansicht öffnen")}
            onClick={() => navigate("xray")}
          >
            <VehicleDiagram body={connected ? (identity?.body ?? 0) : 0} scanning={busy && connected} />
            <span className="dash-hero-xray">
              <Icon name="cube" size={14} />
              {t("caruspex::XrayPage", "In 3D ansehen")}
            </span>
          </button>
        </div>
      </Card>

      <WhatToDoCard connected={connected} />

      <div className="dash-middle">
        <Card className="dash-health">
          <div className="dash-health-row">
            <ScoreRing score={shown.score} caption={shown.caption} subcaption={shown.subcaption} busy={busy && connected} size={220} />
            <div className="dash-findings">
              <h2 className="section-title">{tr("Was Caruspex gefunden hat")}</h2>
              <div className="subtle">{tr("Jeder Punkt Abzug steht hier, mit Grund.")}</div>
              <div className="dash-findings-list">
                {shown.findings.map((f, i) => (
                  <div key={`${f.title}-${i}`} className="fade-in" style={{ animationDelay: `${i * 60}ms` }}>
                    <FindingRow
                      icon={f.icon as IconName}
                      tint={f.tint}
                      title={f.title}
                      text={f.text}
                      meta={f.meta || undefined}
                      onClick={shown.clickableFaults && f.icon === "alert" ? () => navigate("faults") : undefined}
                    />
                  </div>
                ))}
              </div>
            </div>
          </div>
        </Card>
        <LiveTiles />
      </div>

      <GaugeCard connected={connected} />
      {sharing ? <ShareDialog text={shareText(sessionFacts())} onClose={() => setSharing(false)} /> : null}
    </Page>
  );
}
