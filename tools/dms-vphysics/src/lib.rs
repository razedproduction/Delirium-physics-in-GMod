//! VPhysics-Box3D for DMS without Lua (ROADMAP M12.5, docs/adr/0031): the fork's `IPhysicsNM` (`nm`), NM's body bound
//! and made natively on it (`body`), and the fork loaded outside the game (`host`). Shared by the module
//! (`gmsv_dms`) and the rig editor (`dms-rig`), so a body is the same physics in both.

pub mod body;
pub mod host;
pub mod nm;
