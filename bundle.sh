#!/bin/sh
# Build Kompare in release mode and assemble ~/Applications/Kompare.app.
set -eu
cd "$(dirname "$0")"

CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
APP="${APP:-$HOME/Applications/Kompare.app}"

"$CARGO" build --release

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/kompare "$APP/Contents/MacOS/kompare"
cp bundle/Info.plist "$APP/Contents/Info.plist"
[ -f bundle/Kompare.icns ] && cp bundle/Kompare.icns "$APP/Contents/Resources/Kompare.icns"
printf 'APPL????' > "$APP/Contents/PkgInfo"

codesign --force --sign - "$APP" >/dev/null 2>&1 || true
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$APP"

echo "Built $APP"
du -sh "$APP" | cut -f1
