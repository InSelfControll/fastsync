use async_trait::async_trait;
use bytes::Bytes;
use std::ops::Range;
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use super::{FileInfo, Protocol};

#[derive(Clone)]
pub struct LocalProtocol;

impl Default for LocalProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalProtocol {
    pub fn new() -> Self {
        Self
    }

    pub async fn copy<F>(
        &self,
        source: &Path,
        dest: &Path,
        _parallel: usize,
        progress: F,
    ) -> anyhow::Result<u64>
    where
        F: Fn(u64),
    {
        let metadata = tokio::fs::metadata(source).await?;
        let file_size = metadata.len();

        let mut src = tokio::fs::File::open(source).await?;
        let mut dst = tokio::fs::File::create(dest).await?;

        let buffer_size = 64 * 1024;
        let mut buffer = vec![0u8; buffer_size];
        let mut total_read = 0u64;

        while total_read < file_size {
            let to_read = buffer_size.min((file_size - total_read) as usize);
            let n = src.read(&mut buffer[..to_read]).await?;
            if n == 0 {
                break;
            }
            dst.write_all(&buffer[..n]).await?;
            total_read += n as u64;
            progress(total_read);
        }

        dst.sync_all().await?;
        Ok(total_read)
    }
}

#[async_trait]
impl Protocol for LocalProtocol {
    async fn upload(&self, source: &Path, dest: &str) -> anyhow::Result<u64> {
        let dest_path = if let Some(stripped) = dest.strip_prefix("file://") {
            Path::new(stripped).to_path_buf()
        } else {
            Path::new(dest).to_path_buf()
        };
        self.copy(source, &dest_path, 1, |_| {}).await
    }

    async fn download<F>(
        &self,
        source: &str,
        dest: &Path,
        _parallel: usize,
        progress: F,
    ) -> anyhow::Result<u64>
    where
        F: Fn(u64) + Send,
    {
        let source_path = if let Some(stripped) = source.strip_prefix("file://") {
            Path::new(stripped).to_path_buf()
        } else {
            Path::new(source).to_path_buf()
        };
        self.copy(&source_path, dest, 1, progress).await
    }

    async fn get_file_info(&self, url: &str) -> anyhow::Result<FileInfo> {
        let path = if let Some(stripped) = url.strip_prefix("file://") {
            Path::new(stripped).to_path_buf()
        } else {
            Path::new(url).to_path_buf()
        };

        let metadata = tokio::fs::metadata(&path).await?;
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(FileInfo {
            size: metadata.len(),
            filename,
            accepts_ranges: true,
        })
    }

    async fn download_chunk(&self, source: &str, range: Range<u64>) -> anyhow::Result<Bytes> {
        let path = if let Some(stripped) = source.strip_prefix("file://") {
            Path::new(stripped).to_path_buf()
        } else {
            Path::new(source).to_path_buf()
        };

        let mut file = tokio::fs::File::open(&path).await?;
        file.seek(std::io::SeekFrom::Start(range.start)).await?;

        let size = (range.end - range.start) as usize;
        let mut buffer = vec![0u8; size];
        file.read_exact(&mut buffer).await?;

        Ok(Bytes::from(buffer))
    }
}
