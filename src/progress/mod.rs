//! Progress tracking for file transfers

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Manages progress bars for file transfers
#[derive(Clone)]
pub struct ProgressManager {
    multi_progress: Arc<MultiProgress>,
    show_progress: bool,
    show_summary: bool,
}

impl ProgressManager {
    /// Create a new progress manager
    pub fn new(show_progress: bool, show_summary: bool) -> Self {
        Self {
            multi_progress: Arc::new(MultiProgress::new()),
            show_progress,
            show_summary,
        }
    }

    /// Create a progress bar for a file transfer
    pub fn create_file_progress(&self, filename: &str, size: u64) -> FileProgress {
        if !self.show_progress {
            return FileProgress::disabled();
        }

        let pb = self.multi_progress.add(ProgressBar::new(size));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(filename.to_string());

        FileProgress {
            inner: Some(Arc::new(pb)),
            bytes_transferred: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
        }
    }

    /// Print a summary message
    pub fn print_summary(&self, msg: &str) {
        if self.show_summary {
            self.multi_progress.println(msg).ok();
        }
    }
}

/// Progress tracker for a single file transfer
#[derive(Clone)]
pub struct FileProgress {
    inner: Option<Arc<ProgressBar>>,
    bytes_transferred: Arc<AtomicU64>,
    start_time: Instant,
}

impl FileProgress {
    /// Create a disabled progress tracker (no-op)
    fn disabled() -> Self {
        Self {
            inner: None,
            bytes_transferred: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
        }
    }

    /// Update progress
    pub fn update(&self, bytes: u64) {
        self.bytes_transferred.store(bytes, Ordering::Relaxed);
        if let Some(pb) = &self.inner {
            pb.set_position(bytes);
        }
    }

    /// Increment progress
    pub fn inc(&self, bytes: u64) {
        let new = self.bytes_transferred.fetch_add(bytes, Ordering::Relaxed) + bytes;
        if let Some(pb) = &self.inner {
            pb.set_position(new);
        }
    }

    /// Finish the progress bar
    pub fn finish(&self, message: &str) {
        if let Some(pb) = &self.inner {
            pb.finish_with_message(message.to_string());
        }
    }

    /// Get bytes transferred
    pub fn bytes_transferred(&self) -> u64 {
        self.bytes_transferred.load(Ordering::Relaxed)
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Result of a download operation
#[derive(Debug)]
pub struct DownloadResult {
    pub bytes_transferred: u64,
    pub duration: Duration,
}

impl DownloadResult {
    /// Get throughput in bytes per second
    pub fn throughput_bps(&self) -> f64 {
        if self.duration.as_secs_f64() > 0.0 {
            self.bytes_transferred as f64 / self.duration.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Get human-readable throughput
    pub fn throughput_human(&self) -> String {
        let bps = self.throughput_bps();
        humansize::format_size(bps as u64, humansize::BINARY) + "/s"
    }
}
