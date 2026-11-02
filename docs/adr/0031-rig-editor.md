# 0031. The rig editor: `tools/dms-rig`, an application of its own on the module's fit and the fork itself

Status: **accepted** (2026-11-02, ROADMAP M12.5). Refines 0030 (point 3: the editor), 0026 and 0027 (the native body).

## Context

M12 made a model's rig a file (`data/dms/skeletons/<model>.json`, format 2) and removed the in-game tab with sliders
(the user found it awkward). The user's choice: an application of its own (Rust, egui) that edits the rig by the mouse
over the model, writes the file the addon already reloads, and simulates the body on the very physics the game runs
(VPhysics-Box3D), first without NM, then with it. Three things stood in the way:

- the fit was the module's: done in M12 (`tools/dms-fit`);
- the bind and the native body were Lua natives (`bind.rs` pushed its result onto the Lua stack, `ragdolls.rs` made
  the body inside `DMS.RagdollBody`), and the `IPhysicsNM` bindings lived in the module;
- the fork had only ever run inside the game, which makes the environment, the floor and the surface properties.

## Decision

1. **`tools/dms-vphysics`** (new, no Lua), shared by the module and the editor:
   - `nm`: the `IPhysicsNM` bindings (`VBOX3D_NM_006`, moved from `module/src/ffi/vphysics_nm.rs`, which re-exports
     them; the header's copy stays there), plus the few `IPhysicsEnvironment` and `IPhysicsObject` slots a host
     needs (gravity, air density, a static object, `Simulate`; `Wake`, `ApplyForceOffset`);
   - `body`: the bind (`bind`: NM's inertia, materials, pivots, frames and limits) and the native body
     (`NativeBody::make`/`destroy`/`states`) exactly as the module had them; the module's natives call them;
   - `host`: the fork loaded into a process of its own (`LoadLibrary` with Garry's Mod's `bin\win64` for `tier0.dll`),
     `IPhysics` (`CreateEnvironment`), `IPhysicsCollision` (`BBoxToCollide`) and `IPhysicsSurfaceProps`
     (`ParseSurfaceData`, the game's `scripts/surfaceproperties*.txt` by its manifest), an environment as the game's
     server sets one (0.015 s steps, air density 2), a static floor. Source SDK 2013's vtables, MSVC's layout (a
     virtual destructor one slot). Nothing in the fork changes.
2. **`tools/dms-rig`** (Rust, eframe/egui with OpenGL; win64, no AVX2):
   - **models where the game has them**, read only: the loose files and legacy addons, the game's VPKs
     (`garrysmod_dir.vpk`, `sourceengine/hl2_misc`, `content_hl2`, `content_cstrike`), the Workshop's GMAs (Steam's
     `workshop/content/4000` and `garrysmod/cache/workshop`). `.mdl` bones (`dms_fit::skeleton`), `.vvd` and `.dx90.vtx`
     the mesh (the first model of each body part, LOD 0), drawn in its own pose (the mesh's bind);
   - **the rig over it** by the module's own code: Homer from the player's GTA IV as `module/src/data.rs` loads him
     (`--game`, `data/dms/game_dir.txt`, `golden/game_dir.txt`), `dms_fit::fit::fit` on every change;
   - **the mouse**: a part picked by clicking it (or its joint), a bone given to a part by clicking it ("pick"),
     joints dragged in the plane facing the eye (`joints`, units), a capsule's radius and length and a box's extents
     by handles, a part moved on its bone by its centre and turned by three rings (`offset`, `turn`), the numbers
     beside (drag values); undo and redo of every edit; the rig saved where the addon reads it (Ctrl+S), imported
     from a file or pasted, copied as text;
   - **the simulation, step 1**: the fork loaded (`--vphysics`, else Garry's Mod's `bin\win64`, else the repository's
     `vphysics/build`), the fitted body made by `NativeBody::make` (no owner) on a concrete floor with NM's CONCRETE
     row, NM's gravity (9.8 m/s²: what the body feels under NM, 0007) or Source's; stood, dropped, thrown, kicked and
     dragged by the mouse (an impulse toward the pointer each step, as the physgun pulls); the model's mesh skinned on
     the CPU from the parts as the game poses it (the parts' bones, the rest after their parents);
   - for scripts and checks: `--classify <text>` (every model's class, no window), `--bones <model>`,
     `--screenshot <png> [--sim <steps>]`.
3. **Step 2, NM in FOLLOW inside the editor, is not done**: it needs the module's FOLLOW (`follow.rs`, `follow/room.rs`,
   `step.rs`, `agents.rs`: about 150 KB tied to Lua's queues) out into a crate without Lua and Homer's golden held
   after it. It stays a ROADMAP item.

## Consequences

- The module's bind and native body are the editor's: a body simulated in the editor is the game's body (the same
  hulls, masses, tensors, pivots, limits, collision pairs); Homer's golden runs unchanged after the move.
- The editor needs the player's GTA IV (Homer is NM's body) and Garry's Mod with the fork installed; it writes only the
  rig file. No model or game data is copied.
- Rigs stay the files of 0030: the editor adds nothing to the format. A rig made for another build of a model (another
  checksum) still opens: bones are matched by name.
- A model without a bone for a required part (HL2's stalker and skeleton have no hands, the poison zombie no head)
  can't get a rig in the editor either: the fit needs a bone per required joint.
- The fit's feet and hands follow their own bones (`dms-fit`, `turn_leaves`). The fit builds NM's body in Homer's
  stance (ADR 0027) and turns each part back onto the model's bind pose by its segment; the feet and hands have no child
  joint in NM, so they used to take their shin's or forearm's turn and kept Homer's ankle and wrist: on male_07 a foot
  pointed 27–34° off its toes (level), a hand 6–9° off its fingers. Now each is turned on from its parent's turn round
  to its own bones' way (the farthest bone below: the toes, the fingers; a foot's level, its pitch stays its shin's) as
  the fitted part goes from its joint to its centre: 0.2–2.9° and 0°. Only how the model's bones sit on the parts
  changes (`Posed`, the editor's `shown`): the build, the frames, the masses and the joints are as before
  (`golden body --fit` OK). A rig's hand and foot placement edits are now relative to the new frames.
- The editor's Snap (B, Shift+B) is our own placement, not the fit's: from the plain fit, the long parts laid along the
  model's bones at Homer's length and centred in their cross-sections, the feet and hands over their own, thickness
  Homer's times bounded factors (the body's, its group's, a small one of its own). It measures the model's ragdoll
  hulls (`.phy`) where it has them, else its mesh, robustly (the inner shell round a bone, outliers by their median
  deviation, both sides as one), and leaves a part as the plain fit has it where the data are few or contradict each
  other, saying why. Its first version took one factor per group from the mesh's 2–98% spans and ballooned on models
  with armour, pouches or coats (a pelvis given the vest's ×1.87, hands 37 cm off the wrist where the fit put them).
  `dms-rig --snap-report` scores it against each part's bone and mesh with explicit bounds and drops each body on the
  fork (docs/rig-editor.md).