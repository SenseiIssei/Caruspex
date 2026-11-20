// Picks the backend: Tauri inside the app window, the demo car in a plain
// browser (npm run dev), so the whole interface works without the Rust side.
import type { Backend, CommandName, EventName, Events, ResultOf, ArgsOf, Unlisten } from "./contract";
import { createTauriBackend } from "./tauri";
import { createLazyBackend } from "./lazy";

export * from "./types";
export type { Backend, Events, EventName, CommandName } from "./contract";

export const insideTauri: boolean =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// The demo backend carries the fault code table and the PID packs (about half
// a megabyte); the app never uses it, so it is a chunk of its own that only a
// plain browser loads.
export const backend: Backend = insideTauri
  ? createTauriBackend()
  : createLazyBackend("mock", () => import("./mock").then((m) => m.createMockBackend()));

export function call<K extends CommandName>(
  command: K,
  ...args: keyof ArgsOf<K> extends never ? [] : [ArgsOf<K>]
): Promise<ResultOf<K>> {
  return backend.call(command, ...args);
}

export function on<E extends EventName>(event: E, handler: (payload: Events[E]) => void): Unlisten {
  return backend.on(event, handler);
}
