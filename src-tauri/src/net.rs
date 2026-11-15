//! HTTP client, verified downloads and parallel download batches with progress.

use anyhow::{anyhow, bail, Context, Result};
use futures::StreamExt;
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

pub const USER_AGENT: &str = concat!("agate-launcher/", env!("CARGO_PKG_VERSION"));

pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(60))
            .build()
            .expect("http client")
    })
}

pub async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T> {
    let resp = client().get(url).send().await.with_context(|| format!("GET {url}"))?;
    let status = resp.status();
    if !status.is_success() {
        bail!("GET {url} returned {status}");
    }
    resp.json::<T>().await.with_context(|| format!("decoding {url}"))
}

pub async fn get_text(url: &str) -> Result<String> {
    let resp = client().get(url).send().await.with_context(|| format!("GET {url}"))?;
    let status = resp.status();
    if !status.is_success() {
        bail!("GET {url} returned {status}");
    }
    Ok(resp.text().await?)
}

/// One file to fetch. `sha1`/`size` are verified when known.
#[derive(Clone, Debug)]
pub struct Download {
    pub url: String,
    pub path: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

impl Download {
    /// A file counts as present when it exists and its size matches (when known).
    /// Full hashing of every file on every launch is too slow; hashes are checked on download.
    fn is_present(&self) -> bool {
        match std::fs::metadata(&self.path) {
            Ok(m) if m.is_file() => self.size.map_or(m.len() > 0, |s| m.len() == s),
            _ => false,
        }
    }
}

/// Download to a temp file next to the target, verify, then rename into place
/// so an interrupted download never leaves a truncated file that looks valid.
pub async fn fetch(dl: &Download, bytes: &AtomicU64) -> Result<()> {
    if let Some(parent) = dl.path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = dl.path.with_extension("part");
    let mut last_err = anyhow!("no attempt");
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(600 * attempt)).await;
        }
        match fetch_once(dl, &tmp, bytes).await {
            Ok(()) => {
                tokio::fs::rename(&tmp, &dl.path).await?;
                return Ok(());
            }
            Err(e) => {
                let _ = tokio::fs::remove_file(&tmp).await;
                last_err = e;
            }
        }
    }
    Err(last_err.context(format!("downloading {}", dl.url)))
}

async fn fetch_once(dl: &Download, tmp: &Path, bytes: &AtomicU64) -> Result<()> {
    let resp = client().get(&dl.url).send().await?;
    if !resp.status().is_success() {
        bail!("{} returned {}", dl.url, resp.status());
    }
    let mut file = tokio::fs::File::create(tmp).await?;
    let mut hasher = Sha1::new();
    let mut stream = resp.bytes_stream();
    let mut written = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        written += chunk.len() as u64;
        bytes.fetch_add(chunk.len() as u64, Ordering::Relaxed);
    }
    file.flush().await?;
    drop(file);
    if let Some(expected) = &dl.sha1 {
        let got = hex::encode(hasher.finalize());
        if !got.eq_ignore_ascii_case(expected) {
            bytes.fetch_sub(written, Ordering::Relaxed);
            bail!("checksum mismatch for {}", dl.url);
        }
    }
    Ok(())
}

/// Progress callback: (files_done, files_total, bytes_done, bytes_total).
pub type Progress = Arc<dyn Fn(u64, u64, u64, u64) + Send + Sync>;

/// Fetch every missing file with bounded parallelism, reporting progress.
pub async fn fetch_all(items: Vec<Download>, parallel: usize, progress: Progress) -> Result<()> {
    let missing: Vec<Download> = items.into_iter().filter(|d| !d.is_present()).collect();
    let total = missing.len() as u64;
    let total_bytes: u64 = missing.iter().filter_map(|d| d.size).sum();
    if total == 0 {
        return Ok(());
    }
    let bytes = Arc::new(AtomicU64::new(0));
    let done = Arc::new(AtomicU64::new(0));
    progress(0, total, 0, total_bytes);

    // Ticker so large single files (client jar, Java) still show movement.
    let ticker = {
        let (bytes, done, progress) = (bytes.clone(), done.clone(), progress.clone());
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                progress(done.load(Ordering::Relaxed), total, bytes.load(Ordering::Relaxed), total_bytes);
            }
        })
    };

    let results: Vec<Result<()>> = futures::stream::iter(missing)
        .map(|dl| {
            let (bytes, done) = (bytes.clone(), done.clone());
            async move {
                let r = fetch(&dl, &bytes).await;
                done.fetch_add(1, Ordering::Relaxed);
                r
            }
        })
        .buffer_unordered(parallel)
        .collect()
        .await;
    ticker.abort();
    progress(total, total, bytes.load(Ordering::Relaxed), total_bytes);
    let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();
    if let Some(first) = errors.into_iter().next() {
        return Err(first);
    }
    Ok(())
}
