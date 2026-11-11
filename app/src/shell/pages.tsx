// Every page of the sidebar, in the order and sections of
// MainWindow::buildSidebar(). The ids are the names of the --page option.
import type { ComponentType } from "react";
import type { IconName } from "../components/Icon";
import { t } from "../i18n";
import DashboardPage from "../pages/DashboardPage";
import ConnectPage from "../pages/ConnectPage";
import HealthPage from "../pages/HealthPage";
import WhatToDoPage from "../pages/WhatToDoPage";
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
  | "todo"
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
  // Only in Profi mode; beginners see the everyday pages.
  pro?: boolean;
  component: ComponentType;
}

export const PAGES: PageEntry[] = [
  { id: "start", icon: "home", label: () => t(MW, "Start"), component: DashboardPage },
  { id: "connect", icon: "plug", label: () => t(MW, "Verbinden"), component: ConnectPage },
  { id: "todo", icon: "lightbulb", label: () => t(MW, "Was tun?"), section: () => t(MW, "Diagnose"), component: WhatToDoPage },
  { id: "health", icon: "heart", label: () => t(MW, "Gesundheitscheck"), component: HealthPage },
  { id: "faults", icon: "alert", label: () => t(MW, "Fehlerspeicher"), component: FaultsPage },
  { id: "xray", icon: "cube", label: () => t(MW, "Röntgenansicht"), component: XrayPage },
  { id: "tools", icon: "shield", label: () => t(MW, "Prüfen und AU"), component: ToolsPage, pro: true },
  { id: "monitor", icon: "stethoscope", label: () => t(MW, "Bauteilprüfung"), component: MonitorPage, pro: true },
  { id: "live", icon: "activity", label: () => t(MW, "Live-Werte"), section: () => t(MW, "Messen"), component: LivePage, pro: true },
  { id: "trip", icon: "fuel", label: () => t(MW, "Bordcomputer"), component: TripPage, pro: true },
  { id: "performance", icon: "timer", label: () => t(MW, "Leistung"), component: PerformancePage, pro: true },
  { id: "battery", icon: "battery", label: () => t(MW, "Batterie"), component: BatteryPage },
  { id: "ev", icon: "bolt-circle", label: () => t(MW, "E-Auto-Akku"), component: EvPage },
  { id: "stats", icon: "chart", label: () => t(MW, "Statistik"), component: StatsPage, pro: true },
  { id: "ecus", icon: "cpu", label: () => t(MW, "Alle Steuergeräte"), section: () => t(MW, "Werkstatt"), component: EcuPage, pro: true },
  { id: "scan", icon: "cable", label: () => t(MW, "Steuergeräte über KKL"), component: ScanPage, pro: true },
  { id: "throttle", icon: "throttle", label: () => t(MW, "Drosselklappe"), component: ThrottlePage, pro: true },
  { id: "kkl", icon: "sliders", label: () => t(MW, "Anpassung und Stellglieder"), component: KklToolsPage, pro: true },
  { id: "vehicles", icon: "car", label: () => t(MW, "Fahrzeuge"), section: () => t(MW, "Wissen"), component: VehiclesPage, pro: true },
  { id: "lexicon", icon: "book", label: () => t(MW, "Fehlercode-Lexikon"), component: LexiconPage, pro: true },
  { id: "garage", icon: "garage", label: () => t(MW, "Garage"), bottom: true, component: GaragePage },
  { id: "adapter", icon: "cable", label: () => t(MW, "Adapter"), bottom: true, component: AdapterPage, pro: true },
  { id: "settings", icon: "settings", label: () => t(MW, "Einstellungen"), bottom: true, component: SettingsPage },
];

export function isPageId(name: string): name is PageId {
  return PAGES.some((p) => p.id === name);
}
