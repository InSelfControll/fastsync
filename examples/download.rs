//! Example: Download files using fastsync
//!
//! This example demonstrates how to download files from HTTP/HTTPS URLs
//! with parallel streams, resume support, and progress tracking.

use fastsync::{DownloadManager, DownloadOptions, ProgressManager};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create progress manager
    let progress = Arc::new(ProgressManager::new(true, false));

    // Create download manager
    let manager = DownloadManager::new(progress);

    // Example URL to download
    let url = "https://example.com/file.zip";

    // Download options
    let options = DownloadOptions {
        parallel_streams: 8,
        chunk_size: 8 * 1024 * 1024, // 8MB chunks
        resume: true,
        timeout: Duration::from_secs(3600),
        max_retries: 3,
        ..Default::default()
    };

    // Download to current directory
    let output_dir = PathBuf::from("./downloads");
    tokio::fs::create_dir_all(&output_dir).await?;

    println!("Downloading: {}", url);

    match manager.download(url, &output_dir, options).await {
        Ok(result) => {
            println!(
                "✓ Downloaded {} bytes in {:?}",
                result.bytes_transferred, result.duration
            );
            println!("  Speed: {}", result.throughput_human());
        }
        Err(e) => {
            eprintln!("✗ Download failed: {}", e);
        }
    }

    Ok(())
}

/// Example: Download with custom headers and checksum verification
#[allow(dead_code)]
async fn advanced_download() -> anyhow::Result<()> {
    let progress = Arc::new(ProgressManager::new(true, false));
    let manager = DownloadManager::new(progress);

    let options = DownloadOptions {
        parallel_streams: 16,
        chunk_size: 16 * 1024 * 1024, // 16MB chunks
        resume: true,
        verify_checksum: true,
        expected_checksum: Some(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        ),
        headers: vec![
            ("Authorization".to_string(), "Bearer token123".to_string()),
            ("X-Custom-Header".to_string(), "value".to_string()),
        ],
        ..Default::default()
    };

    let result = manager
        .download(
            "https://example.com/protected/file.zip",
            &PathBuf::from("./downloads"),
            options,
        )
        .await?;

    println!("Downloaded: {} bytes", result.bytes_transferred);

    Ok(())
}

/// Example: Batch download multiple files
#[allow(dead_code)]
async fn batch_download() -> anyhow::Result<()> {
    let progress = Arc::new(ProgressManager::new(true, false));
    let manager = DownloadManager::new(progress);

    let urls = vec![
        "https://example.com/file1.zip".to_string(),
        "https://example.com/file2.zip".to_string(),
        "https://example.com/file3.zip".to_string(),
    ];

    let options = DownloadOptions {
        parallel_streams: 8,
        chunk_size: 8 * 1024 * 1024,
        ..Default::default()
    };

    let results = manager
        .download_batch(urls, &PathBuf::from("./downloads"), options)
        .await;

    let mut success = 0;
    let mut failed = 0;

    for result in results {
        match result {
            Ok(_) => success += 1,
            Err(_) => failed += 1,
        }
    }

    println!("Batch complete: {} succeeded, {} failed", success, failed);

    Ok(())
}
