// The owner's own 3D model in place of the drawn shell.
import { useEffect, useRef, useState, type RefObject } from "react";
import { call } from "../../api";
import type { VehicleGenerationInfo } from "../../api";
import { translator } from "../../i18n";
import { setXrayModel, useGarage } from "../../state/garage";
import type { CarLayout } from "../../xray/layout";
import type { XrayScene } from "../../xray/scene";
import { readModel } from "./support";

const tr = translator("caruspex::XrayPage");

export function useOwnModel({
  vin,
  shown,
  sceneRef,
  mode,
  layout,
}: {
  vin: string;
  shown: VehicleGenerationInfo | null;
  sceneRef: RefObject<XrayScene | null>;
  mode: "idle" | "loading" | "3d" | "2d";
  layout: CarLayout | null;
}) {
  // Kept per car in the garage; a car that is not in the garage keeps it for
  // this session only.
  const garageCar = useGarage((s) => (vin ? (s.cars.find((c) => c.vin === vin) ?? null) : null));
  const [sessionModel, setSessionModel] = useState<{ path: string; turned: boolean } | null | undefined>(undefined);
  useEffect(() => setSessionModel(undefined), [vin]);
  const savedModel = garageCar?.xrayModel ? { path: garageCar.xrayModel, turned: !!garageCar.xrayModelTurned } : null;
  const ownModel = shown ? (sessionModel ?? null) : sessionModel === undefined ? savedModel : sessionModel;
  const [modelState, setModelState] = useState<{ path: string; state: "loading" | "shown" | "error"; error?: string } | null>(null);
  const loadedPath = useRef("");
  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene || mode !== "3d" || !layout) return;
    if (!ownModel) {
      if (loadedPath.current) scene.clearShell();
      loadedPath.current = "";
      setModelState(null);
      return;
    }
    if (loadedPath.current === ownModel.path) {
      scene.turnShell(ownModel.turned);
      return;
    }
    loadedPath.current = ownModel.path;
    setModelState({ path: ownModel.path, state: "loading" });
    void readModel(ownModel.path)
      .then(({ data, files }) => scene.loadShell(data, { turned: ownModel.turned, files }))
      .then(
        () => setModelState({ path: ownModel.path, state: "shown" }),
        (e: unknown) => {
          loadedPath.current = "";
          setModelState({ path: ownModel.path, state: "error", error: e instanceof Error ? e.message : String(e) });
        },
      );
    // layout is only needed to be there; the scene re-places the model itself.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ownModel?.path, ownModel?.turned, mode, layout !== null]);
  const keepModel = (model: { path: string; turned: boolean } | null) => {
    setSessionModel(model);
    if (garageCar && !shown) setXrayModel(garageCar.vin, model?.path ?? "", model?.turned ?? false);
  };
  const pickModel = async () => {
    const path = await call("pick_open_path", {
      title: tr("Eigenes 3D-Modell laden"),
      filters: [{ name: tr("3D-Modell (glTF)"), extensions: ["glb", "gltf"] }],
    }).catch(() => null);
    if (path) keepModel({ path, turned: false });
  };

  return { garageCar, setSessionModel, ownModel, modelState, keepModel, pickModel };
}
