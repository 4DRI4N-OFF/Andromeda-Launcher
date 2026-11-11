# Home reference checkpoint

Branch: feat/reference-home. Base: feat/reference-shell at24c070c. Private draft PR, no merge or release. Stop for review before Instances, Mods, Settings redesign or any other next visual phase.

## Included

- Four-column Home composition: live profile cards plus New instance, at the reference1600x900 proportions and1920x1080 scale. Horizontal browsing supports additional profiles.
- Colored profile marks, real profile name/game version/loader metadata, selected primary Play and secondary Play controls. Existing launch callback, selection and busy guard remain.
- New instance opens existing Settings > Profiles. Account chip opens existing Account editor. No new fake account state.
- Footer shows actual launch status. No claimed Java/mod verification that did not happen.
- At860x620, cards shorten, unavailable activity labels hide, bottom actions stay reachable and horizontal browsing remains available. Header uses planet-only mark to avoid overlap.
- All three theme packs, fonts and motion settings remain.

## Official branding

Read-only source files supplied by Adrián:
- C:\Users\Administrator\Pictures\ANDROMEDA\andromeda.svg
- C:\Users\Administrator\Pictures\ANDROMEDA\planeta.svg

Unchanged originals are copied into app/ui/branding/andromeda-original.svg and planet-original.svg. Derived transparent header assets preserve the planet paths and outline the embedded Unbounded700 text, avoiding font substitution. White on dark themes, dark ink on Alba. The wide full logo replaces the earlier AL placeholder. The icon is the planet alone on a dark rounded tile, with multi-size Windows ICO and Slint window icon. Original exports are not changed or moved by this work. The existing animated intro is intentionally unchanged.

## Proof and limits

home_proof renders the actual compiled MainWindow at1920x1080 for all three themes and860x620, with a software renderer and callback spies. It does not open a native window or launch Minecraft. The three sample profiles are proof fixtures, not a claim about the user's current installed profiles. Real app metadata comes from profile storage.

The supplied Home reference is1600x900. The comparison places a uniform1.2x resize of that original at left and the unscaled1920x1080 component render at right. Full comparison3840x1080, each half1920x1080. Separate theme images are1920x1080.

This is not100% identical. Remaining differences:
- No fabricated weekly graph, play hours, last-session times or mock mod counts. The app does not yet collect that telemetry and states that it is not recorded.
- Procedural star/nebula/ring art differs from the supplied artwork, including star size, softness, bloom and noise. True backdrop blur is not verified. Panels are flatter and their translucency/material is different.
- English follows the project's language requirement. Reference sample text/names and font sizing/weight are not identical. The user's new official logo intentionally differs from the original reference branding.
- GPU/high-DPI, native titlebar, window icon appearance and Explorer icon cache remain desktop checks for the user. Software proof is not a native desktop screenshot. The embedded executable icon resource can be inspected separately without opening the app.
- Other tabs retain the previous checkpoint's real functionality. Mods remains explicitly unimplemented. This phase does not claim full Instances/Mods/Settings reference matching.

## Validation

Core tests, strict workspace/all-target Clippy and Windows build. Home pointer spies test selected/unselected Play, selection, busy guard, New instance route, Account route and minimum-size reachable Play. Grouped/theme/motion/shell proofs cover the existing callbacks, theme persistence and spring animation. No account login/game action in these proofs.

Normal executable/user worktree are not modified. Test executable replacement is permitted only while closed, with two process checks. No window is opened/focused, process killed, scheduled task or autostart installed.
