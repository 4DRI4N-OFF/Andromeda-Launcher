# Andromeda UI Lab — iOS 26 style comparison (Slint 1.19 vs Tauri 2)

Isolated experiment. Nothing here is part of the launcher product; `app/` and `core/` are untouched.

Goal: compare the same "Biblioteca" view in the Andromeda iOS-26 direction (light lavender,
floating glass nav, matte cards, spring motion) built twice:

- `slint-demo/` — Rust + Slint 1.19, Skia renderer (only renderer with `backdrop-blur`).
- `tauri-demo/` — Tauri 2 + static HTML/CSS/JS, CSS `backdrop-filter` glass.

Both run the same scripted demo (scroll, tab switch, card press) when launched in demo mode,
so the recorded captures are comparable. The GitHub Actions workflow `ui-lab.yml` builds both
on `windows-latest`, launches them, records ~13 s of window frames + memory/startup metrics,
and uploads them as the `ui-lab-results` artifact.

Assets in `shared-assets/` are original procedural SVGs (no external licenses needed).
