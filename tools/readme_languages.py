"""Keeps the language bar at the top of every README the same.

    py -I tools/readme_languages.py           update the bar in every README that exists
    py -I tools/readme_languages.py --check   only check (CI)

The bar sits between <!-- languages --> and <!-- /languages --> right below
the title; the current language is bold, the others are links. The English
README.md is the reference, the other files are translations of it.
"""

from __future__ import annotations

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


def file_of(code: str) -> str:
    return "README.md" if code == "en" else f"README.{code}.md"


def bar_for(current: str) -> str:
    present = [(c, n) for c, n in LANGUAGES if os.path.exists(os.path.join(ROOT, file_of(c)))]
    parts = [f"**{n}**" if c == current else f"[{n}]({file_of(c)})" for c, n in present]
    return "<!-- languages -->\n" + " · ".join(parts) + "\n<!-- /languages -->\n"


def main() -> int:
    check = "--check" in sys.argv
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
