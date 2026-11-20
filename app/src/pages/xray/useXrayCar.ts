// Which car the X-ray view draws: the connected one, or a generation of the
// screenshot option, with the body, powertrain and kind picked in the view.
import { useCallback, useEffect, useRef, useState } from "react";
import { call } from "../../api";
import type { VehicleAnatomy, VehicleGenerationInfo } from "../../api";
import { BodyType, Powertrain } from "../../api";
import { DEMO_VIN } from "../../lib/vehicleIdentity";
import type { SessionState } from "../../state/session";
import { HEAVY_KINDS, type HeavyKind } from "../../xray/heavy";
import { MOTO_STYLES, type MotoStyle } from "../../xray/moto";
import { VARIANT_NAMES, findGeneration } from "./support";

export function useXrayCar({ connected, identity, vin }: { connected: boolean; identity: SessionState["identity"]; vin: string }) {
  // --- Which car -------------------------------------------------------------------
  // A generation picked by the hidden screenshot option (xray-generation)
  // takes the place of the connected car.
  const [shown, setShown] = useState<VehicleGenerationInfo | null>(null);
  const generation = shown ?? identity?.generation ?? null;
  const demo = vin === DEMO_VIN && !shown;
  // The body of the catalog; variants (an estate of a saloon generation) are
  // drawn on top of it.
  const body = generation ? generation.body : demo ? BodyType.Estate : BodyType.Hatchback;
  const [variant, setVariant] = useState<BodyType | null>(null);
  // The kind of a motorcycle picked in the view, null for the one of the table.
  const [motoStyle, setMotoStyle] = useState<MotoStyle | null>(null);
  // The kind of a truck or bus picked in the view, null for the one of the table.
  const [heavyKind, setHeavyKind] = useState<HeavyKind | null>(null);
  const year = (shown ? shown.yearFrom : identity?.year) || generation?.yearFrom || (connected ? 2012 : 2018);
  const [powertrain, setPowertrain] = useState(0);
  const [anatomy, setAnatomy] = useState<VehicleAnatomy | null>(null);
  const pendingVariant = useRef<{ body: BodyType | null; moto: MotoStyle | null; heavy: HeavyKind | null }>({ body: null, moto: null, heavy: null });
  useEffect(() => {
    setPowertrain(0);
    setVariant(pendingVariant.current.body);
    setMotoStyle(pendingVariant.current.moto);
    setHeavyKind(pendingVariant.current.heavy);
    pendingVariant.current = { body: null, moto: null, heavy: null };
  }, [generation?.id]);
  const [devView, setDevView] = useState<"side" | "rear" | "front" | null>(null);
  // Further switches of the screenshot option after the view: "stats",
  // "explode", "section=x:0.4", "hide=body,cooling", "bare".
  const [devFlags, setDevFlags] = useState<string[]>([]);
  const showGeneration = useCallback((arg: string) => {
    const [query, variantName, view, ...flags] = arg.split("|");
    if (view === "side" || view === "rear" || view === "front") setDevView(view);
    setDevFlags(flags.map((f) => f.trim()).filter(Boolean));
    void findGeneration(query ?? "").then((g) => {
      if (!g) return;
      const name = variantName?.trim().toLowerCase() ?? "";
      pendingVariant.current = {
        body: VARIANT_NAMES[name] ?? null,
        moto: (MOTO_STYLES as string[]).includes(name) ? (name as MotoStyle) : null,
        heavy: (HEAVY_KINDS as string[]).includes(name) ? (name as HeavyKind) : null,
      };
      setShown(g);
    });
  }, []);
  useEffect(() => {
    const arg = new URLSearchParams(window.location.search).get("xray-generation");
    if (arg) showGeneration(arg);
  }, [showGeneration]);
  useEffect(() => {
    let stale = false;
    void call("vehicle_anatomy", {
      generationId: generation?.id ?? -1,
      year,
      body,
      brand: generation?.brand ?? "",
      model: generation?.model ?? "",
      powertrain: generation ? powertrain : Powertrain.Petrol,
    }).then(
      (a) => !stale && setAnatomy(a),
      () => undefined,
    );
    return () => {
      stale = true;
    };
  }, [generation, year, body, powertrain]);

  return {
    shown,
    generation,
    body,
    variant,
    setVariant,
    motoStyle,
    setMotoStyle,
    heavyKind,
    setHeavyKind,
    setPowertrain,
    anatomy,
    devView,
    setDevView,
    devFlags,
    setDevFlags,
    showGeneration,
  };
}
