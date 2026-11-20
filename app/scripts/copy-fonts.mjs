// Copies the bundled fonts and their licences into public/fonts, where the
// style sheet loads them from. The originals stay in resources/fonts.
import { copyFileSync, mkdirSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const from = join(here, "..", "..", "resources", "fonts");
const to = join(here, "..", "public", "fonts");
mkdirSync(to, { recursive: true });
let n = 0;
for (const name of readdirSync(from)) {
  if (!/\.(ttf|txt)$/i.test(name)) continue;
  copyFileSync(join(from, name), join(to, name));
  n++;
}
console.log(`fonts: ${n} files`);

// The licence of Caruspex and the notices of every component the programs
// contain (tools/gen_notices.py) go into public/legal, so every package of
// the app ships them and Settings can show them without a network.
const root = join(here, "..", "..");
const legal = join(here, "..", "public", "legal");
mkdirSync(legal, { recursive: true });
copyFileSync(join(root, "LICENSE"), join(legal, "LICENSE.txt"));
copyFileSync(join(root, "THIRD_PARTY_NOTICES.md"), join(legal, "THIRD_PARTY_NOTICES.txt"));
console.log("legal: 2 files");
