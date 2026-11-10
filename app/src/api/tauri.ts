// The real backend: Tauri commands and events.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { listenToCommandLine } from "./cli";
import type { ArgsOf, Backend, CommandName, EventName, Events, ResultOf, Unlisten } from "./contract";

export function createTauriBackend(): Backend {
  listenToCommandLine();
  return {
    kind: "tauri",
    call<K extends CommandName>(command: K, ...args: unknown[]): Promise<ResultOf<K>> {
      const payload = (args[0] ?? {}) as ArgsOf<K>;
      return invoke<ResultOf<K>>(command, payload as Record<string, unknown>);
    },
    on<E extends EventName>(event: E, handler: (payload: Events[E]) => void): Unlisten {
      // listen() resolves asynchronously; a caller may unsubscribe before
      // that, so the unlisten is remembered and applied when it arrives.
      let stop: UnlistenFn | null = null;
      let cancelled = false;
      void listen<Events[E]>(event, (e) => handler(e.payload)).then((fn) => {
        if (cancelled) fn();
        else stop = fn;
      });
      return () => {
        cancelled = true;
        if (stop) stop();
      };
    },
  } as Backend;
}
