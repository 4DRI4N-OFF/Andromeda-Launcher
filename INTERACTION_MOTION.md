# Interaction motion checkpoint

Based on theme-engine commit 31a4164 in a separate worktree. No merge or normal-launcher replacement.

## Response

The shared analytic damped oscillator uses natural frequency 24 radians/second and damping ratio 0.64. A retarget captures both current position and velocity, including rapid reversal. It settles exactly after 800ms to stop small residual movement. Unit tests cover initial conditions, overshoot, settlement, retarget continuity and finite outputs.

- Buttons: asymmetric press compression and release rebound inside fixed, clipped hit boxes. Text moves with the visual, not separately. Disabled controls do not compress or fire.
- Theme cards: bounded press compression plus the existing theme-selection spring.
- Fields: spring-driven focus border weight. Existing text editing and hit areas are unchanged.
- Switches: spring-driven knob, bounded to the track.
- Profile rows: press nudge, then staggered opacity/offset on page entry. Theme cards enter in the same capped 35ms cascade.
- Header: Play, Settings and Themes use the same existing screens. The active pill follows a velocity-preserving spring; text contrast follows its position.
- Pages: entering content fades and slides up with bounded overshoot. This is an entry transition, not an overlapping outgoing-page crossfade. Minimum page opacity avoids a blank flash. No five-tab redesign is included.
- Reduce animations bypasses all these trajectories, resets moving springs, and makes page/list entry immediate. Theme crossfade still follows its existing reduced-motion rules.

One 16ms clock is shared by all controls. Only changing values produce visual updates. The timer lives with the window. This is not a general-purpose high-performance animation library.

## Proof

Run from the worktree:

```text
cargo test -p andromeda-core
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p andromeda-app --example theme_proof
cargo run -p andromeda-app --example motion_proof
cargo build -p andromeda-app
```

`motion_proof` renders the actual Slint MainWindow in software, drives the production spring callbacks with a deterministic 30fps clock, sends real pointer press/release to Play, Settings, Themes and a theme card, and writes 150 frames. It asserts pill overshoot and settlement, rapid retarget, and reduced motion during a transition and on later pages. All three themes are captured at 860x620. The example never opens a native window, signs in or starts a game. The Play callback is intentionally unbound in the proof.

Native GPU rendering, high-DPI scaling, live desktop input, long lists, keyboard navigation, screen readers and real-game regressions are unverified. This phase does not add keyboard-focus support to the pre-existing custom buttons.

## Ready-to-open test build

A `.andromeda-test` marker next to the executable opts into `data/` beside the executable. Without that marker, normal data selection is unchanged. The delivery folder is `Documents\Instinct\Andromeda-themes-test`. Future checked builds can replace its test executable only while it is closed. No scheduled updater, autostart or desktop changes are installed.
