#!/bin/sh
# Build the web-UI variant (Rust core + macOS WebView via Tauri) and assemble
# ~/Applications/Kompare.app. The UI is the static ui/ folder, embedded at compile time.
set -eu
cd "$(dirname "$0")"

CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
APP="${APP:-$HOME/Applications/Kompare.app}"

(cd src-tauri && "$CARGO" build --release)

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp src-tauri/target/release/kompare-web "$APP/Contents/MacOS/kompare-web"
cp bundle/Info-web.plist "$APP/Contents/Info.plist"
cp bundle/Kompare.icns "$APP/Contents/Resources/Kompare.icns"
printf 'APPL????' > "$APP/Contents/PkgInfo"

codesign --force --sign - "$APP" >/dev/null 2>&1 || true
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$APP"

echo "Built $APP"
du -sh "$APP" | cut -f1
