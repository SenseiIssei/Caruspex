// The interface catalogs are keyed by textKey(); the generator and the
// lookup must agree on it, or every text would fall back to German.
import { afterAll, describe, expect, it } from "vitest";
import { localeDate, localeDateTime, localeNumber, setLanguage, t, textKey } from "./index";

describe("interface catalogs", () => {
  afterAll(() => setLanguage("de"));

  it("keys are short and stable", () => {
    expect(textKey("caruspex::FaultsPage", "Fehler lesen")).toMatch(/^[0-9a-z]{1,7}$/);
    expect(textKey("a", "b")).toBe(textKey("a", "b"));
    expect(textKey("a", "b")).not.toBe(textKey("a", "c"));
  });

  it("finds texts in English and in another language", async () => {
    const context = "caruspex::MainWindow";
    const de = "Einstellungen";
    await setLanguage("de");
    expect(t(context, de)).toBe(de);
    await setLanguage("en");
    const english = t(context, de);
    expect(english).not.toBe(de);
    await setLanguage("ja");
    const japanese = t(context, de);
    expect(japanese).not.toBe(de);
    expect(japanese).not.toBe(english);
  });

  it("formats numbers and dates in the active language with the kept formatters", async () => {
    await setLanguage("de");
    expect(localeNumber(1234.5, 1)).toBe("1.234,5");
    expect(localeNumber(1234.5, 0)).toBe("1.235");
    const when = new Date(2026, 10, 22, 14, 5);
    const germanDate = localeDateTime(when);
    await setLanguage("en");
    // A formatter of the language before must not be reused.
    expect(localeNumber(1234.5, 1)).toBe("1,234.5");
    expect(localeNumber(1234.5, 2)).toBe("1,234.50");
    expect(localeDateTime(when)).not.toBe(germanDate);
    expect(localeDate(when, { year: "numeric" })).toBe("2026");
  });
});
