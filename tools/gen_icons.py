"""Packs the app icon PNGs into the files the installers need.

    caruspex --render-icon build/icon
    python tools/gen_icons.py build/icon

writes resources/icons/caruspex.ico (Windows), caruspex.icns (macOS) and
caruspex.png (Linux, 256 px). Both containers simply hold PNG data, so no
image library is needed.
"""

import shutil
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "resources" / "icons"

ICO_SIZES = [16, 20, 24, 32, 40, 48, 64, 256]

# icns element types that take PNG data, with the pixel size of each.
ICNS_TYPES = [
    (b"icp4", 16),
    (b"icp5", 32),
    (b"icp6", 64),
    (b"ic07", 128),
    (b"ic08", 256),
    (b"ic09", 512),
    (b"ic10", 1024),
    (b"ic11", 32),    # 16 pt at 2x
    (b"ic12", 64),    # 32 pt at 2x
    (b"ic13", 256),   # 128 pt at 2x
    (b"ic14", 512),   # 256 pt at 2x
]


def png(folder: Path, size: int) -> bytes:
    data = (folder / f"icon-{size}.png").read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"icon-{size}.png is not a PNG")
    width, height = struct.unpack(">II", data[16:24])
    if (width, height) != (size, size):
        raise SystemExit(f"icon-{size}.png is {width}x{height}, expected {size}x{size}")
    return data


def write_ico(folder: Path, target: Path) -> None:
    images = [png(folder, s) for s in ICO_SIZES]
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = len(header) + 16 * len(images)
    entries = b""
    for size, data in zip(ICO_SIZES, images):
        dim = 0 if size >= 256 else size   # 0 stands for 256
        entries += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
    target.write_bytes(header + entries + b"".join(images))


def write_icns(folder: Path, target: Path) -> None:
    body = b""
    for kind, size in ICNS_TYPES:
        data = png(folder, size)
        body += kind + struct.pack(">I", len(data) + 8) + data
    target.write_bytes(b"icns" + struct.pack(">I", len(body) + 8) + body)


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    folder = Path(sys.argv[1])
    OUT.mkdir(parents=True, exist_ok=True)
    write_ico(folder, OUT / "caruspex.ico")
    write_icns(folder, OUT / "caruspex.icns")
    shutil.copyfile(folder / "icon-256.png", OUT / "caruspex.png")
    shutil.copyfile(folder / "icon-1024.png", OUT / "caruspex-1024.png")
    for name in ("caruspex.ico", "caruspex.icns", "caruspex.png", "caruspex-1024.png"):
        print(f"{name}: {(OUT / name).stat().st_size} bytes")


if __name__ == "__main__":
    main()
