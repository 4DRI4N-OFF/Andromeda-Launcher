# Tabler UI icons

Source: https://github.com/tabler/tabler-icons/tree/845a74971e5f39fe1360a9987e4ae327ef1c90b9

MIT. Copyright and permission notice in LICENSE, included through the Rust source include_bytes! for distributable retention. Outline icons use a 24x24 viewport. Adaptations: currentColor replaced with white for Slint colorize; stroke width 2 to 1.8 to preserve the existing icon weight. Branding planet and profile artwork unchanged.

## Mapping

- Play nav and Home launch actions: player-play (outline replaces solid/glyph).
- Instances: cube.
- Mods: puzzle.
- Settings: adjustments-horizontal.
- Account nav and player chip: user-circle (replaces decorative planet; branding is unchanged).
- New instance: plus (replaces font glyph).

No icons are added to previously text-only settings/action buttons. Profile art, dashed new-card border, brand logo, colors, hit areas, motion, callbacks and video assets are unchanged. This is an icon-family proposal, not a claim of a 100% reference match.

## Verification

`cargo check -p andromeda-app`, `cargo test --workspace`, and `cargo run -p andromeda-app --example tabler_proof`. The example renders the real Slint component with fixture profile data offscreen at 1920x1080 for all five routes and three themes, plus 860x620 minimum size; uses pointer events for navigation and callback spies for launch actions. It does not launch Minecraft, sign in, or open a desktop window. No native Windows launch or live account/game test is claimed.
