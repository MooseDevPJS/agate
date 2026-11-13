//! Modrinth content: mods (Fabric), resource packs and shader packs.
//! Install with required dependencies, prefer stable releases, list, remove,
//! enable/disable mods and activate one resource pack or shader.

use crate::app::Paths;
use crate::net::{self, client, Download};
use crate::profiles::{self, ModEntry, Profile};
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

const API: &str = "https://api.modrinth.com/v2";
const IRIS: &str = "YL57xq9U";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Mod,
    ResourcePack,
    Shader,
}

impl Kind {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "mod" => Kind::Mod,
            "resourcepack" => Kind::ResourcePack,
            "shader" => Kind::Shader,
            _ => bail!("unknown content type {s}"),
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Mod => "mod",
            Kind::ResourcePack => "resourcepack",
            Kind::Shader => "shader",
        }
    }
    fn folder(self) -> &'static str {
        match self {
            Kind::Mod => "mods",
            Kind::ResourcePack => "resourcepacks",
            Kind::Shader => "shaderpacks",
        }
    }
    /// Modrinth "loaders" accepted for this profile, or why the profile can't use this content.
    fn loaders(self, p: &Profile) -> Result<Vec<&'static str>> {
        Ok(match (self, p.loader.as_str()) {
            (Kind::Mod, "fabric") => vec!["fabric"],
            (Kind::Mod, _) => bail!("Mods need a Fabric profile"),
            (Kind::ResourcePack, _) => vec!["minecraft"],
            (Kind::Shader, "fabric") => vec!["iris", "optifine"],
            (Kind::Shader, "optifine") => vec!["optifine"],
            (Kind::Shader, _) => bail!("Shaders need a Fabric (Iris) or OptiFine profile"),
        })
    }
}

pub fn dir(paths: &Paths, p: &Profile, kind: Kind) -> PathBuf {
    paths.instance(&p.id).join(kind.folder())
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Hit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub author: String,
    #[serde(default)]
    pub gallery: Vec<String>,
    #[serde(default)]
    pub featured_gallery: Option<String>,
    #[serde(default)]
    pub color: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResult {
    pub hits: Vec<Hit>,
    pub total_hits: u64,
    pub offset: u64,
}

pub async fn search(p: &Profile, kind: Kind, query: &str, offset: u64) -> Result<SearchResult> {
    let loaders = kind.loaders(p)?;
    let categories: Vec<String> = loaders.iter().filter(|l| **l != "minecraft").map(|l| format!("categories:{l}")).collect();
    let mut facets = vec![vec![format!("versions:{}", p.mc)], vec![format!("project_type:{}", kind.as_str())]];
    if !categories.is_empty() {
        facets.push(categories);
    }
    let facets = serde_json::to_string(&facets)?;
    let resp = client()
        .get(format!("{API}/search"))
        .query(&[
            ("query", query),
            ("facets", &facets),
            ("limit", "20"),
            ("offset", &offset.to_string()),
            ("index", if query.trim().is_empty() { "downloads" } else { "relevance" }),
        ])
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("Modrinth search failed ({})", resp.status());
    }
    Ok(resp.json().await?)
}

#[derive(Deserialize, Clone)]
struct Version {
    id: String,
    project_id: String,
    version_number: String,
    #[serde(default)]
    version_type: String,
    #[serde(default)]
    date_published: String,
    files: Vec<VFile>,
    #[serde(default)]
    dependencies: Vec<Dependency>,
}
#[derive(Deserialize, Clone)]
struct VFile {
    url: String,
    filename: String,
    primary: bool,
    size: u64,
    hashes: Hashes,
}
#[derive(Deserialize, Clone)]
struct Hashes {
    sha1: Option<String>,
}
#[derive(Deserialize, Clone)]
struct Dependency {
    project_id: Option<String>,
    version_id: Option<String>,
    dependency_type: String,
}
#[derive(Deserialize)]
struct Project {
    title: String,
    icon_url: Option<String>,
    project_type: String,
}

/// Newest stable release for the profile's version; betas only when nothing stable exists.
/// `not_after` (a dependency's parent publish date) keeps a library at a version the parent
/// was released against: a newer Sodium beta once broke the newest Iris that required it.
async fn compatible_version(project: &str, mc: &str, loaders: &[&str], not_after: Option<&str>) -> Result<Version> {
    let loaders = serde_json::to_string(loaders)?;
    let games = serde_json::to_string(&[mc])?;
    let resp = client()
        .get(format!("{API}/project/{project}/version"))
        .query(&[("loaders", loaders.as_str()), ("game_versions", games.as_str())])
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("Modrinth returned {}", resp.status());
    }
    let mut list: Vec<Version> = resp.json().await?;
    if let Some(cap) = not_after {
        let capped: Vec<Version> = list.iter().filter(|v| v.date_published.as_str() <= cap).cloned().collect();
        if !capped.is_empty() {
            list = capped;
        }
    }
    let pick = list.iter().position(|v| v.version_type == "release").unwrap_or(0);
    list.into_iter().nth(pick).ok_or_else(|| anyhow!("No build for Minecraft {mc}"))
}

fn safe_file_name(name: &str) -> Result<&str> {
    if name.is_empty() || name.contains(['/', '\\', ':']) || name.starts_with('.') || name.contains("..") {
        bail!("refusing unsafe file name {name}");
    }
    Ok(name)
}

/// Installs `project` (and required dependencies) into the right folder of the profile.
/// Installing a shader into a Fabric profile without Iris also installs Iris.
/// Returns the titles that were installed.
pub async fn install(paths: &Paths, profile: &Profile, kind: Kind, project: &str) -> Result<Vec<String>> {
    kind.loaders(profile)?;
    let installed: Vec<ModEntry> = profiles::get(paths, &profile.id).map(|p| p.mods).unwrap_or_default();
    // (project, pinned version, kind, publish date of the mod that requires it)
    let mut queue: Vec<(String, Option<String>, Kind, Option<String>)> = vec![(project.to_string(), None, kind, None)];
    if kind == Kind::Shader && profile.loader == "fabric" && !installed.iter().any(|m| m.project_id == IRIS) {
        queue.insert(0, (IRIS.to_string(), None, Kind::Mod, None));
    }
    let mut added = Vec::new();
    let mut seen = HashSet::new();

    while let Some((pid, vid, k, cap)) = queue.pop() {
        if !seen.insert(pid.clone()) {
            continue;
        }
        // An installed dependency stays unless it is newer than the mod requiring it.
        if pid != project {
            if let Some(have) = installed.iter().find(|m| m.project_id == pid) {
                let keep = match (&vid, &cap) {
                    (Some(v), _) => *v == have.version_id,
                    (None, None) => true,
                    (None, Some(c)) => net::get_json::<Version>(&format!("{API}/version/{}", have.version_id))
                        .await
                        .map_or(true, |hv| hv.date_published.as_str() <= c.as_str()),
                };
                if keep {
                    continue;
                }
            }
        }
        let loaders = k.loaders(profile)?;
        let version: Version = match &vid {
            Some(v) => net::get_json(&format!("{API}/version/{v}")).await?,
            None => compatible_version(&pid, &profile.mc, &loaders, cap.as_deref()).await?,
        };
        let meta: Project = net::get_json(&format!("{API}/project/{}", version.project_id)).await?;
        // Dependencies keep their own type (a shader never pulls a mod, a mod may pull a library).
        let k = Kind::parse(&meta.project_type).unwrap_or(k);
        let file = version
            .files
            .iter()
            .find(|f| f.primary)
            .or(version.files.first())
            .ok_or_else(|| anyhow!("version has no files"))?;
        let name = safe_file_name(&file.filename)?;
        let folder = dir(paths, profile, k);
        std::fs::create_dir_all(&folder)?;
        net::fetch(
            &Download { url: file.url.clone(), path: folder.join(name), sha1: file.hashes.sha1.clone(), size: Some(file.size) },
            &Default::default(),
        )
        .await?;
        let entry = ModEntry {
            project_id: version.project_id.clone(),
            version_id: version.id.clone(),
            version_number: version.version_number.clone(),
            file: name.to_string(),
            title: meta.title.clone(),
            icon_url: meta.icon_url,
            kind: k.as_str().into(),
        };
        profiles::update(paths, &profile.id, |p| {
            p.mods.retain(|m| m.project_id != entry.project_id);
            p.mods.push(entry.clone());
        })?;
        if let Some(prev) = installed.iter().find(|m| m.project_id == entry.project_id && m.file != entry.file) {
            let _ = std::fs::remove_file(folder.join(&prev.file));
            let _ = std::fs::remove_file(folder.join(format!("{}.disabled", prev.file)));
        }
        added.push(meta.title);
        for d in version.dependencies.iter().filter(|d| d.dependency_type == "required") {
            if let Some(dp) = &d.project_id {
                queue.push((dp.clone(), d.version_id.clone(), Kind::Mod, Some(version.date_published.clone())));
            }
        }
    }
    Ok(added)
}

#[derive(Serialize)]
pub struct Installed {
    pub file: String,
    pub title: String,
    pub version: Option<String>,
    pub icon_url: Option<String>,
    pub project_id: Option<String>,
    /// Mods: not disabled. Resource packs / shaders: currently selected in game options.
    pub enabled: bool,
}

/// Every file in the content folder, enriched with Modrinth metadata when we installed it.
pub fn list(paths: &Paths, profile: &Profile, kind: Kind) -> Vec<Installed> {
    let active = match kind {
        Kind::Mod => vec![],
        Kind::ResourcePack => crate::content::active_resource_packs(paths, profile),
        Kind::Shader => crate::content::active_shader(paths, profile).into_iter().collect(),
    };
    let mut out: Vec<Installed> = std::fs::read_dir(dir(paths, profile, kind))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let (base, on) = match name.strip_suffix(".disabled") {
                Some(b) => (b.to_string(), false),
                None => (name.clone(), true),
            };
            let is_dir = e.path().is_dir();
            let ok = match kind {
                // The launcher's own cape mod is managed from Settings, not listed with the mods.
                Kind::Mod => base.ends_with(".jar") && !crate::cape::is_mod_file(&base),
                _ => base.ends_with(".zip") || is_dir,
            };
            if !ok {
                return None;
            }
            let meta = profile.mods.iter().find(|m| m.file == base);
            let enabled = if kind == Kind::Mod { on } else { active.contains(&base) };
            Some(Installed {
                title: meta.map(|m| m.title.clone()).unwrap_or_else(|| base.trim_end_matches(".jar").trim_end_matches(".zip").to_string()),
                version: meta.map(|m| m.version_number.clone()),
                icon_url: meta.and_then(|m| m.icon_url.clone()),
                project_id: meta.map(|m| m.project_id.clone()),
                file: base,
                enabled,
            })
        })
        .collect();
    out.sort_by_key(|m| m.title.to_lowercase());
    out
}

pub fn remove(paths: &Paths, profile: &Profile, kind: Kind, file: &str) -> Result<()> {
    let file = safe_file_name(file)?;
    let folder = dir(paths, profile, kind);
    let target = folder.join(file);
    if target.is_dir() {
        std::fs::remove_dir_all(&target)?;
    } else {
        let _ = std::fs::remove_file(&target);
    }
    let _ = std::fs::remove_file(folder.join(format!("{file}.disabled")));
    if kind == Kind::ResourcePack {
        crate::content::set_resource_pack(paths, profile, file, false)?;
    }
    if kind == Kind::Shader && crate::content::active_shader(paths, profile).as_deref() == Some(file) {
        crate::content::set_shader(paths, profile, None)?;
    }
    profiles::update(paths, &profile.id, |p| p.mods.retain(|m| m.file != file))?;
    Ok(())
}

/// Mods: enable/disable the jar. Resource packs: add/remove from the active list.
/// Shaders: select this one, or switch shaders off when it is already selected.
pub fn toggle(paths: &Paths, profile: &Profile, kind: Kind, file: &str) -> Result<()> {
    let file = safe_file_name(file)?;
    match kind {
        Kind::Mod => {
            let folder = dir(paths, profile, kind);
            let (on, off) = (folder.join(file), folder.join(format!("{file}.disabled")));
            if on.exists() {
                std::fs::rename(on, off)?;
            } else if off.exists() {
                std::fs::rename(off, on)?;
            }
        }
        Kind::ResourcePack => {
            let active = crate::content::active_resource_packs(paths, profile).iter().any(|f| f == file);
            crate::content::set_resource_pack(paths, profile, file, !active)?;
        }
        Kind::Shader => {
            let current = crate::content::active_shader(paths, profile);
            crate::content::set_shader(paths, profile, if current.as_deref() == Some(file) { None } else { Some(file) })?;
        }
    }
    Ok(())
}
