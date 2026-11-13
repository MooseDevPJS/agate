//! OptiFine: fetched from optifine.net, patched against the vanilla client with
//! OptiFine's own `optifine.Patcher`, then wired up as a launchwrapper version.

use crate::app::{write_json, Paths};
use crate::mojang;
use crate::net::{self, Download};
use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Serialize, Clone)]
pub struct Build {
    /// Download file name, e.g. `OptiFine_1.20.1_HD_U_I6.jar` or `preview_OptiFine_26.2_HD_U_K2_pre5.jar`.
    pub file: String,
    /// Human label, e.g. `HD U I6` or `HD U K2 pre5`.
    pub label: String,
    pub mc: String,
    /// Preview (beta) build: for many recent versions these are the only builds.
    pub preview: bool,
}

impl Build {
    fn from_file(file: &str) -> Option<Self> {
        let (preview, rest) = match file.strip_prefix("preview_") {
            Some(r) => (true, r),
            None => (false, file),
        };
        let stem = rest.strip_prefix("OptiFine_")?.strip_suffix(".jar")?;
        let (mc, edition) = stem.split_once("_HD")?;
        Some(Self { file: file.into(), label: format!("HD{}", edition.replace('_', " ")), mc: mc.into(), preview })
    }
    /// `1.20.1_HD_U_I6`, used for the library coordinate.
    fn version(&self) -> String {
        self.file.trim_start_matches("preview_").trim_start_matches("OptiFine_").trim_end_matches(".jar").to_string()
    }
}

type BuildMap = BTreeMap<String, Vec<Build>>;

/// All builds per Minecraft version: stable first (newest first), then previews.
/// The download page is ~200 KB, so it is fetched at most every 30 minutes.
pub async fn builds() -> Result<BuildMap> {
    static CACHE: Mutex<Option<(Instant, BuildMap)>> = Mutex::new(None);
    if let Some((at, map)) = CACHE.lock().unwrap().as_ref() {
        if at.elapsed() < Duration::from_secs(1800) {
            return Ok(map.clone());
        }
    }
    let html = net::get_text("https://optifine.net/downloads").await?;
    let re = Regex::new(r"adloadx\?f=((?:preview_)?OptiFine_[A-Za-z0-9_.\-]+\.jar)").unwrap();
    let mut out: BuildMap = BTreeMap::new();
    for cap in re.captures_iter(&html) {
        if let Some(b) = Build::from_file(&cap[1]) {
            let list = out.entry(b.mc.clone()).or_default();
            if !list.iter().any(|x| x.file == b.file) {
                list.push(b);
            }
        }
    }
    if out.is_empty() {
        bail!("optifine.net did not list any build");
    }
    for list in out.values_mut() {
        list.sort_by_key(|b| b.preview); // stable sort keeps page order (newest first) within each group
    }
    *CACHE.lock().unwrap() = Some((Instant::now(), out.clone()));
    Ok(out)
}

pub fn version_id(mc: &str, file: &str) -> Result<String> {
    let b = Build::from_file(file).ok_or_else(|| anyhow!("unrecognised OptiFine build {file}"))?;
    Ok(format!("{mc}-OptiFine{}", b.version().trim_start_matches(mc)))
}

async fn download(file: &str, dest: &Path) -> Result<()> {
    if dest.exists() {
        return Ok(());
    }
    let page = net::get_text(&format!("https://optifine.net/adloadx?f={file}")).await?;
    let re = Regex::new(r#"downloadx\?f=[^'"\s>]+"#).unwrap();
    let link = re.find(&page).ok_or_else(|| anyhow!("OptiFine download link not found"))?.as_str();
    net::fetch(
        &Download { url: format!("https://optifine.net/{}", link.replace("&amp;", "&")), path: dest.into(), sha1: None, size: None },
        &Default::default(),
    )
    .await
}

/// Installs OptiFine for `mc` (vanilla must already be installed) and returns the version id.
pub async fn ensure(paths: &Paths, mc: &str, file: &str, java: &Path) -> Result<String> {
    let build = Build::from_file(file).ok_or_else(|| anyhow!("unrecognised OptiFine build {file}"))?;
    if build.mc != mc {
        bail!("{file} is for Minecraft {}, not {mc}", build.mc);
    }
    let id = version_id(mc, file)?;
    if paths.version_json(&id).exists() {
        return Ok(id);
    }
    let ver = build.version();
    let installer = paths.cache().join(file);
    download(file, &installer).await?;

    let libs = paths.libraries();
    let of_rel = mojang::maven_path(&format!("optifine:OptiFine:{ver}"))?;
    let of_lib = libs.join(&of_rel);
    std::fs::create_dir_all(of_lib.parent().unwrap())?;
    let client = paths.version_jar(mc);
    let mut cmd = std::process::Command::new(java);
    cmd.arg("-cp").arg(&installer).arg("optifine.Patcher").arg(&client).arg(&installer).arg(&of_lib);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = tokio::task::spawn_blocking(move || cmd.output()).await??;
    if !out.status.success() || !of_lib.exists() {
        bail!("OptiFine patcher failed: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or(""));
    }

    // Launchwrapper: modern builds ship their own fork inside the installer jar.
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&installer)?)?;
    let lw_version = zip.by_name("launchwrapper-of.txt").ok().and_then(|mut e| {
        let mut s = String::new();
        e.read_to_string(&mut s).ok().map(|_| s.trim().to_string())
    });
    let mut libraries = vec![json!({ "name": format!("optifine:OptiFine:{ver}"), "downloads": { "artifact": { "path": of_rel, "url": "" } } })];
    match lw_version {
        Some(v) => {
            let rel = mojang::maven_path(&format!("optifine:launchwrapper-of:{v}"))?;
            let dst = libs.join(&rel);
            std::fs::create_dir_all(dst.parent().unwrap())?;
            let mut entry = zip.by_name(&format!("launchwrapper-of-{v}.jar")).context("launchwrapper jar missing")?;
            std::io::copy(&mut entry, &mut std::fs::File::create(&dst)?)?;
            libraries.push(json!({ "name": format!("optifine:launchwrapper-of:{v}"), "downloads": { "artifact": { "path": rel, "url": "" } } }));
        }
        None => libraries.push(json!({ "name": "net.minecraft:launchwrapper:1.12" })),
    }

    let parent: serde_json::Value = serde_json::from_slice(&std::fs::read(paths.version_json(mc))?)?;
    let mut version = json!({
        "id": id,
        "inheritsFrom": mc,
        "type": "release",
        "mainClass": "net.minecraft.launchwrapper.Launch",
        "libraries": libraries,
    });
    if let Some(old) = parent.get("minecraftArguments").and_then(|v| v.as_str()) {
        version["minecraftArguments"] = format!("{old} --tweakClass optifine.OptiFineTweaker").into();
    } else {
        version["arguments"] = json!({ "game": ["--tweakClass", "optifine.OptiFineTweaker"] });
    }
    write_json(&paths.version_json(&id), &version)?;
    Ok(id)
}
