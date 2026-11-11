// The main window (MainWindow.cpp): title bar, sidebar, the page stack with a
// soft transition, the log drawer, toasts and the report preview. All pages
// stay mounted, the way the Qt stack kept its widgets, so every page keeps
// listening to the backend while another one is on screen.
import { useEffect, useRef, useState } from "react";
import { call, type UpdateInfo } from "../api";
import { Toaster } from "../components/Toast";
import { t, tn } from "../i18n";
import { PageActiveContext } from "../lib/hooks";
import { emitApp, onApp, useStore } from "../lib/store";
import { ReportPreview } from "../report/ReportPreview";
import { setFontScale, setHighContrast } from "../lib/access";
import { appendLog, settingsStore } from "../state/session";
import { openSetup } from "../state/setup";
import { SetupAssistant } from "./SetupAssistant";
import { LogDrawer } from "./LogDrawer";
import { navigate, navStore, useCurrentPage } from "./nav";
import { PAGES, isPageId, type PageId } from "./pages";
import { Sidebar } from "./Sidebar";
import { TitleBar } from "./TitleBar";
import { UpdateDialog } from "./UpdateDialog";

const MW = "caruspex::MainWindow";

function PageHost() {
  const current = useCurrentPage();
  const scroller = useRef<HTMLDivElement>(null);
  return (
    <div className="pages" ref={scroller}>
      {PAGES.map(({ id, component: Component }) => (
        <div key={id} className={`page-slot${id === current ? " active" : ""}`} hidden={id !== current} data-page={id}>
          <PageActiveContext.Provider value={id === current}>
            <Component />
          </PageActiveContext.Provider>
        </div>
      ))}
    </div>
  );
}

export function App({ startPage }: { startPage: string | null }) {
  const [compact, setCompact] = useState(false);
  const [logOpen, setLogOpen] = useState(false);
  const [version, setVersion] = useState("");
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  // The dialog, and whether it installs without asking (only the one
  // opened at start with "Updates automatisch installieren").
  const [updateDialog, setUpdateDialog] = useState<{ autoStart: boolean } | null>(null);
  const reportOpen = useStore(navStore, (s) => s.reportOpen);

  useEffect(() => onApp<PageId>("navigate", (page) => navigate(page)), []);
  useEffect(() => onApp("log:open", () => setLogOpen(true)), []);

  // The first start of a new install opens the assistant; runs from the
  // command line (a page, the demo car) go straight to their page.
  useEffect(() => {
    const automated = startPage !== null || new URLSearchParams(location.search).has("demo");
    if (!settingsStore.get().values.setupDone && !automated) openSetup();
  }, [startPage]);

  // --page setup, setup-socket, setup-ignition, setup-connect: the
  // assistant at one of its steps. --page contrast: the settings in high
  // contrast and larger type, for this run only.
  useEffect(
    () =>
      onApp<string>("dev:page", (name) => {
        if (name === "setup") openSetup();
        else if (name === "setup-socket") openSetup({ step: 1, query: "Golf IV" });
        else if (name === "setup-ignition") openSetup({ step: 2 });
        else if (name === "setup-connect") openSetup({ step: 3 });
        else if (name === "term-hint") {
          // The faults page with the hint of its first word open.
          navigate("faults");
          // A pointer over it: an unfocused window gets no focus events.
          setTimeout(() => {
            const word = document.querySelectorAll<HTMLElement>('[data-page="faults"] .term')[1];
            word?.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
          }, 400);
        } else if (name === "contrast") {
          navigate("settings");
          setHighContrast(true);
          setFontScale(115);
        }
      }),
    [],
  );

  // Start-up of MainWindow: ports, settings, the ready line, auto-connect
  // and the update check.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const info = await call("app_info");
        if (!cancelled) setVersion(info.version);
      } catch {
        setVersion("");
      }
      try {
        const ports = await call("list_serial_ports");
        appendLog(
          ports.length === 0
            ? t(MW, "Kein COM-Anschluss gefunden. Steckt der Adapter am Rechner?")
            : tn(MW, "%n Anschluss/Anschlüsse gefunden.", ports.length),
        );
      } catch {
        // Listing ports is best effort.
      }
      appendLog(t(MW, "Caruspex bereit. Adapter einstecken, Zündung an, dann verbinden."));
      const settings = settingsStore.get().values;
      if (settings.autoConnect && settings.lastPort) emitApp("connect:auto", settings.lastPort);
      if (settings.checkUpdates) {
        try {
          const found = await call("check_for_update");
          if (found && !cancelled) {
            setUpdate(found);
            appendLog(t(MW, "Neue Version verfügbar: %1", found.version));
            // Asks at start; with the setting on it installs right away.
            const latest = settingsStore.get().values;
            setUpdateDialog({ autoStart: latest.autoInstallUpdates && found.canInstall });
          }
        } catch {
          // Offline: say nothing.
        }
      }
      if (startPage) {
        if (isPageId(startPage)) navigate(startPage);
        else emitApp("dev:page", startPage);
        if (startPage === "log") setLogOpen(true);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [startPage]);

  return (
    <div className="window">
      <TitleBar
        version={version}
        update={update}
        onOpenUpdate={() => setUpdateDialog({ autoStart: false })}
        logOpen={logOpen}
        onToggleLog={() => setLogOpen((v) => !v)}
        onToggleSidebar={() => setCompact((v) => !v)}
      />
      <div className="window-body">
        <Sidebar compact={compact} />
        <main className="content">
          <PageHost />
          <LogDrawer open={logOpen} onClose={() => setLogOpen(false)} />
        </main>
      </div>
      <Toaster />
      {update && updateDialog ? (
        <UpdateDialog update={update} autoStart={updateDialog.autoStart} onClose={() => setUpdateDialog(null)} />
      ) : null}
      {reportOpen ? <ReportPreview /> : null}
      <SetupAssistant />
    </div>
  );
}
