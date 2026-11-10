# Phase 5: loaders, Fabric checkpoint

Private development branch, draft PR. Phases 2-4 are now integrated into main after the owner's November 10 approval. This branch is based on main, not an unmerged phase-4 stack. No permission to merge phase 5, release or make the repository public.

## Implemented

- Fabric profile validation for exact game/loader, client entrypoint, strict Maven coordinates, required libraries and approved repository.
- Bounded HTTPS metadata/library requests, no redirects, official Meta or Maven-sidecar SHA-1 checks and temporary publish-after-verification writes. Links/reparse points are rejected in the loader cache. Corrupt existing cache is not silently overwritten.
- Verified Fabric libraries added before vanilla libraries; inherited vanilla JVM/game arguments and Fabric overlay arguments are preserved. Optional Microsoft sessions use the existing checked-account launch path.
- Optional per-instance loader field. Old instance metadata defaults to vanilla. Fresh Fabric instances do not convert existing instances or worlds.
- UI: explicit Create Fabric 0.20.1 button and Fabric/Vanilla instance labels. This version is named explicitly, not advertised as permanently latest.

## Verified November 10, 2026

- 49 Windows workspace tests (46 core + 3 app), formatting, strict all-target Clippy and separate Windows exe build passed.
- Installed and verified all 8 Fabric 0.20.1 libraries for Minecraft 1.21.11; a second headless run reverified/reused cache.
- Headless launch-plan check verifies client entrypoint, loader/intermediary classpath, inherited vanilla game args and Fabric JVM overlay without starting Java or reading account credentials.
- After the owner gave a safe testing window, inspected actual phase-5 UI pixels and used its button to create `Instinct disposable Fabric proof`, ID `i-18e69885d0537ea4-2b78-0`.
- Selected that instance with an actual UI click and used Play with Microsoft. Game logs confirm `_4DRI4N_`, Fabric Loader 0.20.1 and the installed mods.
- Successful visible combination: Mod Menu 17.0.1, Fabric API 0.141.3+1.21.11 and Text Placeholder API 2.8.2+1.21.10. Modrinth metadata lists this Placeholder API release as compatible with 1.21.11 despite its version suffix. Each downloaded jar was checked against its Modrinth SHA-512.
- Inspected actual game pixels: title menu says Minecraft 1.21.11/Fabric (50 Mods), Mods screen opens, and Mod Menu details show version 17.0.1. This is real loaded-mod UI proof, not only a process/log check.
- Test Java/launcher windows were closed. A subsequent process read found no Java or Andromeda processes. Tests ended around 06:48 CET, before the 07:10-15:30 no-PC-test period.
- Normal phase-3 exe remains 34,781,184 bytes, modified November 9 at 19:54:38. New executable is separate: `Andromeda-Phase5-Fabric-Test/andromeda-fabric.exe`.
- Bounded application-log pattern scan found zero token/code/verifier/Bearer/JWT-like matches. Tokens/launch arguments were not printed.

## Open caveats and remaining work

- First modded run with Fabric API 0.141.6+1.21.11, same Mod Menu/Placeholder API, crashed before the menu with access violation 0xc0000005 in lwjgl.dll. A no-mod Fabric control run worked; the older API combination above worked. This does not prove which component caused the first crash. Keep it as an unresolved compatibility/reliability caveat.
- Initial automation selected default vanilla rather than the new instance. Those windows were closed at the title menu without entering worlds. A real selection click then visibly confirmed the disposable Fabric instance before the successful test. No saved world was entered or converted.
- No world creation/in-world mod function was tested in this checkpoint. The proof is the working Mod Menu and authenticated game menu. Return-to-ready after the final modded close was not independently captured before the launcher closed.
- Explicit offline Fabric launch, restart persistence of the loader selection, and create/duplicate/trash loader regressions still need live coverage. Existing vanilla instance behavior retains its automated tests.
- Disposable instance and verified mod jars are retained and clearly named for repeat testing. No unrelated data was deleted.
- Quilt, Forge and NeoForge have not started. Phase 5 as a whole is not complete and this PR stays draft.

## Sources used

- https://github.com/FabricMC/fabric-meta/
- https://meta.fabricmc.net/v2/versions/loader/1.21.11
- https://cdn.modrinth.com/data/mOgUt4GM/versions/cLkYaVuL/modmenu-17.0.1.jar
- https://cdn.modrinth.com/data/P7dR8mSH/versions/i5tSkVBH/fabric-api-0.141.3%2B1.21.11.jar
- https://cdn.modrinth.com/data/eXts2L7r/versions/qxjzQ9xY/placeholder-api-2.8.2%2B1.21.10.jar

Modrinth project/version JSON used for compatibility, dependencies, file sizes and hashes is retained outside the repository in the phase-5 test folder. No third-party jars or game assets are committed.
