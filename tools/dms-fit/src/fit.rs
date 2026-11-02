//! Homer's build fitted to a model's skeleton (ROADMAP M9–M12, docs/adr/0025, 0027, 0030): the joints from its rig or
//! the player's, spread and put in Homer's stance, `Build::fit`, the player's edits, and how the model's bones are
//! posed from the parts. The module and `dms-rig` (the rig editor) both fit with it.

use delirium::prelude::*;
use delirium::ragdoll::Build;
use dms_game::body::Body;

use crate::rigs;
use crate::skeleton::Skeleton;
use crate::M;

/// Delirium's behaviours are tuned for about this height (m): a fitted body far from it gets a warning.
const TUNED_HEIGHT: f32 = 1.8;
const HEIGHT_WARN: f32 = 0.25;

/// NM's 21 joints on a mapped skeleton (m, the model's space; `at` its bones' frames in units), as
/// `NmCharacter.Joints` takes them from a Unity humanoid: the pelvis, then each part's joint with its parent; the 4
/// spine joints spread from the spine to the neck by `spread`; a missing neck or clavicle put between its neighbours.
/// Also what was made up.
pub fn mapped_joints(m: &rigs::Map, at: &[(Vec3, Quat)], spread: [f32; 3]) -> Result<([Vec3; 21], Vec<String>), String> {
    let at = |k: usize| m[k].map(|b| at[b].0 * M);
    let need = |k: usize| at(k).ok_or_else(|| format!("no bone for {}", rigs::PARTS[k]));
    let mut made = vec![];
    let (spine, head) = (need(7)?, need(12)?);
    let neck = at(11).unwrap_or_else(|| {
        made.push("Neck".to_string());
        spine.lerp(head, 0.85)
    });
    let (l_arm, r_arm) = (need(14)?, need(18)?);
    let mut clavicle = |k: usize, arm: Vec3, side: &str| {
        at(k).unwrap_or_else(|| {
            made.push(format!("Clavicle {side}"));
            neck.lerp(arm, 0.3)
        })
    };
    let (l_clav, r_clav) = (clavicle(13, l_arm, "L"), clavicle(17, r_arm, "R"));
    let j = [
        need(0)?, need(1)?, need(2)?, need(3)?, need(4)?, need(5)?, need(6)?,
        spine, spine.lerp(neck, spread[0]), spine.lerp(neck, spread[1]), spine.lerp(neck, spread[2]), neck, head,
        l_clav, l_arm, need(15)?, need(16)?,
        r_clav, r_arm, need(19)?, need(20)?,
    ];
    Ok((j, made))
}

/// Where Homer's Spine1, Spine2 and Spine3 joints are along his spine, from Spine0's joint (0) to the neck's (1),
/// as fractions of the way along his joints.
pub fn homer_spread(body: &Body) -> [f32; 3] {
    let pivot = |k: usize| body.joints.iter().find(|j| j.part == k).map_or(body.parts[k].pos, |j| body.parts[k].pos + body.parts[k].rot * j.anchor_child);
    let p: Vec<Vec3> = (7..=11).map(pivot).collect();
    let legs: Vec<f32> = p.windows(2).map(|w| w[0].distance(w[1])).collect();
    let all: f32 = legs.iter().sum::<f32>().max(1e-4);
    [legs[0] / all, (legs[0] + legs[1]) / all, (legs[0] + legs[1] + legs[2]) / all]
}

/// A body fitted to a model.
pub struct Fit {
    /// The fitted build (sizes and masses as edited) and its body as NM's physics has it.
    pub build: Build,
    pub body: Body,
    /// The `.mdl`'s checksum it was fitted from.
    pub checksum: i32,
    /// The fitted build's frames, standing (m, the model's space).
    pub frames: Vec<Transform>,
    /// The 21 joints it was fitted to (`Build::fit`'s input, in Homer's stance; m, the model's space).
    pub joints: Vec<Vec3>,
    /// The model's bones posed from parts (S4).
    pub posed: Vec<Posed>,
    /// The bones NM has a part for but the model has no physics object for, each with its nearest ancestor that has
    /// one: put right on the client (cl_ragdolls.lua).
    pub finish: Vec<(String, String)>,
    /// What the player should know: joints made up, a height far from NM's tuning, a broken bind pose.
    pub warnings: Vec<String>,
    pub height: f32,
    pub mass: f32,
    /// The skeleton's class (`rigs::Class::rig`) and the bone at each part.
    pub rig: String,
    pub map: [Option<String>; 21],
    /// The player's mapping and edits it was fitted by (`None`: the rig's own, none).
    pub names: Option<Vec<(String, String)>>,
    pub edits: Option<rigs::Edits>,
    /// The joints over the model in its own pose (m, the model's space; moved as edited).
    pub shown_joints: Vec<Vec3>,
    /// Each part's frame over the model in its own pose (m), for showing the capsules over it.
    pub shown: Vec<Transform>,
}

/// A model's bone posed from an NM part (S4): its frame in the part's (m), and whether it's the part's own bone (a
/// mapped joint, or a spine bone in the part's span) or a physics bone carried by its nearest posed ancestor's part.
pub struct Posed {
    pub bone: String,
    pub part: usize,
    pub at: Vec3,
    pub turn: Quat,
    pub own: bool,
}

/// How far `poseToBone` (the mesh's bind) may be from the chain of the bones' default transforms before the reader
/// calls the bind pose broken (units, degrees).
const POSE_GAP: f32 = 0.5;
const POSE_TURN: f32 = 2.0;

/// The reader's report on a model's bind pose: whether the mapped bones' default transforms disagree with the mesh's
/// bind (`poseToBone`; proportion tricks, broken exports), and what to say.
pub fn bind_report(s: &Skeleton, m: &rigs::Map) -> (bool, Vec<String>) {
    let mut out = vec![];
    let worst = m.iter().flatten().map(|&b| (b, s.bones[b].bind.0.distance(s.bones[b].pose.0), s.bones[b].bind.1.angle_between(s.bones[b].pose.1).to_degrees())).fold(
        None::<(usize, f32, f32)>,
        |w, x| match w {
            Some(w) if w.1 + w.2 * 0.1 >= x.1 + x.2 * 0.1 => Some(w),
            _ => Some(x),
        },
    );
    let broken = worst.is_some_and(|(_, gap, turn)| gap > POSE_GAP || turn > POSE_TURN);
    if let (true, Some((b, gap, turn))) = (broken, worst) {
        out.push(format!(
            "broken bind pose: the bones' default transforms are up to {gap:.1} units and {turn:.0}° off the mesh's bind ({}); fitted to the mesh's",
            s.bones[b].name
        ));
    }
    if s.bones.iter().any(|b| !(b.bind.0.is_finite() && b.bind.1.is_finite() && b.pose.0.is_finite() && b.pose.1.is_finite())) {
        out.push("the .mdl's bind pose isn't finite".into());
    }
    (broken, out)
}

/// Each part's segment: from its joint to this child's (the leaves, feet, hands and head, turn with their parent).
const SEGMENT: [(usize, usize); 15] = [(1, 2), (2, 3), (4, 5), (5, 6), (7, 8), (8, 9), (9, 10), (10, 11), (11, 12), (13, 14), (14, 15), (15, 16), (17, 18), (18, 19), (19, 20)];

/// Homer's 21 joints as he stands (his build's frames): each part's pivot on its parent, the root's centre.
fn homer_joints(body: &Body) -> Vec<Vec3> {
    (0..body.parts.len())
        .map(|k| body.joints.iter().find(|j| j.part == k).map_or(body.parts[k].pos, |j| body.parts[k].pos + body.parts[k].rot * j.anchor_child))
        .collect()
}

/// The model's joints (`bind`, its reference pose) put in Homer's stance (ROADMAP M10): each segment keeps the model's
/// length and takes the way Homer's goes, turned to the model's facing (its hips' line). NM's standing frames are what
/// it holds as the body's neutral, so a body fitted to ValveBiped's reference pose stood with its arms out and its head
/// forward. Also each part's turn from the reference pose to the stance, for the model's bones.
fn repose(body: &Body, bind: &[Vec3; 21]) -> ([Vec3; 21], [Quat; 21]) {
    let h = homer_joints(body);
    let flat = |v: Vec3| v.with_z(0.0).normalize_or(Vec3::X);
    // Homer's hips' line onto the model's: a turn about Z.
    let face = Quat::from_rotation_arc(flat(h[4] - h[1]), flat(bind[4] - bind[1]));
    let mut at = *bind;
    for k in 1..21 {
        let p = body.parts[k].parent.unwrap_or(0);
        let way = (face * (h[k] - h[p])).normalize_or_zero();
        let len = bind[k].distance(bind[p]);
        at[k] = if way == Vec3::ZERO { at[p] + (bind[k] - bind[p]) } else { at[p] + way * len };
    }
    let mut turn = [Quat::IDENTITY; 21];
    for k in 1..21 {
        let p = body.parts[k].parent.unwrap_or(0);
        turn[k] = match SEGMENT.iter().find(|s| s.0 == k) {
            Some(&(_, c)) => {
                let (a, b) = ((bind[c] - bind[k]).normalize_or_zero(), (at[c] - at[k]).normalize_or_zero());
                if a == Vec3::ZERO || b == Vec3::ZERO { turn[p] } else { Quat::from_rotation_arc(a, b) }
            }
            None => turn[p],
        };
    }
    (at, turn)
}

/// Homer's build (`homer`, its body `homer_body`) fitted to a humanoid skeleton (`Build::fit` on Homer's standing
/// frames): its joints found by its rig or a player's mapping (`names`, `{part, bone}`; ROADMAP M12), NM's spine joints spread as Homer's are along his
/// spine (ADR 0025), the skeleton put in Homer's stance first ([`repose`], ADR 0027). `model` names it in errors.
///
/// The player's `edits` (docs/adr/0030): joints moved before the fit (its input), then the parts' sizes and masses set
/// on the fitted build, and the parts moved and turned on the model's bones (how the model is posed from them).
pub fn fit(homer: &Build, homer_body: &Body, s: &Skeleton, model: &str, names: Option<&[(String, String)]>, edits: Option<&rigs::Edits>) -> Result<Fit, String> {
    let bind_at: Vec<Vec3> = s.bones.iter().map(|b| b.bind.0).collect();
    let class = rigs::classify(s, names, &bind_at)?;
    if !class.complete() {
        return Err(class.why(model));
    }
    let (broken, report) = bind_report(s, &class.map);
    // The mesh's bind where the bones' defaults disagree with it: the capsules go where the model is drawn.
    let at: Vec<(Vec3, Quat)> = s.bones.iter().map(|b| if broken { b.pose } else { b.bind }).collect();
    if at.iter().any(|(p, q)| !(p.is_finite() && q.is_finite())) {
        return Err(format!("{model}: the .mdl's bind pose isn't finite"));
    }
    let spread = homer_spread(homer_body);
    let (mut bind, made) = mapped_joints(&class.map, &at, spread)?;
    if let Some(e) = edits {
        for (j, o) in bind.iter_mut().zip(&e.joints) {
            *j += *o * M;
        }
    }
    let (joints, turn) = repose(homer_body, &bind);
    let frames: Vec<Transform> = homer_body.parts.iter().map(|p| Transform::from_translation(p.pos).with_rotation(p.rot)).collect();
    let (mut build, frames) = homer.fit(&frames, &joints);
    let mut body = Body::new(&build, &frames);
    if let Some(e) = edits.filter(|e| e.sizes() || e.masses()) {
        rigs::apply_sizes(&mut build, e);
        let now: Vec<f32> = Body::new(&build, &frames).parts.iter().map(|p| p.mass).collect();
        rigs::apply_masses(&mut build, e, &now);
        body = Body::new(&build, &frames);
    }
    // How each part lies on the model's bones (its bones' frames in it, and where it shows over the model).
    let adjust: [Option<Transform>; 21] = std::array::from_fn(|k| edits.and_then(|e| e.parts[k].adjust()));
    // The parts' own bones: the mapped joints, then the spine's bones in the spans of Spine0 to Spine3 (ValveBiped's
    // Spine1, Spine2, Spine4 one each, as M10 had them).
    let mut own: Vec<(usize, usize)> = (0..21).filter_map(|k| class.map[k].map(|b| (b, k))).collect();
    let chain = rigs::spine_chain(s, &class.map);
    let (spine, neck) = (bind[7], bind[11]);
    for (i, &b) in chain.iter().enumerate() {
        let k = if chain.len() == 3 {
            8 + i
        } else {
            let t = (at[b].0 * M - spine).dot(neck - spine) / (neck - spine).length_squared().max(1e-6);
            7 + spread.iter().filter(|&&x| t >= x).count()
        };
        own.push((b, k));
    }
    own.sort_by_key(|&(_, k)| k);
    // Every other physics bone is carried by its nearest own ancestor's part (hair, coats, tails stay on the body).
    let part_of = |mut b: usize| -> usize {
        loop {
            if let Some(&(_, k)) = own.iter().find(|o| o.0 == b) {
                return k;
            }
            match s.bones[b].parent {
                Some(p) => b = p,
                None => return 0,
            }
        }
    };
    let carried: Vec<(usize, usize)> = (0..s.bones.len()).filter(|&b| s.has_object(b) && !own.iter().any(|o| o.0 == b)).map(|b| (b, part_of(b))).collect();
    let posed: Vec<Posed> = own
        .iter()
        .map(|&o| (o, true))
        .chain(carried.iter().map(|&c| (c, false)))
        .map(|((b, k), own)| {
            // The bone as the stance has it: turned with its part about the part's joint.
            let (pos, rot) = (joints[k] + turn[k] * (at[b].0 * M - bind[k]), (turn[k] * at[b].1).normalize());
            let f = frames[k];
            let (mut at, mut turn) = (f.rotation.inverse() * (pos - f.translation), (f.rotation.inverse() * rot).normalize());
            if let Some(a) = adjust[k] {
                (at, turn) = (a.rotation.inverse() * (at - a.translation), (a.rotation.inverse() * turn).normalize());
            }
            Posed { bone: s.bones[b].name.clone(), part: k, at, turn, own }
        })
        .collect();
    let finish: Vec<(String, String)> = own
        .iter()
        .filter(|o| !s.has_object(o.0))
        .filter_map(|&(b, _)| {
            let mut at = s.bones[b].parent;
            while let Some(p) = at {
                if s.has_object(p) {
                    return Some((s.bones[b].name.clone(), s.bones[p].name.clone()));
                }
                at = s.bones[p].parent;
            }
            None
        })
        .collect();
    let shown: Vec<Transform> = (0..21)
        .map(|k| {
            let (f, back) = (frames[k], turn[k].inverse());
            let t = Transform::from_translation(bind[k] + back * (f.translation - joints[k])).with_rotation((back * f.rotation).normalize());
            match adjust[k] {
                Some(a) => Transform::from_translation(t.translation + t.rotation * a.translation).with_rotation((t.rotation * a.rotation).normalize()),
                None => t,
            }
        })
        .collect();
    let (height, mass) = (height(&body), body.parts.iter().map(|p| p.mass).sum::<f32>());
    let mut warnings: Vec<String> = made.iter().map(|m| format!("no {m} bone: put between its neighbours")).collect();
    if (height - TUNED_HEIGHT).abs() > HEIGHT_WARN {
        warnings.push(format!("{height:.2} m tall: Delirium's behaviours are tuned for about {TUNED_HEIGHT} m and work worse this far from it"));
    }
    warnings.extend(report);
    let unphysical = own.iter().filter(|o| !s.has_object(o.0)).count();
    if unphysical > 6 {
        warnings.push(format!("{unphysical} of NM's bones have no physics object here: they're drawn on the client only"));
    }
    if finish.len() > FINISH_MAX {
        warnings.push(format!("{} bones to finish on the client: only the first {FINISH_MAX} are", finish.len()));
    }
    let map = std::array::from_fn(|k| class.map[k].map(|b| s.bones[b].name.clone()));
    Ok(Fit {
        build,
        body,
        checksum: s.checksum,
        frames,
        joints: joints.to_vec(),
        posed,
        finish,
        warnings,
        height,
        mass,
        rig: class.rig,
        map,
        names: names.map(<[_]>::to_vec),
        edits: edits.cloned(),
        shown_joints: bind.to_vec(),
        shown,
    })
}

/// The most bones the client finishes (each takes two NW2 vars on the ragdoll).
pub const FINISH_MAX: usize = 16;

/// The body's height (m): its lowest point to its highest, its parts' capsules as they stand.
pub fn height(body: &Body) -> f32 {
    let (lo, hi) = body.parts.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
        let r = p.radius + p.half;
        (lo.min(p.pos.z - r), hi.max(p.pos.z + r))
    });
    hi - lo
}

