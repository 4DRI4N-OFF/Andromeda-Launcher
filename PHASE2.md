# Phase 2: local offline play

## Scope

The native launcher can start the installed vanilla Minecraft Java 1.21.11 client on Windows x64 with Java 21 and a local profile. This is local singleplayer, not Microsoft authentication. It does not grant a game license or bypass online server authentication.

- Enter 1-16 ASCII letters, numbers or underscores, then choose **Play offline**.
- The game runs on a worker thread; the launcher stays responsive and disables repeat clicks until the game exits.
- Memory is fixed at 2048 MB in the UI. The core accepts 512-8192 MB.
- Saves and options live in `data/vanilla/game`, separate from a normal Minecraft installation.
- The local name determines a standard `OfflinePlayer:<name>` UUID. Keep the same name to keep the same local player identity.

## Installation and command line

Install first using the phase-1 downloader. Launch does not download or repair missing files.

```powershell
.\target\debug\andromeda-download.exe install 1.21.11
.\target\debug\andromeda-play.exe 1.21.11 Player --dry-run
.\target\debug\andromeda-play.exe 1.21.11 Player
```

`--dry-run` prepares and verifies the launch plan without starting Java. It may create the game/native folders and extract required DLLs.

## Validation and safety

- Validate local names, supported version and memory limits before launch.
- Verify the Java executable, client JAR, used library/native JARs, asset index and logging configuration against installed manifest hashes and sizes.
- Build modern Windows launch arguments, rules, classpath and placeholders without a shell. Unknown placeholders fail closed.
- Extract DLLs only, rejecting archive traversal, symlinks, oversized entries and conflicting files. Ignore x86 and ARM native variants.
- Hold an exclusive Windows file handle while this launcher waits for its game process. Another cooperating launcher cannot start against the same game folder during that time.
- Keep stdout/stderr under `data/vanilla/game/launcher-logs/latest-stdout.log` and `latest-stderr.log`.

## Verified on the development PC

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`: 18 tests, including 6 launch tests
- `cargo build --workspace`
- Installed 1.21.11 dry run, Java 21 real launch and title screen.
- Create, enter, save and reopen `Andromeda_Phase2_Test` with the `AndromedaTest` local name.
- Launch through the UI, reject an invalid name, prevent a second cooperating launch, close the game and return to an enabled Play button.

## Limits

Only Windows x64 and installed vanilla 1.21.11 are supported in this phase. There is no Microsoft sign-in, online entitlement check, loader/mod support, UI version selector, UI installer or memory selector. The local name is not persisted yet. Runtime support files and individual asset objects are checked by the installer, not rehashed in full on every launch. Installed metadata is local, not a signed trust root. The lock is a cooperating-launcher guard, not a general OS mutex against unrelated launchers, and is released if the launcher is forcibly terminated. Debug-build validation is not a release-performance guarantee.
