# Phase 3: isolated instances

Each instance has its own `data/instances/<id>/instance.json` and `game` directory. Downloads, Java, libraries, assets and natives remain shared under `data/vanilla`. Existing phase-2 saves stay untouched in the default profile.

## UI

- Select **Default profile (phase 2)** to keep using the previous game folder, or select an instance.
- Enter a new instance name and local player name, then **Create**. New instances use installed vanilla 1.21.11 and 2048 MB.
- Select an instance, enter the copy's name, then **Duplicate**. Saves, options and other game-folder files are copied, not linked. Launcher logs and the old lock file are excluded.
- **Move to trash**, then **Confirm trash**, moves the complete instance to `data/instances/trash/<id>`. It never permanently deletes saves.
- **Undo last trash** restores the latest instance trashed in the current app session. Older trashed instances can be restored with the CLI.
- After a management operation, the selector returns to the default profile. Select the intended instance before playing.

The local name recorded at creation is loaded when selecting an instance. Editing the player field affects that launch, not the saved instance metadata. Selection and the last-trash shortcut are session-only.

## CLI

```powershell
.\target\debug\andromeda-instances.exe list
.\target\debug\andromeda-instances.exe create "Survival" Player
.\target\debug\andromeda-instances.exe duplicate INSTANCE_ID "Survival copy"
.\target\debug\andromeda-instances.exe play INSTANCE_ID --dry-run
.\target\debug\andromeda-instances.exe play INSTANCE_ID
.\target\debug\andromeda-instances.exe trash INSTANCE_ID
.\target\debug\andromeda-instances.exe restore INSTANCE_ID
```

Use the ID returned by create/list, or the folder name in `data/instances/trash` for restore. Restore refuses to overwrite an existing active folder. There is no permanent-delete command.

## Safety

IDs, names, version, local profile and memory are validated. Instance roots and copied trees reject symlinks and Windows reparse points. Copies are staged before publishing, and failed copies are cleaned up. Existing staging data and restore destinations are not overwritten. Malformed instance metadata fails closed rather than being silently ignored.

On Windows, the game process, duplication and trash operation use the same exclusive lock, keyed by the canonical game directory in the current user's temporary folder. The lock is outside the instance so a whole instance can be moved while the guard stays held. Lock files are tiny and reusable. This prevents cooperating commands from copying or moving an actively launched world. It does not stop unrelated launchers or survive force-closing the launcher. Do not run old launcher builds against the same folders concurrently.

## Verified on Windows x64

- fmt, clippy all targets with `-D warnings`, 25 tests and workspace build.
- Unit tests cover isolation, deep-copy independence, trash/restore, lock contention, invalid metadata/IDs, staging collisions and restore conflicts.
- Actual UI create, duplicate, two-step trash and undo, with both instances visible.
- Actual UI launch of Phase3_A: its world list showed only a copy of the phase-2 test world.
- Actual UI launch of Phase3_B: no existing worlds; created, entered and saved `Andromeda_Instance_B` there.
- While B was running, CLI duplicate and trash were both rejected.
- After exit: A's saves contain `Andromeda_Phase2_Test`; B's saves contain `Andromeda_Instance_B`; the launcher returns to ready. Test app/game windows closed and existing settings preserved.

## Limits

Vanilla 1.21.11, Windows x64, installed files and local singleplayer only. No instance rename/settings editor, automatic migration, disk-space estimate, progress bar or cancel for large copies. UI copying runs off the event thread, but closing the launcher during a copy can leave a hidden staging folder. Large libraries of instances and alternate DPI/resizing have not been tested. Restore is a launcher trash folder, not the Windows Recycle Bin. No Microsoft sign-in or mods yet. Phase-2 launch integrity limits still apply.
