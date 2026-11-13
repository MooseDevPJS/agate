//! Mojang version metadata: manifest, version JSON (with `inheritsFrom` merging),
//! and installation of client jar, libraries, natives, assets and log config.

use crate::app::{read_json, write_json, Paths, Reporter};
use crate::net::{self, Download};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const LIBRARIES_URL: &str = "https://libraries.minecraft.net/";
const RESOURCES_URL: &str = "https://resources.download.minecraft.net/";

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Manifest {
    pub latest: Latest,
    pub versions: Vec<ManifestEntry>,
}
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub release_time: String,
    pub sha1: Option<String>,
}

/// Fresh manifest from Mojang, falling back to the cached copy when offline.
pub async fn manifest(paths: &Paths) -> Result<Manifest> {
    let cache = paths.cache().join("version_manifest_v2.json");
    match net::get_json::<Manifest>(MANIFEST_URL).await {
        Ok(m) => {
            let _ = write_json(&cache, &m);
            Ok(m)
        }
        Err(e) => {
            let cached: Manifest = read_json(&cache);
            if cached.versions.is_empty() { Err(e) } else { Ok(cached) }
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    pub inherits_from: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub main_class: Option<String>,
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    pub asset_index: Option<AssetIndexRef>,
    pub assets: Option<String>,
    pub downloads: Option<ClientDownloads>,
    pub java_version: Option<JavaVersion>,
    pub logging: Option<Logging>,
    pub release_time: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Value>,
    #[serde(default)]
    pub jvm: Vec<Value>,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibDownloads>,
    pub url: Option<String>,
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<HashMap<String, String>>,
    pub extract: Option<Extract>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct LibDownloads {
    pub artifact: Option<Artifact>,
    pub classifiers: Option<HashMap<String, Artifact>>,
}
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Artifact {
    pub path: Option<String>,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>,
}
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
}
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct ClientDownloads {
    pub client: Option<Artifact>,
}
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub major_version: u32,
}
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Logging {
    pub client: Option<LoggingClient>,
}
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct LoggingClient {
    pub argument: String,
    pub file: LogFile,
}
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct LogFile {
    pub id: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

pub fn os_name() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "osx",
        _ => "linux",
    }
}

/// Mojang rule semantics: last matching rule wins; no rules means allowed.
/// Feature-gated rules (demo, custom resolution, quick play) never match.
pub fn rules_allow(rules: Option<&Vec<Rule>>) -> bool {
    let Some(rules) = rules else { return true };
    let mut allowed = false;
    for r in rules {
        let os_ok = r.os.as_ref().map_or(true, |os| {
            os.name.as_deref().map_or(true, |n| n == os_name())
                && os.arch.as_deref().map_or(true, |a| a == "x86" && cfg!(target_arch = "x86"))
        });
        let features_ok = r.features.as_ref().map_or(true, |f| f.values().all(|v| !v));
        if os_ok && features_ok {
            allowed = r.action == "allow";
        }
    }
    allowed
}

/// `group:artifact:version[:classifier][@ext]` → relative maven path.
pub fn maven_path(name: &str) -> Result<String> {
    let (coords, ext) = match name.split_once('@') {
        Some((c, e)) => (c, e),
        None => (name, "jar"),
    };
    let parts: Vec<&str> = coords.split(':').collect();
    if parts.len() < 3 {
        return Err(anyhow!("bad library name {name}"));
    }
    let (group, artifact, version) = (parts[0].replace('.', "/"), parts[1], parts[2]);
    let file = match parts.get(3) {
        Some(c) => format!("{artifact}-{version}-{c}.{ext}"),
        None => format!("{artifact}-{version}.{ext}"),
    };
    Ok(format!("{group}/{artifact}/{version}/{file}"))
}

/// Dedup key: everything except the version, so a child's library replaces the parent's.
fn library_key(name: &str) -> String {
    let parts: Vec<&str> = name.split('@').next().unwrap_or(name).split(':').collect();
    match parts.as_slice() {
        [g, a, _, c, ..] => format!("{g}:{a}:{c}"),
        [g, a, ..] => format!("{g}:{a}"),
        _ => name.to_string(),
    }
}

/// Fetch and cache a vanilla version JSON.
pub async fn ensure_vanilla_json(paths: &Paths, id: &str) -> Result<()> {
    let path = paths.version_json(id);
    if path.exists() {
        return Ok(());
    }
    let manifest = manifest(paths).await?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == id)
        .ok_or_else(|| anyhow!("Minecraft {id} not found in the version manifest"))?;
    net::fetch(
        &Download { url: entry.url.clone(), path, sha1: entry.sha1.clone(), size: None },
        &Default::default(),
    )
    .await
}

fn load_json(paths: &Paths, id: &str) -> Result<VersionJson> {
    let p = paths.version_json(id);
    let bytes = std::fs::read(&p).with_context(|| format!("reading {}", p.display()))?;
    Ok(serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", p.display()))?)
}

/// A version with its whole `inheritsFrom` chain merged, ready to install and launch.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub id: String,
    pub jar_id: String,
    pub json: VersionJson,
}

pub async fn resolve(paths: &Paths, id: &str) -> Result<Resolved> {
    let mut chain = vec![load_json(paths, id)?];
    while let Some(parent) = chain.last().unwrap().inherits_from.clone() {
        if chain.len() > 8 {
            return Err(anyhow!("inheritsFrom chain too deep"));
        }
        ensure_vanilla_json(paths, &parent).await?;
        chain.push(load_json(paths, &parent)?);
    }
    let base = chain.pop().unwrap();
    let mut jar_id = base.id.clone();
    let mut merged = base;
    while let Some(child) = chain.pop() {
        if child.downloads.as_ref().and_then(|d| d.client.as_ref()).is_some() {
            jar_id = child.id.clone();
        }
        // Child libraries first (they take precedence on the classpath), parent's without duplicates.
        let child_keys: HashSet<String> = child.libraries.iter().map(|l| library_key(&l.name)).collect();
        let mut libs = child.libraries.clone();
        libs.extend(merged.libraries.into_iter().filter(|l| !child_keys.contains(&library_key(&l.name))));
        merged.libraries = libs;
        if let Some(a) = child.arguments {
            let base_args = merged.arguments.get_or_insert_with(Default::default);
            base_args.game.extend(a.game);
            base_args.jvm.extend(a.jvm);
        }
        merged.id = child.id;
        merged.main_class = child.main_class.or(merged.main_class);
        merged.minecraft_arguments = child.minecraft_arguments.or(merged.minecraft_arguments);
        merged.kind = child.kind.or(merged.kind);
        merged.inherits_from = None;
    }
    Ok(Resolved { id: merged.id.clone(), jar_id, json: merged })
}

/// Everything the launch step needs after installation.
pub struct Installed {
    pub classpath: Vec<PathBuf>,
    pub assets_root: PathBuf,
    pub game_assets: PathBuf,
    pub asset_index: String,
    pub log_config: Option<(String, PathBuf)>,
}

#[derive(Deserialize, Default)]
struct AssetIndex {
    objects: HashMap<String, AssetObject>,
    #[serde(default)]
    r#virtual: bool,
    #[serde(default)]
    map_to_resources: bool,
}
#[derive(Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

pub async fn install(
    paths: &Paths,
    v: &Resolved,
    natives_dir: &Path,
    game_dir: &Path,
    rep: &Reporter,
) -> Result<Installed> {
    let json = &v.json;
    let libs_dir = paths.libraries();
    let mut downloads = Vec::new();
    let mut classpath = Vec::new();
    let mut natives = Vec::new();

    // Client jar.
    let jar_json = if v.jar_id == v.id { json.clone() } else { load_json(paths, &v.jar_id)? };
    let client = jar_json
        .downloads
        .and_then(|d| d.client)
        .ok_or_else(|| anyhow!("version {} has no client download", v.jar_id))?;
    let client_jar = paths.version_jar(&v.jar_id);
    downloads.push(Download { url: client.url, path: client_jar.clone(), sha1: client.sha1, size: client.size });

    // Libraries.
    for lib in &json.libraries {
        if !rules_allow(lib.rules.as_ref()) {
            continue;
        }
        if let Some(natives_map) = &lib.natives {
            // Legacy natives: a classifier jar extracted into the natives directory.
            let Some(key) = natives_map.get(os_name()) else { continue };
            let key = key.replace("${arch}", if cfg!(target_pointer_width = "64") { "64" } else { "32" });
            if let Some(a) = lib.downloads.as_ref().and_then(|d| d.classifiers.as_ref()).and_then(|c| c.get(&key)) {
                let rel = a.path.clone().map_or_else(|| maven_path(&format!("{}:{key}", lib.name)), Ok)?;
                let path = libs_dir.join(rel);
                downloads.push(Download { url: a.url.clone(), path: path.clone(), sha1: a.sha1.clone(), size: a.size });
                natives.push((path, lib.extract.clone().unwrap_or_default().exclude));
            }
            if lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()).is_none() {
                continue;
            }
        }
        let artifact = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref());
        let rel = match artifact.and_then(|a| a.path.clone()) {
            Some(p) => p,
            None => maven_path(&lib.name)?,
        };
        let path = libs_dir.join(&rel);
        match artifact {
            Some(a) if !a.url.is_empty() => {
                downloads.push(Download { url: a.url.clone(), path: path.clone(), sha1: a.sha1.clone(), size: a.size })
            }
            Some(_) => {} // Local-only artifact (e.g. OptiFine), produced by its installer.
            None => {
                if let Some(base) = &lib.url {
                    let base = if base.ends_with('/') { base.clone() } else { format!("{base}/") };
                    downloads.push(Download { url: format!("{base}{rel}"), path: path.clone(), sha1: lib.sha1.clone(), size: lib.size });
                } else if !path.exists() {
                    downloads.push(Download { url: format!("{LIBRARIES_URL}{rel}"), path: path.clone(), sha1: None, size: None });
                }
            }
        }
        if !classpath.contains(&path) {
            classpath.push(path);
        }
    }
    classpath.push(client_jar);

    // Log4j config (also carries the Log4Shell mitigation for old versions).
    let mut log_config = None;
    if let Some(lc) = json.logging.as_ref().and_then(|l| l.client.as_ref()) {
        let path = paths.assets().join("log_configs").join(&lc.file.id);
        downloads.push(Download { url: lc.file.url.clone(), path: path.clone(), sha1: lc.file.sha1.clone(), size: lc.file.size });
        log_config = Some((lc.argument.clone(), path));
    }

    rep.stage("Libraries");
    net::fetch_all(downloads, 12, rep.progress_fn("Libraries")).await?;

    // Natives (legacy versions only; modern LWJGL extracts itself from the classpath).
    if !natives.is_empty() {
        std::fs::create_dir_all(natives_dir)?;
        for (jar, exclude) in &natives {
            extract_natives(jar, natives_dir, exclude)?;
        }
    }

    // Assets.
    let idx = json.asset_index.clone().ok_or_else(|| anyhow!("version has no asset index"))?;
    let assets_root = paths.assets();
    let idx_path = assets_root.join("indexes").join(format!("{}.json", idx.id));
    rep.stage("Assets");
    net::fetch_all(
        vec![Download { url: idx.url.clone(), path: idx_path.clone(), sha1: idx.sha1.clone(), size: idx.size }],
        1,
        rep.progress_fn("Assets"),
    )
    .await?;
    let index: AssetIndex = serde_json::from_slice(&std::fs::read(&idx_path)?)?;
    let objects_dir = assets_root.join("objects");
    let asset_downloads: Vec<Download> = index
        .objects
        .values()
        .map(|o| Download {
            url: format!("{RESOURCES_URL}{}/{}", &o.hash[..2], o.hash),
            path: objects_dir.join(&o.hash[..2]).join(&o.hash),
            sha1: Some(o.hash.clone()),
            size: Some(o.size),
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    net::fetch_all(asset_downloads, 24, rep.progress_fn("Assets")).await?;

    // Pre-1.7 versions read assets by name, not hash.
    let mut game_assets = assets_root.clone();
    if index.r#virtual || index.map_to_resources {
        let target = if index.map_to_resources {
            game_dir.join("resources")
        } else {
            assets_root.join("virtual").join(&idx.id)
        };
        for (name, o) in &index.objects {
            let dst = target.join(name);
            if !dst.exists() {
                if let Some(p) = dst.parent() {
                    std::fs::create_dir_all(p)?;
                }
                std::fs::copy(objects_dir.join(&o.hash[..2]).join(&o.hash), &dst)?;
            }
        }
        game_assets = target;
    }

    Ok(Installed { classpath, assets_root, game_assets, asset_index: idx.id, log_config })
}

impl std::hash::Hash for Download {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state)
    }
}
impl PartialEq for Download {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}
impl Eq for Download {}

fn extract_natives(jar: &Path, dest: &Path, exclude: &[String]) -> Result<()> {
    let file = std::fs::File::open(jar).with_context(|| format!("opening {}", jar.display()))?;
    let mut zip = zip::ZipArchive::new(file)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().to_string();
        if entry.is_dir() || name.starts_with("META-INF") || exclude.iter().any(|e| name.starts_with(e)) {
            continue;
        }
        let Some(rel) = entry.enclosed_name() else { continue };
        let out = dest.join(rel);
        if out.exists() {
            continue;
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p)?;
        }
        let mut f = std::fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut f)?;
    }
    Ok(())
}
