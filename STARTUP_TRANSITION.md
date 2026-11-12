# Native splash transition

The accepted v23 intro and logo paths are unchanged. After the intro, the splash surface grows from its current size to the main window's 90% work-area bounds over 650 ms, with a bounded quintic ease. Its centre stays fixed. The logo stays at the original physical size, centred inside the growing surface, rather than stretching with the window.

Only after growth does the main window show at the same bounds, covered by the same splash background and logo. The existing first-render gate remains mandatory before hiding the splash. The menu then reveals over about 360 ms while the logo settles into its existing header slot over 550 ms. The layout is rendered at its final size, not scaled as a bitmap.

## Reduced motion and fallback

- The existing Reduce animations setting skips the intro, native growth and logo movement, but still waits for the first main frame.
- If the resize timer stalls for more than 100 ms, finish in one size change rather than trying to catch up with a burst of late resize calls.
- Set `ANDROMEDA_SIMPLE_RESIZE=1` for a manual single-step resize fallback if the Windows compositor visibly stutters despite timely callbacks. This keeps the first-frame check and menu/logo handoff. Unset it to restore growth. The automatic check detects a stalled application timer, not all compositor problems.
- Monitor lookup failure keeps the existing fallback placement path; it cannot provide native growth without a known target.

## Verification

Linux `cargo check -p andromeda-app` and 29 application tests pass. The deterministic `startup_resize_proof` example renders the actual Slint splash and main UI without desktop windows, accounts or game access. It does not execute or verify native Windows sizing, focus, borders, DWM, DPI or monitor transitions. Windows debug build and `cargo test --workspace` also pass: 29 app tests and 69 core tests. The two executables in the existing v23 test folder were replaced and independently hashed after checking that the old files matched the prior build and were not running. Both now have SHA256 `A5A45C848498CDA06957165FA91048326ACE4AA200FE4ACBCE935126DEDF78F8`. The old executables are retained as `.before-native-resize`. Data was not changed and the portable was not launched. `4-Abrir-resize-simple.bat` is the optional manual resize fallback; `1-Abrir-splash.bat` remains the normal route. These are build and file-integrity checks, not a live visual test.

The proof covers the default dark theme at 1920x1040 work area, using 1248x702 splash and 1728x936 main surface. Light themes, reduced-motion UI, failures/retry during growth, compositor timing and real first-frame continuity need manual or further automated checks. The proof manually drives the UI properties; runtime ordering still needs a Windows check. No merge into main.
