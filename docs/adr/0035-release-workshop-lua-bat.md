# 0035. The release from 0.2.0: the Lua addon from the Workshop, the installer a .bat

Status: accepted (2026-11-03). Replaces point 6 (the package) and the installer of point 8 of 0024.

## Context

0.1.0 shipped everything in one zip: the binaries, the Lua addon (copied into `garrysmod\addons\dms`) and a PowerShell
installer (`install.ps1`), run as `powershell -ExecutionPolicy Bypass -File …`. Players found the PowerShell step
awkward, and a `.bat` was added by hand to the "easy setup" re-release of 0.1.0. The Lua part is published on the
Workshop now (https://steamcommunity.com/sharedfiles/filedetails/?id=3829811829), so a copy in the zip would run beside
the Workshop's and drift from it.

## Decision

1. **The Lua addon is the Workshop's only.** The zip has no `addon\`; the user publishes the Workshop item from the
   repository's `addon\` by hand, with the same `DMS_ADDON_VERSION` as the release (`package.ps1` still checks it
   against `Cargo.toml`, ADR 0024's point 1).
2. **The installer is `install.bat`** (`scripts\release\install.bat`, plain cmd, no PowerShell and no execution
   policy): a menu — install or update and make Homer's model, install or update only, Homer's model only, uninstall.
   It finds Garry's Mod through Steam's registry key and `libraryfolders.vdf` (or asks; a dragged folder works), checks
   the x86-64 branch and that the game is closed, copies the fork (Steam's dll kept once as `vphysics.dll.original`)
   and the module, removes a `garrysmod\addons\dms` an older release left, says whether the Workshop item is downloaded
   (`steamapps\workshop\content\4000\<id>`) and offers its page in Steam; for the model it takes the GTA IV folder
   (the last one from `data\dms\game_dir.txt` offered), writes it there in UTF-8 and runs `dms-homer`. `package.ps1`
   writes the version into it and packs it in CRLF and ASCII.
3. **The package** `dist\DMS-<version>-win64.zip`: `bin\` (`gmsv_dms_win64.dll`, `vphysics.dll`, `dms-homer.exe`),
   `install.bat`, `install.md`, `LICENSE`, `THIRD_PARTY.md`, `licenses\`, the release notes. The rest of 0024 stands.

## Consequences

- One copy of the Lua on a player's machine, updated by Steam; the module from GitHub and the Workshop's addon still
  have to be the same release, or DMS doesn't start and says so (0024).
- A release is published in two places by the user: the zip on GitHub Releases and the addon on the Workshop.
- `scripts\install.ps1` (the developer's) still installs `addon\` into `garrysmod\addons\dms`: on a machine that is
  also subscribed to the Workshop item, both copies load.
