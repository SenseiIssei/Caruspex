// The rating of the start page: the score ring and the findings beside it.
import { useEffect, useRef, useState } from "react";
import type { HealthFindingRow } from "../../api";
import { translator } from "../../i18n";
import { useAppEvent, useBackendEvent } from "../../lib/hooks";
import { recordCheck, session, type SessionState } from "../../state/session";
import { describeFault, severityColor, severityTone } from "../FaultsPage";
import { finding, levelIcon, levelLabel, levelTone, type Shown, type ShownFinding } from "./findings";

const tr = translator("caruspex::DashboardPage");

export function useHealthRating({
  connected,
  haveRead,
  faults,
  status,
  brand,
}: {
  connected: boolean;
  haveRead: boolean;
  faults: SessionState["faults"];
  status: SessionState["status"];
  brand: string;
}): Shown {
  // The result of a full health check replaces the page's own rating until
  // the connection closes.
  const [external, setExternal] = useState<Shown | null>(null);
  const [shown, setShown] = useState<Shown>({ score: -1, caption: "", subcaption: "", findings: [], clickableFaults: false });

  // showFindings(): what the ring and the list show, and what the report and
  // the garage get from the start page.
  const show = (next: Shown) => {
    setShown(next);
    session.set({
      score: next.score,
      scoreCaption: next.caption,
      findings: next.findings.map(({ icon, tone, title, text, meta }) => ({ icon, tone, title, text, meta })),
    });
    if (next.score >= 0) recordCheck();
  };
  const showRef = useRef(show);
  showRef.current = show;

  useAppEvent<{ score: number; caption: string; findings: HealthFindingRow[] }>("dashboard:health", ({ score, caption, findings }) => {
    setExternal({
      score,
      caption,
      subcaption: tr("Gesundheit"),
      findings: findings.map((f) => finding(levelIcon(f.level), levelTone(f.level), f.title, f.explanation, levelLabel(f.level))),
      clickableFaults: false,
    });
  });

  useBackendEvent("closed", () => setExternal(null));

  // rebuildFindings(): a simple rating until the full health check reports.
  useEffect(() => {
    if (external) {
      showRef.current(external);
      return;
    }
    if (!connected) {
      showRef.current({
        score: -1,
        caption: tr("Noch offen"),
        subcaption: tr("Gesundheit"),
        findings: [
          finding("info", "info", tr("Noch nichts geprüft"), tr("Nach dem Verbinden steht hier, was mit dem Auto los ist, in einfachen Worten.")),
        ],
        clickableFaults: false,
      });
      return;
    }
    if (!haveRead) {
      showRef.current({
        score: -1,
        caption: tr("Bereit"),
        subcaption: tr("Auto prüfen drücken"),
        findings: [
          finding(
            "sparkles",
            "info",
            tr("Bereit zur Prüfung"),
            tr("Auto prüfen liest den Fehlerspeicher, die Bereitschaftstests und die Fahrzeugdaten."),
            "",
            "var(--accent)",
          ),
        ],
        clickableFaults: false,
      });
      return;
    }

    let cancelled = false;
    void Promise.all(faults.map((f) => describeFault(f.code, brand))).then((described) => {
      if (cancelled) return;
      // What is stored weighs most, then the lamp, then the open readiness
      // tests.
      let score = 100;
      const list: ShownFinding[] = [];
      const weight = [3, 6, 12, 20, 30];
      faults.forEach((f, i) => {
        const { info, where } = described[i]!;
        // Weight by how urgent the code is; a pending code counts half.
        let minus = weight[info.severity] ?? 12;
        if (!f.stored) minus = Math.floor((minus + 1) / 2);
        score -= minus;
        list.push(
          finding(
            "alert",
            severityTone(info.severity),
            `${f.code}  ${info.known ? info.text : f.text}`,
            `${info.severityLabel} · ${f.kind} · ${where.name}`,
            `−${minus}`,
            severityColor(info.severity),
          ),
        );
      });
      const mil = status?.mil ?? false;
      const incomplete = status ? status.monitorReady.filter((ready) => !ready).length : 0;
      if (status && mil) {
        score -= 12;
        list.push(
          finding(
            "engine",
            "bad",
            tr("Motorkontrollleuchte an"),
            tr("Das Steuergerät meldet einen abgasrelevanten Fehler. Bald in die Werkstatt oder selbst nachsehen."),
            "−12",
          ),
        );
      }
      if (status && incomplete > 0) {
        const minus = Math.min(9, incomplete * 3);
        score -= minus;
        list.push(
          finding(
            "clock",
            "info",
            incomplete === 1 ? tr("1 Bereitschaftstest noch offen") : tr("%1 Bereitschaftstests noch offen", incomplete),
            tr("Nach dem Löschen oder einem Batteriewechsel normal. Für die AU sollten sie abgeschlossen sein."),
            `−${minus}`,
          ),
        );
      }
      if (list.length === 0) {
        list.push(finding("check-circle", "good", tr("Alles in Ordnung"), tr("Kein Fehler gespeichert, die Lampe ist aus.")));
      }
      score = Math.max(0, Math.min(100, score));
      const caption = score >= 85 ? tr("Gut in Schuss") : score >= 60 ? tr("Braucht Pflege") : tr("Bitte ansehen");
      showRef.current({ score, caption, subcaption: tr("Gesundheit"), findings: list, clickableFaults: true });
    });
    return () => {
      cancelled = true;
    };
  }, [external, connected, haveRead, faults, status, brand]);

  return shown;
}
