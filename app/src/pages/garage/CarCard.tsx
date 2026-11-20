// One car in the list on the left of the garage.
import type { GarageCar } from "../../api";
import { Pill, StatusLine, cx } from "../../components/ui";
import { VehicleDiagram } from "../../components/VehicleDiagram";
import { t, tn } from "../../i18n";
import { Q } from "./shared";

export function CarCard({ car, selected, current, onClick }: { car: GarageCar; selected: boolean; current: boolean; onClick: () => void }) {
  const last = car.checks[car.checks.length - 1];
  const faults = last ? last.codes.length : 0;
  const state = !last ? t(Q, "noch keine Prüfung") : faults === 0 ? t(Q, "zuletzt ohne Fehler") : tn(Q, "zuletzt %n Fehler", faults);
  const tone = !last ? "var(--text-faint)" : faults === 0 ? "var(--good)" : "var(--bad)";
  return (
    <div
      className={cx("garage-car", selected && "on")}
      role="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && onClick()}
    >
      <VehicleDiagram className="garage-car-picture" body={car.bodyStyle} />
      <div className="garage-car-texts">
        <div className="garage-car-name">{car.name || t(Q, "Unbekanntes Fahrzeug")}</div>
        <div className="mono faint garage-car-vin ltr">{car.vin}</div>
        <StatusLine text={<span className="garage-car-state">{state}</span>} tone={tone} />
      </div>
      {current ? <Pill tone="var(--good)">{t(Q, "verbunden")}</Pill> : null}
    </div>
  );
}
