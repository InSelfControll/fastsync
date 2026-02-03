//! fastsync - High-performance file transfer library
//!
//! This library provides protocols and utilities for fast file transfers.
//!
//! # Example
//!
//! ```rust,no_run
//! use fastsync::{HttpProtocol, Protocol};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = HttpProtocol::new();
//!     let info = client.get_file_info("https://example.com/file.zip").await?;
//!     println!("File size: {} bytes", info.size);
//!     Ok(())
//! }
//! ```

pub mod download;
pub mod progress;
pub mod protocol;

pub use download::{DownloadManager, DownloadOptions};
pub use progress::{DownloadResult, ProgressManager};
pub use protocol::{FileInfo, HttpProtocol, IpVersion, LocalProtocol, Protocol, SftpConnectionInfo, SftpProtocol};
