//! Java runtimes: one Eclipse Temurin JRE per major version, downloaded on demand.

use crate::app::{Paths, Reporter};
use crate::net::{self, Download};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

fn exe_name() -> &'static str {
    if cfg!(windows) { "javaw.exe" } else { "java" }
}

/// Adoptium never shipped a Temurin 16; 17 runs everything that asked for 16.
fn normalize(major: u32) -> u32 {
    match major {
        0..=8 => 8,
        16 => 17,
        m => m,
    }
}

fn find_exe(dir: &Path) -> Option<PathBuf> {
    let direct = dir.join("bin").join(exe_name());
    if direct.exists() {
        return Some(direct);
    }
    std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
        let p = e.path().join("bin").join(exe_name());
        p.exists().then_some(p)
    })
}

pub fn installed(paths: &Paths, major: u32) -> Option<PathBuf> {
    find_exe(&paths.runtimes().join(format!("java-{}", normalize(major))))
}

/// Path to a Java executable for `major`, downloading Temurin when missing.
pub async fn ensure(paths: &Paths, major: u32, rep: &Reporter) -> Result<PathBuf> {
    let major = normalize(major);
    if let Some(p) = installed(paths, major) {
        return Ok(p);
    }
    let (os, ext) = match std::env::consts::OS {
        "windows" => ("windows", "zip"),
        "macos" => ("mac", "tar.gz"),
        _ => ("linux", "tar.gz"),
    };
    if ext != "zip" {
        return Err(anyhow!("automatic Java download is only implemented on Windows; set a Java path in the profile"));
    }
    let arch = match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        _ => "x64",
    };
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{major}/ga/{os}/{arch}/jre/hotspot/normal/eclipse"
    );
    let archive = paths.cache().join(format!("temurin-{major}.zip"));
    rep.stage(&format!("Java {major}"));
    net::fetch_all(
        vec![Download { url, path: archive.clone(), sha1: None, size: None }],
        1,
        rep.progress_fn(&format!("Java {major}")),
    )
    .await
    .with_context(|| format!("downloading Java {major}"))?;

    rep.stage(&format!("Extracting Java {major}"));
    let dest = paths.runtimes().join(format!("java-{major}"));
    let staging = paths.runtimes().join(format!(".java-{major}-staging"));
    let (a, s) = (archive.clone(), staging.clone());
    tokio::task::spawn_blocking(move || -> Result<()> {
        let _ = std::fs::remove_dir_all(&s);
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&a)?)?;
        zip.extract(&s)?;
        Ok(())
    })
    .await??;
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::rename(&staging, &dest)?;
    let _ = std::fs::remove_file(&archive);
    find_exe(&dest).ok_or_else(|| anyhow!("Java {major} archive did not contain {}", exe_name()))
}
