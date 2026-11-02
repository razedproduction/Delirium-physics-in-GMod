//! A model's mesh as the game draws it (docs/adr/0031): the `.mdl`'s body parts (the first model of each, LOD 0),
//! the `.vvd`'s vertices in the mesh's bind pose (with their bone weights) and the `.dx90.vtx`'s triangles.

use dms_fit::skeleton::{f32_at, i32_at};

/// The mesh in the model's space (units), as the `.vvd` has it: the bind pose `poseToBone` skins from.
#[derive(Default)]
pub struct Mesh {
    pub pos: Vec<[f32; 3]>,
    pub normal: Vec<[f32; 3]>,
    /// Up to three bones and their weights a vertex.
    pub bones: Vec<[(u8, f32); 3]>,
    pub tris: Vec<[u32; 3]>,
}

fn u8_at(b: &[u8], at: usize) -> Result<u8, String> {
    b.get(at).copied().ok_or_else(|| format!("truncated at {at}"))
}

fn u16_at(b: &[u8], at: usize) -> Result<u16, String> {
    b.get(at..at + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or_else(|| format!("truncated at {at}"))
}

/// `.vvd`'s LOD 0 vertices: (position, normal, bones), the fixups applied.
fn read_vvd(b: &[u8], checksum: i32) -> Result<Vec<([f32; 3], [f32; 3], [(u8, f32); 3])>, String> {
    if b.get(..4) != Some(b"IDSV") {
        return Err(".vvd: no IDSV".into());
    }
    if i32_at(b, 8)? != checksum {
        return Err(".vvd: its checksum isn't the .mdl's".into());
    }
    let lod0 = i32_at(b, 16)? as usize;
    let (fixups, fixup_at, data) = (i32_at(b, 48)? as usize, i32_at(b, 52)? as usize, i32_at(b, 56)? as usize);
    let vertex = |i: usize| -> Result<([f32; 3], [f32; 3], [(u8, f32); 3]), String> {
        let o = data + i * 48;
        let n = u8_at(b, o + 15)?.min(3) as usize;
        let mut bones = [(0u8, 0.0f32); 3];
        for (k, w) in bones.iter_mut().enumerate().take(n) {
            *w = (u8_at(b, o + 12 + k)?, f32_at(b, o + k * 4)?);
        }
        let v = |at: usize| -> Result<[f32; 3], String> { Ok([f32_at(b, o + at)?, f32_at(b, o + at + 4)?, f32_at(b, o + at + 8)?]) };
        Ok((v(16)?, v(28)?, bones))
    };
    let mut out = Vec::with_capacity(lod0);
    if fixups == 0 {
        for i in 0..lod0 {
            out.push(vertex(i)?);
        }
    } else {
        for f in 0..fixups {
            let o = fixup_at + f * 12;
            let (lod, src, n) = (i32_at(b, o)?, i32_at(b, o + 4)? as usize, i32_at(b, o + 8)? as usize);
            if lod >= 0 {
                for i in src..src + n {
                    out.push(vertex(i)?);
                }
            }
        }
    }
    Ok(out)
}

/// One `.vtx` strip group's layout: its header's size (25 bytes, 33 with topology) and its vertices' (9).
const VTX_VERTEX: usize = 9;

/// The `.mdl`'s mesh (`mdl`), its `.vvd` and `.vtx` (`vvd`, `vtx`).
pub fn read(mdl: &[u8], vvd: &[u8], vtx: &[u8]) -> Result<Mesh, String> {
    let checksum = i32_at(mdl, 8)?;
    let verts = read_vvd(vvd, checksum)?;
    if i32_at(vtx, 0)? != 7 {
        return Err(format!(".vtx version {} (7 read)", i32_at(vtx, 0)?));
    }
    if i32_at(vtx, 16)? != checksum {
        return Err(".vtx: its checksum isn't the .mdl's".into());
    }
    let mut mesh = Mesh::default();
    for (p, n, w) in &verts {
        mesh.pos.push(*p);
        mesh.normal.push(*n);
        mesh.bones.push(*w);
    }
    let (parts, part_at) = (i32_at(mdl, 232)? as usize, i32_at(mdl, 236)? as usize);
    let (vtx_parts, vtx_part_at) = (i32_at(vtx, 28)? as usize, i32_at(vtx, 32)? as usize);
    if parts != vtx_parts {
        return Err(format!(".mdl has {parts} body parts, .vtx {vtx_parts}"));
    }
    for bp in 0..parts {
        // mstudiobodyparts_t (16 bytes): name, nummodels, base, modelindex.
        let mb = part_at + bp * 16;
        let (models, model_at) = (i32_at(mdl, mb + 4)?, i32_at(mdl, mb + 12)?);
        if models < 1 {
            continue;
        }
        // The first model (the default body group); mstudiomodel_t: nummeshes at 72, meshindex 76, vertexindex 84.
        let mm = (mb as i64 + i64::from(model_at)) as usize;
        let (meshes, mesh_at, first) = (i32_at(mdl, mm + 72)? as usize, i32_at(mdl, mm + 76)?, i32_at(mdl, mm + 84)? as usize / 48);
        let vb = vtx_part_at + bp * 8;
        let vm = vb + i32_at(vtx, vb + 4)? as usize;
        let vl = vm + i32_at(vtx, vm + 4)? as usize;
        let (vtx_meshes, vtx_mesh_at) = (i32_at(vtx, vl)? as usize, vl + i32_at(vtx, vl + 4)? as usize);
        for m in 0..meshes.min(vtx_meshes) {
            // mstudiomesh_t (116 bytes): vertexoffset at 12.
            let me = (mm as i64 + i64::from(mesh_at)) as usize + m * 116;
            let base = first + i32_at(mdl, me + 12)? as usize;
            // MeshHeader_t (9 bytes): numStripGroups, stripGroupHeaderOffset, flags.
            let vme = vtx_mesh_at + m * 9;
            let (groups, group_at) = (i32_at(vtx, vme)? as usize, vme + i32_at(vtx, vme + 4)? as usize);
            let mut size = 25;
            for g in 0..groups {
                let sg = group_at + g * size;
                let tris = match strip_group(vtx, sg, base, verts.len()) {
                    Ok(t) => t,
                    // A newer studiomdl's header (topology): try it once.
                    Err(_) if g == 0 && size == 25 => {
                        size = 33;
                        strip_group(vtx, sg, base, verts.len())?
                    }
                    Err(e) => return Err(e),
                };
                mesh.tris.extend(tris);
            }
        }
    }
    Ok(mesh)
}

/// A strip group's triangles as `.vvd` indices (its indices are a triangle list into its vertices).
fn strip_group(vtx: &[u8], sg: usize, base: usize, count: usize) -> Result<Vec<[u32; 3]>, String> {
    let (nv, vo, ni, io) = (i32_at(vtx, sg)?, i32_at(vtx, sg + 4)?, i32_at(vtx, sg + 8)?, i32_at(vtx, sg + 12)?);
    if nv < 0 || ni < 0 || ni % 3 != 0 || sg + vo as usize + nv as usize * VTX_VERTEX > vtx.len() || sg + io as usize + ni as usize * 2 > vtx.len() {
        return Err(".vtx: a strip group out of the file".into());
    }
    let vert = |i: usize| -> Result<u32, String> {
        let id = u16_at(vtx, sg + vo as usize + i * VTX_VERTEX + 4)? as usize;
        let v = base + id;
        if v >= count { Err(format!(".vtx: vertex {v} of {count}")) } else { Ok(v as u32) }
    };
    let mut out = Vec::with_capacity(ni as usize / 3);
    for t in 0..ni as usize / 3 {
        let idx = |k: usize| -> Result<u32, String> {
            let i = u16_at(vtx, sg + io as usize + (t * 3 + k) * 2)? as usize;
            if i >= nv as usize { Err(".vtx: an index past its vertices".into()) } else { vert(i) }
        };
        out.push([idx(0)?, idx(1)?, idx(2)?]);
    }
    Ok(out)
}
