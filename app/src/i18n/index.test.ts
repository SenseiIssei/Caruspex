// The interface catalogs are keyed by textKey(); the generator and the
// lookup must agree on it, or every text would fall back to German.
import { afterAll, describe, expect, it } from "vitest";
import { setLanguage, t, textKey } from "./index";

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
});
