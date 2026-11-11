// Shared state of the demo backend: the event bus, the demo car, the clock
// and the connection. Every mock module works on this.
import type { ArgsOf, CommandName, EventName, Events, ResultOf } from "../contract";
import { DemoCar } from "./car";

export type Handler<K extends CommandName> = (args: ArgsOf<K>) => ResultOf<K> | Promise<ResultOf<K>>;
export type Handlers = { [K in CommandName]: Handler<K> };
export type PartialHandlers = Partial<Handlers>;

type Listener = (payload: unknown) => void;
const listeners = new Map<EventName, Set<Listener>>();

export function emit<E extends EventName>(event: E, payload: Events[E]): void {
  const set = listeners.get(event);
  if (!set) return;
  for (const fn of [...set]) {
    try {
      fn(payload);
    } catch (error) {
      console.error(error);
    }
  }
}

export function subscribe<E extends EventName>(event: E, fn: (payload: Events[E]) => void): () => void {
  let set = listeners.get(event);
  if (!set) {
    set = new Set();
    listeners.set(event, set);
  }
  set.add(fn as Listener);
  return () => set.delete(fn as Listener);
}

const programStart = performance.now();

// Seconds since the program started, like appSeconds() in Session.cpp.
export function appSeconds(): number {
  return (performance.now() - programStart) / 1000;
}

export const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

export function log(line: string): void {
  emit("log_line", { line });
}

export const car = new DemoCar();

// The connection to the demo car and what runs on it.
export const link = {
  connected: false,
  // Interval of the live loop and the keys it reads, like Worker::pollPids_.
  livePids: [] as number[],
  liveIndex: 0,
  liveTimer: 0 as ReturnType<typeof setInterval> | 0,
  customKeys: [] as number[],
  customTurn: false,
  pollIntervalMs: 20,
  recording: "" as string,
  exchanges: 0,
  cancel: false,
};

export function requireConnection(): boolean {
  return link.connected;
}

// Small busy wrapper: emits busy(true), runs, emits busy(false).
export async function busy<T>(work: () => Promise<T> | T): Promise<T> {
  emit("busy", { active: true });
  try {
    return await work();
  } finally {
    emit("busy", { active: false });
  }
}
