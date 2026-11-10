"""Collects every text that needs translating into translations/source/.

    py -I tools/i18n_extract.py

core_NN.json   texts of the Qt-free core: every L("de", "en") and every
               Text{"de", "en"} in src/core and src/data, the generated data
               included. Keyed by the English text, which is what the
               catalog looks up at run time.
ui_NN.json     texts of the interface from translations/caruspex_en.ts
               (German source, English translation), keyed by context and
               source as Qt does.

Both are split into chunks of at most CHUNK entries so a translator (person
or tool) can work on one file at a time. A translation of chunk core_03.json
goes to translations/<code>/core_03.json as {"id": "text", ...}; for the
interface the same with ui_NN.json, plural messages as a list of forms.
"""

from __future__ import annotations

import glob
import json
import os
import re
import xml.etree.ElementTree as ET

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "translations", "source")
CHUNK = 250

ESCAPES = {"n": "\n", "t": "\t", "r": "\r", "\\": "\\", '"': '"', "'": "'", "0": "\0"}


def read_literal(text: str, at: int) -> tuple[str, int] | None:
    """A C++ string literal starting at text[at] == '"'. Returns value and end."""
    if text[at] != '"':
        return None
    out = bytearray()
    i = at + 1
    while i < len(text):
        c = text[i]
        if c == '"':
            return out.decode("utf-8", errors="replace"), i + 1
        if c == "\\":
            n = text[i + 1]
            if n == "x":
                j = i + 2
                while j < len(text) and text[j] in "0123456789abcdefABCDEF":
                    j += 1
                out.append(int(text[i + 2:j], 16) & 0xFF)
                i = j
                continue
            out += ESCAPES.get(n, n).encode("utf-8")
            i += 2
            continue
        out += c.encode("utf-8")
        i += 1
    return None


def skip_space(text: str, at: int) -> int:
    while at < len(text):
        if text[at].isspace():
            at += 1
        elif text.startswith("//", at):
            at = text.find("\n", at)
            if at < 0:
                return len(text)
        elif text.startswith("/*", at):
            at = text.find("*/", at) + 2
        else:
            break
    return at


def read_concat(text: str, at: int) -> tuple[str, int] | None:
    """One or more adjacent literals, joined as the compiler does."""
    at = skip_space(text, at)
    parts = []
    while at < len(text) and text[at] == '"':
        lit = read_literal(text, at)
        if lit is None:
            return None
        parts.append(lit[0])
        at = skip_space(text, lit[1])
    if not parts:
        return None
    return "".join(parts), at


OPEN = re.compile(r"\bL\(|\{")


def pairs_in(text: str):
    for m in OPEN.finditer(text):
        closing = ")" if m.group(0) == "L(" else "}"
        first = read_concat(text, m.end())
        if not first:
            continue
        at = first[1]
        if at >= len(text) or text[at] != ",":
            continue
        second = read_concat(text, at + 1)
        if not second:
            continue
        at = second[1]
        if at < len(text) and text[at] == closing:
            yield first[0], second[0]


def core_texts() -> list[dict]:
    seen: dict[str, str] = {}
    files = sorted(glob.glob(os.path.join(ROOT, "src", "core", "**", "*.cpp"), recursive=True) +
                   glob.glob(os.path.join(ROOT, "src", "core", "**", "*.h"), recursive=True) +
                   glob.glob(os.path.join(ROOT, "src", "data", "*.cpp")) +
                   glob.glob(os.path.join(ROOT, "src", "data", "*.h")) +
                   [os.path.join(ROOT, "src", "cli_main.cpp")])
    for path in files:
        if path.endswith("Catalog.gen.cpp"):
            continue
        text = open(path, encoding="utf-8").read()
        for de, en in pairs_in(text):
            if not en.strip() or not re.search(r"[A-Za-z]", en):
                continue
            seen.setdefault(en, de)
    return [{"id": f"c{i}", "de": de, "en": en} for i, (en, de) in enumerate(sorted(seen.items()))]


def ui_texts() -> list[dict]:
    root = ET.parse(os.path.join(ROOT, "translations", "caruspex_en.ts")).getroot()
    out = []
    for ctx in root.findall("context"):
        name = ctx.findtext("name")
        for msg in ctx.findall("message"):
            source = msg.findtext("source") or ""
            tr = msg.find("translation")
            plural = msg.get("numerus") == "yes"
            if plural:
                en = [f.text or "" for f in tr.findall("numerusform")]
            else:
                en = tr.text or ""
            out.append({"id": f"u{len(out)}", "context": name, "de": source, "en": en, "plural": plural})
    return out


def write_chunks(prefix: str, items: list[dict]) -> int:
    for old in glob.glob(os.path.join(OUT, prefix + "_*.json")):
        os.remove(old)
    for n in range(0, len(items), CHUNK):
        path = os.path.join(OUT, f"{prefix}_{n // CHUNK:02d}.json")
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            json.dump(items[n:n + CHUNK], f, ensure_ascii=False, indent=1)
    return (len(items) + CHUNK - 1) // CHUNK


def main() -> None:
    os.makedirs(OUT, exist_ok=True)
    core = core_texts()
    ui = ui_texts()
    nc = write_chunks("core", core)
    nu = write_chunks("ui", ui)
    chars = sum(len(c["en"]) for c in core) + sum(len(u["de"]) for u in ui)
    print(f"core: {len(core)} texts in {nc} files, ui: {len(ui)} texts in {nu} files, {chars} characters")


if __name__ == "__main__":
    main()
