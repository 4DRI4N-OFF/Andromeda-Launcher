# Reference shell: first approval checkpoint

This is only the top bar and two-panel shell phase. It is not the completed reference redesign. Stop here for Adrián's review before Home cards, instance detail, Mods or loading work.

## Reference coordinates

The supplied A/B/C files are 1600x900. Their chat previews are smaller; those previews were not used as source dimensions. At 1600x900 the implementation uses 36px side and bottom margins, panel top 160px, 402px left panel, 24px gutter, and 27px panel radius. The 486x78px centered five-icon top pill is at y=30px, with 84x56px selected surface and the section label below. Reference sources remain unchanged, without stretching or re-proportioning. Comparisons place full-size originals and full-size component renders beside each other.

## Included

- Five reachable icon routes: Play, Instances, Mods, Settings, Account. The original two-tab header is replaced.
- Native-sized frame and left-side rows with current values, plus theme-specific display/value type.
- Existing profile selection, launch, all loader/create/copy/trash/undo actions, account controls and appearance controls remain reachable. Callback-pointer regressions verify the existing routes and actions.
- All three themes still work, including the JSON-pack engine and reduced motion. Springs now target five top positions. Custom theme packs keep their selected color; bundled themes use small rounded vector gradient fills.
- Decorative background assets are new procedural artwork, not the supplied reference screenshots embedded as a fake interface.

## Explicit differences still open

- This is not pixel-identical. Top/frame coordinates are matched; contents have not been rebuilt to match all reference screens.
- Play and Instances currently share the existing launch/profile content. Home cards, full instance-detail composition, Mods editor and loading modal are later gated phases. Mods explicitly says it is not implemented. No mock inventory, hours played, activity, backup guarantee, storage metrics, RAM/FPS setting or mod count is presented as real.
- The Instances sidebar has the real profile names, not the reference's example names. Search/count badges, loader subtitle rows, per-profile colored icons and the reference's bottom dashed creation button are not yet reproduced.
- The Settings sidebar contains the actual supported editors, not unsupported RAM/resolution/FPS/runtime controls copied from the concept.
- Glass surfaces use translucent theme colors. A rounded gradient in Slint's software renderer painted square corners, so panel gradients were not retained. The small top selection uses a rounded SVG gradient instead. True backdrop blur, matching light bloom/shadows, texture/noise, exact star/nebula/constellation artwork and per-theme material tuning remain different. No claim is made that Slint cannot support them on another rendering path; it is simply unverified here.
- Body text sizes and the wordmark still need a typography pass. The project stays in English as previously requested, whereas reference labels are Spanish.
- At 860x620, side values stack and right-side settings scroll; this is responsive adaptation, not a claim of identical reference composition at that size. Keyboard/screen-reader, native GPU/high-DPI, live desktop, actual login/game flow remain unverified.

## Evidence and safety

Screenshots are offscreen renders of the same compiled Slint component used by the app, with callback spies and representative current-style profile names. They are not a screenshot of a running native desktop window. No window was opened/focused and no account/game side effects occurred. A native user screenshot is still needed to verify the delivered executable's GPU/high-DPI appearance, because screenshots of this component alone do not prove that.

Work is isolated from the user's own working tree. No merge, release, updater, autostart or PC scheduled task is included. Test executable replacement is permitted only when closed; normal launcher and existing portable data remain unchanged.
