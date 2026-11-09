# Andromeda Launcher

A Minecraft Java launcher written in Rust, with a testable core and native Slint UI. Private development project, currently at phase 2.

## Current state

- Native settings UI with persisted dark mode and reduced-motion preference.
- Approved animated logo intro, click to skip, fully skipped with Reduce animations.
- Headless vanilla version list and verified downloader, including Java.
- Play installed vanilla 1.21.11 offline with a local profile from the UI or CLI.
- Windows x64 debug fmt, clippy, 18 tests and build pass.
- Real singleplayer world creation, saving and reopening verified on the development PC.
- No Microsoft sign-in, UI installer, loaders or mods yet.

See [PHASE1.md](PHASE1.md), [INTRO.md](INTRO.md) and [PHASE2.md](PHASE2.md) for commands, validation and limits.

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