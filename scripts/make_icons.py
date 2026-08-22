import struct
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent / "src-tauri" / "icons"
ROOT.mkdir(parents=True, exist_ok=True)
PUBLIC = HERE.parent / "public"
PUBLIC.mkdir(exist_ok=True)


def png(size: int, rgb=(232, 165, 75)) -> bytes:
    raw = b"".join(b"\x00" + (bytes(rgb) * size) for _ in range(size))
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 2, 0, 0, 0)

    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def ico(png32: bytes) -> bytes:
    # ICO with embedded PNG (Vista+)
    count = 1
    header = struct.pack("<HHH", 0, 1, count)
    entry = struct.pack("<BBBBHHII", 32, 32, 0, 0, 1, 32, len(png32), 6 + 16)
    return header + entry + png32


p32 = png(32)
p128 = png(128)
p256 = png(256)
p512 = png(512)
(ROOT / "32x32.png").write_bytes(p32)
(ROOT / "128x128.png").write_bytes(p128)
(ROOT / "128x128@2x.png").write_bytes(p256)
(ROOT / "icon.png").write_bytes(p512)
(ROOT / "icon.ico").write_bytes(ico(p32))
(PUBLIC / "icon.png").write_bytes(p512)
print("pngs written")
