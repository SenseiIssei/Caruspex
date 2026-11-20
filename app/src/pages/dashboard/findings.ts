// The lines of the findings list on the start page.
import type { IconName } from "../../components/Icon";
import { t } from "../../i18n";
import type { DashboardFinding } from "../../state/session";

const HP = "caruspex::HealthPage";

const TONE_COLOR: Record<DashboardFinding["tone"], string> = {
  bad: "var(--bad)",
  warn: "var(--warn)",
  good: "var(--good)",
  info: "var(--info)",
};

// One line of the findings list, with the exact tint (a critical code has
// its own red) next to the tone the report uses.
export interface ShownFinding extends DashboardFinding {
  tint: string;
}

export interface Shown {
  score: number;
  caption: string;
  subcaption: string;
  findings: ShownFinding[];
  clickableFaults: boolean;
}

// levelIcon / levelColour / levelLabel of HealthPage.cpp.
export function levelIcon(level: number): IconName {
  return level === 0 ? "check-circle" : level === 1 ? "info" : level === 2 ? "alert" : "x-circle";
}
export function levelTone(level: number): DashboardFinding["tone"] {
  return level === 0 ? "good" : level === 1 ? "info" : level === 2 ? "warn" : "bad";
}
export function levelLabel(level: number): string {
  return level === 0 ? t(HP, "In Ordnung") : level === 1 ? t(HP, "Hinweis") : level === 2 ? t(HP, "Bald ansehen") : t(HP, "Dringend");
}

export function finding(icon: string, tone: DashboardFinding["tone"], title: string, text: string, meta = "", tint?: string): ShownFinding {
  return { icon, tone, title, text, meta, tint: tint ?? TONE_COLOR[tone] };
}
