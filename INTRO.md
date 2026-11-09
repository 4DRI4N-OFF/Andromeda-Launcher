# Logo intro

The approved logo intro is a six-second, 30 fps Blockbench render. All 181 frames are embedded as lossless QOI at 960 x 540. The UI uses contain fit to preserve the original aspect ratio.

- Per-letter damped jelly arrival, followed by an upward ANDROMEDA push.
- LAUNCHER enters at 1.25 seconds, lands at 1.85 seconds and settles by 2.63 seconds.
- Ornaments spin from 1.66 to 2.18 seconds, with side clearance while the text rebounds.
- Original logo pose held from 3.0 to 4.7 seconds, then the exit.
- Original Blockbench cubes, pivots and base transforms are preserved in the editable model.

The saved **Reduce animations** setting skips the entire intro. Clicking the intro skips playback. A decode error also opens the launcher instead of blocking startup.

QOI decoding writes directly into Slint's RGBA pixel buffer. The development profile optimizes the decoder and application so debug playback does not use the slow unoptimized PNG path. Playback uses elapsed time rather than counting timer callbacks, so delays do not extend the intro.

## Validation

Local Windows checks for this revision: fmt, clippy with warnings denied, all 12 tests and workspace build passed. All 181 embedded images decode at 960 x 540. Decode benchmark: median 19.20 ms, p95 19.41 ms, maximum 19.62 ms. This measures decoding, not full presentation time or display smoothness.

The approved source MP4 was reviewed across all 181 encoded frames, including consecutive enlarged crops during the rebound and ornament overlap. The hold matches the original-pose reference.

Interactive Windows playback, click-to-skip and setting persistence remain pending a safe idle-PC check. These are not established by the decode benchmark. No desktop interaction or capture is allowed while the owner uses the PC.

CI is manual-only. No paid workflow or release was run.
