# DMS 0.2.0

Delirium, an active ragdoll in the NaturalMotion style, in Garry's Mod. 0.2.0 puts GMod's own humanoid ragdolls under
NM next to Homer: NPCs and players die and get shot as the Playroom's Homer reacts, and Homer swims.

**Needs:** Garry's Mod launched in 64-bit on Windows and your own copy of GTA IV (PC or Xbox 360). Nothing of GTA IV is
included: Homer's model and NM's data come from your copy, on your machine. No AVX2 needed.

**Install:** subscribe to the DMS addon on the Workshop
(https://steamcommunity.com/sharedfiles/filedetails/?id=3829811829), close Garry's Mod, unpack
`DMS-0.2.0-win64.zip`, double-click `install.bat` and choose 1. Details and FAQ: `install.md`.

**Updating from 0.1.0:** the Lua addon now comes only from the Workshop, and `install.bat` replaces `install.ps1`.
Run `install.bat` and choose 2: it updates the physics and the module and removes 0.1.0's `garrysmod\addons\dms`
(otherwise it would run beside the Workshop's copy). Homer's model from 0.1.0 is still good. The physics' interface is
newer (`VBOX3D_NM_006`): 0.1.0's `vphysics.dll` doesn't work with 0.2.0.

## What's new

- **GMod's ragdolls under NM** (ValveBiped: HL2's citizens, combine, police, alyx, Workshop player models): an NM body
  fitted to the model's skeleton, the model posed from it with its own look (skin, bodygroups, colour). NM on and off
  any ragdoll from the DMS window, the tool or `dms_ragdoll`; a falling ragdoll keeps falling as NM takes it, and gets
  its own physics back moving. The physgun and gravity gun grab NM's body, bullets hit the part they hit.
- **Death ragdolls:** a killed NPC's or player's ragdoll gets NM in its dying pose and reacts to the killing hit as the
  Playroom's dying Homer (`dms_autoattach_npc`, `dms_autoattach_players`, at most `dms_autoattach_max` 4 at once).
- **Live bodies:** an NPC shot and not killed becomes its NM ragdoll, staggers or falls as the Playroom's shot reaction
  goes, and gets back on its feet where the body settled with its health and weapon (`dms_live_npc`, `dms_live_max`).
  `dms_drunk` (and a button in the window) makes you your ragdoll with the Playroom's drunk mode, the camera on it.
- **Other skeletons:** known rigs (Biped, Mixamo, Unreal, TF2, VRoid, MMD) by their bones' names, a guess for the
  rest, or a rig file of your own in `garrysmod\data\dms\skeletons` (reloaded as it changes); non-humanoids are refused
  with a message, models far from 1.8 m get a warning. `dms_rig_view` draws NM's bodies over the ragdolls.
- **Water:** Homer in the map's water as in the Playroom's pool (its buoyancy, the water's surface from the map).
- **The installer** is `install.bat` (a menu, no PowerShell): it finds Garry's Mod in any Steam library, checks the
  Workshop addon, remembers your GTA IV folder.
- Fixes along the way in the physics (NM's self-collision pairs, handing a body's motion over) and in sharing the
  Playroom's shot reaction between the module and the tools.

## Known limitations

- Windows, 64-bit Garry's Mod, single player or a listen server only: no dedicated server, no Linux, no 32-bit.
- Fitted ragdolls don't get up by animation (they stay down); a fitted body pushed steps further than its own NM
  reference; behaviours are tuned for ~1.8 m bodies.
- Not yet the Playroom's to the letter where Box3D's soft joints differ from Delirium's physics: a relaxing Homer settles
  at a different moment, the pose after tripping over a kerb or a low wall can differ; the drunk mode from a player's
  pose can fall where the Playroom's Homer doesn't.
- Ice isn't supported; the "learned" get-up (`dms_getup 3`) isn't ported.
- The rig editor (`tools/dms-rig`) isn't in the release yet: it's built from the repository.
- Steam's *Verify integrity of game files* puts Garry's Mod's own `vphysics.dll` back: run `install.bat` again.
- Players joining your listen server need their own Homer model (it can't be shared) or they see an error model;
  GMod's ragdolls under NM need nothing on their side.

## Licences

GPL-3.0-only (DMS and Delirium); `vphysics.dll` is VPhysics-Box3D under MIT with Box3D (MIT), Qhull and Source SDK 2013
code (Valve's Source 1 SDK License): `THIRD_PARTY.md` and `licenses\`. Source:
https://github.com/razedproduction/Delirium-physics-in-GMod (tag `v0.2.0`) and https://github.com/PVCPC/delirium-physics
(tag `v0.2.2`).
