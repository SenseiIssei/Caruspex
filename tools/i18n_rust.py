"""Finds core texts in the Rust crates that have no translation yet.

    py -I tools/i18n_rust.py            list them
    py -I tools/i18n_rust.py --write    append them to translations/source/core_x2.json
    py -I tools/i18n_rust.py --strict   also fail when a text or a translation is missing (CI)

The Rust core writes texts as l("de", "en"), Text::new("de", "en") or
Text { de: "de", en: "en" }. The other 16 languages are looked up by the
English text, so every English text must be in translations/source/core_*.json.
New ones are appended with fresh ids; translators then fill
translations/<code>/core_x2.json for those ids.
"""

from __future__ import annotations

import glob
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STR = r'"((?:[^"\\]|\\.)*)"'
PATTERNS = [
    # rustfmt adds a trailing comma when a call spans several lines.
    re.compile(r"\bl\(\s*" + STR + r"\s*,\s*" + STR + r"\s*,?\s*\)", re.S),
    re.compile(r"Text::new\(\s*" + STR + r"\s*,\s*" + STR + r"\s*,?\s*\)", re.S),
    re.compile(r"\bde:\s*" + STR + r"\s*,\s*en:\s*" + STR, re.S),
    re.compile(r"\bt\(\s*" + STR + r"\s*,\s*" + STR + r"\s*,?\s*\)", re.S),
    # Small local helpers that take the German and the English text.
    re.compile(r"\b(?:bi|tr|text|txt|both|de_en|bilingual)\(\s*" + STR + r"\s*,\s*" + STR + r"\s*,?\s*\)", re.S),
]


def unescape(text: str) -> str:
    return json.loads('"' + text + '"')


def rust_texts() -> dict[str, str]:
    out: dict[str, str] = {}
    files = glob.glob(os.path.join(ROOT, "crates", "**", "*.rs"), recursive=True)
    files += glob.glob(os.path.join(ROOT, "app", "src-tauri", "src", "**", "*.rs"), recursive=True)
    for path in files:
        if any(f"{os.sep}{part}{os.sep}" in path for part in ("tests", "target", "caruspex-i18n")):
            continue
        text = open(path, encoding="utf-8").read()
        for pattern in PATTERNS:
            for de, en in pattern.findall(text):
                try:
                    de, en = unescape(de), unescape(en)
                except json.JSONDecodeError:
                    continue
                if en.strip():
                    out.setdefault(en, de)
    return out


def main() -> int:
    source_dir = os.path.join(ROOT, "translations", "source")
    known: set[str] = set()
    ids: list[int] = []
    for path in sorted(glob.glob(os.path.join(source_dir, "core_*.json"))):
        for item in json.load(open(path, encoding="utf-8")):
            known.add(item["en"])
            ids.append(int(item["id"][1:]))
    missing = {en: de for en, de in rust_texts().items() if en not in known}
    for en, de in sorted(missing.items()):
        print(f"{de!r} / {en!r}")
    print(f"{len(missing)} texts without translation", file=sys.stderr)
    if "--write" in sys.argv and missing:
        path = os.path.join(source_dir, "core_x2.json")
        items = json.load(open(path, encoding="utf-8")) if os.path.exists(path) else []
        next_id = max(ids) + 1
        for en, de in sorted(missing.items()):
            items.append({"id": f"c{next_id}", "de": de, "en": en})
            next_id += 1
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            json.dump(items, f, ensure_ascii=False, indent=1)
            f.write("\n")
    if "--strict" in sys.argv:
        gaps = missing_translations()
        for line in gaps:
            print(line)
        if missing or gaps:
            print("Translate the new texts (translations/TRANSLATING.md), see --write.", file=sys.stderr)
            return 1
    return 0


def missing_translations() -> list[str]:
    """Ids of translations/source/core_*.json without a text in some language."""
    source_dir = os.path.join(ROOT, "translations", "source")
    ids: set[str] = set()
    for path in glob.glob(os.path.join(source_dir, "core_*.json")):
        ids |= {item["id"] for item in json.load(open(path, encoding="utf-8"))}
    gaps = []
    for lang_dir in sorted(glob.glob(os.path.join(ROOT, "translations", "*", ""))):
        code = os.path.basename(os.path.dirname(lang_dir))
        if code == "source":
            continue
        have: set[str] = set()
        for path in glob.glob(os.path.join(lang_dir, "core_*.json")):
            have |= {k for k, v in json.load(open(path, encoding="utf-8")).items() if v}
        lost = ids - have
        if lost:
            gaps.append(f"{code}: {len(lost)} core texts without translation")
    return gaps


if __name__ == "__main__":
    sys.exit(main())
