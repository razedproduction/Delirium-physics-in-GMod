//! Binding Homer's ragdoll (`models/dms/homer.mdl`, `tools/dms-homer`) to NM's body (ROADMAP M3): in the pose
//! it's spawned in, every joint gets NM's pivot (`SetJointAnchor`) and NM's frame and limits (`SetJointLimits`).
//!
//! The ragdoll's physics objects are NM's parts one to one (bones named as `nm_part_name`), each object's frame
//! the part's frame with its centre of mass on the origin, so a body's state is the part's state as it is.

use std::ffi::{CStr, c_int};

use delirium::prelude::*;

use crate::data;
use crate::ffi::vphysics_nm::{self, IPhysicsObject, Version};
use crate::lua::{self, Api, LuaState, TTABLE, native};

pub const NATIVES: &[(&CStr, lua::CFunction)] = &[(c"BindHomer", bind_homer), (c"HomerBody", homer_body)];

/// The 21 PhysObj of the table at `arg`, `[i + 1]` part `i`'s.
unsafe fn parts(api: &Api, l: *mut LuaState, arg: c_int, count: usize) -> Result<Vec<*mut IPhysicsObject>, String> {
    if unsafe { (api.type_)(l, arg) } != TTABLE {
        return Err(format!("bad argument #{arg} (a table of {count} PhysObj in NM's part order expected)"));
    }
    (0..count)
        .map(|i| unsafe {
            (api.rawgeti)(l, arg, i as c_int + 1);
            let p = api.check_physobj(l, -1).map_err(|_| format!("bad argument #{arg} ([{}] isn't a PhysObj)", i + 1));
            (api.settop)(l, -2);
            p
        })
        .collect()
}

native!(bind_homer => bind_homer_impl);
/// `DMS.BindHomer(parts)`: `parts` the ragdoll's 21 PhysObj in NM's part order, standing as it was spawned. Gives
/// every body NM's inertia tensor (`SetBodyInertia`) and every joint NM's pivot, frames and limits (a ball
/// `SetJointBallLimits`, a hinge `SetJointLimits`); `{joint = {part, parent, kind, anchor, limits}, ...}` per
/// joint, or a Lua error naming the part or joint that failed.
unsafe fn bind_homer_impl(api: &Api, l: *mut LuaState) -> Result<c_int, String> {
    let nm = vphysics_nm::get().ok_or("IPhysicsNM isn't available (vphysics.dll isn't VPhysics-Box3D)")?;
    if nm.version < Version::V004 {
        return Err(format!("{} has no SetBodyMaterial (or SetBodyInertia / SetJointBallLimits): update VPhysics-Box3D", nm.version.name()));
    }
    let d = data::get().ok_or_else(|| format!("NM's data isn't loaded (DMS.LoadData): {}", data::how_to()))?;
    let objects = unsafe { parts(api, l, 1, d.body.parts.len())? };
    unsafe { bind_body(api, l, &d.body, &objects) }
}

/// Binds `objects` (a ragdoll's bodies in NM's part order, standing as spawned) to `body` (`dms_vphysics::body::bind`,
/// shared with the rig editor). Pushes the joints' table as `DMS.BindHomer` does.
///
/// # Safety
/// `l` is a live Lua state, called from its own thread; the objects are live.
pub unsafe fn bind_body(api: &Api, l: *mut LuaState, body: &dms_game::body::Body, objects: &[*mut IPhysicsObject]) -> Result<c_int, String> {
    let nm = vphysics_nm::get().ok_or("IPhysicsNM isn't available (vphysics.dll isn't VPhysics-Box3D)")?;
    // SAFETY: the caller's: live objects, on the game's main thread outside the step.
    let bound = unsafe { dms_vphysics::body::bind(&nm, body, objects)? };
    unsafe { (api.createtable)(l, bound.len() as c_int, 0) };
    for (k, j) in bound.iter().enumerate() {
        unsafe {
            (api.createtable)(l, 0, 5);
            api.set_number(l, c"part", j.part as f64);
            api.set_number(l, c"parent", j.parent as f64);
            api.set_str(l, c"kind", if j.hinge { "hinge" } else { "ball" });
            api.set_vector(l, c"anchor", &j.anchor.to_array());
            api.set_vector(l, c"limits", if j.hinge { &j.limits[..2] } else { &j.limits[..] });
            api.append(l, k + 1);
        }
    }
    Ok(1)
}
native!(homer_body => homer_body_impl);
/// `DMS.HomerBody([pos, yaw])`: NM's body of Homer standing as `nm_agent_create` puts him at `pos` (m, his feet's
/// level; default the origin) facing `yaw` (rad): per part `{name, parent, mass, pos, rot, inertia}` (`inertia`
/// the 3x3 tensor about the centre in the part's frame, row by row, kg m^2).
unsafe fn homer_body_impl(api: &Api, l: *mut LuaState) -> Result<c_int, String> {
    let d = data::get().ok_or_else(|| format!("NM's data isn't loaded (DMS.LoadData): {}", data::how_to()))?;
    let at = if unsafe { (api.type_)(l, 1) } > lua::TNIL { Vec3::from_array(unsafe { api.check_vec3(l, 1)? }) } else { Vec3::ZERO };
    let yaw = unsafe { api.opt_number(l, 2, 0.0)? } as f32;
    let placed = d.body.placed(at, yaw);
    unsafe { (api.createtable)(l, d.body.parts.len() as c_int, 0) };
    for (i, (p, (pos, rot))) in d.body.parts.iter().zip(placed).enumerate() {
        let t = p.inertia.transpose().to_cols_array();
        unsafe {
            (api.createtable)(l, 0, 6);
            api.set_str(l, c"name", p.name);
            api.set_number(l, c"parent", p.parent.map_or(-1.0, |q| q as f64));
            api.set_number(l, c"mass", f64::from(p.mass));
            api.set_vector(l, c"pos", &pos.to_array());
            api.set_vector(l, c"rot", &rot.to_array());
            (api.createtable)(l, 9, 0);
            for (k, v) in t.iter().enumerate() {
                (api.pushnumber)(l, f64::from(*v));
                api.append(l, k + 1);
            }
            (api.setfield)(l, -2, c"inertia".as_ptr());
            api.append(l, i + 1);
        }
    }
    Ok(1)
}
