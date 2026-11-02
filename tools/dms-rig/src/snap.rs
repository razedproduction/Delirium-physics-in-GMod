//! Our own placement of NM's parts on a model (not the fit's): each part's shape laid on the mesh its bones carry —
//! along its bone from its joint to the next joint, or along its piece of mesh for the pelvis, feet, hands and head —
//! and sized to that mesh. The result is the rig's edits (sizes, the parts moved and turned on their bones).

use delirium::prelude::*;
use dms_fit::M;
use dms_fit::fit::Fit;
use dms_fit::skeleton::Skeleton;
use dms_game::body::Shape;

use crate::mesh::Mesh;

/// The joint each part runs to (its segment); `None`: laid along its mesh.
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

/// How much of a part's mesh its shape takes in: the thickness's share (the middle: a limb's skin is round it), and
/// the ends' cut off each side.
const THICK: f32 = 0.5;
/// A vertex counts for its bone's part when the bone has this much of it (the blends round the joints are left out
/// where a part has enough mesh without them).
const FIRM: f32 = 0.75;
const ENDS: f32 = 0.02;
/// The most a part is turned to lie along its bone (cos 50°): a shape lying across it (a pelvis, a spine) keeps its way.
const ALIGN: f32 = 0.64;

/// Where a part's shape goes: its centre, its shape's frame (the shape's own axes), its shape with the new sizes.
pub struct Target {
    pub k: usize,
    pub centre: Vec3,
    pub frame: Quat,
    pub shape: Shape,
}

pub fn shape_rot(s: &Shape) -> Quat {
    match *s {
        Shape::Capsule { rot, .. } | Shape::Box { rot, .. } => rot,
    }
}

/// The shape's long axis in its own frame: a capsule's Y, a box's longest.
fn long_axis(s: &Shape) -> Vec3 {
    match *s {
        Shape::Capsule { .. } => Vec3::Y,
        Shape::Box { half, .. } => Vec3::AXES[(0..3).max_by(|&a, &b| half[a].total_cmp(&half[b])).unwrap_or(1)],
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

fn seg_dist(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-9)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// The cloud's main axis (power iteration on its covariance), from `seed`.
fn principal(pts: &[Vec3], seed: Vec3) -> Vec3 {
    let mean = pts.iter().copied().sum::<Vec3>() / pts.len() as f32;
    let mut c = Mat3::ZERO;
    for p in pts {
        let d = *p - mean;
        c += Mat3::from_cols(d * d.x, d * d.y, d * d.z);
    }
    let mut v = seed.normalize_or(Vec3::Z);
    for _ in 0..32 {
        v = (c * v).normalize_or(v);
    }
    v
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
    let (mut firm, mut all) = (vec![vec![]; 21], vec![vec![]; 21]);
    for (p, bones) in mesh.pos.iter().zip(&mesh.bones) {
        let Some((b, w)) = bones.iter().filter(|x| x.1 > 0.0).max_by(|a, b| a.1.total_cmp(&b.1)).map(|x| (x.0 as usize, x.1)) else { continue };
        let Some(mut k) = part_of.get(b).copied().flatten() else { continue };
        let p = Vec3::from_array(*p) * M;
        if (7..=10).contains(&k) {
            k = (7..=10).min_by(|&a, &b| seg_dist(p, joints[a], joints[a + 1]).total_cmp(&seg_dist(p, joints[b], joints[b + 1]))).unwrap_or(k);
        }
        all[k].push(p);
        if w >= FIRM {
            firm[k].push(p);
        }
    }
    firm.into_iter().zip(all).map(|(f, a)| if f.len() >= 8 { f } else { a }).collect()
}

/// The parts' shapes laid on the mesh (those with too little mesh of their own are left out).
pub fn targets(skel: &Skeleton, mesh: &Mesh, map: &[Option<String>; 21], f: &Fit, parts: &[usize]) -> Vec<Target> {
    let j = &f.shown_joints;
    let clouds = clouds(skel, mesh, map, j);
    parts
        .iter()
        .filter_map(|&k| {
            let shape = f.body.parts[k].shape;
            let now = f.shown[k].rotation * shape_rot(&shape);
            let cur = now * long_axis(&shape);
            // The part's segment: its joint to the next.
            let seg = NEXT[k].map(|n| (j[k], j[n])).filter(|(a, b)| a.distance(*b) > 1e-4);
            // A shape lying across its segment (the pelvis', the spine's) takes only its slab of mesh; the spine's from the
            // whole chest (a chest's top is mostly its clavicles').
            let mut pts = if (7..=10).contains(&k) { [7, 8, 9, 10, 13, 17].iter().flat_map(|&i| clouds[i].iter().copied()).collect() } else { clouds[k].clone() };
            if let Some((a, b)) = seg {
                let (d, len) = ((b - a).normalize(), a.distance(b));
                let slab: Vec<Vec3> = pts.iter().copied().filter(|p| (0.0..=len).contains(&(*p - a).dot(d))).collect();
                if slab.len() >= 8 && cur.dot(d).abs() <= ALIGN {
                    pts = slab;
                }
            }
            if pts.len() < 8 {
                return None;
            }
            let mean = pts.iter().copied().sum::<Vec3>() / pts.len() as f32;
            let want = match seg {
                Some((a, b)) => (b - a).normalize(),
                None => principal(&pts, mean - j[k]),
            };
            // Only a long shape has a way to lie (a pelvis, a spine's slab, a neck, a head are round enough to keep theirs).
            let long = match shape {
                Shape::Capsule { radius, half, .. } => half >= radius * 0.75,
                Shape::Box { half, .. } => {
                    let mut h = half.to_array();
                    h.sort_by(f32::total_cmp);
                    h[2] >= h[1] * 1.4
                }
            };
            let aligned = long && cur.dot(want).abs() > ALIGN;
            let frame = if aligned { (Quat::from_rotation_arc(cur, want * cur.dot(want).signum()) * now).normalize() } else { now };
            // The segment's middle for a part along its bone, else the mesh's.
            let segment = seg.filter(|_| aligned).map(|(a, b)| ((a + b) * 0.5, a.distance(b)));
            let across = seg.filter(|_| !aligned).map(|(a, b)| (frame.inverse() * (b - a)).normalize());
            let origin = match (segment, seg) {
                (Some(s), _) => s.0,
                (None, Some((a, b))) => (a + b) * 0.5,
                _ => mean,
            };
            let local: Vec<Vec3> = pts.iter().map(|p| frame.inverse() * (*p - origin)).collect();
            let axis = |i: usize| -> (f32, f32) {
                let mut v: Vec<f32> = local.iter().map(|p| p[i]).collect();
                (quantile(&mut v, ENDS), quantile(&mut v, 1.0 - ENDS))
            };
            // The mesh's middle, kept on its segment's height for a shape across it.
            let middle = || {
                let m = Vec3::from_array([axis(0), axis(1), axis(2)].map(|(a, b)| (a + b) * 0.5));
                across.map_or(m, |d| m - d * m.dot(d))
            };
            let (centre, shape) = match shape {
                Shape::Capsule { rot, .. } => {
                    let (lo, hi) = match segment {
                        Some((_, len)) => (-len * 0.5, len * 0.5),
                        None => axis(1),
                    };
                    // Across the axis: the bone's line for a part along its bone, else the mesh's middle.
                    let mid = if segment.is_some() { Vec3::ZERO } else { middle() * Vec3::new(1.0, 0.0, 1.0) };
                    let near = local.iter().filter(|p| p.y >= lo && p.y <= hi);
                    let radius = match across {
                        // Across a segment: as deep as the slab (its height is the segment's).
                        Some(d) => {
                            let e = Vec3::Y.cross(d).normalize_or(Vec3::Z);
                            let mut v: Vec<f32> = near.map(|p| (*p - mid).dot(e).abs()).collect();
                            quantile(&mut v, 1.0 - ENDS * 4.0)
                        }
                        None => {
                            let mut v: Vec<f32> = near.map(|p| Vec2::new(p.x - mid.x, p.z - mid.z).length()).collect();
                            quantile(&mut v, THICK)
                        }
                    }
                    .max(0.01);
                    let half = ((hi - lo) * 0.5 - radius).max(0.005);
                    (origin + frame * Vec3::new(mid.x, (lo + hi) * 0.5, mid.z), Shape::Capsule { rot, radius, half })
                }
                Shape::Box { rot, .. } => {
                    let e = [axis(0), axis(1), axis(2)];
                    let half = Vec3::from_array(e.map(|(a, b)| ((b - a) * 0.5).max(0.01)));
                    (origin + frame * middle(), Shape::Box { rot, half })
                }
            };
            Some(Target { k, centre, frame, shape })
        })
        .collect()
}
