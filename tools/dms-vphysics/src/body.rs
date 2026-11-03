//! NM's body on VPhysics-Box3D without Lua (ROADMAP M3, M9–M12.5): the bind (every body NM's inertia and contact
//! material, every joint NM's pivot, frames and limits) and the native body (`VBOX3D_NM_005`/`_006`: NM's 21
//! capsules and boxes made by the fork, joined, bound and only NM's self-collision pairs colliding). The module
//! calls them from its natives, the rig editor (`tools/dms-rig`) from its own host.

use delirium::prelude::*;
use dms_game::body::Body;

use crate::nm::{IPhysicsConstraint, IPhysicsEnvironment, IPhysicsObject, Nm, PhysicsNMBodyState, Version};

/// A joint as the bind set it: its pivot (m, world) and limits.
pub struct Bound {
    pub part: usize,
    pub parent: usize,
    pub hinge: bool,
    pub anchor: Vec3,
    pub limits: [f32; 3],
}

/// A body's frame from its state: the centre of mass is the part's centre (`dms-homer`'s `.phy`, the fork's hulls).
fn pose(nm: &Nm, object: *mut IPhysicsObject) -> Result<(Vec3, Quat), String> {
    // SAFETY: a live object (the caller's promise).
    let s = unsafe { nm.body_state(object) }.ok_or("GetBodyState failed")?;
    Ok((Vec3::from_array(s.position), Quat::from_array(s.rotation).normalize()))
}

/// Binds `objects` (a ragdoll's bodies in NM's part order, standing as spawned) to `body`: Homer's, or one fitted to
/// a model (ROADMAP M9). Every body NM's inertia tensor (`SetBodyInertia`) and material (`SetBodyMaterial`), every
/// joint (found by `FindConstraint`) NM's pivot (`SetJointAnchor`), frames and limits (a ball `SetJointBallLimits`,
/// a hinge `SetJointLimits`).
///
/// # Safety
/// The objects are live bodies of this vphysics; called from the thread that runs the physics, outside its step.
pub unsafe fn bind(nm: &Nm, body: &Body, objects: &[*mut IPhysicsObject]) -> Result<Vec<Bound>, String> {
    if nm.version < Version::V004 {
        return Err(format!("{} has no SetBodyMaterial (or SetBodyInertia / SetJointBallLimits): update VPhysics-Box3D", nm.version.name()));
    }
    let poses = objects.iter().map(|&o| pose(nm, o)).collect::<Result<Vec<_>, _>>()?;
    for (p, &o) in body.parts.iter().zip(objects) {
        // Symmetric: its columns are its rows.
        let tensor = p.inertia.to_cols_array();
        // SAFETY: a live body; 9 floats.
        if unsafe { nm.set_body_inertia(o, &tensor) } != Some(true) {
            return Err(format!("{}: SetBodyInertia refused NM's tensor {tensor:?}", p.name));
        }
        // NM's contact material of the part (ROADMAP M5): its friction and elasticity against NM's surfaces.
        let (friction, elasticity) = dms_game::body::part_material(&p.bone);
        // SAFETY: as above.
        if unsafe { nm.set_body_material(o, friction, elasticity) } != Some(true) {
            return Err(format!("{}: SetBodyMaterial refused {friction} {elasticity}", p.name));
        }
    }
    let mut out = Vec::with_capacity(body.joints.len());
    for j in &body.joints {
        let name = body.parts[j.part].name;
        let (parent, child) = (objects[j.parent], objects[j.part]);
        // SAFETY: both are live bodies.
        let c = unsafe { nm.find_constraint(parent, child) };
        if c.is_null() {
            return Err(format!("{name}: no constraint to {} (not models/dms/homer.mdl?)", body.parts[j.parent].name));
        }
        let (pivot, frames) = body.joint_world(j, &poses);
        // SAFETY: `c` joins two live objects; the arrays are as long as the calls read.
        let anchored = unsafe { nm.set_joint_anchor(c, Some(&pivot.to_array())) };
        let frames = [frames[0].to_array(), frames[1].to_array()];
        let limited = if j.hinge {
            unsafe { nm.set_joint_limits(c, parent, &frames, true, &j.limits) }
        } else {
            unsafe { nm.set_joint_ball_limits(c, parent, &frames, &j.limits) }
        } == Some(true);
        if !anchored || !limited {
            return Err(format!("{name}: SetJointAnchor {anchored}, {} {limited} (was the constraint built as a {}?)",
                if j.hinge { "SetJointLimits" } else { "SetJointBallLimits" }, if j.hinge { "hinge" } else { "ragdoll constraint" }));
        }
        out.push(Bound { part: j.part, parent: j.parent, hinge: j.hinge, anchor: pivot, limits: j.limits });
    }
    Ok(out)
}

/// A part's state: frame (m, world) and its centre's velocity and spin.
#[derive(Clone, Copy, Debug)]
pub struct PartState {
    pub pos: Vec3,
    pub rot: Quat,
    pub vel: Vec3,
    pub spin: Vec3,
}

impl PartState {
    /// The rigid motion of this part carried to a point.
    pub fn at(&self, p: Vec3) -> Vec3 {
        self.vel + self.spin.cross(p - self.pos)
    }

    /// Standing still at a frame.
    pub fn still((pos, rot): (Vec3, Quat)) -> PartState {
        PartState { pos, rot, vel: Vec3::ZERO, spin: Vec3::ZERO }
    }
}

/// A body made by the fork (`VBOX3D_NM_005`+): its 21 bodies in part order and their joints (null for the root).
#[derive(Clone)]
pub struct NativeBody {
    pub objects: Vec<*mut IPhysicsObject>,
    pub joints: Vec<*mut IPhysicsConstraint>,
}

// SAFETY: only used on the thread that runs the physics (the game's main thread, the editor's UI thread).
unsafe impl Send for NativeBody {}

impl NativeBody {
    /// `body` made by the fork in `env`, standing in for `owner` (null: none; its game data, material and flags
    /// otherwise, plus `game_flags`): NM's capsules and boxes as `dms-homer` hulls them (the fork keeps the hulls),
    /// NM's masses, a ragdoll constraint per joint, bound to NM's body ([`bind`]), posed and moving as `states` and
    /// only NM's self-collision pairs colliding (`VBOX3D_NM_006`). Nothing is left of it on an error.
    ///
    /// # Safety
    /// `env` is a live environment and `owner` null or a live object of it; outside the step callback, on the
    /// thread that runs the physics.
    pub unsafe fn make(nm: &Nm, env: *mut IPhysicsEnvironment, owner: *mut IPhysicsObject, body: &Body, states: &[PartState], game_flags: u16) -> Result<NativeBody, String> {
        if nm.version < Version::V006 {
            return Err(format!("{} can't make GMod's ragdolls' bodies: update VPhysics-Box3D ({} needed)", nm.version.name(), Version::V006.name()));
        }
        if states.len() != body.parts.len() || states.iter().any(|s| !(s.pos.is_finite() && s.rot.is_finite() && s.vel.is_finite() && s.spin.is_finite())) {
            return Err("the ragdoll's pose isn't finite".into());
        }
        let mut made = NativeBody { objects: vec![], joints: vec![std::ptr::null_mut(); body.parts.len()] };
        for (p, s) in body.parts.iter().zip(states) {
            let points: Vec<[f32; 3]> = crate::hull::points(&p.shape).iter().map(|v| v.to_array()).collect();
            // SAFETY: the caller's.
            let o = unsafe { nm.create_body(env, owner, &points, p.mass, &s.pos.to_array(), &s.rot.to_array(), game_flags) }.unwrap_or(std::ptr::null_mut());
            if o.is_null() {
                unsafe { made.destroy(nm) };
                return Err(format!("{}: CreateBody failed", p.name));
            }
            made.objects.push(o);
        }
        for j in &body.joints {
            // SAFETY: two bodies just made.
            let c = unsafe { nm.create_joint(made.objects[j.parent], made.objects[j.part], j.hinge) }.unwrap_or(std::ptr::null_mut());
            if c.is_null() {
                unsafe { made.destroy(nm) };
                return Err(format!("{}: CreateJoint failed", body.parts[j.part].name));
            }
            made.joints[j.part] = c;
        }
        // NM's pivots, frames and limits in the pose they're in, then their motion and NM's own collision pairs.
        if let Err(e) = unsafe { bind(nm, body, &made.objects) } {
            unsafe { made.destroy(nm) };
            return Err(format!("bind: {e}"));
        }
        for (&o, s) in made.objects.iter().zip(states) {
            let state = PhysicsNMBodyState { position: s.pos.to_array(), rotation: s.rot.to_array(), velocity: s.vel.to_array(), angular_velocity: s.spin.to_array() };
            // SAFETY: a body just made.
            unsafe { nm.set_body_state(o, &state) };
        }
        // SAFETY: as above.
        if unsafe { nm.set_body_collision_pairs(&made.objects, &body.pairs) } != Some(true) {
            unsafe { made.destroy(nm) };
            return Err("SetBodyCollisionPairs refused NM's self-collision pairs".into());
        }
        Ok(made)
    }

    /// Its joints and bodies gone.
    ///
    /// # Safety
    /// Made by [`NativeBody::make`] and not destroyed yet; outside the step callback.
    pub unsafe fn destroy(&self, nm: &Nm) {
        for &j in &self.joints {
            // SAFETY: the caller's; null is a no-op.
            unsafe { nm.destroy_joint(j) };
        }
        for &o in &self.objects {
            unsafe { nm.destroy_body(o) };
        }
    }

    /// Every part's state now.
    ///
    /// # Safety
    /// As [`NativeBody::destroy`]: alive.
    pub unsafe fn states(&self, nm: &Nm) -> Result<Vec<PartState>, String> {
        self.objects
            .iter()
            .map(|&o| {
                // SAFETY: the caller's.
                let s = unsafe { nm.body_state(o) }.ok_or("GetBodyState failed")?;
                Ok(PartState {
                    pos: Vec3::from_array(s.position),
                    rot: Quat::from_array(s.rotation).normalize(),
                    vel: Vec3::from_array(s.velocity),
                    spin: Vec3::from_array(s.angular_velocity),
                })
            })
            .collect()
    }
}
