//! Download manager for parallel file transfers

use crate::progress::{DownloadResult, ProgressManager};
use crate::{HttpProtocol, Protocol};
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Options for download operations
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// Number of parallel streams per file
    pub parallel_streams: usize,
    /// Size of each chunk in bytes
    pub chunk_size: usize,
    /// Whether to resume partial downloads
    pub resume: bool,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Verify checksum after download
    pub verify_checksum: bool,
    /// Expected checksum (if verifying)
    pub expected_checksum: Option<String>,
    /// Custom headers to add to requests
    pub headers: Vec<(String, String)>,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            parallel_streams: 8,
            chunk_size: 8 * 1024 * 1024, // 8MB
            resume: true,
            timeout: Duration::from_secs(300),
            max_retries: 3,
            verify_checksum: false,
            expected_checksum: None,
            headers: Vec::new(),
        }
    }
}

/// Manages file downloads
#[derive(Clone)]
pub struct DownloadManager {
    progress: Arc<ProgressManager>,
    client: HttpProtocol,
}

impl DownloadManager {
    /// Create a new download manager
    pub fn new(progress: Arc<ProgressManager>) -> Self {
        Self {
            progress,
            client: HttpProtocol::new(),
        }
    }

    /// Download a single file
    pub async fn download(
        &self,
        url: impl AsRef<str>,
        output_dir: impl AsRef<Path>,
        options: DownloadOptions,
    ) -> Result<DownloadResult> {
        let url = url.as_ref();
        let output_dir = output_dir.as_ref();

        // Extract filename from URL
        let filename = url
            .split('/')
            .next_back()
            .unwrap_or("download")
            .split('?')
            .next()
            .unwrap_or("download");

        let dest_path = output_dir.join(filename);

        // Get file info
        let info = self.client.get_file_info(url).await?;

        // Create progress tracker
        let file_progress = self.progress.create_file_progress(filename, info.size);

        // Perform download
        let start = std::time::Instant::now();

        let progress = file_progress.clone();
        self.client
            .download(url, &dest_path, options.parallel_streams, move |bytes| {
                progress.update(bytes);
            })
            .await?;

        let duration = start.elapsed();

        file_progress.finish("Complete");

        Ok(DownloadResult {
            bytes_transferred: file_progress.bytes_transferred(),
            duration,
        })
    }

    /// Download multiple files
    pub async fn download_batch(
        &self,
        urls: Vec<String>,
        output_dir: impl AsRef<Path>,
        options: DownloadOptions,
    ) -> Vec<Result<DownloadResult>> {
        let output_dir = Arc::new(output_dir.as_ref().to_path_buf());
        let semaphore = Arc::new(Semaphore::new(3)); // Limit concurrent downloads
        let mut handles = Vec::new();

        for url in urls {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let output_dir = output_dir.clone();
            let options = options.clone();
            let self_clone = self.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;
                self_clone
                    .download(&url, output_dir.as_path(), options)
                    .await
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            results.push(
                handle
                    .await
                    .unwrap_or_else(|e| Err(anyhow::anyhow!("Task panicked: {}", e))),
            );
        }

        results
    }
}
