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
  joints) to move it; the selected part's blue handles change a capsule's radius and length or a box's extents; its
  white square moves the part on its bone and the three rings turn it. Right button orbits, middle (or Shift + right)
  pans, the wheel zooms.
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
`--bones <model>` its bones; `--screenshot <file.png> [--sim <steps>]` saves the window once drawn (after that many
physics steps) and quits.
