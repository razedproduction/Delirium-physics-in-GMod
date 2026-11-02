//! Our own placement of NM's parts on a model: Homer's proportions kept, laid on the model's bones, as thick as its
//! mesh.
//!
//! From the plain fit (the model's joints, no size or placement edits: Homer's parts scaled by the model's bone
//! lengths), each part
//! - keeps Homer's length for its bone (his capsule's half length and its caps' reach past the joints, scaled);
//! - lies along the model's bone when it is long (a limb's, a clavicle's along the joint to the next; a foot's box along
//!   the foot's bones, level; a hand's along its fingers), its cross-section centred in its mesh; a round part (the
//!   pelvis, the spine, the neck, the head) keeps its way and place;
//! - is as thick as Homer's times its group's factor (legs, arms, torso; the head its own): the mesh's thickness round
//!   the group's long parts to the plain fit's, one factor so that one odd part doesn't spoil the rest.
//!
//! The result is the rig's edits (sizes, the parts moved and turned on their bones); masses stay.

use delirium::prelude::*;
use dms_fit::M;
use dms_fit::fit::Fit;
use dms_fit::skeleton::Skeleton;
use dms_game::body::Shape;

use crate::mesh::Mesh;

/// The joint each part runs to (its segment); `None`: a leaf or the pelvis.
const NEXT: [Option<usize>; 21] = [
    None,
    Some(2),
    Some(3),
    None,
    Some(5),
    Some(6),
    None,
    Some(8),
    Some(9),
    Some(10),
    Some(11),
    Some(12),
    None,
    Some(14),
    Some(15),
    Some(16),
    None,
    Some(18),
    Some(19),
    Some(20),
    None,
];

/// The groups' long parts that measure their thickness: the thighs and shins, the upper and lower arms, the spine.
const LEGS: [usize; 4] = [1, 2, 4, 5];
const ARMS: [usize; 4] = [14, 15, 18, 19];
const SPINE: [usize; 4] = [7, 8, 9, 10];
/// A vertex counts for its bone's part when the bone has this much of it (the blends round the joints are left out
/// where a part has enough mesh without them).
const FIRM: f32 = 0.75;
/// The share of a segment's middle that measures it (the ends are its joints' blends).
const MIDDLE: (f32, f32) = (0.15, 0.85);
/// The most a part is turned to lie along its bone (cos 50°).
const ALIGN: f32 = 0.64;
/// A group's factor's bounds.
const FACTOR: (f32, f32) = (0.5, 2.0);

/// Where a part's shape goes: its centre, its shape's frame (the shape's own axes), its shape with the new sizes.
pub struct Target {
    pub k: usize,
    pub centre: Vec3,
    pub frame: Quat,
    pub shape: Shape,
}

/// The thickness factors found (the mesh's to Homer's scaled), for the status line.
pub struct Factors {
    pub legs: f32,
    pub arms: f32,
    pub torso: f32,
    pub head: f32,
}

pub fn shape_rot(s: &Shape) -> Quat {
    match *s {
        Shape::Capsule { rot, .. } | Shape::Box { rot, .. } => rot,
    }
}

/// The shape's long axis in its own frame (a capsule's Y, a box's longest) and whether it has one worth lying along.
fn long_axis(s: &Shape) -> (Vec3, bool) {
    match *s {
        Shape::Capsule { radius, half, .. } => (Vec3::Y, half >= radius * 0.75),
        Shape::Box { half, .. } => {
            let i = (0..3).max_by(|&a, &b| half[a].total_cmp(&half[b])).unwrap_or(1);
            let mut h = half.to_array();
            h.sort_by(f32::total_cmp);
            (Vec3::AXES[i], h[2] >= h[1] * 1.4)
        }
    }
}

/// The `q` quantile of `v` (sorted in place).
fn quantile(v: &mut [f32], q: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f32::total_cmp);
    v[((v.len() - 1) as f32 * q).round() as usize]
}

fn median(mut v: Vec<f32>) -> Option<f32> {
    (!v.is_empty()).then(|| quantile(&mut v, 0.5))
}

fn seg_dist(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-9)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// The mesh's vertices (m) by the part their main bone belongs to: its own mapped bone's, else its nearest mapped
/// ancestor's; the spine's by the nearest of its four segments.
fn clouds(skel: &Skeleton, mesh: &Mesh, map: &[Option<String>; 21], joints: &[Vec3]) -> Vec<Vec<Vec3>> {
    let own: Vec<Option<usize>> = skel.bones.iter().map(|b| map.iter().position(|x| x.as_deref() == Some(b.name.as_str()))).collect();
    let part_of: Vec<Option<usize>> = (0..skel.bones.len())
        .map(|mut b| loop {
            if let Some(k) = own[b] {
                break Some(k);
            }
            match skel.bones[b].parent {
                Some(p) => b = p,
                None => break None,
            }
        })
        .collect();
    // Only the vertices drawn: the `.vvd` also holds the other body groups' (a model's held props, its other heads).
    let mut drawn = vec![false; mesh.pos.len()];
    for &i in mesh.tris.iter().flatten() {
        if let Some(d) = drawn.get_mut(i as usize) {
            *d = true;
        }
    }
    let (mut firm, mut all) = (vec![vec![]; 21], vec![vec![]; 21]);
    for ((p, bones), _) in mesh.pos.iter().zip(&mesh.bones).zip(&drawn).filter(|x| *x.1) {
        let Some((b, w)) = bones.iter().filter(|x| x.1 > 0.0).max_by(|a, b| a.1.total_cmp(&b.1)).map(|x| (x.0 as usize, x.1)) else { continue };
        let Some(mut k) = part_of.get(b).copied().flatten() else { continue };
        let p = Vec3::from_array(*p) * M;
        if SPINE.contains(&k) {
            k = SPINE.into_iter().min_by(|&a, &b| seg_dist(p, joints[a], joints[a + 1]).total_cmp(&seg_dist(p, joints[b], joints[b + 1]))).unwrap_or(k);
        }
        all[k].push(p);
        if w >= FIRM {
            firm[k].push(p);
        }
    }
    firm.into_iter().zip(all).map(|(f, a)| if f.len() >= 8 { f } else { a }).collect()
}

/// The points of `pts` in the middle of the segment `a`–`b`, as offsets across it.
fn across(pts: &[Vec3], a: Vec3, b: Vec3) -> Vec<Vec3> {
    let (d, len) = ((b - a).normalize_or_zero(), a.distance(b));
    pts.iter()
        .filter_map(|p| {
            let t = (*p - a).dot(d);
            (t >= len * MIDDLE.0 && t <= len * MIDDLE.1).then(|| *p - a - d * t)
        })
        .collect()
}

/// The middle of points (each axis' 2–98% span's middle).
fn middle(pts: &[Vec3]) -> Vec3 {
    Vec3::from_array([0, 1, 2].map(|i| {
        let mut v: Vec<f32> = pts.iter().map(|o| o[i]).collect();
        (quantile(&mut v, 0.02) + quantile(&mut v, 0.98)) * 0.5
    }))
}

/// The middle of offsets and how far round it they lie (the median).
fn round(offsets: &[Vec3]) -> Option<(Vec3, f32)> {
    if offsets.len() < 8 {
        return None;
    }
    let mid = middle(offsets);
    median(offsets.iter().map(|o| (*o - mid).length()).collect()).map(|r| (mid, r))
}

fn thickness(s: &Shape) -> f32 {
    match *s {
        Shape::Capsule { radius, .. } => radius,
        Shape::Box { half, .. } => half.min_element(),
    }
}

/// The model's way from a leaf part's bone to its farthest bone below (a foot's toes, a hand's fingers), `None` without.
fn leaf_way(skel: &Skeleton, map: &[Option<String>; 21], k: usize) -> Option<Vec3> {
    let b = skel.bones.iter().position(|x| map[k].as_deref() == Some(x.name.as_str()))?;
    let below = |mut c: usize| loop {
        match skel.bones[c].parent {
            Some(p) if p == b => break true,
            Some(p) => c = p,
            None => break false,
        }
    };
    let at = skel.bones[b].bind.0;
    (0..skel.bones.len()).filter(|&c| below(c)).map(|c| skel.bones[c].bind.0 - at).max_by(|x, y| x.length().total_cmp(&y.length())).filter(|w| w.length() > 0.5)
}

/// The groups' thickness factors from the mesh against the plain fit.
fn factors(clouds: &[Vec<Vec3>], plain: &Fit) -> Factors {
    let j = &plain.shown_joints;
    let limb = |ks: &[usize]| {
        let ratios = ks
            .iter()
            .filter_map(|&k| {
                let n = NEXT[k]?;
                round(&across(&clouds[k], j[k], j[n])).map(|(_, r)| r / thickness(&plain.body.parts[k].shape).max(1e-3))
            })
            .collect();
        median(ratios).map_or(1.0, |g| g.clamp(FACTOR.0, FACTOR.1))
    };
    // The torso: the chest's depth (front to back) at each spine segment to the spine capsule's radius (they lie
    // across the body: their radius is its depth). The chest's top is mostly its clavicles'.
    let chest: Vec<Vec3> = [7, 8, 9, 10, 13, 17].iter().flat_map(|&i| clouds[i].iter().copied()).collect();
    let torso = median(
        SPINE
            .iter()
            .filter_map(|&k| {
                let off = across(&chest, j[k], j[k + 1]);
                let s = plain.body.parts[k].shape;
                let side = plain.shown[k].rotation * shape_rot(&s) * long_axis(&s).0;
                let depth = side.cross(j[k + 1] - j[k]).normalize_or_zero();
                if off.len() < 8 || depth == Vec3::ZERO {
                    return None;
                }
                let mut v: Vec<f32> = off.iter().map(|o| o.dot(depth)).collect();
                let half = (quantile(&mut v, 0.98) - quantile(&mut v, 0.02)) * 0.5;
                Some(half / thickness(&s).max(1e-3))
            })
            .collect(),
    )
    .map_or(1.0, |g| g.clamp(FACTOR.0, FACTOR.1));
    // The head: its mesh's reach round its middle to the head capsule's.
    let head = round(&clouds[12])
        .map(|(_, r)| match plain.body.parts[12].shape {
            Shape::Capsule { radius, half, .. } => r / (radius + half * 0.5),
            Shape::Box { half, .. } => r / half.length(),
        })
        .map_or(1.0, |g| g.clamp(FACTOR.0, FACTOR.1));
    Factors { legs: limb(&LEGS), arms: limb(&ARMS), torso, head }
}

/// The parts' shapes on the model from the plain fit (`plain`), and the thickness factors used.
pub fn targets(skel: &Skeleton, mesh: &Mesh, map: &[Option<String>; 21], plain: &Fit, parts: &[usize]) -> (Vec<Target>, Factors) {
    let j = &plain.shown_joints;
    let clouds = clouds(skel, mesh, map, j);
    let g = factors(&clouds, plain);
    let out = parts
        .iter()
        .map(|&k| {
            let shape = plain.body.parts[k].shape;
            let now = plain.shown[k].rotation * shape_rot(&shape);
            let centre0 = plain.shown[k].translation;
            let (long, lengthy) = long_axis(&shape);
            let cur = now * long;
            let factor = match k {
                1..=6 => g.legs,
                13..=20 => g.arms,
                12 => g.head,
                _ => g.torso,
            };
            // The model's way for a long part: its bone, its foot's bones level, its fingers.
            let way = match (NEXT[k], k) {
                (Some(n), _) => Some(j[n] - j[k]),
                (None, 3 | 6) => leaf_way(skel, map, k).map(|w| w.with_z(0.0)),
                (None, 16 | 20) => leaf_way(skel, map, k),
                _ => None,
            }
            .map(|w| w.normalize_or_zero())
            .filter(|w| lengthy && *w != Vec3::ZERO && cur.dot(*w).abs() > ALIGN);
            let frame = way.map_or(now, |w| (Quat::from_rotation_arc(cur, w * cur.dot(w).signum()) * now).normalize());
            let centre = match (way, NEXT[k]) {
                // Along its bone: where the plain fit has it along the bone, centred across in its mesh.
                (Some(_), Some(n)) => {
                    let (a, d) = (j[k], (j[n] - j[k]).normalize());
                    let on = a + d * (centre0 - a).dot(d);
                    on + round(&across(&clouds[k], j[k], j[n])).map_or(Vec3::ZERO, |(m, _)| m)
                }
                // A foot or a hand: centred across in its mesh; along its length where the plain fit has it.
                (Some(_), None) if clouds[k].len() >= 8 => {
                    let along = frame * long;
                    let m = middle(&clouds[k].iter().map(|p| *p - centre0).collect::<Vec<_>>());
                    centre0 + m - along * m.dot(along)
                }
                _ => centre0,
            };
            let shape = match shape {
                Shape::Capsule { rot, radius, half } => Shape::Capsule { rot, radius: radius * factor, half },
                Shape::Box { rot, half } => Shape::Box { rot, half: Vec3::from_array([0, 1, 2].map(|i| if Vec3::AXES[i] == long { half[i] } else { half[i] * factor })) },
            };
            Target { k, centre, frame, shape }
        })
        .collect();
    (out, g)
}
