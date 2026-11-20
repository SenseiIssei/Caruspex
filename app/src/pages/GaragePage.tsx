// The garage (GaragePage.cpp): every car seen so far, each with its file:
// checks with their score trend, service plan, trips, fuel log and a note.
// The rules live in state/garage.ts (Garage.cpp); this page only shows them.
import { useEffect, useMemo, useState } from "react";
import { call } from "../api";
import { confirm } from "../components/Modal";
import { toast } from "../components/Toast";
import { Button, Card, EmptyState, Page, PageHeader, Tabs } from "../components/ui";
import { VehicleDiagram } from "../components/VehicleDiagram";
import { translator } from "../i18n";
import { useAppEvent, useBackendEvent } from "../lib/hooks";
import { navigate, openGarageReport, openReport } from "../shell/nav";
import { importGarage, isoDate, latestOdometer, removeCar, serialiseGarage, useGarage } from "../state/garage";
import { useSession } from "../state/session";
import { CarCard } from "./garage/CarCard";
import { FuelTab } from "./garage/FuelTab";
import { HistoryTab } from "./garage/HistoryTab";
import { NoteTab } from "./garage/NoteTab";
import { ServicesTab } from "./garage/ServicesTab";
import { TripsTab } from "./garage/TripsTab";
import { cancelText, shortDate } from "./garage/shared";
import "./GaragePage.css";

const tr = translator("caruspex::GaragePage");

// --- Page -------------------------------------------------------------------------------------------

export default function GaragePage() {
  const cars = useGarage((s) => s.cars);
  const sessionVin = useSession((s) => s.vin);
  const identity = useSession((s) => s.identity);
  const [selected, setSelected] = useState("");
  const [current, setCurrent] = useState("");
  const [currentKm, setCurrentKm] = useState(-1);
  const [tab, setTab] = useState(0);

  // The connected car is marked and picked once it is recognised and in the
  // garage (MainWindow called setCurrentVin right after Garage::touch).
  useEffect(() => {
    if (!sessionVin) {
      setCurrent("");
      return;
    }
    if (!identity) return;
    setCurrent(sessionVin);
    setSelected(sessionVin);
  }, [sessionVin, identity]);

  // Odometer of the connected car, for the service plan.
  useBackendEvent("live_value", ({ row }) => {
    if (row.pid === 0xa6) setCurrentKm(row.value);
  });

  useAppEvent<string>("dev:page", (name) => {
    if (!name.startsWith("garage:")) return;
    navigate("garage");
    const index = Number.parseInt(name.slice(7), 10);
    if (index >= 0 && index <= 4) setTab(index);
  });

  // Most recently seen first.
  const sorted = useMemo(() => [...cars].sort((a, b) => (a.lastSeen > b.lastSeen ? -1 : a.lastSeen < b.lastSeen ? 1 : 0)), [cars]);
  const car = cars.find((c) => c.vin === selected) ?? cars[cars.length - 1];
  const odometer = car ? (car.vin === current && currentKm >= 0 ? currentKm : latestOdometer(car.vin)) : -1;

  const exportGarage = async () => {
    try {
      const path = await call("save_text_file", {
        title: tr("Garage sichern"),
        suggestedName: `caruspex-garage-${isoDate()}.json`,
        filters: [{ name: tr("Garage (*.json)"), extensions: ["json"] }],
        content: serialiseGarage(),
      });
      if (path) toast(tr("Garage gesichert: %1", path), "success");
    } catch {
      toast(tr("Sichern hat nicht geklappt."), "error");
    }
  };

  const importFile = async () => {
    let file;
    try {
      file = await call("open_text_file", { title: tr("Sicherung laden"), filters: [{ name: tr("Garage (*.json)"), extensions: ["json"] }] });
    } catch (error) {
      toast(tr("Laden hat nicht geklappt: %1", String(error)), "error");
      return;
    }
    if (!file) return;
    try {
      const count = importGarage(file.content);
      toast(tr.n("%n Auto(s) übernommen, nichts wurde gelöscht.", count), "success");
    } catch (error) {
      toast(tr("Laden hat nicht geklappt: %1", error instanceof Error ? error.message : String(error)), "error");
    }
  };

  const remove = async () => {
    if (!car) return;
    const ok = await confirm({
      title: tr("Entfernen"),
      text: tr("Das Auto und seinen ganzen Verlauf aus der Garage entfernen? Das lässt sich nicht rückgängig machen."),
      confirm: tr("Entfernen"),
      cancel: cancelText(),
      danger: true,
    });
    if (!ok) return;
    const vin = car.vin;
    setSelected("");
    removeCar(vin);
  };

  const meta: string[] = [];
  if (car) {
    if (car.description) meta.push(car.description);
    meta.push(tr("VIN %1", car.vin));
    meta.push(tr("zuerst gesehen %1", shortDate(car.firstSeen)));
  }
  const isCurrent = !!car && car.vin === current;

  return (
    <Page>
      <PageHeader
        eyebrow={tr("Garage")}
        title={tr("Deine Fahrzeuge")}
        subtitle={tr(
          "Jedes Auto, das Caruspex einmal gesehen hat, erkannt an seiner Fahrgestellnummer. Mit allen Prüfungen und Kilometerständen. Bleibt auf diesem Rechner.",
        )}
        actions={
          <>
            <Button variant="ghost" icon="download" onClick={() => void exportGarage()}>
              {tr("Garage sichern")}
            </Button>
            <Button variant="ghost" icon="refresh" onClick={() => void importFile()}>
              {tr("Sicherung laden")}
            </Button>
          </>
        }
      />

      {car ? (
        <div className="garage-columns">
          <div className="garage-list">
            {sorted.map((c) => (
              <CarCard key={c.vin} car={c} selected={c.vin === car.vin} current={c.vin === current} onClick={() => setSelected(c.vin)} />
            ))}
          </div>

          <Card className="garage-detail">
            <div className="garage-head">
              <VehicleDiagram className="garage-picture" body={car.bodyStyle} />
              <div className="garage-head-text">
                <h2 className="page-title selectable">{car.name || tr("Unbekanntes Fahrzeug")}</h2>
                <div className="subtle selectable">{meta.join(" · ")}</div>
                <div className="row wrap-row garage-actions">
                  <Button icon="file" disabled={!isCurrent} title={isCurrent ? undefined : tr("Der Bericht braucht das verbundene Auto.")} onClick={openReport}>
                    {tr("Prüfbericht")}
                  </Button>
                  <Button
                    icon="printer"
                    title={tr("Prüfungen, Wartung, Fahrten und Tankbuch, auch ohne verbundenes Auto")}
                    onClick={() => openGarageReport(car.vin)}
                  >
                    {tr("Akte als PDF")}
                  </Button>
                  <Button variant="ghost" icon="trash" title={tr("Aus der Garage entfernen")} onClick={() => void remove()}>
                    {tr("Entfernen")}
                  </Button>
                </div>
              </div>
            </div>

            <Tabs
              value={tab}
              onChange={setTab}
              tabs={[
                { value: 0, label: tr("Verlauf") },
                { value: 1, label: tr("Wartung") },
                { value: 2, label: tr("Fahrten") },
                { value: 3, label: tr("Tankbuch") },
                { value: 4, label: tr("Notiz") },
              ]}
            />
            {tab === 0 ? <HistoryTab car={car} /> : null}
            {tab === 1 ? <ServicesTab car={car} odometer={odometer} /> : null}
            {tab === 2 ? <TripsTab car={car} /> : null}
            {tab === 3 ? <FuelTab car={car} odometer={odometer} /> : null}
            {tab === 4 ? <NoteTab car={car} /> : null}
          </Card>
        </div>
      ) : (
        <Card className="garage-empty">
          <EmptyState icon="garage" title={tr("Die Garage ist noch leer")}>
            <div className="subtle garage-empty-text">
              {tr(
                "Sobald ein Auto verbunden ist und seine Fahrgestellnummer verrät, steht es hier. Ältere Autos ohne VIN im Steuergerät lassen sich leider nicht wiedererkennen.",
              )}
            </div>
          </EmptyState>
        </Card>
      )}
    </Page>
  );
}
