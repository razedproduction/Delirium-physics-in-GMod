//! dms-rig: the rig editor (ROADMAP M12.5, docs/adr/0031). A model of Garry's Mod or the Workshop drawn in its own
//! pose with NM's capsules and boxes fitted over it by the module's own fit (`dms_fit`, Homer from the player's GTA
//! IV); the rig edited by the mouse and saved as `garrysmod/data/dms/skeletons/<model>.json`, which the addon reloads
//! as it changes; and the fitted body simulated on the fork itself (`vphysics.dll`, VPhysics-Box3D).

mod app;
mod gamefs;
mod mesh;
mod render;
mod rigfile;
mod scene;
mod sim;

use std::path::{Path, PathBuf};

const USAGE: &str = "\
usage: dms-rig [--gmod <GarrysMod dir>] [--game <GTA IV dir>] [--vphysics <vphysics.dll>] [--model <models/...mdl>]
                [--select <part>] [--tool <select|move|rotate|scale|all>] [--screenshot <file.png> [--sim <steps>]]
  the rig editor: a model's NM rig made by the mouse, saved as garrysmod\\data\\dms\\skeletons\\<model>.json
  --gmod      Garry's Mod's folder (default: DMS_GMOD_DIR, else Steam's library from the registry)
  --game      GTA IV's folder (default: DMS_GAME_DIR, else garrysmod\\data\\dms\\game_dir.txt, else golden\\game_dir.txt)
  --vphysics  the fork's vphysics.dll (default: Garry's Mod's bin\\win64, else the repository's vphysics\\build)
  --model     a model to open at once
  --select    an NM part to select at once (Thigh_Left, Hand_Right, ...); --tool the gizmo's tool
  --screenshot  saves the window to a PNG once drawn and quits (with --sim: after that many steps on the fork)
  --classify <text>  prints the class of every model whose path or source has <text> (\"workshop\": all of its), no window
  --bones <model>    prints a model's bones (index, name, parent, mesh bind position, physics object)";

/// Homer from the player's GTA IV as the module loads him (`module/src/data.rs`): his build and body, NM's surfaces.
pub struct Homer {
    pub build: delirium::ragdoll::Build,
    pub body: dms_game::body::Body,
    /// NM's CONCRETE row (friction, elasticity), the Playroom's floor.
    pub concrete: [f32; 2],
}

fn load_homer(game: &Path) -> Result<Homer, String> {
    let g = dms_game::Game::open(game)?;
    let materials = g.materials();
    let rows = materials.as_deref().map(dms_game::surfaces).unwrap_or_default();
    if let Some(m) = materials.as_deref() {
        dms_game::set_surfaces(m);
    }
    let fred = g.fred()?;
    // SAFETY: `fred` is `len` bytes.
    if unsafe { delirium_physics::nm_init_memory(fred.as_ptr(), fred.len() as i32) } < 0 {
        return Err("nm_init_memory: fred.xrt didn't load".into());
    }
    let (build, frames) = g.homer()?;
    let body = dms_game::body::Body::new(&build, &frames);
    let concrete = delirium::ragdoll::SURFACES.read().ok().and_then(|r| r.get(dms_game::concrete(&rows) as usize).copied()).unwrap_or([1.0, 0.1]);
    Ok(Homer { build, body, concrete })
}

pub struct Options {
    pub gmod: PathBuf,
    pub repo: Option<PathBuf>,
    pub vphysics: Option<PathBuf>,
    pub model: Option<String>,
    pub screenshot: Option<(PathBuf, u64)>,
    pub select: Option<String>,
    pub tool: Option<String>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("dms-rig: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(());
    }
    let mut take = |flag: &str| {
        args.iter().position(|a| a == flag).map(|i| {
            let v = args.get(i + 1).cloned().unwrap_or_default();
            args.drain(i..(i + 2).min(args.len()));
            v
        })
    };
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(Path::parent).map(Path::to_path_buf).filter(|r| r.join("Cargo.toml").is_file());
    let read_dir = |p: PathBuf| std::fs::read_to_string(p).ok().map(|t| PathBuf::from(t.trim())).filter(|p| !p.as_os_str().is_empty());
    let gmod = take("--gmod").map(PathBuf::from).or_else(|| std::env::var_os("DMS_GMOD_DIR").map(PathBuf::from)).or_else(dms_homer::steam_gmod)
        .ok_or("Garry's Mod not found: pass --gmod <GarrysMod folder>")?;
    let game = take("--game")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("DMS_GAME_DIR").map(PathBuf::from))
        .or_else(|| read_dir(gmod.join("garrysmod").join("data").join("dms").join("game_dir.txt")))
        .or_else(|| repo.as_ref().and_then(|r| read_dir(r.join("golden").join("game_dir.txt"))))
        .ok_or("no GTA IV folder: pass --game <your GTA IV folder> (NM's body is Homer's, fitted)")?;
    let vphysics = take("--vphysics").map(PathBuf::from);
    let model = take("--model");
    let select = take("--select");
    let tool = take("--tool");
    let shot = take("--screenshot").map(PathBuf::from);
    let classify = take("--classify");
    let bones = take("--bones");
    let steps = take("--sim").map(|s| s.parse::<u64>().map_err(|e| format!("--sim {s}: {e}"))).transpose()?;
    if !args.is_empty() {
        return Err(format!("unknown arguments {args:?}\n{USAGE}"));
    }
    if let Some(model) = bones {
        let fs = gamefs::GameFs::open(&gmod)?;
        let s = dms_fit::skeleton::read_mdl(&fs.read(&model)?)?;
        for (i, b) in s.bones.iter().enumerate() {
            let parent = b.parent.map_or("-".to_string(), |p| s.bones[p].name.clone());
            let p = b.pose.0;
            println!("{i}\t{}\t{parent}\t{:.1} {:.1} {:.1}\tphysics {}", b.name, p.x, p.y, p.z, b.physics);
        }
        return Ok(());
    }
    if let Some(filter) = classify {
        // The models' classes as the module sees them, without a window: which need a rig made by hand.
        let fs = gamefs::GameFs::open(&gmod)?;
        for (p, s) in fs.models().into_iter().filter(|(p, s)| p.contains(&filter) || fs.sources[*s].name.contains(&filter)) {
            let c = fs.read(&p).and_then(|b| dms_fit::skeleton::read_mdl(&b)).and_then(|sk| {
                let at: Vec<_> = sk.bones.iter().map(|b| b.bind.0).collect();
                dms_fit::rigs::classify(&sk, None, &at).map(|c| if c.complete() { c.rig } else { format!("{}: {}", c.rig, c.why(&p)) })
            });
            println!("{p}\t{}\t{}", c.unwrap_or_else(|e| format!("error: {e}")), fs.sources[s].name);
        }
        return Ok(());
    }
    println!("dms-rig: Homer from {}", game.display());
    let homer = load_homer(&game)?;
    println!("dms-rig: Garry's Mod at {}", gmod.display());
    let fs = gamefs::GameFs::open(&gmod)?;
    let opts = Options { gmod, repo, vphysics, model, screenshot: shot.map(|s| (s, steps.unwrap_or(0))), select, tool };
    let native = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1440.0, 880.0]).with_title("DMS rig editor"),
        depth_buffer: 24,
        multisampling: 4,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native("dms-rig", native, Box::new(move |cc| Ok(Box::new(app::App::new(cc, homer, fs, opts)))))
        .map_err(|e| e.to_string())
}
