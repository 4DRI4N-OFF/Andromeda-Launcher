# Original-pose logo intro

A six-second 30 fps splash uses pre-rendered frames from the owner-supplied Blockbench logo. Entry/exit and the small landing bounce use only native translation keyframes; the original geometry, pivots, rotations and textures are unchanged. The hold returns to the original pose. No PNG hold plate is used. Renderer antialiasing differs slightly from the reference PNG.

The UI uses contain fit, never stretching the asset. The intro can be clicked to skip. When the saved Reduce animations setting is enabled, the intro is skipped entirely and no frame timer starts. A frame-decode error opens the launcher instead of blocking startup. Frame timing uses elapsed time so slow rendering does not extend the six-second intro.

The editable Blockbench copy and preview video are kept separately on the owner's PC; the repository contains only the rendered splash frames. Visual/interaction testing of the Slint window on the owner's desktop is pending while the PC is in active use. Source frame imagery has been inspected, but this is not proof of in-app playback speed or click behavior.
