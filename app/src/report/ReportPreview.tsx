// The report on screen before it is saved or printed. The Qt version went
// straight to the save dialog (MainWindow::showReport, saveGaragePdf); here
// the page is shown first, as a sheet of paper, with the same two ways out.
import { useEffect, useRef, useState } from "react";
import { call } from "../api";
import { Modal } from "../components/Modal";
import { toast } from "../components/Toast";
import { Button, Spinner } from "../components/ui";
import { t } from "../i18n";
import { useStore } from "../lib/store";
import { closeReport, navStore } from "../shell/nav";
import { findCar, isoDate } from "../state/garage";
import { session } from "../state/session";
import { collectReportExtras, collectReportState, garageHtml, reportHtml } from "./report";
import "./ReportPreview.css";

const R = "caruspex::Report";
const MW = "caruspex::MainWindow";
const GP = "caruspex::GaragePage";
const RP = "caruspex::ReportPreview";

export function ReportPreview() {
  const garageVin = useStore(navStore, (s) => s.garageReportVin);
  const [html, setHtml] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const frame = useRef<HTMLIFrameElement>(null);

  // The document is written once when the preview opens: what the car said
  // up to now, or the file of the chosen garage car.
  useEffect(() => {
    let cancelled = false;
    setHtml(null);
    if (garageVin) {
      const car = findCar(garageVin);
      setHtml(car ? garageHtml(car) : "");
    } else {
      void collectReportState().then((state) => {
        if (!cancelled) setHtml(reportHtml(state, collectReportExtras()));
      });
    }
    return () => {
      cancelled = true;
    };
  }, [garageVin]);

  const title = garageVin ? t(R, "Fahrzeugakte") : t(R, "Prüfbericht");

  const savePdf = async () => {
    if (!html) return;
    setSaving(true);
    try {
      const today = isoDate();
      let path: string | null;
      if (garageVin) {
        path = await call("save_pdf", {
          title: t(R, "Fahrzeugakte speichern"),
          suggestedName: `${garageVin}_Akte_${today}.pdf`,
          html,
        });
        if (path) toast(t(GP, "Akte gespeichert: %1", path), "success");
      } else {
        const vin = session.get().vin;
        path = await call("save_pdf", {
          title: t(R, "Prüfbericht speichern"),
          suggestedName: `${vin || "Caruspex"}_${today}.pdf`,
          html,
        });
        if (path) toast(t(MW, "Prüfbericht gespeichert: %1", path), "success");
      }
    } catch (error) {
      toast(t(MW, "Prüfbericht ließ sich nicht schreiben: %1", String(error)), "error", 6000);
    } finally {
      setSaving(false);
    }
  };

  const print = () => {
    frame.current?.contentWindow?.focus();
    frame.current?.contentWindow?.print();
  };

  return (
    <Modal
      wide
      title={title}
      icon="file"
      onClose={closeReport}
      footer={
        <>
          <Button variant="ghost" onClick={closeReport}>
            {t(RP, "Schließen")}
          </Button>
          <span className="spacer" />
          <Button icon="printer" disabled={!html} onClick={print}>
            {t(RP, "Drucken")}
          </Button>
          <Button variant="primary" icon="download" disabled={!html} busy={saving} onClick={() => void savePdf()}>
            {t(RP, "Als PDF speichern")}
          </Button>
        </>
      }
    >
      <div className="report-sheet">
        {html === null ? (
          <div className="report-loading">
            <Spinner size={26} />
          </div>
        ) : (
          // No scripts run inside; same origin only so the print button can
          // reach the frame's window.
          <iframe ref={frame} className="report-frame fade-in" title={title} sandbox="allow-same-origin allow-modals" srcDoc={html} />
        )}
      </div>
    </Modal>
  );
}
