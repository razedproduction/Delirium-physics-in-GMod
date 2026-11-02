//! VPhysics-Box3D outside the game (ROADMAP M12.5, docs/adr/0031): `vphysics.dll` loaded into a process of our own
//! (the rig editor, `tools/dms-rig`), an environment made the way the game's server makes one (gravity, air,
//! timestep), the surface properties parsed from the game's scripts, and a static floor. What's put in it is the
//! module's own (`crate::body`), so a body behaves here as it does in the game.
//!
//! Only Source's interfaces of vphysics (`IPhysics`, `IPhysicsCollision`, `IPhysicsSurfaceProps`; Source SDK 2013
//! `vphysics_interface.h`, MSVC's vtables: a virtual destructor takes one slot) and the fork's `IPhysicsNM`.

use std::ffi::{CString, c_char, c_void};
use std::path::Path;
use std::ptr;

use crate::nm::{self, IPhysicsEnvironment, IPhysicsObject, Nm};

/// `CPhysCollide`, only ever handled by pointer.
#[repr(C)]
pub struct CPhysCollide {
    _private: [u8; 0],
}

/// `objectparams_t`.
#[repr(C)]
pub struct ObjectParams {
    pub mass_center_override: *const [f32; 3],
    pub mass: f32,
    pub inertia: f32,
    pub damping: f32,
    pub rot_damping: f32,
    pub rot_inertia_limit: f32,
    pub name: *const c_char,
    pub game_data: *mut c_void,
    pub volume: f32,
    pub drag_coefficient: f32,
    pub enable_collisions: bool,
}

impl Default for ObjectParams {
    /// `g_PhysDefaultObjectParams` (Source's `physics_shared`): 1 kg, no damping, collisions on.
    fn default() -> Self {
        ObjectParams {
            mass_center_override: ptr::null(),
            mass: 1.0,
            inertia: 1.0,
            damping: 0.1,
            rot_damping: 0.1,
            rot_inertia_limit: 0.05,
            name: c"DEFAULT".as_ptr(),
            game_data: ptr::null_mut(),
            volume: 0.0,
            drag_coefficient: 1.0,
            enable_collisions: true,
        }
    }
}

/// `vphysics.dll` loaded and its interfaces.
pub struct Host {
    physics: *mut c_void,
    collision: *mut c_void,
    surfaces: *mut c_void,
    pub nm: Nm,
}

// SAFETY: vphysics' interfaces are process-wide singletons; the editor calls them from its UI thread only.
unsafe impl Send for Host {}
unsafe impl Sync for Host {}

unsafe fn slot(this: *mut c_void, i: usize) -> *const () {
    // SAFETY: a live C++ object, its first field the vtable (the caller's promise); the slot is in it.
    unsafe { *(*(this as *const *const *const ())).add(i) }
}

/// `IPhysics`: `IAppSystem`'s five (`Connect`, `Disconnect`, `QueryInterface`, `Init`, `Shutdown`), then its own.
const CREATE_ENVIRONMENT: usize = 5;
const DESTROY_ENVIRONMENT: usize = 6;
/// `IPhysicsCollision`.
const DESTROY_COLLIDE: usize = 16;
const BBOX_TO_COLLIDE: usize = 29;
/// `IPhysicsSurfaceProps`.
const PARSE_SURFACE_DATA: usize = 1;
const GET_SURFACE_INDEX: usize = 3;

impl Host {
    /// Loads `dll` (the fork's `vphysics.dll`) with `deps` searched for its own imports (`tier0.dll`, Garry's Mod's
    /// `bin\win64`), and takes its interfaces. One per process: a second load is the same DLL.
    pub fn load(dll: &Path, deps: &Path) -> Result<Host, String> {
        sys::load(dll, deps)?;
        let create = nm::create_interface().ok_or_else(|| format!("{}: no CreateInterface", dll.display()))?;
        let get = |name: &std::ffi::CStr| {
            // SAFETY: vphysics' exported factory, a null terminated name.
            let p = unsafe { create(name.as_ptr(), ptr::null_mut()) };
            if p.is_null() { Err(format!("{}: no {}", dll.display(), name.to_string_lossy())) } else { Ok(p) }
        };
        let (physics, collision, surfaces) = (get(c"VPhysics031")?, get(c"VPhysicsCollision007")?, get(c"VPhysicsSurfaceProps001")?);
        let version = nm::acquire().map_err(|e| format!("{}: {e:?} (not VPhysics-Box3D with IPhysicsNM)", dll.display()))?;
        let nm = nm::get().ok_or("IPhysicsNM lost")?;
        if version < nm::Version::V006 {
            return Err(format!("{} has {}: the native bodies need {} (update VPhysics-Box3D)", dll.display(), version.name(), nm::Version::V006.name()));
        }
        Ok(Host { physics, collision, surfaces, nm })
    }

    /// `ParseSurfaceData`: a surface properties script (`scripts/surfaceproperties*.txt`), as the engine parses the
    /// manifest's. The count of properties.
    pub fn parse_surfaces(&self, file: &str, text: &str) -> i32 {
        let (Ok(file), Ok(text)) = (CString::new(file), CString::new(text)) else { return 0 };
        // SAFETY: `int ParseSurfaceData(const char *, const char *)`, `this` first.
        let f: unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> i32 = unsafe { std::mem::transmute(slot(self.surfaces, PARSE_SURFACE_DATA)) };
        unsafe { f(self.surfaces, file.as_ptr(), text.as_ptr()) }
    }

    /// `GetSurfaceIndex`, -1 for none.
    pub fn surface_index(&self, name: &str) -> i32 {
        let Ok(name) = CString::new(name) else { return -1 };
        // SAFETY: `int GetSurfaceIndex(const char *) const`, `this` first.
        let f: unsafe extern "C" fn(*mut c_void, *const c_char) -> i32 = unsafe { std::mem::transmute(slot(self.surfaces, GET_SURFACE_INDEX)) };
        unsafe { f(self.surfaces, name.as_ptr()) }
    }

    /// A new environment as the game's server sets one up: `gravity` (units/s^2), air density 2, `timestep` (s).
    pub fn environment(&self, gravity: [f32; 3], timestep: f32) -> Result<Environment<'_>, String> {
        // SAFETY: `IPhysicsEnvironment *CreateEnvironment()`, `this` first.
        let f: unsafe extern "C" fn(*mut c_void) -> *mut IPhysicsEnvironment = unsafe { std::mem::transmute(slot(self.physics, CREATE_ENVIRONMENT)) };
        let env = unsafe { f(self.physics) };
        if env.is_null() {
            return Err("CreateEnvironment failed".into());
        }
        // SAFETY: the environment just made, on this thread.
        unsafe {
            nm::env::set_gravity(env, gravity);
            nm::env::set_air_density(env, 2.0);
            nm::env::set_timestep(env, timestep);
        }
        Ok(Environment { host: self, env, statics: vec![] })
    }

    fn bbox(&self, mins: [f32; 3], maxs: [f32; 3]) -> *mut CPhysCollide {
        // SAFETY: `CPhysCollide *BBoxToCollide(const Vector &, const Vector &)`, `this` first.
        let f: unsafe extern "C" fn(*mut c_void, *const [f32; 3], *const [f32; 3]) -> *mut CPhysCollide = unsafe { std::mem::transmute(slot(self.collision, BBOX_TO_COLLIDE)) };
        unsafe { f(self.collision, &mins, &maxs) }
    }

    fn destroy_collide(&self, c: *mut CPhysCollide) {
        // SAFETY: `void DestroyCollide(CPhysCollide *)`, `this` first; `c` from `bbox`, its objects gone.
        let f: unsafe extern "C" fn(*mut c_void, *mut CPhysCollide) = unsafe { std::mem::transmute(slot(self.collision, DESTROY_COLLIDE)) };
        unsafe { f(self.collision, c) }
    }
}

/// An environment of the host's, destroyed with its static objects when dropped (the bodies in it go first:
/// `crate::body::NativeBody::destroy`).
pub struct Environment<'h> {
    host: &'h Host,
    pub env: *mut IPhysicsEnvironment,
    statics: Vec<(*mut IPhysicsObject, *mut CPhysCollide)>,
}

impl Environment<'_> {
    /// A static box from `mins` to `maxs` (units, world) of the surface property `material` (-1: `default`).
    pub fn static_box(&mut self, mins: [f32; 3], maxs: [f32; 3], material: i32) -> Result<*mut IPhysicsObject, String> {
        let c = self.host.bbox(mins, maxs);
        if c.is_null() {
            return Err("BBoxToCollide failed".into());
        }
        let mut params = ObjectParams { name: c"dms_floor".as_ptr(), ..ObjectParams::default() };
        // SAFETY: this live environment; the collide outlives the object (destroyed after it on drop).
        let o = unsafe { nm::env::create_poly_object_static(self.env, c, material, [0.0; 3], [0.0; 3], &mut params) };
        if o.is_null() {
            self.host.destroy_collide(c);
            return Err("CreatePolyObjectStatic failed".into());
        }
        self.statics.push((o, c));
        Ok(o)
    }

    /// The environment run on by `dt` (s).
    pub fn simulate(&self, dt: f32) {
        // SAFETY: a live environment, on the thread that made it.
        unsafe { nm::env::simulate(self.env, dt) }
    }
}

impl Drop for Environment<'_> {
    fn drop(&mut self) {
        for &(o, c) in &self.statics {
            // SAFETY: objects this environment made, then their collides.
            unsafe { nm::env::destroy_object(self.env, o) };
            self.host.destroy_collide(c);
        }
        // SAFETY: `void DestroyEnvironment(IPhysicsEnvironment *)`, `this` first.
        let f: unsafe extern "C" fn(*mut c_void, *mut IPhysicsEnvironment) = unsafe { std::mem::transmute(slot(self.host.physics, DESTROY_ENVIRONMENT)) };
        unsafe { f(self.host.physics, self.env) };
    }
}

#[cfg(windows)]
mod sys {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    unsafe extern "system" {
        fn SetDllDirectoryW(path: *const u16) -> i32;
        fn LoadLibraryW(path: *const u16) -> *mut std::ffi::c_void;
        fn GetLastError() -> u32;
    }

    fn wide(p: &Path) -> Vec<u16> {
        p.as_os_str().encode_wide().chain([0]).collect()
    }

    pub fn load(dll: &Path, deps: &Path) -> Result<(), String> {
        // SAFETY: null terminated wide paths.
        unsafe {
            SetDllDirectoryW(wide(deps).as_ptr());
            let m = LoadLibraryW(wide(dll).as_ptr());
            if m.is_null() {
                return Err(format!("{}: LoadLibrary failed ({}; its imports are looked for in {})", dll.display(), GetLastError(), deps.display()));
            }
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod sys {
    pub fn load(dll: &std::path::Path, _: &std::path::Path) -> Result<(), String> {
        Err(format!("{}: Windows only", dll.display()))
    }
}
