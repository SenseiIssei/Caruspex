"""Pictures of the 3D X-ray view for a list of catalog generations.

The app draws every generation of the catalog as a glass car (or bike) after
its dimensions and drivetrain. This tool starts the built app once per
generation with the hidden screenshot option of the X-ray page, saves one
picture each and puts them together on one contact sheet: for the README and
for a look over many cars at once after a change to app/src/xray/.

    py -I tools/xray_shots.py                     the default set into target/xray-shots/
    py -I tools/xray_shots.py --readme --sheet docs/xray-cars.png
    py -I tools/xray_shots.py "BMW|3er|F30|estate" "Yamaha|MT-07|II|touring" 1924
    py -I tools/xray_shots.py "MAN|TGS|II|citybus=City bus (on the MAN TGS)"
    py -I tools/xray_shots.py --check             only check that the default set is in the catalog

A car is "Brand|Model|Generation" as in the catalog (data/vehicles/), with an
optional fourth part for the body variant (hatch, sedan, estate, suv, van,
coupe, cabrio, pickup, mpv, roadster), the kind of motorcycle (naked,
sport, touring, enduro, cruiser, scooter) or the kind of truck or bus
(tractor, rigid, citybus, coach), or the catalog id of a generation;
"=Caption" at the end replaces the caption on the sheet. Names are looked
up in the copy of the catalog the browser preview uses
(app/src/api/mock/data/catalog.json), whose order is the order of the ids.

Needs the release build of the app (cargo build --release -p caruspex-app
after npm --prefix app run build, or tools/package.sh); --app points
elsewhere. Every window opens at SHOT_WINDOW_POS (default -1880,40: on a
monitor left of the main one) and never takes the focus; one window at a
time, each closes itself after its picture. A throwaway garage keeps the own
one untouched. The contact sheet needs Pillow; without it only the single
pictures are written.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CATALOG = os.path.join(ROOT, "app", "src", "api", "mock", "data", "catalog.json")

# One of each body and drivetrain the view draws differently: front engine
# across and along, rear and mid engine, electric, hybrid, kei cars, a
# convertible variant, a mid and a large van, an off-roader with its spare
# wheel, two pickups, a European and an American tractor, a rigid truck, a
# city bus and a coach, and four kinds of motorcycle.
DEFAULT_SET = [
    "Volkswagen|Golf|VII",
    "BMW|3er|F30",
    "Mercedes-Benz|E-Klasse|W213|cabrio",
    "Porsche|911|992",
    "Tesla|Model 3|I",
    "Toyota|Prius|XW30",
    "Mitsubishi|i-MiEV|HA",
    "Suzuki|Cappuccino|EA11R",
    "Fiat|500|I",
    "Volkswagen|Transporter|T6",
    "Mercedes-Benz|Sprinter|W907",
    "Land Rover|Defender|L663",
    "Toyota|Hilux|AN120",
    "Ford|F-150|XIV",
    "MAN|TGX|II",
    "Freightliner|Cascadia|II",
    "MAN|TGS|II",
    "Mercedes-Benz|Actros|MP5|citybus=City bus (kind picked on the Actros)",
    "Mercedes-Benz|Actros|MP5|coach=Coach (kind picked on the Actros)",
    "Yamaha|MT-07|II",
    "BMW|R 1250 GS|I",
    "Harley-Davidson|Sportster S|I",
    "Kawasaki|Ninja 650|I",
]

# The mix on the README's contact sheet: a few cars, a van, an off-roader,
# a pickup, trucks, a bus and a motorcycle.
README_SET = [
    "Volkswagen|Golf|VII",
    "BMW|3er|F30",
    "Mercedes-Benz|E-Klasse|W213|cabrio",
    "Porsche|911|992",
    "Tesla|Model 3|I",
    "Fiat|500|I",
    "Volkswagen|Transporter|T6",
    "Mercedes-Benz|Sprinter|W907",
    "Land Rover|Defender|L663",
    "Toyota|Hilux|AN120",
    "Ford|F-150|XIV",
    "MAN|TGX|II",
    "Freightliner|Cascadia|II",
    "Mercedes-Benz|Actros|MP5|citybus=City bus (generic shape)",
    "Yamaha|MT-07|II",
    "BMW|R 1250 GS|I",
]

VARIANTS = {"hatch", "sedan", "estate", "suv", "van", "coupe", "cabrio", "pickup", "mpv", "roadster",
            "naked", "sport", "touring", "enduro", "cruiser", "scooter",
            "tractor", "rigid", "citybus", "coach"}

# The dark theme's window colour, behind and between the pictures.
SHEET_BACKGROUND = (10, 12, 17)
SHEET_TEXT = (214, 220, 232)


def load_catalog() -> list[list]:
    with open(CATALOG, encoding="utf-8") as f:
        return json.load(f)["generations"]


MOTORCYCLE = 11  # BodyType of the catalog


def resolve(spec: str, generations: list[list]) -> tuple[int, str, str, bool]:
    """The id, the variant, a caption and whether it is a motorcycle."""
    spec, _, own_caption = spec.partition("=")
    if own_caption.strip():
        gid, variant, _caption, moto = resolve(spec, generations)
        return gid, variant, own_caption.strip(), moto
    if re.fullmatch(r"\d+", spec.strip()):
        gid = int(spec)
        if gid >= len(generations):
            raise SystemExit(f"no generation with the id {gid} (the catalog has {len(generations)})")
        g = generations[gid]
        return gid, "", f"{g[0]} {g[1]} {g[2]}", g[5] == MOTORCYCLE
    parts = [p.strip() for p in spec.split("|")]
    if len(parts) not in (3, 4):
        raise SystemExit(f"{spec!r}: expected Brand|Model|Generation[|variant] or an id")
    brand, model, generation = parts[:3]
    variant = parts[3].lower() if len(parts) == 4 else ""
    if variant and variant not in VARIANTS:
        raise SystemExit(f"{spec!r}: unknown variant {variant!r}, one of {', '.join(sorted(VARIANTS))}")
    for gid, g in enumerate(generations):
        if (g[0], g[1], g[2]) == (brand, model, generation):
            caption = f"{brand} {model} {generation}" + (f" ({variant})" if variant else "")
            return gid, variant, caption, g[5] == MOTORCYCLE
    raise SystemExit(f"{spec!r} is not in the catalog")


def find_app(given: str | None) -> str:
    candidates = [given] if given else [
        os.path.join(ROOT, "target", "release", "caruspex.exe"),
        os.path.join(ROOT, "target", "release", "caruspex"),
    ]
    for c in candidates:
        if c and os.path.isfile(c):
            # Windows does not start a relative path with forward slashes.
            return os.path.abspath(c)
    raise SystemExit("build the app first: npm --prefix app run build, then cargo build --release -p caruspex-app (or pass --app)")


def slug(caption: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", caption.lower()).strip("-")


def shoot(app: str, gid: int, variant: str, moto: bool, file: str, args: argparse.Namespace, garage: str) -> bool:
    # A bike seen from the front is mostly fork and lamp: from the side.
    view = args.view or ("side" if moto else "front")
    page = f"xray-generation:{gid}|{variant}|{view}|bare"
    cmd = [app, "--theme", args.theme, "--mode", "pro", "--language", "en", "--page", page,
           "--window-pos", args.window_pos, "--size", args.size, "--shot", file, "--shot-delay", str(args.delay)]
    env = dict(os.environ, CARUSPEX_GARAGE=garage)
    if os.path.exists(file):
        os.remove(file)
    try:
        # The app quits by itself after the picture; the timeout only
        # catches a window that hangs.
        subprocess.run(cmd, env=env, timeout=args.delay / 1000 + 60, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
    except subprocess.TimeoutExpired:
        print("  no picture: the app did not finish in time", file=sys.stderr)
        return False
    return os.path.isfile(file) and os.path.getsize(file) > 0


def contact_sheet(shots: list[tuple[str, str]], out: str, columns: int, tile_width: int, crop: float) -> bool:
    try:
        from PIL import Image, ImageDraw, ImageFont
    except ImportError:
        # Also when Pillow has no build for this interpreter (a free-threaded
        # Python, for one): the single pictures are there all the same.
        print("no contact sheet: Pillow is missing or not built for this Python (python -m pip install pillow)", file=sys.stderr)
        return False

    def middle(image: "Image.Image") -> "Image.Image":
        # The camera leaves room round the car for the page's controls; the
        # sheet keeps the middle of each picture.
        w, h = image.size
        cw, ch = round(w * crop), round(h * crop)
        left, top = (w - cw) // 2, (h - ch) // 2
        return image.crop((left, top, left + cw, top + ch))

    images = [(middle(Image.open(f).convert("RGB")), caption) for f, caption in shots]
    w0, h0 = images[0][0].size
    tile_height = round(tile_width * h0 / w0)
    gap, caption_height = 12, 30
    rows = (len(images) + columns - 1) // columns
    width = columns * tile_width + (columns + 1) * gap
    height = rows * (tile_height + caption_height) + (rows + 1) * gap
    sheet = Image.new("RGB", (width, height), SHEET_BACKGROUND)
    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.load_default(size=17)
    except TypeError:
        # Pillow before 10.1 has only the small bitmap font.
        font = ImageFont.load_default()
    for i, (image, caption) in enumerate(images):
        x = gap + (i % columns) * (tile_width + gap)
        y = gap + (i // columns) * (tile_height + caption_height + gap)
        sheet.paste(image.resize((tile_width, tile_height), Image.LANCZOS), (x, y))
        draw.text((x + 4, y + tile_height + 6), caption, fill=SHEET_TEXT, font=font)
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    # 256 colours keep the sheet small enough for the repository; the dark
    # glass needs no more.
    sheet.quantize(colors=256).save(out, optimize=True)
    return True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("cars", nargs="*", help="Brand|Model|Generation[|variant][=caption] or a catalog id; default: a set of 23")
    parser.add_argument("--readme", action="store_true", help="the 16 of the README's contact sheet instead of the default set")
    parser.add_argument("--out", default=os.path.join(ROOT, "target", "xray-shots"), help="folder for the single pictures")
    parser.add_argument("--sheet", help="the contact sheet (default: xray-cars.png in --out)")
    parser.add_argument("--app", help="the app to start (default: target/release/caruspex)")
    parser.add_argument("--view", choices=["front", "side", "rear"],
                        help="camera, front is three quarters from the front (default: front for cars, side for motorcycles)")
    parser.add_argument("--size", default="1200x800", help="window size of each picture")
    parser.add_argument("--delay", type=int, default=9000, help="milliseconds until the picture")
    parser.add_argument("--theme", default="dark", choices=["dark", "light"])
    parser.add_argument("--columns", type=int, default=4)
    parser.add_argument("--tile", type=int, default=420, help="width of one picture on the sheet")
    parser.add_argument("--crop", type=float, default=0.72, help="share of each picture (its middle) on the sheet")
    parser.add_argument("--window-pos", default=os.environ.get("SHOT_WINDOW_POS", "-1880,40"))
    parser.add_argument("--check", action="store_true", help="only check that the cars are in the catalog")
    args = parser.parse_args()

    generations = load_catalog()
    cars = [resolve(spec, generations) for spec in (args.cars or (README_SET if args.readme else DEFAULT_SET))]
    if args.check:
        for gid, _variant, caption, _moto in cars:
            print(f"{gid:5}  {caption}")
        print(f"{len(cars)} cars in the catalog")
        return 0

    app = find_app(args.app)
    os.makedirs(args.out, exist_ok=True)
    shots: list[tuple[str, str]] = []
    with tempfile.TemporaryDirectory() as work:
        garage = os.path.join(work, "garage.json")
        for n, (gid, variant, caption, moto) in enumerate(cars, 1):
            file = os.path.join(os.path.abspath(args.out), f"xray-{slug(caption)}.png")
            print(f"[{n}/{len(cars)}] {caption} (id {gid})")
            if shoot(app, gid, variant, moto, file, args, garage):
                shots.append((file, caption))
            else:
                print(f"  no picture for {caption}", file=sys.stderr)
    if not shots:
        return 1
    sheet = args.sheet or os.path.join(args.out, "xray-cars.png")
    if contact_sheet(shots, sheet, args.columns, args.tile, args.crop):
        print(f"contact sheet {sheet}")
    print(f"{len(shots)} of {len(cars)} pictures in {args.out}")
    return 0 if len(shots) == len(cars) else 1


if __name__ == "__main__":
    sys.exit(main())
