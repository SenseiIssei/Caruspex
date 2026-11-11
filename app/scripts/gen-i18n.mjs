// Builds the interface catalogs from the shared translation files.
//
// translations/source/ui_*.json lists every interface text with its Qt
// context and the German source; translations/<code>/ui_*.json maps the id to
// the text in that language (a list of forms for plural texts). The output is
// one JSON object per language in src/i18n/generated/, keyed by
// "context|german source", so the frontend looks texts up exactly the way
// Qt's tr() did. English comes from the source files themselves. Texts that
// are new in this interface live in src/i18n/new-texts.json and are merged in
// with their English text until they get translated.
import { readdirSync, readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const app = join(here, "..");
const root = join(app, "..", "translations");
const out = join(app, "src", "i18n", "generated");

const read = (path) => JSON.parse(readFileSync(path, "utf8"));
const uiFiles = (dir) =>
  readdirSync(dir)
    .filter((name) => /^ui_.*\.json$/.test(name))
    .sort()
    .map((name) => join(dir, name));

const source = [];
for (const file of uiFiles(join(root, "source"))) source.push(...read(file));

const languages = readdirSync(root, { withFileTypes: true })
  .filter((d) => d.isDirectory() && d.name !== "source")
  .map((d) => d.name)
  .sort();

const key = (entry) => `${entry.context}|${entry.de}`;

mkdirSync(out, { recursive: true });

// new-texts.json is the list for the translators: [{context, de, en}]. While
// several pages are being ported at once, each may keep its own list in
// new-texts/<area>.json; those are merged the same way.
const newTextsPath = join(app, "src", "i18n", "new-texts.json");
const newTexts = existsSync(newTextsPath) ? read(newTextsPath) : [];
const newTextsDir = join(app, "src", "i18n", "new-texts");
if (existsSync(newTextsDir)) {
  for (const name of readdirSync(newTextsDir).filter((n) => n.endsWith(".json")).sort()) {
    newTexts.push(...read(join(newTextsDir, name)));
  }
}

// English: straight from the source.
const english = {};
for (const entry of source) english[key(entry)] = entry.en;
for (const entry of newTexts) {
  const k = key(entry);
  if (!(k in english)) english[k] = entry.en;
}
writeFileSync(join(out, "en.json"), JSON.stringify(english));

let summary = `en ${Object.keys(english).length}`;
let totalMissing = 0;
for (const code of languages) {
  const byId = {};
  for (const file of uiFiles(join(root, code))) Object.assign(byId, read(file));
  const catalog = {};
  let missing = 0;
  for (const entry of source) {
    const text = byId[entry.id];
    if (text === undefined || text === "" || (Array.isArray(text) && text.length === 0)) {
      missing++;
      continue;
    }
    catalog[key(entry)] = text;
  }
  // New texts carry their translations along: {"context", "de", "en", "tr": {"es": ...}}.
  for (const entry of newTexts) {
    const k = key(entry);
    const text = entry.tr?.[code];
    if (text && !(k in catalog)) catalog[k] = text;
    else if (!text && !(k in catalog)) missing++;
  }
  writeFileSync(join(out, `${code}.json`), JSON.stringify(catalog));
  totalMissing += missing;
  summary += `, ${code} ${Object.keys(catalog).length}${missing ? ` (${missing} missing)` : ""}`;
}

// Core texts (fault descriptions, value names, messages) by their English
// text, the way the Rust core looks them up. Only the demo car of the browser
// preview uses these; inside the app the backend translates them.
const coreFiles = (dir) =>
  readdirSync(dir)
    .filter((name) => /^core_.*\.json$/.test(name))
    .sort()
    .map((name) => join(dir, name));
const coreSource = [];
for (const file of coreFiles(join(root, "source"))) coreSource.push(...read(file));
for (const code of languages) {
  const byId = {};
  for (const file of coreFiles(join(root, code))) Object.assign(byId, read(file));
  const catalog = {};
  for (const entry of coreSource) {
    const text = byId[entry.id];
    if (typeof text === "string" && text !== "") catalog[entry.en] = text;
  }
  writeFileSync(join(out, `core-${code}.json`), JSON.stringify(catalog));
}

// The list of languages with a catalog, for the settings page and the loader.
writeFileSync(join(out, "languages.json"), JSON.stringify(["de", "en", ...languages]));
console.log(`i18n: ${source.length} texts, ${newTexts.length} new; ${summary}; core ${coreSource.length}`);

// In the CI (I18N_STRICT=1) a missing translation fails the build, so new
// interface texts get translated before they reach main.
const noEnglish = source.filter((e) => !e.plural && !e.en).map((e) => e.id);
if (process.env.I18N_STRICT === "1" && noEnglish.length > 0) {
  console.error(`i18n: interface texts without English: ${noEnglish.join(", ")} (translations/source/ui_*.json)`);
  process.exit(1);
}
if (process.env.I18N_STRICT === "1" && totalMissing > 0) {
  console.error(`i18n: ${totalMissing} interface translations are missing, see src/i18n/new-texts.json`);
  process.exit(1);
}
