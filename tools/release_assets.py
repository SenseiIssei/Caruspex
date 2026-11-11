"""Prepares the update files of a release in one folder (the release job).

    python tools/release_assets.py <version> <folder>

The folder holds the packages of all three builds and, next to the update
packages, their signatures (<file>.sig from `tauri signer sign`). The script

1. checks every signature against the public key built into the app
   (PUBLIC_KEY in app/src-tauri/src/update.rs), so a release signed with a
   wrong key fails here instead of on the users' computers,
2. writes latest.json: version, the change list of changelog/<version>.md
   (German and English part), the date and per platform the download
   address and the signature,
3. copies the user-facing files to names without the version, so
   https://github.com/SenseiIssei/Caruspex/releases/latest/download/<name>
   always gives the newest one.

--key <base64> checks against another public key (for a local test with a
throwaway key). Needs the Python package `cryptography`.
"""

from __future__ import annotations

import base64
import datetime
import hashlib
import json
import os
import re
import shutil
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOWNLOAD = "https://github.com/SenseiIssei/Caruspex/releases/download/v{version}/{name}"

# Manifest key -> the package the app installs on that platform.
UPDATE_PACKAGES = {
    "windows-x86_64": "Caruspex-{version}-windows-x64-setup.exe",
    "linux-x86_64": "Caruspex-{version}-linux-x86_64.AppImage",
    "darwin-aarch64": "Caruspex-{version}-macos.app.tar.gz",
}

# Versioned file -> stable name for releases/latest/download/<name>.
STABLE_NAMES = {
    "Caruspex-{version}-windows-x64-setup.exe": "Caruspex-windows-x64-setup.exe",
    "Caruspex-{version}-windows-x64.zip": "Caruspex-windows-x64.zip",
    "Caruspex-{version}-macos.dmg": "Caruspex-macos.dmg",
    "Caruspex-{version}-linux-x86_64.AppImage": "Caruspex-linux-x86_64.AppImage",
    "Caruspex-{version}-linux-amd64.deb": "Caruspex-linux-amd64.deb",
    "caruspex_cli-{version}-windows-x64.zip": "caruspex_cli-windows-x64.zip",
    "caruspex_cli-{version}-linux-x86_64.tar.gz": "caruspex_cli-linux-x86_64.tar.gz",
    "caruspex_cli-{version}-macos.tar.gz": "caruspex_cli-macos.tar.gz",
}


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def embedded_public_key() -> str:
    text = open(os.path.join(ROOT, "app", "src-tauri", "src", "update.rs"), encoding="utf-8").read()
    match = re.search(r'const PUBLIC_KEY: &str = "([A-Za-z0-9+/=]+)";', text)
    if not match:
        fail("PUBLIC_KEY not found in app/src-tauri/src/update.rs")
    return match.group(1)


def minisign_lines(text_b64: str) -> list[str]:
    """The lines of a minisign file that tauri wrote as one base64 string."""
    return base64.b64decode(text_b64.strip()).decode("utf-8").strip().splitlines()


def verify(path: str, signature_b64: str, public_key_b64: str) -> None:
    """Checks a minisign signature the way the app does (minisign-verify)."""
    from cryptography.exceptions import InvalidSignature
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

    key = base64.b64decode(minisign_lines(public_key_b64)[1])
    lines = minisign_lines(signature_b64)
    if len(lines) < 4 or not lines[2].startswith("trusted comment: "):
        fail(f"{path}: the signature file is malformed")
    sig = base64.b64decode(lines[1])
    algorithm, key_id, signature = sig[:2], sig[2:10], sig[10:74]
    if key_id != key[2:10]:
        fail(f"{path}: signed with key {key_id[::-1].hex().upper()}, the app expects {key[2:10][::-1].hex().upper()}")
    if algorithm != b"ED":
        fail(f"{path}: not a prehashed signature, the app refuses it")
    digest = hashlib.blake2b(open(path, "rb").read(), digest_size=64).digest()
    public = Ed25519PublicKey.from_public_bytes(key[10:42])
    trusted = lines[2][len("trusted comment: "):].encode("utf-8")
    try:
        public.verify(signature, digest)
        public.verify(base64.b64decode(lines[3]), signature + trusted)
    except InvalidSignature:
        fail(f"{path}: the signature does not match")


def release_notes(version: str) -> str:
    path = os.path.join(ROOT, "changelog", f"{version}.md")
    notes = open(path, encoding="utf-8").read().strip()
    for part in ("## Deutsch", "## English"):
        if part not in notes:
            fail(f"{path} has no '{part}' part")
    return notes


def main() -> int:
    arguments = sys.argv[1:]
    key = embedded_public_key()
    if "--key" in arguments:
        at = arguments.index("--key")
        key = arguments[at + 1]
        del arguments[at : at + 2]
    if len(arguments) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    version, folder = arguments

    platforms = {}
    for platform, pattern in UPDATE_PACKAGES.items():
        name = pattern.format(version=version)
        path = os.path.join(folder, name)
        if not os.path.isfile(path):
            fail(f"{name} is missing")
        if not os.path.isfile(path + ".sig"):
            fail(f"{name}.sig is missing, the package was not signed")
        signature = open(path + ".sig", encoding="utf-8").read().strip()
        verify(path, signature, key)
        platforms[platform] = {"url": DOWNLOAD.format(version=version, name=name), "signature": signature}
        print(f"{platform}: {name} signature ok")

    manifest = {
        "version": version,
        "notes": release_notes(version),
        "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": platforms,
    }
    with open(os.path.join(folder, "latest.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(manifest, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print("latest.json written")

    for pattern, stable in STABLE_NAMES.items():
        name = pattern.format(version=version)
        if not os.path.isfile(os.path.join(folder, name)):
            fail(f"{name} is missing")
        shutil.copyfile(os.path.join(folder, name), os.path.join(folder, stable))
        print(f"{name} -> {stable}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
