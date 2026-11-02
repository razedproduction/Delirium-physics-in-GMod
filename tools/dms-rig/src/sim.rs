//! The fitted body on the fork itself (ROADMAP M12.5 step 1, docs/adr/0031): `vphysics.dll` (VPhysics-Box3D) loaded
//! into the editor, an environment as the game's server sets one up (0.015 s steps, air, a concrete floor with NM's
//! material), the body made natively as the module makes it (`dms_vphysics::body::NativeBody::make`), then dropped,
//! thrown and dragged by the mouse — the body's physics only, no NM.

use std::path::{Path, PathBuf};

use delirium::prelude::*;
use dms_fit::M;
use dms_game::body::Body;
use dms_vphysics::body::{NativeBody, PartState};
use dms_vphysics::host::{Environment, Host};
use dms_vphysics::nm::{self, PhysicsNMBodyState};

/// The game's physics step (`IPhysicsEnvironment::SetSimulationTimestep` on the server, whatever the tickrate).
pub const TIMESTEP: f32 = 0.015;
/// Source's `sv_gravity` (units/s^2), and NM's (m/s^2) that the module makes the body feel under NM (docs/adr/0007).
pub const SOURCE_GRAVITY: f32 = 600.0;
pub const NM_GRAVITY: f32 = delirium::ragdoll::GRAVITY;

/// Where the fork's `vphysics.dll` is: Garry's Mod's (installed by `scripts/install.ps1 -Fork`), else the
/// repository's build. Its imports (`tier0.dll`) come from Garry's Mod's `bin\win64` either way.
pub fn find_vphysics(gmod: &Path, repo: Option<&Path>) -> Vec<PathBuf> {
    let mut v = vec![gmod.join("bin").join("win64").join("vphysics.dll")];
    if let Some(r) = repo {
        for p in ["vphysics/build/windows-x64/src/Release/vphysics.dll", "vphysics/build/windows-x64/Release/vphysics.dll", "vphysics/build/src/Release/vphysics.dll"] {
            v.push(r.join(p));
        }
    }
    v.into_iter().filter(|p| p.is_file()).collect()
}

/// A part held by the mouse: the point on it (m, the part's frame) and where it's pulled to (m, world).
pub struct Grab {
    pub part: usize,
    pub local: Vec3,
    pub target: Vec3,
}

/// The fork loaded, its environment and the body in it.
pub struct Sim {
    pub host: &'static Host,
    env: Environment<'static>,
    pub dll: PathBuf,
    pub surfaces: i32,
    body: Option<(NativeBody, Body)>,
    acc: f32,
    pub time: f32,
    pub steps: u64,
    pub paused: bool,
    pub speed: f32,
    pub grab: Option<Grab>,
    pub nm_gravity: bool,
    /// The step's cost (ms), for the status line.
    pub step_ms: f32,
}

impl Sim {
    /// Loads the fork (`dll`, its imports from `deps`) and makes the environment: the game's surface properties
    /// (`surfaces`: the scripts' names and texts), a floor at z = 0 of `floor_material`, NM's friction and elasticity
    /// on it (`concrete`, the Playroom's floor row).
    pub fn new(dll: &Path, deps: &Path, surfaces: &[(String, String)], floor_material: &str, concrete: [f32; 2]) -> Result<Sim, String> {
        // One host a process: the DLL can't be loaded twice anyway.
        static HOST: std::sync::OnceLock<Result<&'static Host, String>> = std::sync::OnceLock::new();
        let host = HOST.get_or_init(|| Host::load(dll, deps).map(|h| &*Box::leak(Box::new(h)))).clone()?;
        let mut count = 0;
        for (name, text) in surfaces {
            count = host.parse_surfaces(name, text);
        }
        let mut env = host.environment([0.0, 0.0, -SOURCE_GRAVITY], TIMESTEP)?;
        let material = host.surface_index(floor_material);
        if material >= 0 {
            host.nm.set_surface_material(material, concrete[0], concrete[1]);
        }
        env.static_box([-4000.0, -4000.0, -64.0], [4000.0, 4000.0, 0.0], material)?;
        let mut s = Sim {
            host,
            env,
            dll: dll.to_path_buf(),
            surfaces: count,
            body: None,
            acc: 0.0,
            time: 0.0,
            steps: 0,
            paused: false,
            speed: 1.0,
            grab: None,
            nm_gravity: true,
            step_ms: 0.0,
        };
        s.set_gravity(true);
        Ok(s)
    }

    /// The gravity: NM's 9.8 m/s^2 (what the body feels in the game under NM: Source's and the module's
    /// compensating force, docs/adr/0007) or Source's own 600 units/s^2.
    pub fn set_gravity(&mut self, nm: bool) {
        self.nm_gravity = nm;
        let g = if nm { NM_GRAVITY / M } else { SOURCE_GRAVITY };
        // SAFETY: our live environment, outside its step.
        unsafe { nm::env::set_gravity(self.env.env, [0.0, 0.0, -g]) };
    }

    pub fn has_body(&self) -> bool {
        self.body.is_some()
    }

    /// The body taken out.
    pub fn clear(&mut self) {
        if let Some((b, _)) = self.body.take() {
            // SAFETY: made by `place`, on this thread.
            unsafe { b.destroy(&self.host.nm) };
        }
        self.grab = None;
    }

    /// `body` made anew standing as NM puts an agent (feet at `lift` m above the floor), turned by `tilt` (rad about
    /// X) and moving at `vel` (m/s), spinning at `spin` (rad/s).
    pub fn place(&mut self, body: &Body, lift: f32, tilt: f32, vel: Vec3, spin: Vec3) -> Result<(), String> {
        self.clear();
        let placed = body.placed(Vec3::new(0.0, 0.0, lift), 0.0);
        let centre = placed.iter().map(|p| p.0).sum::<Vec3>() / placed.len() as f32;
        let turn = Quat::from_rotation_x(tilt);
        let states: Vec<PartState> = placed
            .into_iter()
            .map(|(p, q)| {
                let pos = centre + turn * (p - centre);
                PartState { pos, rot: (turn * q).normalize(), vel: vel + spin.cross(pos - centre), spin }
            })
            .collect();
        // SAFETY: our live environment, on its thread, outside its step.
        let made = unsafe { NativeBody::make(&self.host.nm, self.env.env, std::ptr::null_mut(), body, &states, 0)? };
        self.body = Some((made, body.clone()));
        self.time = 0.0;
        self.steps = 0;
        Ok(())
    }

    /// The parts' states now.
    pub fn states(&self) -> Option<Vec<PartState>> {
        // SAFETY: a live body of ours.
        self.body.as_ref().and_then(|(b, _)| unsafe { b.states(&self.host.nm) }.ok())
    }

    /// Every part given `vel` more (m/s) and `spin` (rad/s about the body's centre).
    pub fn kick(&mut self, vel: Vec3, spin: Vec3) {
        let Some(states) = self.states() else { return };
        let Some((b, _)) = &self.body else { return };
        let centre = states.iter().map(|s| s.pos).sum::<Vec3>() / states.len() as f32;
        for (s, &o) in states.iter().zip(&b.objects) {
            let v = s.vel + vel + spin.cross(s.pos - centre);
            let state = PhysicsNMBodyState { position: s.pos.to_array(), rotation: s.rot.to_array(), velocity: v.to_array(), angular_velocity: (s.spin + spin).to_array() };
            // SAFETY: a live body of ours, outside the step.
            unsafe { self.host.nm.set_body_state(o, &state) };
        }
    }

    /// The physics run on by `dt` of real time (s), in the game's steps; the mouse's pull applied each step as the
    /// physgun pulls (an impulse toward the target, damped).
    pub fn run(&mut self, dt: f32) {
        if self.paused {
            return;
        }
        self.acc = (self.acc + dt.min(0.1) * self.speed).min(TIMESTEP * 8.0);
        let started = std::time::Instant::now();
        let mut stepped = 0;
        while self.acc >= TIMESTEP {
            self.acc -= TIMESTEP;
            self.pull();
            self.env.simulate(TIMESTEP);
            self.time += TIMESTEP;
            self.steps += 1;
            stepped += 1;
        }
        if stepped > 0 {
            self.step_ms = started.elapsed().as_secs_f32() * 1000.0 / stepped as f32;
        }
    }

    fn pull(&mut self) {
        let (Some(g), Some((b, _))) = (&self.grab, &self.body) else { return };
        let o = b.objects[g.part];
        // SAFETY: a live body of ours, outside the step.
        let Some(s) = (unsafe { self.host.nm.body_state(o) }) else { return };
        let (pos, rot) = (Vec3::from_array(s.position), Quat::from_array(s.rotation).normalize());
        let at = pos + rot * g.local;
        let v = Vec3::from_array(s.velocity) + Vec3::from_array(s.angular_velocity).cross(at - pos);
        // Toward the target in about a tenth of a second, at most 10 m/s, damped; on the part's own mass.
        let want = ((g.target - at) / 0.1).clamp_length_max(10.0);
        // SAFETY: as above.
        let mass = unsafe { nm::object::mass(o) };
        let impulse = (want - v) * mass * 0.5 / M;
        unsafe {
            nm::object::apply_force_offset(o, impulse.to_array(), (at / M).to_array());
            nm::object::wake(o);
        }
    }
}

impl Drop for Sim {
    fn drop(&mut self) {
        self.clear();
    }
}
