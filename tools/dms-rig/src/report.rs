//! `--snap-report`: how well the parts lie on models' bones and meshes, measured without a window. Each part is
//! scored against its own bone and its own mesh (the drawn vertices its bones carry), each pair against its mirror
//! image; a score past its bound is a fault, printed with the part. Snap's own estimates aren't used: the scores are
//! the truth Snap is checked by.

use delirium::prelude::*;
use dms_fit::M;
use dms_fit::fit::Fit;
use dms_fit::rigs::{self, PARTS};
use dms_fit::skeleton::{Skeleton, read_mdl};
use dms_game::body::Shape;

use crate::Homer;
use crate::gamefs::GameFs;
use crate::mesh::{self, Mesh};
use crate::rigfile::Rig;
use crate::sim::{self, Sim};
use crate::snap;

/// The long parts and the joint each runs to: the thighs, shins, clavicles, upper and lower arms.
const LONG: [(usize, usize); 10] = [(1, 2), (2, 3), (4, 5), (5, 6), (13, 14), (14, 15), (15, 16), (17, 18), (18, 19), (19, 20)];
/// The feet and hands: their way is their mesh's (from the ankle or wrist to their vertices' middle).
const LEAVES: [usize; 4] = [3, 6, 16, 20];
const SPINE: [usize; 4] = [7, 8, 9, 10];
const PAIRS: [(usize, usize); 7] = [(1, 4), (2, 5), (3, 6), (13, 17), (14, 18), (15, 19), (16, 20)];

/// The faults' bounds.
/// A long part's axis off its bone, a foot's or hand's off its mesh's way (degrees).
const ANGLE: f32 = 20.0;
/// The head's face off the model's (degrees).
const FACE: f32 = 10.0;
/// A part's thickness to its mesh's (a limb's radius to its cross-section's, a foot's or hand's extents to its
/// vertices').
const RATIO: (f32, f32) = (0.55, 1.4);
/// How far the shape's surface stands out of the mesh (its 80th percentile): to the part's thickness, and at least; a
/// foot's or hand's box to its middle extent (a box is flat over fingers and a foot's wedge).
const OUT: (f32, f32) = (0.5, 0.025);
const OUT_BOX: f32 = 0.5;
/// The share of the part's own vertices inside its shape (a shape as thick as its mesh holds about half of them; a
/// shape fatter than its mesh swallows them).
const INSIDE: f32 = 0.85;
/// A limb's centre off the middle of its mesh across, to its thickness.
const OFF_MESH: f32 = 0.6;
/// A part's centre off its bone's line: to its thickness, and at least.
const OFF_BONE: (f32, f32) = (1.5, 0.04);
/// The sides' differences: thickness (share), centres mirrored (m), axes mirrored (degrees).
const SYM: (f32, f32, f32) = (0.15, 0.03, 12.0);
/// A part's thickness to the plain fit's (Homer's scaled to the model's bones; Snap's floor, a little rounding).
const HOMER: (f32, f32) = (snap::FLOOR - 0.005, 2.2);

/// One part's scores.
#[derive(Default, Clone)]
pub struct Score {
    pub angle: Option<f32>,
    pub ratio: Option<f32>,
    pub out: f32,
    pub inside: Option<f32>,
    pub off_mesh: Option<f32>,
    pub off_bone: Option<f32>,
    pub homer: f32,
    pub thick: f32,
    pub verts: usize,
    /// How far out of the mesh is a fault.
    pub out_bound: f32,
}

fn thickness(s: &Shape) -> f32 {
    match *s {
        Shape::Capsule { radius, .. } => radius,
        Shape::Box { half, .. } => half.min_element(),
    }
}

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

/// The drawn vertices (m) and their normals, and each one's part (its main bone's, or its nearest mapped ancestor's;
/// the spine's by the nearest of its segments).
fn parts_of(skel: &Skeleton, mesh: &Mesh, map: &[Option<String>; 21], j: &[Vec3]) -> (Vec<Vec3>, Vec<Vec3>, Vec<Option<usize>>) {
    let own: Vec<Option<usize>> = skel.bones.iter().map(|b| map.iter().position(|x| x.as_deref() == Some(b.name.as_str()))).collect();
    let part_of = |mut b: usize| loop {
        if let Some(k) = own.get(b).copied().flatten() {
            break Some(k);
        }
        match skel.bones.get(b).and_then(|x| x.parent) {
            Some(p) => b = p,
            None => break None,
        }
    };
    let mut drawn = vec![false; mesh.pos.len()];
    for &i in mesh.tris.iter().flatten() {
        if let Some(d) = drawn.get_mut(i as usize) {
            *d = true;
        }
    }
    let (mut pos, mut nor, mut part) = (vec![], vec![], vec![]);
    for i in (0..mesh.pos.len()).filter(|&i| drawn[i]) {
        let p = Vec3::from_array(mesh.pos[i]) * M;
        let main = mesh.bones[i].iter().filter(|x| x.1 > 0.0).max_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0 as usize);
        let mut k = main.and_then(part_of);
        if k.is_some_and(|k| SPINE.contains(&k)) {
            k = SPINE.into_iter().min_by(|&a, &b| seg_dist(p, j[a], j[a + 1]).total_cmp(&seg_dist(p, j[b], j[b + 1])));
        }
        pos.push(p);
        nor.push(Vec3::from_array(mesh.normal[i]).normalize_or_zero());
        part.push(k);
    }
    (pos, nor, part)
}

/// Points on a shape's surface (its own frame, centred); a capsule's caps with `caps` (a limb's reach past its joints
/// into its neighbours', as Homer's do, and stand out of a bent elbow or knee however thick the limb is).
fn surface(s: &Shape, caps: bool) -> Vec<Vec3> {
    let mut out = vec![];
    match *s {
        Shape::Capsule { rot, radius, half } => {
            for i in 0..=6 {
                let y = -half + 2.0 * half * i as f32 / 6.0;
                for a in 0..12 {
                    let t = a as f32 / 12.0 * std::f32::consts::TAU;
                    out.push(rot * Vec3::new(t.cos() * radius, y, t.sin() * radius));
                }
            }
            for sign in [-1.0f32, 1.0].into_iter().filter(|_| caps) {
                for lat in 1..=3 {
                    let phi = lat as f32 / 3.0 * std::f32::consts::FRAC_PI_2;
                    for a in 0..12 {
                        let t = a as f32 / 12.0 * std::f32::consts::TAU;
                        let v = Vec3::new(t.cos() * phi.cos(), sign * phi.sin(), t.sin() * phi.cos()) * radius;
                        out.push(rot * (v + Vec3::Y * sign * half));
                    }
                }
            }
        }
        // A box's faces within their edges: a box's edges and corners stand out of any round mesh (a hand's, a
        // foot's) however well it lies.
        Shape::Box { rot, half } => {
            for axis in 0..3 {
                for sign in [-1.0f32, 1.0] {
                    for u in 0..5 {
                        for v in 0..5 {
                            let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
                            let mut p = Vec3::ZERO;
                            p[axis] = sign * half[axis];
                            p[a] = (u as f32 / 4.0 * 2.0 - 1.0) * half[a] * 0.6;
                            p[b] = (v as f32 / 4.0 * 2.0 - 1.0) * half[b] * 0.6;
                            out.push(rot * p);
                        }
                    }
                }
            }
        }
    }
    out
}

/// Whether `p` (the shape's frame, centred) is inside the shape.
fn inside(s: &Shape, p: Vec3) -> bool {
    match *s {
        Shape::Capsule { rot, radius, half } => {
            let q = rot.inverse() * p;
            q.distance(Vec3::new(0.0, q.y.clamp(-half, half), 0.0)) <= radius
        }
        Shape::Box { rot, half } => {
            let q = (rot.inverse() * p).abs();
            q.x <= half.x && q.y <= half.y && q.z <= half.z
        }
    }
}

/// The shape's long axis (its frame): a capsule's Y, a box's longest.
fn long_axis(s: &Shape) -> Vec3 {
    match *s {
        Shape::Capsule { rot, .. } => rot * Vec3::Y,
        Shape::Box { rot, half } => rot * Vec3::AXES[(0..3).max_by(|&a, &b| half[a].total_cmp(&half[b])).unwrap_or(1)],
    }
}

/// The parts' scores on a fitted body (`f`, its edits applied) against the model's mesh, and the pairs' faults.
pub fn score(skel: &Skeleton, mesh: &Mesh, map: &[Option<String>; 21], f: &Fit, plain: &Fit) -> (Vec<Score>, Vec<String>) {
    let j = &f.shown_joints;
    let (pos, nor, part) = parts_of(skel, mesh, map, j);
    let world = |k: usize| (f.shown[k].translation, f.shown[k].rotation);
    let scores: Vec<Score> = (0..21)
        .map(|k| {
            let shape = f.body.parts[k].shape;
            let (c, r) = world(k);
            let thick = thickness(&shape);
            let cloud: Vec<Vec3> = pos.iter().zip(&part).filter(|x| *x.1 == Some(k)).map(|x| *x.0).collect();
            let mut s = Score {
                thick,
                verts: cloud.len(),
                homer: thick / thickness(&plain.body.parts[k].shape).max(1e-4),
                out_bound: (thick * OUT.0).max(OUT.1),
                ..Score::default()
            };
            // How far the surface stands out: each sample against its nearest vertex's side.
            let mut out: Vec<f32> = surface(&shape, !LONG.iter().any(|x| x.0 == k))
                .into_iter()
                .map(|p| {
                    let w = c + r * p;
                    let (i, _) = pos.iter().enumerate().map(|(i, v)| (i, v.distance_squared(w))).min_by(|a, b| a.1.total_cmp(&b.1)).unwrap_or((0, 0.0));
                    pos.get(i).map_or(0.0, |v| (w - *v).dot(nor[i]).max(0.0))
                })
                .collect();
            s.out = quantile(&mut out, 0.8);
            // A capsule only: a box is sized to its mesh's extents and holds most of it however well it lies.
            if cloud.len() >= 20 && matches!(shape, Shape::Capsule { .. }) {
                s.inside = Some(cloud.iter().filter(|p| inside(&shape, r.inverse() * (**p - c))).count() as f32 / cloud.len() as f32);
            }
            let axis = r * long_axis(&shape);
            // The head's face to the model's (its hips' line across the up).
            if k == 12 {
                let forward = (j[1] - j[4]).with_z(0.0).normalize_or_zero().cross(Vec3::Z);
                s.angle = Some(snap::face(f).angle_between(forward).to_degrees());
            }
            if let Some(&(_, n)) = LONG.iter().find(|x| x.0 == k) {
                let (a, d) = (j[k], (j[n] - j[k]).normalize_or_zero());
                let len = j[k].distance(j[n]);
                s.angle = Some(axis.dot(d).abs().clamp(0.0, 1.0).acos().to_degrees());
                s.off_bone = Some(((c - a) - d * (c - a).dot(d)).length());
                // The cross-section in the bone's middle: its centre (the median) and its radius (the median).
                let off: Vec<Vec3> = cloud
                    .iter()
                    .filter_map(|p| {
                        let t = (*p - a).dot(d);
                        (t > len * 0.2 && t < len * 0.8).then(|| *p - a - d * t)
                    })
                    .collect();
                // The clavicles' vertices are the shoulder's top and the chest's: no ring round their bones.
                if let Some(ring) = snap::ring(&off, d).filter(|_| k != 13 && k != 17) {
                    s.ratio = Some(thick / ring.radius.max(1e-3));
                    let cc = (c - a) - d * (c - a).dot(d);
                    s.off_mesh = ring.round.then(|| cc.distance(ring.centre) / thick.max(1e-3));
                }
            }
            if LEAVES.contains(&k) && cloud.len() >= 8 {
                let foot = k == 3 || k == 6;
                let level = |v: Vec3| if foot { v.with_z(0.0) } else { v };
                let mid = Vec3::from_array([0, 1, 2].map(|i| quantile(&mut cloud.iter().map(|o| o[i]).collect::<Vec<_>>(), 0.5)));
                // A hand's way: from its wrist to its mesh's middle; a foot's: its mesh's long axis on the floor (a
                // boot's top round the ankle puts its middle near the joint).
                let way = if foot {
                    let sole: Vec<Vec3> = cloud.iter().copied().filter(|p| p.z < j[k].z).collect();
                    let sole = if sole.len() >= 12 { sole } else { cloud.clone() };
                    let m = Vec3::from_array([0, 1, 2].map(|i| quantile(&mut sole.iter().map(|o| o[i]).collect::<Vec<_>>(), 0.5)));
                    snap::principal(&sole.iter().map(|p| level(*p - m)).collect::<Vec<_>>()).map_or(Vec3::ZERO, |a| level(a.0).normalize_or_zero())
                } else {
                    (mid - j[k]).normalize_or_zero()
                };
                if let Shape::Box { rot, half } = shape {
                    let fr = r * rot;
                    // A hand's box is nearly as wide as it's long: its way is whichever of its two longer axes lies
                    // nearer the mesh's (its extents across are checked below).
                    let mut order = [0, 1, 2];
                    order.sort_by(|&a, &b| half[b].total_cmp(&half[a]));
                    let (along, ang) = order[..2]
                        .iter()
                        .map(|&i| (i, level(fr * Vec3::AXES[i]).normalize_or_zero()))
                        .filter(|x| x.1 != Vec3::ZERO && way != Vec3::ZERO)
                        .map(|(i, a)| (i, a.dot(way).abs().clamp(0.0, 1.0).acos().to_degrees()))
                        .min_by(|a, b| a.1.total_cmp(&b.1))
                        .unwrap_or((order[0], 0.0));
                    s.angle = Some(ang);
                    // Across: the shape's half to its vertices' (5–95%) on each of its other axes, and its centre off
                    // the middle of their span to its half there (a foot's bottom off its sole, up and down).
                    let (mut ratio, mut off) = (0.0f32, 0.0f32);
                    for i in (0..3).filter(|&i| i != along) {
                        let ax = fr * Vec3::AXES[i];
                        let mut v: Vec<f32> = cloud.iter().map(|p| ax.dot(*p - c)).collect();
                        let (lo, hi) = (quantile(&mut v, 0.05), quantile(&mut v, 0.95));
                        ratio = ratio.max(half[i] / ((hi - lo) * 0.5).max(1e-3));
                        let gap = match (foot, ax.z) {
                            (true, z) if z > 0.7 => lo + half[i],
                            (true, z) if z < -0.7 => hi - half[i],
                            _ => (lo + hi) * 0.5,
                        };
                        off = off.max(gap.abs() / half[i].max(1e-3));
                    }
                    s.ratio = Some(ratio);
                    s.off_mesh = Some(off);
                    s.out_bound = (half[order[1]] * OUT_BOX).max(OUT.1);
                }
            }
            s
        })
        .collect();
    // The pairs against their mirror images across the hips' plane, each from its own joint (the model's bones may
    // not be symmetric themselves).
    let mut faults = vec![];
    let n = Vec3::new(j[1].x - j[4].x, j[1].y - j[4].y, 0.0).normalize_or_zero();
    let flip = |v: Vec3| v - 2.0 * n * n.dot(v);
    for (a, b) in PAIRS {
        let (ta, tb) = (scores[a].thick, scores[b].thick);
        let dt = (ta - tb).abs() / ta.max(tb).max(1e-4);
        let gap = flip(world(a).0 - j[a]).distance(world(b).0 - j[b]);
        let (ax, bx) = (flip(world(a).1 * long_axis(&f.body.parts[a].shape)), world(b).1 * long_axis(&f.body.parts[b].shape));
        let ang = ax.dot(bx).abs().clamp(0.0, 1.0).acos().to_degrees();
        let side = PARTS[a].trim_end_matches("_Left");
        if dt > SYM.0 {
            faults.push(format!("{side} sides' thickness {:.0}% apart", dt * 100.0));
        }
        if gap > SYM.1 {
            faults.push(format!("{side} sides {:.1} cm off each other's mirror", gap * 100.0));
        }
        if ang > SYM.2 {
            faults.push(format!("{side} sides' axes {ang:.0}° off each other's mirror"));
        }
    }
    for (k, s) in scores.iter().enumerate() {
        faults.extend(faults_of(k, s).into_iter().map(|f| format!("{} {f}", PARTS[k])));
    }
    (scores, faults)
}

/// A part's scores past their bounds.
fn faults_of(k: usize, s: &Score) -> Vec<String> {
    let mut out = vec![];
    if let Some(a) = s.angle.filter(|a| *a > if k == 12 { FACE } else { ANGLE }) {
        out.push(if k == 12 { format!("looks {a:.0}° off the model's way") } else { format!("axis {a:.0}° off") });
    }
    if let Some(r) = s.ratio.filter(|r| *r < RATIO.0 || *r > RATIO.1) {
        out.push(format!("×{r:.2} its mesh's thickness"));
    }
    if s.out > s.out_bound {
        out.push(format!("stands {:.1} cm out of the mesh", s.out * 100.0));
    }
    // The head and the pelvis are round in a round mesh; the rest is checked for swallowing its vertices.
    if let Some(i) = s.inside.filter(|i| *i > INSIDE && k != 0) {
        out.push(format!("swallows {:.0}% of its vertices", i * 100.0));
    }
    if let Some(o) = s.off_mesh.filter(|o| *o > OFF_MESH) {
        out.push(format!("{o:.1}× its thickness off its mesh's middle"));
    }
    if let Some(o) = s.off_bone.filter(|o| *o > (s.thick * OFF_BONE.0).max(OFF_BONE.1)) {
        out.push(format!("{:.1} cm off its bone", o * 100.0));
    }
    if s.homer < HOMER.0 || s.homer > HOMER.1 {
        out.push(format!("×{:.2} Homer's thickness", s.homer));
    }
    out
}

/// After a drop on the fork: the fastest part (m/s) and the farthest from the body's middle (m) bounds, past which
/// the body hasn't settled or has come apart.
const SETTLED: f32 = 0.5;
const APART: f32 = 1.6;

/// The body dropped on the fork (`sim`) as the editor drops it and run `steps`: its faults (not settled, apart, not
/// finite) and how it lay.
fn drop_test(sim: &mut Sim, body: &dms_game::body::Body, steps: u64) -> Result<(Vec<String>, String), String> {
    sim.place(body, 1.0, 0.4, Vec3::ZERO, Vec3::ZERO)?;
    while sim.steps < steps {
        sim.run(sim::TIMESTEP);
    }
    let states = sim.states().ok_or("no body after the drop")?;
    sim.clear();
    if states.iter().any(|s| !(s.pos.is_finite() && s.vel.is_finite())) {
        return Ok((vec!["the body's state isn't finite after the drop".into()], String::new()));
    }
    let mid = states.iter().map(|s| s.pos).sum::<Vec3>() / states.len() as f32;
    let fast = states.iter().map(|s| s.vel.length()).fold(0.0f32, f32::max);
    let far = states.iter().map(|s| s.pos.distance(mid)).fold(0.0f32, f32::max);
    let mut out = vec![];
    if fast > SETTLED {
        out.push(format!("the body hasn't settled after the drop: a part at {fast:.2} m/s"));
    }
    if far > APART {
        out.push(format!("the body has come apart after the drop: a part {far:.2} m from its middle"));
    }
    Ok((out, format!("after {steps} steps: the fastest part at {fast:.3} m/s, the farthest {far:.2} m from the middle")))
}

fn fit(homer: &Homer, skel: &Skeleton, path: &str, r: &Rig) -> Result<Fit, String> {
    let (names, edits) = r.args();
    dms_fit::fit::fit(&homer.build, &homer.body, skel, path, names, edits)
}

/// Prints the scores of every complete humanoid model whose path has one of `filters` (comma-separated): snapped, or
/// the plain fit (`plain`); every part's with `verbose`; each body dropped on the fork (`sim`) for that many steps.
/// Returns the models with faults.
pub fn run(homer: &Homer, fs: &GameFs, filters: &str, plain: bool, verbose: bool, mut sim: Option<(Sim, u64)>) -> usize {
    let filters: Vec<&str> = filters.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
    let mut models: Vec<String> = fs.models().into_iter().map(|m| m.0).filter(|p| filters.iter().any(|f| p.contains(f))).collect();
    models.sort();
    models.dedup();
    let (mut bad, mut done) = (0, 0);
    let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
    for path in &models {
        let r = (|| -> Result<(Vec<Score>, Vec<String>, String), String> {
            let bytes = fs.read(path)?;
            let skel = read_mdl(&bytes)?;
            let base = path.strip_suffix(".mdl").unwrap_or(path);
            let mesh = mesh::read(&bytes, &fs.read(&format!("{base}.vvd"))?, &fs.read(&format!("{base}.dx90.vtx"))?)?;
            if mesh.tris.is_empty() {
                return Err("no triangles".into());
            }
            let at: Vec<Vec3> = skel.bones.iter().map(|b| b.bind.0).collect();
            let c = rigs::classify(&skel, None, &at)?;
            if !c.complete() {
                return Err(c.why(path));
            }
            let map: [Option<String>; 21] = std::array::from_fn(|k| c.map[k].map(|b| skel.bones[b].name.clone()));
            let rig = Rig { model: Some(path.clone()), checksum: Some(skel.checksum), ..Rig::default() };
            let p = fit(homer, &skel, path, &rig)?;
            let (f, status) = if plain {
                (fit(homer, &skel, path, &rig)?, String::from("plain fit"))
            } else {
                let phy = fs.read(&format!("{base}.phy")).ok();
                let s = snap::snap_rig(homer, &skel, &mesh, phy.as_deref(), path, &rig, &map, &(0..21).collect::<Vec<_>>())?;
                let notes = if verbose && s.notes.len() > 1 { format!("\n    notes: {}", s.notes.join("; ")) } else { String::new() };
                (fit(homer, &skel, path, &s.rig)?, s.status + &notes)
            };
            let (scores, mut faults) = score(&skel, &mesh, &map, &f, &p);
            let mut status = status;
            if let Some((s, steps)) = &mut sim {
                let (more, how) = drop_test(s, &f.body, *steps)?;
                faults.extend(more);
                if verbose {
                    status += &format!("\n    drop: {how}");
                }
            }
            Ok((scores, faults, status))
        })();
        match r {
            Ok((scores, faults, status)) => {
                done += 1;
                if !faults.is_empty() {
                    bad += 1;
                }
                for f in &faults {
                    let kind = f.split_whitespace().skip(1).filter(|w| !w.chars().any(|c| c.is_ascii_digit())).collect::<Vec<_>>().join(" ");
                    *kinds.entry(kind).or_default() += 1;
                }
                println!("{path}\t{} fault(s)\t{status}", faults.len());
                for f in &faults {
                    println!("    {f}");
                }
                if verbose {
                    println!("    part              verts  thick  ×homer  ×mesh  angle  out cm  inside  off-mesh  off-bone cm");
                    let o = |v: Option<f32>, m: f32, w: usize| v.map_or(format!("{:>w$}", "-"), |v| format!("{:>w$.2}", v * m));
                    for (k, s) in scores.iter().enumerate() {
                        println!(
                            "    {:<16} {:>6} {:>6.3} {:>7.2} {} {} {:>7.1} {} {} {}",
                            PARTS[k],
                            s.verts,
                            s.thick,
                            s.homer,
                            o(s.ratio, 1.0, 6),
                            o(s.angle, 1.0, 6),
                            s.out * 100.0,
                            o(s.inside, 1.0, 7),
                            o(s.off_mesh, 1.0, 9),
                            o(s.off_bone, 100.0, 11)
                        );
                    }
                }
            }
            Err(e) if verbose => println!("{path}\tskipped: {e}"),
            Err(_) => {}
        }
    }
    println!("{done} model(s) scored, {bad} with faults");
    for (k, n) in &kinds {
        println!("    {n:>4}  {k}");
    }
    bad
}
