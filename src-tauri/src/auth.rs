//! Accounts: Microsoft (device code → Xbox Live → XSTS → Minecraft) and offline names.
//!
//! Without a custom Azure client id, the login uses the public Minecraft launcher
//! client id on the login.live.com device-code endpoint (the approach used by
//! several open-source launchers). Users can set their own Mojang-approved Azure
//! app id in Settings, which switches to the Microsoft identity platform endpoints.

use crate::app::{read_json, write_json, Paths};
use crate::net::client;
use anyhow::{anyhow, bail, Context, Result};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const LIVE_CLIENT_ID: &str = "00000000402b5328";
const LIVE_SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";
const AZURE_SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Account {
    pub uuid: String,
    pub name: String,
    /// "msa" or "offline".
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mc_token: Option<String>,
    #[serde(default)]
    pub mc_expires: i64,
    #[serde(default)]
    pub xuid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin_url: Option<String>,
    /// "CLASSIC" or "SLIM".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin_variant: Option<String>,
    /// Texture URL of the cape shown on the account, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cape_url: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Accounts {
    pub active: Option<String>,
    pub list: Vec<Account>,
}

/// What the UI sees: no tokens.
#[derive(Serialize, Clone)]
pub struct AccountView {
    pub uuid: String,
    pub name: String,
    pub kind: String,
    pub active: bool,
}

pub fn load(paths: &Paths) -> Accounts {
    read_json(&paths.file("accounts.json"))
}
pub fn save(paths: &Paths, a: &Accounts) -> Result<()> {
    write_json(&paths.file("accounts.json"), a)
}

pub fn views(a: &Accounts) -> Vec<AccountView> {
    a.list
        .iter()
        .map(|x| AccountView {
            uuid: x.uuid.clone(),
            name: x.name.clone(),
            kind: x.kind.clone(),
            active: a.active.as_deref() == Some(&x.uuid),
        })
        .collect()
}

/// Java's `UUID.nameUUIDFromBytes("OfflinePlayer:" + name)`, as vanilla servers compute it.
pub fn offline_uuid(name: &str) -> String {
    let mut h = Md5::digest(format!("OfflinePlayer:{name}").as_bytes());
    h[6] = (h[6] & 0x0f) | 0x30;
    h[8] = (h[8] & 0x3f) | 0x80;
    hex::encode(h)
}

pub fn valid_offline_name(name: &str) -> bool {
    (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceCode {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
    #[serde(default)]
    pub message: Option<String>,
}

fn endpoints(client_id: Option<&str>) -> (&str, &'static str, &'static str, &'static str) {
    match client_id {
        Some(id) => (
            id,
            "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode",
            "https://login.microsoftonline.com/consumers/oauth2/v2.0/token",
            AZURE_SCOPE,
        ),
        None => (
            LIVE_CLIENT_ID,
            "https://login.live.com/oauth20_connect.srf",
            "https://login.live.com/oauth20_token.srf",
            LIVE_SCOPE,
        ),
    }
}

pub async fn start_device_code(client_id: Option<&str>) -> Result<DeviceCode> {
    let (id, url, _, scope) = endpoints(client_id);
    let mut form = vec![("client_id", id), ("scope", scope)];
    if client_id.is_none() {
        form.push(("response_type", "device_code"));
    }
    let resp = client().post(url).form(&form).send().await?;
    let status = resp.status();
    let body: Value = resp.json().await?;
    if !status.is_success() {
        bail!("Microsoft refused the login request: {}", body["error_description"].as_str().unwrap_or("unknown error"));
    }
    Ok(serde_json::from_value(body)?)
}

pub enum Poll {
    Pending,
    Done(Account),
}

#[derive(Deserialize)]
struct MsToken {
    access_token: String,
    refresh_token: Option<String>,
}

/// One poll of the token endpoint; completes the whole Minecraft chain on success.
pub async fn poll_device_code(client_id: Option<&str>, device_code: &str) -> Result<Poll> {
    let (id, _, token_url, _) = endpoints(client_id);
    let resp = client()
        .post(token_url)
        .form(&[
            ("client_id", id),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
        ])
        .send()
        .await?;
    let status = resp.status();
    let body: Value = resp.json().await?;
    if !status.is_success() {
        return match body["error"].as_str() {
            Some("authorization_pending") | Some("slow_down") => Ok(Poll::Pending),
            Some("expired_token") => bail!("The code expired. Start the login again."),
            Some("access_denied") | Some("authorization_declined") => bail!("Login was cancelled in the browser."),
            _ => bail!("Login failed: {}", body["error_description"].as_str().unwrap_or("unknown error")),
        };
    }
    let token: MsToken = serde_json::from_value(body)?;
    let acc = minecraft_chain(client_id, &token).await?;
    Ok(Poll::Done(acc))
}

async fn minecraft_chain(client_id: Option<&str>, ms: &MsToken) -> Result<Account> {
    // Xbox Live user token. The ticket prefix depends on the token flavour; try the expected one first.
    let prefixes: &[&str] = if client_id.is_some() { &["d=", ""] } else { &["", "t=", "d="] };
    let mut xbl: Option<Value> = None;
    for p in prefixes {
        let resp = client()
            .post("https://user.auth.xboxlive.com/user/authenticate")
            .json(&json!({
                "Properties": { "AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("{p}{}", ms.access_token) },
                "RelyingParty": "http://auth.xboxlive.com",
                "TokenType": "JWT"
            }))
            .send()
            .await?;
        if resp.status().is_success() {
            xbl = Some(resp.json().await?);
            break;
        }
    }
    let xbl = xbl.ok_or_else(|| anyhow!("Xbox Live rejected the Microsoft token"))?;
    let xbl_token = xbl["Token"].as_str().context("no Xbox token")?;

    let resp = client()
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await?;
    let status = resp.status();
    let xsts: Value = resp.json().await?;
    if !status.is_success() {
        let msg = match xsts["XErr"].as_u64() {
            Some(2148916233) => "This Microsoft account has no Xbox profile. Sign in once at xbox.com, then retry.",
            Some(2148916235) => "Xbox Live is not available in this account's country.",
            Some(2148916238) => "This is a child account: an adult must add it to a Microsoft family first.",
            _ => "Xbox Live authorization failed.",
        };
        bail!(msg);
    }
    let xsts_token = xsts["Token"].as_str().context("no XSTS token")?;
    let uhs = xsts["DisplayClaims"]["xui"][0]["uhs"].as_str().context("no user hash")?;

    let resp = client()
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&json!({ "identityToken": format!("XBL3.0 x={uhs};{xsts_token}") }))
        .send()
        .await?;
    if !resp.status().is_success() {
        let s = resp.status();
        let t = resp.text().await.unwrap_or_default();
        if s.as_u16() == 403 {
            bail!("Minecraft services refused this client id (403). Set a Mojang-approved Azure client id in Settings.");
        }
        bail!("Minecraft login failed ({s}): {t}");
    }
    let mc: Value = resp.json().await?;
    let mc_token = mc["access_token"].as_str().context("no Minecraft token")?.to_string();
    let expires = chrono::Utc::now().timestamp() + mc["expires_in"].as_i64().unwrap_or(86400) - 300;

    let resp = client()
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&mc_token)
        .send()
        .await?;
    if resp.status().as_u16() == 404 {
        bail!("This Microsoft account does not own Minecraft Java Edition.");
    }
    let profile: Value = resp.error_for_status()?.json().await?;
    let mut acc = Account {
        uuid: profile["id"].as_str().context("no profile id")?.to_string(),
        name: profile["name"].as_str().context("no profile name")?.to_string(),
        kind: "msa".into(),
        refresh_token: ms.refresh_token.clone(),
        client_id: client_id.map(String::from),
        mc_token: Some(mc_token),
        mc_expires: expires,
        xuid: xsts["DisplayClaims"]["xui"][0]["xid"].as_str().unwrap_or("").to_string(),
        ..Default::default()
    };
    apply_profile(&mut acc, &profile);
    Ok(acc)
}

/// A valid Minecraft token for `acc`, refreshing through Microsoft when expired.
pub async fn ensure_fresh(acc: &Account) -> Result<Account> {
    if acc.kind != "msa" || (acc.mc_token.is_some() && acc.mc_expires > chrono::Utc::now().timestamp()) {
        return Ok(acc.clone());
    }
    let refresh = acc.refresh_token.as_deref().ok_or_else(|| anyhow!("Session expired: sign in again"))?;
    let cid = acc.client_id.as_deref();
    let (id, _, token_url, scope) = endpoints(cid);
    let resp = client()
        .post(token_url)
        .form(&[("client_id", id), ("grant_type", "refresh_token"), ("refresh_token", refresh), ("scope", scope)])
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("Session expired: sign in again");
    }
    let token: MsToken = resp.json().await?;
    let mut fresh = minecraft_chain(cid, &token).await?;
    if fresh.refresh_token.is_none() {
        fresh.refresh_token = acc.refresh_token.clone();
    }
    Ok(fresh)
}

/// Copies the current skin and cape from a Minecraft profile response.
fn apply_profile(acc: &mut Account, profile: &Value) {
    let https = |u: &str| u.replacen("http://", "https://", 1);
    let skins = profile["skins"].as_array().cloned().unwrap_or_default();
    let skin = skins.iter().find(|s| s["state"] == "ACTIVE").or(skins.first());
    acc.skin_url = skin.and_then(|s| s["url"].as_str()).map(https);
    acc.skin_variant = skin.and_then(|s| s["variant"].as_str()).map(String::from);
    acc.cape_url = capes(profile).into_iter().find(|c| c.active).map(|c| c.url);
}

#[derive(serde::Serialize, Clone)]
pub struct Cape {
    pub id: String,
    pub alias: String,
    pub url: String,
    pub active: bool,
}

pub fn capes(profile: &Value) -> Vec<Cape> {
    profile["capes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|c| {
            Some(Cape {
                id: c["id"].as_str()?.to_string(),
                alias: c["alias"].as_str().unwrap_or("Cape").to_string(),
                url: c["url"].as_str()?.replacen("http://", "https://", 1),
                active: c["state"] == "ACTIVE",
            })
        })
        .collect()
}

/// The full Minecraft profile (skins and capes) for a signed-in account.
pub async fn fetch_profile(acc: &Account) -> Result<Value> {
    let token = acc.mc_token.clone().ok_or_else(|| anyhow!("not signed in"))?;
    Ok(client()
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

/// Shows one of the account's capes, or hides the cape with `None`.
pub async fn set_cape(acc: &mut Account, id: Option<&str>) -> Result<()> {
    let token = acc.mc_token.clone().ok_or_else(|| anyhow!("not signed in"))?;
    let url = "https://api.minecraftservices.com/minecraft/profile/capes/active";
    let req = match id {
        Some(id) => client().put(url).json(&json!({ "capeId": id })),
        None => client().delete(url),
    };
    let resp = req.bearer_auth(token).send().await?;
    if !resp.status().is_success() {
        bail!("Minecraft did not accept the cape change ({})", resp.status());
    }
    refresh_skin(acc).await
}

/// Re-reads the account's current skin from Minecraft services (it may have changed on the website).
pub async fn refresh_skin(acc: &mut Account) -> Result<()> {
    let profile = fetch_profile(acc).await?;
    apply_profile(acc, &profile);
    Ok(())
}

/// Width and height from a PNG header, if `data` is a PNG.
pub fn png_size(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 || &data[..8] != b"\x89PNG\r\n\x1a\n" || &data[12..16] != b"IHDR" {
        return None;
    }
    Some((u32::from_be_bytes(data[16..20].try_into().ok()?), u32::from_be_bytes(data[20..24].try_into().ok()?)))
}

/// Sets a new skin on the Microsoft account.
pub async fn upload_skin(acc: &mut Account, slim: bool, png: Vec<u8>) -> Result<()> {
    match png_size(&png) {
        Some((64, 64)) | Some((64, 32)) => {}
        _ => bail!("The skin must be a 64×64 PNG image (or a classic 64×32)."),
    }
    let token = acc.mc_token.clone().ok_or_else(|| anyhow!("not signed in"))?;
    let form = reqwest::multipart::Form::new()
        .text("variant", if slim { "slim" } else { "classic" })
        .part("file", reqwest::multipart::Part::bytes(png).file_name("skin.png").mime_str("image/png")?);
    let resp = client()
        .post("https://api.minecraftservices.com/minecraft/profile/skins")
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await?;
    if !resp.status().is_success() {
        let s = resp.status();
        bail!("Minecraft did not accept the skin ({s})");
    }
    let profile: Value = resp.json().await.unwrap_or(Value::Null);
    if profile["skins"].is_array() {
        apply_profile(acc, &profile);
    } else {
        refresh_skin(acc).await?;
    }
    Ok(())
}
