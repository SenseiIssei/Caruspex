// Tells listeners about changed keys at most once per animation frame. A
// live value can arrive a hundred times a second; the screen changes sixty
// times at most, and a listener of several keys hears about them together.
type Schedule = (run: () => void) => void;

const nextFrame: Schedule = (run) => {
  if (typeof requestAnimationFrame === "function") requestAnimationFrame(run);
  else setTimeout(run, 16);
};

export class FrameNotifier<K> {
  private readonly byKey = new Map<K, Set<() => void>>();
  private readonly dirty = new Set<K>();
  private everything = false;
  private pending = false;

  constructor(private readonly schedule: Schedule = nextFrame) {}

  subscribe(keys: readonly K[], fn: () => void): () => void {
    for (const key of keys) {
      let set = this.byKey.get(key);
      if (!set) {
        set = new Set();
        this.byKey.set(key, set);
      }
      set.add(fn);
    }
    return () => {
      for (const key of keys) {
        const set = this.byKey.get(key);
        if (!set) continue;
        set.delete(fn);
        if (set.size === 0) this.byKey.delete(key);
      }
    };
  }

  // A key that nobody shows costs nothing.
  mark(key: K): void {
    if (!this.byKey.has(key)) return;
    this.dirty.add(key);
    this.request();
  }

  // Every key changed at once (the values were cleared).
  markAll(): void {
    this.everything = true;
    this.request();
  }

  listeners(): number {
    let n = 0;
    for (const set of this.byKey.values()) n += set.size;
    return n;
  }

  private request(): void {
    if (this.pending) return;
    this.pending = true;
    this.schedule(() => this.flush());
  }

  flush(): void {
    this.pending = false;
    const keys = this.everything ? [...this.byKey.keys()] : [...this.dirty];
    this.everything = false;
    this.dirty.clear();
    const due = new Set<() => void>();
    for (const key of keys) for (const fn of this.byKey.get(key) ?? []) due.add(fn);
    for (const fn of due) fn();
  }
}
