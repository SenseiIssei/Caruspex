"""Keeps the parts that every README shares the same.

    py -I tools/readme_languages.py           update every README that exists
    py -I tools/readme_languages.py --check   only check (CI)

1. The language bar between <!-- languages --> and <!-- /languages --> right
   below the title; the current language is bold, the others are links.
2. The link to the documentation website (docs/site, GitHub Pages) in the
   reader's language, between <!-- docs --> and <!-- /docs --> below the bar.
3. The honest list of what has been tested, between <!-- tested --> and
   <!-- /tested -->, put right before the download section the first time.

The texts of 2 and 3 are in tools/readme_blocks.json, one entry per language.
The English README.md is the reference, the other files are translations of it.
"""

from __future__ import annotations

import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Same order and names as the language list of the app.
LANGUAGES = [
    ("en", "English"),
    ("de", "Deutsch"),
    ("es", "Español"),
    ("fr", "Français"),
    ("pt", "Português"),
    ("it", "Italiano"),
    ("nl", "Nederlands"),
    ("pl", "Polski"),
    ("cs", "Čeština"),
    ("ru", "Русский"),
    ("uk", "Українська"),
    ("tr", "Türkçe"),
    ("ar", "العربية"),
    ("zh", "中文"),
    ("ja", "日本語"),
    ("ko", "한국어"),
    ("hi", "हिन्दी"),
    ("id", "Bahasa Indonesia"),
]
BAR = re.compile(r"<!-- languages -->.*?<!-- /languages -->\n?", re.S)
DOCS_URL = "https://senseiissei.github.io/Caruspex/{code}/index.html"
FORM_URL = "https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml"


def set_block(text: str, name: str, body: str, after: str = "", before: str = "") -> str:
    """Replaces the block, or puts it the first time right after the line
    `after` (with a blank line) or right before the line `before`."""
    pattern = re.compile(rf"<!-- {name} -->.*?<!-- /{name} -->\n", re.S)
    new = f"<!-- {name} -->\n{body.strip()}\n<!-- /{name} -->\n"
    if pattern.search(text):
        return pattern.sub(lambda _m: new, text, count=1)
    if after:
        at = text.find(after + "\n")
        if at >= 0:
            end = at + len(after) + 1
            return text[:end] + "\n" + new + text[end:]
    else:
        at = text.find("\n" + before + "\n")
        if at >= 0:
            return text[: at + 1] + new + "\n" + text[at + 1 :]
    raise SystemExit(f"cannot place the {name} block: line '{after or before}' not found")


def file_of(code: str) -> str:
    return "README.md" if code == "en" else f"README.{code}.md"


def bar_for(current: str) -> str:
    present = [(c, n) for c, n in LANGUAGES if os.path.exists(os.path.join(ROOT, file_of(c)))]
    parts = [f"**{n}**" if c == current else f"[{n}]({file_of(c)})" for c, n in present]
    return "<!-- languages -->\n" + " · ".join(parts) + "\n<!-- /languages -->\n"


def main() -> int:
    check = "--check" in sys.argv
    blocks = json.load(open(os.path.join(ROOT, "tools", "readme_blocks.json"), encoding="utf-8"))
    stale = []
    for code, _ in LANGUAGES:
        path = os.path.join(ROOT, file_of(code))
        if not os.path.exists(path):
            continue
        text = open(path, encoding="utf-8").read()
        bar = bar_for(code)
        if BAR.search(text):
            new = BAR.sub(lambda _m: bar, text, count=1)
        else:
            # Right below the first heading.
            first, _, rest = text.partition("\n")
            new = first + "\n\n" + bar + rest
        texts = blocks[code]
        if chr(0x2014) in str(texts) or chr(0x2013) in str(texts):
            raise SystemExit(f"tools/readme_blocks.json ({code}) contains a long dash")
        docs = texts["docs"].replace("{docs}", DOCS_URL.format(code=code))
        new = set_block(new, "docs", docs, after="<!-- /languages -->")
        tested = texts["tested"].replace("{form}", FORM_URL)
        new = set_block(new, "tested", tested, before=texts["before"])
        if new != text:
            stale.append(file_of(code))
            if not check:
                open(path, "w", encoding="utf-8", newline="\n").write(new)
    if check and stale:
        print("Language bar out of date in: " + ", ".join(stale) + ". Run tools/readme_languages.py.")
        return 1
    print(("checked " if check else "updated ") + str(len(stale) if not check else 0) + " file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
