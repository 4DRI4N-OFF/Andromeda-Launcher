# Phase 5: four-loader checkpoint

Private development branch and draft PR. Phases 2-4 are integrated into main. Phase 5 has no merge, release or public-repository approval.

## Implemented

- Fabric 0.20.1, Quilt 0.30.2, Forge 61.2.5 and NeoForge 21.11.48 profiles for Minecraft 1.21.11. Exact game/loader/client-entrypoint checks, strict Maven paths, required mappings and repository allowlists.
- Bounded HTTPS downloads without redirects, official metadata/Maven sidecar SHA-1 checks, size checks and publish-after-verification writes. Links and Windows reparse points are rejected. Corrupt existing files fail without being overwritten.
- Verified pinned profiles retain library digests and sizes. Later launches check the local files without requiring metadata downloads. A missing library can still require a download. Partial/corrupt manifests fail safely.
- Loader libraries precede vanilla libraries. Vanilla JVM/game arguments and loader overlay arguments are preserved. Microsoft sessions use the existing checked-account path; local profiles do not read account credentials.
- Optional per-instance loader. Existing metadata remains vanilla. Fresh loader instances do not convert old instances or worlds.
- Explicit create buttons for all four loaders, pinned versions and correct instance labels.
- Forge-family uses SHA-256-pinned upstream client installers in separate launcher-owned roots. Installer logs, profiles, libraries and generated jars stay there, never in the real .minecraft folder. Every launch artifact is checked against embedded official SHA-1/size; generated libraries retain local SHA-256 integrity receipts. Processes have a ten-minute timeout.

## Verified November 10, 2026

- 63 workspace tests (60 core + 3 app), formatting, strict all-target Clippy and Windows app build passed.
- Eight Fabric libraries, eleven Quilt libraries, 52 Forge launch-library entries and 26 NeoForge launch libraries installed and checked. Forge metadata repeats some identical artifacts; conflicting duplicates are rejected.
- Cached plans for all four loaders passed with HTTPS redirected to an unreachable local proxy. A fresh-cache negative control failed as expected. This tests launcher installation/plan offline behavior, not a machine-wide network disconnect.
- All four loaders: create/read, duplicate, recoverable trash and restore preserved loader metadata in newly created test-only instances.
- Actual launcher UI created separate disposable Fabric and Quilt instances. Real mouse selection was visually checked before Play.
- Fabric authenticated game menu and Mod Menu details passed. A later local-profile Fabric launch also reached the menu with 50 mods; log identity was LoaderTest and graceful exit was 0.
- Quilt authenticated game menu and Mod Menu details passed. The title showed Quilt Loader 0.30.2 with 50 mods. Log identity was _4DRI4N_. The UI-created Quilt instance is i-18e69ad635205724-18f8-0.
- Working mod set on both loaders: Mod Menu 17.0.1, Fabric API 0.141.3+1.21.11 and Text Placeholder API 2.8.2+1.21.10. Modrinth lists the latter as compatible with 1.21.11 despite the suffix. Jars were SHA-512 checked; copies used for Quilt were identical. Mod Menu metadata explicitly lists Fabric and Quilt.
- Actual UI/game screenshots were inspected. Proof is the working mod UI, not only process presence or a log.
- Test game and launcher windows were closed. Subsequent process read found no Java or Andromeda processes.
- Test executables remain separate from the normal phase-3 exe. The normal exe was checked byte-identical after the build and restored from its pre-build copy.

## Caveats and remaining work

- First Fabric modded run using API 0.141.6 crashed with access violation 0xc0000005 in lwjgl.dll. A no-mod control and the older API set worked. The cause is not proven; this is still an unresolved reliability caveat.
- Earlier automation selected default vanilla rather than the intended instance. Those windows closed at the menu, without entering worlds. A real click and visible label then confirmed each loader selection.
- A new disposable NeoForge world named Instinct disposable Jade proof was created only inside its test instance. Actual rendered terrain/HUD was inspected, and graceful close saved every dimension. No prior world was entered or converted. Jade overlay behavior has not yet been proven; no in-world checks yet for the other three loaders.
- Real UI restart listed and selected the existing Forge profile with the correct label. Duplicate, armed trash confirmation, confirmed trash and Undo last trash passed on a new Forge-only copy; restored ID, loader and both mod jars were checked. Last selected dropdown row is not persisted. Return-to-ready after NeoForge closed was visually confirmed with Play enabled. Headless serialization/duplicate/trash checks passed for all four loaders.
- Quilt, Forge and NeoForge local-profile game menus passed with log identity LoaderTest and graceful exit 0, supplementing the Fabric local-profile proof. Loaded-mod UI was visually inspected for all four. These are local identities, not machine-wide network-isolation tests.
- Forge 61.2.5 authenticated live proof passed with Controlling 29.0.1 and Searchables 1.0.4, both SHA-512 checked. Inspected title menu and built-in mod details show them loaded.
- NeoForge 21.11.48 authenticated live proof passed with SHA-512-checked Jade 21.1.5+neoforge. Inspected title menu and Jade details explicitly show State: loaded.
- Four-loader installation and loaded-mod menu proof is complete, but phase-5 regression/gameplay coverage is not. Actual local-profile game-menu proof covers all four loaders.
- Disposable instances and checked mod jars are retained for repeat tests. No unrelated data was deleted.
- Phase 5 remains incomplete and the PR stays draft.

## Forge and NeoForge live checks

Both were created and selected through the real launcher UI in clearly named disposable instances. Microsoft profile logs showed _4DRI4N_. No world was entered during those authenticated menu checks. The later local-profile NeoForge test created only a fresh disposable world, as described above. The separate family build uses target-phase5-family, leaving the normal phase-3 executable unchanged throughout its build. Cached launch plans were rechecked after the final integrity tightening.

## Sources

- https://github.com/FabricMC/fabric-meta/
- https://meta.fabricmc.net/v2/versions/loader/1.21.11
- https://meta.quiltmc.org/v3/versions/loader/1.21.11
- https://meta.quiltmc.org/v3/versions/loader/1.21.11/0.30.2/profile/json
- https://api.modrinth.com/v2/version/cLkYaVuL
- https://cdn.modrinth.com/data/mOgUt4GM/versions/cLkYaVuL/modmenu-17.0.1.jar
- https://cdn.modrinth.com/data/P7dR8mSH/versions/i5tSkVBH/fabric-api-0.141.3%2B1.21.11.jar
- https://cdn.modrinth.com/data/eXts2L7r/versions/qxjzQ9xY/placeholder-api-2.8.2%2B1.21.10.jar

Modrinth responses, screenshots and test logs are retained outside the repository. No third-party jars, game assets, accounts or credentials are committed.

- https://files.minecraftforge.net/net/minecraftforge/forge/index_1.21.11.html
- https://maven.minecraftforge.net/net/minecraftforge/forge/1.21.11-61.2.5/forge-1.21.11-61.2.5-installer.jar.sha256
- https://maven.neoforged.net/releases/net/neoforged/neoforge/21.11.48/
- https://maven.neoforged.net/releases/net/neoforged/neoforge/21.11.48/neoforge-21.11.48-installer.jar.sha256
- https://api.modrinth.com/v2/version/mkgphVKG
- https://api.modrinth.com/v2/version/LSsLLBSG
- https://api.modrinth.com/v2/version/usEgI3B0
