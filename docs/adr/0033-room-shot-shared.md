# 0033. The room's shot shared as `dms_vphysics::follow::shot`; NM's messages and buttons in the rig editor

Status: **proposed** (2026-11-03). Refines 0016 and 0032 (its point 3: the editor leaves the room out).

## Context

ROADMAP M12.5 step 2 asks for NM in FOLLOW in the rig editor as in the game: `bodyBalance`, a push, a shot and NM's
messages. 0032 gave the editor the FOLLOW step, bodyBalance, the Playroom's poke and grab, and a queue of message
lines, but no shot and no way to send a line. A shot in the game isn't a message: it's the Playroom room's `fire`
(`module/src/follow/room.rs`): the weapon's damage on his health (`ped::damage`), the limb's injury mask, the bullet's
impulse and the ShotTask with the weapon's taskparams set, then the task's messages every other step (`shot_frame`)
and death at health 0. Writing it again for the editor would be a second table of the Playroom's reactions (0008,
0016 forbid that).

## Decision

1. The room's shot moves from `room.rs` into `tools/dms-vphysics/src/follow/shot.rs`, unchanged in what it computes
   and in its order: the Playroom's weapons (`WEAPONS`, `weapon`), his forward (`forward`, the room's `Side`), its
   clock (`now`), the injury mask (`injury`), `fire` (damage, the line about it, the default aim from his front, the
   injury's line, `applyImpulse`, the ShotTask made or its next hit) and the ShotTask's frame (`task_frame`). The state
   they change is the caller's (`Victim`: health, the room's generator, the injuries, the task); what they say and send
   comes back as text and lines in the Playroom's order. The module's `Follower::fire`, `injure` and `shot_frame` are
   made of them; the module's behaviour doesn't change.
2. The editor's agent keeps the room's state the shot needs (`sim::Shots`: health, death, the generator, the
   injuries, the task, its switches as the Playroom's defaults) and runs it where the module's `room_before` does:
   after the queued lines, the outbox and the grab, the shots asked for, then on even steps the task's frame and death
   at health 0 (`STOP all`, `relax`). weaponinfo.xml and taskparams.txt are read from the player's GTA IV as the module
   reads them.
3. The editor sends NM's messages as the module does (`parse_line`, `msg::send`, `applyImpulse` on the fork's bodies,
   the tuning profile off): a line typed in the Simulate tab, or the Playroom's buttons that only send lines
   (`button_lines`).
4. The check (`--snap-report ... --sim <steps> --nm balance --shoot <weapon>`) shoots each body in the chest halfway
   and compares it with the same build in LEAD shot the same way (the LEAD reference now takes the same blow as the
   fork's body, push or shot).

## Consequences

- A rig is checked under a shot without the game, by the code that shoots in the game; a change to the shot is made
  once, for both.
- What's left out of the editor is the rest of the room (its tasks, get-ups, `in_air`, the additional reactions,
  blasts, fire, the taser, cars).
- The module's golden scenarios that shoot (`shot`, `shotgun`, `death`) are to be run again on the bench after the
  move.
- First results (600 physics steps, a pistol shot halfway): Homer's body staggers 1.8 m and stays up (health 175), as
  in LEAD; `humans/group01/male_07` (snapped) goes down 3.2 s in though it stands in LEAD, `player/group01/male_07`
  stays up.
