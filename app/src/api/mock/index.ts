// The demo backend for a plain browser: the car of DemoTransport.cpp behind
// a simulated Worker. Every command of the contract is answered here or in
// one of the area modules; a command nobody answers logs a warning.
import type { Backend, CommandName, EventName, Events } from "../contract";
import { appHandlers } from "./app";
import { knowledgeHandlers } from "./knowledge";
import { emit, subscribe, type Handlers, type PartialHandlers } from "./runtime";
import { sessionHandlers } from "./session";
import { diagnoseHandlers } from "./areas/diagnose";
import { measureHandlers } from "./areas/measure";
import { workshopHandlers } from "./areas/workshop";
import { knowledgeHandlers as catalogHandlers } from "./areas/knowledge";

const handlers: PartialHandlers = {
  ...appHandlers,
  ...knowledgeHandlers,
  ...sessionHandlers,
  ...diagnoseHandlers,
  ...measureHandlers,
  ...workshopHandlers,
  ...catalogHandlers,
};

export function createMockBackend(): Backend {
  return {
    kind: "mock",
    async call(command: CommandName, ...args: unknown[]) {
      const handler = handlers[command] as Handlers[CommandName] | undefined;
      if (!handler) {
        console.warn(`[mock] no demo answer for ${command}`);
        return undefined;
      }
      return await (handler as (a: unknown) => unknown)(args[0] ?? {});
    },
    on<E extends EventName>(event: E, handler: (payload: Events[E]) => void) {
      return subscribe(event, handler);
    },
  } as Backend;
}

// For pages that want to show something happen without the backend, e.g. a
// toast for a log line. Not used by the interface itself.
export { emit };
