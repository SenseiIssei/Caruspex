// The short "what to do" card of the start page.
import type { CSSProperties } from "react";
import type { Urgency } from "../../api";
import { Button, Card } from "../../components/ui";
import { translator } from "../../i18n";
import { navigate } from "../../shell/nav";
import { useAdvice } from "../../state/advice";
import { TIER_TONE, reportTone, tierTitle } from "../WhatToDoPage";

const tw = translator("caruspex::WhatToDoPage");

// "Was soll ich jetzt tun?" in short: the verdict, how many points per step,
// the most urgent one, and the way to the page.
export function WhatToDoCard({ connected }: { connected: boolean }) {
  const report = useAdvice();
  const tone = reportTone(report);
  const first = report && !report.nothingRead ? report.items[0] : undefined;
  return (
    <Card className="dash-todo" style={{ "--tone": tone } as CSSProperties}>
      <div className="dash-todo-row">
        <div className="dash-todo-text">
          <div className="eyebrow">{tw("Was soll ich jetzt tun?")}</div>
          <h2 className="section-title">{report?.headline ?? ""}</h2>
          <div className="subtle dash-todo-first">
            {first ? first.title : (report?.summary ?? "")}
          </div>
        </div>
        {report && !report.nothingRead ? (
          <div className="dash-todo-counts">
            {([0, 1, 2] as Urgency[]).map((u) => (
              <span key={u} className="dash-todo-count" style={{ "--tone": TIER_TONE[u] } as CSSProperties}>
                <strong>{report.items.filter((i) => i.urgency === u).length}</strong>
                {tierTitle(u)}
              </span>
            ))}
          </div>
        ) : null}
        <Button variant={connected ? "primary" : "default"} icon="chevron-right" onClick={() => navigate("todo")}>
          {tw("Ansehen")}
        </Button>
      </div>
    </Card>
  );
}
