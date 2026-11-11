// A small QR code encoder (ISO/IEC 18004, model 2): byte mode with UTF-8,
// error correction level L or M, versions 1 to 40, all eight masks with the
// standard penalty rules. Written for the report summary that goes to a
// phone without any server; no library needed.

export type Ecc = "L" | "M";

// Error correction codewords per block and number of blocks, index =
// version (index 0 unused). From the tables of the standard.
const ECC_PER_BLOCK: Record<Ecc, readonly number[]> = {
  L: [-1, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
  M: [-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28],
};
const BLOCKS: Record<Ecc, readonly number[]> = {
  L: [-1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25],
  M: [-1, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49],
};
// The two bits of the level in the format information.
const ECC_FORMAT: Record<Ecc, number> = { L: 1, M: 0 };

export interface QrCode {
  version: number;
  size: number;
  ecc: Ecc;
  mask: number;
  // modules[y][x], true = dark.
  modules: boolean[][];
}

// Modules that carry data, after the function patterns are taken out.
export function rawDataModules(version: number): number {
  let result = (16 * version + 128) * version + 64;
  if (version >= 2) {
    const align = Math.floor(version / 7) + 2;
    result -= (25 * align - 10) * align - 55;
    if (version >= 7) result -= 36;
  }
  return result;
}

export function dataCodewords(version: number, ecc: Ecc): number {
  return Math.floor(rawDataModules(version) / 8) - ECC_PER_BLOCK[ecc][version]! * BLOCKS[ecc][version]!;
}

// The length field of byte mode: 8 bits up to version 9, 16 above.
function countBits(version: number): number {
  return version <= 9 ? 8 : 16;
}

// How many bytes fit in byte mode.
export function byteCapacity(version: number, ecc: Ecc): number {
  return Math.floor((dataCodewords(version, ecc) * 8 - 4 - countBits(version)) / 8);
}

// --- Reed-Solomon over GF(256) with the polynomial 0x11D -----------------------------------

export function gfMultiply(x: number, y: number): number {
  let z = 0;
  for (let i = 7; i >= 0; i--) {
    z = (z << 1) ^ ((z >>> 7) * 0x11d);
    z ^= ((y >>> i) & 1) * x;
  }
  return z & 0xff;
}

export function rsDivisor(degree: number): number[] {
  const result = new Array<number>(degree).fill(0);
  result[degree - 1] = 1;
  let root = 1;
  for (let i = 0; i < degree; i++) {
    for (let j = 0; j < degree; j++) {
      result[j] = gfMultiply(result[j]!, root);
      if (j + 1 < degree) result[j]! ^= result[j + 1]!;
    }
    root = gfMultiply(root, 0x02);
  }
  return result;
}

export function rsRemainder(data: readonly number[], divisor: readonly number[]): number[] {
  const result = new Array<number>(divisor.length).fill(0);
  for (const b of data) {
    const factor = b ^ result.shift()!;
    result.push(0);
    for (let i = 0; i < divisor.length; i++) result[i]! ^= gfMultiply(divisor[i]!, factor);
  }
  return result;
}

// --- Bits ----------------------------------------------------------------------------------

function appendBits(out: number[], value: number, length: number): void {
  for (let i = length - 1; i >= 0; i--) out.push((value >>> i) & 1);
}

// The data codewords for the text in byte mode, padded to the version.
export function encodeData(bytes: Uint8Array, version: number, ecc: Ecc): number[] {
  const bits: number[] = [];
  appendBits(bits, 0b0100, 4);
  appendBits(bits, bytes.length, countBits(version));
  for (const b of bytes) appendBits(bits, b, 8);
  const capacity = dataCodewords(version, ecc) * 8;
  appendBits(bits, 0, Math.min(4, capacity - bits.length));
  appendBits(bits, 0, (8 - (bits.length % 8)) % 8);
  for (let pad = 0xec; bits.length < capacity; pad ^= 0xec ^ 0x11) appendBits(bits, pad, 8);
  const out: number[] = [];
  for (let i = 0; i < bits.length; i += 8) {
    let v = 0;
    for (let j = 0; j < 8; j++) v = (v << 1) | bits[i + j]!;
    out.push(v);
  }
  return out;
}

// Splits into blocks, adds the error correction and interleaves.
export function addEcc(data: readonly number[], version: number, ecc: Ecc): number[] {
  const blocks = BLOCKS[ecc][version]!;
  const eccLen = ECC_PER_BLOCK[ecc][version]!;
  const raw = Math.floor(rawDataModules(version) / 8);
  const shortBlocks = blocks - (raw % blocks);
  const shortLen = Math.floor(raw / blocks);
  const divisor = rsDivisor(eccLen);
  const all: number[][] = [];
  for (let i = 0, k = 0; i < blocks; i++) {
    const dat = data.slice(k, k + shortLen - eccLen + (i < shortBlocks ? 0 : 1));
    k += dat.length;
    const rem = rsRemainder(dat, divisor);
    if (i < shortBlocks) dat.push(0);
    all.push([...dat, ...rem]);
  }
  const out: number[] = [];
  for (let i = 0; i < all[0]!.length; i++) {
    for (let j = 0; j < all.length; j++) {
      // The padding byte of the short blocks is skipped.
      if (i !== shortLen - eccLen || j >= shortBlocks) out.push(all[j]![i]!);
    }
  }
  return out;
}

// --- Format and version information --------------------------------------------------------

export function formatBits(ecc: Ecc, mask: number): number {
  const data = (ECC_FORMAT[ecc] << 3) | mask;
  let rem = data;
  for (let i = 0; i < 10; i++) rem = (rem << 1) ^ ((rem >>> 9) * 0x537);
  return ((data << 10) | rem) ^ 0x5412;
}

export function versionBits(version: number): number {
  let rem = version;
  for (let i = 0; i < 12; i++) rem = (rem << 1) ^ ((rem >>> 11) * 0x1f25);
  return (version << 12) | rem;
}

export function alignmentPositions(version: number): number[] {
  if (version === 1) return [];
  const count = Math.floor(version / 7) + 2;
  const size = version * 4 + 17;
  const step = version === 32 ? 26 : Math.ceil((version * 4 + 4) / (count * 2 - 2)) * 2;
  const result = [6];
  for (let pos = size - 7; result.length < count; pos -= step) result.splice(1, 0, pos);
  return result;
}

// --- The matrix ------------------------------------------------------------------------------

class Matrix {
  readonly modules: boolean[][];
  readonly fixed: boolean[][];
  constructor(readonly size: number) {
    this.modules = Array.from({ length: size }, () => new Array<boolean>(size).fill(false));
    this.fixed = Array.from({ length: size }, () => new Array<boolean>(size).fill(false));
  }
  set(x: number, y: number, dark: boolean): void {
    this.modules[y]![x] = dark;
    this.fixed[y]![x] = true;
  }
}

function drawFunctionPatterns(m: Matrix, version: number): void {
  const size = m.size;
  for (let i = 0; i < size; i++) {
    m.set(6, i, i % 2 === 0);
    m.set(i, 6, i % 2 === 0);
  }
  for (const [cx, cy] of [
    [3, 3],
    [size - 4, 3],
    [3, size - 4],
  ] as const) {
    for (let dy = -4; dy <= 4; dy++) {
      for (let dx = -4; dx <= 4; dx++) {
        const x = cx + dx;
        const y = cy + dy;
        const dist = Math.max(Math.abs(dx), Math.abs(dy));
        if (x >= 0 && x < size && y >= 0 && y < size) m.set(x, y, dist !== 2 && dist !== 4);
      }
    }
  }
  const align = alignmentPositions(version);
  const n = align.length;
  for (let i = 0; i < n; i++) {
    for (let j = 0; j < n; j++) {
      // Not on top of the three finder patterns.
      if ((i === 0 && j === 0) || (i === 0 && j === n - 1) || (i === n - 1 && j === 0)) continue;
      for (let dy = -2; dy <= 2; dy++) {
        for (let dx = -2; dx <= 2; dx++) m.set(align[i]! + dx, align[j]! + dy, Math.max(Math.abs(dx), Math.abs(dy)) !== 1);
      }
    }
  }
  drawFormat(m, formatBits("M", 0)); // reserved, overwritten later
  if (version >= 7) {
    const bits = versionBits(version);
    for (let i = 0; i < 18; i++) {
      const dark = ((bits >>> i) & 1) !== 0;
      const a = size - 11 + (i % 3);
      const b = Math.floor(i / 3);
      m.set(a, b, dark);
      m.set(b, a, dark);
    }
  }
}

function drawFormat(m: Matrix, bits: number): void {
  const size = m.size;
  const bit = (i: number) => ((bits >>> i) & 1) !== 0;
  for (let i = 0; i <= 5; i++) m.set(8, i, bit(i));
  m.set(8, 7, bit(6));
  m.set(8, 8, bit(7));
  m.set(7, 8, bit(8));
  for (let i = 9; i < 15; i++) m.set(14 - i, 8, bit(i));
  for (let i = 0; i < 8; i++) m.set(size - 1 - i, 8, bit(i));
  for (let i = 8; i < 15; i++) m.set(8, size - 15 + i, bit(i));
  m.set(8, size - 8, true);
}

// Which modules belong to the fixed patterns of a version (true), the rest
// carries data.
export function functionModules(version: number): boolean[][] {
  const m = new Matrix(version * 4 + 17);
  drawFunctionPatterns(m, version);
  return m.fixed;
}

// The order in which the data bits fill the matrix: two columns at a time
// from the right, up and down in turn, around the timing column.
export function dataOrder(size: number, fixed: readonly (readonly boolean[])[]): [number, number][] {
  const out: [number, number][] = [];
  for (let right = size - 1; right >= 1; right -= 2) {
    if (right === 6) right = 5;
    for (let vert = 0; vert < size; vert++) {
      for (let j = 0; j < 2; j++) {
        const x = right - j;
        const upward = ((right + 1) & 2) === 0;
        const y = upward ? size - 1 - vert : vert;
        if (!fixed[y]![x]) out.push([x, y]);
      }
    }
  }
  return out;
}

export function maskBit(mask: number, x: number, y: number): boolean {
  switch (mask) {
    case 0:
      return (x + y) % 2 === 0;
    case 1:
      return y % 2 === 0;
    case 2:
      return x % 3 === 0;
    case 3:
      return (x + y) % 3 === 0;
    case 4:
      return (Math.floor(x / 3) + Math.floor(y / 2)) % 2 === 0;
    case 5:
      return ((x * y) % 2) + ((x * y) % 3) === 0;
    case 6:
      return (((x * y) % 2) + ((x * y) % 3)) % 2 === 0;
    default:
      return (((x + y) % 2) + ((x * y) % 3)) % 2 === 0;
  }
}

const FINDER_AFTER_LIGHT = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1] as const;
const FINDER_BEFORE_LIGHT = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0] as const;

// The penalty of the standard: long runs, 2x2 blocks, finder-like
// patterns and an unbalanced share of dark modules.
export function penalty(modules: readonly (readonly boolean[])[]): number {
  const size = modules.length;
  let score = 0;
  const line = (get: (i: number) => boolean) => {
    let run = 1;
    for (let i = 1; i <= size; i++) {
      if (i < size && get(i) === get(i - 1)) run++;
      else {
        if (run >= 5) score += 3 + (run - 5);
        run = 1;
      }
    }
    // 1:1:3:1:1 with four light modules on one side; outside the code is
    // the light quiet zone.
    const padded = [false, false, false, false];
    for (let i = 0; i < size; i++) padded.push(get(i));
    padded.push(false, false, false, false);
    const matches = (at: number, pattern: readonly number[]) => pattern.every((v, k) => padded[at + k] === (v === 1));
    for (let i = 0; i + 11 <= padded.length; i++) {
      if (matches(i, FINDER_AFTER_LIGHT) || matches(i, FINDER_BEFORE_LIGHT)) score += 40;
    }
  };
  for (let y = 0; y < size; y++) line((i) => modules[y]![i]!);
  for (let x = 0; x < size; x++) line((i) => modules[i]![x]!);
  for (let y = 0; y + 1 < size; y++) {
    for (let x = 0; x + 1 < size; x++) {
      const c = modules[y]![x];
      if (c === modules[y]![x + 1] && c === modules[y + 1]![x] && c === modules[y + 1]![x + 1]) score += 3;
    }
  }
  let dark = 0;
  for (const row of modules) for (const v of row) if (v) dark++;
  const total = size * size;
  const k = Math.ceil(Math.abs(dark * 20 - total * 10) / total) - 1;
  score += Math.max(0, k) * 10;
  return score;
}

// Encodes the text as a QR code with the smallest version that holds it.
export function encodeQr(text: string, ecc: Ecc = "M"): QrCode {
  const bytes = new TextEncoder().encode(text);
  let version = 1;
  while (version <= 40 && byteCapacity(version, ecc) < bytes.length) version++;
  if (version > 40) throw new Error("text too long for a QR code");
  const size = version * 4 + 17;
  const codewords = addEcc(encodeData(bytes, version, ecc), version, ecc);

  const base = new Matrix(size);
  drawFunctionPatterns(base, version);
  const order = dataOrder(size, base.fixed);
  order.forEach(([x, y], i) => {
    const byte = codewords[i >>> 3] ?? 0;
    base.modules[y]![x] = i < codewords.length * 8 && ((byte >>> (7 - (i & 7))) & 1) === 1;
  });

  let best: QrCode | null = null;
  let bestScore = Infinity;
  for (let mask = 0; mask < 8; mask++) {
    const m = new Matrix(size);
    for (let y = 0; y < size; y++) {
      for (let x = 0; x < size; x++) {
        m.modules[y]![x] = base.modules[y]![x]!;
        m.fixed[y]![x] = base.fixed[y]![x]!;
      }
    }
    for (const [x, y] of order) if (maskBit(mask, x, y)) m.modules[y]![x] = !m.modules[y]![x];
    drawFormat(m, formatBits(ecc, mask));
    const score = penalty(m.modules);
    if (score < bestScore) {
      bestScore = score;
      best = { version, size, ecc, mask, modules: m.modules };
    }
  }
  return best!;
}

// The code as SVG path data, one unit per module, for a viewBox of
// size + 2 * quiet.
export function qrPath(code: QrCode, quiet = 4): string {
  let d = "";
  for (let y = 0; y < code.size; y++) {
    for (let x = 0; x < code.size; x++) {
      if (code.modules[y]![x]) d += `M${x + quiet} ${y + quiet}h1v1h-1z`;
    }
  }
  return d;
}
