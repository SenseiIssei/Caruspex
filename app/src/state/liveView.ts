// Live readings for display. A component that shows readings renders at
// most once per animation frame, and not at all while its page is hidden
// (every page stays mounted); it catches up when the page comes back.
// Whatever needs every single reading (charts, recordings, timers) keeps
// listening to "live_values" itself.
import { useCallback, useRef, useSyncExternalStore } from "react";
import type { LiveRow } from "../api";
import { usePageActive } from "../lib/hooks";
import { live, liveFrames } from "./session";

const idle = () => undefined;

// What `pick` takes from each row; the component renders again only when one
// of the picked parts changes.
export function useLive<T>(keys: readonly number[], pick: (row: LiveRow | undefined) => T): readonly T[] {
  const active = usePageActive();
  const id = keys.join(",");
  const keysRef = useRef(keys);
  keysRef.current = keys;
  const pickRef = useRef(pick);
  pickRef.current = pick;
  const cache = useRef<T[]>([]);
  const subscribe = useCallback(
    (fn: () => void) => (active ? liveFrames.subscribe(keysRef.current, fn) : idle),
    // The keys by value: a new array with the same keys keeps the subscription.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [active, id],
  );
  const snapshot = () => {
    const now = keysRef.current;
    const before = cache.current;
    const next = now.map((k) => pickRef.current(live.get(k)));
    if (before.length === next.length && next.every((v, i) => Object.is(v, before[i]))) return before;
    cache.current = next;
    return next;
  };
  return useSyncExternalStore(subscribe, snapshot);
}

const wholeRow = (row: LiveRow | undefined) => row;
const valueOf = (row: LiveRow | undefined) => row?.value;

export function useLiveRows(keys: readonly number[]): readonly (LiveRow | undefined)[] {
  return useLive(keys, wholeRow);
}

// Only the numbers: the same value again does not render.
export function useLiveValues(keys: readonly number[]): readonly (number | undefined)[] {
  return useLive(keys, valueOf);
}

export function useLiveRow(key: number): LiveRow | undefined {
  return useLive([key], wholeRow)[0];
}
