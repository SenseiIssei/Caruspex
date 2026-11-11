// Hooks for pages: backend events, the visible state of the page, timers.
import { createContext, useContext, useEffect, useRef } from "react";
import { on, type EventName, type Events } from "../api";
import { onApp } from "./store";

// Subscribes to a backend event for the life of the component. The handler
// may change between renders; the newest one is always called.
export function useBackendEvent<E extends EventName>(event: E, handler: (payload: Events[E]) => void): void {
  const ref = useRef(handler);
  ref.current = handler;
  useEffect(() => on(event, (payload) => ref.current(payload)), [event]);
}

// Same for the events pages send each other (see lib/store.ts).
export function useAppEvent<T>(name: string, handler: (payload: T) => void): void {
  const ref = useRef(handler);
  ref.current = handler;
  useEffect(() => onApp<T>(name, (payload) => ref.current(payload)), [name]);
}

// Every page stays mounted, like the widgets of the Qt stack, so it keeps
// listening while hidden. This tells it whether it is the one on screen, for
// work that only matters while visible (animations, charts).
export const PageActiveContext = createContext(true);

export function usePageActive(): boolean {
  return useContext(PageActiveContext);
}

// setInterval that follows the newest callback; null pauses it.
export function useInterval(callback: () => void, ms: number | null): void {
  const ref = useRef(callback);
  ref.current = callback;
  useEffect(() => {
    if (ms === null) return;
    const id = setInterval(() => ref.current(), ms);
    return () => clearInterval(id);
  }, [ms]);
}
