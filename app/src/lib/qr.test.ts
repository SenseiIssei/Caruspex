import { describe, expect, it } from "vitest";
import {
  alignmentPositions,
  byteCapacity,
  dataOrder,
  encodeQr,
  formatBits,
  functionModules,
  maskBit,
  rsDivisor,
  rsRemainder,
  versionBits,
  type Ecc,
  type QrCode,
} from "./qr";

// The format information table of the standard (15 bits, already masked).
const FORMAT_TABLE: Record<Ecc, string[]> = {
  L: [
    "111011111000100",
    "111001011110011",
    "111110110101010",
    "111100010011101",
    "110011000101111",
    "110001100011000",
    "110110001000001",
    "110100101110110",
  ],
  M: [
    "101010000010010",
    "101000100100101",
    "101111001111100",
    "101101101001011",
    "100010111111001",
    "100000011001110",
    "100111110010111",
    "100101010100000",
  ],
};

describe("tables of the standard", () => {
  it("writes the format information", () => {
    for (const ecc of ["L", "M"] as const) {
      FORMAT_TABLE[ecc].forEach((bits, mask) => expect(formatBits(ecc, mask)).toBe(Number.parseInt(bits, 2)));
    }
  });

  it("writes the version information", () => {
    expect(versionBits(7)).toBe(0x07c94);
    expect(versionBits(8)).toBe(0x085bc);
    expect(versionBits(10)).toBe(0x0a4d3);
  });

  it("knows the byte capacity", () => {
    expect([1, 2, 3, 4, 5, 10, 40].map((v) => byteCapacity(v, "L"))).toEqual([17, 32, 53, 78, 106, 271, 2953]);
    expect([1, 2, 3, 4, 5, 10, 40].map((v) => byteCapacity(v, "M"))).toEqual([14, 26, 42, 62, 84, 213, 2331]);
  });

  it("places the alignment patterns", () => {
    expect(alignmentPositions(1)).toEqual([]);
    expect(alignmentPositions(2)).toEqual([6, 18]);
    expect(alignmentPositions(7)).toEqual([6, 22, 38]);
    expect(alignmentPositions(32)).toEqual([6, 34, 60, 86, 112, 138]);
  });

  it("computes Reed-Solomon codewords like the HELLO WORLD example (1-M)", () => {
    const data = [32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17];
    expect(rsRemainder(data, rsDivisor(10))).toEqual([196, 35, 39, 119, 235, 215, 231, 226, 93, 23]);
  });
});

// Reads a code back: format bits, mask, codewords, blocks, error
// correction and the byte-mode text.
function decode(code: QrCode): string {
  const { size, modules, version, ecc } = code;
  const bit = (x: number, y: number) => (modules[y]![x] ? 1 : 0);
  let first = 0;
  const firstPositions: [number, number][] = [
    [8, 0],
    [8, 1],
    [8, 2],
    [8, 3],
    [8, 4],
    [8, 5],
    [8, 7],
    [8, 8],
    [7, 8],
    [5, 8],
    [4, 8],
    [3, 8],
    [2, 8],
    [1, 8],
    [0, 8],
  ];
  firstPositions.forEach(([x, y], i) => (first |= bit(x, y) << i));
  let second = 0;
  for (let i = 0; i < 8; i++) second |= bit(size - 1 - i, 8) << i;
  for (let i = 8; i < 15; i++) second |= bit(8, size - 15 + i) << i;
  expect(second).toBe(first);
  expect(first).toBe(formatBits(ecc, code.mask));
  expect(bit(8, size - 8)).toBe(1);

  const order = dataOrder(size, functionModules(version));
  const bits = order.map(([x, y]) => bit(x, y) ^ (maskBit(code.mask, x, y) ? 1 : 0));
  const codewords: number[] = [];
  for (let i = 0; i + 8 <= bits.length; i += 8) codewords.push(bits.slice(i, i + 8).reduce((a, b) => (a << 1) | b, 0));

  // Undo the interleaving and check every block against its error correction.
  const blockCount = { L: [0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4], M: [0, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5] }[ecc][version]!;
  const eccLen = { L: [0, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18], M: [0, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26] }[ecc][version]!;
  const shortBlocks = blockCount - (codewords.length % blockCount);
  const shortLen = Math.floor(codewords.length / blockCount);
  const blocks: number[][] = Array.from({ length: blockCount }, () => []);
  let k = 0;
  for (let i = 0; i < shortLen + 1; i++) {
    for (let j = 0; j < blockCount; j++) {
      // The short blocks have no codeword at the end of the data part.
      if (i === shortLen - eccLen && j < shortBlocks) continue;
      blocks[j]!.push(codewords[k++]!);
    }
  }
  const data: number[] = [];
  for (const block of blocks) {
    const dat = block.slice(0, block.length - eccLen);
    expect(rsRemainder(dat, rsDivisor(eccLen))).toEqual(block.slice(block.length - eccLen));
    data.push(...dat);
  }

  const stream = data.flatMap((b) => Array.from({ length: 8 }, (_, i) => (b >>> (7 - i)) & 1));
  const read = (from: number, n: number) => stream.slice(from, from + n).reduce((a, b) => (a << 1) | b, 0);
  expect(read(0, 4)).toBe(0b0100);
  const countLength = version <= 9 ? 8 : 16;
  const length = read(4, countLength);
  const bytes = new Uint8Array(length);
  for (let i = 0; i < length; i++) bytes[i] = read(4 + countLength + i * 8, 8);
  return new TextDecoder().decode(bytes);
}

describe("encodeQr", () => {
  it("draws the finder patterns and the timing lines", () => {
    const code = encodeQr("Caruspex");
    expect(code.version).toBe(1);
    expect(code.size).toBe(21);
    const m = code.modules;
    for (const [ox, oy] of [
      [0, 0],
      [14, 0],
      [0, 14],
    ]) {
      expect(m[oy!]![ox!]).toBe(true);
      expect(m[oy! + 1]![ox! + 1]).toBe(false);
      expect(m[oy! + 3]![ox! + 3]).toBe(true);
    }
    for (let i = 8; i < 13; i++) {
      expect(m[6]![i]).toBe(i % 2 === 0);
      expect(m[i]![6]).toBe(i % 2 === 0);
    }
  });

  it("reads back what it wrote, in short and long texts with umlauts", () => {
    const texts = [
      "Caruspex",
      "Fahrzeug: Volkswagen Golf IV 2003\nVIN: WVWZZZ1JZ3W386752\nFehler: P0171, P0300\nPunkte: 72/100\nDatum: 11.11.2026",
      `Prüfbericht ${"äöü ß €".repeat(15)}`,
    ];
    for (const text of texts) {
      for (const ecc of ["L", "M"] as const) {
        const code = encodeQr(text, ecc);
        expect(code.version).toBeLessThanOrEqual(10);
        expect(decode(code)).toBe(text);
      }
    }
  });

  it("uses the smallest version that holds the text", () => {
    expect(encodeQr("x".repeat(14), "M").version).toBe(1);
    expect(encodeQr("x".repeat(15), "M").version).toBe(2);
    expect(encodeQr("x".repeat(200), "M").version).toBe(10);
  });
});
