// The first-start assistant: which adapter, where the socket is in this
// car, ignition on, connect. Opens by itself on the first start of a new
// install, can be skipped at every step, and comes back from the settings
// and the connect page.
import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { call, type VehicleGenerationInfo } from "../api";
import { Icon } from "../components/Icon";
import { Modal } from "../components/Modal";
import { AdapterArt, IgnitionArt, SocketSketch, type AdapterKind } from "../components/SetupArt";
import { Button, Note, cx } from "../components/ui";
import { t, translator } from "../i18n";
import { ACCESS } from "../lib/access";
import { emitApp, useStore } from "../lib/store";
import { setProMode } from "../state/session";
import { closeSetup, setupStore } from "../state/setup";
import { navigate } from "./nav";
import "./SetupAssistant.css";

const tu = translator("caruspex::Usability");

// What ConnectPage does with the result (app event "connect:setup").
export interface SetupChoice {
  mode: "cable" | "ble" | "wifi" | "enet" | "demo";
  generation: VehicleGenerationInfo | null;
  year: number;
  connect: boolean;
}

const KINDS: { kind: AdapterKind; title: () => string; text: () => string }[] = [
  {
    kind: "usb",
    title: () => tu("ELM327 mit USB-Kabel"),
    text: () => tu("Stecker mit Kabel zum USB-Anschluss. Am zuverlässigsten."),
  },
  {
    kind: "bluetooth",
    title: () => tu("ELM327 mit Bluetooth"),
    text: () => tu("Kleiner Stecker ohne Kabel, oft mit Knopf oder Lämpchen."),
  },
  {
    kind: "wifi",
    title: () => tu("ELM327 mit WLAN"),
    text: () => tu("Kleiner Stecker, der ein eigenes WLAN öffnet."),
  },
  {
    kind: "enet",
    title: () => tu("ENET-Kabel"),
    text: () => tu("OBD-Stecker auf Netzwerkkabel, für BMW und Mini ab etwa 2008."),
  },
  {
    kind: "kkl",
    title: () => tu("KKL-Kabel"),
    text: () => tu("USB-Kabel ohne ELM327, für ältere VW, Audi, Seat und Skoda."),
  },
];

const MODE_OF: Record<AdapterKind, SetupChoice["mode"]> = {
  usb: "cable",
  bluetooth: "ble",
  wifi: "wifi",
  enet: "enet",
  kkl: "cable",
};

function currentYear(): number {
  return new Date().getFullYear();
}

export function SetupAssistant() {
  const open = useStore(setupStore, (s) => s.open);
  if (!open) return null;
  return <SetupDialog />;
}

function SetupDialog() {
  const [step, setStep] = useState(0);
  const [kind, setKind] = useState<AdapterKind>("usb");
  // The car: search, the chosen generation and year, what the catalog says.
  const [query, setQuery] = useState("");
  const [found, setFound] = useState<VehicleGenerationInfo[]>([]);
  const [generation, setGeneration] = useState<VehicleGenerationInfo | null>(null);
  const [year, setYear] = useState(0);
  const [location, setLocation] = useState(0);
  const [locationText, setLocationText] = useState("");
  const seq = useRef(0);
  const steps = [tu("Adapter"), tu("Buchse"), tu("Zündung"), tu("Verbinden")];

  // --page setup-socket: the second step with a car chosen.
  useEffect(() => {
    const wanted = setupStore.get().preset;
    if (!wanted) return;
    setStep(wanted.step);
    if (wanted.query) search(wanted.query, true);
    // Runs once, for the preset the dialog opened with.
  }, []);

  const search = (text: string, pickFirst = false) => {
    setQuery(text);
    const q = text.trim();
    const mine = ++seq.current;
    if (q.length < 2) {
      setFound([]);
      return;
    }
    call("catalog_search", { query: q, limit: 6 })
      .then((list) => {
        if (mine !== seq.current) return;
        setFound(list ?? []);
        if (pickFirst && list && list[0]) pick(list[0]);
      })
      .catch(() => undefined);
  };

  const pick = (g: VehicleGenerationInfo) => {
    const last = g.yearTo > 0 ? g.yearTo : currentYear();
    setGeneration(g);
    setYear(Math.round((g.yearFrom + last) / 2));
    setFound([]);
    setQuery(g.displayName);
  };

  // The spot of the socket follows the car and the year.
  useEffect(() => {
    if (!generation) {
      setLocation(0);
      setLocationText(
        tu("Meist unter dem Armaturenbrett auf der Fahrerseite, manchmal hinter einer Abdeckung oder in der Mittelkonsole."),
      );
      return;
    }
    let cancelled = false;
    call("catalog_diagnosis", { generationId: generation.id, year })
      .then((info) => {
        if (cancelled || !info) return;
        setLocation(info.location ?? 0);
        setLocationText(info.locationText);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [generation, year]);

  const finish = (choice: Partial<SetupChoice>) => {
    closeSetup();
    if (kind === "kkl" && choice.mode !== "demo") {
      // The KKL cable talks to the control units directly, on its own page.
      setProMode(true);
      navigate("scan");
      return;
    }
    const full: SetupChoice = { mode: MODE_OF[kind], generation, year, connect: false, ...choice };
    navigate("connect");
    emitApp("connect:setup", full);
  };

  const onKindKey = (e: KeyboardEvent<HTMLDivElement>) => {
    const keys = ["ArrowRight", "ArrowDown", "ArrowLeft", "ArrowUp"];
    if (!keys.includes(e.key)) return;
    e.preventDefault();
    const at = KINDS.findIndex((k) => k.kind === kind);
    const next = (at + (e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : -1) + KINDS.length) % KINDS.length;
    setKind(KINDS[next]!.kind);
    e.currentTarget.querySelectorAll<HTMLButtonElement>("[role=radio]")[next]?.focus();
  };

  const chosenKind = KINDS.find((k) => k.kind === kind)!;
  const last = steps.length - 1;

  return (
    <Modal
      title={tu("Einrichtung")}
      icon="sparkles"
      onClose={closeSetup}
      footer={
        <>
          <Button variant="ghost" onClick={closeSetup}>
            {tu("Überspringen")}
          </Button>
          <span className="spacer" />
          {step > 0 ? (
            <Button icon="chevron-left" onClick={() => setStep((s) => s - 1)}>
              {t(ACCESS, "Zurück")}
            </Button>
          ) : null}
          {step < last ? (
            <Button variant="primary" onClick={() => setStep((s) => s + 1)}>
              {t(ACCESS, "Weiter")}
            </Button>
          ) : (
            <Button variant="primary" icon={kind === "kkl" ? "cable" : "plug"} onClick={() => finish({ connect: true })}>
              {kind === "kkl" ? tu("Zur KKL-Seite") : tu("Jetzt verbinden")}
            </Button>
          )}
        </>
      }
    >
      <ol className="setup-steps" aria-label={tu("Schritte")}>
        {steps.map((name, i) => (
          <li key={name} className={cx("setup-step", i === step && "on", i < step && "done")} aria-current={i === step ? "step" : undefined}>
            <span className="setup-step-dot">{i < step ? <Icon name="check" size={13} /> : i + 1}</span>
            <span className="setup-step-name">{name}</span>
          </li>
        ))}
      </ol>

      <div className="setup-page fade-in" key={step}>
        {step === 0 ? (
          <>
            <h3 className="setup-title">{tu("Welchen Adapter hast du?")}</h3>
            <div className="setup-kinds" role="radiogroup" aria-label={tu("Welchen Adapter hast du?")} onKeyDown={onKindKey}>
              {KINDS.map((k) => (
                <button
                  key={k.kind}
                  type="button"
                  role="radio"
                  aria-checked={k.kind === kind}
                  tabIndex={k.kind === kind ? 0 : -1}
                  className={cx("setup-kind", k.kind === kind && "on")}
                  onClick={() => setKind(k.kind)}
                >
                  <AdapterArt kind={k.kind} />
                  <span className="setup-kind-title">{k.title()}</span>
                  <span className="setup-kind-text">{k.text()}</span>
                </button>
              ))}
            </div>
            <div className="hint">
              {tu("Unsicher? Auf dem Stecker steht meist ELM327 oder OBD-II. Hat er kein Kabel, ist es Bluetooth oder WLAN.")}
            </div>
          </>
        ) : null}

        {step === 1 ? (
          <div className="setup-socket">
            <div className="setup-socket-text">
              <h3 className="setup-title">{tu("Wo ist die Buchse?")}</h3>
              <div className="subtle">{tu("Such dein Auto, dann zeigt die Skizze die Stelle. Ohne Auswahl siehst du die üblichen Stellen.")}</div>
              <div className="setup-search">
                <input
                  className="input"
                  value={query}
                  onChange={(e) => search(e.target.value)}
                  placeholder={tu("Zum Beispiel Golf 4, A2, E46 ...")}
                  aria-label={tu("Auto suchen")}
                />
                {found.length > 0 ? (
                  <ul className="setup-found" aria-label={tu("Auto suchen")}>
                    {found.map((g) => (
                      <li key={g.id}>
                        <button type="button" onClick={() => pick(g)}>
                          <span>{g.displayName}</span>
                          <span className="faint">({g.yearsText})</span>
                        </button>
                      </li>
                    ))}
                  </ul>
                ) : null}
              </div>
              {generation ? (
                <label className="setup-year">
                  <span className="faint">{tu("Baujahr")}</span>
                  <input
                    type="number"
                    className="input num"
                    value={year}
                    min={generation.yearFrom}
                    max={generation.yearTo > 0 ? generation.yearTo : currentYear()}
                    onChange={(e) => {
                      const v = Number(e.target.value);
                      if (e.target.value !== "" && Number.isFinite(v)) setYear(v);
                    }}
                  />
                </label>
              ) : null}
              <Note icon="map-pin" tone="var(--accent)" title={generation ? generation.displayName : tu("Die üblichen Stellen")}>
                {locationText}
              </Note>
              <div className="hint">{tu("Bei Autos mit dem Lenkrad rechts sitzt die Buchse meist spiegelbildlich.")}</div>
            </div>
            <SocketSketch location={location} label={locationText} />
          </div>
        ) : null}

        {step === 2 ? (
          <div className="setup-ignition">
            <IgnitionArt />
            <div>
              <h3 className="setup-title">{tu("Adapter einstecken, Zündung an")}</h3>
              <ul className="setup-list">
                <li>{tu("Den Adapter fest in die Buchse stecken, bis er nicht mehr wackelt.")}</li>
                <li>{tu("Zündung einschalten, bis die Lampen im Kombiinstrument leuchten. Bei einem Startknopf: drücken, ohne auf die Bremse zu treten.")}</li>
                <li>{tu("Den Motor nicht starten. Für Live-Werte darf er laufen, aber nur im Freien.")}</li>
              </ul>
            </div>
          </div>
        ) : null}

        {step === 3 ? (
          <div className="setup-done">
            <h3 className="setup-title">{tu("Bereit zum Verbinden")}</h3>
            <div className="setup-summary">
              <AdapterArt kind={kind} />
              <div>
                <div className="strong">{chosenKind.title()}</div>
                <div className="subtle">{generation ? `${generation.displayName}, ${year}` : tu("Auto wird beim Verbinden erkannt")}</div>
              </div>
            </div>
            {kind === "kkl" ? (
              <Note icon="info" tone="var(--info)">
                {tu("Das KKL-Kabel spricht direkt mit den Steuergeräten. Das geht auf der Seite Steuergeräte über KKL, die mit dem Profimodus kommt; der wird dafür eingeschaltet.")}
              </Note>
            ) : (
              <div className="subtle">
                {tu("Caruspex sucht den Adapter und findet selbst heraus, welches Protokoll das Auto spricht. Beim ersten Mal dauert das ein paar Sekunden.")}
              </div>
            )}
            <Button variant="ghost" icon="sparkles" onClick={() => finish({ mode: "demo", connect: true })}>
              {tu("Lieber erst mit dem Demo-Auto ausprobieren")}
            </Button>
          </div>
        ) : null}
      </div>
    </Modal>
  );
}
