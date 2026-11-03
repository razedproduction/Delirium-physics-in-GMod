//! VPhysics-Box3D for DMS without Lua (ROADMAP M12.5, docs/adr/0031): the fork's `IPhysicsNM` (`nm`), NM's body bound
//! and made natively on it (`body`), NM's FOLLOW step on it (`follow`), the fork loaded outside the game (`host`),
//! NM's shapes hulled as the fork grows them (`hull`) and Source's `.phy` (`phy`). The core shared by the module
//! (`gmsv_dms`) and, read only, by the ragdolls project (docs/adr/0036), so a body is the same physics in both.

pub mod body;
pub mod follow;
pub mod host;
pub mod hull;
pub mod nm;
pub mod phy;

/// Garry's Mod in Steam's main library, from the registry (`HKCU\Software\Valve\Steam\SteamPath`).
pub fn steam_gmod() -> Option<std::path::PathBuf> {
    let o = std::process::Command::new("reg").args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"]).output().ok()?;
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let path = text.lines().find(|l| l.contains("SteamPath"))?.split("REG_SZ").nth(1)?.trim().to_string();
    let g = std::path::PathBuf::from(path).join("steamapps").join("common").join("GarrysMod");
    g.is_dir().then_some(g)
}
