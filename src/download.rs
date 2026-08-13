//! Optional tokenizer download.
//!
//! Downloads `tokenizer.json.xz` assets from the `ljh-sh/tokenizer-json`
//! GitHub release `data`. The fallback strategy uses the same mirror endpoints
//! as `x-bash/eget`, but `ta` does not depend on x-cmd or the `eget` CLI:
//!
//! 1. Try `github.com` directly with a speed-based stall detector.
//! 2. If direct is slow or fails, fall back to the eget hosted mirror
//!    (`https://eget.ljh.sh/gh/...`).
//! 3. If `GHPROXY_ENDPOINT` is set, also try that mirror.
//!
//! Decompressed files are stored under `~/.local/data/tokenizer-json/`.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const OWNER: &str = "ljh-sh";
const REPO: &str = "tokenizer-json";
const TAG: &str = "data";

const RECOMMENDED_IDS: &[&str] = &["qwen2_5", "llama3", "deepseek_v3"];

/// Default download timeout for direct GitHub downloads.
const DOWNLOAD_TIMEOUT_SECONDS: u64 = 60;

/// Minimum acceptable download speed in bytes/second. Below this the direct
/// download is aborted and we fall back to mirrors.
const MIN_SPEED_BYTES_PER_SECOND: u64 = 1024;

/// Returns the list of recommended tokenizer IDs.
pub fn recommended_ids() -> &'static [&'static str] {
    RECOMMENDED_IDS
}

/// Resolve the data directory for downloaded tokenizers.
///
/// On Unix this is `~/.local/data/tokenizer-json/`; on Windows it falls back
/// to `%LOCALAPPDATA%\tokenizer-json`.
pub fn data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir()
            .map(|d| d.join("tokenizer-json"))
            .unwrap_or_else(|| PathBuf::from(r"C:\tokenizer-json"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::home_dir()
            .map(|h| h.join(".local/data/tokenizer-json"))
            .unwrap_or_else(|| PathBuf::from(".local/data/tokenizer-json"))
    }
}

/// Information about a download result.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub id: String,
    pub path: PathBuf,
    pub method: DownloadMethod,
    pub bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadMethod {
    Github,
    EgetMirror,
    Ghproxy,
    Cached,
}

impl std::fmt::Display for DownloadMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadMethod::Github => write!(f, "github"),
            DownloadMethod::EgetMirror => write!(f, "eget-mirror"),
            DownloadMethod::Ghproxy => write!(f, "ghproxy"),
            DownloadMethod::Cached => write!(f, "cached"),
        }
    }
}

/// Download one or more tokenizer IDs.
pub fn download_ids(ids: &[String], force: bool) -> anyhow::Result<Vec<DownloadResult>> {
    let dir = data_dir();
    fs::create_dir_all(&dir)?;

    let mut results = Vec::with_capacity(ids.len());
    for id in ids {
        let res = download_one(id, &dir, force)?;
        results.push(res);
    }
    Ok(results)
}

/// Download recommended tokenizer IDs.
pub fn download_recommended(force: bool) -> anyhow::Result<Vec<DownloadResult>> {
    let ids: Vec<String> = RECOMMENDED_IDS.iter().map(|s| s.to_string()).collect();
    download_ids(&ids, force)
}

fn asset_name(id: &str) -> String {
    format!("{}.tokenizer.json.xz", id)
}

fn github_url(id: &str) -> String {
    format!(
        "https://github.com/{}/{}/releases/download/{}/{}",
        OWNER,
        REPO,
        TAG,
        asset_name(id)
    )
}

fn eget_mirror_url(id: &str) -> String {
    let host = std::env::var("EGET_MIRROR_HOST").unwrap_or_else(|_| "eget.ljh.sh".to_string());
    let prefix = std::env::var("EGET_MIRROR_PATH").unwrap_or_else(|_| "gh".to_string());
    let prefix = prefix.trim_start_matches('/');
    let prefix = if prefix.is_empty() { "gh" } else { prefix };
    format!(
        "https://{}/{}/{}/{}/releases/download/{}/{}",
        host,
        prefix,
        OWNER,
        REPO,
        TAG,
        asset_name(id)
    )
}

fn ghproxy_url(original: &str) -> Option<String> {
    let endpoint = std::env::var("GHPROXY_ENDPOINT").ok()?;
    let ep = endpoint.trim_end_matches('/');
    if ep.is_empty() {
        return None;
    }
    if ep.ends_with("/github.com") {
        Some(original.replacen("https://github.com", ep, 1))
    } else {
        Some(format!("{}/{}", ep, original))
    }
}

fn download_one(id: &str, dir: &Path, force: bool) -> anyhow::Result<DownloadResult> {
    let json_path = dir.join(format!("{}.tokenizer.json", id));

    if !force && json_path.exists() {
        let meta = fs::metadata(&json_path)?;
        return Ok(DownloadResult {
            id: id.to_string(),
            path: json_path,
            method: DownloadMethod::Cached,
            bytes: meta.len(),
        });
    }

    let xz_path = dir.join(asset_name(id));

    // 1. Direct GitHub.
    let url = github_url(id);
    if let Ok(bytes) = try_download(&url, &xz_path, DownloadMethod::Github) {
        decompress(&xz_path, &json_path)?;
        let _ = fs::remove_file(&xz_path);
        return Ok(DownloadResult {
            id: id.to_string(),
            path: json_path,
            method: DownloadMethod::Github,
            bytes,
        });
    }

    // 2. eget hosted mirror.
    let mirror_url = eget_mirror_url(id);
    if let Ok(bytes) = try_download(&mirror_url, &xz_path, DownloadMethod::EgetMirror) {
        decompress(&xz_path, &json_path)?;
        let _ = fs::remove_file(&xz_path);
        return Ok(DownloadResult {
            id: id.to_string(),
            path: json_path,
            method: DownloadMethod::EgetMirror,
            bytes,
        });
    }

    // 3. User-configured GHPROXY endpoint.
    if let Some(proxy_url) = ghproxy_url(&url) {
        if let Ok(bytes) = try_download(&proxy_url, &xz_path, DownloadMethod::Ghproxy) {
            decompress(&xz_path, &json_path)?;
            let _ = fs::remove_file(&xz_path);
            return Ok(DownloadResult {
                id: id.to_string(),
                path: json_path,
                method: DownloadMethod::Ghproxy,
                bytes,
            });
        }
    }

    Err(anyhow::anyhow!(
        "failed to download {} from github.com, eget mirror, and ghproxy",
        id
    ))
}

fn try_download(url: &str, dest: &Path, method: DownloadMethod) -> anyhow::Result<u64> {
    let label = method.to_string();
    eprintln!("trying {}: {}", label, url);
    let start = Instant::now();

    let mut reader = ureq::get(url)
        .timeout(Duration::from_secs(DOWNLOAD_TIMEOUT_SECONDS))
        .call()
        .map_err(|e| anyhow::anyhow!("{} GET failed: {}", label, e))?
        .into_reader();

    let mut file = fs::File::create(dest)?;
    let mut buf = [0u8; 8192];
    let mut total: u64 = 0;
    let mut last_report = Instant::now();

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        total += n as u64;

        // Stall detection: if average speed is too low for too long, abort.
        let elapsed = start.elapsed().as_secs().max(1);
        if total / elapsed < MIN_SPEED_BYTES_PER_SECOND {
            return Err(anyhow::anyhow!(
                "{} download too slow: {} B/s",
                label,
                total / elapsed
            ));
        }

        // Progress report every 2 seconds.
        if last_report.elapsed() > Duration::from_secs(2) {
            eprintln!("{} downloaded {} bytes", label, total);
            last_report = Instant::now();
        }
    }

    file.flush()?;
    eprintln!("{} finished {} bytes in {:?}", label, total, start.elapsed());
    Ok(total)
}

fn decompress(xz_path: &Path, json_path: &Path) -> anyhow::Result<()> {
    let xz_file = fs::File::open(xz_path)?;
    let mut json_file = fs::File::create(json_path)?;
    lzma_rs::xz_decompress(&mut std::io::BufReader::new(xz_file), &mut json_file)?;
    json_file.flush()?;
    Ok(())
}
