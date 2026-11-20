"""Checks the size of the built interface against a budget.

    py -I tools/bundle_budget.py              after npm --prefix app run build
    py -I tools/bundle_budget.py --dist DIR   another build directory

The interface loads one entry chunk at start (app/dist/index.html names it);
everything else comes later or never: three.js with the X-ray page, the
catalog of a language when it is chosen, the demo backend only in a plain
browser. The budgets below are generous on purpose: they catch a big
dependency or a data file that slipped into the start, not a few kilobytes.
How to look at what is in a chunk is in docs/performance.md.
"""

from __future__ import annotations

import argparse
import gzip
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
KB = 1024

# (what, raw bytes, gzip bytes). Measured in November 2026: the entry chunk
# 715 KB (229 KB gzip), its style sheet 87 KB, the largest lazy chunk (the
# X-ray scene with three.js) 651 KB.
ENTRY_JS = ("the entry chunk", 900 * KB, 300 * KB)
ENTRY_CSS = ("the style sheet", 140 * KB, 30 * KB)
LAZY = ("a chunk loaded later", 1200 * KB, 320 * KB)
# Every file of the build together, fonts and the language catalogs included
# (14.1 MB; the app carries all of it inside its program file).
TOTAL = ("the whole build", 20 * 1024 * KB, None)

# Modules that must never be part of the start: the demo backend with its
# fault code table is only for a plain browser (app/src/api/index.ts).
NOT_AT_START = re.compile(r"/assets/(mock|dtc|catalog|scene|core-)[-.]")


def gzipped(data: bytes) -> int:
    return len(gzip.compress(data, compresslevel=9))


def human(n: int) -> str:
    return f"{n / KB:.0f} KB"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dist", default=os.path.join(ROOT, "app", "dist"))
    args = parser.parse_args()

    index = os.path.join(args.dist, "index.html")
    if not os.path.exists(index):
        print(f"bundle budget: {index} is missing, run `npm --prefix app run build` first.")
        return 1
    with open(index, encoding="utf-8") as f:
        html = f.read()
    entry = set(re.findall(r'(?:src|href)="/(assets/[^"]+\.(?:js|css))"', html))

    problems: list[str] = []
    for preloaded in re.findall(r'href="(/assets/[^"]+)"', html) + re.findall(r'src="(/assets/[^"]+)"', html):
        if NOT_AT_START.search(preloaded):
            problems.append(f"{preloaded} is loaded at start; it belongs in a chunk of its own (a dynamic import).")

    rows = []
    total = 0
    for folder, _dirs, files in os.walk(args.dist):
        for name in files:
            path = os.path.join(folder, name)
            rel = os.path.relpath(path, args.dist).replace(os.sep, "/")
            if rel.endswith(".map"):
                continue  # source maps of a local analysis build are not shipped
            size = os.path.getsize(path)
            total += size
            if not rel.endswith((".js", ".css")):
                continue
            with open(path, "rb") as f:
                data = f.read()
            if rel in entry:
                what = ENTRY_JS if rel.endswith(".js") else ENTRY_CSS
            else:
                what = LAZY
            rows.append((rel, size, gzipped(data), what))

    for rel, size, packed, (what, raw_max, gzip_max) in sorted(rows, key=lambda r: -r[1]):
        if size > raw_max or (gzip_max is not None and packed > gzip_max):
            limit = human(raw_max) + (f" ({human(gzip_max)} gzip)" if gzip_max else "")
            problems.append(f"{rel} is {human(size)} ({human(packed)} gzip), {what} may have {limit}.")
    if total > TOTAL[1]:
        problems.append(f"{TOTAL[0]} is {human(total)}, the budget is {human(TOTAL[1])}.")

    start = [r for r in rows if r[0] in entry]
    print("bundle budget: at start " + ", ".join(f"{r[0]} {human(r[1])} ({human(r[2])} gzip)" for r in start))
    largest = max((r for r in rows if r[0] not in entry), key=lambda r: r[1], default=None)
    if largest:
        print(f"bundle budget: largest later chunk {largest[0]} {human(largest[1])}, whole build {human(total)}")
    if problems:
        print("bundle budget exceeded:")
        for p in problems:
            print(f"  - {p}")
        print("If the growth is intended, raise the budget in tools/bundle_budget.py and say why in the pull request;")
        print("docs/performance.md describes how to see what a chunk contains.")
        return 1
    print("bundle budget: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
