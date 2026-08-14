//! Optional tokenizer download.
//!
//! Downloads `tokenizer.json.xz` assets from the `ljh-sh/tokenizer-json`
//! GitHub release `data`. The fallback strategy uses the same mirror endpoints
//! as `x-bash/eget`, but `token-actuary` does not depend on x-cmd or the `eget` CLI:
//!
//! Default order:
//! 1. Try `github.com` directly with a speed-based stall detector.
//! 2. Fall back to the eget hosted mirror (`https://eget.ljh.sh/gh/...`).
//! 3. If `GHPROXY_ENDPOINT` is set, try that mirror.
//!
//! China-optimized order (`--china` or `TA_CHINA=1`):
//! 1. `GHPROXY_ENDPOINT` mirror if set.
//! 2. eget hosted mirror (`https://eget.ljh.sh/gh/...`).
//! 3. Built-in China-accessible GitHub proxies.
//! 4. `github.com` direct as last resort.
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

/// Built-in China-accessible GitHub proxy mirrors (ghproxy-style: prefix the
/// full GitHub URL).
const CHINA_PROXIES: &[&str] = &[
    "https://ghfast.top",
    "https://mirror.ghproxy.com",
    "https://gh-proxy.com",
];

/// Default download timeout.
const DOWNLOAD_TIMEOUT_SECONDS: u64 = 60;

/// Minimum acceptable download speed in bytes/second. Below this the current
/// attempt is aborted and we try the next source.
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

/// Download options.
pub struct Options {
    pub force: bool,
    pub china: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            force: false,
            china: std::env::var("TA_CHINA").is_ok_and(|v| !v.is_empty()),
        }
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
pub fn download_ids(ids: &[String], opts: &Options) -> anyhow::Result<Vec<DownloadResult>> {
    let dir = data_dir();
    fs::create_dir_all(&dir)?;

    let mut results = Vec::with_capacity(ids.len());
    for id in ids {
        let res = download_one(id, &dir, opts)?;
        results.push(res);
    }
    Ok(results)
}

/// Download recommended tokenizer IDs.
pub fn download_recommended(opts: &Options) -> anyhow::Result<Vec<DownloadResult>> {
    let ids: Vec<String> = RECOMMENDED_IDS.iter().map(|s| s.to_string()).collect();
    download_ids(&ids, opts)
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

fn eget_url(id: &str) -> String {
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

fn ghproxy_url(proxy: &str, original: &str) -> String {
    let proxy = proxy.trim_end_matches('/');
    if proxy.ends_with("/github.com") {
        original.replacen("https://github.com", proxy, 1)
    } else {
        format!("{}/{}", proxy, original)
    }
}

fn user_ghproxy_url(original: &str) -> Option<String> {
    let endpoint = std::env::var("GHPROXY_ENDPOINT").ok()?;
    let endpoint = endpoint.trim_end_matches('/');
    if endpoint.is_empty() {
        return None;
    }
    Some(ghproxy_url(endpoint, original))
}

fn download_urls(id: &str, china: bool) -> Vec<(DownloadMethod, String)> {
    let github = github_url(id);
    let eget = eget_url(id);
    let user_proxy = user_ghproxy_url(&github);
    let mut out = Vec::new();

    if china {
        // China-optimized: user proxy first, then eget, then built-in proxies,
        // then GitHub direct as last resort.
        if let Some(url) = user_proxy {
            out.push((DownloadMethod::Ghproxy, url));
        }
        out.push((DownloadMethod::EgetMirror, eget));
        for proxy in CHINA_PROXIES {
            out.push((DownloadMethod::Ghproxy, ghproxy_url(proxy, &github)));
        }
        out.push((DownloadMethod::Github, github));
    } else {
        // Default: direct first, then eget mirror, then user ghproxy.
        out.push((DownloadMethod::Github, github.clone()));
        out.push((DownloadMethod::EgetMirror, eget));
        if let Some(url) = user_proxy {
            out.push((DownloadMethod::Ghproxy, url));
        }
    }

    out
}

fn download_one(id: &str, dir: &Path, opts: &Options) -> anyhow::Result<DownloadResult> {
    let json_path = dir.join(format!("{}.tokenizer.json", id));

    if !opts.force && json_path.exists() {
        let meta = fs::metadata(&json_path)?;
        return Ok(DownloadResult {
            id: id.to_string(),
            path: json_path,
            method: DownloadMethod::Cached,
            bytes: meta.len(),
        });
    }

    let xz_path = dir.join(asset_name(id));
    let urls = download_urls(id, opts.china);

    for (method, url) in urls {
        match try_download(&url, &xz_path, method) {
            Ok(bytes) => {
                decompress(&xz_path, &json_path)?;
                let _ = fs::remove_file(&xz_path);
                return Ok(DownloadResult {
                    id: id.to_string(),
                    path: json_path,
                    method,
                    bytes,
                });
            }
            Err(e) => {
                eprintln!("{} failed: {}", method, e);
            }
        }
    }

    Err(anyhow::anyhow!(
        "failed to download {} from all available sources",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_urls_start_with_github() {
        let urls = download_urls("qwen2_5", false);
        assert!(!urls.is_empty());
        assert_eq!(urls[0].0, DownloadMethod::Github);
        assert!(urls[0].1.starts_with("https://github.com/"));
    }

    #[test]
    fn default_urls_include_eget_mirror() {
        let urls = download_urls("qwen2_5", false);
        let methods: Vec<_> = urls.iter().map(|u| u.0).collect();
        assert!(methods.contains(&DownloadMethod::EgetMirror));
    }

    #[test]
    fn china_urls_start_with_eget_mirror() {
        let urls = download_urls("qwen2_5", true);
        assert!(!urls.is_empty());
        assert_eq!(urls[0].0, DownloadMethod::EgetMirror);
        assert!(urls[0].1.starts_with("https://eget.ljh.sh/gh/"));
        assert!(!urls[0].1.contains("https://github.com"));
    }

    #[test]
    fn china_urls_end_with_github() {
        let urls = download_urls("qwen2_5", true);
        let last = urls.last().unwrap();
        assert_eq!(last.0, DownloadMethod::Github);
        assert!(last.1.starts_with("https://github.com/"));
    }

    #[test]
    fn china_urls_include_ghproxy_mirrors() {
        let urls = download_urls("qwen2_5", true);
        let ghproxy_count = urls
            .iter()
            .filter(|u| u.0 == DownloadMethod::Ghproxy)
            .count();
        assert!(ghproxy_count >= 1, "expected at least one ghproxy mirror");
    }

    #[test]
    fn ghproxy_url_prefixes_github() {
        let original = "https://github.com/ljh-sh/tokenizer-json/releases/download/data/qwen2_5.tokenizer.json.xz";
        assert_eq!(
            ghproxy_url("https://ghfast.top", original),
            "https://ghfast.top/https://github.com/ljh-sh/tokenizer-json/releases/download/data/qwen2_5.tokenizer.json.xz"
        );
    }

    #[test]
    fn ghproxy_url_replaces_github_host() {
        let original = "https://github.com/ljh-sh/tokenizer-json/releases/download/data/qwen2_5.tokenizer.json.xz";
        assert_eq!(
            ghproxy_url("https://mirror.example.com/github.com", original),
            "https://mirror.example.com/github.com/ljh-sh/tokenizer-json/releases/download/data/qwen2_5.tokenizer.json.xz"
        );
    }
}
