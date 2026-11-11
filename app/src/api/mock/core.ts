// Core texts for the demo backend. The Rust core writes German and English
// next to each other (L(de, en)) and looks every other language up by the
// English text; the demo does the same with the catalogs that
// scripts/gen-i18n.mjs writes from translations/<code>/core_*.json.

type CoreCatalog = Record<string, string>;

const loaders = import.meta.glob<{ default: CoreCatalog }>("../../i18n/generated/core-*.json");

let active = "de";
let catalog: CoreCatalog = {};

export async function setCoreLanguage(code: string): Promise<void> {
  active = code;
  catalog = {};
  if (code === "de" || code === "en") return;
  const loader = loaders[`../../i18n/generated/core-${code}.json`];
  if (loader) catalog = (await loader()).default;
}

export function coreLanguage(): string {
  return active;
}

// Picks the text for the active language, like L() in core/Lang.h.
export function L(de: string, en: string): string {
  if (active === "de") return de;
  if (active === "en") return en;
  return catalog[en] ?? en;
}

// For lists of lines (causes, checks): the catalog holds the whole block,
// one line per entry, the way the C++ table keeps it.
export function Llist(de: string[], en: string[]): string[] {
  if (active === "de") return de;
  if (active === "en") return en;
  const block = catalog[en.join("\n")];
  return block ? block.split("\n") : en;
}
