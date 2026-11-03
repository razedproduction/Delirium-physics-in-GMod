# 0032. NM in the rig editor: the FOLLOW step's pieces shared as `dms_vphysics::follow`

Status: **proposed** (2026-11-03). Refines 0016, 0017 and 0031 (its "first without NM, then with it").

## Context

The rig editor (`tools/dms-rig`, 0031) hosts the fork and makes the fitted body as the module does, but simulated it
without NM: whether a rig stands under NM could only be seen in the game. The module's agent (`module/src/follow.rs`,
`Follower`) is NM's FOLLOW step on the fork's bodies wrapped in the Playroom room's reactions (`room.rs`: tasks,
get-ups, `in_air`, the additional reactions), the game's data (`Data`), the tuning profile and the map's world; it
can't be lifted out whole without the game's state coming with it. Writing the editor's agent anew would be a second
table of the same step (0008, 0016 forbid that).

## Decision

1. The FOLLOW step's pieces that only read and write the crate `delirium` move from `module/src/follow.rs` into
   `tools/dms-vphysics/src/follow.rs`, unchanged: the Playroom's agent (`agent`, seed 0.3), its buttons' lines
   (`button_lines`), a message line parsed as its console does (`parse_line`, `parse_value`), `applyImpulse`'s params
   and its impulse on the fork's body (`impulse_params`, `kick`), the grab (`pull_params`), the poke (`poke_line`),
   the parts' states into NM's doll with the blow-up checks and NM's sleep kept (`set_state`, `keep_sleep`), the
   balancer's impulses (`balancer_unit`, `balancer_kicks`), the drives and armature out of the doll (`outputs`) and a
   physics step's forces with gravity's compensation (`forces`, 0007). The module's `Follower` is made of them; its
   behaviour doesn't change.
2. The editor's agent (`dms-rig`'s `sim::NmAgent`) is made of the same pieces and run the module's way: a step
   callback on the editor's environment (`SetStepCallback`), NM's 1/60 s steps on an accumulator over the 0.015 s
   physics steps (at most 4), the states read once a physics step, the armature set as it changes, the drives held
   between NM steps; the environment keeps Source's gravity and the compensation is the module's `nm` mode.
3. What the editor leaves out is the room: its reactions, tasks and get-ups (the module's; a fitted body stays down
   in the game too), the tuning profile (off, as in the Playroom, 0009) and the map's world (NM's floor is the
   Playroom's CONCRETE floor, with no pool). The editor runs the behaviour it's given and nothing more.
4. The check (`--snap-report ... --sim <steps> --nm balance [--push <N s>]`) stands each body under `bodyBalance`,
   pokes it halfway and compares it with the same build in LEAD (NM's own physics, as `DMS.FitLead`); `homer` adds
   Homer's own body, golden's.

## Consequences

- A rig is checked under NM without the game; a fitted body that falls in FOLLOW but stands in LEAD is a difference
  between the fork's physics and NM's on that body, which is the fork's to fix (0008, 0017), not the rig's or NM's.
- First results (600 physics steps, a 40 N s push halfway): Homer's body and `humans/group01/male_07` (snapped)
  stand and take the push (male_07 steps 0.43 m); `kleiner`, `alyx` and `combine_soldier` (snapped) go down after
  1.2–2.1 s while all of them stand in LEAD; every plain fit goes down. The module runs the same step, so these
  bodies are expected to behave the same in the game.
- A change to the FOLLOW step is made once, in `dms_vphysics::follow`, for both.
