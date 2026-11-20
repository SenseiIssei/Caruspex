// The real backend: Tauri commands and events.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { listenToCommandLine } from "./cli";
import type { ArgsOf, Backend, CommandName, EventName, Events, ResultOf, Unlisten } from "./contract";
import { createEventHub } from "./hub";

export function createTauriBackend(): Backend {
  listenToCommandLine();
  // One Tauri listener per event name, shared by every page that listens:
  // a hundred subscriptions at start would otherwise be a hundred round
  // trips to the backend, and every event would be handed over once per
  // subscriber.
  const hub = createEventHub((event, deliver) => listen(event, (e) => deliver(e.payload)));
  return {
    kind: "tauri",
    call<K extends CommandName>(command: K, ...args: unknown[]): Promise<ResultOf<K>> {
      const payload = (args[0] ?? {}) as ArgsOf<K>;
      return invoke<ResultOf<K>>(command, payload as Record<string, unknown>);
    },
    on<E extends EventName>(event: E, handler: (payload: Events[E]) => void): Unlisten {
      return hub.on(event, handler as (payload: unknown) => void);
    },
  } as Backend;
}
