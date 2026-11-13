//! Fabric loader: its meta server hands out a version JSON that inherits from vanilla.

use crate::app::Paths;
use crate::net;
use anyhow::{anyhow, Result};
use serde::Deserialize;

const META: &str = "https://meta.fabricmc.net/v2";

#[derive(Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}
#[derive(Deserialize)]
struct LoaderInfo {
    version: String,
    stable: bool,
}

#[derive(Deserialize)]
struct GameEntry {
    version: String,
}

/// Minecraft versions Fabric supports.
pub async fn game_versions() -> Result<Vec<String>> {
    let v: Vec<GameEntry> = net::get_json(&format!("{META}/versions/game")).await?;
    Ok(v.into_iter().map(|g| g.version).collect())
}

/// Latest stable loader for `mc`.
pub async fn latest_loader(mc: &str) -> Result<String> {
    let list: Vec<LoaderEntry> = net::get_json(&format!("{META}/versions/loader/{mc}")).await?;
    list.iter()
        .find(|l| l.loader.stable)
        .or(list.first())
        .map(|l| l.loader.version.clone())
        .ok_or_else(|| anyhow!("Fabric does not support Minecraft {mc} yet"))
}

/// Writes the Fabric version JSON and returns its id.
pub async fn ensure(paths: &Paths, mc: &str, loader: &str) -> Result<String> {
    let id = format!("fabric-loader-{loader}-{mc}");
    let path = paths.version_json(&id);
    if !path.exists() {
        let text = net::get_text(&format!("{META}/versions/loader/{mc}/{loader}/profile/json")).await?;
        let mut json: serde_json::Value = serde_json::from_str(&text)?;
        json["id"] = id.clone().into();
        crate::app::write_json(&path, &json)?;
    }
    Ok(id)
}
