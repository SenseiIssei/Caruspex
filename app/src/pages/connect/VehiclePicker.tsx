// The optional car of the connection page: the catalog search, the year,
// the note on how it is diagnosed and the way to the BMW page.
import { Icon } from "../../components/Icon";
import { Button, cx } from "../../components/ui";
import { t, translator } from "../../i18n";
import { ACCESS } from "../../lib/access";
import type { Mode } from "./shared";
import type { VehicleChoice } from "./useVehicleChoice";

const tr = translator("caruspex::ConnectPage");
const tbmw = translator("caruspex::BmwPage");

export function VehiclePicker({ vehicle, mode, setMode }: { vehicle: VehicleChoice; mode: Mode; setMode: (m: Mode) => void }) {
  const {
    vehicleText,
    suggestions,
    suggestOpen,
    setSuggestOpen,
    suggestIndex,
    setSuggestIndex,
    year,
    setYear,
    yearRange,
    profileNote,
    bmwKline,
    onVehicleEdited,
    onVehicleKey,
    activateSuggestion,
    clearVehicle,
  } = vehicle;
  return (
    <>
      <div className="connect-vehicle-title">
        <h2 className="section-title">{tr("Fahrzeug")}</h2>
        <span className="hint">{tr("freiwillig")}</span>
      </div>
      <div className="connect-line connect-vehicle-row">
        <div className="connect-search grow">
          <input
            className="input connect-field"
            value={vehicleText}
            placeholder={tr("Automatisch erkennen, oder suchen: Golf 4, A2, E46 ...")}
            onChange={(e) => onVehicleEdited(e.target.value)}
            onKeyDown={onVehicleKey}
            onFocus={() => suggestions.length > 0 && setSuggestOpen(true)}
            onBlur={() => setTimeout(() => setSuggestOpen(false), 150)}
            role="combobox"
            aria-expanded={suggestOpen}
            aria-autocomplete="list"
          />
          {vehicleText ? (
            <button type="button" className="connect-clear" onClick={clearVehicle} tabIndex={-1} aria-label={t(ACCESS, "Eingabe leeren")}>
              <Icon name="x" size={14} />
            </button>
          ) : null}
          {suggestOpen && suggestions.length > 0 ? (
            <ul className="connect-suggestions fade-in" role="listbox">
              {suggestions.slice(0, 15).map((g, i) => (
                <li
                  key={g.id}
                  role="option"
                  aria-selected={i === suggestIndex}
                  className={cx(i === suggestIndex && "on")}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    activateSuggestion(g);
                  }}
                  onMouseEnter={() => setSuggestIndex(i)}
                >
                  <span>{g.displayName}</span>
                  <span className="faint">({g.yearsText})</span>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
        <label className="connect-year input">
          <span className="faint">{tr("Baujahr ")}</span>
          <input
            type="number"
            className="num"
            value={year}
            min={yearRange[0]}
            max={yearRange[1]}
            onChange={(e) => {
              const v = Number(e.target.value);
              if (e.target.value !== "" && Number.isFinite(v)) setYear(v);
            }}
            onBlur={() => setYear((v) => Math.min(yearRange[1], Math.max(yearRange[0], v)))}
          />
        </label>
      </div>
      <div className="hint">{profileNote}</div>
      {bmwKline && mode !== "bmw" ? (
        <Button variant="ghost" icon="car" className="connect-bmw" onClick={() => setMode("bmw")}>
          {tbmw("Alle Steuergeräte über K+DCAN oder KKL")}
        </Button>
      ) : null}
    </>
  );
}
