# Grouped settings checkpoint

From interaction-motion commit 3fc198b, in its own worktree. No merge or normal-launcher replacement.

## Audit and before / after

| Before | After |
| --- | --- |
| Themes top-level tab, Themes and motion settings banner, Choose a theme button | One Appearance section inside Settings |
| Reduce animations in Settings and again in Themes | One switch inside Appearance |
| Header Settings/Play/Back button beside a navigation pill doing the same navigation | One Play / Settings pill |
| New profile and Selected profile in separate settings cards | Profiles contains creation and selected-profile actions together |
| Offline player name under selected-profile management, Microsoft controls elsewhere | Account & player contains both identities and the Use Microsoft for Play switch |
| Long stack of all settings cards | Three left-side sections with live summaries, right-side explanation and controls |

The contextual Manage profiles link in Play remains. It is a route into the one Profiles editor, not a second copy of its controls. Play's profile selection and header identity badge also remain as operational context, not duplicate editors.

No functional removal: all five loader creation actions, duplicate, reversible trash, undo, sign-in, cancel, sign-out, identity switch, offline name, profile/copy name, custom theme packs and reduced motion remain. The offline player name and profile/copy name are different values and were deliberately not merged. The identity switch was retained separately from sign-in, since sign-in and choosing the identity for Play are different actions.

No placeholder settings or new account/game services were added. The approved concept's left list + values / right explanation pattern is applied to existing functionality, not a claim to implement its full five-tab design.

## Verification

69 core tests, strict workspace/all-target Clippy, Windows build, theme proof and motion proof. `grouped_proof` renders every section and the three themes at 860x620, and uses real pointer events with callback spies to verify all five loader actions, duplicate/trash/undo, busy guard, account actions, section navigation, theme selection, identity and motion toggles. Spies do not sign in or mutate game files. Theme proof checks persistence and reduced motion. Motion proof checks overshoot, settlement and rapid retarget.

Pixel review found excess vertical stretching, which was fixed. Toggling motion off then on initially replayed page entry and shifted the switch hit target; the page now stays settled in both toggle directions and the pointer regression passes.

Native desktop/GPU/high-DPI, keyboard and screen-reader use, long lists, real Microsoft login and real game actions remain unverified. Backend actions are unchanged. No native window was opened during delivery. Settings are isolated with the existing `.andromeda-test` marker in the phase test folder.
