# DMS in Gmod — Roadmap

Delirium (an active ragdoll, NM-style) natively in Garry's Mod in **FOLLOW** mode: the body is an ordinary
`prop_ragdoll` on the VPhysics-Box3D physics, every physics step Delirium reads the parts' state and returns forces and
torques, and the joints' armature is solved inside Box3D's solver.

## Inputs

| Component | Where | What it gives | Rule |
|---|---|---|---|
| Delirium SDK **v0.6.0** (`93eed4d`) | `../delirium-physics (CLAUDE DONT CHANGE ANYTHING HERE)/crates/delirium-sdk` | the C API `delirium_physics.h` (`nm_*`), the Rust crate `delirium_physics` (`rlib` + `cdylib`) | **read only**, a path dependency |
| VPhysics-Box3D (the fork) | `vphysics/` (git subtree, ADR 0015) | `vphysics.dll` + `IPhysicsNM` (`VBOX3D_NM_004`, `vphysics/include/vphysics_nm.h`) | changed only to match the Playroom (ADR 0008), upstream through `git subtree pull` ([docs/vphysics.md](docs/vphysics.md)) |
| Delirium's documentation (v0.6.0) | `../delirium-physics (…)/docs/how-it-works.md`, `docs/modding.md`, `docs/start-here.md`, `CHANGELOG.md` | how the engine works, the step, LEAD/FOLLOW, messages and their params, the Playroom's scripts and `settings.txt` | a reference; where it differs from the code, the Playroom's code is right |
| The previous attempt | `../delirium-physics (gmod addon & api)` | notes and pitfalls (README) | only as a reference, its code isn't carried over |

Key facts from the API the plan is built on:

- The FOLLOW loop: `GetBodyState` → `nm_agent_set_part_states` → `nm_agent_step(dt)` → `nm_agent_get_part_drives` →
  `ApplyBodyDrive` + `nm_agent_get_joint(...armature)` → `SetJointArmature`. All of it inside `SetStepCallback`, on the
  main thread, inside `IPhysicsEnvironment::Simulate`.
- NM's body is **21 parts** (capsules along the local Y), ball/hinge joints with `pivot`, `frame`, `limits`, `armature`.
  A ValveBiped ragdoll has ~15 physics objects. The fork is designed for **1 part = 1 physics object, 1 joint = 1
  constraint**.
- `SetJointAnchor` / `SetJointLimits` are called once at binding, in the rest pose.
- NM lives at 60 Hz, Source at 66 ticks (1/66 s, 4 Box3D sub-steps). SI units, Z up, 1 unit = 0.0254 m.
- Delirium needs the character's data (`fred.xrt` / `character.json`, `build.txt`, optionally `taskparams.txt`) from the
  user's copy of GTA IV: the addon doesn't have it and can't.
- In FOLLOW `nm_agent_step` still gets an `NmWorld`: NM needs the world's geometry (support, raycasts, the parts'
  contacts).
- The step: `dt` ∈ (0, 0.25] s, inside it `round(dt·60)` sub-steps (1…8); the native 1/60 step is exactly one sub-step.
  So NM always gets 1/60 (an accumulator, M4), not 1/66.
- FOLLOW's drives come without gravity ("don't add gravity compensation"): the host gives the body 9.8 m/s² itself, as
  the Playroom does. Our compensation (ADR 0007) removes only the difference Source − 9.8, not NM's gravity.
- The agent's feedback is cleared at the start of every NM step: read it after every `nm_agent_step`, not once a tick.
- A sleeping agent skips behaviours until woken (`nm_agent_is_awake`).
- The C API doesn't cover everything the Playroom does: the engine's tasks (`shottask`, `meleetask`, `explosion`,
  `onfire`, `playerctl`), `extras.outbox` (messages behaviours send each other, e.g. catchFall), reactions at the room's
  level: `delirium-room` calls them directly through the `delirium` crate. The module does the same (ADR 0016).

## Architecture (the target)

```
Lua (addon/)                         Binary module gmsv_dms (Rust)                         vphysics.dll (Box3D fork)
─────────────                        ──────────────────────────────                        ─────────────────────────
dms.Attach(rag) ──► DMS.* native ──► agents' registry ──► command queue ──┐
hooks (damage, remove)                 │                                   ▼
                                       │                  OnStep(env, dt) ◄── SetStepCallback ── Simulate()
                                       │                    GetBodyState ─────────────────────────► IPhysicsNM
                                       │                    nm_agent_step (delirium-sdk, rlib)
                                       │                    ApplyBodyDrive / SetJointArmature ───► IPhysicsNM
                                       └── events queue ──► Think hook ──► hook.Run("DMS_*")
```

Decisions taken at once (ADRs in `docs/adr/`):

1. **The module in Rust**, the SDK linked as an `rlib` by path: one DLL, without a separate `delirium_physics.dll`. The
   SDK's folder isn't touched: Cargo builds it in this repository's `target/`.
2. **A server module only** (`gmsv_dms_win64.dll`; GMod's 32-bit branch isn't supported). The client needs only the
   ragdoll's ordinary network replication; client Lua is only debugging and UI.
3. **Lua is never called from the step callback.** Lua → module: a command queue, applied at the start of `OnStep`.
   Module → Lua: an event queue, taken in `Think`.
4. **Homer's body only.** We work with NM's body as it is (Homer): a model with 21 bones = 21 physics objects, the
   bone's frame = the NM capsule's frame (zero offsets). GMod's original models (ValveBiped) aren't done for now: see
   "Deferred".
5. **No `IPhysicsNM` → the addon turns off** with a clear message (no fallback to the vanilla physics in FOLLOW).
6. Licence **GPL-3.0-only** (as the SDK's). GTA IV's game data isn't distributed.
7. **Gravity:** `sv_gravity` isn't touched, a force on every part removes the difference from NM's 9.8 m/s² (ADR 0007).
8. **The Playroom is golden** (ADR 0008): Homer's behaviour has to match the Playroom's one to one. We don't interfere
   with Delirium's work (the same inputs, drives as they are, no tuning of our own; the exception is gravity);
   divergences are fixed in the physics (the fork) and checked by an automatic comparison with the Playroom
   (`tools/golden`).

The repository's layout:

```
DMS in Gmod/
  module/            gmsv_dms (Rust): ffi/vphysics_nm.rs, ffi/lua.rs, agent.rs, world.rs, bind.rs, step.rs
  tools/             dms-homer (Rust CLI): Homer imported from the copy of GTA IV → homer.mdl/.phy for NM's body
                     golden (Rust CLI): running the Playroom headless, comparing the body and trajectories with GMod
  golden/            scenarios/*.txt (scenarios); out/: the Playroom's references, local, not in git
  addon/lua/         autorun/server/dms_init.lua, dms/*.lua, autorun/client/dms_debug.lua
  maps/ (opt.)       a test map: a floor, stairs, a ledge, a wall
  vphysics/          the VPhysics-Box3D fork (git subtree, ADR 0015): vphysics.dll + IPhysicsNM
  docs/              adr/, data-setup.md, vphysics.md, api.md
  scripts/           build.ps1, build-vphysics.ps1, install.ps1 (copying the dll, addon and fork into GMod)
```

---

## Milestones

Estimates are rough, in "one person's working days". Each milestone closes only by its DoD checklist.

### M0. Foundation — 1–2 days

- [x] `git init`, `.gitignore`, LICENSE (GPL-3.0), this roadmap.
- [x] ADRs 1–6 (above) written down.
- [x] In the VPhysics-Box3D fork, commit the NM extension (now `physics_nm.*`, `include/`, `cmake/patches/` aren't in
      git) on a separate branch, build `windows-x64` (only GMod's x86-64 branch is supported). Committed in
      razedproduction/VPhysics-Box3D (PR #1, #2), then the fork moved into this repository's `vphysics/`
      (`git subtree`, ADR 0015), built with `scripts\build-vphysics.ps1`, installed with `install.ps1 -Fork`.
- [x] Copy `vphysics_nm.h` into `module/` as the FFI's reference (an unchanging file, version `VBOX3D_NM_002`; now
      `module/src/ffi/vphysics_nm.h`, `VBOX3D_NM_004`).
- [x] `scripts/build.ps1` (cargo for `x86_64-pc-windows-msvc`, renamed to `gmsv_dms_win64.dll`),
      `scripts/install.ps1` (the dll → `garrysmod/lua/bin`, the addon → `garrysmod/addons/dms`, the fork's
      `vphysics.dll` → `bin/win64` with the original backed up).
- [x] A test bench: GMod's x86-64 branch + the fork + `sv_cheats 1`, `developer 1` (then M2.5's bench).

**DoD:** the empty module builds for win64, `require("dms")` in GMod prints the module's version and `nm_version()`.

### M1. The bridge to VPhysics-Box3D — 3–4 days

- [x] `ffi/vphysics_nm.rs`: a `#[repr(C)]` vtable of `IPhysicsNM` strictly in declaration order (10 methods). x64 is an
      ordinary `extern "C"` with `this` as the first argument (x86 isn't supported, ADR 0002). `PhysicsNMBodyState_t`
      as `#[repr(C)]`.
- [x] Getting the interface: `GetModuleHandleA("vphysics.dll")` + `CreateInterface("VBOX3D_NM_002")` (Linux:
      `dlopen(..., RTLD_NOLOAD)`, `vphysics_srv.so` on dedicated, in M7). If `NULL`, try `_001` and warn that there'll
      be no joint limits; if that's missing too, the addon is off (`DMS.Available()` = false).
- [x] `lua.rs`: Lua userdata `PhysObj` → `IPhysicsObject*` (GMod's structure `UserData { void* data; uint8 type; }`,
      checking the `PhysObj` type). No third-party hacks: only through the public layout.
- [x] Debug natives: `DMS.Available()`, `DMS.InterfaceVersion()`, `DMS.BodyState(physobj)`,
      `DMS.Drive(physobj, force, torque[, steps])`, `DMS.FindJoint(a, b)`, `DMS.SetArmature(a, b, value)`,
      `DMS.GetArmature(a, b)`, `DMS.SubstepCount()`, `DMS.Hover(physobj, mass, g | false)`; console commands
      `dms_info`, `dms_bodystate`, `dms_hover`, `dms_armature` (`addon/lua/dms/debug_commands.lua`).
- [x] The step callback on the server's environment (`step.rs`): set at the first drive, removed when the module
      unloads (`gmod13_close`) and at `ShutDown` (`DMS.Shutdown()`).

**DoD:** (1) `BodyState` matches `PhysObj:GetPos()/GetVelocity()` allowing for units and the COM; (2) a prop given an
upward `m·g` force by `Drive` hangs in the air; (3) armature 1e6 on a ragdoll's joint makes it rigid, 0 puts it back;
(4) `changelevel` and `lua_openscript` with a repeated `require` don't crash the game.

### M2. Delirium inside the module — 2–3 days (in parallel with M1)

- [x] A path dependency on `delirium-sdk`, SDK calls as Rust functions (wrappers checking the `NM_E_*` codes and
      `nm_last_error`): `module/src/nm.rs`.
- [x] ~~Loading data from `garrysmod/data/dms/`~~ → by ADR 0010 the data is read straight from the copy of the game
      (the convar `dms_game_dir`) by the shared crate `tools/dms-game`, with the Playroom's code: PC (`Fred.art`
      little-endian through `delirium_data::LE`, as in the Playroom) and Xbox 360.
- [x] ~~A tool "make `character.json`"~~ isn't needed (ADR 0010); `docs/data-setup.md`.
- [x] An offline self-test inside the module: an agent in LEAD on a flat floor, 2 s of `bodyBalance`: it stands (the
      command `dms_selftest`, the native `DMS.SelfTest`).
- [x] Reference natives: part names, the list of messages and params, `nm_build_part_info`, `nm_agent_get_joint`
      (`DMS.PartNames/Messages/MessageParams/BuildParts/BuildJoints/BuildDump`, the commands `dms_nm_*`).
- [x] NM's inputs are the Playroom's (ADR 0008): Homer's build = `NM_DUMP` byte for byte, the links from the idle clip
      bit for bit, the agent in LEAD bit for bit with the Playroom (`golden dump|links|lead`).
- [x] The golden bench (`tools/golden`, ADR 0008, 0011): building the Playroom from the SDK's folder without writing to
      it (`--locked`, `--target-dir` in this repository), the `NM_DUMP` of Homer's build, runs of the scenarios in
      `golden/scenarios/` (standing under `bodyBalance` 60 s, a shot in the chest, a push, `catchFall` from a height,
      `relax`; headless bit for bit, the others `--script --rec` in the window, since the headless Playroom doesn't run
      scenarios) → `golden/out/` (not in git). The commands' and recording's format: `golden/README.md`.

**DoD:** `dms_selftest` passes in the game; without data, a clear error with the path to put the files in;
`tools/golden` records the Playroom's references for all scenarios in one command.

### M2.5. A test Playroom on Source: the bench in the game + `dms-ctl` — 4–6 days

The goal: a bench where Claude itself, without the user, starts Homer in GMod, drives him and sees his state in real
time: the same role as the Playroom's `--headless --script --rec`, but on Source + Box3D. The user checks only the
visual side in the game.

Done ([docs/bench.md](docs/bench.md), ADR 0012). The main difference from the plan: **the bench is the game itself
without a window**, not a dedicated server: GMod's x64 server build is built against newer vphysics interfaces
(`IAppSystem` with `GetDependencies…`, longer `IPhysicsCollision`/`IPhysicsSurfaceProps`), and the fork crashes it.

> **Decision (2026-10-21): the dedicated server is dropped, all tests are in the game.** The bench is the user's game
> without a window (a listen server), `dms-ctl` connects to it. The fork isn't ported to the server's ABI for the
> bench; the dedicated server comes back only with Linux/dedicated in M7, if it's needed there.

- [x] ~~**A GMod dedicated server**~~ → the bench on the game (ADR 0012). A dedicated server is installed
      (`E:\Games\GModServer`, `server.ps1 update`, `start -Dedicated`, `install.ps1 -Server`) and works with its own
      `vphysics.dll` (the channel, the test agent, recordings), but **without the fork**. The dedicated server **isn't
      used** for tests (the decision above); the VPhysics-Box3D fork isn't ported to the server's ABI (`IAppSystem` +
      the vphysics interfaces): a question only for Linux/dedicated (M7).
- [x] **Starting without a window:** `scripts\server.ps1 start|stop|restart|status`: `gmod_win64.exe -textmode`, the
      window hidden, a listen server `maxplayers 2` + `-usercon` (RCON), `sv_hibernate_think 1`, `sv_cheats 1`,
      everything on `127.0.0.1`, RCON and the channel's token new on every start, the `-condebug` log →
      `garrysmod\console.log`. While the bench runs, the user doesn't play.
- [x] **The test scene:** `gm_flatgrass`: a flat floor, the start of the Playroom's room = the floor under
      `info_player_start` (`dms_bench_scene`, `DMS.BenchScene()`). Deferred to M5: the Playroom's stages (stairs, a
      wall, a ledge, a bridge), together with NM's world from the map (the game has `vbsp/vvis/vrad`); spawning Homer in
      the Playroom's pose: M3/M4.
- [x] **The control and telemetry channel in the module** (`dev.rs`, `dev.lua`): only `127.0.0.1`, only with
      `dms_dev 1` (turned on only by the one-off `data/dms/bench.txt` from `server.ps1`), a token; the network threads
      only queue and hand out finished frames (ADR 0003), the work is done in `Think`. Telemetry: a JSON line a server
      frame: time/tick, `step.rs`'s drives, NM's and Source's gravities (the compensation `null` until M4), room for
      FOLLOW agents (`agents`, M4); the test agent's step, the `balancing/fallen` flags, COM, parts, contacts, drives,
      feedback.
- [x] **The CLI `tools/dms-ctl`**: `status`, `exec` (RCON with output; `--module`), `lua` (`-f`), `nm`, `bench`,
      `snapshot`, `watch --for … [--fields] [--every]`, `run <scenario>` → the Playroom's `.rec`; `lua` goes to RCON's
      `lua_run`, `exec` to the module, if the other way doesn't answer.
- [x] **Comparison in one step:** `golden behaviour --source [names]`: scenarios on the bench against `golden/out/`.
      For now the scenarios are run by an SDK agent in LEAD (as `dms_selftest`), i.e. the pipeline is checked, not
      Box3D: `stand` (60 s), `stand-1s`, `relax`, `relax-1s` match the Playroom to 3 digits of the recording on every
      step. Not matching, and known: `shot` from step 62 (the Playroom's shot is ShotTask, M6); `push`: a poke from the
      Playroom's camera, which moves in the window (the bench takes its starting point); **`catchfall`: the Playroom
      forwards the messages of its `extras.outbox` to the agent (catchFall itself starts `pedalLegs`, `bodyRollUp`), and
      the SDK's C API doesn't expose this outbox: solve in M4** (otherwise a FOLLOW agent won't match the Playroom on
      catchFall). A meaningful comparison of Homer's behaviour comes after M3 (the model) and M4 (FOLLOW).
- [x] Time: runs in real time (the game's ticks, NM steps of 1/60 by an accumulator, like the Playroom's window);
      `host_timescale` only for debugging.

**DoD:** one command (`scripts\server.ps1 start`) brings up the game without a window (a listen server); `dms-ctl exec
status`, `dms-ctl lua "return DMS.Version"` and `dms-ctl watch --for 2s` work from PowerShell; a scenario from
`golden/scenarios/` runs in the game and gives a recording in the Playroom's format; everything is available from
Claude's session without the user.

### M3. Homer's model: imported from scratch for NM — 6–9 days

The goal: a Homer ragdoll whose physics matches NM's body, so that any FOLLOW problems are integration problems, not
fitting ones. From here every milestone works only with him (from here "the reference" = Homer).

The model is **imported anew**, nothing is taken from the previous attempt (its `delirium-ragdoll`, `homer.mdl` and
`.nmfit.txt` aren't used). The principle is maximum compatibility with NM: everything about the body is taken from the
same data and with the same code Delirium builds Homer with itself (`delirium-data` + `delirium::ragdoll::Rig`, like
`getup::homer` in the Playroom), without fitting (`nm_build_fit`) or hand edits.

Done ([docs/homer.md](docs/homer.md), ADR 0013). Differences from the plan: the physics object's frame is the NM
part's frame, but the capsule's axis in it is turned (NM turns the shape with its link), feet and hands are NM's
boxes; the look is the Playroom's (NM's capsules and boxes, without GTA IV's mesh and textures).

- [x] `tools/dms-homer` (a Rust CLI): Homer from the copy of GTA IV through `tools/dms-game` (the same build as the
      Playroom's, without `nm_build_fit`); NM's body is the shared `dms_game::body` (`Ragdoll` + `Agent::joints`), the
      module and `golden` take it too.
- [x] Physics: 21 physics objects in NM's part order, the bone turned as the NM part's frame, its origin in the joint's
      pivot (Source draws a child's bone at a constant offset from the parent), the centre of mass in the part's
      centre, the hulls of NM's capsules and boxes (shrunk by 6.35 mm: the fork inflates them; a step ≥ 1.4 cm: the
      fork welds closer than 1 cm), NM's masses exactly, the centre of mass in the part's centre and NM's inertia
      diagonal right in the `.phy`, ball/hinge by NM, only NM's self-collision pairs, damping 0.
- [x] The look, as the Playroom draws Homer (`part_mesh`, `part_color`): each part's NM capsule or box in its capsule
      frame, left parts red, right blue, the rest grey, translucent; the other `PED_COLORS` schemes (all blue, all red,
      solid blue, mixed opaque) are the model's skins (`$texturegroup`). GTA IV's mesh and textures aren't exported.
      The Playroom's face and skeleton overlay: M6.
- [x] Building: SMD + `.qc` → GMod's `studiomdl` (`bin\win64`, with the fork) → `garrysmod/models/dms/homer.mdl` +
      `.phy` (the text rewritten: solids by index, as the game takes them), materials in
      `materials/models/dms/homer/`.
- [x] The model and working files only on the user's machine (`target/dms-homer/`, GMod's folders), not in git.
- [x] A check in the game: 21 physics objects, masses, all 20 joints through `FindConstraint` (`golden body`).
- [x] Bind (`bind.rs`, `DMS.BindHomer`): `SetJointAnchor` + `SetJointLimits` (NM's frames on the parent and the
      child) on every joint; parts → physics objects by the bones' names (`DMS_HomerParts`).
- [x] Spawn: `dms_homer`: Homer in the Playroom's standing pose in front of the player, facing them, bound to NM at once.
- [x] **`golden body`**: build = `NM_DUMP` (byte for byte; the fields and the SDK's C API 0…5e-7); `.phy`:
      masses/centre of mass/inertia diagonal exactly, hulls −3.1…+0.5 mm, volume ≤ 3.7 %, NM's constraints and pairs; in
      the game, positions ≤ 0.6 mm, rotations ≤ 0.1°, masses and inertia diagonal exactly, a fall without NM without
      blow-ups, pivots ≤ 1.5 mm.
- [x] **The fork's divergences fixed** (ADR 0014, `VBOX3D_NM_003`): the full inertia tensor (`SetBodyInertia`; it was
      only the diagonal, the arms up to 46 % off) and a ball with NM's semantics (`SetJointBallLimits`: twist about Z, an
      ellipse of leans in the parent's frame, the Box3D patch `box3d-elliptical-cone.patch`; it was twist/cone/plane
      with a heuristic axis and X/Y mixed up, the hip went ~15° past). `golden body` passes: the tensor 7e-8, joints in a
      fall ≤ 0.5° past NM's limits, pivots ≤ 0.6 mm.
- [x] The user's visual check (docs/homer.md: the look, `vcollide_wireframe 1`, a fall): passed (2026-11-03).

**DoD:** `dms-homer` makes `homer.mdl` from the copy of GTA IV in one command; `tools/golden body` passes: the build
matches the Playroom's (1e-5), a spawned Homer has all 21 parts matching the Playroom (position ≤ 1 mm, rotation ≤ 0.5°,
mass exactly, inertia ≤ 1 %); thrown without NM he behaves as an ordinary ragdoll, the joints bend within NM's limits,
nothing blows up; the limits are visible through `IPhysicsConstraint::GetConstraintTransform`.

### SDK update: Delirium v0.1.9+7 → v0.2.2 (2026-10-26)

- [x] The SDK at `v0.2.2` (`91ee246`); the C API (`delirium-sdk`) didn't change, no API fixes were needed. `bevy_math` /
      `bevy_transform` 0.18 → 0.20 (`glam` 0.30 → 0.33, versions as in the SDK's `Cargo.lock`). Built without AVX2: the
      module has the same runtime branches with a CPU check (`ymm`/BMI in memchr and std) as before the update, no FMA
      instructions.
- [x] Homer's body didn't change: tapered capsules (`Shape::Tapered`, `8c47480`) exist only in GJK, Homer's parts
      don't get them; the build differs from the former in the last bit of a float (≤ 1e-7, the new `glam`). The
      `dms-homer` model rebuilt (the same shapes and `.phy`, the hulls' vertices moved by thousandths of a mm).
- [x] `tools/dms-game` compared with the Playroom: `getup::{key, plat, homer, Clips::stand_up}` and the `load` order are
      the same (`Clips::load` only gained the swimming clips). The Playroom room's floor changed: `world::floor_tris(pool,
      50)`: 1 m tiles, like the former `add_floor`, but with the pool's pit (x −3…3, y 5…25 m); `dms_game::floor`
      repeats it (the world of `dms_selftest` and the bench, `golden lead`).
- [x] The Playroom's references recorded anew (`golden/out/`): the behaviour changed (the stance diverges from the
      former from ~0.87 s, `catchFall` now sends `stopAllBehaviours` at 0.67 s instead of `pedalLegs`/`bodyRollUp`
      etc.: "tester fixes"). The Playroom's window since v0.2.1 reads `%APPDATA%\delirium\settings.txt` (the behaviour
      switches are there too) and on ordinary graphics runs at < 10 FPS (losing steps), so `golden run` starts the
      window with `--safe`: the Playroom's defaults and minimal graphics, 60 steps/s.
- [x] `golden`: dump byte for byte, links and lead bit for bit, `body` passes whole; `behaviour --source`: `stand`
      (60 s), `stand-1s`, `relax`, `relax-1s` match to 3 digits on every step. As before, `shot` (M6), `push` (the
      Playroom's camera; the COM now within 1.4 mm) and `catchfall` (the outbox, M4) don't match.

### SDK update: Delirium v0.2.2 → v0.6.0 (2026-11-04)

- [x] Upstream rewrote its history (v0.2.5 is a root commit now): the SDK's checkout moved to the tag `v0.6.0`
      (detached), `v0.2.2` kept by its tag. `async-executor` pinned to the SDK's `Cargo.lock` (1.14.0).
- [x] **The engine's numbers come from the player's game** (0.2.5/0.2.6, `delirium_data::kv`): all 0 until
      `kv::unlock` with the game's program. `dms_game::Game::open` unlocks first (the Playroom's `numbers`: the PC
      `GTAIV.exe` or the disc's `default.xex`, `xex::program`). Only one PC exe is known to the SDK (17 425 752 bytes);
      the Steam Complete Edition's **1.2.0.59 (28 225 536 bytes) is refused** by the SDK and the Playroom alike. The
      references and the bench now run on the Xbox 360 disc, unpacked by the Playroom (`--game <.iso>` → `game\`).
- [x] API changes: Homer's links moved from the global `nmdbm::DERIVED_LINKS` into `Build::links` (`Rig::linked`;
      `Game::homer` as the Playroom's `getup::homer`, `dms_game::links`); constants turned functions (`GRAVITY()`,
      `ped::INSTANT()`); `shottask::bullet`'s `soft` (false, as the Playroom); `rage_link(&build.links, k)`;
      `bump::Kind::Vehicle { train }`, `MeleeTask::Balance { clock }`.
- [x] 0.6.0: on PC a file not loose is read from `pc\data\game.rpf` (the PC's `taskparams.txt` is only there):
      `Game::archive` as the Playroom's `archive_read`.
- [x] **A build read back from a dump has no links** (`Build::undump`, `nm_build_from_text`, the Playroom's `--export`
      `build.txt`: the dump doesn't carry them, `nmdbm::asset_links` is empty without `NM_LINKS`), and its layout
      differs (the hands 0.3 m off): the C API's Homer isn't the Playroom's. `dms_selftest` and `golden lead` moved to
      the crate (`Agent::new` on Homer's build, `msg::send`): bit for bit with the Playroom, 60 steps; `DMS.BuildParts`
      from `dms_game::body`; `golden body` hands DMS's links to the SDK through `NM_LINKS` (then exact). Left on the C
      API: the bench's LEAD agent (`dms-ctl nm`, `behaviour --lead`) and `DMS.BuildJoints` (a printout).
- [x] `golden`: `dump` byte for byte, `links` and `lead` bit for bit, `body` OK (all 3 steps), `dms_selftest` passes.
      The references recorded anew (19 scenarios; `blast` twice: the first recording lost 12 steps of the window).
- [ ] `behaviour --source`: 12 of 19 within (`catchfall`, `death`, `getup`, `nearmiss`, `push`, `relax`, `shot`,
      `shotgun`, `stairs`, `stand-1s`, `stand`, `tasks`). Out: `kerb`, `lowwall`, `relax-1s` (known, M5), `drunk`
      (out on the base too); **new**: `blast` (COM max 0.38 m, tolerance 0.30), `pool` (the end 0.45 m deep, the
      Playroom 0.28), `rooftop` (he ends lying, 0.12 m, the Playroom's stands, 0.94), the same in two runs each.
- [x] ~~Fitted bodies (male_07)~~ → moved to the ragdolls repository with M9–M12.5 (ADR 0036). Measured before the
      split: 0.6.0's fitted joints' pivots (`Ragdoll::jconst`) and 0.5.0's `Build::fit`; `golden fit`: `stand` within;
      `push` COM mean 0.056 (tolerance 0.05), `shot` 0.255 (0.25), `catchfall` end 0.220 m (LEAD 0.165, 0.03);
      `golden body --fit`: the build exact against `nm_build_fit`, in the game the head's inertia 32 % off NM's.

### M4. The FOLLOW loop on the reference — 5–8 days (the main milestone)

Done (ADR 0017). The main difference from the plan: the FOLLOW agent works not through the C API but on the `delirium`
crate, as the Playroom does (the outbox, `in_air`, NM's impulses past `doll.out`), and the fork's physics needed fixes:
without them Homer fell in ~2 s.

- [x] `step.rs` + `follow.rs`: on every NM step: commands → `GetBodyState` ×21 → `set_body` → the Playroom's room layer
      → `Agent::step_follow` → drives (`doll.out`) → `ApplyBodyDrive` ×21 on every Source step → armature from
      `doll.armature` → `SetJointArmature` ×20. Inside the step callback only `IPhysicsNM` (ADR 0003). Homer's body's
      frame = the NM part's frame (M3), the centre of mass in the part's centre: no conversions needed.
- [x] Time: NM always 1/60 (an accumulator over the 0.015 s physics steps, drives held until the next NM step).
      Checked: `-tickrate 60` doesn't change the physics step (the game sets 0.015), a 1/60 physics step
      (`DMS.EnvTimestep`) and an NM step = the Source step are no better. A scenario's recording in the Playroom's tick
      order.
- [x] Gravity (ADR 0007): `sv_gravity` isn't touched, `F_k = m_k · (g_nm − g_src)`, `g_src` from `physenv.GetGravity()`;
      the modes `dms_gravity_mode nm | hybrid | off` measured against the Playroom, **the default `nm`** (the user's
      decision): only with it are all the scenarios within tolerance.
- [x] Messages between behaviours: `extras.outbox` forwarded after every NM step as in the Playroom (`extras_frame`).
      In v0.2.2 the `stopAllBehaviours` in `catchfall` is sent by the room layer's `in_air`: carried over with the same
      code (ADR 0016, 0017).
- [x] Feedback and flags after every NM step into the event queue (`DMS.FollowEvents`, the hooks `DMS_Feedback`,
      `DMS_Fallen`, `DMS_Detached`).
- [x] Sleep: the NM doll's own sleep is kept until Box3D moves the body; the bodies fall asleep with it (the `sleep`
      event), a blow wakes both.
- [x] Protection: NaN/Inf or a blow-up (> 80 m/s, > 300 rad/s) → the agent comes off, armature 0, an ordinary ragdoll.
- [x] Debugging: `dms_status`, `dms_follow_send`, the telemetry of `dms-ctl watch` (`agents`: step, flags, COM, parts,
      drives, armature, `pred_err`: Box3D's divergence from NM physics' prediction, `limit_use`, `state_v`, the
      compensation, NM's cost). The client overlay of capsules and forces **isn't done**: moved to M6 (with Homer's face
      and skeleton).
- [x] The golden comparison: the bench's scenarios are run by Homer in FOLLOW on Box3D (`--lead`: the former LEAD
      agent), the commands on the Playroom's steps from its recording's marks; `golden behaviour --source` checks the
      scenario's tolerances (`#! tol`).
- [x] The fork's fixes (ADR 0017, `VBOX3D_NM_003` without changing the interface): armature (`Cdot0` once a sub-step, 8
      Gauss–Seidel passes over the joints with armature), the step's starting velocity in `GetBodyState`, continuous
      gravity for NM bodies. `golden body` passes.
- [x] Known divergences (the plan is the next step in the physics, ADR 0017): `relax-1s` outside tolerance (the
      supporting foot sinks 2–4 cm into Box3D's soft contact under the impact load); Box3D's floor 6.35 mm above the
      BSP; the shot: M6.
      → M5 (ADR 0018): the floor is taken into account; the cause of `relax-1s` refined: the joints stretching, not
      the contact.
      → Left as known divergences (the user's decision, 2026-11-03): see M5's item.

**DoD:** standing under `bodyBalance` 60 s, a push, `catchFall`, `relax`, the first 60 steps of a shot within the
scenario's tolerances; `relax-1s` outside tolerance, the cause and the plan in ADR 0017; 4 Homers at once: 67 ticks/s
with a target of 66.7 (~1 ms of NM an agent).

### M5. The world for NM — 4–6 days (in parallel with the end of M4)

Done (ADR 0018). Differences from the plan: the world is built by the `delirium` crate (`Collision`, as for M4's FOLLOW
agent), not the C API; the floor is cut into 1 m squares, like the Playroom's floor; the fork needed fixes: NM's
contact materials (`VBOX3D_NM_004`) and the stiffness of NM's joints.

- [x] Statics: the map's `.bsp` (Lua reads the game's file, `module/src/world.rs`): the world model's faces without
      `SKY/SKY2D/WARP/TRIGGER/NODRAW/HINT/SKIP`, displacements, wound anticlockwise about the outward normal, without
      T-junctions; the floor in 1 m squares on NM's frame's grid (like `world::floor_tris`) within 50 m. Materials:
      `$surfaceprop` → a materials.dat row (`world::SURFACES`, unknown → CONCRETE). The cache: the map for the session,
      the world by the agent's frame's start. The check `DMS_MapCheck`: NM's heights and the game's traces match to
      0.1 mm.
- [x] Dynamics: props within 3 m of the agent → an `Obstacle` every tick (OBB, velocity, rotation, centre of mass;
      `DMS.FollowObstacles`, the telemetry `obstacles`); vehicles the same way (a box by the OBB).
- [x] Fallback: the map can't be read: the Playroom's floor at the height of a trace downwards (`DMS_FloorBelow`).
- [x] Box3D's floor 6.35 mm above the BSP: NM's world's faces moved out by the fork's shell, the floor under Homer is
      the trace's plane (minus `DIST_EPSILON`) plus the shell; each agent's NM frame on the floor under him.
- [x] Contact friction and restitution as in NM's physics: the fork's `VBOX3D_NM_004` (`SetBodyMaterial`,
      `SetSurfaceMaterial`): `μ_part × μ_materials.dat row`, `0.4 × e`; joints between NM bodies skip Box3D's "stiffness
      budget".
- [x] The Playroom's room layer on the stages with the same code (ADR 0016): `stairs_frame` (`rollDownStairs`),
      `ledge` (rooftop).
- [x] Water: Homer in the map's water as in the Playroom's pool (v0.2.1): the same `buoyancy` message with the same
      params and the water's surface from the map; the Playroom's GPU water simulation isn't carried over.
      Done (ADR 0034): the `.bsp`'s water brushes; the room layer's pool line where the Playroom has it
      (`self_preserve`, so with the additional reactions, and again after `stopAllBehaviours`); the fork lets water
      act on a body NM drives only through NM (Source's buoyancy floated Homer and male_07 on its own). On
      gm_construct: without the additional reactions he sinks, with them he floats 0.19 m under the surface (Homer
      and male_07). Golden `pool` (the Playroom's room with its pool on `dms_stages`, a blast throws him in): into the
      pool 3.43–3.47 s against 3.50, the same `buoyancy`, floating at −0.28 m against −0.29.
- [x] A test map: `tools/dms-map` → `dms_stages.bsp` (the Playroom's stages `bigstairs`, `stairsdown`, `rooftop`,
      `highledge`, `lowwall`, `kerb`, `rail`, `open` and a 20° slope, a site a stage); ice isn't done (with the water).
- [x] Golden scenarios on the stages (`#! window stage=…` → the Playroom's `--stage`, the bench
      `server.ps1 start -Map dms_stages`): `kerb`, `rooftop`, `lowwall`, `stairs`; the `said=` check (the room layer's
      reactions). Within tolerance: `kerb`, all the earlier scenarios except `relax-1s`, for `rooftop` the reactions
      and the fall (except the pose after landing).
- [x] Known divergences (ADR 0018): `relax-1s` (0.77 s against 1.00) isn't the soft contact but Box3D's joints
      stretching under the weight (the ankle 6–10 mm; the fork's extra passes don't include the feet's contacts);
      `lowwall`, `stairs` and the `rooftop` landing are a chaotic amplification of the same stance divergence (2 cm
      before the push). The plan: NM bodies' contacts in the fork's extra Gauss–Seidel passes.
      → ADR 0019: NM bodies' contacts in the extra passes done (`box3d-armature-contacts.patch`): `stairs` within
      tolerance, `lowwall` stands (the COM gap 0.12 m instead of 2.2), the ankle's stretch peaks 20 → 10 mm. What's
      left is the springiness of Box3D's soft joint `F / (m_eff · ω²)` (~5 mm standing); because of it `relax-1s` (now
      after 1.00 s), `kerb`'s pose, `lowwall` (close) and the `rooftop` landing are outside tolerance for now; the plan
      is NM joints' stiffness above the ¼ ceiling.
      → Measured again on the bench (2026-11-03) and **left as they are (the user's decision)**: NM's own physics is
      articulated (reduced coordinates, `delirium` `artic.rs`), its pivots never part; Box3D's soft joints stretch,
      and the chaotic scenarios amplify those millimetres one way or the other, so no small change of the fork closes
      all three. Now (4 sub-steps): the ankle's gap standing 4.8 mm mean / 7.5 max, `stand-1s` COM max 0.040 m,
      `relax-1s` never below 0.85 m in its 60 steps (Playroom 1.00 s), `kerb` below 0.5 m at 3.4–4.8 s (Playroom 2.98,
      tolerance 0.25) ending 0.39–0.49 m (0.208), `lowwall` 0.101 / 0.054 m, `rooftop` and `stairs` within; `kerb` is
      the same on main's fork and the old map (3.27–4.78 s). Tried, not taken: 8 sub-steps (the whole world's): ankle
      1.2 mm, `stand-1s` 0.022 m, `kerb`'s end within (0.217 m) but below 0.5 m at 3.97 s, `lowwall` 0.100 / 0.057,
      `relax-1s` early (0.72 s ×3), male_07's `push` 0.057 (M10), the physics step with 8 Homers 11.3 → 16.2 ms;
      NM joints alone at ½ the sub-step rate (ADR 0019): worse stages. Not tried, larger: projecting NM joints' pivots
      after the step (1–2 days, fights the feet's contacts); an articulated solver for NM bodies in the fork (weeks).

**DoD:** golden scenarios on the Playroom's stages (`--stage`: stairs, a wall, a ledge) match the Playroom on the same
geometry built in GMod; NM's contacts (`nm_agent_get_part_contact`) match the touches in Box3D.

### M6. The Lua API, behaviours, reactions — 4–5 days

Done (ADR 0020, [docs/api.md](docs/api.md)). Differences from the plan: the reaction layer is a port of the Playroom's
`Sim` methods (`module/src/follow/room.rs`), not calls of ready functions (the crate has none); getting up "by
animation": the bodies are held by the clip (frozen and posed every tick), since a ragdoll can't be hidden the way the
Playroom hides its doll; the Playroom for the references was rebuilt (the former `target-playroom` was older than
v0.2.2's sources and didn't know `!miss`, `!set`), the references of all scenarios recorded anew (`shot` and
`catchfall` byte for byte as before).

- [x] The public API (`dms.*`): `Attach(rag, opts)`, `Detach`, `ctrl:Send(msg, params)`, `ctrl:Shot`, `Balance`,
      `CatchFall`, `Flinch`, `Relax`, `Stop`, `GetState`, `GetParam`; `dms.Messages()` for introspection. Also
      `dms.Get/All/Reactions`, `ctrl:React/Button/Kill/SetHealth`; a Homer from the spawn menu is bound as he spawns.
- [x] Hooks: `DMS_Attached`, `DMS_Detached`, `DMS_Fallen`, `DMS_Feedback` (from the event queue); also `DMS_Died`,
      `DMS_Reaction` (the room layer's log).
- [x] **Reactions are a port of the Playroom's reaction layer, not our own** (ADR 0016, 0020): a shot (`fire` +
      ShotTask, the weapon's set `weapon_set`, pellets: `more`), a blow (`land`: MeleeTask), a blast (`blast_on` +
      explosion), fire (`burn`: OnFire), a stun gun (`zap`), a high fall (`in_air`), a miss (`near_miss`), death, damage
      `ped::damage`/`bump::damage`, a car (`car_react`), a prop (`obj_hit`), a throw (`throw`), additional reactions
      (`self_preserve`). The bullet's and blast's impulse is the Playroom's `applyImpulse`, GMod's damage force is zeroed.
      Golden: `shot` (600 steps), `blast`, `nearmiss`, `shotgun`, `death`, `getup`, `tasks` within tolerance. The
      original plan: what the Playroom does on a shot (ShotTask, the `taskparams` sets by weapon), a blow (`meleetask`),
      a blast (`explosion`), fire (`onfire`), a stun gun, a fall from a height (high fall and landing), a near miss
      (flinch, keeps balance), death (a dead Homer goes limp), health and damage (`delirium-data` `ped::damage`,
      `fall_damage`, `car_hit`) is repeated with the engine's same code through the `delirium` crate.
      `EntityTakeDamage` on Homer → a part (`nm_agent_nearest_part`/hitgroup) → the same Playroom task; the bullet's
      impulse in FOLLOW is already applied by the engine: **don't duplicate** `apply_push`.
- [x] Switches as in the Playroom with its defaults (`settings.txt` `--safe`): `tasks`, `avoid` (dodging), v0.2.1's
      additional reactions (hands on a wall, rails, leaning on a crate, watches and ducks from something falling, leans
      away from something thrown, looks round at who poked him), v0.2.2's "additional experimental", each its own
      convar, off by default as in the Playroom. Done: the convars `dms_tasks look avoid getup extra_reactions extra_pick
      fire_reaction dive_shot dodge_gap weak god` with the `--safe` defaults; of v0.2.2's experimental, what's about
      Homer's body: the dive under fire (`dive_shot`); the rest (Hero, Skate 3): "Deferred".
- [x] **Getting up** as in the Playroom (`getup`: 0 animation, 1 muscles along the clip, 2 off, 3 the trained network
      `getup.net`; the trigger: health ≥ 50, the head within 0.3 m of the pelvis, the body still). The default mode as
      in the Playroom. Get-up clips from the copy of the game (`Clips`, ADR 0010); the "animation" mode isn't NM:
      decide how to drive the body along the clip in FOLLOW (kinematic bodies in the fork, or the "muscles" mode through
      `incomingTransforms`); `getup.net` from Delirium (check the licence/CREDITS before distributing). Done: the clips
      `get_up_fast`/`idle` with the same code (`tools/dms-game/src/getup.rs`); "animation": the clip holds the bodies, at
      the end a new agent + `bodyBalance` (ADR 0020); "muscles": `task_pose`; "the trained network" isn't carried over
      (`learn.rs` is the room's code): Homer lies, as the Playroom without `getup.net` (so `getup.net` isn't distributed,
      M8). After getting up, the stance from the clip's pose drifts on Box3D and Homer steps (the Playroom stands): the
      same stance divergence as in M5.
- [x] Console commands (superadmin): `dms_attach`, `dms_send`, `dms_react`, `dms_detach_all`.
- [x] **Tuning and setting up behaviours** (ADR 0009), the server side: profiles `data/dms/profiles/*.json` (only the
      differences from the Playroom's defaults; the "Playroom" profile empty and unchangeable), overrides of NM message
      params (`nm_msg_set_*`) and `taskparams` sets (`<task> <set> <param>`), applied on the next step,
      `dms_tuning 0|1`, the active profile in `dms_status` and the telemetry; golden runs always without tuning. Done
      (`tuning.rs`, `tuning.lua`): the profile's values are added to the lines that start a behaviour; taskparams are a
      copy of the game's file (the PC version has none: nothing to change).
- [x] **Homer's face as in the Playroom** (`delirium-room` `face_view`): client drawing on the head capsule: the whites
      of the eyes, pupils, brows; the eyes blink, look where he's hit or along the velocity, squint in pain, close when
      unconscious, the brows when frightened, by the agent's telemetry (a blow, velocity, pain, unconscious). There too
      the optional skeleton overlay (`skeleton`, `bone_tint`) and the choice of colour scheme (the model's skin). Here
      from M4: the debug overlay: NM's capsules over the ragdoll, the drives' vectors, COM, stance.
- [x] The `shot` behaviour's impulses (`shot_out.impulse`) and the tasks' on Box3D's bodies, like stabilisation and
      `applyImpulse` in M4. The tasks' impulses (a bullet, a blast, a blow, a poke) are `applyImpulse`, carried over.
      `shot_out.impulse` (hand↔wound, a pair with a zero sum) and `grab` lie in the private `Ragdoll::pending`: they
      can't be reached without changing Delirium: not carried over (ADR 0020, point 6).
- [x] **The tuning menu** in the spawnmenu (the "DMS" tab, superadmin only, `net` messages to the server): the list of
      behaviours from `nm_message_count`/`nm_message_name`, for each the params with type, description
      (`nm_message_param_doc`), default and range: sliders, checkboxes, vectors; params the port takes but doesn't use
      (`nm_message_param_used` = 0) hidden or marked; reactions from the reaction table (which behaviour with what
      params on a shot, blast, fire, fall); live application to the chosen Homer, "Reset to Playroom", saving, loading
      and deleting profiles. Done: "Utilities → DMS" (`cl_menu.lua`); the API has no ranges for params: from the
      description or around the default.
- [x] The user's visual check: the face, skeleton, overlay, menu, getting up (docs/api.md, the "Client" section).

**DoD:** every reaction from the table starts from the console and Lua on Homer and sends NM the same messages with the
same params as the Playroom on the same reaction (golden reaction scenarios); documentation `docs/api.md`. In the menu
any param of any behaviour changes, is saved to a profile and resets to the Playroom; with the "Playroom" profile the
golden scenarios pass unchanged.

### M7. Reliability, performance, platforms — 5–7 days (cross-cutting, the final step before the release)

- [x] The lifecycle: removing the ragdoll, `undo`, `gmod_admin_cleanup`, changing the map, `lua_refresh`, unloading
      the module, save/load (agents aren't saved: recreated), the duplicator, objects falling asleep, freezing with the
      physgun, welds and ropes on parts, a teleport (`SetPos`): the agent reset (ADR 0021: copies without DMS's fields,
      parking while frozen, giving up the clip when picked up, NM's frame moving with a teleport).
- [x] Listen server: two environments in one process: hook only the server's (`GetObjectEnvironment`).
- [x] Performance: a budget per agent (the target ≤ 0.15 ms/step), the agents' limit (`dms_max_agents`), LOD: far or
      sleeping agents go to `relax` and are detached. Our layer ~0.02 ms, NM itself 1–2 ms (not ours); NM's step in
      parallel on threads (8 agents 14 → 4.3 ms), LOD: parking, NM's world: a crop (a spawn 175 → 29 ms).
- [x] Linux: not done: GMod's x64 server ABI is incompatible with the fork (ADR 0012), the user's decision: no
      dedicated server, tests in the game (ADR 0021). A port only on a separate request.
- [x] Crash resistance: no panics across the FFI boundary (`catch_unwind` in every `extern` entry).

**DoD:** a 30-minute run "bot server, 8 agents, spawn/remove/map change" without crashes or leaks; Linux dedicated
works (dropped: see Linux above). Done on 2026-10-27 on the bench (a listen server in the game): 30 min, 288 rounds of
spawning/removing 8 agents with shots, 11 map changes, without crashes or Lua errors, memory +~10 MB a map change (as
for the game without DMS); 2 blow-ups of bodies caught by the protection (`blew up`), the game went on.

### M8. Release — 2–3 days

Done (ADR 0024, [docs/install.md](docs/install.md), [THIRD_PARTY.md](THIRD_PARTY.md)). Version 0.1.0. Publishing (a tag,
a GitHub release, the Workshop) is the user's decision; the release notes are `docs/release-notes-0.1.0.md`.

- [x] Convars and the menu in the spawnmenu (utilities: attach, react, debug) with M6's tuning menu: one "DMS" tab,
      polishing the UI and hints. Done: the menu is the DMS window (`dms_menu`, ADR 0022) with Homer (spawning, NM on/off
      the Homer looked at, `dms_attach`/`dms_detach`), reactions, tuning, taskparams, M7's convars and a Debug tab
      (versions and problems, buttons for `dms_status`, `dms_selftest`, `dms_data`, `dms_info`, `dms_tuning_status`,
      `dms_nm_*`, an NM message line, `dms_react_log`, the overlay); attaching, reactions at a point and debugging (the
      inspector) are also the toolgun tool "DMS" (ADR 0023).
- [x] Documentation: installation (the fork's `vphysics.dll` + a backup, the binary module in `lua/bin`, NM's data
      from the player's GTA IV through `dms_game_dir` or `install.ps1 -Gta`, not `data/dms`: ADR 0010; Homer's model by
      `dms-homer`), FAQ ("Steam's file check puts the original vphysics back", Homer doesn't appear, no dedicated
      server/Linux), API (`docs/api.md`). All the documentation is in English.
- [x] Distribution: the Lua part on the Workshop; Homer's model isn't distributed, `dms-homer` makes it on the player's
      machine; the Workshop doesn't take `vphysics.dll` and `gmsv_dms_*.dll`: GitHub Releases + `install.ps1`. Done:
      `scripts\package.ps1` → `dist\DMS-<version>-win64.zip` (the module, the fork, `dms-homer.exe`, the addon, the
      player's `install.ps1`, `install.md`, licences), the C runtime linked in, no AVX2/FMA needed (checked by
      `dumpbin`); `dms_dev` off, the `dms-ctl` channel closed. Publishing waits for the user.
- [x] Versioning and a compatibility check at start: the module's version ↔ the Lua addon's version ↔ NM's interface.
      Done: the module and the addon of different releases don't start; `DMS_Problems()` (the fork and its interface,
      Homer's model by its mark `homer_version.txt` ↔ `DMS.HomerModelVersion`, the GTA IV folder), `DMS.DeliriumVersion`,
      `dms_info`.

---

## After 0.1.0: GMod's ragdolls moved out (M9–M12.5)

M9–M12.5 (NM on GMod's own ragdolls: the fit to any humanoid model, native NM bodies, ValveBiped and Workshop
skeletons, NPCs' death ragdolls, live NPCs and the drunk player, the rig editor `tools/dms-rig`) were done here up to
0.2.0 and moved to their own repository, `razedproduction/Delirium-ragdolls-in-GMod`, with their history
([ADR 0036](docs/adr/0036-project-split.md)). This repository is the golden project: Homer only, held to the Playroom.
The ragdolls project uses this one's core (the fork, `tools/dms-vphysics`, `tools/dms-game`, `tools/dms-ctl`) read only.

### The split (ADR 0036)

- [x] The ragdolls' code and Lua out of this repository: `tools/dms-fit`, `tools/dms-rig`, `module/src/ragdolls.rs`,
      the bench's `fit=` and the fitted LEAD agent, `golden fit` / `body --fit`, `ragdolls.lua`, `skeletons.lua`,
      `deaths.lua`, `live.lua`, the rig view, the menu's ragdoll section and their convars.
- [x] The hulls as the fork grows them and the `.phy` reader moved from `dms-homer` into the core (`dms-vphysics`).
- [ ] The new repository made from this one's history (the user: `gh repo create`), cleared of Homer there, its module
      and addon renamed, its core taken from here by path dependencies.
- [ ] The parts of `module/` the ragdolls project needs (the FOLLOW loop, the room layer, the Lua bridge) made a
      library here, Homer's behaviour unchanged (the golden comparison).

### Release 0.2.0

- [x] Version 0.2.0 (`Cargo.toml`, `DMS_ADDON_VERSION`); `MODEL_VERSION` stays 1 (what `dms-homer` writes for Homer
      hasn't changed since 0.1.0).
- [x] The Lua addon only from the Workshop (id 3829811829, published by the user from `addon\`), the player's installer
      `install.bat` instead of `install.ps1` (ADR 0035); `docs/install.md`, `docs/release-notes-0.2.0.md`. Publishing
      (the tag, the GitHub release, the Workshop update) waits for the user.

### Deferred: Playroom features not needed for Homer's behaviour

Not carried over (it's the Playroom application, not the body's behaviour): Hero mode, the weapon panel and the floating
pistol, Portal and Skate 3, learned walking and running (`walk.net`/`run.net`), GPU water, fire/blood/charring/damage
layers as graphics, graphics modes, crowds (`crowd`/`grid`). Candidates for later: body presets (`--body S|A|L|K|F|T`,
Homer through `Build::scaled`: cheap, but no longer 1.8 m), several Homers in one scene as in "Spawn Homers".

---

## Dependencies and parallelism

```
M0 ──► M1 ─┐
   └─► M2 ─┴─► M2.5 ──► M3 ──► M4 ──► M6 ──► M8
                                └───► M5 ─┘    ▲
                       M7 (cross-cutting, closed before M8) ┘
```

After 0.1.0, M9–M12.5 went on in the ragdolls repository (ADR 0036).

The critical path: **M0 → M1/M2 → M2.5 → M3 → M4**. From M2.5 on, every check in the game except the visual ones Claude
does itself on the bench in the game (without a window).

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Source's 66 Hz vs NM's 60 Hz | jitter, wrong drives | check in M4 on the reference; a step accumulator |
| Gravity 15.24 (Source) vs 9.8 (NM) | the body sags and falls | a compensating force on every part, `sv_gravity` unchanged (ADR 0007, M4) |
| NM's contacts (by its own world) diverge from Box3D | the balance "doesn't see" the support | an exact BSP mesh (M5), boxes for props; at worst, a fork extension with the parts' contacts |
| The order of `IPhysicsNM`'s vtable | a crash on a call | M1's FFI tests |
| The SDK's C API doesn't expose what the Playroom does (the outbox, the engine's tasks, the room's reactions) | catchFall, the shot, the blast not 1 to 1 | the same code through the `delirium` crate, as `delirium-room` (ADR 0016, M4/M6) |
| New Delirium versions change the behaviour (v0.2.2: the stance, catchFall) | the references go stale | at every SDK update, new Playroom references and a full golden comparison (as for v0.2.2) |
| The bench takes the user's game (one copy of GMod) | the user can't play during runs | `server.ps1 stop` right after a check; long runs when the user isn't playing |
| GMod's x64 dedicated server is incompatible with the fork (a new vphysics ABI) | no dedicated/Linux | tests only in the game (M2.5's decision); porting the fork only if needed in M7 |
| The `dms-ctl` channel open to the outside | remote control of the server | only `127.0.0.1`, only with `dms_dev 1`, a password a start, off in the release (M2.5, M8) |
| Box3D's physics differs from the Playroom's (inertia, the joint solver, contacts) | behaviour not 1 to 1 | golden comparison of the body (M3) and the behaviour (M4); fixes in the fork, not in NM (ADR 0008) |
| `dms-homer` ports the body inaccurately | every later milestone measures the wrong thing | `tools/golden body` against the Playroom's `NM_DUMP`, tolerance 1e-5 (M3) |
| PC `Fred.art` is little-endian | garbage in the joints, NaN | `delirium_data::LE` before reading, as in the Playroom; the `golden dump/lead` comparison (M2, ADR 0010) |
| Steam rolls `vphysics.dll` back | the addon "silently" doesn't work | an explicit interface check at start and a message to the player (M1, M8: `DMS_Problems`) |
| Incompatible changes of `IPhysicsNM` in the fork | crashes | a strict check of the version string, a copy of the header in the repository |
| The module, the addon or Homer's model from different releases | errors from the middle of the code | one version, checked at start; the model's mark (M8, ADR 0024) |
| The GPL-3 licence and GTA IV's data | legal | GPL-3 for everything, the data only from the user's copy |
| A change to the core breaks the ragdolls project (ADR 0036) | its build or behaviour | the core's API changed only with a note in the PR; the ragdolls repository builds against this one's checkout |

## The nearest steps (the first week)

1. All of M0: git, ADRs, committing the NM extension in the fork, the build scripts, a "hello" module.
2. M1: the `IPhysicsNM` FFI + `PhysObj → IPhysicsObject*` + the "hovering prop" test.
3. M2 in parallel: loading the data and `dms_selftest`.
