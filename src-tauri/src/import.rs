//! Import instances from other launchers (Modrinth App, CurseForge, Prism Launcher).
//! Nothing is moved: the game folder is copied into a new Blocco profile, the original
//! stays untouched. Only Vanilla and Fabric instances can be imported (what Blocco runs).

use crate::app::Paths;
use crate::profiles::{self, Profile};
use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Found {
    /// "modrinth" | "curseforge" | "prism"
    pub source: String,
    pub name: String,
    pub mc: String,
    /// "vanilla" | "fabric" | another loader Blocco cannot run (forge, neoforge, quilt…)
    pub loader: String,
    pub loader_version: Option<String>,
    /// The instance's game folder (saves, mods, options.txt…).
    pub path: String,
    pub supported: bool,
    pub worlds: usize,
    pub mods: usize,
}

fn count(dir: &Path, ext: Option<&str>) -> usize {
    std::fs::read_dir(dir)
        .map(|r| {
            r.flatten()
                .filter(|e| ext.map_or(e.path().is_dir(), |x| e.file_name().to_string_lossy().ends_with(x)))
                .count()
        })
        .unwrap_or(0)
}

fn found(source: &str, name: String, mc: String, loader: String, loader_version: Option<String>, game_dir: PathBuf) -> Found {
    let loader = loader.to_ascii_lowercase();
    Found {
        source: source.into(),
        supported: (loader == "vanilla" || loader == "fabric") && !mc.is_empty(),
        worlds: count(&game_dir.join("saves"), None),
        mods: count(&game_dir.join("mods"), Some(".jar")),
        name,
        mc,
        loader,
        loader_version: loader_version.filter(|v| !v.is_empty() && v != "None"),
        path: game_dir.display().to_string(),
    }
}

/* ---------- Modrinth App ---------- */

fn modrinth() -> Vec<Found> {
    let Some(root) = dirs::data_dir().map(|d| d.join("ModrinthApp")) else { return vec![] };
    let db = root.join("app.db");
    if !db.exists() {
        return vec![];
    }
    let read = || -> Result<Vec<Found>> {
        let c = rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        // Current schema: instances + their applied content set. Older apps had a `profiles` table.
        let new_schema = "SELECT i.name, i.path, s.game_version, s.loader, s.loader_version
             FROM instances i JOIN instance_content_sets s ON s.id = i.applied_content_set_id";
        let old_schema = "SELECT name, path, game_version, mod_loader, mod_loader_version FROM profiles";
        let mut stmt = c.prepare(new_schema).or_else(|_| c.prepare(old_schema))?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                r.get::<_, Option<String>>(3)?.unwrap_or_else(|| "vanilla".into()),
                r.get::<_, Option<String>>(4)?,
            ))
        })?;
        Ok(rows
            .flatten()
            .map(|(name, path, mc, loader, lv)| found("modrinth", name, mc, loader, lv, root.join("profiles").join(path)))
            .filter(|f| Path::new(&f.path).is_dir())
            .collect())
    };
    read().unwrap_or_default()
}

/* ---------- CurseForge ---------- */

fn curseforge() -> Vec<Found> {
    let Some(root) = dirs::home_dir().map(|h| h.join("curseforge").join("minecraft").join("Instances")) else { return vec![] };
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let dir = e.path();
            let v: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("minecraftinstance.json")).ok()?).ok()?;
            let name = v["name"].as_str().map(String::from).unwrap_or_else(|| e.file_name().to_string_lossy().into());
            let mc = v["gameVersion"].as_str().unwrap_or_default().to_string();
            // baseModLoader.name looks like "fabric-0.18.4-1.21.11" or "forge-47.2.0"; null means vanilla.
            let (loader, lv) = match v["baseModLoader"]["name"].as_str() {
                Some(n) => {
                    let mut parts = n.splitn(3, '-');
                    (parts.next().unwrap_or("").to_string(), parts.next().map(String::from))
                }
                None => ("vanilla".to_string(), None),
            };
            Some(found("curseforge", name, mc, loader, lv, dir))
        })
        .collect()
}

/* ---------- Prism Launcher ---------- */

/// Reads one Prism (or MultiMC-style) instance folder.
fn prism_instance(dir: &Path) -> Option<Found> {
    let pack: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("mmc-pack.json")).ok()?).ok()?;
    let cfg = std::fs::read_to_string(dir.join("instance.cfg")).unwrap_or_default();
    let name = cfg
        .lines()
        .find_map(|l| l.strip_prefix("name="))
        .map(|n| n.trim().to_string())
        .unwrap_or_else(|| dir.file_name().unwrap_or_default().to_string_lossy().into());
    let comps = pack["components"].as_array().cloned().unwrap_or_default();
    let version = |uid: &str| comps.iter().find(|c| c["uid"] == uid).and_then(|c| c["version"].as_str()).map(String::from);
    let mc = version("net.minecraft").unwrap_or_default();
    let (loader, lv) = [
        ("net.fabricmc.fabric-loader", "fabric"),
        ("org.quiltmc.quilt-loader", "quilt"),
        ("net.minecraftforge", "forge"),
        ("net.neoforged", "neoforge"),
    ]
    .iter()
    .find_map(|(uid, l)| version(uid).map(|v| (l.to_string(), Some(v))))
    .unwrap_or(("vanilla".into(), None));
    let game_dir = [".minecraft", "minecraft"].iter().map(|d| dir.join(d)).find(|d| d.is_dir()).unwrap_or_else(|| dir.join(".minecraft"));
    Some(found("prism", name, mc, loader, lv, game_dir))
}

fn prism() -> Vec<Found> {
    let Some(root) = dirs::data_dir().map(|d| d.join("PrismLauncher").join("instances")) else { return vec![] };
    std::fs::read_dir(root).into_iter().flatten().flatten().filter_map(|e| prism_instance(&e.path())).collect()
}

/// Every instance found on this computer, supported or not.
pub fn scan() -> Vec<Found> {
    let mut all = modrinth();
    all.extend(curseforge());
    all.extend(prism());
    all
}

/* ---------- copy ---------- */

/// Launcher bookkeeping and caches that a Blocco profile does not need.
const SKIP: &[&str] = &[
    "logs", "crash-reports", ".fabric", ".cache", "cache", "natives", "bin", "downloads", ".mixin.out",
    "minecraftinstance.json", "instance.cfg", "mmc-pack.json", "profile.json", "usercache.json", "usernamecache.json",
];

fn copy_dir(from: &Path, to: &Path, top: bool) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let name = e.file_name();
        if top && SKIP.iter().any(|s| name.eq_ignore_ascii_case(s)) {
            continue;
        }
        let ty = e.file_type()?;
        if ty.is_symlink() {
            continue; // never follow links out of the instance
        }
        let dest = to.join(&name);
        if ty.is_dir() {
            copy_dir(&e.path(), &dest, false)?;
        } else {
            std::fs::copy(e.path(), &dest).with_context(|| format!("copying {}", e.path().display()))?;
        }
    }
    Ok(())
}

/// Imports one instance found by `scan` (looked up again by path: only real instances are copied).
pub fn import(paths: &Paths, path: &str, color: &str, icon: &str) -> Result<Profile> {
    let f = scan().into_iter().find(|f| f.path == path).ok_or_else(|| anyhow!("Instance not found"))?;
    if !f.supported {
        bail!("{}: Blocco runs Vanilla and Fabric only", f.name);
    }
    let settings = profiles::settings(paths);
    let id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
    let name: String = f.name.chars().take(48).collect();
    let profile = Profile {
        id: id.clone(),
        name: if name.trim().is_empty() { "Importato".into() } else { name },
        mc: f.mc.clone(),
        loader: f.loader.clone(),
        loader_version: if f.loader == "fabric" { f.loader_version.clone() } else { None },
        memory_mb: settings.default_memory_mb,
        java_path: None,
        jvm_args: String::new(),
        color: color.into(),
        icon: icon.into(),
        sync_settings: true,
        created: chrono::Utc::now().timestamp(),
        last_played: None,
        mods: vec![],
    };
    profiles::validate(&profile)?;
    let dest = paths.instance(&id);
    if let Err(e) = copy_dir(Path::new(&f.path), &dest, true) {
        let _ = std::fs::remove_dir_all(&dest);
        return Err(e);
    }
    profiles::upsert(paths, profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prism_instance_is_read() {
        let dir = std::env::temp_dir().join(format!("blocco-prism-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".minecraft/saves/World 1")).unwrap();
        std::fs::create_dir_all(dir.join(".minecraft/mods")).unwrap();
        std::fs::write(dir.join(".minecraft/mods/sodium.jar"), b"").unwrap();
        std::fs::write(dir.join("instance.cfg"), "[General]\nname=My Pack\nJoinServerOnLaunch=false\n").unwrap();
        std::fs::write(
            dir.join("mmc-pack.json"),
            r#"{"components":[{"uid":"org.lwjgl3","version":"3.3.3"},{"uid":"net.minecraft","version":"1.21.4"},{"uid":"net.fabricmc.fabric-loader","version":"0.16.10"}],"formatVersion":1}"#,
        )
        .unwrap();
        let f = prism_instance(&dir).unwrap();
        assert_eq!((f.name.as_str(), f.mc.as_str(), f.loader.as_str()), ("My Pack", "1.21.4", "fabric"));
        assert_eq!(f.loader_version.as_deref(), Some("0.16.10"));
        assert!(f.supported);
        assert_eq!((f.worlds, f.mods), (1, 1));
        std::fs::write(dir.join("mmc-pack.json"), r#"{"components":[{"uid":"net.minecraft","version":"1.20.1"},{"uid":"net.minecraftforge","version":"47.2.0"}]}"#).unwrap();
        let f = prism_instance(&dir).unwrap();
        assert_eq!(f.loader, "forge");
        assert!(!f.supported);
        let _ = std::fs::remove_dir_all(dir);
    }
}
