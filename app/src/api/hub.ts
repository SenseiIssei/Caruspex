// Shares one subscription per event name between many handlers. The
// subscription is made with the first handler and dropped with the last.
// listen() answers asynchronously, so a channel that empties and fills again
// before the old subscription is gone would deliver twice; every
// subscription carries a token and only the newest one delivers.
type Handler = (payload: unknown) => void;
type Subscribe = (event: string, deliver: Handler) => Promise<() => void>;

interface Channel {
  handlers: Set<Handler>;
  token: object | null;
}

export interface EventHub {
  on(event: string, handler: Handler): () => void;
  // Handlers per event name, for tests and the performance notes.
  count(event: string): number;
}

export function createEventHub(subscribe: Subscribe): EventHub {
  const channels = new Map<string, Channel>();
  const closers = new WeakMap<object, () => void>();

  const open = (event: string, channel: Channel) => {
    const token = {};
    channel.token = token;
    void subscribe(event, (payload) => {
      if (channel.token !== token) return;
      // One failing page must not keep the event from the others, as with
      // separate listeners.
      for (const fn of [...channel.handlers]) {
        try {
          fn(payload);
        } catch (error) {
          console.error(error);
        }
      }
    }).then(
      // Closed again before the subscription arrived: stop it right away.
      (stop) => (channel.token === token ? closers.set(token, stop) : stop()),
      () => undefined,
    );
  };

  return {
    on(event, handler) {
      let channel = channels.get(event);
      if (!channel) {
        channel = { handlers: new Set(), token: null };
        channels.set(event, channel);
      }
      const ch = channel;
      // The same function twice gets its own entry, like listen() does.
      const own: Handler = (payload) => handler(payload);
      ch.handlers.add(own);
      if (!ch.token) open(event, ch);
      return () => {
        if (!ch.handlers.delete(own) || ch.handlers.size > 0 || !ch.token) return;
        const token = ch.token;
        ch.token = null;
        // Already subscribed: stop now; otherwise open() stops it on arrival.
        closers.get(token)?.();
        closers.delete(token);
      };
    },
    count(event) {
      return channels.get(event)?.handlers.size ?? 0;
    },
  };
}
