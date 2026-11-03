# 0036. The project split: this repository is the golden Homer project; GMod's ragdolls move out

- Status: accepted
- Date: 2026-11-04
- Refines: 0004, 0008. Moves out: 0025–0032 (their work lives on in the ragdolls repository)

## Context

Until 0.2.0 one repository held two projects of a different kind:

- **The Playroom in Garry's Mod** (M0–M8): Homer's body, imported from the player's GTA IV, driven by Delirium in
  FOLLOW on the VPhysics-Box3D fork, held to the Playroom (`delirium-room`) one to one (ADR 0008).
- **NM on GMod's own ragdolls** (M9–M12.5): Homer's build fitted to any humanoid model (`tools/dms-fit`), native NM
  bodies under the model's ragdoll, ValveBiped and Workshop skeletons, NPCs' death ragdolls, live NPCs and the drunk
  player, the rig editor (`tools/dms-rig`).

The second has no reference to be held to (the Playroom can't load a fitted build) and keeps adding behaviour of its
own (a fitted body stays down instead of getting up, the swap from the model's ragdoll, the player's camera). In one
module and one addon, every such branch sits in the golden code path (`BodyDef::fitted`, `Follower::lead`, the bench's
`fit=`, `DMS_Native` in the reactions, the menu's ragdoll section) and every change to it has to be shown not to touch
Homer.

## Decision

1. **This repository (`Delirium-physics-in-GMod`) is the golden project: Homer only.** The Playroom stays the
   reference for everything in it (ADR 0008); nothing here drives a body that isn't Homer's. Removed: `tools/dms-fit`,
   `tools/dms-rig`, `module/src/ragdolls.rs`, the bench's `fit=` and `DMS.BenchFitted`, the fitted LEAD agent
   (`Follower::new_lead`), `BodyDef::fitted`, `golden fit` and `golden body --fit`; in Lua `ragdolls.lua`,
   `cl_ragdolls.lua`, `skeletons.lua`, `rig_view.lua`, `cl_rig_view.lua`, `deaths.lua`, `live.lua` and their convars,
   menu rows and branches. The addon needs `VBOX3D_NM_004` again (Homer's bind), not 006.
2. **GMod's ragdolls, NPCs, Workshop models and the rig editor move to a new repository,
   `razedproduction/Delirium-ragdolls-in-GMod`**, made as a copy of this one with its history (M0–M12.5 as they were
   at the split), then cleared of Homer: no `dms-homer`, no Homer model, spawn or Homer golden. Its module and addon
   get names of their own, so both can be installed side by side on the same fork `vphysics.dll`, and its own
   Workshop item; the Workshop item 3829811829 stays this project's.
3. **The core is this repository's, and the ragdolls repository uses it read only**, as this one uses the SDK
   (ADR 0001): a sibling checkout and path dependencies, never a copy. The core is the fork (`vphysics/`), its host
   and step without Lua (`tools/dms-vphysics`, which now also has the hulls as the fork grows them and the `.phy`
   reader, moved out of `dms-homer` so the core doesn't depend on Homer's model), NM's data from the game
   (`tools/dms-game`) and the bench's channel (`tools/dms-ctl`). Fixes to the physics are made here, held to the
   Playroom, and reach the ragdolls project from here. What of `module/` it needs (the FOLLOW loop, the room layer,
   the Lua bridge) is made a library here first, without changing Homer's behaviour (a later step, its own PR, checked
   by the golden comparison).
4. The fitting template stays NM's own: a fitted body is Homer's *build* (NM's data from the player's game, read by
   the core) fitted to a model's skeleton. "No Homer" in the ragdolls repository means no Homer body, model or golden,
   not no NM data.

## Consequences

- Golden runs here cover everything this repository ships; there's no code path for other bodies to keep apart.
- Until the module is a library, the ragdolls repository keeps its own copy of `module/` from the split; changes to
  the shared FOLLOW code in that time are made here and ported there by hand.
- ROADMAP M9–M12.5 are closed here as moved; their ADRs (0025–0032) stay in the index as history, their code is in
  the ragdolls repository.
