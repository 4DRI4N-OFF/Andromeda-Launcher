# Native splash transition

The accepted v23 intro and logo paths are unchanged. After the intro and its one-shot sheen, the splash surface grows from its current size to the main window's 90% work-area bounds over 650 ms, with a bounded quintic ease. Its centre stays fixed. The logo stays at the original physical size, centred inside the growing surface, rather than stretching with the window.

Only after growth does the main window show at the same bounds, covered by the same splash background and logo. The existing first-render gate remains mandatory before hiding the splash. The menu then reveals over about 360 ms while the logo settles into its existing header slot over 550 ms. The layout is rendered at its final size, not scaled as a bitmap.

## Reduced motion and fallback

- The existing Reduce animations setting skips the intro, native growth and logo movement, but still waits for the first main frame.
- If the resize timer stalls for more than 100 ms, finish in one size change rather than trying to catch up with a burst of late resize calls.
- Set `ANDROMEDA_SIMPLE_RESIZE=1` for a manual single-step resize fallback if the Windows compositor visibly stutters despite timely callbacks. This keeps the first-frame check and menu/logo handoff. Unset it to restore growth. The automatic check detects a stalled application timer, not all compositor problems.
- Monitor lookup failure keeps the existing fallback placement path; it cannot provide native growth without a known target.

## Verification

Linux `cargo check -p andromeda-app` and 32 application tests pass. The deterministic `startup_resize_proof` example renders the actual Slint splash and main UI without desktop windows, accounts or game access. It does not execute or verify native Windows sizing, focus, borders, DWM, DPI or monitor transitions. Windows debug build and `cargo test --workspace` also pass: 32 app tests and 69 core tests. The two executables in the existing v23 test folder were replaced and independently hashed after checking that the old files matched the prior build and were not running. Both now have SHA256 `BC7E1B448FD1693AD9F146A377EF90CF8261BC8336B8472D2FAD8E03D6AF2B5D`. The previous executables are retained as `.before-sheen` (and the earlier pre-resize copies as `.before-native-resize`). Data was not changed and the portable was not launched. `4-Abrir-resize-simple.bat` is the optional manual resize fallback; `1-Abrir-splash.bat` remains the normal route. These are build and file-integrity checks, not a live visual test.

The proof covers the default dark theme at 1920x1040 work area, using 1248x702 splash and 1728x936 main surface. Light themes, reduced-motion UI, failures/retry during growth, compositor timing and real first-frame continuity need manual or further automated checks. The proof manually drives the UI properties; runtime ordering still needs a Windows check. No merge into main.

## Completion sheen

At 2.1 s, after the complete accepted v23 logo, one pearlescent band crosses from left to right over 900 ms. It is clipped to the same letter and planet Paths, not a rectangle painted over the background. A white core and pale-violet shoulders make it visible against the white logo without changing its final color. Solid-fill strips are used because Path gradients were not reproduced by the software renderer. Only the active band adds copies of the original geometry; no extra geometry moves. The original ten Path command strings and pre-sheen animation are unchanged. After a 120 ms white-logo pause, native growth starts. Reduced motion skips the whole intro, sheen and growth. No particles or audio.

The updated proof shows 168 actual Slint UI frames at 30 fps, from intro through sheen to the main layout. Every frame and the encoded video were visually checked. Windows build and 101 workspace tests pass. The portable was replaced without launching it, using source and executable hash checks and checking it was not open first. Actual Windows GPU performance, native resize, focus, first-frame timing, DPI and multi-monitor behavior still require the user's manual test. The offscreen proof drives properties directly and is not a runtime capture.
