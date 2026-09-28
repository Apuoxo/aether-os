#!/usr/bin/env python3
import pathlib, subprocess, sys, urllib.request

out = pathlib.Path(sys.argv[1])
src = out / "cursor_src"
png = out / "cursor_png"
src.mkdir(parents=True, exist_ok=True)
png.mkdir(parents=True, exist_ok=True)

variants = [
    ("default", "red", "#E81123"), ("default", "blue", "#0078D7"),
    ("default", "green", "#107C10"), ("default", "gold", "#FFB900"),
    ("default", "purple", "#5C2D91"), ("default", "cyan", "#00B7C3"),
    ("pointer", "red", "#E81123"), ("pointer", "blue", "#0078D7"),
    ("pointer", "green", "#107C10"), ("pointer", "gold", "#FFB900"),
    ("pointer", "purple", "#5C2D91"), ("pointer", "cyan", "#00B7C3"),
    ("crosshair", "red", "#E81123"), ("crosshair", "blue", "#0078D7"),
    ("crosshair", "green", "#107C10"), ("crosshair", "gold", "#FFB900"),
    ("crosshair", "purple", "#5C2D91"), ("crosshair", "cyan", "#00B7C3"),
    ("text", "red", "#E81123"), ("text", "blue", "#0078D7"),
    ("text", "green", "#107C10"), ("text", "gold", "#FFB900"),
    ("text", "purple", "#5C2D91"), ("text", "cyan", "#00B7C3"),
]
base = "https://raw.githubusercontent.com/phisch/phinger-cursors/master/theme/light/"
raw_cache = {}
for i, (shape, color_name, color) in enumerate(variants):
    if shape not in raw_cache:
        raw_cache[shape] = urllib.request.urlopen(base + shape + "_32.svg", timeout=30).read().decode()
    svg = raw_cache[shape].replace("#FBFBFB", color)
    svg_path = src / f"{shape}_{color_name}.svg"
    png_path = png / f"{shape}_{color_name}.png"
    rgba_path = png / f"{shape}_{color_name}.rgba"
    svg_path.write_text(svg)
    subprocess.run(["rsvg-convert", "-w", "16", "-h", "16", str(svg_path), "-o", str(png_path)], check=True)
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", str(png_path),
                    "-f", "rawvideo", "-pix_fmt", "rgba", "-y", str(rgba_path)], check=True)
    if rgba_path.stat().st_size != 16 * 16 * 4:
        raise SystemExit("invalid cursor raster size: " + str(rgba_path))

lines = ["pub const COUNT: usize = 24;"]
for i, (shape, color_name, _) in enumerate(variants):
    lines.append(f'pub static C{i}: &[u8] = include_bytes!("cursor_png/{shape}_{color_name}.rgba");')
path = out / "cursor_builtin.rs"
path.write_text("\n".join(lines) + "\n")
