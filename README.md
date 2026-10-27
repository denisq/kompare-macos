# Kompare

A from-scratch reimplementation of KDE's [Kompare](https://apps.kde.org/kompare/) diff viewer:
same layout and workflow (navigation sidebar, two aligned panes joined by colored connecting
bands, previous/next difference), with a Rust core and a native WebView UI (Tauri 2). No Qt,
no KDE runtime — a single self-contained app of about 5 MB.

Written for macOS first; Windows and Linux installers are produced by the release workflow.

<img width="1262" height="892" alt="image" src="https://github.com/user-attachments/assets/857a3e58-590b-45db-a38d-2654d64db1e8" />


## Download

Grab the installer for your platform from the [Releases](https://github.com/denisq/kompare-macos/releases) page.

- **macOS** (`.dmg`, universal: Apple Silicon and Intel): the app is not notarized with Apple, so the first
  launch is blocked once. Open *System Settings → Privacy & Security* and click **Open Anyway**
  (or run `xattr -dr com.apple.quarantine /Applications/Kompare.app`).
- **Windows** (`.msi` or `-setup.exe`): SmartScreen may warn because the installer is unsigned;
  choose *More info → Run anyway*.
- **Linux** (`.deb`, `.rpm` or `.AppImage`): needs WebKitGTK 4.1 (`libwebkit2gtk-4.1`), which
  the `.deb`/`.rpm` pull in; the AppImage runs anywhere with FUSE.

The installers register `.diff` / `.patch` files, so double-clicking a diff opens it rendered.

## What it does

- Opens unified diff files (`diff -u`, multi-file `git diff` output); compares two files directly
- Kompare's view: navigation sidebar (files + changes), two aligned panes with connecting bands,
  character-level change marks, shared proportional scrolling, 3px strips where lines were
  inserted or removed, hunk separators
- Hover: a line and its counterpart highlight together; a change highlights both blocks and its band
- Find (⌘F / Ctrl+F) across both panes; minimap with viewport indicator; keyboard scrolling (arrows, j/k, PgUp/PgDn, n/p)
- Diff options for Compare mode: ignore whitespace / case / blank lines, lines of context
- Start screen with two drop-target tiles and a Recent list; drag and drop anywhere
- Wraps long lines (good for markdown prose), light and dark mode, native menus and context menu

Not (yet) implemented compared to Kompare: folder comparison, applying/unapplying differences
and saving merged results, blending a diff into a source tree, printing.

## Build from source

Requires Rust (via [rustup](https://rustup.rs)). The UI is the static `ui/` folder, embedded at
compile time — no Node/npm.

```sh
# macOS quick build: release binary wrapped into ~/Applications/Kompare.app (no installer)
./bundle-web.sh

# installers for the current platform (needs: cargo install tauri-cli --version '^2')
cd src-tauri && cargo tauri build
```

Linux needs the WebKitGTK development packages first (see `.github/workflows/release.yml`).

To work on the UI without compiling, serve `ui/` (e.g. `python3 -m http.server`) and open
`index.html#mock`, which loads sample data from `mock.js`.

## Releasing

Push a tag `vX.Y.Z` and the [release workflow](.github/workflows/release.yml) builds the
macOS, Windows and Linux installers on GitHub's runners and attaches them to the release.

## Layout

| Path | Role |
|------|------|
| `src-tauri/src/model.rs` | Difference / hunk / file model, intra-line change marks |
| `src-tauri/src/unified.rs` | Unified diff parser |
| `src-tauri/src/compare.rs` | Two-file comparison (`similar` crate) with diff options |
| `src-tauri/src/main.rs` | Tauri app: commands, native menu, files opened from the OS |
| `src-tauri/tauri.conf.json` | Window, bundler and file-association configuration |
| `ui/index.html` | The whole UI: layout, Kompare view, bands, find, minimap, start screen |
| `bundle/` | App icon (SVG source, iconset, icns) |
| `src/`, `Cargo.toml`, `bundle.sh` | The earlier egui (pure Rust) version, kept for reference |

## License

GPL-2.0-or-later, like the original Kompare.
