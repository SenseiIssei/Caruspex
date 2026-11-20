// Helpers of the X-ray view: the scene palette from the theme, reduced
// motion, the generation of the screenshot option and model files.
import { useEffect, useState } from "react";
import { call } from "../../api";
import type { VehicleGenerationInfo, ZoneInfo } from "../../api";
import { BodyType, CarZone } from "../../api";
import { cssColor, toCss } from "../../lib/theme";
import type { Vec3 } from "../../xray/layout";
import type { XrayPalette } from "../../xray/scene";
import { externalFiles } from "../../xray/shell";

export function readPalette(): XrayPalette {
  const css = (name: string) => toCss(cssColor(name));
  return {
    light: document.documentElement.dataset.theme === "light",
    accent: css("--accent"),
    sky: css("--sky"),
    good: css("--good"),
    warn: css("--warn"),
    bad: css("--bad"),
    info: css("--info"),
    text: css("--text"),
    muted: css("--text-muted"),
    bus: {
      powertrain: css("--bus-powertrain"),
      comfort: css("--bus-comfort"),
      diagnostic: css("--bus-diagnostic"),
      kline: css("--bus-kline"),
      infotainment: css("--bus-infotainment"),
      mscan: css("--bus-mscan"),
      swcan: css("--bus-swcan"),
    },
  };
}

export function useReducedMotion(): boolean {
  const query = typeof window !== "undefined" ? window.matchMedia("(prefers-reduced-motion: reduce)") : null;
  const [reduced, setReduced] = useState(query?.matches ?? false);
  useEffect(() => {
    if (!query) return;
    const on = () => setReduced(query.matches);
    query.addEventListener("change", on);
    return () => query.removeEventListener("change", on);
  }, [query]);
  return reduced;
}

// Body variants for the hidden screenshot option "xray-generation:<id or
// search>|<variant>".
export const VARIANT_NAMES: Record<string, BodyType> = {
  hatch: BodyType.Hatchback, sedan: BodyType.Sedan, estate: BodyType.Estate, suv: BodyType.Suv, van: BodyType.Van,
  coupe: BodyType.Coupe, cabrio: BodyType.Convertible, pickup: BodyType.Pickup, mpv: BodyType.Mpv, roadster: BodyType.Roadster,
};

// A generation by its catalog id or a search ("Golf VII", "Gran Coupe F36").
export async function findGeneration(text: string): Promise<VehicleGenerationInfo | null> {
  const query = text.trim();
  if (/^\d+$/.test(query)) {
    const d = await call("catalog_diagnosis", { generationId: Number(query), year: 0 }).catch(() => null);
    return d?.generation ?? null;
  }
  const found = await call("catalog_search", { query, limit: 1 }).catch(() => []);
  return found[0] ?? null;
}

// The folder of a path, with its separator.
function folderOf(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i >= 0 ? path.slice(0, i + 1) : "";
}

// The bytes of a model and, for a .gltf, the files it names next to itself.
export async function readModel(path: string): Promise<{ data: ArrayBuffer; files: Map<string, Blob> }> {
  const data = await call("read_model_file", { path });
  const files = new Map<string, Blob>();
  if (/\.gltf$/i.test(path)) {
    const json = JSON.parse(new TextDecoder().decode(data)) as Parameters<typeof externalFiles>[0];
    for (const name of externalFiles(json)) {
      files.set(name, new Blob([await call("read_model_file", { path: folderOf(path) + name })]));
    }
  }
  return { data, files };
}

export interface ZoneFinding {
  zone: CarZone;
  info: ZoneInfo;
  codes: string[];
}

export interface Finding {
  id: string;
  anchor: Vec3;
  title: string;
  text: string;
  zone: CarZone | null;
  ecu: string | null;
  tone: "bad" | "warn" | "good";
}

// The file name of a path, for the own model.
export const fileName = (path: string) => path.replace(/^.*[\\/]/, "");
