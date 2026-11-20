"""Software bills of materials (CycloneDX 1.6, JSON) for the release packages.

    py -I tools/sbom.py --out DIR      write one SBOM per platform into DIR
    py -I tools/sbom.py --check        build them twice, validate, write nothing

One file per platform, named Caruspex-<version>-sbom-<platform>.cdx.json, so
the names never collide with the packages, latest.json or the copies without
version that the updater reads. Each SBOM lists what that platform's packages
are built from:

- the Rust crates of the workspace and every crate they use on that target,
  from `cargo metadata --locked --filter-platform` (so Cargo.lock decides the
  versions) with the sha256 of Cargo.lock; crates only needed to build
  (build scripts, proc macros) are listed with scope "excluded",
  test-only crates are left out,
- the npm packages that end up in the interface bundle (the dependencies of
  app/package.json without the dev ones), from app/package-lock.json with
  their sha512,
- the fonts in resources/fonts and the bundled data sets with their licences.

Not listed: what the operating system provides (WebView2, WebKitGTK,
WKWebView), the libraries linuxdeploy copies into the AppImage, the Inno
Setup stub and, for Android, the Gradle dependencies. docs/cra/requirements.md
says so too.

The output is deterministic: no serial number, and a timestamp only when
SOURCE_DATE_EPOCH is set (the release job sets it to the commit time).
Only the Python standard library; `cargo` must be on the PATH and is the only
thing that may need the network (to fetch the crate manifests once).
"""

from __future__ import annotations

import base64
import datetime
import hashlib
import json
import os
import re
import subprocess
import sys
import tomllib
from collections import deque

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REPO = "https://github.com/SenseiIssei/Caruspex"
CRATES_IO = "registry+https://github.com/rust-lang/crates.io-index"

# Platform name in the file name -> Rust targets whose crates go in. The
# Windows packages also carry the 32-bit J2534 bridge (tools/package.sh).
PLATFORMS = {
    "windows-x64": [("x86_64-pc-windows-msvc", None), ("i686-pc-windows-msvc", "caruspex-j2534")],
    "linux-x86_64": [("x86_64-unknown-linux-gnu", None)],
    "macos": [("aarch64-apple-darwin", None)],
    "android-arm64": [("aarch64-linux-android", None)],
}

# Fonts: file prefix -> (family, licence). All three are under the SIL OFL 1.1
# (resources/fonts/OFL*.txt).
FONTS = {"Inter-": "Inter", "JetBrainsMono-": "JetBrains Mono", "PlusJakartaSans-": "Plus Jakarta Sans"}

# Data sets built into the program, with their licence and upstream. The
# sources.json key, where there is one, gives the upstream commit.
DATA_SETS = [
    ("obdb", "OBDb vehicle signal sets (converted)", "CC-BY-SA-4.0", "data/obdb", "https://github.com/OBDb", None),
    ("opendbc", "opendbc CAN signals (converted)", "MIT", "data/opendbc", "https://github.com/commaai/opendbc", "opendbc_commit"),
    ("model3dbc", "model3dbc Tesla CAN signals (converted)", "MIT", "data/pids/tesla-model3-y.csv",
     "https://github.com/joshwardell/model3dbc", "model3dbc_commit"),
    ("engines", "Engine codes from Wikipedia", "CC-BY-SA-4.0", "data/engines", "https://en.wikipedia.org", None),
    ("wikidata", "Vehicle dimensions from Wikidata", "CC0-1.0", "data/wikidata", "https://www.wikidata.org", None),
]

COMPONENT_TYPES = {"application", "library", "framework", "file", "data"}
HASH_LENGTHS = {"SHA-256": 64, "SHA-512": 128}
PURL = re.compile(r"^pkg:(cargo|npm)/(%40[a-z0-9._-]+/)?[A-Za-z0-9._-]+@[A-Za-z0-9.+_-]+$")


def fail(message: str) -> None:
    print(f"sbom: {message}", file=sys.stderr)
    sys.exit(1)


def workspace_version() -> str:
    data = tomllib.load(open(os.path.join(ROOT, "Cargo.toml"), "rb"))
    return data["workspace"]["package"]["version"]


def cargo_metadata(target: str) -> dict:
    command = ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", target]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
    if result.returncode != 0:
        fail(f"{' '.join(command)} failed:\n{result.stderr}")
    return json.loads(result.stdout)


def lock_checksums() -> dict[tuple[str, str], str]:
    lock = tomllib.load(open(os.path.join(ROOT, "Cargo.lock"), "rb"))
    return {(p["name"], p["version"]): p["checksum"] for p in lock.get("package", []) if "checksum" in p}


def spdx(expression: str) -> str:
    # Old crates write "MIT/Apache-2.0"; SPDX says "MIT OR Apache-2.0".
    return re.sub(r"\s*/\s*", " OR ", expression.strip())


def licences(expression: str | None) -> list[dict]:
    return [{"expression": spdx(expression)}] if expression else []


def sha256_file(path: str) -> str:
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


# --- Rust ----------------------------------------------------------------------------------


def walk_rust(metadata: dict, roots: list[str]) -> tuple[set[str], set[str], dict[str, set[str]]]:
    """Runtime crates, build-only crates and the edges between them (no dev edges)."""
    packages = {p["id"]: p for p in metadata["packages"]}
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}

    def proc_macro(pid: str) -> bool:
        return any("proc-macro" in t["kind"] for t in packages[pid]["targets"])

    edges: dict[str, set[str]] = {}
    normal: dict[str, set[str]] = {}
    for pid, node in nodes.items():
        for dep in node["deps"]:
            kinds = {k["kind"] for k in dep["dep_kinds"]}
            if kinds - {"dev"}:
                edges.setdefault(pid, set()).add(dep["pkg"])
            if None in kinds:
                normal.setdefault(pid, set()).add(dep["pkg"])

    def reach(start: list[str], graph: dict[str, set[str]], stop) -> set[str]:
        seen, queue = set(start), deque(start)
        while queue:
            for nxt in graph.get(queue.popleft(), ()):
                if nxt not in seen and not stop(nxt):
                    seen.add(nxt)
                    queue.append(nxt)
        return seen

    runtime = reach(roots, normal, proc_macro)
    everything = reach(roots, edges, lambda _: False)
    return runtime, everything - runtime, edges


def rust_components(version: str, targets: list[tuple[str, str | None]]) -> tuple[list[dict], dict[str, set[str]], list[str]]:
    checksums = lock_checksums()
    components: dict[str, dict] = {}
    depends: dict[str, set[str]] = {}
    members: list[str] = []
    for target, only in targets:
        metadata = cargo_metadata(target)
        packages = {p["id"]: p for p in metadata["packages"]}
        roots = [m for m in metadata["workspace_members"] if only is None or packages[m]["name"] == only]
        runtime, build_only, edges = walk_rust(metadata, roots)

        def ref(pid: str) -> str:
            p = packages[pid]
            if p["source"] is None:
                return f"crate:{p['name']}"
            return f"pkg:cargo/{p['name']}@{p['version']}"

        for pid in sorted(runtime | build_only):
            p = packages[pid]
            r = ref(pid)
            if p["source"] is None:
                if r not in members:
                    members.append(r)
                bins = any("bin" in t["kind"] for t in p["targets"])
                component = {
                    "type": "application" if bins else "library",
                    "bom-ref": r,
                    "name": p["name"],
                    "version": version,
                    "licenses": licences(p["license"]),
                    "externalReferences": [{"type": "vcs", "url": REPO}],
                }
            else:
                if p["source"] != CRATES_IO:
                    fail(f"{p['name']} {p['version']} comes from {p['source']}, only crates.io is expected")
                component = {
                    "type": "library",
                    "bom-ref": r,
                    "name": p["name"],
                    "version": p["version"],
                    "purl": r,
                    "licenses": licences(p["license"]),
                }
                checksum = checksums.get((p["name"], p["version"]))
                if checksum:
                    component["hashes"] = [{"alg": "SHA-256", "content": checksum}]
                if p.get("repository"):
                    component["externalReferences"] = [{"type": "vcs", "url": p["repository"]}]
            # A crate that is runtime on one target stays runtime.
            scope = "required" if pid in runtime else "excluded"
            old = components.get(r)
            if old is None or old.get("scope") == "excluded":
                component["scope"] = scope
                components[r] = component
            for dep in edges.get(pid, ()):
                if dep in runtime or dep in build_only:
                    depends.setdefault(r, set()).add(ref(dep))
    return list(components.values()), depends, members


# --- npm -----------------------------------------------------------------------------------


def npm_purl(name: str, version: str) -> str:
    if name.startswith("@"):
        scope, _, rest = name.partition("/")
        return f"pkg:npm/%40{scope[1:]}/{rest}@{version}"
    return f"pkg:npm/{name}@{version}"


def npm_components() -> tuple[list[dict], dict[str, set[str]], list[str]]:
    lock = json.load(open(os.path.join(ROOT, "app", "package-lock.json"), encoding="utf-8"))
    if lock.get("lockfileVersion") != 3:
        fail("app/package-lock.json: lockfileVersion 3 expected")
    entries = lock["packages"]

    def resolve(at: str, name: str) -> str | None:
        # Node's lookup: the nearest node_modules folder upwards.
        base = at
        while True:
            candidate = (base + "/" if base else "") + "node_modules/" + name
            if candidate in entries:
                return candidate
            if "node_modules/" not in base:
                return None
            base = base[: base.rfind("/node_modules/")] if "/node_modules/" in base else ""

    root = entries[""]
    tops = [resolve("", n) for n in sorted(root.get("dependencies", {}))]
    components, depends, seen = [], {}, set()
    queue = deque(t for t in tops if t)
    while queue:
        path = queue.popleft()
        if path in seen:
            continue
        seen.add(path)
        entry = entries[path]
        name = path.rsplit("node_modules/", 1)[1]
        purl = npm_purl(name, entry["version"])
        group, _, short = name.rpartition("/")
        component = {"type": "library", "bom-ref": purl, "name": short, "version": entry["version"], "purl": purl,
                     "scope": "required", "licenses": licences(entry.get("license"))}
        if group:
            component["group"] = group
        integrity = entry.get("integrity", "")
        if integrity.startswith("sha512-"):
            component["hashes"] = [{"alg": "SHA-512", "content": base64.b64decode(integrity[7:]).hex()}]
        components.append(component)
        for dep in sorted({**entry.get("dependencies", {}), **entry.get("optionalDependencies", {})}):
            found = resolve(path, dep)
            if found:
                depends.setdefault(purl, set()).add(npm_purl(dep, entries[found]["version"]))
                queue.append(found)
    top_refs = [npm_purl(t.rsplit("node_modules/", 1)[1], entries[t]["version"]) for t in tops if t]
    return sorted(components, key=lambda c: c["bom-ref"]), depends, top_refs


# --- Fonts and data ------------------------------------------------------------------------


def asset_components() -> list[dict]:
    out = []
    fonts = os.path.join(ROOT, "resources", "fonts")
    for name in sorted(os.listdir(fonts)):
        family = next((f for p, f in FONTS.items() if name.startswith(p)), None)
        if not name.endswith(".ttf") or family is None:
            continue
        out.append({"type": "file", "bom-ref": f"font:{name}", "name": name, "description": f"{family} font",
                    "scope": "required", "licenses": [{"license": {"id": "OFL-1.1"}}],
                    "hashes": [{"alg": "SHA-256", "content": sha256_file(os.path.join(fonts, name))}]})
    sources = json.load(open(os.path.join(ROOT, "data", "sources.json"), encoding="utf-8"))
    for key, title, licence, path, upstream, commit in DATA_SETS:
        if not os.path.exists(os.path.join(ROOT, path)):
            fail(f"{path} is missing (DATA_SETS in tools/sbom.py)")
        component = {"type": "data", "bom-ref": f"data:{key}", "name": title, "scope": "required",
                     "licenses": [{"license": {"id": licence}}],
                     "externalReferences": [{"type": "distribution", "url": upstream}],
                     "properties": [{"name": "caruspex:source", "value": path}]}
        if commit and sources.get(commit):
            component["version"] = sources[commit]
        out.append(component)
    return out


# --- The document --------------------------------------------------------------------------


def build(platform: str, version: str) -> dict:
    rust, rust_deps, members = rust_components(version, PLATFORMS[platform])
    npm, npm_deps, npm_tops = npm_components()
    assets = asset_components()
    components = sorted(rust, key=lambda c: c["bom-ref"]) + npm + assets
    for c in components:
        if not c.get("licenses"):
            del c["licenses"]
    depends = {**rust_deps, **npm_deps}
    root_ref = f"caruspex@{version}"
    depends[root_ref] = set(members) | set(npm_tops) | {a["bom-ref"] for a in assets}
    document = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "version": 1,
        "metadata": {
            "lifecycles": [{"phase": "build"}],
            "tools": {"components": [{"type": "application", "name": "tools/sbom.py", "version": version,
                                      "externalReferences": [{"type": "vcs", "url": REPO}]}]},
            "component": {
                "type": "application",
                "bom-ref": root_ref,
                "name": "Caruspex",
                "version": version,
                "description": "Car diagnostics through an OBD adapter, desktop and command line",
                "licenses": [{"expression": "MIT"}],
                "purl": f"pkg:github/SenseiIssei/Caruspex@v{version}",
                "externalReferences": [
                    {"type": "vcs", "url": REPO},
                    {"type": "website", "url": "https://senseiissei.github.io/Caruspex/"},
                    {"type": "issue-tracker", "url": f"{REPO}/issues"},
                    {"type": "security-contact", "url": f"{REPO}/security/advisories/new"},
                    {"type": "distribution", "url": f"{REPO}/releases/tag/v{version}"},
                ],
            },
            "manufacturer": {"name": "Caruspex project", "url": [REPO]},
            "properties": [{"name": "caruspex:platform", "value": platform}]
            + [{"name": "caruspex:rust-target", "value": t} for t, _ in PLATFORMS[platform]],
        },
        "components": components,
        "dependencies": [{"ref": r, "dependsOn": sorted(depends.get(r, ()))}
                         for r in [root_ref] + [c["bom-ref"] for c in components]],
    }
    epoch = os.environ.get("SOURCE_DATE_EPOCH")
    if epoch:
        stamp = datetime.datetime.fromtimestamp(int(epoch), datetime.timezone.utc)
        document["metadata"]["timestamp"] = stamp.strftime("%Y-%m-%dT%H:%M:%SZ")
    return document


def validate(document: dict, platform: str) -> list[str]:
    """The rules of the CycloneDX 1.6 JSON schema that this generator can break."""
    problems = []
    if document.get("bomFormat") != "CycloneDX" or document.get("specVersion") != "1.6":
        problems.append("bomFormat/specVersion")
    refs = [document["metadata"]["component"]["bom-ref"]]
    for c in document["components"]:
        where = f"{platform}: {c.get('bom-ref')}"
        refs.append(c.get("bom-ref"))
        if c.get("type") not in COMPONENT_TYPES or not c.get("name"):
            problems.append(f"{where}: type or name")
        if c.get("scope") not in ("required", "optional", "excluded"):
            problems.append(f"{where}: scope")
        if "purl" in c and not PURL.match(c["purl"]):
            problems.append(f"{where}: purl '{c['purl']}'")
        for h in c.get("hashes", []):
            if len(h.get("content", "")) != HASH_LENGTHS.get(h.get("alg"), -1) or not re.fullmatch(r"[0-9a-f]+", h["content"]):
                problems.append(f"{where}: hash")
        for entry in c.get("licenses", []):
            if len(entry) != 1 or ("license" in entry and len(set(entry["license"]) & {"id", "name"}) != 1):
                problems.append(f"{where}: licence entry")
            if "expression" in entry and ("/" in entry["expression"] or not entry["expression"].strip()):
                problems.append(f"{where}: licence expression")
        if c.get("licenses") == []:
            problems.append(f"{where}: empty licences")
    if len(set(refs)) != len(refs):
        problems.append(f"{platform}: duplicate bom-ref")
    known = set(refs)
    for d in document["dependencies"]:
        for r in [d["ref"], *d["dependsOn"]]:
            if r not in known:
                problems.append(f"{platform}: dependency on unknown ref {r}")
    names = {c["name"] for c in document["components"] if c.get("scope") == "required"}
    for must in ("tauri", "serde", "react", "three"):
        if must not in names:
            problems.append(f"{platform}: '{must}' is missing")
    for test_only in ("proptest",):
        if test_only in names:
            problems.append(f"{platform}: test-only crate '{test_only}' listed as required")
    return problems


def file_name(version: str, platform: str) -> str:
    return f"Caruspex-{version}-sbom-{platform}.cdx.json"


def render(document: dict) -> bytes:
    return (json.dumps(document, ensure_ascii=False, indent=1, sort_keys=False) + "\n").encode("utf-8")


def main() -> int:
    arguments = sys.argv[1:]
    check = "--check" in arguments
    out = arguments[arguments.index("--out") + 1] if "--out" in arguments else None
    if check == bool(out):
        print(__doc__, file=sys.stderr)
        return 2
    version = workspace_version()
    problems = []
    for platform in PLATFORMS:
        first = render(build(platform, version))
        document = json.loads(first)
        problems += validate(document, platform)
        if check and render(build(platform, version)) != first:
            problems.append(f"{platform}: two runs gave different output")
        counts = {s: sum(1 for c in document["components"] if c.get("scope") == s) for s in ("required", "excluded")}
        print(f"{file_name(version, platform)}: {counts['required']} shipped, {counts['excluded']} build-only components")
        if out:
            os.makedirs(out, exist_ok=True)
            with open(os.path.join(out, file_name(version, platform)), "wb") as f:
                f.write(first)
    for p in problems:
        print(f"sbom: {p}", file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
