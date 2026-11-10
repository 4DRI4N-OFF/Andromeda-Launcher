# Andromeda Launcher

A Minecraft Java launcher written in Rust, with a testable core and native Slint UI. Private development project, currently at phase 3.

## Current state

- Native Slint UI with a profile list, a Play panel and account/settings cards; dark mode, reduced motion and the selected instance are persisted.
- Controls are drawn in the Slint UI instead of using platform widgets, so the launcher's own dark/light switch applies regardless of the Windows theme.
- Approved animated logo intro, click to skip, fully skipped with Reduce animations.
- Headless vanilla version list and verified downloader, including Java.
- Play installed vanilla 1.21.11 offline with a local profile from the UI or CLI.
- Windows x64 debug fmt, clippy, 25 tests and build pass.
- Real singleplayer world creation, saving and reopening verified on the development PC.
- Isolated instances: create, deep duplicate, recoverable launcher trash and restore in UI/CLI.
- No Microsoft sign-in, UI installer, loaders or mods yet.

See [PHASE1.md](PHASE1.md), [INTRO.md](INTRO.md) [PHASE2.md](PHASE2.md) and [PHASE3.md](PHASE3.md) for commands, validation and limits.

## Development

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

UI: `target/debug/andromeda.exe`. Downloader: `target/debug/andromeda-download.exe list` or `install 1.21.11`.

Runtime data stays in the source workspace's `data` folder. Do not commit downloaded game assets, Java binaries, settings or logs.

## Language and contributions

English is the default for UI, documentation, comments and new commit/PR text. Slint UI strings use `@tr()` so translation catalogs can be added later. English is currently the only supplied locale; this is translation-ready, not a completed language selector.

Use a separate branch and pull request for every change. Keep this repository private. No release or public publication without the owner's approval.

## Disclaimer

Not an official Minecraft product. Not affiliated with Mojang or Microsoft. Downloading game files does not grant a Minecraft license. Project licensing is not yet decided.
## Interface

`app/ui/main.slint` has two screens, and the Play screen only shows what is needed to
start the game:

- **Play** (default): the profile list on the left, and on the right the selected profile,
  its Minecraft version, one Play button and the live launch status. Nothing else.
- **Settings** (the button in the header): Microsoft account, new profile (name plus the
  five loader shortcuts), the selected profile (duplicate, recoverable trash, in-game
  name) and appearance (reduce animations, dark mode). It is a single scrolling column of
  cards, so it stays readable at the 860x620 minimum.
- The header keeps the logo, the account state as a read-only chip and the screen switch.
  The open screen is not persisted: the launcher always starts on Play.

Buttons, rows, the text field and the switches are drawn in the `.slint` file rather than
taken from `std-widgets`, because platform widgets follow the Windows theme while the
launcher follows its own dark-mode setting; `ScrollView` is the only standard widget left.
Both themes come from `dark-mode` through the `c-*` colour tokens on the window.

JetBrains Mono (Apache/OFL, `app/ui/fonts`) is embedded at build time with `import "...ttf"`
and set as the family of every text element, so the launcher does not depend on the font
being installed on the machine. `app/ui/logo-mark.png` is the AL mark cropped from the
owner's `ANDROMEDA LAUNCHER LOGO PEQUEÑO.png` render; it is the window icon and the header
mark. The `.exe` icon resource is still the default one.

Layout was checked by rendering the window headlessly with Slint's software renderer at
1040x780 and at the 900x700 minimum, in dark and light mode, with a long profile list and
with the intro overlay shown, and by launching the real Windows build and inspecting the
window (header mark, embedded font, card heights, window icon). Clicking every control by
hand, resizing during a session and multi-DPI remain untested.
