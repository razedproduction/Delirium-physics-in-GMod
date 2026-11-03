# 0033. The map's water as the Playroom's pool

Status: accepted (2026-11-03, ROADMAP M5). Refines 0016 (the room layer), 0018 (NM's world from the map), 0020.

## Context

The Playroom's room has a pool (`world::POOL`: x −3…3, y 5…25 m, 1.5 m deep, the water at `world::WATER` = −0.1 m).
Its doll feels the water only through NM: the room layer's `self_preserve` (the additional reactions) sends
`buoyancy start=true surfacePoint=0,0,WATER` once the body's centre of mass is over the pool and under 0.3 m (out at
0.8 m: `buoyancy start=false`), and its `send` sends `buoyancy` again after every `stopAllBehaviours` while he's wet.
The Playroom's GPU water (`real_water`) is its own rendering and fluid, not carried over. Our port of `self_preserve`
had left the pool out. On the bench (gm_construct's pool, 2 m deep) Source's water acted on NM's bodies too: the fork's
`CBuoyancyController` floated Homer standing on the bottom up to 0.03 m under the surface with no message at all, and
a fitted male_07 the same; with `buoyancy` sent, the two pushed together.

## Decision

1. **The water is the map's:** the module reads the `.bsp`'s brushes with `CONTENTS_WATER` (their planes, metres) with
   the rest of NM's world (`world.rs`, `DMS.MapWater` to check it); on gm_construct its answers match
   `util.PointContents` on all 3876 grid points tried, the surface to the unit.
2. **The room layer's pool line, at its place in `self_preserve`** (after the in-the-air reactions, before the grab's
   release; so, as in the Playroom, only with the additional reactions): the Playroom's `over_pool(me) && me.z <
   WATER + 0.4` (out at + 0.9) is "a water brush's column through the centre of mass, its bottom under him and its
   top less than 0.4 m (0.9) under him"; `surfacePoint` is `0,0,<that top>` in NM's frame (the Playroom's
   `v3(Vec3::Z * WATER)`; a top at 0 would read as "his chest" to NM, so then the point over him). The log lines are
   the Playroom's (`ours  into the pool at …`, `out of the pool`). `Follower::send` starts `buoyancy` again after
   `stopAllBehaviours` while he's wet.
3. **The fork: water acts on an NM body only through NM.** A body that got `ApplyBodyDrive` this step or the last isn't
   simulated by the fluid controllers (`IsNMDriveCurrent` in `CBuoyancyController::Prepare`, the interface
   unchanged, docs/vphysics.md); a body NM lets go of floats again.
4. **A golden scenario `pool`:** the Playroom's room with its pool (`--stage pool`: the Playroom gives its room for
   any name that isn't a stage), a blast 1.5 m behind him throws him into the pool, `!set extra_reactions 1`. The
   stages map gets the room as a site (`dms_game::stage` "pool": the room's boxes of `world::Room::blocks` with the
   default `portal` feature, the floor with the pool's hole, the water brush), the water 6.35 mm higher than the
   Playroom's, as Box3D's floor is over the BSP's (0018), so it is −0.100 m in NM's frame on both.

## Consequences

- With the additional reactions off (the default, as the Playroom's `--safe`), Homer in the water sinks and stands or
  lies on the bottom, as the Playroom's doll in its pool; on, he floats on NM's `buoyancy`: COM 0.19 m under the
  surface on gm_construct, Homer and male_07 alike.
- `pool` on the bench (2 runs): into the pool at 3.43–3.47 s (Playroom 3.50), `buoyancy` with the same
  `surfacePoint=-0.000,-0.000,-0.100`, floating at −0.281…−0.286 m (Playroom −0.292); the flight is `blast`'s
  (Box3D's ~0.15 m lower), up to 0.29–0.35 m off before the water. The Playroom's explosion task also starts its
  `catchFall` as he hits the water (`vz < −8` m/s with ground in 2 m); the bench's entry is slower after the lower
  flight (9.0 against 10.0 m/s), so it doesn't (`marks` isn't held).
- The stages map's sites moved along X (one more), the sky's floor went down to −3 m and the floor brushes down to it.
- Detaching in water, the model's own physics floats again; a parked agent (LOD) floats while parked.
