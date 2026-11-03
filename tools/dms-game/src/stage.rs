//! The Playroom's stages (`delirium-room` `world::stage`, `--stage <name>`, v0.2.2): the boxes each puts on the room's
//! floor (no pool) and where Homer starts. The same numbers, for the stages map (`tools/dms-map`) and the bench
//! (ROADMAP M5). Only the stages of fixed boxes are here (no bridge, platforms, cab or street traffic).

/// What a box is made of: materials.dat's BRICK_WALL or CONCRETE row, as the Playroom picks them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Block {
    Brick,
    Concrete,
}

/// A stage: its boxes (centre, half sizes, m), Homer's start (his lowest point there, the pelvis above it) and whether
/// its floor has the room's pool ([`crate::POOL`], [`POOL_DEEP`] deep, water up to [`WATER`]).
pub struct Stage {
    pub name: &'static str,
    pub boxes: Vec<([f32; 3], [f32; 3], Block)>,
    pub start: [f32; 3],
    pub pool: bool,
}

/// The stages there are; `pool` is the Playroom's own room (`world::Room::blocks`, the build with its default
/// `portal` feature), which the Playroom gives for any `--stage` that isn't one of its stages (ROADMAP M5: water).
pub const NAMES: [&str; 9] = ["bigstairs", "stairsdown", "rooftop", "highledge", "lowwall", "kerb", "rail", "open", "pool"];

/// The room's pool: its depth under the floor and its water's surface (`world::POOL_DEEP`, `world::WATER`, m).
pub const POOL_DEEP: f32 = 1.5;
pub const WATER: f32 = -0.1;

/// The Playroom room's boxes (`world::Room::blocks(.., false)` with `portal`): a low brick wall, steps and a block,
/// the pool's walls, bottom and steps down, the portal mode's wall and platform.
fn room() -> Vec<([f32; 3], [f32; 3], Block)> {
    use Block::{Brick, Concrete};
    let mut v = vec![([0.0, 3.5, 0.5], [4.0, 0.15, 0.5], Brick)];
    for i in 0..10 {
        let top = 0.18 * (i + 1) as f32;
        v.push(([-3.75, -1.5 + 0.3 * i as f32 + 0.15, top * 0.5], [0.75, 0.15, top * 0.5], Concrete));
    }
    v.push(([-3.75, 2.0, 0.9], [0.75, 0.5, 0.9], Concrete));
    let ([a, b], d, rim) = (crate::POOL, POOL_DEEP, 0.3);
    let (m, hw, z) = ([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5], [(b[0] - a[0]) * 0.5, (b[1] - a[1]) * 0.5], -(d + 0.2) * 0.5 - 0.001);
    for (c, h) in [
        ([a[0] - rim * 0.5, m[1]], [rim * 0.5, hw[1] + rim]),
        ([b[0] + rim * 0.5, m[1]], [rim * 0.5, hw[1] + rim]),
        ([m[0], a[1] - rim * 0.5], [hw[0], rim * 0.5]),
        ([m[0], b[1] + rim * 0.5], [hw[0], rim * 0.5]),
    ] {
        v.push(([c[0], c[1], z], [h[0], h[1], z.abs() - 0.001], Concrete));
    }
    v.push(([m[0], m[1], -d - 0.1], [hw[0], hw[1], 0.1], Concrete));
    for i in 0..8 {
        let top = -0.18 * (i + 1) as f32;
        v.push(([b[0] - 0.3 * i as f32 - 0.15, a[1] + 0.6, (top - d) * 0.5], [0.15, 0.6, (top + d) * 0.5], Concrete));
    }
    v.extend([([10.0, 0.0, 3.0], [0.15, 4.0, 3.0], Concrete), ([6.0, 3.0, 5.0], [3.0, 3.0, 0.1], Concrete)]);
    v
}

/// A stage by name, as `world::stage` has it.
pub fn stage(name: &str) -> Option<Stage> {
    use Block::{Brick, Concrete};
    let b = |c: [f32; 3], h: [f32; 3], r| (c, h, r);
    let (boxes, start) = match name {
        "bigstairs" => {
            let mut v = vec![b([0.0, -0.45, 3.0], [1.5, 1.05, 3.0], Concrete)];
            v.extend([-1.6, 1.6].map(|x| b([x, -0.45, 3.5], [0.1, 1.05, 3.5], Brick)));
            for i in 0..39 {
                let (top, y) = (6.0 - 0.15 * (i + 1) as f32, 0.75 + 0.3 * i as f32);
                v.push(b([0.0, y, top * 0.5], [1.5, 0.15, top * 0.5], Concrete));
                v.extend([-1.6, 1.6].map(|x| b([x, y, (top + 1.0) * 0.5], [0.1, 0.15, (top + 1.0) * 0.5], Brick)));
            }
            (v, [0.0, 0.0, 6.0])
        }
        "stairsdown" => {
            let s = stage("bigstairs")?;
            (s.boxes.into_iter().map(|(c, h, r)| ([c[0], -c[1], c[2]], h, r)).collect(), s.start)
        }
        "rooftop" => (vec![b([0.0, -2.25, 2.0], [3.0, 2.75, 2.0], Concrete)], [0.0, 0.0, 4.0]),
        "highledge" => (vec![b([0.0, 4.5, 6.0], [5.0, 5.0, 6.0], Concrete)], [0.0, 0.0, 12.0]),
        "lowwall" => (vec![b([0.0, 0.45, 0.45], [4.0, 0.15, 0.45], Brick)], [0.0; 3]),
        "kerb" => (vec![b([0.0, 0.45, 0.22], [4.0, 0.15, 0.22], Brick)], [0.0; 3]),
        "rail" => (
            vec![b([0.0, -0.5, 1.18], [1.5, 0.03, 0.03], Concrete), b([-1.5, -0.5, 0.6], [0.03, 0.03, 0.6], Concrete), b([1.5, -0.5, 0.6], [0.03, 0.03, 0.6], Concrete)],
            [0.0; 3],
        ),
        "open" => (vec![], [0.0; 3]),
        "pool" => (room(), [0.0; 3]),
        _ => return None,
    };
    Some(Stage { name: NAMES.iter().copied().find(|n| *n == name)?, boxes, start, pool: name == "pool" })
}
