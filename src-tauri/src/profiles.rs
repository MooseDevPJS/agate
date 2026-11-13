//! Profiles (instances) and launcher settings, persisted as JSON.

use crate::app::{read_json, write_json, Paths};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ModEntry {
    pub project_id: String,
    pub version_id: String,
    pub version_number: String,
    pub file: String,
    pub title: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    /// "mod" | "resourcepack" | "shader"
    #[serde(default = "mod_kind")]
    pub kind: String,
}

fn mod_kind() -> String {
    "mod".into()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub mc: String,
    /// "vanilla" | "fabric" | "optifine"
    pub loader: String,
    /// Fabric loader version or OptiFine file; None = latest at first launch.
    #[serde(default)]
    pub loader_version: Option<String>,
    pub memory_mb: u32,
    #[serde(default)]
    pub java_path: Option<String>,
    #[serde(default)]
    pub jvm_args: String,
    /// Chapter colour: "grass" | "amber" | "sky" | "redstone" | "amethyst".
    pub color: String,
    /// Pixel icon shown for the profile (see `ICONS`).
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default = "yes")]
    pub sync_settings: bool,
    pub created: i64,
    #[serde(default)]
    pub last_played: Option<i64>,
    #[serde(default)]
    pub mods: Vec<ModEntry>,
}

/// Profile icons drawn by the UI (ui/icons.js).
pub const ICONS: &[&str] = &["block", "grass", "gem", "pickaxe", "sword", "tree", "potion", "heart", "house", "flame"];

fn default_icon() -> String {
    "block".into()
}

fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    /// Optional Mojang-approved Azure app id for Microsoft login.
    #[serde(default)]
    pub azure_client_id: Option<String>,
    pub default_memory_mb: u32,
    #[serde(default)]
    pub show_snapshots: bool,
    /// Hide the launcher while the game runs, show it again when the game closes.
    #[serde(default)]
    pub close_on_launch: bool,
    /// Interface language: "it" or "en".
    #[serde(default = "default_language")]
    pub language: String,
    /// Show the custom cape (shared/cape.png) in game, Fabric 26.x profiles only.
    #[serde(default)]
    pub custom_cape: bool,
    /// Interface animations: "auto" (follow Windows), "on" or "off".
    #[serde(default = "default_motion")]
    pub motion: String,
}

fn default_motion() -> String {
    "auto".into()
}

fn default_language() -> String {
    "it".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self { azure_client_id: None, default_memory_mb: 4096, show_snapshots: false, close_on_launch: false, language: default_language(), custom_cape: false, motion: default_motion() }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Store {
    profiles: Vec<Profile>,
}

pub fn list(paths: &Paths) -> Vec<Profile> {
    read_json::<Store>(&paths.file("profiles.json")).profiles
}

fn save_all(paths: &Paths, profiles: Vec<Profile>) -> Result<()> {
    write_json(&paths.file("profiles.json"), &Store { profiles })
}

pub fn get(paths: &Paths, id: &str) -> Result<Profile> {
    list(paths).into_iter().find(|p| p.id == id).ok_or_else(|| anyhow!("profile not found"))
}

pub fn validate(p: &Profile) -> Result<()> {
    if p.name.trim().is_empty() || p.name.len() > 48 {
        return Err(anyhow!("Name must be 1–48 characters"));
    }
    if !["vanilla", "fabric", "optifine"].contains(&p.loader.as_str()) {
        return Err(anyhow!("Unknown loader"));
    }
    if !ICONS.contains(&p.icon.as_str()) {
        return Err(anyhow!("Unknown icon"));
    }
    if !(512..=65536).contains(&p.memory_mb) {
        return Err(anyhow!("Memory must be between 512 MB and 64 GB"));
    }
    // Version ids become directory names: allow only what Mojang ids use.
    let id_ok = p.mc.chars().all(|c| c.is_ascii_alphanumeric() || "._- ".contains(c));
    if p.mc.is_empty() || p.mc.len() > 64 || !id_ok || p.mc.contains("..") {
        return Err(anyhow!("Invalid Minecraft version"));
    }
    Ok(())
}

/// Insert or replace by id. Keeps the stored mod list: mods change only through mod commands.
pub fn upsert(paths: &Paths, mut p: Profile) -> Result<Profile> {
    validate(&p)?;
    p.name = p.name.trim().to_string();
    let mut all = list(paths);
    match all.iter_mut().find(|x| x.id == p.id) {
        Some(existing) => {
            // Changing version or loader invalidates installed mods' compatibility; keep files, user decides.
            p.mods = std::mem::take(&mut existing.mods);
            p.created = existing.created;
            p.last_played = existing.last_played;
            *existing = p.clone();
        }
        None => all.push(p.clone()),
    }
    std::fs::create_dir_all(paths.instance(&p.id))?;
    save_all(paths, all)?;
    Ok(p)
}

pub fn update<F: FnOnce(&mut Profile)>(paths: &Paths, id: &str, f: F) -> Result<Profile> {
    let mut all = list(paths);
    let p = all.iter_mut().find(|x| x.id == id).ok_or_else(|| anyhow!("profile not found"))?;
    f(p);
    let out = p.clone();
    save_all(paths, all)?;
    Ok(out)
}

pub fn delete(paths: &Paths, id: &str) -> Result<()> {
    let all: Vec<Profile> = list(paths).into_iter().filter(|p| p.id != id).collect();
    save_all(paths, all)?;
    let dir = paths.instance(id);
    if dir.starts_with(paths.instances()) && dir != paths.instances() {
        let _ = std::fs::remove_dir_all(dir);
    }
    let _ = std::fs::remove_dir_all(paths.natives(id));
    Ok(())
}

pub fn settings(paths: &Paths) -> Settings {
    let s: Option<Settings> = std::fs::read(paths.file("settings.json")).ok().and_then(|b| serde_json::from_slice(&b).ok());
    s.unwrap_or_default()
}

pub fn save_settings(paths: &Paths, s: &Settings) -> Result<()> {
    write_json(&paths.file("settings.json"), s)
}
