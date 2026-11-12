# Native startup draft

## Reference

The accepted intro is v23, the rigid letter train, approved on 2026-11-11. The video, source archive and geometric report are retained under `docs/startup-v23`. Earlier intros are not selected by this implementation.

- 2.1 seconds, 126 reference frames at60fps.
- One horizontal displacement for all nine original glyphs: `-1120 * (1 - smootherstep(frame / 94))`.
- Fixed final glyph sizes and spacing, no individual delays, deformation, impacts or opacity fades.
- Original planet motion and exact final paths/transforms preserved.
- Planet outer-contour occlusion plus a hidden left-back region; the whole row really moves, not a per-letter reveal timer.

## Implementation

`startup_runtime.rs` opens a small borderless topmost splash, starts the event loop, and dispatches local settings/themes/instance preparation to a worker. The moving icon reflects pending local preparation, not a made-up progress percentage. The protected session's presence is checked, but no token is read or refreshed by the startup worker. The existing independent account restoration remains in main UI preparation and is not a readiness gate.

The intro starts only after local data and main UI preparation. The old QOI intro is no longer called. Startup state and attempt tokens reject late callbacks after close or retry. Local preparation has a60-second timeout; first main render has an8-second timeout. Failure has explicit Retry and Close controls. Logging setup failure is handled as a visible error instead of a panic.

Slint Paths draw the moving logo. A static violet backdrop and a sampled outer-contour cover texture implement the original occlusion without dark patches. These are not prerecorded animation frames. Asset generation is reproducible with `scripts/generate_startup_assets.py`.

The main window has its header copy hidden while the final logo overlay is aligned to the splash in physical screen coordinates and converted by the main window's current scale factor. Transfer waits for `AfterRendering`, never just `show()` or a fixed delay. Position corrections require another rendered frame before hiding the splash. The overlay moves to the live header slot in0.55seconds, blending white into the theme's logo color. Reduced motion skips travel but still waits for an actual main frame. The same Path component stays at the destination, avoiding a final SVG/Path rasterization swap. The full wordmark remains present at860px; it is no longer silently replaced by a planet.

## Reproducible checks

- `cargo check -p andromeda-app`
- `cargo test --workspace`:91tests passed (22app,69core).
- `cargo test -p andromeda-app --example startup_proof`:3tests, including comparison of every one of the126native states to accepted v23 within1e-8.
- `cargo test -p andromeda-app --example startup_logic_proof`:15fixture tests, including readiness ordering, stale attempts, cancellation, failure/retry, reduced motion and preserving corrupt local files.
- `cargo run -p andromeda-app --example startup_proof`:40loading frames,126intro frames and failure controls, using the actual Slint software renderer with no OS window or account access.
- `cargo run -p andromeda-app --example startup_full_proof`:63handoff frames and3ready frames at1600x900,1100x700 and860x620,18theme endpoint/ready renders, plus a150%scale fixture. This does not call real main preparation, sign in, refresh a saved session or launch a game.

All126intro renders,40loader renders,63handoff renders, final layouts, theme endpoints and the failure layout have been visually reviewed. The150%fixture checks layout scaling only, not an actual monitor transition.

## Not verified by offscreen proof

This is a draft, not a release or merge request approval. Live Windows compositor behavior, topmost ordering, focus, decorations/client-origin offsets, actual monitor DPI changes, backend rendering notifier support and the real startup/handoff timing remain to be checked in an allowed desktop session. The normal launcher and main branch are not replaced. No live PC test during school hours or while its owner is using it.

API references:

- https://docs.rs/slint/latest/slint/struct.Window.html
- https://docs.slint.dev/latest/docs/slint/reference/elements/path/

## Centered, larger splash revision

Windows selects the monitor under the cursor (primary monitor fallback), reads its usable work area in a temporary per-monitor-aware thread context, and passes physical size/position to Slint before showing. It uses65%of usable width, a960physical-pixel minimum where there is room, and an85%cap on both axes, preserving exact16:9. A1920x1040work area gives1248x702;3840x2080gives2496x1404. Negative monitor origins and portrait/small work areas have pure geometry tests. Retry reselects and centers on the current cursor monitor. No display/DPI settings are changed. Actual multi-monitor desktop behavior still needs owner testing.

## Static original icon loader (selected revision)

The original compound path is shown without any motion during local preparation. No added arc, balance, spinner or extra cycle delay remains. Its center(960,540),size650 and exact path are the same as frame0of accepted v23, so readiness starts that intro directly. The earlier moving-icon proposal is superseded. The static original icon and native intro frame0are checked by software-rendered pixel comparison. No new desktop verification is claimed.

Windows API references:
- https://learn.microsoft.com/en-us/windows/win32/gdi/multiple-display-monitors-functions
- https://learn.microsoft.com/en-us/windows/win32/hidpi/high-dpi-desktop-application-development-on-windows

## Colored breathing background

During static-icon preparation, a precomputed violet/indigo/magenta backdrop blends over the original at0.65+0.2*sin(2*pi*t/2.8). Only texture opacity changes; no particles, network, runtime blur or background generation. No per-frame occlusion image is allocated while waiting because no glyphs need hiding. Once prepared, the current level fades to zero over300ms while the icon remains fixed; then the original v23 begins with its unchanged background, logo and handoff. Twenty breathing/link renders were inspected; the last link and first v23 frame match pixel-for-pixel. Actual desktop timing/performance still needs the owner's test.


## Nov 12: hide the v23 train with a native clip

Adrian reported hidden letters briefly showing behind the planet. The previous
composition still drew all nine letters, then painted a background-coloured RGBA
image over the left half-plane and planet silhouette. The 126 software-rendered
frames matched the accepted v23 video, so the reported desktop transient has not
been independently reproduced. Do not claim the breathing backdrop caused it.

The left half-plane now clips the glyph subtree before drawing it. This is the
same x < planet-x clipping rule in v23's original SVG mask. The existing exact
silhouette cover remains for the planet's ring holes. No timeline, glyph position,
planet geometry, breathing/fade setting or main-window handoff changed. With
occlude=false (main-window logo), the wrapper has zero offset and no clip.

Only offscreen checks are allowed for this revision. Final desktop confirmation
is still the user's manual check, not established by a successful build.

Verification: Windows debug build passed; workspace tests 22 app + 69 core passed.
All 126 new 720x405 intro frames are pixel-identical to the previous accepted
native implementation; comparison against v23 MP4 frames shows only the same
edge antialiasing differences as before (maximum 60 extra thresholded pixels).
All 126 1248x702 intro frames were visually inspected. All 22 loading/fade/static
fixtures are unchanged. Nineteen same-size theme/end/DPI fixtures are
pixel-identical. No desktop window was opened for these checks.
