# The rig editor (`dms-rig`)

A model whose skeleton DMS doesn't map by itself (or maps, but fits badly) gets a rig of its own: a file
`garrysmod/data/dms/skeletons/<model>.json` ([ADR 0030](adr/0030-workshop-skeletons.md)). `dms-rig` makes it by the mouse
over the model, with the module's own fit, and tries the body on VPhysics-Box3D itself
([ADR 0031](adr/0031-rig-editor.md)). The game can stay open beside it: the addon reloads a rig file within a second of
its change and gives the model's ragdolls with NM their new body.

```powershell
cargo build --release -p dms-rig
.\target\release\dms-rig.exe                                   # Garry's Mod from Steam, GTA IV from data\dms\game_dir.txt
.\target\release\dms-rig.exe --model models/player/foo.mdl     # a model at once
```

It needs your GTA IV (NM's body is Homer's, fitted: `--game`, else `garrysmod\data\dms\game_dir.txt`, else
`golden\game_dir.txt`) and Garry's Mod with the fork installed (`scripts\install.ps1 -Fork`; `--gmod`, `--vphysics`). It
reads models where the game has them (its VPKs, loose files, the Workshop's GMAs) and writes only the rig file.

## Rig

- **Models** (left): filter by words, *Workshop* for the Workshop's only; *Classify shown* tags each with its rig
  (`ValveBiped`, `Biped`, `by names`, `! unknown` …). Click one to open it: its mesh in its own pose, NM's 21 parts over
  it, its bones as lines.
- **Mapping** (right): a bone for each NM part, from the list or by *pick* and a click on the bone in the view.
  NM's Spine1–3 are always spread between the spine and the neck. A mapping equal to the model's own isn't written.
- **The mouse** in the view: click a part or a joint to select it. Drag a joint (yellow; violet for the spread spine
  joints) to move it. Right button orbits, middle (or Shift + right) pans, the wheel zooms.
- **The gizmo** on the selected part, one tool at a time (the second toolbar): *Select* (Q, none), *Move* (W: arrows
  along the axes, the white square on the screen's plane), *Rotate* (E: a ring about each axis, the outer grey one about
  the view), *Scale* (R: the blue squares change a capsule's radius and length or a box's extents), *All* (T). Its axes
  are the part's own (*Part*: a capsule's green axis is its length, along its bone), the model's bone's (*Bone*) or the
  world's; the rings turn the part about its centre or its joint. Move moves the part on its bone, or the joint when a
  joint was clicked last. Ctrl snaps: moves to 1 cm, sizes to 5 mm, turns to 5°; Esc cancels a drag.
- **Symmetry** (M): an edit of an arm's or a leg's part or joint is made on the other side too, across the model's plane
  between its hips: the joint's move mirrored, the size and mass copied, the part's placement its mirror image.
- **Snap to bones** (B: the selected part; Shift+B: all): our own placement, not the fit's. Each vertex of the mesh goes
  to the part of its main bone (firmly weighted vertices where a part has enough of them; an unmapped bone's to its
  nearest mapped ancestor's part, the spine's by its segments). A long part (a limb's, a clavicle's, a foot's or hand's
  box) is turned to lie along its bone from its joint to the next one (the feet and hands along their mesh's main axis),
  its capsule from joint to joint as thick as the middle of its mesh around the bone; a round one (the pelvis, the spine,
  the neck, the head) keeps its way and takes its piece of mesh: the spine's from the chest's slab between its joints,
  as deep as the chest. The result is ordinary edits (sizes, the parts moved and turned on their bones): undo, symmetry
  and the file as for any other; the masses stay.
- **IK** (I): dragging a wrist's or an ankle's joint carries the elbow or knee (the shoulder or hip stays, the limb's
  lengths kept, bent as it was); dragging an elbow or a knee turns it about the limb's line.
- **Numbers**: the selected part's joint offset (units), size (m), mass (kg), offset (m) and turn (degrees), editable
  by dragging or typing.
- **File**: *Save* (Ctrl+S) writes `data/dms/skeletons/<model>.json`; *Import* reads a rig file from anywhere (a rig sent
  by someone), *Paste JSON* takes its text, *Copy JSON* gives yours; *Model's own* clears the mapping and edits. Ctrl+Z
  and Ctrl+Y undo and redo every change.

## Simulate

The fitted body made by the fork as the module makes it (`CreateBody`, `CreateJoint`, NM's pivots, frames, limits,
inertia, materials and self-collision pairs) on a concrete floor, without NM: *Stand*, *Drop*, *Throw*, *Kick*, and
drag any part with the left button. Gravity is NM's 9.8 m/s² (what the body feels in the game under NM) or Source's
600 units/s². The model's mesh follows the parts as the game poses it. NM itself in the editor is a later step.

## For scripts

`--classify <text>` prints the class of every model whose path or source contains the text (no window);
`--bones <model>` its bones; `--select <part>` and `--tool <name>` set the view up, `--snap` snaps all the parts at once; `--screenshot <file.png> [--sim <steps>]` saves the window once drawn (after that many
physics steps) and quits.
