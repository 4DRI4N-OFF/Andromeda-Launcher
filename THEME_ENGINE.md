# Theme engine - phase 1

Private development phase. Do not merge without Adrián's OK.

## Included

- Settings > Appearance > Themes and motion, with Horizonte as the default.
- Built-in Horizonte, Alba and Cartógrafo JSON packs in `core/themes/`.
- Shared semantic colors across Play, Settings, inputs, buttons and profile rows.
- Bundled Inter, Unbounded, Fraunces and Sora fonts, each with its SIL Open Font License. JetBrains Mono supplies Cartógrafo's small values/footer.
- A 400 ms color crossfade, staggered accent changes (navigation, profile list, panel), short display-font fade, and a damped 500 ms selection spring. Button compression stays inside its hit box.
- Reduce animations disables the theme transition and button animation. Enabling it mid-transition settles on the next timer frame.
- Theme ID and reduced motion persist in `data/settings.json`. Invalid or missing theme IDs fall back to Horizonte. Old settings remain readable.
- Custom JSON packs in `data/themes/`, loaded on restart. Use a built-in file as the template and change its ID. Schema version 1 rejects unknown fields, unsafe IDs, unknown fonts/backgrounds and malformed colors; oversized files and duplicate IDs are skipped. Packs are data, not scripts. At most 64 custom files are considered.
- Small Instinct credit in Appearance and this repository's README.

## Verification

On Windows, from the isolated theme-engine worktree:

```
cargo test -p andromeda-core --lib
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p andromeda-app
cargo run -p andromeda-app --example theme_proof
```

The offscreen proof uses the actual `MainWindow` and theme adapter with Slint's software renderer. It does not create a native window, read accounts, touch a real instance or change the user's settings. It renders all three packs, Play, Settings and 860x620 minimum size. It dispatches a real pointer press/release on Alba, verifies its selected ID, saves to a disposable settings file, and reloads that ID and reduced-motion setting. It asserts animated settling, immediate reduced motion, and enabling reduced motion during a transition. Proof BMPs are local test output, not repository assets.

## Deliberately not claimed

This phase adds the theme engine to the current two-screen Play/Settings structure. It is not a pixel-for-pixel reproduction of the proposed five-tab design. True backdrop blur, galaxy artwork, full constellations, seasonal overlays, a visual custom-theme editor, animated tab navigation and cascading lists are not implemented. The glass is tinted translucent panels, not a native blurred compositor. Native GPU rendering and live desktop interaction are not yet visually verified. No game-launch regression was run for this UI-only phase.

Custom packs are loaded at startup, not watched live. The system's own reduced-motion preference is not read; the launcher's Reduce animations setting is used. `dark_mode` remains as compatibility metadata while the selected theme controls the palette.

## Font sources

Bundled from the Google Fonts OFL repository:

- https://github.com/google/fonts/tree/main/ofl/inter
- https://github.com/google/fonts/tree/main/ofl/unbounded
- https://github.com/google/fonts/tree/main/ofl/fraunces
- https://github.com/google/fonts/tree/main/ofl/sora

## Credits

Theme-engine implementation assistance: Instinct.
