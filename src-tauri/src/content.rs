//! What lives inside a profile's game folder: active resource packs and shader,
//! worlds and screenshots; plus textures read from the game jars (title-screen
//! panorama, default skin) so the launcher can show the game's own art.

use crate::app::Paths;
use crate::profiles::Profile;
use anyhow::{Context, Result};
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/* ---------- resource packs (options.txt `resourcePacks`) ---------- */

fn legacy(paths: &Paths, p: &Profile) -> bool {
    crate::sync::legacy_version(paths, &p.mc)
}

fn options_path(paths: &Paths, p: &Profile) -> PathBuf {
    paths.instance(&p.id).join("options.txt")
}

fn read_packs(paths: &Paths, p: &Profile) -> Vec<String> {
    std::fs::read_to_string(options_path(paths, p))
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("resourcePacks:").map(String::from)))
        .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
        .unwrap_or_default()
}

/// File names of the resource packs currently selected in game.
pub fn active_resource_packs(paths: &Paths, p: &Profile) -> Vec<String> {
    let legacy = legacy(paths, p);
    read_packs(paths, p)
        .into_iter()
        .filter_map(|e| if legacy { Some(e) } else { e.strip_prefix("file/").map(String::from) })
        .collect()
}

/// Adds the pack on top of the list (highest priority) or removes it.
pub fn set_resource_pack(paths: &Paths, p: &Profile, file: &str, on: bool) -> Result<()> {
    let legacy = legacy(paths, p);
    let entry = if legacy { file.to_string() } else { format!("file/{file}") };
    let mut packs = read_packs(paths, p);
    packs.retain(|x| x != &entry);
    if !legacy && !packs.iter().any(|x| x == "vanilla") {
        packs.insert(0, "vanilla".into());
    }
    if on {
        packs.push(entry);
    }
    let path = options_path(paths, p);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let line = format!("resourcePacks:{}", serde_json::to_string(&packs)?);
    let mut found = false;
    let mut out: Vec<String> = text
        .lines()
        .map(|l| {
            if l.starts_with("resourcePacks:") {
                found = true;
                line.clone()
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        out.push(line);
    }
    std::fs::create_dir_all(paths.instance(&p.id))?;
    std::fs::write(path, out.join("\n") + "\n")?;
    Ok(())
}

/* ---------- shaders (Iris properties / OptiFine optionsshaders.txt) ---------- */

fn shader_config(paths: &Paths, p: &Profile) -> PathBuf {
    let dir = paths.instance(&p.id);
    if p.loader == "optifine" { dir.join("optionsshaders.txt") } else { dir.join("config").join("iris.properties") }
}

fn read_props(path: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().replace('\\', "")))
        .collect()
}

pub fn active_shader(paths: &Paths, p: &Profile) -> Option<String> {
    let props = read_props(&shader_config(paths, p));
    let get = |k: &str| props.iter().find(|(x, _)| x == k).map(|(_, v)| v.clone());
    if p.loader == "fabric" && get("enableShaders").as_deref() == Some("false") {
        return None;
    }
    get("shaderPack").filter(|v| !v.is_empty() && v != "OFF" && v != "(internal)")
}

/// Selects a shader pack, or switches shaders off with `None`.
pub fn set_shader(paths: &Paths, p: &Profile, file: Option<&str>) -> Result<()> {
    let path = shader_config(paths, p);
    let mut props = read_props(&path);
    let mut set = |k: &str, v: String| match props.iter_mut().find(|(x, _)| x == k) {
        Some(slot) => slot.1 = v,
        None => props.push((k.to_string(), v)),
    };
    if p.loader == "optifine" {
        set("shaderPack", file.unwrap_or("OFF").to_string());
    } else {
        if let Some(f) = file {
            // Java properties: escape the characters that would end the key or value.
            set("shaderPack", f.replace(':', "\\:").replace('=', "\\="));
        }
        set("enableShaders", file.is_some().to_string());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, props.iter().map(|(k, v)| format!("{k}={v}\n")).collect::<String>())?;
    Ok(())
}

/* ---------- worlds and screenshots ---------- */

fn mtime(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64)
}

#[derive(Serialize)]
pub struct World {
    pub folder: String,
    pub modified: i64,
    pub icon: Option<String>,
}

pub fn worlds(paths: &Paths, p: &Profile) -> Vec<World> {
    let saves = paths.instance(&p.id).join("saves");
    let mut out: Vec<World> = std::fs::read_dir(saves)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join("level.dat").exists())
        .map(|e| {
            let dir = e.path();
            let icon = dir.join("icon.png");
            World {
                folder: e.file_name().to_string_lossy().to_string(),
                modified: mtime(&dir.join("level.dat")),
                icon: icon.exists().then(|| icon.display().to_string()),
            }
        })
        .collect();
    out.sort_by_key(|w| -w.modified);
    out
}

#[derive(Serialize)]
pub struct Shot {
    pub path: String,
    pub name: String,
    pub modified: i64,
}

pub fn screenshots(paths: &Paths, p: &Profile) -> Vec<Shot> {
    let dir = paths.instance(&p.id).join("screenshots");
    let mut out: Vec<Shot> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("png")))
        .map(|e| Shot {
            path: e.path().display().to_string(),
            name: e.file_name().to_string_lossy().to_string(),
            modified: mtime(&e.path()),
        })
        .collect();
    out.sort_by_key(|s| -s.modified);
    out.truncate(120);
    out
}

/* ---------- saved servers (servers.dat, uncompressed NBT) ---------- */

#[derive(Serialize)]
pub struct Server {
    pub name: String,
    pub ip: String,
    /// The server's favicon as a data URL, when the game has cached one.
    pub icon: Option<String>,
}

/// Minimal reader for Minecraft's NBT format, enough for servers.dat.
mod nbt {
    use serde_json::{Map, Value};

    pub struct Reader<'a> {
        b: &'a [u8],
        i: usize,
    }

    impl<'a> Reader<'a> {
        pub fn new(b: &'a [u8]) -> Self {
            Self { b, i: 0 }
        }
        fn take(&mut self, n: usize) -> Option<&'a [u8]> {
            let s = self.b.get(self.i..self.i.checked_add(n)?)?;
            self.i += n;
            Some(s)
        }
        fn u8(&mut self) -> Option<u8> {
            self.take(1).map(|s| s[0])
        }
        fn i16(&mut self) -> Option<i16> {
            self.take(2).map(|s| i16::from_be_bytes([s[0], s[1]]))
        }
        fn i32(&mut self) -> Option<i32> {
            self.take(4).map(|s| i32::from_be_bytes(s.try_into().unwrap()))
        }
        fn string(&mut self) -> Option<String> {
            let n = self.i16()? as u16 as usize;
            self.take(n).map(|s| String::from_utf8_lossy(s).into_owned())
        }
        fn count(&mut self) -> Option<usize> {
            usize::try_from(self.i32()?).ok().filter(|n| *n <= self.b.len())
        }
        /// The root: one named compound.
        pub fn root(&mut self) -> Option<Value> {
            if self.u8()? != 10 {
                return None;
            }
            self.string()?;
            self.payload(10, 0)
        }
        fn payload(&mut self, tag: u8, depth: u32) -> Option<Value> {
            if depth > 64 {
                return None;
            }
            Some(match tag {
                1 => Value::from(self.u8()? as i8),
                2 => Value::from(self.i16()?),
                3 => Value::from(self.i32()?),
                4 => Value::from(i64::from_be_bytes(self.take(8)?.try_into().ok()?)),
                5 => Value::from(f32::from_be_bytes(self.take(4)?.try_into().ok()?) as f64),
                6 => Value::from(f64::from_be_bytes(self.take(8)?.try_into().ok()?)),
                7 => {
                    let n = self.count()?;
                    self.take(n)?;
                    Value::Null
                }
                8 => Value::from(self.string()?),
                9 => {
                    let inner = self.u8()?;
                    let n = self.count()?;
                    let mut list = Vec::new();
                    for _ in 0..n {
                        list.push(self.payload(inner, depth + 1)?);
                    }
                    Value::Array(list)
                }
                10 => {
                    let mut map = Map::new();
                    loop {
                        let t = self.u8()?;
                        if t == 0 {
                            break;
                        }
                        let name = self.string()?;
                        map.insert(name, self.payload(t, depth + 1)?);
                    }
                    Value::Object(map)
                }
                11 => {
                    let n = self.count()?;
                    self.take(n.checked_mul(4)?)?;
                    Value::Null
                }
                12 => {
                    let n = self.count()?;
                    self.take(n.checked_mul(8)?)?;
                    Value::Null
                }
                _ => return None,
            })
        }
    }
}

fn parse_servers(bytes: &[u8]) -> Vec<Server> {
    let Some(root) = nbt::Reader::new(bytes).root() else { return vec![] };
    root["servers"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|s| s["hidden"].as_i64() != Some(1))
        .filter_map(|s| {
            let ip = s["ip"].as_str()?.trim().to_string();
            (!ip.is_empty()).then(|| Server {
                name: s["name"].as_str().filter(|n| !n.is_empty()).unwrap_or(&ip).to_string(),
                icon: s["icon"]
                    .as_str()
                    .filter(|i| i.chars().all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c)))
                    .map(|i| format!("data:image/png;base64,{i}")),
                ip,
            })
        })
        .collect()
}

/// Servers saved in this profile's multiplayer list.
pub fn servers(paths: &Paths, p: &Profile) -> Vec<Server> {
    std::fs::read(paths.instance(&p.id).join("servers.dat")).map(|b| parse_servers(&b)).unwrap_or_default()
}

/* ---------- textures from the game jars ---------- */

/// Client jars on disk, the one for `prefer` first, then newest first.
fn jars(paths: &Paths, prefer: Option<&str>) -> Vec<PathBuf> {
    let mut all: Vec<PathBuf> = std::fs::read_dir(paths.versions())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| {
            let id = e.file_name().to_string_lossy().to_string();
            e.path().join(format!("{id}.jar"))
        })
        .filter(|j| j.exists())
        .collect();
    all.sort_by_key(|j| -mtime(j));
    if let Some(id) = prefer {
        let wanted = paths.version_jar(id);
        if let Some(i) = all.iter().position(|j| *j == wanted) {
            let j = all.remove(i);
            all.insert(0, j);
        }
    }
    all
}

fn extract(jar: &Path, entry: &str, dest: &Path) -> Result<bool> {
    if dest.exists() {
        return Ok(true);
    }
    let mut zip = zip::ZipArchive::new(std::fs::File::open(jar)?)?;
    let Ok(mut e) = zip.by_name(entry) else { return Ok(false) };
    let mut buf = Vec::new();
    e.read_to_end(&mut buf)?;
    if let Some(d) = dest.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = dest.with_extension("tmp");
    std::fs::write(&tmp, buf)?;
    std::fs::rename(tmp, dest)?;
    Ok(true)
}

/// A real image, not one of the 1×1 placeholders newer jars ship in place of their art.
fn png_ok(path: &Path) -> bool {
    let mut head = [0u8; 24];
    std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut head)).is_ok()
        && crate::auth::png_size(&head).is_some_and(|(w, h)| w > 1 && h > 1)
}

/// Copies an asset by name out of the version's asset index (where newer versions keep their art).
fn from_assets(paths: &Paths, version_id: &str, name: &str, dest: &Path) -> Result<bool> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct V {
        asset_index: Option<Idx>,
    }
    #[derive(serde::Deserialize)]
    struct Idx {
        id: String,
    }
    let Ok(bytes) = std::fs::read(paths.version_json(version_id)) else { return Ok(false) };
    let Some(idx) = serde_json::from_slice::<V>(&bytes)?.asset_index else { return Ok(false) };
    let Ok(index) = std::fs::read(paths.assets().join("indexes").join(format!("{}.json", idx.id))) else { return Ok(false) };
    let index: serde_json::Value = serde_json::from_slice(&index)?;
    let Some(hash) = index["objects"][name]["hash"].as_str() else { return Ok(false) };
    if hash.len() < 3 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(false);
    }
    let src = paths.assets().join("objects").join(&hash[..2]).join(hash);
    if !src.exists() {
        return Ok(false);
    }
    if let Some(d) = dest.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::copy(src, dest)?;
    Ok(true)
}

/// The six cube faces of the title-screen panorama, preferably from `mc` itself.
/// Returns `(version id the art comes from, face paths)` or None before any version is installed.
pub fn panorama(paths: &Paths, mc: &str) -> Result<Option<(String, Vec<String>)>> {
    for jar in jars(paths, Some(mc)) {
        let id = jar.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let dir = paths.cache().join("panorama").join(&id);
        let faces: Vec<PathBuf> = (0..6).map(|n| dir.join(format!("panorama_{n}.png"))).collect();
        let mut ok = true;
        for (n, dest) in faces.iter().enumerate() {
            if png_ok(dest) {
                continue;
            }
            let _ = std::fs::remove_file(dest);
            let name = format!("minecraft/textures/gui/title/background/panorama_{n}.png");
            let found = from_assets(paths, &id, &name, dest)? && png_ok(dest);
            let found = found || {
                let _ = std::fs::remove_file(dest);
                extract(&jar, &format!("assets/{name}"), dest).with_context(|| format!("reading {}", jar.display()))? && png_ok(dest)
            };
            if !found {
                ok = false;
                break;
            }
        }
        if ok {
            return Ok(Some((id, faces.iter().map(|f| f.display().to_string()).collect())));
        }
    }
    Ok(None)
}

/// The default skin, taken from any installed jar.
pub fn steve(paths: &Paths) -> Option<PathBuf> {
    let dest = paths.cache().join("skins").join("steve.png");
    for jar in jars(paths, None) {
        for entry in ["assets/minecraft/textures/entity/player/wide/steve.png", "assets/minecraft/textures/entity/steve.png"] {
            if extract(&jar, entry, &dest).unwrap_or(false) {
                return Some(dest);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn servers_dat_is_read() {
        // root compound "" { servers: list<compound> [ {name:"Hypixel", ip:"mc.hypixel.net", icon:"iVBOR"} , {ip:"localhost", hidden:1b} ] }
        fn s(b: &mut Vec<u8>, v: &str) {
            b.extend((v.len() as u16).to_be_bytes());
            b.extend(v.as_bytes());
        }
        let mut b = vec![10];
        s(&mut b, "");
        b.push(9);
        s(&mut b, "servers");
        b.push(10);
        b.extend(2i32.to_be_bytes());
        for (k, v) in [("name", "Hypixel"), ("ip", "mc.hypixel.net"), ("icon", "iVBOR")] {
            b.push(8);
            s(&mut b, k);
            s(&mut b, v);
        }
        b.push(0);
        b.push(8);
        s(&mut b, "ip");
        s(&mut b, "localhost");
        b.push(1);
        s(&mut b, "hidden");
        b.push(1);
        b.push(0);
        b.push(0);
        let list = parse_servers(&b);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Hypixel");
        assert_eq!(list[0].ip, "mc.hypixel.net");
        assert_eq!(list[0].icon.as_deref(), Some("data:image/png;base64,iVBOR"));
        assert!(parse_servers(b"garbage").is_empty());
    }

    fn profile(mc: &str, loader: &str) -> Profile {
        serde_json::from_value(serde_json::json!({
            "id": "t", "name": "t", "mc": mc, "loader": loader, "memory_mb": 1024, "color": "grass", "created": 0
        }))
        .unwrap()
    }

    #[test]
    fn resource_packs_and_shaders_round_trip() {
        let root = std::env::temp_dir().join(format!("agate-test-{}", std::process::id()));
        let paths = Paths { root: root.clone() };
        let p = profile("1.20.1", "fabric");
        std::fs::create_dir_all(paths.instance("t")).unwrap();
        std::fs::write(paths.instance("t").join("options.txt"), "fov:0.5\nresourcePacks:[\"vanilla\",\"fabric\"]\n").unwrap();

        set_resource_pack(&paths, &p, "Faithful 32x.zip", true).unwrap();
        assert_eq!(active_resource_packs(&paths, &p), vec!["Faithful 32x.zip"]);
        let text = std::fs::read_to_string(paths.instance("t").join("options.txt")).unwrap();
        assert!(text.contains("fov:0.5"));
        assert!(text.contains(r#"resourcePacks:["vanilla","fabric","file/Faithful 32x.zip"]"#), "{text}");
        set_resource_pack(&paths, &p, "Faithful 32x.zip", false).unwrap();
        assert!(active_resource_packs(&paths, &p).is_empty());

        assert_eq!(active_shader(&paths, &p), None);
        set_shader(&paths, &p, Some("BSL_v8.4.zip")).unwrap();
        assert_eq!(active_shader(&paths, &p).as_deref(), Some("BSL_v8.4.zip"));
        set_shader(&paths, &p, None).unwrap();
        assert_eq!(active_shader(&paths, &p), None);

        let of = profile("1.20.1", "optifine");
        set_shader(&paths, &of, Some("Sildurs.zip")).unwrap();
        assert_eq!(active_shader(&paths, &of).as_deref(), Some("Sildurs.zip"));
        let _ = std::fs::remove_dir_all(root);
    }
}
