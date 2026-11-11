# Andromeda intro v23: official train reference

Approved on 2026-11-11. Silent 1920x1080 video at 60fps: 126 frames, 2.1 seconds. The MP4 is a review reference, not played by the launcher.

All nine original glyphs stay at their final sizes and spacing. One common translation moves the row: `-1120 * (1 - ease(frame / 94))`, with quintic smootherstep. The row brakes together; the last 32 frames hold. No individual spawn delays, growth, impacts, spring, deformation or opacity fade.

The original planet contour and a hidden left-back halfplane conceal the row. The last A exits first. First visible pixels in ANDROMEDA order are at frames 72, 67, 63, 60, 57, 52, 50, 48 and 44. These are geometric exits, not separate delays. Planet motion, original paths, E and final transforms are preserved from the prior reference.

The source archive includes the Remotion scene, state export and geometric verifier. All 126 encoded video frames were reviewed in sequence, plus enlarged emergence and final views. The native `startup_proof` example compares all 126 native states against the reference fixture to within 1e-8.

## Build the video from the source archive

Extract the archive and enter `intro-v23`, then run:

```sh
npm ci
npx remotion render src/index.tsx AndromedaIntroV23 andromeda-intro-v23.mp4 --concurrency=2
npx esbuild src/check-state.ts --bundle --platform=node --outfile=state.cjs
node state.cjs
```

The original `verify-train.py` additionally expects the earlier `../intro-v17/states.json` comparison fixture. That earlier fixture is not part of this official archive. The standalone native parity fixture is retained in `app/src/startup_v23_parity.rs`.

See `STARTUP_PLAN.md` for native implementation, proof commands and outstanding desktop checks.
