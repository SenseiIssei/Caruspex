// The fuel log tab.
import { useState, type ReactNode } from "react";
import type { GarageCar } from "../../api";
import { FindingRow, StatTile } from "../../components/Tiles";
import { Button } from "../../components/ui";
import { localeNumber, translator } from "../../i18n";
import { fuelLogConsumption, removeFuel } from "../../state/garage";
import { FuelDialog } from "./dialogs";
import { shortDate } from "./shared";

const tr = translator("caruspex::GaragePage");

export function FuelTab({ car, odometer }: { car: GarageCar; odometer: number }) {
  const [adding, setAdding] = useState(false);
  let total = 0;
  const rows: ReactNode[] = [];
  for (let i = car.fuel.length - 1; i >= 0; i--) {
    const f = car.fuel[i]!;
    const price = f.litres * f.pricePerLitre;
    total += price;
    let text = tr("%1 l zu %2 €/l", localeNumber(f.litres, 2), localeNumber(f.pricePerLitre, 3));
    if (f.odometerKm >= 0) text += tr(", bei %1 km", localeNumber(f.odometerKm, 0));
    if (!f.full) text += tr(", nicht voll getankt");
    const index = i;
    rows.push(
      <div key={`${f.date}-${f.odometerKm}-${i}`} className="garage-line fade-in">
        <div className="grow">
          <FindingRow icon="fuel" tint="var(--accent)" title={shortDate(f.date)} text={text} meta={price > 0 ? `${localeNumber(price, 2)} €` : undefined} />
        </div>
        <Button variant="ghost" small icon="trash" title={tr("Entfernen")} onClick={() => removeFuel(car.vin, index)} />
      </div>,
    );
  }
  const consumption = fuelLogConsumption(car);
  return (
    <div className="garage-tab">
      <div className="garage-tiles">
        <StatTile
          icon="fuel"
          caption={tr("Verbrauch laut Tankbuch")}
          value={consumption > 0 ? localeNumber(consumption, 1) : "--"}
          unit="l/100 km"
          note={consumption > 0 ? undefined : tr("braucht zwei volle Tankfüllungen")}
        />
        <StatTile icon="file" caption={tr("Getankt gesamt")} value={total > 0 ? localeNumber(total, 2) : "--"} unit="€" />
        <div className="garage-tiles-action">
          <Button icon="fuel" onClick={() => setAdding(true)}>
            {tr("Tankfüllung eintragen")}
          </Button>
        </div>
      </div>
      <div className="garage-rows">
        {rows}
        {car.fuel.length === 0 ? (
          <FindingRow
            icon="fuel"
            tint="var(--info)"
            title={tr("Noch nichts eingetragen")}
            text={tr("Mit zwei vollen Tankfüllungen samt Kilometerstand rechnet Caruspex den echten Verbrauch aus.")}
          />
        ) : null}
      </div>
      {adding ? <FuelDialog vin={car.vin} odometer={odometer} onClose={() => setAdding(false)} /> : null}
    </div>
  );
}
