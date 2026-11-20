"""Writes THIRD_PARTY_NOTICES.md: the licences of everything the programs ship.

    py -I tools/gen_notices.py           write the file
    py -I tools/gen_notices.py --check   only compare (CI), exit 1 when stale

What goes in:

1. Rust: every crate that `cargo metadata` resolves as a normal (not dev, not
   build) dependency of the shipped programs caruspex-app, caruspex-cli and
   caruspex-j2534 (the 32-bit bridge), on every platform. The licence texts
   are read from the crate sources in the Cargo registry, which
   `cargo metadata` downloads.
2. The interface: the runtime packages of app/package-lock.json (not the
   dev tools) plus the bundler, whose small helper functions end up in the
   built JavaScript. The texts come from app/node_modules, so run
   `npm --prefix app ci` first.
3. Android: the libraries in app/src-tauri/gen/android/app/build.gradle.kts.
   Their licences are listed in ANDROID below; an unknown library fails the
   run until somebody checks its licence and adds it.
4. The data and fonts that are built into the programs (DATA below).

Many permissive licences (MIT, BSD, ISC, Apache-2.0) ask that the copyright
notice and the licence text travel with binary copies. The app shows this
file under Settings (app/scripts/copy-fonts.mjs puts it into the interface),
and tools/package.sh puts it next to LICENSE in every package.

For a crate under a choice of licences ("MIT OR Apache-2.0") the first one
in PREFERENCE is used and its text reproduced. Identical texts are printed
once with the list of components that use them.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TARGET = os.path.join(ROOT, "THIRD_PARTY_NOTICES.md")
APP = os.path.join(ROOT, "app")
GRADLE = os.path.join(APP, "src-tauri", "gen", "android", "app", "build.gradle.kts")
EXTRA = os.path.join(ROOT, "tools", "notices")

SHIPPED = ["caruspex-app", "caruspex-cli", "caruspex-j2534"]
# Bundler packages whose runtime helpers can end up in app/dist.
BUNDLERS = ["vite", "rolldown"]

# Which licence of a choice is reproduced, most preferred first.
PREFERENCE = [
    "MIT", "Apache-2.0", "BSD-3-Clause", "BSD-2-Clause", "ISC", "Zlib", "0BSD", "MIT-0",
    "Unlicense", "CC0-1.0", "Unicode-3.0", "BSL-1.0", "MPL-2.0", "CDLA-Permissive-2.0",
    "Apache-2.0 WITH LLVM-exception",
]
# Licences that can be shipped in a binary of an MIT program. MPL-2.0 is
# file based copyleft: allowed, but the source of those crates has to stay
# available, which crates.io does (said in the header of the output).
ALLOWED = set(PREFERENCE)

# Android libraries: Maven group (or group:artifact) -> (licence, source).
# Checked by hand in the POM files of the Gradle cache and on the project
# pages; the AndroidX and Kotlin libraries these pull in are Apache-2.0 too.
ANDROID = {
    "androidx.": ("Apache-2.0", "https://developer.android.com/jetpack/androidx"),
    "com.google.android.material:material": ("Apache-2.0", "https://github.com/material-components/material-components-android"),
    "com.github.mik3y:usb-serial-for-android": ("MIT", "https://github.com/mik3y/usb-serial-for-android"),
}
ANDROID_TEXTS = {"com.github.mik3y:usb-serial-for-android": "usb-serial-for-android-LICENSE.txt"}

# Data and fonts compiled into the programs, with what their licence asks.
DATA = """\
## Data and fonts

These are part of the programs as well. Their licence texts are in the source
code next to the data, the MIT texts of opendbc, model3dbc, OVMS and evDash
also at the end of this file; the CC BY-SA 4.0 licence is at
https://creativecommons.org/licenses/by-sa/4.0/legalcode, the SIL Open Font
License 1.1 at https://openfontlicense.org.

| What | From | Licence | In the source code |
|---|---|---|---|
| Manufacturer values (PID packs) for about 550 vehicles | OBDb, https://github.com/OBDb, by its contributors; converted and compressed by tools/gen_obdb.py | CC BY-SA 4.0 | data/obdb/ (LICENSE, ATTRIBUTION.md with every repository and commit) |
| Battery value packs for the Hyundai Ioniq, Kona, Kia e-Niro, Nissan Leaf and Renault Zoe | OBDb (CC BY-SA 4.0), Open Vehicle Monitoring System, https://github.com/openvehicles/Open-Vehicle-Monitoring-System-3, and evDash, https://github.com/nickn17/evDash (both MIT); written as Torque equations | CC BY-SA 4.0 (adapted from OBDb); the OVMS and evDash parts MIT, Copyright (c) 2011-2017 Open Vehicles and the authors named in LICENSE-OVMS, Copyright (c) 2020 NiCK.n17 | data/pids/ (ATTRIBUTION.md, LICENSE-CC-BY-SA-4.0, LICENSE-OVMS, LICENSE-evDash) |
| CAN signal definitions | opendbc by comma.ai, https://github.com/commaai/opendbc; converted by tools/gen_opendbc.py | MIT, Copyright (c) 2020, Comma.ai, Inc. | data/opendbc/ (LICENSE) |
| Tesla Model 3 and Y signal definitions | model3dbc, https://github.com/joshwardell/model3dbc | MIT, Copyright (c) 2019 Josh Wardell | data/tesla/LICENSE-model3dbc, data/pids/tesla-model3-y.csv |
| Engine codes per model generation | Wikipedia articles, the article and revision are named in every row | CC BY-SA 4.0 | data/engines/ (LICENSE) |
| World manufacturer identifiers (VIN) | NHTSA vPIC, https://vpic.nhtsa.dot.gov (US government work), and Wikidata | public domain, CC0 1.0 | crates/caruspex-vehicles/src/vin.rs |
| The world manufacturer identifiers that vPIC and Wikidata do not have | the WMI list on Wikibooks, https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_(VIN_codes)/World_Manufacturer_Identifier_(WMI), revision 4682957, by the Wikibooks contributors | CC BY-SA 4.0 | data/wmi/ (LICENSE, ATTRIBUTION.md) |
| Control unit addresses per brand | own compilation, confirmed by opendbc, OVMS, evDash (MIT) and OBDb (addresses only, as facts); named per row | MIT | data/ecus/ |
| Manufacturer fault codes and generic fault codes | own compilation and wording; generic numbering after SAE J2012 / ISO 15031-6 | MIT | tools/dtc/ |
| Outer dimensions per model generation | Wikidata, https://www.wikidata.org | CC0 1.0 | data/wikidata/ |
| Fonts JetBrains Mono, Inter, Plus Jakarta Sans | https://github.com/JetBrains/JetBrainsMono, https://github.com/rsms/inter, https://github.com/tokotype/PlusJakartaSans | SIL Open Font License 1.1 | resources/fonts/ (OFL.txt, OFL-Inter.txt, OFL-PlusJakartaSans.txt; shipped next to the fonts in the app) |

The adapted data under CC BY-SA 4.0 (the OBDb packs, the battery value packs
in data/pids, the engine codes and the WMI codes in data/wmi) is itself
licensed under CC BY-SA 4.0, not under the MIT licence of the code.
"""


class Failure(Exception):
    pass


def norm(text: str) -> str:
    """Line endings, BOM and trailing blanks only; the words stay as they are."""
    text = text.replace("\r\n", "\n").replace("\r", "\n").lstrip("﻿")
    lines = [line.rstrip() for line in text.split("\n")]
    while lines and not lines[0]:
        lines.pop(0)
    while lines and not lines[-1]:
        lines.pop()
    return "\n".join(lines) + "\n"


def kind_of(name: str, text: str) -> str | None:
    """Which licence a file holds, from its words first, then its name."""
    # Whitespace folded, the phrases below are often wrapped across lines.
    head = re.sub(r"\s+", " ", text[:4000].lower())
    upper = name.upper()
    checks = [
        ("Apache-2.0", "apache license" in head and "version 2.0" in head),
        ("MPL-2.0", "mozilla public license" in head),
        ("CDLA-Permissive-2.0", "community data license agreement" in head),
        ("Unicode-3.0", "unicode license" in head or "unicode, inc." in head),
        ("Unlicense", "this is free and unencumbered software" in head),
        ("CC0-1.0", "cc0 1.0" in head or "creative commons legal code" in head),
        ("BSL-1.0", "boost software license" in head),
        ("Zlib", "this software is provided 'as-is'" in head or "altered source versions must be plainly marked" in head),
        ("ISC", "permission to use, copy, modify, and/or distribute" in head
         or "permission to use, copy, modify, and distribute this software for any" in head),
        ("MIT-0", "permission is hereby granted" in head and "subject to the following conditions" not in head),
        ("MIT", "permission is hereby granted" in head),
        ("BSD-3-Clause", "redistribution and use in source and binary forms" in head
         and ("neither the name" in head or "contributors may be used" in head)),
        ("BSD-2-Clause", "redistribution and use in source and binary forms" in head),
    ]
    for kind, hit in checks:
        if hit:
            return kind
    for key, kind in [("APACHE", "Apache-2.0"), ("MIT", "MIT"), ("BSD", "BSD-3-Clause"), ("ZLIB", "Zlib"),
                      ("UNLICENSE", "Unlicense"), ("ISC", "ISC"), ("MPL", "MPL-2.0")]:
        if key in upper:
            return kind
    return None


LICENSE_FILE = re.compile(r"^(LICEN[CS]E|COPYING|COPYRIGHT|NOTICE|UNLICENSE)", re.I)


def licence_files(folder: str) -> list[tuple[str, str]]:
    """(file name, text) of the licence and notice files in a package folder."""
    found = []
    if not os.path.isdir(folder):
        return found
    for name in sorted(os.listdir(folder)):
        path = os.path.join(folder, name)
        if not os.path.isfile(path) or not LICENSE_FILE.match(name):
            continue
        if name.lower().endswith((".spdx", ".json", ".toml", ".rs", ".js", ".ts")):
            continue
        with open(path, "rb") as f:
            raw = f.read()
        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError:
            text = raw.decode("latin-1")
        found.append((name, norm(text)))
    return found


def terms(expression: str) -> list[list[str]]:
    """'(MIT OR Apache-2.0) AND Unicode-3.0' -> [['MIT', 'Apache-2.0'], ['Unicode-3.0']].
    Old style 'MIT/Apache-2.0' is read as OR."""
    expression = expression.replace("/", " OR ").replace("(", " ").replace(")", " ")
    groups = []
    for part in re.split(r"\s+AND\s+", expression.strip()):
        options = [o.strip() for o in re.split(r"\s+OR\s+", part) if o.strip()]
        groups.append(options)
    return groups


def ranked(expression: str) -> list[list[str]]:
    """Per AND group the allowed options, most preferred first."""
    groups = []
    for options in terms(expression):
        allowed = sorted((o for o in options if o in ALLOWED), key=PREFERENCE.index)
        if not allowed:
            raise Failure(f"no allowed licence in '{expression}'")
        groups.append(allowed)
    return groups


def pick_texts(expression: str, files: list[tuple[str, str]], holders: str) -> list[tuple[str, str]]:
    """The texts to reproduce for one package.

    Per AND group the most preferred option the package ships a file for.
    When no file matches any option (one combined LICENSE, a COPYRIGHT file
    pointing elsewhere), every licence file of the package is reproduced, so
    nothing the authors wrote is lost. A package without any file gets the
    standard text. NOTICE files always go along (Apache-2.0 section 4d).
    A package under several licences at once gets all its files."""
    picked: list[tuple[str, str]] = []
    classified = [(n, t, kind_of(n, t)) for n, t in files if not n.upper().startswith("NOTICE")]
    groups = ranked(expression)
    if len(groups) > 1 and files:
        # Several licences at once (ring: Apache-2.0 AND ISC, with extra
        # files for the BoringSSL parts): every file the package ships.
        return list(files)
    for options in groups:
        for option in options:
            base = option.split(" WITH ")[0]
            hits = [(n, t) for n, t, k in classified if k == base]
            if hits:
                break
        else:
            hits = [(n, t) for n, t, _ in classified] or [(options[0], standard_text(options[0], holders))]
        picked.extend(h for h in hits if h not in picked)
    picked.extend((n, t) for n, t in files if n.upper().startswith("NOTICE") and (n, t) not in picked)
    return picked


def standard_text(licence: str, holders: str) -> str:
    """For packages that ship no licence file: the standard text, with the
    authors from the package metadata as holders where the text has a slot."""
    base = licence.split(" WITH ")[0]
    path = os.path.join(EXTRA, f"{base}.txt")
    if not os.path.exists(path):
        raise Failure(f"no licence file and no standard text tools/notices/{base}.txt")
    text = norm(open(path, encoding="utf-8").read())
    if base in ("MIT", "BSD-3-Clause") and holders:
        text = f"Copyright (c) {holders}\n\n" + text
    return text


def version_key(version: str) -> list:
    """0.8.0 before 0.11.1."""
    return [(0, int(part), "") if part.isdigit() else (1, 0, part) for part in re.split(r"[.+-]", version)]


class Notices:
    def __init__(self) -> None:
        self.rows: list[tuple[str, str, str, str, str]] = []  # group, name, version, licence, source
        self.texts: dict[str, tuple[str, list[str]]] = {}  # key -> (text, users)
        self.order: list[str] = []

    def add(self, group: str, name: str, version: str, licence: str, source: str,
            texts: list[tuple[str, str]]) -> None:
        self.rows.append((group, name, version, licence, source))
        label = f"{name} {version}".strip()
        for _, text in texts:
            key = hashlib.sha256(re.sub(r"\s+", " ", text).strip().encode("utf-8")).hexdigest()
            if key not in self.texts:
                self.texts[key] = (text, [])
                self.order.append(key)
            if label not in self.texts[key][1]:
                self.texts[key][1].append(label)

    def render(self) -> str:
        out = [
            "# Third party notices",
            "",
            "Written by `tools/gen_notices.py` from Cargo.lock, app/package-lock.json and the",
            "Android build file. Do not edit by hand; run the script after changing dependencies.",
            "",
            "Caruspex itself is under the MIT licence (see LICENSE). The programs (the app on",
            "Windows, Linux, macOS and Android, caruspex_cli and caruspex-j2534-bridge) also",
            "contain the components below. Each keeps its own licence; the full texts follow",
            "the list. Where a component offers a choice of licences, Caruspex uses it under",
            "the one whose text is reproduced here.",
            "",
            "Components under the Mozilla Public License 2.0 are used unchanged. Their source",
            "code is available from the source named in the list (crates.io), as section 3.2",
            "of that licence asks.",
            "",
            "A few packages ship no licence file of their own. For those the standard text",
            "of the licence they declare is printed, with the authors named in the package",
            "as the holders.",
            "",
            "On Linux the app uses the system libraries of the distribution (GTK, WebKitGTK,",
            "D-Bus, udev), on Windows Microsoft WebView2, on macOS WebKit; these are not part",
            "of the Caruspex packages. The Windows installer carries Microsoft's WebView2",
            "bootstrapper and runs it only when the runtime is missing.",
            "",
            DATA.rstrip("\n"),
            "",
        ]
        groups = [("rust", "## Rust crates"), ("npm", "## Interface (JavaScript)"), ("android", "## Android libraries")]
        for group, title in groups:
            rows = sorted((r for r in self.rows if r[0] == group), key=lambda r: (r[1].lower(), version_key(r[2])))
            out += [title, "", f"{len(rows)} components.", "", "| Component | Version | Licence | Source |", "|---|---|---|---|"]
            for _, name, version, licence, source in rows:
                out.append(f"| {name} | {version} | {licence} | {source} |")
            out.append("")
        out += ["## Licence texts", ""]
        for number, key in enumerate(self.order, 1):
            text, users = self.texts[key]
            out.append(f"### Text {number}")
            out.append("")
            out.append("Used by: " + ", ".join(sorted(users, key=str.lower)) + ".")
            out.append("")
            fence = "~~~~" if "```" in text else "```"
            out.append(fence + "text")
            out.append(text.rstrip("\n"))
            out.append(fence)
            out.append("")
        return "\n".join(out)


def rust(notices: Notices) -> None:
    raw = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=ROOT, check=True, capture_output=True,
    ).stdout
    meta = json.loads(raw)
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    workspace = set(meta["workspace_members"])
    roots = [i for i in workspace if packages[i]["name"] in SHIPPED]
    if len(roots) != len(SHIPPED):
        raise Failure("a shipped crate is missing from the workspace")
    seen, todo = set(), list(roots)
    while todo:
        current = todo.pop()
        if current in seen:
            continue
        seen.add(current)
        for dep in nodes[current]["deps"]:
            if any(k["kind"] is None for k in dep["dep_kinds"]):
                todo.append(dep["pkg"])
    # Sorted, so the numbering of the texts does not depend on set order.
    for package_id in sorted(seen, key=lambda i: (packages[i]["name"], version_key(packages[i]["version"]))):
        if package_id in workspace:
            continue
        p = packages[package_id]
        expression = p.get("license")
        folder = os.path.dirname(p["manifest_path"])
        if not expression:
            if not p.get("license_file"):
                raise Failure(f"crate {p['name']} {p['version']} has no licence field")
            files = [(os.path.basename(p["license_file"]), norm(open(os.path.join(folder, p["license_file"]), encoding="utf-8").read()))]
            expression = "see licence file"
            texts = files
        else:
            holders = ", ".join(a.split(" <")[0] for a in p.get("authors") or [])
            texts = pick_texts(expression, licence_files(folder), holders)
        source = p.get("repository") or f"https://crates.io/crates/{p['name']}"
        notices.add("rust", p["name"], p["version"], expression, source, texts)


def npm(notices: Notices) -> None:
    lock = json.load(open(os.path.join(APP, "package-lock.json"), encoding="utf-8"))
    for path, info in lock["packages"].items():
        if not path.startswith("node_modules/"):
            continue
        name = path[len("node_modules/"):]
        if "node_modules/" in name:
            continue  # nested copies are dev tools here; checked below
        if info.get("dev") and name not in BUNDLERS:
            continue
        folder = os.path.join(APP, path)
        if not os.path.isdir(folder):
            raise Failure(f"{path} is not installed, run npm --prefix app ci first")
        expression = info.get("license") or json.load(open(os.path.join(folder, "package.json"), encoding="utf-8")).get("license")
        if not expression:
            raise Failure(f"npm package {name} has no licence")
        texts = pick_texts(expression, licence_files(folder), "")
        notices.add("npm", name, info.get("version", ""), expression, f"https://www.npmjs.com/package/{name}", texts)
    for path, info in lock["packages"].items():
        if path.count("node_modules/") > 1 and not info.get("dev"):
            raise Failure(f"nested runtime package {path}: teach tools/gen_notices.py about it")


def data(notices: Notices) -> None:
    """The MIT texts of the data compiled in (the table in DATA lists them;
    their copyright lines differ from the other MIT texts)."""
    for label, rel in [("opendbc (data)", "data/opendbc/LICENSE"), ("model3dbc (data)", "data/tesla/LICENSE-model3dbc"),
                       ("Open Vehicle Monitoring System (data)", "data/pids/LICENSE-OVMS"),
                       ("evDash (data)", "data/pids/LICENSE-evDash")]:
        text = norm(open(os.path.join(ROOT, rel), encoding="utf-8").read())
        notices.add("data", label, "", "MIT", rel, [(rel, text)])


def android(notices: Notices) -> None:
    text = open(GRADLE, encoding="utf-8").read()
    apache = standard_text("Apache-2.0", "")
    for coordinate in re.findall(r'^\s*implementation\("([^"]+)"\)', text, re.M):
        group, artifact, version = coordinate.split(":")
        key = f"{group}:{artifact}"
        entry = ANDROID.get(key) or next((v for k, v in ANDROID.items() if k.endswith(".") and group.startswith(k)), None)
        if not entry:
            raise Failure(f"Android library {key}: check its licence and add it to ANDROID in tools/gen_notices.py")
        licence, source = entry
        if licence == "Apache-2.0":
            texts = [("LICENSE", apache)]
        else:
            name = ANDROID_TEXTS[key]
            texts = [(name, norm(open(os.path.join(EXTRA, name), encoding="utf-8").read()))]
        notices.add("android", key, version, licence, source, texts)
    notices.add("android", "Kotlin standard library and kotlinx", "", "Apache-2.0",
                "https://github.com/JetBrains/kotlin", [("LICENSE", apache)])


def main() -> int:
    check = "--check" in sys.argv
    notices = Notices()
    try:
        rust(notices)
        npm(notices)
        android(notices)
        data(notices)
    except (Failure, subprocess.CalledProcessError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    new = notices.render().encode("utf-8")
    old = open(TARGET, "rb").read() if os.path.exists(TARGET) else b""
    # A Windows checkout with core.autocrlf has CRLF; keep what the file has.
    if b"\r\n" in old:
        new = new.replace(b"\n", b"\r\n")
    if check:
        if old != new:
            print("THIRD_PARTY_NOTICES.md is out of date, run tools/gen_notices.py")
            return 1
        print("THIRD_PARTY_NOTICES.md is current")
        return 0
    with open(TARGET, "wb") as f:
        f.write(new)
    print(f"wrote THIRD_PARTY_NOTICES.md: {len(notices.rows)} components, {len(notices.order)} licence texts, {len(new) // 1024} KiB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
