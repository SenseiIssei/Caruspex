// The trips tab: what the trip computer kept of every drive.
import type { ReactNode } from "react";
import type { GarageCar } from "../../api";
import { FindingRow, StatTile } from "../../components/Tiles";
import { localeNumber, translator } from "../../i18n";
import { shortDateTime } from "./shared";

const tr = translator("caruspex::GaragePage");

export function TripsTab({ car }: { car: GarageCar }) {
  let distance = 0;
  let fuel = 0;
  let cost = 0;
  const rows: ReactNode[] = [];
  for (let i = car.trips.length - 1; i >= 0; i--) {
    const trip = car.trips[i]!;
    distance += trip.distanceKm;
    fuel += trip.fuelLitres;
    cost += trip.cost;
    let text = tr("%1 km, %2 l/100 km", localeNumber(trip.distanceKm, 1), trip.averageL100 > 0 ? localeNumber(trip.averageL100, 1) : "--");
    if (trip.cost > 0) text += tr(", %1 €", localeNumber(trip.cost, 2));
    if (trip.maxSpeedKmh > 0) text += tr(", höchstens %1 km/h", localeNumber(trip.maxSpeedKmh, 0));
    rows.push(
      <FindingRow
        key={`${trip.start}-${i}`}
        icon="route"
        tint="var(--accent)"
        title={shortDateTime(trip.start)}
        text={text}
        meta={trip.ecoScore >= 0 ? tr("Fahrstil %1", trip.ecoScore) : undefined}
      />,
    );
  }
  return (
    <div className="garage-tab">
      <div className="garage-tiles">
        <StatTile icon="route" caption={tr("Strecke gesamt")} value={localeNumber(distance, 0)} unit="km" />
        <StatTile icon="fuel" caption={tr("Verbrauch im Schnitt")} value={distance > 1 ? localeNumber((fuel / distance) * 100, 1) : "--"} unit="l/100 km" />
        <StatTile icon="file" caption={tr("Kosten gesamt")} value={cost > 0 ? localeNumber(cost, 2) : "--"} unit="€" />
      </div>
      <div className="garage-rows">
        {rows}
        {car.trips.length === 0 ? (
          <FindingRow
            icon="route"
            tint="var(--info)"
            title={tr("Noch keine Fahrt gespeichert")}
            text={tr("Der Bordcomputer legt hier jede Fahrt ab, sobald sie zurückgesetzt oder die Verbindung getrennt wird.")}
          />
        ) : null}
      </div>
    </div>
  );
}
