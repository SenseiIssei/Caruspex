// The box of the guided check over the 3D view.
import type { Dispatch, SetStateAction } from "react";
import { Icon } from "../../components/Icon";
import { Button } from "../../components/ui";
import { t, translator } from "../../i18n";
import { ACCESS } from "../../lib/access";
import type { Finding } from "./support";

const tr = translator("caruspex::XrayPage");

export function XrayTour({
  tour,
  step,
  steps,
  connected,
  setTour,
  endTour,
}: {
  tour: number | null;
  step: Finding | null;
  steps: Finding[] | null;
  connected: boolean;
  setTour: Dispatch<SetStateAction<number | null>>;
  endTour: () => void;
}) {
  return (
    <div className="xray-tour fade-in" key={step?.id ?? "none"} data-tone={step?.tone ?? "good"}>
      <div className="xray-tour-head">
        <Icon name={step ? "alert" : "check-circle"} size={18} />
        <span className="strong">{step ? step.title : tr("Nichts gefunden")}</span>
        {steps ? <span className="hint">{tr("%1 von %2", (tour ?? 0) + 1, steps.length)}</span> : null}
      </div>
      <div className="subtle">
        {step ? step.text : connected ? tr("Kein Fehler gespeichert und jedes gefundene Steuergerät ist in Ordnung.") : tr("Erst verbinden, dann zeigt die Prüfung, wo am Auto etwas ist.")}
      </div>
      <div className="xray-tour-buttons">
        {steps && steps.length > 1 ? (
          <>
            <Button
              small
              icon="chevron-left"
              title={t(ACCESS, "Zurück")}
              disabled={!tour}
              onClick={() => setTour((i) => Math.max(0, (i ?? 0) - 1))}
            />
            <Button
              small
              icon="chevron-right"
              title={t(ACCESS, "Weiter")}
              disabled={(tour ?? 0) + 1 >= steps.length}
              onClick={() => setTour((i) => Math.min(steps.length - 1, (i ?? 0) + 1))}
            />
          </>
        ) : null}
        <Button small onClick={endTour}>
          {t("caruspex::TitleBar", "Schließen")}
        </Button>
      </div>
    </div>
  );
}
