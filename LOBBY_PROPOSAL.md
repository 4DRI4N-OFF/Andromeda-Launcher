# Lobby redesign: visual proposal

2026-11-12. Separate draft checkpoint, not an integrated launcher replacement.

## Reference interpretation

The supplied reference is a lobby: a horizontal navigation band, stacked mode
choices left, a large character in an open center, supporting panels right and
an action at the bottom. This proposal transfers that hierarchy to Andromeda:

- Jugar / Instancias / Mods / Ajustes / Cuenta.
- Instance choices on the left and a clear Jugar action below.
- A large 3D player in the center, with camera movement instead of a 2D drawing.
- Selected-profile and identity panels on the right. Hide those panels below
  1220 px rather than squeeze the player between unreadable sidebars.
- Original Andromeda logo at its own ratio, existing Andromeda backdrop,
  own synthetic skin, round panels and generous outer margins.
- No game logos, screenshots, military photography, rewards, ranks or missions.

## What runs

`cargo run -p andromeda-app --example lobby_proof` creates offscreen images only.
`cargo test -p andromeda-app --example lobby_proof` tests the mesh renderer.

The renderer uses six actual cuboids, perspective-correct textured triangles,
a depth buffer and a camera. The texture is an original 64x64 demonstration,
not a real player or an account request. The narrow/normal arm geometries differ.
The normal launcher entry point, splash, data and authentication are unchanged.

## Review status

The 60 final 1728x936 frames were inspected individually in contact sheets before
encoding the 2-second 30 fps video. Also inspected 1280x720 and 860x620 renders.
Reference and proposal were compared side by side without stretching either.
Selected-row and Jugar fills use solid colors so the software-rendered background
honors their rounded silhouette. Three geometry tests pass.

This is not a desktop test. No real mouse, keyboard, gamepad or launcher action
has been tested. Handlers change prototype selection/tab state; Jugar intentionally
has no callback. Other tabs are explicit placeholders, not completed screens.

## Next work, not delivered

1. Agree the lobby direction. Decide whether to retain the quiet space backdrop
   or create an original Minecraft-style setting with more scene depth.
2. Move the real instance list, launch status and busy guards into this layout.
3. Rebuild Instances, Mods, Settings and Account consistently without losing their
   current operations, settings, error states and theme packs.
4. Wire keyboard focus, scrolling with many instances, selected profile actions,
   and accessibility. Validate real input and desktop DPI only when permitted.
5. Fetch the real premium skin after verified sign-in. Support outer skin layer,
   old 64x32 skins, slim UVs, optional capes and caching. The current renderer is
   a demonstration, not complete Minecraft skin support or measured performance.

## Skin retrieval direction

The existing `core/src/auth.rs` already validates Java entitlement and reads
`https://api.minecraftservices.com/minecraft/profile`, but retains only id/name.
The documented response contains skins with ACTIVE state, URL and CLASSIC/SLIM
variant. Reuse that response rather than sending credentials to an avatar host.
Validate allowed HTTPS texture URLs, size, dimensions and content before decoding.
A public UUID skin lookup is not proof of license or ownership.

Sources read for feasibility:
- https://minecraft.wiki/w/Mojang_API
- https://mojang-api-docs.gapple.pw/no-auth/uuid-to-profile
- https://minestom.net/docs/feature/player-skin
- https://github.com/slint-ui/slint/tree/master/examples/opengl_texture
- https://github.com/slint-ui/slint/discussions/5599
- https://docs.rs/slint/latest/slint/struct.Image.html
- https://docs.rs/slint/latest/slint/struct.SharedPixelBuffer.html
- https://github.com/bs-community/skinview3d

The API guides are community documentation, not a vendor stability promise.


## Night scene choice, 01:52 Nov 12
The user chose a Minecraft-at-night backdrop. Replaced the quiet space image in
the proposal only with original generated voxel-night artwork: square moon,
layered block mountains, trees, blue fog and warm lights. It is a flat background
illustration with depth cues, not a live 3D world and not a game screenshot.
The model in front remains actual geometry. Background uses proportional cover
cropping, not stretching. No copyrighted game assets were imported.
