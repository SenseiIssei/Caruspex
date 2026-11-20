// The four value tiles of the start page.
import { useState } from "react";
import { StatTile } from "../../components/Tiles";
import { fixed, localeNumber, translator } from "../../i18n";
import { useBackendEvent } from "../../lib/hooks";

const tr = translator("caruspex::DashboardPage");

// Battery, coolant, odometer and tank. Kept in their own component so a live
// value redraws four tiles, not the whole page.
export function LiveTiles() {
  const [values, setValues] = useState<Record<number, number>>({});
  useBackendEvent("live_value", ({ row }) => {
    if (row.pid !== 0x42 && row.pid !== 0x05 && row.pid !== 0xa6 && row.pid !== 0x2f) return;
    setValues((v) => (v[row.pid] === row.value ? v : { ...v, [row.pid]: row.value }));
  });
  useBackendEvent("closed", () => setValues({}));

  const battery = values[0x42];
  const coolant = values[0x05];
  const odometer = values[0xa6];
  const fuel = values[0x2f];
  return (
    <div className="dash-tiles">
      <StatTile
        icon="battery"
        caption={tr("Batterie")}
        value={battery !== undefined ? fixed(battery, 1) : undefined}
        unit={battery !== undefined ? "V" : undefined}
        state={battery === undefined ? undefined : battery < 11.8 ? "var(--bad)" : battery < 12.3 ? "var(--warn)" : "var(--good)"}
      />
      <StatTile
        icon="thermometer"
        caption={tr("Kühlmittel")}
        value={coolant !== undefined ? fixed(coolant, 0) : undefined}
        unit={coolant !== undefined ? "°C" : undefined}
        state={coolant === undefined ? undefined : coolant > 110 ? "var(--bad)" : coolant < 70 ? "var(--info)" : "var(--good)"}
        note={coolant !== undefined && coolant < 70 ? tr("noch nicht warm") : undefined}
      />
      <StatTile
        icon="route"
        caption={tr("Kilometerstand")}
        value={odometer !== undefined ? localeNumber(odometer, 0) : undefined}
        unit={odometer !== undefined ? "km" : undefined}
      />
      <StatTile
        icon="fuel"
        caption={tr("Tank")}
        value={fuel !== undefined ? fixed(fuel, 0) : undefined}
        unit={fuel !== undefined ? "%" : undefined}
        state={fuel === undefined ? undefined : fuel < 12 ? "var(--warn)" : "var(--good)"}
      />
    </div>
  );
}
