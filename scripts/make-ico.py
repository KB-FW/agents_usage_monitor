#!/usr/bin/env python3
"""Build icon.ico (Windows) and icon.icns (macOS) from src-tauri/icons/icon.png (needs Pillow)."""
from pathlib import Path

from PIL import Image

icons = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
src = Image.open(icons / "icon.png").convert("RGBA")
src.save(icons / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
src.save(icons / "icon.icns")
print("wrote", icons / "icon.ico", "and", icons / "icon.icns")
