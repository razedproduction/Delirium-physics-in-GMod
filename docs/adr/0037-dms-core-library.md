# 0037. The module's FOLLOW core as a library: `crates/dms-core`

- Status: accepted
- Date: 2026-11-04
- Refines: 0036 (its point 3: "what of `module/` it needs is made a library here first"), 0003, 0016, 0021

## Context

After the split (ADR 0036) the ragdolls project (`razedproduction/Delirium-ragdolls-in-GMod`) uses this repository's
core read only, by path. `tools/dms-vphysics` (the fork's `IPhysicsNM`, the bind, the hulls, NM's step pieces without
Lua) and `tools/dms-game` (NM's data from the game) already were libraries; the rest of what a GMod module needs to
run FOLLOW agents lived only inside the cdylib `gmsv_dms`: the agent and the Playroom's room on it, the step
callback, the agents' natives, the map's world, the LuaJIT bridge. The ragdolls repository kept a copy of `module/`
from the split, to be ported by hand on every fix here.

Homer's behaviour must not change for it: the move is held to the golden comparison like any change to the body or
the loop (ADR 0008).

## Decision

1. **A library crate `crates/dms-core`** (`dms_core`, an `rlib`, a workspace member) holds, moved as they were, with
   no logic edits:
   - `lua` — the LuaJIT API from the loaded `lua_shared.dll`, `native!` (now `#[macro_export]`, re-exported as
     `dms_core::lua::native`; `$crate` is the core, so a host's natives run through the core's `lua::run`);
   - `ffi` — the fork's `IPhysicsNM` (`dms_vphysics::nm` re-exported) and the header's copy `vphysics_nm.h`;
   - `nm` (the SDK's C API), `data` (NM's data from the game), `tuning` (the player's tuning, ADR 0009),
     `world` (the map's `.bsp` as NM's world, ADR 0018);
   - `follow` with `follow/room` (the FOLLOW agent and the Playroom's room layer, ADR 0016, 0020), `step` (the step
     callback, ADR 0003, 0021);
   - the natives on them: `agents` (`Follow`, `FollowSend`, `FollowEvents`, ...), `react` (`FollowReact`,
     `SetReactions`, `SetTuning`, ...), `natives` (the debug natives on `IPhysicsNM`), `world::NATIVES`;
   - `json` (telemetry's, moved out of the bench so the agent's `telemetry` doesn't need it).
   `dms_core::unload` frees the statics as `gmod13_close` did.
2. **The core registers nothing.** Each module's `NATIVES` is a `(name, function)` list; the host module puts them on
   its own Lua table under its own global name (here `DMS`), and calls `step::shutdown` and `unload` on close. Two
   hosts built on the core are two DLLs with their own statics.
3. **`gmsv_dms` keeps only what is Homer's and the golden bench's**: `gmod13_open`/`gmod13_close` and the `DMS` table,
   `bind` (Homer's ragdoll on NM's body), `bench` and `dev` (the LEAD bench agent and `dms-ctl`'s channel), `selftest`
   and `nm_natives` (the reference lists). It brings the core's modules into its root under their old names
   (`use dms_core::{agents, data, follow, ...}`), so its own code reads `crate::follow` as before.
4. **The extension point for other bodies is data, not a trait: `follow::BodyDef`** (NM's build as the crate steps it,
   and the same body as NM's physics has it). Homer's is `BodyDef::homer`; a host adds its own with `BodyDef::new`
   (checked to have NM's 21 parts) and starts an agent on it with `agents::start_body`. The agent only ever reads a
   build and a body, so a trait would only add a layer; a body that isn't NM's 21 parts (head 12, thighs 1 and 4) is
   not something the agent or the Playroom's room can run, and isn't offered. There is no code path for a fitted body in
   the core: whatever the ragdolls project needs beyond that (its bind, the swap from the model's ragdoll, a body that
   stays down) is its own module's code, on the core's public API.
5. **The ragdolls repository depends on `crates/dms-core` by path, read only** (as on the SDK, ADR 0001): fixes to the
   shared FOLLOW code are made here, held to the Playroom, and reach it from here.

## Consequences

- Golden runs here still cover the whole of the shared code: it's the code Homer runs on (checked for the move with
  `golden body` and `golden behaviour --source` on the bench: the same result as before it).
- The core's public API is whatever its modules export (`pub`); it's not versioned apart from this repository. A
  change to it that breaks the ragdolls project is fixed there.
- Two modules on the core in one game share the fork's single step callback per environment (`SetStepCallback`
  replaces the one set before, `vphysics_nm.h`): side by side, each host's `step::attach` takes the environment from
  the other. Running Homer and the ragdolls project's bodies at once needs either one host or a fork change (more than
  one callback per environment, a new interface version), decided when the ragdolls project gets there.
- The copy of `module/` in the ragdolls repository is replaced by the dependency there (its own PR).
