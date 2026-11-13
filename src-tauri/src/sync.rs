//! Shared game settings across profiles.
//!
//! `options.txt` (keybinds, video, sound, controls…) and `servers.dat` live in a
//! shared store. Before launch the shared values are merged into the profile;
//! after the game exits the profile's values are merged back, so a keybind
//! changed in one profile appears in every other one.
//!
//! Minecraft 1.13 changed the keybind format (numeric LWJGL2 codes → `key.keyboard.*`),
//! so pre-1.13 profiles share a separate legacy store.

use crate::app::Paths;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Keys that belong to one profile, not the player.
const PER_PROFILE: &[&str] = &["resourcePacks", "incompatibleResourcePacks", "lastServer"];

fn store(paths: &Paths, legacy: bool) -> PathBuf {
    paths.shared().join(if legacy { "options-legacy.txt" } else { "options.txt" })
}

/// 1.13 released 2018-07-18; anything before uses the legacy option format.
pub fn is_legacy(release_time: Option<&str>) -> bool {
    release_time.is_some_and(|t| t < "2018-07-18")
}

/// Legacy option format for an installed version id (falls back to modern when unknown).
pub fn legacy_version(paths: &Paths, mc: &str) -> bool {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct V {
        release_time: Option<String>,
    }
    std::fs::read(paths.version_json(mc))
        .ok()
        .and_then(|b| serde_json::from_slice::<V>(&b).ok())
        .is_some_and(|v| is_legacy(v.release_time.as_deref()))
}

fn parse(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// `base` updated with `incoming`: existing order kept, new keys appended.
/// `version` (the options DataVersion) keeps the higher value so newer games never re-run old fixers.
fn merge(base: &mut Vec<(String, String)>, incoming: &[(String, String)], skip_per_profile: bool) {
    for (k, v) in incoming {
        if skip_per_profile && PER_PROFILE.contains(&k.as_str()) {
            continue;
        }
        match base.iter_mut().find(|(bk, _)| bk == k) {
            Some((_, bv)) if k == "version" => {
                let (a, b) = (bv.parse::<u64>().unwrap_or(0), v.parse::<u64>().unwrap_or(0));
                if b > a {
                    *bv = v.clone();
                }
            }
            Some((_, bv)) => *bv = v.clone(),
            None => base.push((k.clone(), v.clone())),
        }
    }
}

fn render(entries: &[(String, String)]) -> String {
    entries.iter().map(|(k, v)| format!("{k}:{v}\n")).collect()
}

fn read(path: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(path).map(|t| parse(&t)).unwrap_or_default()
}

/// Shared → profile, before launch.
pub fn before_launch(paths: &Paths, game_dir: &Path, legacy: bool) -> Result<()> {
    std::fs::create_dir_all(game_dir)?;
    let shared = read(&store(paths, legacy));
    if !shared.is_empty() {
        let target = game_dir.join("options.txt");
        let mut local = read(&target);
        merge(&mut local, &shared, true);
        std::fs::write(&target, render(&local))?;
    }
    let servers = paths.shared().join("servers.dat");
    if servers.exists() {
        std::fs::copy(&servers, game_dir.join("servers.dat"))?;
    }
    Ok(())
}

/// Profile → shared, after the game exits.
pub fn after_exit(paths: &Paths, game_dir: &Path, legacy: bool) -> Result<()> {
    std::fs::create_dir_all(paths.shared())?;
    let local = read(&game_dir.join("options.txt"));
    if !local.is_empty() {
        let path = store(paths, legacy);
        let mut shared = read(&path);
        merge(&mut shared, &local, true);
        std::fs::write(&path, render(&shared))?;
    }
    let servers = game_dir.join("servers.dat");
    if servers.exists() {
        std::fs::copy(&servers, paths.shared().join("servers.dat"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_overrides_appends_and_keeps_newer_version() {
        let mut base = parse("version:3465\nfov:0.0\nkey_key.jump:key.keyboard.space\nresourcePacks:[\"a\"]\n");
        let incoming = parse("version:2975\nfov:0.5\nkey_key.sprint:key.keyboard.left.control\nresourcePacks:[\"b\"]\n");
        merge(&mut base, &incoming, true);
        let out = render(&base);
        assert!(out.contains("version:3465\n"));
        assert!(out.contains("fov:0.5\n"));
        assert!(out.contains("key_key.jump:key.keyboard.space\n"));
        assert!(out.contains("key_key.sprint:key.keyboard.left.control\n"));
        assert!(out.contains("resourcePacks:[\"a\"]\n"));
    }

    #[test]
    fn values_with_colons_survive() {
        let e = parse("lastServer:play.example.net:25565\n");
        assert_eq!(e[0].1, "play.example.net:25565");
    }

    #[test]
    fn legacy_cutoff() {
        assert!(is_legacy(Some("2017-09-18T08:39:46+00:00")));
        assert!(!is_legacy(Some("2018-07-18T15:11:46+00:00")));
    }
}
