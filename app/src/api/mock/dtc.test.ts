// Fault code texts of the demo backend in a language other than German and
// English: the core catalog of the language is loaded on demand, and generic
// codes put together from phrases read in that language too.
import { afterAll, describe, expect, it } from "vitest";
import { setCoreLanguage } from "./core";
import { describeDtc, lookupDtc } from "./dtc";

describe("demo fault code texts", () => {
  afterAll(() => setCoreLanguage("de"));

  it("written out codes follow the language", async () => {
    await setCoreLanguage("en");
    const english = lookupDtc("P0420").text;
    await setCoreLanguage("fr");
    const french = lookupDtc("P0420");
    expect(french.text).not.toBe(english);
    expect(french.text.length).toBeGreaterThan(0);
    expect(french.severityLabel).not.toBe("High");
  });

  it("codes put together from phrases follow the language", async () => {
    await setCoreLanguage("en");
    // B0003 is "Driver frontal stage ... deployment control - ..." from two
    // phrases of the generic table, not a written out text.
    const english = describeDtc("B0003");
    expect(english).not.toMatch(/%\d/);
    for (const code of ["fr", "hi", "ja"]) {
      await setCoreLanguage(code);
      const text = describeDtc("B0003");
      expect(text, code).not.toBe(english);
      expect(text, code).not.toMatch(/%\d/);
    }
  });

  it("the single source note is translated", async () => {
    await setCoreLanguage("en");
    expect(lookupDtc("U0177").sourceNote).toBe("Meaning from one source");
    await setCoreLanguage("ko");
    const note = lookupDtc("U0177").sourceNote;
    expect(note).not.toBe("");
    expect(note).not.toBe("Meaning from one source");
  });
});
