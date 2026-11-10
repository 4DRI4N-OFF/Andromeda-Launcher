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

`app/ui/main.slint` is organised around what the launcher actually does:

- **Profiles** (left): one clickable row per instance, then the new-instance name, Create/Duplicate, the four loader shortcuts and the recoverable trash pair.
- **Play** (right): the selected profile, a full-width Play button and the live launch status.
- **Microsoft account**: sign-in, cancel, local sign-out and the "use for Play" switch.
- **Offline name**: the local player name, kept next to its singleplayer-only note.
- **Footer**: reduce animations, dark mode.

Buttons, rows, the text field and the switches are drawn in the `.slint` file rather than
taken from `std-widgets`, because platform widgets follow the Windows theme while the
launcher follows its own dark-mode setting; `ScrollView` is the only standard widget left.
Both themes come from `dark-mode` through the `c-*` colour tokens on the window.

Layout was checked by rendering the window headlessly with Slint's software renderer at
1040x780 and at the 900x700 minimum, in dark and light mode, with a long profile list and
with the intro overlay shown. No interactive click-through of the new controls on real
Windows has been done yet, and the resize and multi-DPI cases are still untested.
