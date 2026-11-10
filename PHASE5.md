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
