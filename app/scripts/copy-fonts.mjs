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
