// The side column of the X-ray view: the car with its dimensions, kinds and
// own model, and the list of control units.
import type { CSSProperties, RefObject } from "react";
import type { GarageCar, VehicleAnatomy, VehicleGenerationInfo } from "../../api";
import { BodyType } from "../../api";
import { Icon } from "../../components/Icon";
import { Button, Card, Select, type Option } from "../../components/ui";
import { localeNumber, t, translator } from "../../i18n";
import { navigate } from "../../shell/nav";
import type { SessionState } from "../../state/session";
import { HEAVY_KINDS, heavySpec, type HeavyKind } from "../../xray/heavy";
import type { CarLayout } from "../../xray/layout";
import { MOTO_STYLES, motoSpec, type MotoStyle } from "../../xray/moto";
import { busName, heavyKindName, motoStyleName } from "./names";
import { fileName } from "./support";

const tr = translator("caruspex::XrayPage");

export function XrayCarCard({
  shown,
  connected,
  identity,
  generation,
  anatomy,
  layout,
  variants,
  drawn,
  setVariant,
  setPowertrain,
  isMoto,
  setMotoStyle,
  isHeavy,
  setHeavyKind,
  mode,
  ownModel,
  modelState,
  garageCar,
  keepModel,
  pickModel,
}: {
  shown: VehicleGenerationInfo | null;
  connected: boolean;
  identity: SessionState["identity"];
  generation: VehicleGenerationInfo | null;
  anatomy: VehicleAnatomy | null;
  layout: CarLayout | null;
  variants: BodyType[];
  drawn: BodyType;
  setVariant: (variant: BodyType) => void;
  setPowertrain: (powertrain: number) => void;
  isMoto: boolean;
  setMotoStyle: (style: MotoStyle) => void;
  isHeavy: boolean;
  setHeavyKind: (kind: HeavyKind) => void;
  mode: "idle" | "loading" | "3d" | "2d";
  ownModel: { path: string; turned: boolean } | null;
  modelState: { path: string; state: "loading" | "shown" | "error"; error?: string } | null;
  garageCar: GarageCar | null;
  keepModel: (model: { path: string; turned: boolean } | null) => void;
  pickModel: () => Promise<void>;
}) {
  const title = shown
    ? [shown.brand, shown.model, shown.generation].filter(Boolean).join(" ")
    : connected
      ? identity?.title || t("caruspex::DashboardPage", "Verbundenes Fahrzeug")
      : tr("Beispielauto");
  const facts = anatomy
    ? [anatomy.engine !== 0 ? anatomy.engineName : "", anatomy.drivenName].filter(Boolean).join(" · ")
    : "";
  // Values Wikidata lacks for a measured car are estimates: marked with ≈.
  const metres = (mm: number, bit: number) =>
    `${anatomy && anatomy.dimensionSource !== "typical" && !(anatomy.measuredFields & bit) ? "≈" : ""}${localeNumber(mm / 1000, 2)}`;
  // The dimensions drawn: without any of the generation the typical ones of
  // its class (a Sprinter, a truck), not the catalog's typical van.
  const size = layout?.size ?? anatomy;
  const dims = size
    ? tr(
        "%1 m lang, %2 m breit, %3 m hoch, Radstand %4 m",
        metres(size.lengthMm, 1),
        metres(size.widthMm, 2),
        metres(size.heightMm, 4),
        metres(size.wheelbaseMm, 8),
      )
    : "";
  const source =
    modelState?.state === "shown"
      ? tr("Eigenes Modell")
      : anatomy?.dimensionSource === "manufacturer"
        ? tr("Maße vom Hersteller, Form nachempfunden")
        : anatomy?.dimensionSource === "wikidata"
          ? tr("Maße aus Wikidata, Form nachempfunden")
          : tr("Typische Maße für Aufbau und Baujahr");
  const variantOptions: Option<number>[] = variants.map((b) => ({ value: b, label: anatomy?.bodyNames[b] ?? String(b) }));
  const powertrainOptions: Option<number>[] = (anatomy?.powertrainChoices ?? []).map((c) => ({ value: c.flag, label: c.name }));

  return (
    <Card className="xray-car">
      <div className="eyebrow">{connected ? t("caruspex::DashboardPage", "DEIN FAHRZEUG") : tr("NOCH NICHT VERBUNDEN")}</div>
      <h2 className="section-title">{title}</h2>
      {anatomy ? (
        <>
          <div className="subtle">{[generation?.bodyName, facts].filter(Boolean).join(" · ")}</div>
          <div className="xray-reason">{anatomy.reason}</div>
          <div className="xray-source" data-source={modelState?.state === "shown" ? "model" : anatomy.dimensionSource}>
            <Icon name={modelState?.state === "shown" ? "cube" : anatomy.dimensionSource !== "typical" ? "check-circle" : "info"} size={14} />
            <span>{source}</span>
          </div>
          <div className="hint">
            {dims}
            {anatomy.wikidataId ? ` · Wikidata ${anatomy.wikidataId}` : ""}
          </div>
          {anatomy.dimensionDocument ? <div className="hint xray-document">{tr("Quelle: %1", anatomy.dimensionDocument)}</div> : null}
          <div className="hint">{anatomy.dimensionsText}</div>
          {isMoto ? (
            <>
              <div className="hint">{tr("Gezeichnet wird ein Motorrad dieser Bauart mit diesem Motor, nicht das genaue Modell.")}</div>
              <label className="xray-select">
                <span className="hint">{tr("Bauart")}</span>
                <Select
                  value={layout?.moto?.style ?? motoSpec(anatomy.brand, anatomy.model).style}
                  options={MOTO_STYLES.map((s) => ({ value: s, label: motoStyleName(s) }))}
                  onChange={(v) => setMotoStyle(v as MotoStyle)}
                />
              </label>
            </>
          ) : null}
          {isHeavy ? (
            <>
              <div className="hint">{tr("Gezeichnet wird ein Lkw oder Bus dieser Bauart in typischen Maßen, nicht das genaue Modell.")}</div>
              <label className="xray-select">
                <span className="hint">{tr("Bauart")}</span>
                <Select
                  value={layout?.heavy?.kind ?? heavySpec(anatomy.brand, anatomy.model).kind}
                  options={HEAVY_KINDS.map((k) => ({ value: k, label: heavyKindName(k) }))}
                  onChange={(v) => setHeavyKind(v as HeavyKind)}
                />
              </label>
            </>
          ) : null}
          {variantOptions.length > 1 ? (
            <label className="xray-select">
              <span className="hint">{tr("Karosserie")}</span>
              <Select value={drawn} options={variantOptions} onChange={(v) => setVariant(Number(v) as BodyType)} />
            </label>
          ) : null}
          {powertrainOptions.length > 1 ? (
            <label className="xray-select">
              <span className="hint">{tr("Antrieb")}</span>
              <Select value={anatomy.powertrain} options={powertrainOptions} onChange={(v) => setPowertrain(Number(v))} />
            </label>
          ) : null}
        </>
      ) : null}
      {!connected ? <div className="hint">{tr("Verbinde dich mit deinem Auto oder dem Demo-Auto, dann zeigt das Modell seine Form, seine Steuergeräte und seine Fehler.")}</div> : null}
      {mode === "3d" ? (
        <div className="xray-model">
          <div className="xray-model-buttons">
            <Button small icon="cube" onClick={() => void pickModel()}>
              {tr("Eigenes 3D-Modell laden")}
            </Button>
            {ownModel ? (
              <>
                <Button small icon="refresh" onClick={() => keepModel({ ...ownModel, turned: !ownModel.turned })}>
                  {tr("Modell umdrehen")}
                </Button>
                <Button small icon="x" onClick={() => keepModel(null)}>
                  {tr("Eigenes Modell entfernen")}
                </Button>
              </>
            ) : null}
          </div>
          {ownModel ? (
            <div className="hint">
              {modelState?.state === "loading" ? tr("Das 3D-Modell wird geladen ...") : fileName(ownModel.path)}
              {modelState?.state === "shown" && !garageCar ? ` · ${tr("Nur für diese Sitzung: das Auto ist nicht in der Garage.")}` : ""}
            </div>
          ) : (
            <div className="hint">{tr("Ein glTF- oder GLB-Modell, das dir gehört, ersetzt die erzeugte Hülle; Antrieb, Steuergeräte und Leitungen bleiben.")}</div>
          )}
          {modelState?.state === "error" ? <div className="hint xray-model-error">{tr("Das Modell ließ sich nicht laden: %1", modelState.error ?? "")}</div> : null}
        </div>
      ) : null}
    </Card>
  );
}

export function XrayEcuCard({
  connected,
  scanning,
  layout,
  selectedEcu,
  pickRef,
  udsEcus,
}: {
  connected: boolean;
  scanning: { index: number; total: number; name: string } | null;
  layout: CarLayout | null;
  selectedEcu: string | null;
  pickRef: RefObject<(id: string | null) => void>;
  udsEcus: SessionState["udsEcus"];
}) {
  return (
    <Card className="xray-ecus" title={t("caruspex::MainWindow", "Alle Steuergeräte")} icon="cpu">
      {scanning ? (
        <div className="hint">{t("caruspex::EcuPage", "Frage %1 (%2 von %3)", scanning.name, scanning.index + 1, scanning.total)}</div>
      ) : null}
      {layout && layout.ecus.length > 0 ? (
        <div className="xray-ecu-list">
          {layout.ecus.map((e) => (
            <button
              key={e.key}
              type="button"
              className={`xray-ecu${selectedEcu === e.key ? " selected" : ""}${e.asking ? " asking" : ""}`}
              style={{ "--bus": `var(--bus-${e.bus})` } as CSSProperties}
              onClick={() => pickRef.current(`ecu:${e.key}`)}
            >
              <span className="xray-ecu-dot" data-faulty={e.faulty || undefined} />
              <span className="xray-ecu-name">{e.name}</span>
              <span className="hint">{e.faulty ? tr("Fehler") : e.asking ? tr("fragt ...") : busName(e.bus)}</span>
            </button>
          ))}
        </div>
      ) : (
        <div className="hint">{connected ? tr("Noch nicht gesucht. Steuergeräte suchen zeigt jedes Gerät an seinem Platz.") : tr("Nach dem Verbinden erscheinen hier die Steuergeräte.")}</div>
      )}
      {udsEcus.length > 0 ? (
        <Button small icon="chevron-right" onClick={() => navigate("ecus")}>
          {t("caruspex::MainWindow", "Alle Steuergeräte")}
        </Button>
      ) : null}
    </Card>
  );
}
