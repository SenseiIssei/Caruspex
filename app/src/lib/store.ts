// A tiny external store with a selector hook, enough for the few pieces of
// shared state (vehicle, settings, garage, navigation).
import { useCallback, useRef, useSyncExternalStore } from "react";

export interface Store<T> {
  get(): T;
  set(update: Partial<T> | ((state: T) => Partial<T>)): void;
  subscribe(fn: () => void): () => void;
}

export function createStore<T extends object>(initial: T): Store<T> {
  let state = initial;
  const listeners = new Set<() => void>();
  return {
    get: () => state,
    set(update) {
      const patch = typeof update === "function" ? update(state) : update;
      state = { ...state, ...patch };
      for (const fn of [...listeners]) fn();
    },
    subscribe(fn) {
      listeners.add(fn);
      return () => listeners.delete(fn);
    },
  };
}

function shallowEqual(a: unknown, b: unknown): boolean {
  if (Object.is(a, b)) return true;
  if (typeof a !== "object" || typeof b !== "object" || !a || !b) return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  const ka = Object.keys(a);
  const kb = Object.keys(b);
  if (ka.length !== kb.length) return false;
  for (const k of ka) {
    if (!Object.is((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k])) return false;
  }
  return true;
}

// Reads a slice of a store and re-renders only when that slice changes
// (shallow comparison, so selectors may return small objects).
export function useStore<T, S>(store: Store<T>, selector: (state: T) => S): S {
  const cache = useRef<{ value: S } | null>(null);
  const getSnapshot = useCallback(() => {
    const next = selector(store.get());
    if (cache.current && shallowEqual(cache.current.value, next)) return cache.current.value;
    cache.current = { value: next };
    return next;
  }, [store, selector]);
  return useSyncExternalStore(store.subscribe, getSnapshot);
}

// A plain event bus between pages, for the hand-overs MainWindow did with
// signal connections (open the live view and start it, use a generation for
// connecting, open a garage tab ...).
type Handler = (payload: unknown) => void;
const bus = new Map<string, Set<Handler>>();

export function emitApp<T = undefined>(name: string, payload?: T): void {
  for (const fn of [...(bus.get(name) ?? [])]) fn(payload);
}

export function onApp<T = undefined>(name: string, fn: (payload: T) => void): () => void {
  let set = bus.get(name);
  if (!set) {
    set = new Set();
    bus.set(name, set);
  }
  set.add(fn as Handler);
  return () => set.delete(fn as Handler);
}
