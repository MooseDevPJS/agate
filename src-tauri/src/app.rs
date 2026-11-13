//! Data locations, persisted JSON stores and progress reporting to the UI.

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new() -> Self {
        let root = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("Blocco");
        Self { root }
    }
    pub fn versions(&self) -> PathBuf { self.root.join("versions") }
    pub fn version_dir(&self, id: &str) -> PathBuf { self.versions().join(id) }
    pub fn version_json(&self, id: &str) -> PathBuf { self.version_dir(id).join(format!("{id}.json")) }
    pub fn version_jar(&self, id: &str) -> PathBuf { self.version_dir(id).join(format!("{id}.jar")) }
    pub fn libraries(&self) -> PathBuf { self.root.join("libraries") }
    pub fn assets(&self) -> PathBuf { self.root.join("assets") }
    pub fn runtimes(&self) -> PathBuf { self.root.join("runtimes") }
    pub fn instances(&self) -> PathBuf { self.root.join("instances") }
    pub fn instance(&self, id: &str) -> PathBuf { self.instances().join(id) }
    pub fn natives(&self, instance_id: &str) -> PathBuf { self.root.join("natives").join(instance_id) }
    pub fn shared(&self) -> PathBuf { self.root.join("shared") }
    pub fn cache(&self) -> PathBuf { self.root.join("cache") }
    pub fn file(&self, name: &str) -> PathBuf { self.root.join(name) }
}

pub fn read_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Atomic write: temp file then rename, so a crash never corrupts a store.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Sends task progress for one profile to the UI as `task` events.
#[derive(Clone)]
pub struct Reporter {
    app: AppHandle,
    profile: String,
}

#[derive(Serialize, Clone)]
struct TaskEvent<'a> {
    profile: &'a str,
    label: &'a str,
    done: u64,
    total: u64,
    bytes: u64,
    total_bytes: u64,
}

impl Reporter {
    pub fn new(app: AppHandle, profile: &str) -> Self {
        Self { app, profile: profile.to_string() }
    }
    pub fn stage(&self, label: &str) {
        self.progress(label, 0, 0, 0, 0);
    }
    pub fn progress(&self, label: &str, done: u64, total: u64, bytes: u64, total_bytes: u64) {
        let _ = self.app.emit(
            "task",
            TaskEvent { profile: &self.profile, label, done, total, bytes, total_bytes },
        );
    }
    pub fn progress_fn(&self, label: &str) -> crate::net::Progress {
        let me = self.clone();
        let label = label.to_string();
        Arc::new(move |d, t, b, tb| me.progress(&label, d, t, b, tb))
    }
    pub fn log(&self, line: &str) {
        #[derive(Serialize, Clone)]
        struct LogEvent<'a> { profile: &'a str, line: &'a str }
        let _ = self.app.emit("log", LogEvent { profile: &self.profile, line });
    }
}
