#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_PNG="${1:-$ROOT_DIR/assets/icons/antares_icon.png}"
OUTPUT_ROOT="${2:-$ROOT_DIR/assets/icons/game/generated}"
WEB_DIR="$OUTPUT_ROOT/web"
MACOS_DIR="$OUTPUT_ROOT/macos"

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required command: $1" >&2
        exit 1
    fi
}

render_png() {
    local source_png="$1"
    local output_png="$2"
    local target_size="$3"

    swift - "$source_png" "$output_png" "$target_size" <<'SWIFT'
import AppKit
import Foundation

let arguments = CommandLine.arguments
guard arguments.count == 4 else {
    fputs("usage: swift render <source> <dest> <size>\n", stderr)
    exit(1)
}

let sourcePath = arguments[1]
let destinationPath = arguments[2]
guard let size = Int(arguments[3]), size > 0 else {
    fputs("invalid target size\n", stderr)
    exit(1)
}

let sourceUrl = URL(fileURLWithPath: sourcePath)
guard let sourceImage = NSImage(contentsOf: sourceUrl) else {
    fputs("failed to load source image\n", stderr)
    exit(1)
}

guard let bitmap = NSBitmapImageRep(
    bitmapDataPlanes: nil,
    pixelsWide: size,
    pixelsHigh: size,
    bitsPerSample: 8,
    samplesPerPixel: 4,
    hasAlpha: true,
    isPlanar: false,
    colorSpaceName: .deviceRGB,
    bitmapFormat: [],
    bytesPerRow: 0,
    bitsPerPixel: 0
) else {
    fputs("failed to allocate bitmap\n", stderr)
    exit(1)
}

guard let context = NSGraphicsContext(bitmapImageRep: bitmap) else {
    fputs("failed to create graphics context\n", stderr)
    exit(1)
}

NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = context
context.imageInterpolation = .high
NSColor.clear.setFill()
NSBezierPath(rect: NSRect(x: 0, y: 0, width: size, height: size)).fill()
sourceImage.draw(
    in: NSRect(x: 0, y: 0, width: size, height: size),
    from: NSRect.zero,
    operation: .sourceOver,
    fraction: 1.0
)
context.flushGraphics()
NSGraphicsContext.restoreGraphicsState()

guard let pngData = bitmap.representation(using: .png, properties: [:]) else {
    fputs("failed to encode png\n", stderr)
    exit(1)
}

let destinationUrl = URL(fileURLWithPath: destinationPath)
try pngData.write(to: destinationUrl)
SWIFT
}

build_favicon_ico() {
    local output_ico="$1"
    shift
    python3 - "$output_ico" "$@" <<'PYTHON'
import pathlib
import struct
import sys

output_path = pathlib.Path(sys.argv[1])
png_paths = [pathlib.Path(path) for path in sys.argv[2:]]
png_payloads = [path.read_bytes() for path in png_paths]

header = struct.pack('<HHH', 0, 1, len(png_paths))
offset = 6 + len(png_paths) * 16
entries = []
payload = bytearray()

for path, data in zip(png_paths, png_payloads):
    stem = path.stem
    size_token = stem.rsplit('-', 1)[-1]
    width = int(size_token.split('x')[0])
    height = int(size_token.split('x')[1])
    entries.append(
        struct.pack(
            '<BBBBHHII',
            width if width < 256 else 0,
            height if height < 256 else 0,
            0,
            0,
            1,
            32,
            len(data),
            offset,
        )
    )
    payload.extend(data)
    offset += len(data)

output_path.write_bytes(header + b''.join(entries) + payload)
PYTHON
}

require_command swift
require_command python3

if [[ ! -f "$SOURCE_PNG" ]]; then
    echo "Source PNG not found: $SOURCE_PNG" >&2
    exit 1
fi

mkdir -p "$WEB_DIR" "$MACOS_DIR"

render_png "$SOURCE_PNG" "$WEB_DIR/favicon-16x16.png" 16
render_png "$SOURCE_PNG" "$WEB_DIR/favicon-32x32.png" 32
render_png "$SOURCE_PNG" "$WEB_DIR/favicon-48x48.png" 48
render_png "$SOURCE_PNG" "$WEB_DIR/apple-touch-icon.png" 180
render_png "$SOURCE_PNG" "$WEB_DIR/android-chrome-192x192.png" 192
render_png "$SOURCE_PNG" "$WEB_DIR/android-chrome-512x512.png" 512

build_favicon_ico \
    "$WEB_DIR/favicon.ico" \
    "$WEB_DIR/favicon-16x16.png" \
    "$WEB_DIR/favicon-32x32.png" \
    "$WEB_DIR/favicon-48x48.png"

render_png "$SOURCE_PNG" "$MACOS_DIR/tray_icon_1x.png" 22
render_png "$SOURCE_PNG" "$MACOS_DIR/tray_icon_2x.png" 44

cat <<EOF
Generated icon assets:
  Web:
    $WEB_DIR/favicon.ico
    $WEB_DIR/favicon-16x16.png
    $WEB_DIR/favicon-32x32.png
    $WEB_DIR/favicon-48x48.png
    $WEB_DIR/apple-touch-icon.png
    $WEB_DIR/android-chrome-192x192.png
    $WEB_DIR/android-chrome-512x512.png
  macOS:
        $MACOS_DIR/tray_icon_1x.png
        $MACOS_DIR/tray_icon_2x.png
EOF
