// Every page of the sidebar, in the order and sections of
// MainWindow::buildSidebar(). The ids are the names of the --page option.
import type { ComponentType } from "react";
import type { IconName } from "../components/Icon";
import { t } from "../i18n";
import DashboardPage from "../pages/DashboardPage";
import ConnectPage from "../pages/ConnectPage";
import HealthPage from "../pages/HealthPage";
import FaultsPage from "../pages/FaultsPage";
import XrayPage from "../pages/XrayPage";
import ToolsPage from "../pages/ToolsPage";
import MonitorPage from "../pages/MonitorPage";
import LivePage from "../pages/LivePage";
import TripPage from "../pages/TripPage";
import PerformancePage from "../pages/PerformancePage";
import BatteryPage from "../pages/BatteryPage";
import EvPage from "../pages/EvPage";
import StatsPage from "../pages/StatsPage";
import EcuPage from "../pages/EcuPage";
import ScanPage from "../pages/ScanPage";
import ThrottlePage from "../pages/ThrottlePage";
import KklToolsPage from "../pages/KklToolsPage";
import VehiclesPage from "../pages/VehiclesPage";
import LexiconPage from "../pages/LexiconPage";
import GaragePage from "../pages/GaragePage";
import AdapterPage from "../pages/AdapterPage";
import SettingsPage from "../pages/SettingsPage";

const MW = "caruspex::MainWindow";

export type PageId =
  | "start"
  | "connect"
  | "health"
  | "faults"
  | "xray"
  | "tools"
  | "monitor"
  | "live"
  | "trip"
  | "performance"
  | "battery"
  | "ev"
  | "stats"
  | "ecus"
  | "scan"
  | "throttle"
  | "kkl"
  | "vehicles"
  | "lexicon"
  | "garage"
  | "adapter"
  | "settings";

export interface PageEntry {
  id: PageId;
  icon: IconName;
  label: () => string;
  // Section heading above the entry, if it starts one.
  section?: () => string;
  // Garage, Adapter and Settings sit below the divider.
  bottom?: boolean;
  component: ComponentType;
}

export const PAGES: PageEntry[] = [
  { id: "start", icon: "home", label: () => t(MW, "Start"), component: DashboardPage },
  { id: "connect", icon: "plug", label: () => t(MW, "Verbinden"), component: ConnectPage },
  { id: "health", icon: "heart", label: () => t(MW, "Gesundheitscheck"), section: () => t(MW, "Diagnose"), component: HealthPage },
  { id: "faults", icon: "alert", label: () => t(MW, "Fehlerspeicher"), component: FaultsPage },
  { id: "xray", icon: "cube", label: () => t(MW, "Röntgenansicht"), component: XrayPage },
  { id: "tools", icon: "shield", label: () => t(MW, "Prüfen und AU"), component: ToolsPage },
  { id: "monitor", icon: "stethoscope", label: () => t(MW, "Bauteilprüfung"), component: MonitorPage },
  { id: "live", icon: "activity", label: () => t(MW, "Live-Werte"), section: () => t(MW, "Messen"), component: LivePage },
  { id: "trip", icon: "fuel", label: () => t(MW, "Bordcomputer"), component: TripPage },
  { id: "performance", icon: "timer", label: () => t(MW, "Leistung"), component: PerformancePage },
  { id: "battery", icon: "battery", label: () => t(MW, "Batterie"), component: BatteryPage },
  { id: "ev", icon: "bolt-circle", label: () => t(MW, "E-Auto-Akku"), component: EvPage },
  { id: "stats", icon: "chart", label: () => t(MW, "Statistik"), component: StatsPage },
  { id: "ecus", icon: "cpu", label: () => t(MW, "Alle Steuergeräte"), section: () => t(MW, "Werkstatt"), component: EcuPage },
  { id: "scan", icon: "cable", label: () => t(MW, "Steuergeräte über KKL"), component: ScanPage },
  { id: "throttle", icon: "throttle", label: () => t(MW, "Drosselklappe"), component: ThrottlePage },
  { id: "kkl", icon: "sliders", label: () => t(MW, "Anpassung und Stellglieder"), component: KklToolsPage },
  { id: "vehicles", icon: "car", label: () => t(MW, "Fahrzeuge"), section: () => t(MW, "Wissen"), component: VehiclesPage },
  { id: "lexicon", icon: "book", label: () => t(MW, "Fehlercode-Lexikon"), component: LexiconPage },
  { id: "garage", icon: "garage", label: () => t(MW, "Garage"), bottom: true, component: GaragePage },
  { id: "adapter", icon: "cable", label: () => t(MW, "Adapter"), bottom: true, component: AdapterPage },
  { id: "settings", icon: "settings", label: () => t(MW, "Einstellungen"), bottom: true, component: SettingsPage },
];

export function isPageId(name: string): name is PageId {
  return PAGES.some((p) => p.id === name);
}
