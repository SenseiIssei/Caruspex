// A backend whose module loads on first use. Subscriptions made before it
// arrives are attached the moment it does, before any call goes through, so
// no event of the first call is missed.
import type { Backend, CommandName, EventName, Events, ResultOf, Unlisten } from "./contract";

export function createLazyBackend(kind: Backend["kind"], load: () => Promise<Backend>): Backend {
  let real: Backend | null = null;
  const waiting = new Set<() => void>();
  let ready: Promise<Backend> | null = null;
  const start = (): Promise<Backend> =>
    (ready ??= load().then((b) => {
      real = b;
      for (const attach of [...waiting]) attach();
      waiting.clear();
      return b;
    }));

  return {
    kind,
    call<K extends CommandName>(command: K, ...args: unknown[]): Promise<ResultOf<K>> {
      if (real) return (real.call as (c: K, ...a: unknown[]) => Promise<ResultOf<K>>)(command, ...args);
      return start().then((b) => (b.call as (c: K, ...a: unknown[]) => Promise<ResultOf<K>>)(command, ...args));
    },
    on<E extends EventName>(event: E, handler: (payload: Events[E]) => void): Unlisten {
      if (real) return real.on(event, handler);
      let stop: Unlisten | null = null;
      let cancelled = false;
      const attach = () => {
        if (!cancelled) stop = real!.on(event, handler);
      };
      waiting.add(attach);
      void start();
      return () => {
        cancelled = true;
        waiting.delete(attach);
        stop?.();
      };
    },
  } as Backend;
}
