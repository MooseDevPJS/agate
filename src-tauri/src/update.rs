//! Self-update from the GitHub releases (tauri-plugin-updater). Every installer is
//! signed with the release key; the public half lives in tauri.conf.json, so Blocco
//! only ever installs builds that were signed by that key.

use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Update, UpdaterExt};

/// The update found by the last check, ready to install.
static PENDING: Mutex<Option<Update>> = Mutex::new(None);

#[derive(Serialize)]
pub struct Available {
    pub version: String,
    pub notes: Option<String>,
}

#[derive(Serialize, Clone)]
struct Progress {
    done: u64,
    total: Option<u64>,
}

pub async fn check(app: &AppHandle) -> anyhow::Result<Option<Available>> {
    let found = app.updater()?.check().await?;
    let out = found.as_ref().map(|u| Available { version: u.version.clone(), notes: u.body.clone().filter(|b| !b.trim().is_empty()) });
    *PENDING.lock().unwrap() = found;
    Ok(out)
}

/// Downloads the pending update (emitting `update-progress`), then runs its installer.
/// On Windows the installer replaces Blocco and starts it again, so this does not return on success.
pub async fn install(app: &AppHandle) -> anyhow::Result<()> {
    let pending = PENDING.lock().unwrap().take();
    let update = match pending {
        Some(u) => u,
        None => app.updater()?.check().await?.ok_or_else(|| anyhow::anyhow!("No update available"))?,
    };
    let mut done = 0u64;
    let emitter = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                done += chunk as u64;
                let _ = emitter.emit("update-progress", Progress { done, total });
            },
            || {},
        )
        .await?;
    Ok(())
}
