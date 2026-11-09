# Phase 1: vanilla downloads

Windows x64, core-first implementation. This tool downloads vanilla Minecraft and its Java runtime; it does not launch Minecraft or open the Slint window.

```powershell
cargo run -p andromeda-core --bin andromeda-download -- list
cargo run -p andromeda-core --bin andromeda-download -- install 1.21.11
```

Files stay under `data/vanilla` in this source workspace. This is a development path, not a portable installed-app data location. Downloads, runtime binaries and Mojang assets are ignored by Git. No account credentials are required for the download step. Playing Minecraft requires the relevant license/account; downloads do not grant ownership.

The installer gets version metadata from Mojang, verifies its SHA-1, downloads the client JAR, Windows libraries/native archives, logging config, asset index/objects and Mojang's Windows x64 Java runtime. Every artifact with metadata is SHA-1/size checked before publication. Downloads use `.part` files, three attempts, bounded timeouts and eight workers. Existing valid files are reused; bad files are replaced after a verified download. A failed install leaves reusable verified files but no new success receipt. SHA-1 here follows Mojang's integrity metadata, not a signature or general proof of authenticity.

Run only one installer at a time for a data folder. There is no interprocess install lock, cancellation UI, progress bar, proxy setting, ranged resume or disk-space estimate yet. Interrupted files restart from the beginning. The current rule handling targets Windows x64 and conservatively skips unknown OS-version rules. Other platforms, older legacy asset layouts, runtime link entries and arbitrary old releases are not guaranteed. Only 1.21.11 has had a live end-to-end install check.

## Validation on Windows

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`: nine tests (three settings, six downloader tests).
- `cargo build --workspace`
- Live install of Mojang 1.21.11: 77 client/library/log files, 4,591 asset objects, 402 Java runtime files.
- Bundled Java: `java -version`, Microsoft OpenJDK 21.0.7, 64-bit.

Downloader tests cover hash/size mismatch, unsafe manifest paths, Windows rules, HTTP failure retry, corrupt response retry, exhausted attempts without a published bad file, partial cleanup and valid-cache reuse. Small mock HTTP servers use loopback only; production manifests require HTTPS artifact URLs.

## Not phase 1

No UI install button, native archive extraction, launch arguments/classpath, Microsoft login, game launch, loaders or mods. These come in later phases. The phase-0 Slint window still needs a safe interactive/visual test while the owner is away from the PC. Do not infer a working game launch from a successful download or Java version check.

CI is manual-trigger-only to avoid unapproved usage charges on the private repository. Phases 0 and 1 are merged into main; PRs may be merged after review and passing local checks under the owner's grant.
