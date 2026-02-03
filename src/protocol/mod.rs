use async_trait::async_trait;
use bytes::Bytes;
use std::ops::Range;
use std::path::Path;

pub mod http;
pub mod local;
pub mod sftp;

pub use http::HttpProtocol;
pub use local::LocalProtocol;
pub use sftp::{SftpConnectionInfo, SftpProtocol};

/// IP version preference for connections
#[derive(Clone, Copy, Debug, Default)]
pub enum IpVersion {
    /// Use IPv4 only
    Ipv4,
    /// Use IPv6 only
    Ipv6,
    /// Use any available (default)
    #[default]
    Any,
}

#[derive(Debug)]
pub struct FileInfo {
    pub size: u64,
    #[allow(dead_code)]
    pub filename: String,
    pub accepts_ranges: bool,
}

#[async_trait]
#[allow(unused)]
pub trait Protocol: Send + Sync {
    async fn upload(&self, source: &Path, dest: &str) -> anyhow::Result<u64>;
    async fn download<F>(
        &self,
        source: &str,
        dest: &Path,
        parallel: usize,
        progress: F,
    ) -> anyhow::Result<u64>
    where
        F: Fn(u64) + Send + Sync + 'static;
    async fn get_file_info(&self, url: &str) -> anyhow::Result<FileInfo>;
    async fn download_chunk(&self, source: &str, range: Range<u64>) -> anyhow::Result<Bytes>;
}
