// The service plan tab.
import { useState } from "react";
import type { GarageCar, GarageService } from "../../api";
import type { IconName } from "../../components/Icon";
import { FindingRow } from "../../components/Tiles";
import { Button } from "../../components/ui";
import { localeNumber, translator } from "../../i18n";
import { removeService, serviceStatus } from "../../state/garage";
import { DoneDialog, ServiceDialog } from "./dialogs";
import { shortDate } from "./shared";

const tr = translator("caruspex::GaragePage");

export function ServicesTab({ car, odometer }: { car: GarageCar; odometer: number }) {
  const [dialog, setDialog] = useState<{ kind: "add" } | { kind: "edit"; service: GarageService } | { kind: "done"; service: GarageService } | null>(null);
  const statuses = serviceStatus(car, odometer);
  return (
    <div className="garage-tab">
      <div className="garage-bar">
        <span className="subtle grow">{tr("Was wann fällig ist, nach Kilometer oder Zeit, je nachdem was zuerst kommt.")}</span>
        <Button icon="wrench" onClick={() => setDialog({ kind: "add" })}>
          {tr("Wartung hinzufügen")}
        </Button>
      </div>
      <div className="garage-rows">
        {statuses.map((st) => {
          const s = st.service;
          let tint = "var(--text-faint)";
          let icon: IconName = "clock";
          if (st.state === "due") {
            tint = "var(--bad)";
            icon = "alert";
          } else if (st.state === "soon") {
            tint = "var(--warn)";
          } else if (st.state === "ok") {
            tint = "var(--good)";
            icon = "check-circle";
          }
          const rule: string[] = [];
          if (s.intervalKm > 0) rule.push(tr("alle %1 km", localeNumber(s.intervalKm, 0)));
          if (s.intervalMonths > 0) rule.push(tr.n("alle %n Monat(e)", s.intervalMonths));
          let text = rule.join(tr(" oder "));
          if (s.lastDate || s.lastKm >= 0) {
            const last: string[] = [];
            if (s.lastDate) last.push(shortDate(s.lastDate));
            if (s.lastKm >= 0) last.push(`${localeNumber(s.lastKm, 0)} km`);
            text += `\n${tr("Zuletzt: %1", last.join(", "))}`;
          }
          return (
            <div key={s.id} className="garage-line fade-in">
              <div className="grow">
                <FindingRow
                  icon={icon}
                  tint={tint}
                  title={s.title}
                  text={<span className="wrap">{text.trim()}</span>}
                  meta={st.text}
                  onClick={() => setDialog({ kind: "edit", service: s })}
                />
              </div>
              <Button variant="ghost" small icon="check" onClick={() => setDialog({ kind: "done", service: s })}>
                {tr("Erledigt")}
              </Button>
              <Button variant="ghost" small icon="trash" title={tr("Entfernen")} onClick={() => removeService(car.vin, s.id)} />
            </div>
          );
        })}
        {car.services.length === 0 ? (
          <FindingRow
            icon="wrench"
            tint="var(--info)"
            title={tr("Noch keine Wartung eingetragen")}
            text={tr("Ölwechsel, Inspektion, HU und mehr: einmal eintragen, Caruspex erinnert beim Verbinden.")}
          />
        ) : null}
      </div>
      {dialog?.kind === "add" ? <ServiceDialog vin={car.vin} odometer={odometer} edit={null} onClose={() => setDialog(null)} /> : null}
      {dialog?.kind === "edit" ? <ServiceDialog vin={car.vin} odometer={odometer} edit={dialog.service} onClose={() => setDialog(null)} /> : null}
      {dialog?.kind === "done" ? <DoneDialog vin={car.vin} service={dialog.service} odometer={odometer} onClose={() => setDialog(null)} /> : null}
    </div>
  );
}
