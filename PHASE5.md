# Phase 5: loaders (Fabric groundwork)

Separate stacked branch from phase 4. No earlier PR is merged and no test executable or saved instance is changed by this groundwork.

Order from the project plan: Fabric, then Quilt, Forge, NeoForge. Each loader needs a real loaded-mod test, not only a main-menu launch.

## Started

- Checked the official Fabric Meta documentation and current metadata for Minecraft 1.21.11 on November 10, 2026.
- Added strict Maven-coordinate path conversion and validation of the requested game/loader profile, client entrypoint, required loader/intermediary libraries, duplicate coordinates and allowed library repository.
- This module does not download jars, write instances or change launch arguments yet.

## Next implementation

1. Discover stable compatible loader versions through Fabric Meta, preserving the user's exact game selection.
2. Install each library using verified digest and size; obtain trusted Maven sidecar digests where Meta omits them. Bounded downloads, no zip installer execution.
3. Merge the Fabric client overlay into the already-verified vanilla launch plan. Preserve its inherited JVM/game arguments rather than hard-code an entrypoint alone.
4. Store loader selection per isolated instance, defaulting existing instances to vanilla. Never rewrite existing worlds to test.
5. Build a separate exe and test an isolated instance with a compatible small mod. Verify both Microsoft and explicit offline launch paths without logging launch tokens.

No Fabric support is claimed complete. Quilt, Forge and NeoForge have not started.

Sources:

- https://github.com/FabricMC/fabric-meta/
- https://meta.fabricmc.net/v2/versions/loader/1.21.11

Live observation: first stable loader returned was 0.20.1. This is a recorded observation, not a pinned permanent latest version. The returned profile included a JVM argument and inherited vanilla 1.21.11, so ignoring overlay arguments is not safe.

## Verified downloader checkpoint

The bounded headless installer is implemented. Redirects are disabled, metadata/jar sizes are capped, Maven paths are strict, directories/files reject links/reparse points, existing corrupt cache files are not overwritten, and every jar requires a SHA-1 supplied by official Meta or the official Maven sidecar. New cache writes are temporary and renamed only after digest/size verification.

On November 10, a live headless check installed and verified all 8 client libraries for Minecraft 1.21.11 / Fabric 0.20.1 in data/fabric-test-cache. A second run reused and reverified the same cache. No game, mod, account prompt or saved instance was opened or changed. 43 core tests and strict all-target Clippy pass.

Launch overlay integration, per-instance loader selection and real loaded-mod proof remain unfinished. The headless example is: cargo run -p andromeda-core --example fabric_check -- CACHE_DIR GAME_VERSION LOADER_VERSION.

## Launch/instance checkpoint (not visually validated)

Fabric launch-plan integration now prepends verified Fabric libraries to the inherited vanilla classpath, uses the Fabric client entrypoint and includes its JVM/game overlay arguments. The headless plan check passed against the installed vanilla base without starting Java or reading a Microsoft session.

Instances have an optional loader field; old metadata defaults to vanilla. Fresh Fabric instances can be created without converting or touching existing worlds. The UI has an explicit Create Fabric 0.20.1 action and labels Fabric/Vanilla instances. That initial version is explicitly named, not advertised as permanently latest. Both Microsoft and explicit offline paths use the selected instance loader. 49 workspace tests (46 core + 3 app) and strict all-target Clippy pass.

A separate Andromeda-Phase5-Fabric-Test/andromeda-fabric.exe was built, preserving the last phase-3 executable. It has NOT been opened yet. UI pixels, creating/selecting a fresh Fabric instance and a real loaded-mod game test are still unverified. No completion claim and no merge permission for phase 5. The UI/launch checkpoint needs a safe PC testing window before it counts as ready.

Earlier bullets describing only metadata groundwork or unfinished integration are historical checkpoints; this section is the current state. Per-instance loader storage and launch integration are implemented but not yet exercised by a real game. Quilt, Forge and NeoForge are still unstarted.
