# Phase 5: Fabric and Quilt checkpoint

Private development branch and draft PR. Phases 2-4 are integrated into main. Phase 5 has no merge, release or public-repository approval.

## Implemented

- Fabric 0.20.1 and Quilt 0.30.2 profiles for Minecraft 1.21.11. Exact game/loader/client-entrypoint checks, strict Maven paths, required mappings and repository allowlists.
- Bounded HTTPS downloads without redirects, official metadata/Maven sidecar SHA-1 checks, size checks and publish-after-verification writes. Links and Windows reparse points are rejected. Corrupt existing files fail without being overwritten.
- Verified pinned profiles retain library digests and sizes. Later launches check the local files without requiring metadata downloads. A missing library can still require a download. Partial/corrupt manifests fail safely.
- Loader libraries precede vanilla libraries. Vanilla JVM/game arguments and loader overlay arguments are preserved. Microsoft sessions use the existing checked-account path; local profiles do not read account credentials.
- Optional per-instance loader. Existing metadata remains vanilla. Fresh Fabric/Quilt instances do not convert old instances or worlds.
- Explicit Create Fabric and Create Quilt buttons with pinned versions and correct instance labels.

## Verified November 10, 2026

- 60 workspace tests (57 core + 3 app), formatting, strict all-target Clippy and Windows app build passed.
- Eight Fabric libraries and eleven Quilt libraries installed and rechecked.
- Cached Fabric/Quilt launch plans passed with HTTPS redirected to an unreachable local proxy. A fresh-cache negative control failed as expected. This tests launcher installation/plan offline behavior, not a machine-wide network disconnect.
- Fabric create/read, duplicate, recoverable trash and restore preserved loader metadata in newly created test-only instances.
- Actual launcher UI created separate disposable Fabric and Quilt instances. Real mouse selection was visually checked before Play.
- Fabric authenticated game menu and Mod Menu details passed. A later local-profile launch also reached the menu with 50 mods; log identity was LoaderTest and graceful exit was 0.
- Quilt authenticated game menu and Mod Menu details passed. The title showed Quilt Loader 0.30.2 with 50 mods. Log identity was _4DRI4N_. The UI-created Quilt instance is i-18e69ad635205724-18f8-0.
- Working mod set on both loaders: Mod Menu 17.0.1, Fabric API 0.141.3+1.21.11 and Text Placeholder API 2.8.2+1.21.10. Modrinth lists the latter as compatible with 1.21.11 despite the suffix. Jars were SHA-512 checked; copies used for Quilt were identical. Mod Menu metadata explicitly lists Fabric and Quilt.
- Actual UI/game screenshots were inspected. Proof is the working mod UI, not only process presence or a log.
- Test game and launcher windows were closed. Subsequent process read found no Java or Andromeda processes.
- Test executables remain separate from the normal phase-3 exe. The normal exe was checked byte-identical after the build and restored from its pre-build copy.

## Caveats and remaining work

- First Fabric modded run using API 0.141.6 crashed with access violation 0xc0000005 in lwjgl.dll. A no-mod control and the older API set worked. The cause is not proven; this is still an unresolved reliability caveat.
- Earlier automation selected default vanilla rather than the intended instance. Those windows closed at the menu, without entering worlds. A real click and visible label then confirmed each loader selection.
- No world creation, world conversion or in-world gameplay has been tested here. No existing worlds were entered.
- UI restart persistence, UI duplicate/trash flow, and return-to-ready after the final modded close still need explicit visual coverage. Headless serialization/duplicate/trash checks passed for Fabric.
- Quilt offline launch-plan coverage passed; actual local-profile Quilt game coverage remains.
- Forge and NeoForge installer research has started, but neither is implemented. They require their processor-based installers, not Fabric-style classpath overlays alone.
- Disposable instances and checked mod jars are retained for repeat tests. No unrelated data was deleted.
- Phase 5 remains incomplete and the PR stays draft.

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
