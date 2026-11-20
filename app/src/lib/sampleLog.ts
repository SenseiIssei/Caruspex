// The readings of a measurement for the CSV export, packed into typed arrays.
// A reading as an object took about 70 bytes of the heap; here it is 20
// (time, value, and the index of its name and unit). The arrays double when
// full, so an hour at a hundred readings a second stays under 11 MB.
export interface Sample {
  seconds: number;
  pid: number;
  name: string;
  value: number;
  unit: string;
}

interface Label {
  pid: number;
  name: string;
  unit: string;
}

export class SampleLog {
  private seconds = new Float64Array(1024);
  private values = new Float64Array(1024);
  private labelOf = new Uint32Array(1024);
  private labels: Label[] = [];
  // The label each key used last: a key keeps its name and unit, so this
  // finds it without building a lookup string per reading.
  private lastLabel = new Map<number, number>();
  length = 0;

  push(seconds: number, pid: number, name: string, value: number, unit: string): void {
    if (this.length === this.seconds.length) this.grow();
    let label = this.lastLabel.get(pid);
    const known = label !== undefined ? this.labels[label]! : undefined;
    if (!known || known.name !== name || known.unit !== unit) {
      label = this.labels.findIndex((l) => l.pid === pid && l.name === name && l.unit === unit);
      if (label < 0) label = this.labels.push({ pid, name, unit }) - 1;
      this.lastLabel.set(pid, label);
    }
    const i = this.length++;
    this.seconds[i] = seconds;
    this.values[i] = value;
    this.labelOf[i] = label!;
  }

  at(i: number): Sample {
    const label = this.labels[this.labelOf[i]!]!;
    return { seconds: this.seconds[i]!, pid: label.pid, name: label.name, value: this.values[i]!, unit: label.unit };
  }

  clear(): void {
    this.length = 0;
    this.labels = [];
    this.lastLabel.clear();
    // Gives the memory of a long recording back.
    if (this.seconds.length > 1024) {
      this.seconds = new Float64Array(1024);
      this.values = new Float64Array(1024);
      this.labelOf = new Uint32Array(1024);
    }
  }

  // Bytes held by the arrays, for the performance notes.
  bytes(): number {
    return this.seconds.byteLength + this.values.byteLength + this.labelOf.byteLength;
  }

  private grow(): void {
    const size = this.seconds.length * 2;
    const seconds = new Float64Array(size);
    seconds.set(this.seconds);
    const values = new Float64Array(size);
    values.set(this.values);
    const labelOf = new Uint32Array(size);
    labelOf.set(this.labelOf);
    this.seconds = seconds;
    this.values = values;
    this.labelOf = labelOf;
  }
}
