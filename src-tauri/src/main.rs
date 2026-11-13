#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod auth;
mod cape;
mod content;
mod fabric;
mod import;
mod java;
mod launch;
mod modrinth;
mod mojang;
mod net;
mod optifine;
mod profiles;
mod sync;
mod update;

use app::Paths;
use profiles::{Profile, Settings};
use serde::Serialize;
use std::sync::OnceLock;
use tauri::{Manager, State};

type Res<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String {
    // Outermost context first, then root cause, e.g. "downloading … : connection reset".
    let chain: Vec<String> = e.chain().map(|c| c.to_string()).collect();
    match chain.as_slice() {
        [one] => one.clone(),
        [first, .., last] if first != last => format!("{first}: {last}"),
        _ => e.to_string(),
    }
}

fn runtime() -> &'static launch::Runtime {
    static RT: OnceLock<launch::Runtime> = OnceLock::new();
    RT.get_or_init(Default::default)
}

#[derive(Serialize)]
struct Snapshot {
    profiles: Vec<Profile>,
    accounts: Vec<auth::AccountView>,
    settings: Settings,
    running: Vec<String>,
    busy: Vec<String>,
    data_dir: String,
    version: String,
}

#[tauri::command]
fn snapshot(app: tauri::AppHandle, paths: State<Paths>) -> Snapshot {
    let rt = runtime();
    Snapshot {
        profiles: profiles::list(&paths),
        accounts: auth::views(&auth::load(&paths)),
        settings: profiles::settings(&paths),
        running: rt.running.lock().unwrap().keys().cloned().collect(),
        busy: rt.busy.lock().unwrap().iter().cloned().collect(),
        data_dir: paths.root.display().to_string(),
        version: app.package_info().version.to_string(),
    }
}

#[derive(Serialize)]
struct VersionItem {
    id: String,
    kind: String,
    release_time: String,
}

#[tauri::command]
async fn versions(paths: State<'_, Paths>) -> Res<Vec<VersionItem>> {
    let m = mojang::manifest(&paths).await.map_err(err)?;
    Ok(m.versions
        .into_iter()
        .filter(|v| v.kind == "release" || v.kind == "snapshot")
        .map(|v| VersionItem { id: v.id, kind: v.kind, release_time: v.release_time })
        .collect())
}

#[tauri::command]
async fn fabric_versions() -> Res<Vec<String>> {
    fabric::game_versions().await.map_err(err)
}

#[tauri::command]
async fn optifine_builds() -> Res<std::collections::BTreeMap<String, Vec<optifine::Build>>> {
    optifine::builds().await.map_err(err)
}

#[tauri::command]
fn save_profile(paths: State<Paths>, mut profile: Profile) -> Res<Profile> {
    if profile.id.is_empty() {
        profile.id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
        profile.created = chrono::Utc::now().timestamp();
    } else if !profile.id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("invalid profile id".into());
    }
    if runtime().busy.lock().unwrap().contains(&profile.id) {
        return Err("Close the game before changing this profile".into());
    }
    profiles::upsert(&paths, profile).map_err(err)
}

#[tauri::command]
fn delete_profile(paths: State<Paths>, id: String) -> Res<()> {
    if runtime().busy.lock().unwrap().contains(&id) {
        return Err("Close the game before deleting this profile".into());
    }
    profiles::get(&paths, &id).map_err(err)?;
    profiles::delete(&paths, &id).map_err(err)
}

#[tauri::command]
async fn play(app: tauri::AppHandle, paths: State<'_, Paths>, id: String, world: Option<String>, server: Option<String>) -> Res<()> {
    let quick = match (world, server) {
        (Some(w), _) => Some(launch::QuickPlay::World(w)),
        (None, Some(s)) if !s.trim().is_empty() && !s.chars().any(char::is_whitespace) => Some(launch::QuickPlay::Server(s)),
        _ => None,
    };
    launch::play(app, paths.inner().clone(), runtime(), id, quick).await.map_err(err)
}

#[tauri::command]
fn stop(id: String) -> Res<()> {
    launch::kill(runtime(), &id).map_err(err)
}

#[tauri::command]
fn logs(id: String) -> Vec<String> {
    runtime().logs.lock().unwrap().get(&id).map(|l| l.iter().cloned().collect()).unwrap_or_default()
}

/// Hands a local file or folder to the system without waiting for it: Explorer opens
/// folders and opens files with their usual app. (Shell-execute through the `open` crate on
/// the command thread made the UI wait for Windows to finish.)
fn reveal(path: &std::path::Path) -> Res<()> {
    #[cfg(windows)]
    {
        // Plain `C:\...` form: Explorer handles `\\?\` verbatim paths slowly or not at all.
        let plain = path.to_string_lossy().trim_start_matches(r"\\?\").to_string();
        std::process::Command::new("explorer.exe").arg(plain).spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    open::that_detached(path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_folder(paths: State<'_, Paths>, id: Option<String>) -> Res<()> {
    let dir = match id {
        Some(id) => {
            profiles::get(&paths, &id).map_err(err)?;
            paths.instance(&id)
        }
        None => paths.root.clone(),
    };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    reveal(&dir)
}

#[tauri::command]
fn open_url(url: String) -> Res<()> {
    const ALLOWED: &[&str] = &["https://www.microsoft.com/", "https://microsoft.com/", "https://modrinth.com/", "https://optifine.net/"];
    if !ALLOWED.iter().any(|p| url.starts_with(p)) {
        return Err("blocked URL".into());
    }
    open::that_detached(url).map_err(|e| e.to_string())
}

#[tauri::command]
async fn content_search(paths: State<'_, Paths>, id: String, kind: String, query: String, offset: u64) -> Res<modrinth::SearchResult> {
    let p = profiles::get(&paths, &id).map_err(err)?;
    modrinth::search(&p, modrinth::Kind::parse(&kind).map_err(err)?, &query, offset).await.map_err(err)
}

#[tauri::command]
async fn content_install(paths: State<'_, Paths>, id: String, kind: String, project: String) -> Res<Vec<String>> {
    let p = profiles::get(&paths, &id).map_err(err)?;
    let kind = modrinth::Kind::parse(&kind).map_err(err)?;
    let added = modrinth::install(&paths, &p, kind, &project).await.map_err(err)?;
    // A freshly installed resource pack or shader is what the player wants to see: select it.
    if kind != modrinth::Kind::Mod && !runtime().running.lock().unwrap().contains_key(&id) {
        let p = profiles::get(&paths, &id).map_err(err)?;
        if let Some(entry) = p.mods.iter().find(|m| m.project_id == project) {
            match kind {
                modrinth::Kind::ResourcePack => content::set_resource_pack(&paths, &p, &entry.file, true),
                _ => content::set_shader(&paths, &p, Some(&entry.file)),
            }
            .map_err(err)?;
        }
    }
    Ok(added)
}

#[tauri::command]
fn content_list(paths: State<Paths>, id: String, kind: String) -> Res<Vec<modrinth::Installed>> {
    let p = profiles::get(&paths, &id).map_err(err)?;
    Ok(modrinth::list(&paths, &p, modrinth::Kind::parse(&kind).map_err(err)?))
}

#[tauri::command]
fn content_remove(paths: State<Paths>, id: String, kind: String, file: String) -> Res<()> {
    let p = profiles::get(&paths, &id).map_err(err)?;
    modrinth::remove(&paths, &p, modrinth::Kind::parse(&kind).map_err(err)?, &file).map_err(err)
}

#[tauri::command]
fn content_toggle(paths: State<Paths>, id: String, kind: String, file: String) -> Res<()> {
    if runtime().running.lock().unwrap().contains_key(&id) {
        return Err("Chiudi il gioco prima: le modifiche verrebbero sovrascritte".into());
    }
    let p = profiles::get(&paths, &id).map_err(err)?;
    modrinth::toggle(&paths, &p, modrinth::Kind::parse(&kind).map_err(err)?, &file).map_err(err)
}

#[derive(Serialize)]
struct Worlds {
    worlds: Vec<content::World>,
    servers: Vec<content::Server>,
    quick_play: bool,
}

#[tauri::command]
async fn worlds(paths: State<'_, Paths>, id: String) -> Res<Worlds> {
    let p = profiles::get(&paths, &id).map_err(err)?;
    // Quick Play needs 1.20+; known only once the version is installed.
    let quick_play = match paths.version_json(&p.mc).exists() {
        true => mojang::resolve(&paths, &p.mc).await.map(|v| launch::supports_quick_play(&v.json)).unwrap_or(false),
        false => false,
    };
    Ok(Worlds { worlds: content::worlds(&paths, &p), servers: content::servers(&paths, &p), quick_play })
}

#[tauri::command]
fn screenshots(paths: State<Paths>, id: String) -> Res<Vec<content::Shot>> {
    let p = profiles::get(&paths, &id).map_err(err)?;
    Ok(content::screenshots(&paths, &p))
}

#[derive(Serialize)]
struct Panorama {
    from: String,
    faces: Vec<String>,
}

#[tauri::command]
async fn panorama(paths: State<'_, Paths>, mc: String) -> Res<Option<Panorama>> {
    let paths = paths.inner().clone();
    tokio::task::spawn_blocking(move || content::panorama(&paths, &mc))
        .await
        .map_err(|e| e.to_string())?
        .map(|o| o.map(|(from, faces)| Panorama { from, faces }))
        .map_err(err)
}

/// Opens a file or folder that lives inside the launcher's data directory.
#[tauri::command]
async fn open_path(paths: State<'_, Paths>, path: String) -> Res<()> {
    let canon = std::path::PathBuf::from(&path).canonicalize().map_err(|e| e.to_string())?;
    let root = paths.root.canonicalize().map_err(|e| e.to_string())?;
    if !canon.starts_with(&root) {
        return Err("blocked path".into());
    }
    reveal(&canon)
}

#[derive(Serialize)]
struct Skin {
    path: String,
    slim: bool,
    default: bool,
    cape: Option<String>,
}

/// Downloads a Mojang texture (skin or cape) into the cache once, by its content hash.
async fn texture(paths: &Paths, url: &str) -> Option<String> {
    if !url.starts_with("https://textures.minecraft.net/texture/") {
        return None;
    }
    let hash: String = url.rsplit('/').next().unwrap_or("").chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let path = paths.cache().join("skins").join(format!("{hash}.png"));
    let fetched = net::fetch(&net::Download { url: url.to_string(), path: path.clone(), sha1: None, size: None }, &Default::default()).await;
    (fetched.is_ok() || path.exists()).then(|| path.display().to_string())
}

/// The active account with fresh tokens (saved back), or None.
async fn active_account(paths: &Paths) -> Option<auth::Account> {
    let mut all = auth::load(paths);
    let mut acc = all.active.clone().and_then(|u| all.list.iter().find(|a| a.uuid == u).cloned())?;
    if acc.kind == "msa" {
        if let Ok(fresh) = auth::ensure_fresh(&acc).await {
            acc = fresh;
            let _ = auth::refresh_skin(&mut acc).await;
            if let Some(slot) = all.list.iter_mut().find(|a| a.uuid == acc.uuid) {
                *slot = acc.clone();
            }
            let _ = auth::save(paths, &all);
        }
    }
    Some(acc)
}

fn store_account(paths: &Paths, acc: auth::Account) -> Res<()> {
    let mut all = auth::load(paths);
    if let Some(slot) = all.list.iter_mut().find(|a| a.uuid == acc.uuid) {
        *slot = acc;
    }
    auth::save(paths, &all).map_err(err)
}

/// The active account's skin and cape as local PNGs, or the default skin.
#[tauri::command]
async fn account_skin(paths: State<'_, Paths>) -> Res<Option<Skin>> {
    let Some(acc) = active_account(&paths).await else { return Ok(None) };
    let cape = match &acc.cape_url {
        Some(u) => texture(&paths, u).await,
        None => None,
    };
    if let Some(path) = match &acc.skin_url {
        Some(u) => texture(&paths, u).await,
        None => None,
    } {
        return Ok(Some(Skin { path, slim: acc.skin_variant.as_deref() == Some("SLIM"), default: false, cape }));
    }
    let paths = paths.inner().clone();
    let steve = tokio::task::spawn_blocking(move || content::steve(&paths)).await.map_err(|e| e.to_string())?;
    Ok(steve.map(|p| Skin { path: p.display().to_string(), slim: false, default: true, cape }))
}

#[derive(Serialize)]
struct CapeView {
    id: String,
    alias: String,
    path: Option<String>,
    active: bool,
}

/// Capes owned by the active Microsoft account.
#[tauri::command]
async fn capes(paths: State<'_, Paths>) -> Res<Vec<CapeView>> {
    let Some(acc) = active_account(&paths).await.filter(|a| a.kind == "msa") else { return Ok(vec![]) };
    let profile = auth::fetch_profile(&acc).await.map_err(err)?;
    let mut out = Vec::new();
    for c in auth::capes(&profile) {
        let path = texture(&paths, &c.url).await;
        out.push(CapeView { id: c.id, alias: c.alias, path, active: c.active });
    }
    Ok(out)
}

#[tauri::command]
async fn set_cape(paths: State<'_, Paths>, id: Option<String>) -> Res<()> {
    let mut acc = active_account(&paths).await.filter(|a| a.kind == "msa").ok_or("Serve un account Microsoft")?;
    auth::set_cape(&mut acc, id.as_deref()).await.map_err(err)?;
    store_account(&paths, acc)
}

#[derive(Serialize)]
struct CustomCape {
    path: String,
    source: Option<String>,
    options: Option<String>,
}

/// The custom cape, if one was made: texture, source picture and editor options.
#[tauri::command]
fn custom_cape(paths: State<Paths>) -> Option<CustomCape> {
    let p = cape::image(&paths);
    p.exists().then(|| CustomCape {
        path: p.display().to_string(),
        source: Some(cape::source(&paths)).filter(|s| s.exists()).map(|s| s.display().to_string()),
        options: std::fs::read_to_string(cape::meta(&paths)).ok(),
    })
}

#[tauri::command]
fn save_custom_cape(paths: State<Paths>, png: Vec<u8>, source: Option<Vec<u8>>, options: Option<String>) -> Res<()> {
    cape::save(&paths, &png, source.as_deref(), options.as_deref()).map_err(err)
}

#[tauri::command]
fn delete_custom_cape(paths: State<Paths>) -> Res<()> {
    cape::remove(&paths).map_err(err)
}

/// Instances of other launchers found on this computer.
#[tauri::command]
async fn import_scan() -> Res<Vec<import::Found>> {
    tokio::task::spawn_blocking(import::scan).await.map_err(|e| e.to_string())
}

/// Copies one found instance into a new profile (the original stays untouched).
#[tauri::command]
async fn import_instance(paths: State<'_, Paths>, path: String, color: String, icon: String) -> Res<Profile> {
    let paths = paths.inner().clone();
    tokio::task::spawn_blocking(move || import::import(&paths, &path, &color, &icon))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

#[tauri::command]
async fn upload_skin(paths: State<'_, Paths>, slim: bool, png: Vec<u8>) -> Res<()> {
    let mut all = auth::load(&paths);
    let uuid = all.active.clone().ok_or("Nessun account attivo")?;
    let acc = all.list.iter().find(|a| a.uuid == uuid).cloned().ok_or("Nessun account attivo")?;
    if acc.kind != "msa" {
        return Err("Gli account offline non hanno una skin online".into());
    }
    let mut acc = auth::ensure_fresh(&acc).await.map_err(err)?;
    auth::upload_skin(&mut acc, slim, png).await.map_err(err)?;
    if let Some(slot) = all.list.iter_mut().find(|a| a.uuid == uuid) {
        *slot = acc;
    }
    auth::save(&paths, &all).map_err(err)
}

fn client_id(paths: &Paths) -> Option<String> {
    profiles::settings(paths).azure_client_id.filter(|s| !s.trim().is_empty()).map(|s| s.trim().to_string())
}

#[tauri::command]
async fn login_start(paths: State<'_, Paths>) -> Res<auth::DeviceCode> {
    auth::start_device_code(client_id(&paths).as_deref()).await.map_err(err)
}

/// Returns the new account once the user finished in the browser, None while pending.
#[tauri::command]
async fn login_poll(paths: State<'_, Paths>, device_code: String) -> Res<Option<auth::AccountView>> {
    match auth::poll_device_code(client_id(&paths).as_deref(), &device_code).await.map_err(err)? {
        auth::Poll::Pending => Ok(None),
        auth::Poll::Done(acc) => {
            let mut all = auth::load(&paths);
            all.list.retain(|a| a.uuid != acc.uuid);
            all.active = Some(acc.uuid.clone());
            let view = auth::AccountView { uuid: acc.uuid.clone(), name: acc.name.clone(), kind: acc.kind.clone(), active: true };
            all.list.push(acc);
            auth::save(&paths, &all).map_err(err)?;
            Ok(Some(view))
        }
    }
}

#[tauri::command]
fn add_offline(paths: State<Paths>, name: String) -> Res<()> {
    let name = name.trim().to_string();
    if !auth::valid_offline_name(&name) {
        return Err("Use 3–16 letters, digits or underscores".into());
    }
    let mut all = auth::load(&paths);
    let uuid = auth::offline_uuid(&name);
    all.list.retain(|a| a.uuid != uuid);
    all.list.push(auth::Account { uuid: uuid.clone(), name, kind: "offline".into(), ..Default::default() });
    all.active = Some(uuid);
    auth::save(&paths, &all).map_err(err)
}

#[tauri::command]
fn set_account(paths: State<Paths>, uuid: String) -> Res<()> {
    let mut all = auth::load(&paths);
    if !all.list.iter().any(|a| a.uuid == uuid) {
        return Err("unknown account".into());
    }
    all.active = Some(uuid);
    auth::save(&paths, &all).map_err(err)
}

#[tauri::command]
fn remove_account(paths: State<Paths>, uuid: String) -> Res<()> {
    let mut all = auth::load(&paths);
    all.list.retain(|a| a.uuid != uuid);
    if all.active.as_deref() == Some(&uuid) {
        all.active = all.list.first().map(|a| a.uuid.clone());
    }
    auth::save(&paths, &all).map_err(err)
}

#[tauri::command]
fn save_settings(paths: State<Paths>, settings: Settings) -> Res<()> {
    if !(512..=65536).contains(&settings.default_memory_mb) {
        return Err("Memory must be between 512 MB and 64 GB".into());
    }
    if !["auto", "on", "off"].contains(&settings.motion.as_str()) || !["it", "en"].contains(&settings.language.as_str()) {
        return Err("invalid setting".into());
    }
    profiles::save_settings(&paths, &settings).map_err(err)
}

#[tauri::command]
fn system_memory_mb() -> u64 {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct MemoryStatusEx {
            len: u32,
            load: u32,
            total_phys: u64,
            avail_phys: u64,
            total_page: u64,
            avail_page: u64,
            total_virt: u64,
            avail_virt: u64,
            avail_ext: u64,
        }
        extern "system" {
            fn GlobalMemoryStatusEx(buf: *mut MemoryStatusEx) -> i32;
        }
        let mut m = MemoryStatusEx { len: std::mem::size_of::<MemoryStatusEx>() as u32, load: 0, total_phys: 0, avail_phys: 0, total_page: 0, avail_page: 0, total_virt: 0, avail_virt: 0, avail_ext: 0 };
        if unsafe { GlobalMemoryStatusEx(&mut m) } != 0 {
            return m.total_phys / 1024 / 1024;
        }
    }
    8192
}

/// `blocco://` (http://blocco.localhost on Windows): images from the data folder only
/// (title-screen panorama, skins, world icons, screenshots).
fn serve_data_file(uri_path: &str) -> tauri::http::Response<Vec<u8>> {
    let respond = |status: u16, ct: &str, body: Vec<u8>| {
        tauri::http::Response::builder().status(status).header("Content-Type", ct).header("Access-Control-Allow-Origin", "*").header("Cache-Control", "no-cache").body(body).unwrap()
    };
    let rel = percent_decode(uri_path.trim_start_matches('/'));
    let safe = !rel.is_empty() && rel.split(['/', '\\']).all(|seg| !seg.is_empty() && seg != ".." && !seg.contains(':'));
    let ct = match rel.rsplit('.').next().map(|e| e.to_ascii_lowercase()) {
        Some(e) if e == "png" => "image/png",
        Some(e) if e == "jpg" || e == "jpeg" => "image/jpeg",
        _ => return respond(404, "text/plain", vec![]),
    };
    if !safe {
        return respond(403, "text/plain", vec![]);
    }
    match std::fs::read(Paths::new().root.join(rel)) {
        Ok(bytes) => respond(200, ct, bytes),
        Err(_) => respond(404, "text/plain", vec![]),
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[tauri::command]
async fn update_check(app: tauri::AppHandle) -> Res<Option<update::Available>> {
    update::check(&app).await.map_err(err)
}

#[tauri::command]
async fn update_install(app: tauri::AppHandle) -> Res<()> {
    // The installer closes Blocco: a running game would lose its settings sync on exit.
    let rt = runtime();
    if !rt.running.lock().unwrap().is_empty() || !rt.busy.lock().unwrap().is_empty() {
        return Err("Close the game before updating".into());
    }
    update::install(&app).await.map_err(err)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let paths = Paths::new();
            std::fs::create_dir_all(&paths.root)?;
            app.manage(paths);
            Ok(())
        })
        .register_uri_scheme_protocol("blocco", |_ctx, req| serve_data_file(req.uri().path()))
        .invoke_handler(tauri::generate_handler![
            snapshot, versions, fabric_versions, optifine_builds, save_profile, delete_profile, play, stop, logs,
            open_folder, open_url, content_search, content_install, content_list, content_remove, content_toggle, worlds,
            screenshots, panorama, open_path, account_skin, upload_skin, capes, set_cape, custom_cape, save_custom_cape, delete_custom_cape, import_scan, import_instance, login_start, login_poll,
            add_offline, set_account, remove_account, save_settings, system_memory_mb, update_check, update_install
        ])
        .run(tauri::generate_context!())
        .expect("error while running Blocco");
}
