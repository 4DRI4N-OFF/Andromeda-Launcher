# Coolicons UI icons

Source: @coolicons/svg 5.3.0, line/regular SVG package, https://coolicons.cool/ . License: https://coolicons.cool/license . MIT Copyright (c) 2026 Kryston Schwarze. Full notice in LICENSE and embedded through app/src/main.rs for executable distribution.

Adaptations: currentColor replaced with white for Slint colorize. Stroke changed from 2 to 1.8 to retain the prior UI weight. Original 24x24 viewports and path geometry retained. These settings can also be adjusted on the Coolicons site.

- Play navigation and Home launch actions: play.
- Instances: box.
- Mods: puzzle.
- Settings: settings gear.
- Account navigation and player chip: user.
- New instance: plus.

All six semantic equivalents exist. The gear replaces Tabler's sliders; the user silhouette replaces its user circle. This is not an exact-shape match to Tabler. Branding planet, profile artwork, colors, image dimensions/positions, hit areas, callbacks and existing motion are unchanged. No icons added to text-only actions. No animation framework is added.

The branch is an isolated proposal based on the Tabler checkpoint, not a merge. Proof command: cargo run -p andromeda-app --example coolicons_proof. It renders the real Slint component offscreen at 1920x1080 across five routes and three themes, plus 860x620, and checks pointer callbacks. It does not open a desktop window, sign in or run a game. New execution results are reported separately rather than inferred from the old Tabler proof.

Executed on Windows, 2026-11-11: cargo check -p andromeda-app PASS; cargo test --workspace PASS; coolicons_proof PASS for the checks above. All 15 native offscreen route/theme captures and minimum-size capture were visually reviewed. No live/native-window or account/game launch test.
