use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use reqwest::Client;
use std::net::{IpAddr, SocketAddr};
use std::ops::Range;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::net::lookup_host;
use tokio::sync::Semaphore;

use super::{FileInfo, IpVersion, Protocol};

#[derive(Clone)]
pub struct HttpProtocol {
    client: Client,
}

impl Default for HttpProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpProtocol {
    pub fn new() -> Self {
        Self::new_with_ip_version(IpVersion::Any)
    }

    pub fn new_with_ip_version(ip_version: IpVersion) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .local_address(Self::get_local_bind_address(ip_version))
            .build()
            .expect("Failed to create HTTP client");
        Self { client }
    }

    /// Get local address to bind to based on IP version preference
    fn get_local_bind_address(ip_version: IpVersion) -> Option<IpAddr> {
        match ip_version {
            IpVersion::Ipv4 => Some("0.0.0.0".parse().unwrap()),
            IpVersion::Ipv6 => Some("::".parse().unwrap()),
            IpVersion::Any => None,
        }
    }

    /// Resolve hostname to socket address with IP version preference
    #[allow(dead_code)]
    pub async fn resolve_host(
        host: &str,
        port: u16,
        ip_version: IpVersion,
    ) -> anyhow::Result<SocketAddr> {
        let addrs: Vec<SocketAddr> = lookup_host((host, port)).await?.collect();

        let selected = match ip_version {
            IpVersion::Ipv4 => addrs
                .into_iter()
                .find(|addr| matches!(addr.ip(), IpAddr::V4(_))),
            IpVersion::Ipv6 => addrs
                .into_iter()
                .find(|addr| matches!(addr.ip(), IpAddr::V6(_))),
            IpVersion::Any => addrs.into_iter().next(),
        };

        selected.ok_or_else(|| {
            anyhow::anyhow!(
                "No {} address found for host: {}",
                match ip_version {
                    IpVersion::Ipv4 => "IPv4",
                    IpVersion::Ipv6 => "IPv6",
                    IpVersion::Any => "suitable",
                },
                host
            )
        })
    }
}

#[async_trait]
impl Protocol for HttpProtocol {
    async fn upload(&self, _source: &Path, _dest: &str) -> anyhow::Result<u64> {
        anyhow::bail!("HTTP upload not implemented")
    }

    async fn download<F>(
        &self,
        source: &str,
        dest: &Path,
        parallel: usize,
        progress: F,
    ) -> anyhow::Result<u64>
    where
        F: Fn(u64) + Send + Sync + 'static,
    {
        let info = self.get_file_info(source).await?;
        let file_size = info.size;

        let dest_file = tokio::fs::File::create(dest).await?;
        dest_file.set_len(file_size).await?;
        drop(dest_file);

        let start = std::time::Instant::now();
        let progress = Arc::new(progress);

        if info.accepts_ranges && parallel > 1 {
            let chunk_size = (8 * 1024 * 1024).min(file_size as usize / parallel + 1);
            let num_chunks = (file_size as usize).div_ceil(chunk_size);

            let semaphore = Arc::new(Semaphore::new(parallel));
            let mut handles = FuturesUnordered::new();
            let bytes_downloaded = Arc::new(AtomicU64::new(0));

            for i in 0..num_chunks {
                let offset = i * chunk_size;
                let end = (offset + chunk_size).min(file_size as usize);
                let url = source.to_string();
                let dest_path = dest.to_path_buf();
                let semaphore = semaphore.clone();
                let client = self.client.clone();
                let bytes_counter = bytes_downloaded.clone();
                let progress_fn = progress.clone();

                handles.push(tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();

                    let response = client
                        .get(&url)
                        .header("Range", format!("bytes={}-{}", offset, end - 1))
                        .send()
                        .await?;

                    let data = response.bytes().await?;

                    let mut file = tokio::fs::OpenOptions::new()
                        .write(true)
                        .open(&dest_path)
                        .await?;

                    file.seek(std::io::SeekFrom::Start(offset as u64)).await?;
                    file.write_all(&data).await?;

                    let new_bytes = bytes_counter.fetch_add(data.len() as u64, Ordering::Relaxed)
                        + data.len() as u64;
                    progress_fn(new_bytes);

                    Ok::<(), anyhow::Error>(())
                }));
            }

            while let Some(result) = handles.next().await {
                result??;
            }
        } else {
            let response = self.client.get(source).send().await?;
            let data = response.bytes().await?;

            let mut file = tokio::fs::File::create(dest).await?;
            file.write_all(&data).await?;
            progress(data.len() as u64);
        }

        Ok(start.elapsed().as_millis() as u64)
    }

    async fn get_file_info(&self, url: &str) -> anyhow::Result<FileInfo> {
        let response = self.client.head(url).send().await?;
        let headers = response.headers();

        let size = headers
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let accepts_ranges = headers
            .get("accept-ranges")
            .map(|v| v.to_str().unwrap_or("").to_lowercase() == "bytes")
            .unwrap_or(false);

        let filename = url.split('/').next_back().unwrap_or("download").to_string();

        Ok(FileInfo {
            size,
            filename,
            accepts_ranges,
        })
    }

    async fn download_chunk(&self, source: &str, range: Range<u64>) -> anyhow::Result<Bytes> {
        let response = self
            .client
            .get(source)
            .header("Range", format!("bytes={}-{}", range.start, range.end - 1))
            .send()
            .await?;

        Ok(response.bytes().await?)
    }
}
