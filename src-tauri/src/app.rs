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
        let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        Self { root: data_root(&base) }
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

/// The data folder, `%APPDATA%\Agate`. Agate was called Blocco before 0.2.0: on the first
/// start its `%APPDATA%\Blocco` is renamed in place (instant, nothing is copied), so profiles,
/// worlds, accounts and settings carry over. If the rename is not possible yet (a file is still
/// open, e.g. by a game the old app started), the old folder keeps being used and the rename is
/// tried again at the next start: data is never split between the two.
pub fn data_root(base: &Path) -> PathBuf {
    let root = base.join("Agate");
    let old = base.join("Blocco");
    let ours = old.join("profiles.json").is_file() || old.join("instances").is_dir();
    if root.exists() || !ours {
        return root;
    }
    match std::fs::rename(&old, &root) {
        Ok(()) => root,
        Err(_) => old,
    }
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
    /// Game output, sent in batches: one event per line froze the UI when a mod logged thousands of lines.
    pub fn log(&self, lines: &[String]) {
        #[derive(Serialize, Clone)]
        struct LogEvent<'a> { profile: &'a str, lines: &'a [String] }
        let _ = self.app.emit("log", LogEvent { profile: &self.profile, lines });
    }
}

#[cfg(test)]
mod tests {
    use super::data_root;
    use std::path::PathBuf;

    fn base(name: &str) -> PathBuf {
        let b = std::env::temp_dir().join(format!("agate-root-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&b);
        std::fs::create_dir_all(&b).unwrap();
        b
    }

    #[test]
    fn blocco_data_moves_to_agate() {
        let b = base("move");
        std::fs::create_dir_all(b.join("Blocco/instances/abc/saves/World")).unwrap();
        std::fs::write(b.join("Blocco/accounts.json"), "{}").unwrap();
        assert_eq!(data_root(&b), b.join("Agate"));
        assert!(b.join("Agate/instances/abc/saves/World").is_dir());
        assert!(b.join("Agate/accounts.json").is_file());
        assert!(!b.join("Blocco").exists());
        // Second start: nothing left to move.
        assert_eq!(data_root(&b), b.join("Agate"));
        let _ = std::fs::remove_dir_all(b);
    }

    #[test]
    fn existing_agate_wins_and_unrelated_folders_stay() {
        let b = base("keep");
        std::fs::create_dir_all(b.join("Agate")).unwrap();
        std::fs::create_dir_all(b.join("Blocco/instances")).unwrap();
        assert_eq!(data_root(&b), b.join("Agate"));
        assert!(b.join("Blocco/instances").is_dir());
        // A "Blocco" folder that is not ours is never touched.
        let c = base("other");
        std::fs::create_dir_all(c.join("Blocco/something")).unwrap();
        assert_eq!(data_root(&c), c.join("Agate"));
        assert!(c.join("Blocco/something").is_dir());
        let _ = (std::fs::remove_dir_all(b), std::fs::remove_dir_all(c));
    }

    #[cfg(windows)]
    #[test]
    fn locked_blocco_folder_is_used_in_place() {
        use std::os::windows::fs::OpenOptionsExt;
        let b = base("locked");
        std::fs::create_dir_all(b.join("Blocco/instances")).unwrap();
        std::fs::write(b.join("Blocco/profiles.json"), "[]").unwrap();
        // An open file without delete sharing blocks renaming its folder, like a running game.
        let lock = std::fs::OpenOptions::new().read(true).share_mode(0).open(b.join("Blocco/profiles.json")).unwrap();
        assert_eq!(data_root(&b), b.join("Blocco"));
        drop(lock);
        assert_eq!(data_root(&b), b.join("Agate"));
        let _ = std::fs::remove_dir_all(b);
    }
}
