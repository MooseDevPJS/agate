//! The custom cape: an image of the player's choosing, shown only to them in game by the
//! bundled "Blocco Cape" Fabric mod (Minecraft 26.x). Nothing is sent to Mojang.

use crate::app::Paths;
use crate::profiles::Profile;
use anyhow::{bail, Result};
use std::path::PathBuf;

/// The mod, built from `cape-mod/` with `cape-mod/build.sh`.
const MOD_JAR: &[u8] = include_bytes!("../assets/blocco-cape.jar");
const MOD_FILE: &str = "blocco-cape.jar";

pub fn image(paths: &Paths) -> PathBuf {
    paths.shared().join("cape.png")
}

/// The picture the cape was made from, kept so the cape can be edited again.
pub fn source(paths: &Paths) -> PathBuf {
    paths.shared().join("cape-source.png")
}

/// Editor options (fit, edge colour) as the UI saved them.
pub fn meta(paths: &Paths) -> PathBuf {
    paths.shared().join("cape.json")
}

/// The mod works on Fabric for Minecraft 26.x only.
pub fn supported(p: &Profile) -> bool {
    p.loader == "fabric" && p.mc.starts_with("26.")
}

fn write(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

/// Stores a cape texture (PNG, 2:1: the UI fits any picture to the template first),
/// with the picture it came from and the editor options.
pub fn save(paths: &Paths, png: &[u8], src: Option<&[u8]>, options: Option<&str>) -> Result<()> {
    match crate::auth::png_size(png) {
        Some((w, h)) if (64..=4096).contains(&w) && w == h * 2 && w % 64 == 0 => {}
        _ => bail!("The cape must be a PNG twice as wide as it is tall (64×32, 128×64…)"),
    }
    if let Some(s) = src {
        match crate::auth::png_size(s) {
            Some((w, h)) if w <= 8192 && h <= 8192 => {}
            _ => bail!("The source picture must be a PNG up to 8192×8192"),
        }
    }
    if options.is_some_and(|o| o.len() > 2048 || serde_json::from_str::<serde_json::Value>(o).is_err()) {
        bail!("invalid cape options");
    }
    std::fs::create_dir_all(paths.shared())?;
    write(&image(paths), png)?;
    match src {
        Some(s) => write(&source(paths), s)?,
        None => {
            let _ = std::fs::remove_file(source(paths));
        }
    }
    match options {
        Some(o) => write(&meta(paths), o.as_bytes())?,
        None => {
            let _ = std::fs::remove_file(meta(paths));
        }
    }
    Ok(())
}

pub fn remove(paths: &Paths) -> Result<()> {
    for p in [image(paths), source(paths), meta(paths)] {
        let _ = std::fs::remove_file(p);
    }
    Ok(())
}

pub fn is_mod_file(name: &str) -> bool {
    name == MOD_FILE || name == format!("{MOD_FILE}.disabled")
}

/// Before launch: install the mod and the image into the profile, or take both away.
pub fn apply(paths: &Paths, p: &Profile, enabled: bool) -> Result<()> {
    let dir = paths.instance(&p.id);
    let jar = dir.join("mods").join(MOD_FILE);
    let target = dir.join("config").join("blocco").join("cape.png");
    let source = image(paths);
    if enabled && supported(p) && source.exists() {
        std::fs::create_dir_all(dir.join("mods"))?;
        if std::fs::read(&jar).map_or(true, |b| b != MOD_JAR) {
            std::fs::write(&jar, MOD_JAR)?;
        }
        std::fs::create_dir_all(target.parent().unwrap())?;
        std::fs::copy(&source, &target)?;
    } else {
        let _ = std::fs::remove_file(&jar);
        let _ = std::fs::remove_file(&target);
    }
    Ok(())
}
