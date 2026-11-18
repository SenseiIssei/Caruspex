"""Fills in the package manager manifests for one release (winget, Homebrew,
Flathub, Snap) from the templates in packaging/.

    py -I tools/dist_manifests.py 0.14.0               download the release files
    py -I tools/dist_manifests.py 0.14.0 --from DIR    take them from a folder (e.g. dist/)
    py -I tools/dist_manifests.py 0.14.0 --out DIR     write somewhere else
    py -I tools/dist_manifests.py --check-templates    offline check of the templates (CI)

It downloads the Windows installer, the macOS dmg and the Linux .deb of the
GitHub release v<version> (or reads them from --from), computes their
SHA-256, takes the date from the release's latest.json (else today) and the
release notes from changelog/<version>.md, and writes ready-to-submit files
to dist/manifests/<version>/ (not checked in):

    winget/manifests/s/SenseiIssei/Caruspex/<version>/   the four YAML files for winget-pkgs
    homebrew/Casks/caruspex.rb                           for a tap repository
    flathub/                                             manifest, desktop file, metainfo
    snap/                                                snapcraft.yaml, desktop file, metainfo

Nothing is submitted anywhere; packaging/README.md says how the owner does it.
"""

from __future__ import annotations

import datetime
import hashlib
import json
import os
import re
import shutil
import sys
import tempfile
import urllib.request
import xml.etree.ElementTree as ET

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PACKAGING = os.path.join(ROOT, "packaging")
DOWNLOAD = "https://github.com/SenseiIssei/Caruspex/releases/download/v{version}/{name}"

# Placeholder -> release file whose SHA-256 it gets.
FILES = {
    "SHA256_WINDOWS_SETUP": "Caruspex-{version}-windows-x64-setup.exe",
    "SHA256_MACOS_DMG": "Caruspex-{version}-macos.dmg",
    "SHA256_LINUX_DEB": "Caruspex-{version}-linux-amd64.deb",
}

DESKTOP = "linux/dev.senseiissei.caruspex.desktop"
METAINFO = "linux/dev.senseiissei.caruspex.metainfo.xml"

# Template in packaging/ -> place in the output, with {version}.
OUTPUTS = {
    "winget/SenseiIssei.Caruspex.yaml": "winget/manifests/s/SenseiIssei/Caruspex/{version}/SenseiIssei.Caruspex.yaml",
    "winget/SenseiIssei.Caruspex.installer.yaml": "winget/manifests/s/SenseiIssei/Caruspex/{version}/SenseiIssei.Caruspex.installer.yaml",
    "winget/SenseiIssei.Caruspex.locale.en-US.yaml": "winget/manifests/s/SenseiIssei/Caruspex/{version}/SenseiIssei.Caruspex.locale.en-US.yaml",
    "winget/SenseiIssei.Caruspex.locale.de-DE.yaml": "winget/manifests/s/SenseiIssei/Caruspex/{version}/SenseiIssei.Caruspex.locale.de-DE.yaml",
    "homebrew/Casks/caruspex.rb": "homebrew/Casks/caruspex.rb",
    "flathub/dev.senseiissei.caruspex.yml": "flathub/dev.senseiissei.caruspex.yml",
    DESKTOP: "flathub/dev.senseiissei.caruspex.desktop",
    METAINFO: "flathub/dev.senseiissei.caruspex.metainfo.xml",
    "snap/snapcraft.yaml": "snap/snapcraft.yaml",
}
# The snap build takes desktop file and metainfo from its own folder too.
EXTRA_COPIES = {
    "flathub/dev.senseiissei.caruspex.desktop": "snap/dev.senseiissei.caruspex.desktop",
    "flathub/dev.senseiissei.caruspex.metainfo.xml": "snap/dev.senseiissei.caruspex.metainfo.xml",
}

PLACEHOLDER = re.compile(r"@([A-Z0-9_]+)@")


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def fetch(url: str, path: str) -> None:
    request = urllib.request.Request(url, headers={"User-Agent": "caruspex-dist-manifests"})
    with urllib.request.urlopen(request, timeout=120) as response, open(path, "wb") as out:
        shutil.copyfileobj(response, out)


def sha256(path: str) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def release_notes_xml(version: str) -> str:
    """The English part of changelog/<version>.md as AppStream markup."""
    path = os.path.join(ROOT, "changelog", f"{version}.md")
    if not os.path.isfile(path):
        return "        <p>Caruspex %s</p>" % version
    text = open(path, encoding="utf-8").read()
    english = text.split("## English", 1)[1] if "## English" in text else text
    lines: list[str] = []
    items: list[str] = []

    def plain(s: str) -> str:
        s = re.sub(r"\*\*([^*]+)\*\*", r"\1", s)
        s = re.sub(r"`([^`]+)`", r"\1", s)
        s = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", s)
        return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").strip()

    def flush_items() -> None:
        if items:
            lines.append("        <ul>")
            lines.extend(f"          <li>{item}</li>" for item in items)
            lines.append("        </ul>")
            items.clear()

    for raw in english.strip().splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            flush_items()
            continue
        if line.startswith("- "):
            items.append(plain(line[2:]))
        else:
            flush_items()
            lines.append(f"        <p>{plain(line)}</p>")
    flush_items()
    return "\n".join(lines)


def fill(text: str, values: dict[str, str], where: str) -> str:
    def replace(match: re.Match[str]) -> str:
        key = match.group(1)
        if key not in values:
            fail(f"{where}: unknown placeholder @{key}@")
        return values[key]

    return PLACEHOLDER.sub(replace, text)


def check_output(path: str) -> None:
    """Cheap offline checks of what was written."""
    text = open(path, encoding="utf-8").read()
    if PLACEHOLDER.search(text):
        fail(f"{path}: a placeholder is left over")
    if chr(0x2014) in text or chr(0x2013) in text:
        fail(f"{path}: long dash")
    if path.endswith(".xml"):
        root = ET.parse(path).getroot()
        if root.findtext("id") != "dev.senseiissei.caruspex":
            fail(f"{path}: wrong component id")
        release = root.find("releases/release")
        if release is None or not re.fullmatch(r"\d+\.\d+\.\d+", release.get("version", "")):
            fail(f"{path}: no release version")
        if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", release.get("date", "")):
            fail(f"{path}: no release date")
    if path.endswith(".desktop"):
        lines = text.splitlines()
        if lines[0] != "[Desktop Entry]":
            fail(f"{path}: must start with [Desktop Entry]")
        keys = {line.split("=", 1)[0] for line in lines[1:] if "=" in line}
        for key in ("Type", "Name", "Exec", "Icon", "Categories"):
            if key not in keys:
                fail(f"{path}: {key} is missing")
    if "sha256" in text.lower():
        for found in re.findall(r"(?i)sha256[^0-9a-f]{1,20}([0-9a-f]{20,})", text):
            if len(found) != 64:
                fail(f"{path}: a SHA-256 has {len(found)} instead of 64 digits")


def write(version: str, values: dict[str, str], out: str) -> list[str]:
    written = []
    for template, target in OUTPUTS.items():
        source = os.path.join(PACKAGING, template)
        text = open(source, encoding="utf-8").read()
        path = os.path.join(out, target.format(version=version))
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(fill(text, values, template))
        written.append(path)
    for source, target in EXTRA_COPIES.items():
        path = os.path.join(out, target)
        shutil.copyfile(os.path.join(out, source), path)
        written.append(path)
    for path in written:
        check_output(path)
    return written


def check_templates() -> int:
    """Fills every template with fake values and checks the result (offline)."""
    values = {key: "0" * 64 for key in FILES}
    values.update(VERSION="0.0.0", RELEASE_DATE="2000-01-01", RELEASE_NOTES=release_notes_xml("0.0.0"))
    with tempfile.TemporaryDirectory() as out:
        written = write("0.0.0", values, out)
    print(f"templates ok ({len(written)} files)")
    return 0


def main() -> int:
    arguments = sys.argv[1:]
    if "--check-templates" in arguments:
        return check_templates()
    source_dir = out = None
    if "--from" in arguments:
        at = arguments.index("--from")
        source_dir = os.path.abspath(arguments[at + 1])
        del arguments[at : at + 2]
    if "--out" in arguments:
        at = arguments.index("--out")
        out = os.path.abspath(arguments[at + 1])
        del arguments[at : at + 2]
    if len(arguments) != 1 or not re.fullmatch(r"\d+\.\d+\.\d+", arguments[0]):
        print(__doc__, file=sys.stderr)
        return 2
    version = arguments[0]
    out = out or os.path.join(ROOT, "dist", "manifests", version)

    values = {"VERSION": version}
    with tempfile.TemporaryDirectory() as downloads:
        for key, pattern in FILES.items():
            name = pattern.format(version=version)
            if source_dir:
                path = os.path.join(source_dir, name)
                if not os.path.isfile(path):
                    fail(f"{path} is missing")
            else:
                path = os.path.join(downloads, name)
                print(f"downloading {name}")
                fetch(DOWNLOAD.format(version=version, name=name), path)
            values[key] = sha256(path)
            print(f"{name}  {values[key]}")

        manifest = None
        if source_dir and os.path.isfile(os.path.join(source_dir, "latest.json")):
            manifest = json.load(open(os.path.join(source_dir, "latest.json"), encoding="utf-8"))
        elif not source_dir:
            path = os.path.join(downloads, "latest.json")
            try:
                fetch(DOWNLOAD.format(version=version, name="latest.json"), path)
                manifest = json.load(open(path, encoding="utf-8"))
            except OSError as error:
                print(f"latest.json not available ({error}), using today's date")
    if manifest is not None and manifest.get("version") != version:
        fail(f"latest.json is for version {manifest.get('version')}, not {version}")
    date = (manifest or {}).get("pub_date", "")[:10] or datetime.date.today().isoformat()
    values["RELEASE_DATE"] = date
    values["RELEASE_NOTES"] = release_notes_xml(version)

    if os.path.isdir(out):
        shutil.rmtree(out)
    for path in write(version, values, out):
        print("wrote " + os.path.relpath(path, ROOT if path.startswith(ROOT) else out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
