//! Garry's Mod's files where the game has them (docs/adr/0031): its loose files and legacy addons, its VPKs (its own
//! and the Source engine content it ships), and the Workshop's GMAs it downloaded. Read only: nothing is copied or
//! written, as the module reads models through `file.Read(model, "GAME")` (docs/adr/0030).

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Where a file's bytes are.
#[derive(Clone)]
enum At {
    Loose(PathBuf),
    /// A VPK's: the archive (`_dir` itself for 0x7fff), offset and length, then its preload in the `_dir` file.
    Vpk { archive: PathBuf, offset: u64, len: u64, dir: PathBuf, preload: (u64, u64) },
    Gma { file: PathBuf, offset: u64, len: u64 },
}

/// A place files come from, for the model list.
#[derive(Clone)]
pub struct Source {
    pub name: String,
}

#[derive(Clone)]
struct Entry {
    at: At,
    source: usize,
}

/// The game's files by path (lower case, `/`), the first found winning as the game's search path does.
pub struct GameFs {
    pub gmod: PathBuf,
    pub sources: Vec<Source>,
    files: HashMap<String, Entry>,
    /// What couldn't be read (a broken GMA), for the status line.
    pub problems: Vec<String>,
}

fn norm(p: &str) -> String {
    p.replace('\\', "/").trim_start_matches('/').to_ascii_lowercase()
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// A null terminated string at `*at`, moved past.
fn cstr(b: &[u8], at: &mut usize) -> Option<String> {
    let s = b.get(*at..)?;
    let end = s.iter().position(|&c| c == 0)?;
    *at += end + 1;
    Some(String::from_utf8_lossy(&s[..end]).into_owned())
}

impl GameFs {
    /// Garry's Mod at `gmod` (the folder with `garrysmod\` and `sourceengine\`).
    pub fn open(gmod: &Path) -> Result<GameFs, String> {
        let gm = gmod.join("garrysmod");
        if !gm.is_dir() {
            return Err(format!("{}: not Garry's Mod's folder (no garrysmod\\)", gmod.display()));
        }
        let mut fs = GameFs { gmod: gmod.to_path_buf(), sources: vec![], files: HashMap::new(), problems: vec![] };
        // The game's own search path first: its loose files, the legacy addons, its VPK, the engine's content.
        fs.add_loose(&gm, "garrysmod");
        if let Ok(rd) = std::fs::read_dir(gm.join("addons")) {
            let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
            dirs.sort();
            for d in dirs {
                let name = format!("addons/{}", d.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
                fs.add_loose(&d, &name);
            }
        }
        for vpk in [gm.join("garrysmod_dir.vpk"), gmod.join("sourceengine").join("hl2_misc_dir.vpk"), gmod.join("sourceengine").join("content_hl2_dir.vpk"), gmod.join("sourceengine").join("content_cstrike_dir.vpk")] {
            if vpk.is_file()
                && let Err(e) = fs.add_vpk(&vpk)
            {
                fs.problems.push(e);
            }
        }
        // The Workshop's: as Steam downloads them, and GMod's cache.
        let mut gmas: Vec<PathBuf> = vec![];
        if let Some(steamapps) = gmod.parent().and_then(Path::parent) {
            if let Ok(rd) = std::fs::read_dir(steamapps.join("workshop").join("content").join("4000")) {
                for d in rd.flatten() {
                    if let Ok(files) = std::fs::read_dir(d.path()) {
                        gmas.extend(files.flatten().map(|f| f.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gma"))));
                    }
                }
            }
        }
        if let Ok(rd) = std::fs::read_dir(gm.join("cache").join("workshop")) {
            gmas.extend(rd.flatten().map(|f| f.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gma"))));
        }
        gmas.sort();
        for g in gmas {
            if let Err(e) = fs.add_gma(&g) {
                fs.problems.push(e);
            }
        }
        Ok(fs)
    }

    fn source(&mut self, name: String) -> usize {
        self.sources.push(Source { name });
        self.sources.len() - 1
    }

    fn insert(&mut self, path: String, at: At, source: usize) {
        self.files.entry(path).or_insert(Entry { at, source });
    }

    /// A folder's `models/` and `scripts/` (what the editor reads).
    fn add_loose(&mut self, root: &Path, name: &str) {
        let s = self.source(name.to_string());
        for sub in ["models", "scripts"] {
            let mut stack = vec![root.join(sub)];
            while let Some(d) = stack.pop() {
                let Ok(rd) = std::fs::read_dir(&d) else { continue };
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if let Ok(rel) = p.strip_prefix(root) {
                        let key = norm(&rel.to_string_lossy());
                        self.insert(key, At::Loose(p), s);
                    }
                }
            }
        }
    }

    /// A VPK's directory (versions 1 and 2): its `models/` and `scripts/` entries.
    fn add_vpk(&mut self, dir: &Path) -> Result<(), String> {
        let b = std::fs::read(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let bad = || format!("{}: not a VPK directory", dir.display());
        if u32_at(&b, 0) != Some(0x55aa_1234) {
            return Err(bad());
        }
        let version = u32_at(&b, 4).ok_or_else(bad)?;
        let tree = u32_at(&b, 8).ok_or_else(bad)? as usize;
        let header = match version {
            1 => 12,
            2 => 28,
            v => return Err(format!("{}: VPK version {v}", dir.display())),
        };
        let data_start = (header + tree) as u64;
        let stem = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let base = stem.trim_end_matches("_dir.vpk").to_string();
        let s = self.source(base.clone());
        let mut at = header;
        loop {
            let ext = cstr(&b, &mut at).ok_or_else(bad)?;
            if ext.is_empty() {
                break;
            }
            loop {
                let path = cstr(&b, &mut at).ok_or_else(bad)?;
                if path.is_empty() {
                    break;
                }
                loop {
                    let file = cstr(&b, &mut at).ok_or_else(bad)?;
                    if file.is_empty() {
                        break;
                    }
                    let preload = u16_at(&b, at + 4).ok_or_else(bad)? as u64;
                    let archive = u16_at(&b, at + 6).ok_or_else(bad)?;
                    let offset = u32_at(&b, at + 8).ok_or_else(bad)? as u64;
                    let len = u32_at(&b, at + 12).ok_or_else(bad)? as u64;
                    at += 18;
                    let pre = (at as u64, preload);
                    at += preload as usize;
                    let full = if path == " " { format!("{file}.{ext}") } else { format!("{path}/{file}.{ext}") };
                    let key = norm(&full);
                    if !(key.starts_with("models/") || key.starts_with("scripts/")) {
                        continue;
                    }
                    let (archive, offset) = if archive == 0x7fff {
                        (dir.to_path_buf(), data_start + offset)
                    } else {
                        (dir.with_file_name(format!("{base}_{archive:03}.vpk")), offset)
                    };
                    self.insert(key, At::Vpk { archive, offset, len, dir: dir.to_path_buf(), preload: pre }, s);
                }
            }
        }
        Ok(())
    }

    /// A Workshop GMA's files (`GMAD`, versions 1 to 3).
    fn add_gma(&mut self, file: &Path) -> Result<(), String> {
        let mut f = File::open(file).map_err(|e| format!("{}: {e}", file.display()))?;
        // The header and the file table are at the start; 4 MB hold them for any addon.
        let mut b = vec![];
        (&mut f).take(4 << 20).read_to_end(&mut b).map_err(|e| format!("{}: {e}", file.display()))?;
        let bad = || format!("{}: not a GMA", file.display());
        if b.get(..4) != Some(b"GMAD") {
            return Err(bad());
        }
        let version = b[4];
        let mut at = 5 + 8 + 8;
        if version > 1 {
            while !cstr(&b, &mut at).ok_or_else(bad)?.is_empty() {}
        }
        let title = cstr(&b, &mut at).ok_or_else(bad)?;
        let _desc = cstr(&b, &mut at).ok_or_else(bad)?;
        let _author = cstr(&b, &mut at).ok_or_else(bad)?;
        at += 4;
        let mut entries = vec![];
        loop {
            let num = u32_at(&b, at).ok_or_else(bad)?;
            at += 4;
            if num == 0 {
                break;
            }
            let name = cstr(&b, &mut at).ok_or_else(bad)?;
            let size = b.get(at..at + 8).map(|s| u64::from_le_bytes(s.try_into().unwrap_or([0; 8]))).ok_or_else(bad)?;
            at += 12;
            entries.push((name, size));
        }
        let id = file.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let s = self.source(format!("workshop {id}: {title}"));
        let mut offset = at as u64;
        for (name, size) in entries {
            let key = norm(&name);
            if key.starts_with("models/") || key.starts_with("scripts/") {
                self.insert(key, At::Gma { file: file.to_path_buf(), offset, len: size }, s);
            }
            offset += size;
        }
        Ok(())
    }

    /// A file's bytes.
    pub fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        let key = norm(path);
        let e = self.files.get(&key).ok_or_else(|| format!("{key}: not found"))?;
        let range = |p: &Path, offset: u64, len: u64| -> Result<Vec<u8>, String> {
            let mut f = File::open(p).map_err(|e| format!("{}: {e}", p.display()))?;
            f.seek(SeekFrom::Start(offset)).map_err(|e| format!("{}: {e}", p.display()))?;
            let mut b = vec![0u8; len as usize];
            f.read_exact(&mut b).map_err(|e| format!("{}: {key}: {e}", p.display()))?;
            Ok(b)
        };
        match &e.at {
            At::Loose(p) => std::fs::read(p).map_err(|e| format!("{}: {e}", p.display())),
            At::Vpk { archive, offset, len, dir, preload } => {
                let mut b = if preload.1 > 0 { range(dir, preload.0, preload.1)? } else { vec![] };
                if *len > 0 {
                    b.extend(range(archive, *offset, *len)?);
                }
                Ok(b)
            }
            At::Gma { file, offset, len } => range(file, *offset, *len),
        }
    }

    pub fn exists(&self, path: &str) -> bool {
        self.files.contains_key(&norm(path))
    }

    /// Every model (`models/*.mdl`) and where it comes from, sorted.
    pub fn models(&self) -> Vec<(String, usize)> {
        let mut v: Vec<(String, usize)> = self.files.iter().filter(|(k, _)| k.ends_with(".mdl")).map(|(k, e)| (k.clone(), e.source)).collect();
        v.sort();
        v
    }

    /// The game's surface properties' scripts, as the engine's manifest lists them (`scripts/surfaceproperties_manifest.txt`).
    pub fn surface_scripts(&self) -> Vec<(String, String)> {
        let manifest = self.read("scripts/surfaceproperties_manifest.txt").map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
        let mut names: Vec<String> = manifest
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                l.to_ascii_lowercase().starts_with("\"file\"").then(|| l.split('"').nth(3).map(str::to_string)).flatten()
            })
            .collect();
        if names.is_empty() {
            names.push("scripts/surfaceproperties.txt".into());
        }
        names.into_iter().filter_map(|n| self.read(&n).ok().map(|b| (n, String::from_utf8_lossy(&b).into_owned()))).collect()
    }
}
