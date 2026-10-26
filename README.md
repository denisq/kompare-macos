# Kompare (Rust / macOS)

A from-scratch reimplementation of KDE's [Kompare](https://apps.kde.org/kompare/) diff viewer
for macOS: same layout and workflow (navigation lists, two aligned panes joined by colored
connecting bands, previous/next difference), written in Rust with egui, no Qt or KDE runtime.
The result is a single self-contained `Kompare.app`.

## What it does

- Opens unified diff files (`.diff`, `.patch`; `diff -u` and multi-file `git diff` output)
- Compares two files directly (File → Compare Files…, or `open -a Kompare a.md b.md`)
- Kompare palette: removed = green, changed = red, added = blue; character-level change marks
- Wraps long lines (good for markdown prose) — toggle in the View menu
- Start screen with Open Diff / Compare Files buttons and a persisted Recent list
- Theme: System / Light / Dark (View → Theme, or Preferences)
- Registered as the Finder handler for `.diff` / `.patch`, so double-click opens the rendered diff

Not (yet) implemented compared to Kompare: folder comparison, applying/unapplying differences
and saving merged results, blending a diff into a source tree.

## Build

Rust is installed via rustup in `~/.cargo` (not on `PATH`).

```sh
./bundle.sh            # release build → ~/Applications/Kompare.app
~/.cargo/bin/cargo test
~/.cargo/bin/cargo run -- path/to/changes.diff
```

Debug aid: `kompare --screenshot out.ppm file.diff` renders the first frame to a PPM and exits.

## Layout

| File | Role |
|------|------|
| `src/model.rs` | Difference / hunk / file model, intra-line change marks |
| `src/unified.rs` | Unified diff parser |
| `src/compare.rs` | Two-file comparison via the `similar` crate |
| `src/view.rs` | The Kompare view: panes, line numbers, connecting bands, scrolling |
| `src/nav.rs` | Files + changes navigation tables |
| `src/app.rs` | Window, native menu bar, status bar, preferences, dialogs |
| `src/macos.rs` | Receives files opened from Finder (`application:openFiles:`) |
| `bundle/` | `Info.plist` (document types) and the app icon |

## Uninstall

Delete `~/Applications/Kompare.app` and this folder. Remove Rust with `~/.cargo/bin/rustup self uninstall`.
