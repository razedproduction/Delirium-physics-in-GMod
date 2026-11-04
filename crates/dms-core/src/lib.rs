//! dms-core: the module's FOLLOW core (docs/adr/0037), moved out of `gmsv_dms` as it was, for it and for the ragdolls
//! project (`Delirium-ragdolls-in-GMod`, docs/adr/0036), which depends on it by path, read only.
//!
//! What's here: the LuaJIT bridge (`lua`, its `native!`), the fork's `IPhysicsNM` (`ffi`), NM's data from the game
//! (`data`, the SDK's C API `nm`), the map's world (`world`), the player's tuning (`tuning`), the FOLLOW agent and the
//! Playroom's room on it (`follow`), the step callback (`step`), the agents' natives (`agents`, `react`), the debug
//! natives on `IPhysicsNM` (`natives`) and telemetry's JSON (`json`). Each module's `NATIVES` is a list of
//! `(name, function)` the host module puts on its own Lua table; nothing here registers a global.
//!
//! The extension point for other bodies is [`follow::BodyDef`]: an agent drives whatever NM build and body it's given
//! ([`agents::start_body`]); Homer's is [`follow::BodyDef::homer`]. A host module adds its bodies by making its own
//! `BodyDef`s, with no code path for them here.

pub mod agents;
pub mod data;
pub mod ffi;
pub mod follow;
pub mod json;
pub mod lua;
pub mod natives;
pub mod nm;
pub mod react;
pub mod step;
pub mod tuning;
pub mod world;

/// Frees what the statics hold: GMod unloads the module with each map, and its statics' heap isn't freed with it
/// (docs/adr/0021): ~65 MB a map with agents (NM's data, the map read, its worlds). The step callback goes first
/// ([`step::shutdown`]).
pub fn unload() {
    data::unload();
    world::unload();
    tuning::unload();
    *delirium::ragdoll::SURFACES.write().unwrap_or_else(|e| e.into_inner()) = Vec::new();
}
