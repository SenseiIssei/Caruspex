// Small helpers shared by the measure pages (live values, statistics).
import type { FileFilter } from "../api";

// "Torque-CSV (*.csv);;Alle Dateien (*)" as dialog filters.
export function qtFilters(text: string): FileFilter[] {
  return text.split(";;").map((part) => {
    const m = part.match(/^(.*?)\s*\(([^)]*)\)\s*$/);
    const name = m ? m[1]!.trim() || part : part;
    const patterns = m ? m[2]!.split(/\s+/).filter(Boolean) : ["*"];
    return { name, extensions: patterns.map((p) => p.replace(/^\*\.?/, "") || "*") };
  });
}

// QDateTime::toString("yyyy-MM-dd_HH-mm")
export function stamp(date = new Date()): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${p(date.getMonth() + 1)}-${p(date.getDate())}_${p(date.getHours())}-${p(date.getMinutes())}`;
}

