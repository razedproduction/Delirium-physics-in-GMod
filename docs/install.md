# Installing DMS

DMS puts Delirium, an active ragdoll in the NaturalMotion style, into Garry's Mod: Homer, a ragdoll whose muscles
keep him standing, so he balances, stumbles, braces and falls on his own and reacts to shots, blasts, fire and blows
as he does in the Delirium Playroom. DMS is Homer only (docs/adr/0036); NM on GMod's own humanoid ragdolls, NPCs and
Workshop models is a separate project, `Delirium-ragdolls-in-GMod`.

DMS comes in two parts:

- **the DMS addon on the Steam Workshop** (the Lua part):
  https://steamcommunity.com/sharedfiles/filedetails/?id=3829811829
- **`DMS-<version>-win64.zip` from GitHub Releases** (the physics, the module and the tool that makes Homer's model,
  with `install.bat`): the Workshop doesn't take these files.

Both have to be the same release: Steam updates the addon by itself, so install the new zip when a new DMS comes out.

## What you need

- **Garry's Mod launched in 64-bit** (the stable branch has it: the folder `GarrysMod\bin\win64`), on **Windows**.
  32-bit, Linux and dedicated servers aren't supported (see the FAQ).
- **Your own copy of GTA IV**: the PC version (the folder with `GTAIV.exe`) or Xbox 360 (the folder with
  `default.xex`). DMS reads Homer's body, poses and NM's character from it. No game files come with DMS and none are
  copied anywhere: everything is read from your copy on your machine.
- Any x86-64 CPU: **AVX2 isn't needed**.

## Install

1. Subscribe to the DMS addon on the Workshop (link above).
2. Close Garry's Mod.
3. Unpack `DMS-<version>-win64.zip` anywhere.
4. Double-click `install.bat` in the unpacked folder and choose **1** (install and make Homer's model). It finds Garry's
   Mod through Steam (or asks for its folder) and asks for your GTA IV folder; you can drag a folder onto its window.

The menu:

| Choice | What it does |
|---|---|
| 1. Install or update DMS and make Homer's model | the files below, then Homer's model from your GTA IV |
| 2. Install or update DMS only | the files below; Homer's model stays as it is |
| 3. Make Homer's model only | after a DMS update that asks for it, or another GTA IV folder |
| 4. Uninstall DMS | see [Uninstall](#uninstall) |

What `install.bat` puts where:

| What | Where |
|---|---|
| `bin\vphysics.dll` (VPhysics-Box3D, the physics DMS needs) | `GarrysMod\bin\win64\vphysics.dll`; Steam's own is kept once as `vphysics.dll.original` |
| `bin\gmsv_dms_win64.dll` (the DMS module) | `GarrysMod\garrysmod\lua\bin\` |
| your GTA IV folder | its path into `garrysmod\data\dms\game_dir.txt`, then `bin\dms-homer.exe` makes Homer's model |

It also removes `garrysmod\addons\dms` if an older DMS (0.1.0) put its Lua there: the addon is the Workshop's now. If
the Workshop item isn't downloaded yet, it says so and offers to open its page in Steam.

`dms-homer` builds `garrysmod\models\dms\homer.mdl` (and its materials) from your GTA IV with Garry's Mod's own
`studiomdl`: Homer's physics is NM's body exactly, drawn as the Playroom draws it (coloured capsules and boxes). The
model is yours: it isn't shared and can't be.

### By hand

The same steps without the script: subscribe to the Workshop addon, copy the two dlls as in the table (keep a copy of
the original `vphysics.dll`), then run `bin\dms-homer.exe --game "<GTA IV folder>"` and, in Garry's Mod,
`dms_game_dir "<GTA IV folder>"`.

## First steps

Start a local game (single player or a listen server you host) on any map.

- `dms_homer` in the console (`~`): Homer stands in front of you, balancing.
- `dms_menu` opens the **DMS window** (bind a key in its Homer tab, or hold `C` and click the DMS icon): spawning,
  the Playroom's reactions and switches, behaviour tuning, GMod's ragdolls, performance, and a Debug tab with versions
  and checks.
- Toolgun → Tools → **DMS**: put NM on or off a Homer or a ragdoll, reactions where you hit him, his behaviours, a
  target to look at or point at, an inspector, a kick, throwing Homer, health, pins.
- Shoot him, hit him, blow him up, set him on fire, drop him from a roof or into water, throw things at him: he reacts
  as the Playroom's Homer does.
- `dms_selftest` checks that NM's data reads from your GTA IV; `dms_info` shows the versions and anything wrong.

The server's settings (console commands, the window's server tabs, the tool) are for superadmins: in single player
and on a listen server you host, that's you.

## FAQ

**The console says "vphysics.dll isn't VPhysics-Box3D" / DMS is disabled.**
Steam's *Verify integrity of game files* (and some Garry's Mod updates) put the original `vphysics.dll` back. Run
`install.bat` again and choose 2.

**"gmsv_dms isn't installed".**
The module isn't in `garrysmod\lua\bin`: run `install.bat` (choice 2).

**Nothing of DMS in the game: no `dms_homer`, no DMS window.**
The Workshop addon isn't subscribed or downloaded yet: subscribe (link above) and restart Garry's Mod.

**Homer doesn't appear / `dms_homer` says the model isn't there.**
Homer's model is made on your machine: `install.bat`, choice 3. If the console says "NM's data isn't loaded" or "No
GTA IV folder", DMS doesn't know where your GTA IV is: `install.bat` (choice 3) or, in the console,
`dms_game_dir "<GTA IV folder>"` and restart Garry's Mod (the data loads once a game). The DMS window's Debug tab lists
what's missing.

**"gmsv_dms_win64.dll is X but the Lua addon is Y".**
The module and the Workshop's addon come from different releases (usually Steam has updated the addon already):
download the zip of the addon's release and run `install.bat` again.

**"homer.mdl was made by an older dms-homer".**
The new release changed Homer's body: make the model again (`install.bat`, choice 3).

**"Run as administrator" / can't write `vphysics.dll`.**
Garry's Mod's folder isn't writable for you (a library under `Program Files` with changed permissions): right-click
`install.bat` → *Run as administrator*.

**Does it work on a dedicated server or Linux?**
No. Garry's Mod's x86-64 dedicated server is built against newer physics interfaces than the game, and VPhysics-Box3D
doesn't run there; there's no Linux build either. DMS runs in the game: single player or a listen server.

**Does 32-bit Garry's Mod work?**
No: only Garry's Mod launched in 64-bit.

**Do other players on my listen server need DMS?**
The physics and the module are the host's only, and the addon's client part (Homer's face, the window, the tool)
comes to them from the server as Garry's Mod sends any addon's Lua. Homer's model doesn't: it's made from GTA IV and
isn't shared, so a player without their own (`install.bat`, choice 3) sees an error model.

**Does DMS change my other games or Garry's Mod's physics?**
VPhysics-Box3D replaces Garry's Mod's physics engine for everything (it's a Box3D port of VPhysics), not only for
Homer; `sv_gravity` and the rest of the game aren't touched. To go back: [uninstall](#uninstall).

## Uninstall

`install.bat`, choice 4: puts Steam's `vphysics.dll` back and removes the module. Then unsubscribe from the DMS addon
on the Workshop. Your Homer model, its materials and `garrysmod\data\dms` (profiles, rigs, the GTA IV path) stay;
delete `garrysmod\models\dms`, `garrysmod\materials\models\dms` and `garrysmod\data\dms` by hand if you like.

## For modders

The Lua API (`dms.Attach`, controllers, hooks, reactions): `docs/api.md` in the repository
(https://github.com/razedproduction/Delirium-physics-in-GMod). Building from source: the repository's README.
