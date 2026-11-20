// The optional car of the connection page: the catalog search, the year
// and what the catalog says about connecting to it.
import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";
import { BodyType, call, type VehicleGenerationInfo } from "../../api";
import { translator } from "../../i18n";
import { currentYear, defaultSocketHint } from "./shared";

const tr = translator("caruspex::ConnectPage");

// setTruck: a truck from the catalog switches the J1939 option on.
export function useVehicleChoice(setTruck: (on: boolean) => void) {
  // Vehicle
  const [vehicleText, setVehicleText] = useState("");
  const [suggestions, setSuggestions] = useState<VehicleGenerationInfo[]>([]);
  const [suggestOpen, setSuggestOpen] = useState(false);
  const [suggestIndex, setSuggestIndex] = useState(-1);
  const [generation, setGenerationState] = useState<VehicleGenerationInfo | null>(null);
  const [year, setYear] = useState(2010);
  const [yearRange, setYearRange] = useState<[number, number]>([1980, currentYear() + 1]);
  const [profileNote, setProfileNote] = useState("");
  // The chosen car is an E-series BMW on the K-line (catalog).
  const [bmwKline, setBmwKline] = useState(false);
  const [socketHint, setSocketHint] = useState(defaultSocketHint);

  // Motorcycles: cable, kill switch and Euro 5, from the core.
  const [motoSteps, setMotoSteps] = useState<string[]>([]);

  const searchSeq = useRef(0);

  // --- Vehicle -----------------------------------------------------------------------------

  // Preselects a car from the catalog (or none for automatic).
  const setGeneration = useCallback((g: VehicleGenerationInfo | null, wanted: number) => {
    setGenerationState(g);
    // Trucks speak SAE J1939: the option follows the choice of a truck.
    if (g?.body === BodyType.Truck) setTruck(true);
    if (g) {
      const last = g.yearTo > 0 ? g.yearTo : currentYear();
      setYearRange([g.yearFrom, last]);
      setYear(Math.min(last, Math.max(g.yearFrom, wanted)));
      setVehicleText(g.displayName);
    } else {
      setYearRange([1980, currentYear() + 1]);
    }
  }, []);

  // The profile note and the socket hint follow the vehicle and the year
  // (updateProfileNote).
  useEffect(() => {
    if (!generation) {
      setProfileNote(
        tr(
          "Ohne Auswahl probiert Caruspex die Protokolle der Reihe nach " +
            "durch und erkennt das Auto danach an der Fahrgestellnummer.",
        ),
      );
      setSocketHint(defaultSocketHint());
      setMotoSteps([]);
      setBmwKline(false);
      return;
    }
    let cancelled = false;
    call("catalog_diagnosis", { generationId: generation.id, year })
      .then((info) => {
        if (cancelled || !info) return;
        setProfileNote(info.diagnosisText);
        setBmwKline(info.bmwKlinePossible ?? false);
        setSocketHint(tr("Bei diesem Auto: %1", info.locationText));
        setMotoSteps(info.connectSteps ?? []);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [generation, year]);

  const onVehicleEdited = (value: string) => {
    setVehicleText(value);
    const query = value.trim();
    const seq = ++searchSeq.current;
    if (query.length >= 2) {
      call("catalog_search", { query, limit: 15 })
        .then((found) => {
          if (seq !== searchSeq.current) return;
          setSuggestions(found ?? []);
          setSuggestIndex(-1);
          setSuggestOpen((found ?? []).length > 0);
        })
        .catch(() => undefined);
    } else {
      setSuggestions([]);
      setSuggestOpen(false);
    }
    if (query === "") setGeneration(null, year);
  };

  const activateSuggestion = (g: VehicleGenerationInfo) => {
    const last = g.yearTo > 0 ? g.yearTo : currentYear();
    setSuggestOpen(false);
    setGeneration(g, Math.min(last, Math.max(g.yearFrom, year)));
  };

  const onVehicleKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (!suggestOpen || suggestions.length === 0) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSuggestIndex((i) => (i + 1) % suggestions.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSuggestIndex((i) => (i <= 0 ? suggestions.length - 1 : i - 1));
    } else if (e.key === "Enter" && suggestIndex >= 0) {
      e.preventDefault();
      activateSuggestion(suggestions[suggestIndex]!);
    } else if (e.key === "Escape") {
      setSuggestOpen(false);
    }
  };

  const clearVehicle = () => {
    searchSeq.current++;
    setVehicleText("");
    setSuggestions([]);
    setSuggestOpen(false);
    setGeneration(null, year);
  };

  return {
    vehicleText,
    suggestions,
    suggestOpen,
    setSuggestOpen,
    suggestIndex,
    setSuggestIndex,
    generation,
    year,
    setYear,
    yearRange,
    profileNote,
    bmwKline,
    socketHint,
    motoSteps,
    setGeneration,
    onVehicleEdited,
    activateSuggestion,
    onVehicleKey,
    clearVehicle,
  };
}

export type VehicleChoice = ReturnType<typeof useVehicleChoice>;
