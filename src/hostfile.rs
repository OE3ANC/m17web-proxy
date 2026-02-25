use std::collections::HashMap;

use log::{info, warn};
use serde::Deserialize;

/// User-Agent string for hostfile HTTP requests.
/// Must be descriptive and unique per the hostfile server requirements.
const USER_AGENT: &str = concat!(
    "m17web-proxy/",
    env!("CARGO_PKG_VERSION"),
    " (M17 Web Proxy; hostfile-fetch)"
);

/// Top-level JSON structure of the M17Hosts.json file.
#[derive(Deserialize, Debug)]
pub struct M17HostFile {
    #[allow(dead_code)]
    pub _refcheck_metadata: Option<serde_json::Value>,
    pub reflectors: Vec<HostEntry>,
}

/// A single reflector entry from the M17 host file.
#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct HostEntry {
    pub designator: String,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub url: Option<String>,
    pub dns: Option<String>,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub port: Option<u16>,
    pub sponsor: Option<String>,
    pub country: Option<String>,
    pub ip_source: Option<String>,
    pub dns_cache_updated_at: Option<String>,
    pub last_verified_at: Option<String>,
}

/// Cached host file data, keyed by uppercase designator (e.g. "M17-XOR").
pub struct HostFileCache {
    entries: HashMap<String, HostEntry>,
}

impl HostFileCache {
    /// Look up a reflector by designator and return its address as "ip:port".
    pub fn resolve(&self, designator: &str) -> Option<String> {
        let key = designator.to_uppercase();
        self.entries.get(&key).and_then(|entry| {
            // Port is required for a valid address
            let port = entry.port?;

            // Prefer IPv4, fall back to IPv6
            let ip = entry
                .ipv4
                .as_deref()
                .filter(|s| !s.is_empty())
                .or(entry.ipv6.as_deref().filter(|s| !s.is_empty()));

            ip.map(|addr| {
                if addr.contains(':') {
                    // IPv6 addresses must be wrapped in brackets for socket notation
                    format!("[{}]:{}", addr, port)
                } else {
                    format!("{}:{}", addr, port)
                }
            })
        })
    }
}

/// Fetch the M17 host file from the given URL and return a cache.
pub async fn fetch_hostfile(url: &str) -> Result<HostFileCache, String> {
    info!("Hostfile: Fetching from {} (User-Agent: {})", url, USER_AGENT);

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch hostfile: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Hostfile server returned HTTP {}",
            response.status()
        ));
    }

    let hostfile: M17HostFile = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse hostfile JSON: {}", e))?;

    let count = hostfile.reflectors.len();
    let mut entries = HashMap::with_capacity(count);

    for entry in hostfile.reflectors {
        let key = entry.designator.to_uppercase();
        entries.insert(key, entry);
    }

    info!("Hostfile: Loaded {} reflector entries", count);

    Ok(HostFileCache { entries })
}

/// Attempt to resolve a reflector address, first from DHT, then from the hostfile cache.
/// Logs appropriate warnings when DHT fails and fallback is used.
pub fn resolve_from_hostfile(
    cache: &Option<HostFileCache>,
    designator: &str,
) -> Option<String> {
    match cache {
        Some(c) => match c.resolve(designator) {
            Some(addr) => {
                info!(
                    "Hostfile: Resolved {} -> {} (fallback)",
                    designator, addr
                );
                Some(addr)
            }
            None => {
                warn!(
                    "Hostfile: Reflector {} not found in hostfile either",
                    designator
                );
                None
            }
        },
        None => {
            warn!(
                "Hostfile: No hostfile cache available, cannot resolve {}",
                designator
            );
            None
        }
    }
}
