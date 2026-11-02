# 0030. Skeletons beyond ValveBiped: known rigs, a guess by names, the player's mappings; Workshop models as mounted

Status: **proposed** (2026-11-02, ROADMAP M12; accepted once the 10 Workshop models are checked). Refines 0025 (point 6:
other humanoids) and 0027 (the swap, the client's finishing).

## Context

M10–M11.5 fitted Homer's build to ValveBiped only: `VALVEBIPED` named NM's joints, `POSED` the bones posed from parts
and the client finished six named bones. Workshop models bring other rigs (3ds Max's Biped without the `ValveBiped.`
prefix, Mixamo, Unreal's mannequin, TF2/SFM, VRoid, MMD, arbitrary names), extra physics objects (hair, coats, tails)
and sometimes a bind pose that disagrees with the bones' default transforms. Two things showed up on the bench:

- `mstudiobone_t::physicsbone` is the object that moves a bone, so a bone without an object of its own carries its
  nearest ancestor's index: whether a bone has its own object is "its parent's index differs".
- A server console command `dms_skeleton` clashes with the client's convar of the same name (the Playroom's skeleton
  over Homer): on a listen server the client's `CreateClientConVar` returned nil. The command is `dms_rig`.

## Decision

1. **The skeleton's class** (`module/src/rigs.rs`, `DMS.ModelClassify`): a known rig by its bones' names (compared
   without case, after any `prefix:` or `prefix|`); else a guess by the names' words (split at separators, case changes
   and digits) and the hierarchy: each part the bone nearest the root that matches its words (and none of the helpers'
   words: twist, roll, IK, end, ...), a limb's bone only under the one before it, sides by `L`/`Left`/`R`/`Right`. The
   mapping is then checked: the required parts (all but the neck, the clavicles and NM's Spine1–3) found, no bone
   twice, each limb's bone under the one before it, no zero-length segment. The class is `ValveBiped`, a known rig's
   name, `by names`, `mapped` (the player's), `unknown` or `not humanoid` (no two thighs and two upper arms, or neither
   pelvis nor spine): the last two can't be fitted and say why.
2. **NM's Spine1–3 are never mapped**: spread between the spine and the neck in Homer's proportions (0025). The rig's
   bones between them are posed from the part whose span they're in (three bones: one each, as ValveBiped's Spine1,
   Spine2, Spine4 were).
3. **The rig editor** (the DMS window's Skeletons tab, `dms_rig_edit [model] [part]`), because `Build::fit` on the
   bones alone doesn't always fit a model well (the user's request): per NM part
   - its **bone** (NM's Spine1–3 stay spread, 2);
   - its **joint moved** from where the bone puts it (units, the model's space): the fit's input, as the spine's spread;
   - its **size** (a capsule's radius and half length, a box's half extents) and **mass** set on the fitted build (the
     inertia scaled as the shape's own moments are; the mass through the part's `heft`, so NM weighs it so);
   - how it **lies on the model's bone** (moved in the part's frame, turned): only how the model is posed from the part
     and drawn over it, NM's body untouched (a head or a hand that sits crooked on its capsule).
   The model is drawn in its own pose (the mesh's bind, from the `.mdl`, the bones it's skinned to at LOD 0) with the
   fitted capsules and joints over it, refitted on the server (`DMS.FitPreview`) as anything changes; a joint is
   dragged with the right mouse button. **Try it in the world** puts NM again on the model's ragdolls with the rig (a
   new one if there's none) without saving it, their NM bodies drawn over them (`dms_rig_view`, the fork's parts 15
   times a second, `DMS.RagdollParts`/`DMS.RagdollShapes`). Rigs are kept on the server in
   `data/dms/skeletons/<model path without models/ and .mdl>.json` (`{dms_skeleton = 2, model, checksum, bones = {[NM
   part] = bone}, joints = {[part] = {x, y, z}}, parts = {[part] = {radius, half, box, mass, offset, turn}}}`; format 1,
   bones only, still reads), copied as that text and pasted on another machine (a rig names bones, so it also fits
   another model with the same names). A tried rig wins over the saved one, the saved one over the model's own.
   Sizes and masses make this model's NM body, not Homer's, and nobody's by default: ADR 0008 holds for Homer and every
   golden run, and a body edited far from its fit behaves as its edits make it (a knee moved 10 cm up on one side falls).
4. **Physics bones that are no joint** (hair, coats) are posed from their nearest posed ancestor's part, rigidly as in
   the bind pose; the bones the client finishes are the parts' own bones without an object of their own, each in the
   frame of its nearest ancestor with one (sent as `DMS_Finish`, at most 16).
5. **A broken bind pose is reported by the reader**: where the mapped bones' default transforms are more than 0.5
   units or 2° off the mesh's bind (`poseToBone`; proportion tricks, broken exports), the fit uses the mesh's bind
   (the capsules go where the model is drawn) and says so; a non-finite bind pose isn't fitted.
6. **Workshop models are read where the game mounted them** (`file.Read(model, "GAME")`): nothing is copied, cached on
   disk or shipped. A model's first use from the player (the spawn menu with autoattach, `dms_attach`, `dms_ragdoll`)
   prepares it in the background (the fit, then the fork's 21 hulls a tick) with a chat message, and NM goes on once
   it's ready; later uses in the session are immediate. Deaths and live bodies keep M11's hulls made as NPCs and players
   spawn.
7. **Limits stated**: non-humanoids get the message of 1; a body far from 1.8 m a warning (0025); a broken bind pose
   the reader's line.
8. **Multiplayer**: with native bodies (0026) a joining client needs only the model, which it already has or
   subscribes to; the bones to finish come with the ragdoll (NW2). Mappings live on the server only.

## Consequences

- `DMS_IsValveBiped`/`DMS_HasValveBiped` are `DMS_IsMapped`/`DMS_HasMapped` (the old names kept); NPCs, players and
  ragdolls of any mapped humanoid get NM, death and live bodies included.
- ValveBiped's fit is unchanged: `golden body --fit` on male_07, female_01, combine_soldier and two Workshop
  ValveBiped models within the SDK's `nm_build_fit` (1e-5); the 6 finished bones as before.
- A rig's hulls are the fitted sizes: a mapping saved again makes the model's body again.
- Japanese MMD names only match when the `.mdl` stores them as UTF-8.
