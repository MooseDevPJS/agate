//! Play: install whatever is missing, build the command line, run and watch the game.

use crate::app::{Paths, Reporter};
use crate::auth::{self, Account};
use crate::profiles::{self, Profile};
use crate::{fabric, java, mojang, optifine, sync};
use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

const LOG_LINES: usize = 3000;

#[derive(Default)]
pub struct Runtime {
    /// Profiles currently installing or running.
    pub busy: Mutex<HashSet<String>>,
    /// Game process ids by profile.
    pub running: Mutex<HashMap<String, u32>>,
    pub logs: Mutex<HashMap<String, VecDeque<String>>>,
    /// Profiles whose game the user stopped on purpose.
    pub stopped: Mutex<HashSet<String>>,
}

impl Runtime {
    fn push_log(&self, profile: &str, line: &str) {
        let mut logs = self.logs.lock().unwrap();
        let buf = logs.entry(profile.to_string()).or_default();
        if buf.len() >= LOG_LINES {
            buf.pop_front();
        }
        buf.push_back(line.to_string());
    }
}

#[derive(Serialize, Clone)]
struct ExitEvent {
    profile: String,
    code: Option<i32>,
    stopped: bool,
    error: Option<String>,
    /// Best guess at why the game crashed, read from its log.
    reason: Option<String>,
}

/// Fabric explains dependency problems in plain words; otherwise use the last exception.
fn crash_reason(lines: &[String]) -> Option<String> {
    if let Some(i) = lines.iter().position(|l| l.contains("Incompatible mods found") || l.contains("Mod resolution failed")) {
        let details: Vec<&str> = lines[i..]
            .iter()
            .map(|l| l.trim())
            .filter(|l| l.starts_with("- "))
            .take(3)
            .collect();
        if !details.is_empty() {
            return Some(format!("Incompatible mods: {}", details.join(" ").trim_start_matches("- ")));
        }
        return Some("Some mods are incompatible with each other or with this version.".into());
    }
    if lines.iter().any(|l| l.contains("UnsupportedClassVersionError")) {
        return Some("The Java version is not suitable for this game version.".into());
    }
    lines
        .iter()
        .rev()
        .find(|l| l.contains("Exception:") || l.contains("Error:"))
        .map(|l| l.trim().chars().take(240).collect())
}

/// Mojang's log4j config prints events as XML for launchers; turn them back into
/// the familiar `[12:00:00] [Render thread/INFO]: message` lines.
#[derive(Default)]
struct XmlLog {
    head: Option<String>,
    body: String,
    in_event: bool,
}

impl XmlLog {
    fn attr(line: &str, name: &str) -> Option<String> {
        let start = line.find(&format!("{name}=\""))? + name.len() + 2;
        let end = line[start..].find('"')?;
        Some(line[start..start + end].to_string())
    }

    /// Feed one raw line; returns finished readable lines.
    fn feed(&mut self, line: &str) -> Vec<String> {
        let t = line.trim();
        if t.starts_with("<log4j:Event") {
            let ts = Self::attr(t, "timestamp").and_then(|v| v.parse::<i64>().ok());
            let time = ts
                .and_then(chrono::DateTime::from_timestamp_millis)
                .map(|d| d.with_timezone(&chrono::Local).format("%H:%M:%S").to_string())
                .unwrap_or_default();
            let thread = Self::attr(t, "thread").unwrap_or_default();
            let level = Self::attr(t, "level").unwrap_or_default();
            self.head = Some(format!("[{time}] [{thread}/{level}]: "));
            self.body.clear();
            self.in_event = true;
            return vec![];
        }
        if !self.in_event {
            return vec![line.to_string()];
        }
        if t.starts_with("</log4j:Event>") {
            self.in_event = false;
            let head = self.head.take().unwrap_or_default();
            let body = std::mem::take(&mut self.body);
            let mut lines = body.trim_end().lines();
            let first = format!("{head}{}", lines.next().unwrap_or(""));
            return std::iter::once(first).chain(lines.map(String::from)).collect();
        }
        // Message / Throwable content, possibly spanning lines inside CDATA.
        let cleaned = t
            .replace("<log4j:Message><![CDATA[", "")
            .replace("<log4j:Throwable><![CDATA[", "")
            .replace("]]></log4j:Message>", "")
            .replace("]]></log4j:Throwable>", "");
        let content = if t.starts_with("<log4j:") || t.starts_with("]]>") { cleaned } else { line.replace("]]></log4j:Message>", "").replace("]]></log4j:Throwable>", "") };
        if !content.trim().is_empty() {
            self.body.push_str(&content);
            self.body.push('\n');
        }
        vec![]
    }
}

/// Resolves and installs the profile's version; returns (version id, resolved, installed, java).
async fn prepare(paths: &Paths, p: &mut Profile, rep: &Reporter) -> Result<(mojang::Resolved, mojang::Installed, std::path::PathBuf)> {
    let game_dir = paths.instance(&p.id);
    let natives = paths.natives(&p.id);
    rep.stage("Reading version");
    mojang::ensure_vanilla_json(paths, &p.mc).await?;
    let vanilla = mojang::resolve(paths, &p.mc).await?;
    let major = vanilla.json.java_version.as_ref().map_or(8, |j| j.major_version);
    let java = match p.java_path.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(custom) if Path::new(custom).exists() => custom.into(),
        Some(custom) => bail!("Java not found at {custom}"),
        None => java::ensure(paths, major, rep).await?,
    };

    let version_id = match p.loader.as_str() {
        "fabric" => {
            rep.stage("Fabric");
            let loader = match &p.loader_version {
                Some(v) => v.clone(),
                None => fabric::latest_loader(&p.mc).await?,
            };
            if p.loader_version.is_none() {
                p.loader_version = Some(loader.clone());
                let l = loader.clone();
                profiles::update(paths, &p.id, |x| x.loader_version = Some(l))?;
            }
            fabric::ensure(paths, &p.mc, &loader).await?
        }
        "optifine" => {
            // OptiFine patches the vanilla client, so vanilla goes first.
            mojang::install(paths, &vanilla, &natives, &game_dir, rep).await?;
            rep.stage("OptiFine");
            let file = match &p.loader_version {
                Some(f) => f.clone(),
                None => {
                    let builds = optifine::builds().await?;
                    let b = builds
                        .get(&p.mc)
                        .and_then(|l| l.first())
                        .ok_or_else(|| anyhow!("OptiFine is not available for Minecraft {}", p.mc))?;
                    let f = b.file.clone();
                    profiles::update(paths, &p.id, |x| x.loader_version = Some(f.clone()))?;
                    f
                }
            };
            optifine::ensure(paths, &p.mc, &file, &java).await?
        }
        _ => p.mc.clone(),
    };
    let resolved = mojang::resolve(paths, &version_id).await?;
    let installed = mojang::install(paths, &resolved, &natives, &game_dir, rep).await?;
    Ok((resolved, installed, java))
}

fn expand(values: &[Value], out: &mut Vec<String>) {
    for v in values {
        match v {
            Value::String(s) => out.push(s.clone()),
            Value::Object(o) => {
                let rules: Option<Vec<mojang::Rule>> = o.get("rules").and_then(|r| serde_json::from_value(r.clone()).ok());
                if !mojang::rules_allow(rules.as_ref()) {
                    continue;
                }
                match o.get("value") {
                    Some(Value::String(s)) => out.push(s.clone()),
                    Some(Value::Array(a)) => out.extend(a.iter().filter_map(|x| x.as_str().map(String::from))),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn build_args(
    paths: &Paths,
    p: &Profile,
    v: &mojang::Resolved,
    inst: &mojang::Installed,
    acc: &Account,
    quick: Option<&QuickPlay>,
) -> Vec<String> {
    let game_dir = paths.instance(&p.id);
    let natives = paths.natives(&p.id);
    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath = inst.classpath.iter().map(|c| c.display().to_string()).collect::<Vec<_>>().join(sep);
    let token = acc.mc_token.clone().unwrap_or_else(|| "0".into());
    let vars: HashMap<&str, String> = HashMap::from([
        ("auth_player_name", acc.name.clone()),
        ("version_name", v.id.clone()),
        ("game_directory", game_dir.display().to_string()),
        ("assets_root", inst.assets_root.display().to_string()),
        ("game_assets", inst.game_assets.display().to_string()),
        ("assets_index_name", inst.asset_index.clone()),
        ("auth_uuid", acc.uuid.clone()),
        ("auth_access_token", token.clone()),
        ("auth_session", format!("token:{token}:{}", acc.uuid)),
        ("auth_xuid", acc.xuid.clone()),
        ("clientid", String::new()),
        ("user_type", if acc.kind == "msa" { "msa".into() } else { "legacy".into() }),
        ("user_properties", "{}".into()),
        ("version_type", v.json.kind.clone().unwrap_or_else(|| "release".into())),
        ("natives_directory", natives.display().to_string()),
        ("launcher_name", "Blocco".into()),
        ("launcher_version", env!("CARGO_PKG_VERSION").into()),
        ("classpath", classpath),
        ("classpath_separator", sep.into()),
        ("library_directory", paths.libraries().display().to_string()),
    ]);
    let subst = |s: &str| -> String {
        // Single forward pass, so substituted values are never re-expanded.
        let mut out = String::with_capacity(s.len());
        let mut rest = s;
        while let Some(start) = rest.find("${") {
            let Some(end) = rest[start..].find('}') else { break };
            out.push_str(&rest[..start]);
            out.push_str(vars.get(&rest[start + 2..start + end]).map_or("", |v| v.as_str()));
            rest = &rest[start + end + 1..];
        }
        out.push_str(rest);
        out
    };

    let mut args = vec![format!("-Xmx{}M", p.memory_mb), format!("-Xms{}M", p.memory_mb.min(1024))];
    // Trust the Windows certificate store, like every other Windows app. Without it, HTTPS
    // scanning by antivirus software (e.g. Norton Web Shield) breaks skins and server login.
    if cfg!(windows) {
        args.push("-Djavax.net.ssl.trustStoreType=Windows-ROOT".into());
    }
    args.extend(p.jvm_args.split_whitespace().map(String::from));
    let mut jvm = Vec::new();
    match v.json.arguments.as_ref().filter(|a| !a.jvm.is_empty()) {
        Some(a) => expand(&a.jvm, &mut jvm),
        None => {
            if cfg!(windows) {
                jvm.push("-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump".into());
            }
            jvm.extend(["-Djava.library.path=${natives_directory}".into(), "-cp".into(), "${classpath}".into()]);
        }
    }
    args.extend(jvm.iter().map(|a| subst(a)));
    if let Some((arg, path)) = &inst.log_config {
        args.push(arg.replace("${path}", &path.display().to_string()));
    }
    args.push(v.json.main_class.clone().unwrap_or_else(|| "net.minecraft.client.main.Main".into()));
    let mut game = Vec::new();
    if let Some(a) = &v.json.arguments {
        expand(&a.game, &mut game);
    }
    if let Some(legacy) = &v.json.minecraft_arguments {
        game.extend(legacy.split_whitespace().map(String::from));
    }
    args.extend(game.iter().map(|a| subst(a)));
    match quick {
        Some(QuickPlay::World(w)) => args.extend(["--quickPlaySingleplayer".to_string(), w.clone()]),
        Some(QuickPlay::Server(addr)) => args.extend(["--quickPlayMultiplayer".to_string(), addr.clone()]),
        None => {}
    }
    args
}

/// Full Play flow. Returns once the game process has started.
/// Start straight into a singleplayer world (folder name) or a server (host[:port]).
pub enum QuickPlay {
    World(String),
    Server(String),
}

/// Whether this version understands Quick Play (`--quickPlay*`, 1.20 and later).
pub fn supports_quick_play(json: &mojang::VersionJson) -> bool {
    json.arguments.as_ref().is_some_and(|a| a.game.iter().any(|v| v.to_string().contains("quickPlaySingleplayer")))
}

pub async fn play(app: tauri::AppHandle, paths: Paths, rt: &'static Runtime, profile_id: String, quick: Option<QuickPlay>) -> Result<()> {
    {
        let mut busy = rt.busy.lock().unwrap();
        if !busy.insert(profile_id.clone()) {
            bail!("This profile is already running or installing");
        }
    }
    let result = play_inner(app.clone(), paths, rt, &profile_id, quick).await;
    if result.is_err() {
        rt.busy.lock().unwrap().remove(&profile_id);
    }
    result
}

async fn play_inner(app: tauri::AppHandle, paths: Paths, rt: &'static Runtime, profile_id: &str, quick: Option<QuickPlay>) -> Result<()> {
    let rep = Reporter::new(app.clone(), profile_id);
    let mut p = profiles::get(&paths, profile_id)?;

    // Account first: fail fast before downloading gigabytes.
    let mut accounts = auth::load(&paths);
    let active = accounts.active.clone().ok_or_else(|| anyhow!("Add an account before playing"))?;
    let acc = accounts.list.iter().find(|a| a.uuid == active).cloned().ok_or_else(|| anyhow!("Add an account before playing"))?;
    rep.stage("Signing in");
    let acc = auth::ensure_fresh(&acc).await?;
    if let Some(slot) = accounts.list.iter_mut().find(|a| a.uuid == acc.uuid) {
        *slot = acc.clone();
    }
    auth::save(&paths, &accounts)?;

    let (resolved, installed, java) = prepare(&paths, &mut p, &rep).await?;
    let game_dir = paths.instance(&p.id);
    crate::cape::apply(&paths, &p, profiles::settings(&paths).custom_cape)?;
    let legacy = sync::is_legacy(resolved.json.release_time.as_deref());
    if p.sync_settings {
        sync::before_launch(&paths, &game_dir, legacy)?;
    }

    rep.stage("Starting");
    let quick = quick.filter(|_| supports_quick_play(&resolved.json));
    let args = build_args(&paths, &p, &resolved, &installed, &acc, quick.as_ref());
    let mut cmd = tokio::process::Command::new(&java);
    cmd.args(&args).current_dir(&game_dir).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    let mut child = cmd.spawn().map_err(|e| anyhow!("Could not start Java ({}): {e}", java.display()))?;
    let pid = child.id().unwrap_or(0);
    rt.running.lock().unwrap().insert(p.id.clone(), pid);
    rt.logs.lock().unwrap().remove(&p.id);
    profiles::update(&paths, &p.id, |x| x.last_played = Some(chrono::Utc::now().timestamp()))?;
    let _ = app.emit("game-start", &p.id);
    let hide = crate::profiles::settings(&paths).close_on_launch;
    if hide {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
    }

    let token = acc.mc_token.clone().unwrap_or_default();
    for stream in [child.stdout.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
                   child.stderr.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>)]
        .into_iter()
        .flatten()
    {
        let (rep, pid_s, token) = (rep.clone(), p.id.clone(), token.clone());
        tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            let mut xml = XmlLog::default();
            while let Ok(Some(raw)) = lines.next_line().await {
                for line in xml.feed(&raw) {
                    // Never surface the access token in logs.
                    let line = if !token.is_empty() && line.contains(&token) { line.replace(&token, "<token>") } else { line };
                    rt.push_log(&pid_s, &line);
                    rep.log(&line);
                }
            }
        });
    }

    let id = p.id.clone();
    let sync_on = p.sync_settings;
    tokio::spawn(async move {
        let status = child.wait().await;
        let mut error = None;
        if sync_on {
            if let Err(e) = sync::after_exit(&paths, &paths.instance(&id), legacy) {
                error = Some(format!("Settings were not synced: {e}"));
            }
        }
        rt.running.lock().unwrap().remove(&id);
        rt.busy.lock().unwrap().remove(&id);
        let code = status.ok().and_then(|s| s.code());
        let stopped = rt.stopped.lock().unwrap().remove(&id);
        let reason = if !stopped && code.unwrap_or(0) != 0 {
            let lines: Vec<String> = rt.logs.lock().unwrap().get(&id).map(|l| l.iter().cloned().collect()).unwrap_or_default();
            crash_reason(&lines)
        } else {
            None
        };
        if hide {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
        let _ = app.emit("game-exit", ExitEvent { profile: id, code, stopped, error, reason });
    });
    Ok(())
}

pub fn kill(rt: &Runtime, profile_id: &str) -> Result<()> {
    let pid = *rt.running.lock().unwrap().get(profile_id).ok_or_else(|| anyhow!("not running"))?;
    rt.stopped.lock().unwrap().insert(profile_id.to_string());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000)
            .status()?;
    }
    #[cfg(not(windows))]
    std::process::Command::new("kill").arg(pid.to_string()).status()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::XmlLog;

    #[test]
    fn xml_events_become_plain_lines() {
        let mut x = XmlLog::default();
        let input = [
            r#"<log4j:Event logger="FabricLoader" timestamp="1794571713944" level="INFO" thread="main">"#,
            "  <log4j:Message><![CDATA[Loading Minecraft 1.20.1]]></log4j:Message>",
            "</log4j:Event>",
            r#"<log4j:Event logger="x" timestamp="1794571713944" level="ERROR" thread="Render thread">"#,
            "  <log4j:Message><![CDATA[Boom]]></log4j:Message>",
            "  <log4j:Throwable><![CDATA[java.lang.RuntimeException: bad",
            "	at a.b(C.java:1)",
            "]]></log4j:Throwable>",
            "</log4j:Event>",
            "plain stdout line",
        ];
        let out: Vec<String> = input.iter().flat_map(|l| x.feed(l)).collect();
        assert!(out[0].ends_with("[main/INFO]: Loading Minecraft 1.20.1"), "{out:?}");
        assert!(out[1].ends_with("[Render thread/ERROR]: Boom"), "{out:?}");
        assert_eq!(out[2], "java.lang.RuntimeException: bad");
        assert_eq!(out[3], "	at a.b(C.java:1)");
        assert_eq!(out[4], "plain stdout line");
    }

    #[test]
    fn fabric_incompatibility_is_explained() {
        let log: Vec<String> = [
            "[13:32:24] [main/ERROR]: Incompatible mods found!",
            "A potential solution has been determined, this may resolve your problem:",
            "	 - Replace mod 'Sodium' (sodium) 0.9.3-beta.5 with any 0.9.x version",
            "More details:",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let r = super::crash_reason(&log).unwrap();
        assert!(r.contains("Replace mod 'Sodium'"), "{r}");
    }
}
