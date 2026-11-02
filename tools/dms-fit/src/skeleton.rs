//! A model's skeleton read from its `.mdl` (ROADMAP M9, docs/adr/0025): bones, hierarchy, bind pose.

use delirium::prelude::*;

/// `mstudiobone_t`'s size (studio versions 44 to 49).
const BONE_SIZE: usize = 216;

pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    /// Its frame in the model's space (units): the chain of its default transforms (`pos`, `quat` in its parent's),
    /// and the inverse of the `.mdl`'s `poseToBone`.
    pub bind: (Vec3, Quat),
    pub pose: (Vec3, Quat),
    /// Its physics object's index (`physicsbone`).
    pub physics: i32,
}

pub struct Skeleton {
    pub name: String,
    pub version: i32,
    pub checksum: i32,
    pub bones: Vec<Bone>,
}

impl Skeleton {
    pub fn find(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b.name.eq_ignore_ascii_case(name))
    }

    /// Whether a bone has a physics object of its own: `physicsbone` is the object that moves it, its own or (for a
    /// bone without one) its nearest ancestor's, so it's its own where its parent's differs.
    pub fn has_object(&self, b: usize) -> bool {
        let p = self.bones[b].physics;
        p >= 0 && self.bones[b].parent.is_none_or(|q| self.bones[q].physics != p)
    }
}

pub fn i32_at(b: &[u8], at: usize) -> Result<i32, String> {
    b.get(at..at + 4).map(|s| i32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or_else(|| format!("truncated at {at}"))
}

pub fn f32_at(b: &[u8], at: usize) -> Result<f32, String> {
    i32_at(b, at).map(|i| f32::from_bits(i as u32))
}

pub fn str_at(b: &[u8], at: usize) -> Result<String, String> {
    let s = b.get(at..).ok_or_else(|| format!("a name past the end ({at})"))?;
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len()).min(256);
    Ok(String::from_utf8_lossy(&s[..end]).into_owned())
}

/// The bones of a `.mdl` (`studiohdr_t`, versions 44 to 49: GMod's), their hierarchy and bind pose.
pub fn read_mdl(b: &[u8]) -> Result<Skeleton, String> {
    if b.len() < 408 || &b[..4] != b"IDST" {
        return Err("not a .mdl (no IDST)".into());
    }
    let version = i32_at(b, 4)?;
    if !(44..=49).contains(&version) {
        return Err(format!("studio version {version} (44 to 49 read)"));
    }
    let (count, index) = (i32_at(b, 156)?, i32_at(b, 160)?);
    if !(1..=256).contains(&count) || index < 0 || index as usize + count as usize * BONE_SIZE > b.len() {
        return Err(format!("{count} bones at {index}: not in the file"));
    }
    let mut bones: Vec<Bone> = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let o = index as usize + i * BONE_SIZE;
        let v = |at: usize| -> Result<Vec3, String> { Ok(Vec3::new(f32_at(b, o + at)?, f32_at(b, o + at + 4)?, f32_at(b, o + at + 8)?)) };
        let name = str_at(b, (o as i64 + i64::from(i32_at(b, o)?)) as usize)?;
        let parent = i32_at(b, o + 4)?;
        let parent = (parent >= 0).then_some(parent as usize).filter(|&p| p < i);
        let pos = v(32)?;
        let rot = Quat::from_xyzw(f32_at(b, o + 44)?, f32_at(b, o + 48)?, f32_at(b, o + 52)?, f32_at(b, o + 56)?).normalize();
        // poseToBone: rows of [R | t], the model's space into the bone's.
        let row = |r: usize| v(96 + r * 16);
        let (r0, r1, r2) = (row(0)?, row(1)?, row(2)?);
        let t = Vec3::new(f32_at(b, o + 96 + 12)?, f32_at(b, o + 96 + 28)?, f32_at(b, o + 96 + 44)?);
        let inv = Mat3::from_cols(r0, r1, r2);
        let pose = (-(inv * t), Quat::from_mat3(&inv).normalize());
        let bind = match parent {
            Some(p) => {
                let (pp, pq) = bones[p].bind;
                (pp + pq * pos, (pq * rot).normalize())
            }
            None => (pos, rot),
        };
        bones.push(Bone { name, parent, bind, pose, physics: i32_at(b, o + 172)? });
    }
    Ok(Skeleton { name: str_at(b, 12)?, version, checksum: i32_at(b, 8)?, bones })
}
