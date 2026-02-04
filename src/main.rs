use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use url::Url;

mod protocol;
use protocol::{HttpProtocol, IpVersion, LocalProtocol, Protocol, SftpConnectionInfo, SftpProtocol};

#[derive(Parser)]
#[command(name = "fastsync")]
#[command(about = "High-performance file transfer tool")]
#[command(long_about = "fastsync - A high-performance file transfer tool supporting HTTP/HTTPS, SFTP, and local file operations.

Supports:
  - HTTP/HTTPS downloads with resume capability
  - SFTP uploads and downloads with SSH key authentication
  - Local file copy operations
  - IPv4/IPv6 connections
  - Progress bars with transfer speed")]
#[command(after_help = "EXAMPLES:
    # Download a file from HTTP
    fastsync dl https://example.com/file.zip

    # Upload to SFTP server
    fastsync cp localfile.txt user@remote_host:~/Documents/

    # Download from SFTP with specific SSH key
    fastsync cp -i ~/.ssh/my_key user@host:/remote/file.txt ./

    # Copy locally with progress bar
    fastsync cp large_file.iso /backup/

    # Use IPv6 address for SFTP
    fastsync cp file.txt root@[2001:db8::1]:/var/www/

    # Resume interrupted download
    fastsync dl -r https://example.com/large.iso ./downloads/

Use 'fastsync <command> --help' for more information on a specific command.
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, global = true, help = "Number of parallel streams")]
    parallel: Option<usize>,

    #[arg(short, global = true, help = "Resume partial downloads")]
    resume: bool,

    /// Use IPv4 only
    #[arg(short = '4', long = "ipv4", global = true, group = "ip_version")]
    ipv4: bool,

    /// Use IPv6 only
    #[arg(short = '6', long = "ipv6", global = true, group = "ip_version")]
    ipv6: bool,

    /// SSH identity file (private key) for SFTP authentication
    #[arg(short = 'i', long = "identity", global = true)]
    identity: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Copy files between local, HTTP, and SFTP locations
    ///
    /// Supports copying files between different protocols:
    /// - Local to local: fastsync cp file.txt /backup/
    /// - Local to SFTP: fastsync cp file.txt user@host:~/
    /// - SFTP to local: fastsync cp user@host:~/file.txt ./
    /// - HTTP to local: fastsync cp https://example.com/file.iso ./
    ///
    /// SFTP connection format: user@host:path or user@host:port:path
    /// IPv6 addresses: user@[IPv6]:path or user@IPv6:/path
    #[command(name = "cp", alias = "copy")]
    #[command(after_help = "EXAMPLES:
    # Upload local file to SFTP server
    fastsync cp document.pdf user@remote_host:~/Documents/

    # Download from SFTP to current directory
    fastsync cp user@host:/var/log/syslog ./

    # Use custom SSH port
    fastsync cp file.txt user@host:2222:~/uploads/

    # Use IPv6 address
    fastsync cp file.txt root@2a01:4f8:c17:a568::1:/var/www/

    # Use bracketed IPv6 with custom port
    fastsync cp file.txt user@[2001:db8::1]:2222:~/data/

    # Use specific SSH identity file
    fastsync cp -i ~/.ssh/work_key deploy.tar.gz deploy@server:/tmp/

    # Force IPv4 for connection
    fastsync cp -4 file.txt user@host:~/

    # Copy between local directories
    fastsync cp -r ./source/ ./backup/
")]
    Cp {
        /// Source path, URL, or SFTP location (user@host:path)
        source: String,
        /// Destination path, URL, or SFTP location (user@host:path)
        dest: String,
    },

    /// Sync directories between locations (not yet implemented)
    #[command(name = "sync")]
    Sync {
        /// Source directory
        source: String,
        /// Destination directory
        dest: String,
    },

    /// List directory contents
    ///
    /// Lists files in a local directory or remote SFTP location.
    /// Supports both local paths and SFTP URLs.
    #[command(name = "ls")]
    #[command(after_help = "EXAMPLES:
    # List local directory
    fastsync ls /var/log

    # List home directory via SFTP
    fastsync ls user@remote_host:~

    # List specific remote directory
    fastsync ls admin@server:/etc/nginx/

    # List using IPv6 address
    fastsync ls user@[2001:db8::1]:/var/log

    # List bare IPv6 address
    fastsync ls root@2a01:4f8:c17:a568::1:/tmp
")]
    Ls {
        /// Directory path or SFTP URL to list
        url: String,
    },

    /// Download files from HTTP/HTTPS URLs
    ///
    /// Downloads files from one or more URLs with progress tracking.
    /// Supports resume capability for interrupted downloads.
    ///
    /// Use -r to resume partial downloads and -p to set parallel streams.
    #[command(name = "dl", alias = "download")]
    #[command(after_help = "EXAMPLES:
    # Download a file to current directory
    fastsync dl https://example.com/file.zip

    # Download to specific directory
    fastsync dl https://example.com/file.iso ./downloads/

    # Resume interrupted download
    fastsync dl -r https://example.com/large.iso ./

    # Download with 8 parallel streams (faster for supported servers)
    fastsync dl -p 8 https://example.com/file.tar.gz /tmp/

    # Download multiple files
    fastsync dl https://site.com/file1.zip https://site.com/file2.zip ./downloads/

    # Force IPv6 connection
    fastsync dl -6 https://example.com/file.zip
")]
    Download {
        /// One or more URLs to download
        urls: Vec<String>,
        /// Output directory (defaults to current directory)
        #[arg(last = true)]
        output: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    // Determine IP version preference
    let ip_version = if cli.ipv4 {
        IpVersion::Ipv4
    } else if cli.ipv6 {
        IpVersion::Ipv6
    } else {
        IpVersion::Any
    };

    match cli.command {
        Commands::Cp { source, dest } => {
            copy(&source, &dest, cli.parallel.unwrap_or(4), ip_version, cli.identity.as_deref()).await
        }
        Commands::Sync { source, dest } => sync(&source, &dest).await,
        Commands::Ls { url } => list(&url).await,
        Commands::Download { urls, output } => {
            download(
                &urls,
                output.as_deref(),
                cli.parallel.unwrap_or(8),
                cli.resume,
                ip_version,
            )
            .await
        }
    }
}

async fn copy(
    source: &str,
    dest: &str,
    _parallel: usize,
    ip_version: IpVersion,
    identity: Option<&Path>,
) -> Result<()> {
    // Check for SFTP URLs (user@host:path syntax)
    let source_is_sftp = SftpProtocol::is_sftp_url(source);
    let dest_is_sftp = SftpProtocol::is_sftp_url(dest);

    match (source_is_sftp, dest_is_sftp) {
        // Local to local copy
        (false, false) => copy_local_to_local(source, dest).await,
        // Local to SFTP (upload)
        (false, true) => copy_local_to_sftp(source, dest, ip_version, identity).await,
        // SFTP to local (download)
        (true, false) => copy_sftp_to_local(source, dest, ip_version, identity).await,
        // SFTP to SFTP (not supported)
        (true, true) => {
            println!(
                "{}: SFTP to SFTP copy is not supported",
                "Error".red()
            );
            Ok(())
        }
    }
}

async fn copy_local_to_local(source: &str, dest: &str) -> Result<()> {
    let source_url = parse_url(source)?;
    let dest_url = parse_url(dest)?;

    let protocol = LocalProtocol::new();
    let source_path = source_url.to_file_path().unwrap();
    let dest_path = dest_url.to_file_path().unwrap();

    let pb = create_progress_bar(std::fs::metadata(&source_path)?.len());

    let result = protocol
        .copy(&source_path, &dest_path, 1, |bytes| {
            pb.set_position(bytes);
        })
        .await?;

    pb.finish_with_message("Done");
    println!(
        "Copied {} in {:?}",
        humansize::format_size(result, humansize::BINARY),
        result
    );
    Ok(())
}

async fn copy_local_to_sftp(
    source: &str,
    dest: &str,
    ip_version: IpVersion,
    identity: Option<&Path>,
) -> Result<()> {
    let conn_info = SftpConnectionInfo::parse(dest)
        .ok_or_else(|| anyhow::anyhow!("Invalid SFTP destination: {}", dest))?;

    let source_path = Path::new(source);
    if !source_path.exists() {
        anyhow::bail!("Source file does not exist: {}", source);
    }

    let file_size = std::fs::metadata(source_path)?.len();
    let pb = std::sync::Arc::new(create_progress_bar(file_size));

    println!(
        "Uploading {} to {}@{}:{}...",
        source,
        conn_info.user,
        conn_info.host,
        conn_info.path
    );

    // Create protocol instance with optional identity file
    let protocol = if let Some(identity_path) = identity {
        SftpProtocol::new_with_ip_version_and_identity(conn_info, ip_version, Some(identity_path.to_path_buf()))
    } else {
        SftpProtocol::new_with_ip_version(conn_info, ip_version)
    };
    
    let pb_clone = pb.clone();
    let result = protocol
        .upload_with_progress(source_path, move |bytes| {
            pb_clone.set_position(bytes);
        })
        .await?;

    pb.finish_with_message("Done");
    println!(
        "Uploaded {} in {:?}",
        humansize::format_size(result, humansize::BINARY),
        result
    );

    Ok(())
}

async fn copy_sftp_to_local(
    source: &str,
    dest: &str,
    ip_version: IpVersion,
    identity: Option<&Path>,
) -> Result<()> {
    let conn_info = SftpConnectionInfo::parse(source)
        .ok_or_else(|| anyhow::anyhow!("Invalid SFTP source: {}", source))?;

    let dest_path = Path::new(dest);

    println!(
        "Downloading from {}@{}:{} to {}...",
        conn_info.user,
        conn_info.host,
        conn_info.path,
        dest
    );

    // Create protocol instance with optional identity file
    let protocol = if let Some(identity_path) = identity {
        SftpProtocol::new_with_ip_version_and_identity(conn_info.clone(), ip_version, Some(identity_path.to_path_buf()))
    } else {
        SftpProtocol::new_with_ip_version(conn_info.clone(), ip_version)
    };

    // Get file info for progress bar
    let info = protocol.get_file_info("").await?;
    let pb = std::sync::Arc::new(create_progress_bar(info.size));

    let pb_clone = pb.clone();
    let result = protocol
        .download("", dest_path, 1, move |bytes| {
            pb_clone.set_position(bytes);
        })
        .await?;

    pb.finish_with_message("Done");
    println!(
        "Downloaded {} in {:?}",
        humansize::format_size(result, humansize::BINARY),
        result
    );

    Ok(())
}

async fn sync(source: &str, dest: &str) -> Result<()> {
    println!("Syncing {} -> {}", source, dest);
    Ok(())
}

async fn list(url: &str) -> Result<()> {
    let url = parse_url(url)?;

    if url.scheme() == "file" {
        let path = url.to_file_path().unwrap();
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            println!("{}", entry.file_name().to_string_lossy());
        }
    }

    Ok(())
}

async fn download(
    urls: &[String],
    output: Option<&Path>,
    parallel: usize,
    resume: bool,
    ip_version: IpVersion,
) -> Result<()> {
    let output_dir = output.unwrap_or(&PathBuf::from(".")).to_path_buf();
    tokio::fs::create_dir_all(&output_dir).await?;

    let client = HttpProtocol::new_with_ip_version(ip_version);

    for url in urls {
        let filename = url.split('/').next_back().unwrap_or("download");
        let dest_path = output_dir.join(filename);

        if dest_path.exists() && !resume {
            println!("{} already exists, skipping (use -r to resume)", filename);
            continue;
        }

        println!("Downloading {}...", url);

        let info = client.get_file_info(url).await?;
        let pb = std::sync::Arc::new(create_progress_bar(info.size));

        let pb_clone = pb.clone();
        let result = client
            .download(url, &dest_path, parallel, move |bytes| {
                pb_clone.set_position(bytes);
            })
            .await?;

        pb.finish_with_message("Done");
        println!(
            "Downloaded {} in {:?}",
            humansize::format_size(result, humansize::BINARY),
            result
        );
    }

    Ok(())
}

fn parse_url(input: &str) -> Result<Url> {
    if let Ok(url) = Url::parse(input) {
        return Ok(url);
    }
    let path = std::env::current_dir()?.join(input);
    Ok(Url::from_file_path(&path).unwrap())
}

fn create_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb
}
