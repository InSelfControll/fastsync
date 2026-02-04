//! SFTP protocol implementation for remote file transfers
//!
//! Supports the rsync/scp-style syntax: user@host:path

use async_trait::async_trait;
use bytes::Bytes;
use russh::client::{self, Handler};
use russh::keys::agent::client::AgentClient;
use russh::ChannelId;
use russh_sftp::client::SftpSession;
use std::net::{IpAddr, SocketAddr};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::lookup_host;

use super::{FileInfo, IpVersion, Protocol};

/// Authentication methods tried during connection
#[derive(Debug)]
pub struct AuthAttempts {
    pub tried_agent: bool,
    pub tried_keys: Vec<String>,
    pub errors: Vec<String>,
}

/// Parsed SFTP connection info from user@host:path syntax
#[derive(Debug, Clone)]
pub struct SftpConnectionInfo {
    pub user: String,
    pub host: String,
    pub port: u16,
    pub path: String,
}

impl SftpConnectionInfo {
    /// Parse user@host:path or user@host:port:path syntax
    /// 
    /// Supported formats:
    /// - user@host:/absolute/path
    /// - user@host:~/relative/path  
    /// - user@host:port:/absolute/path
    /// - user@host:port:~/relative/path
    /// - user@[IPv6]:/path (bracketed IPv6)
    /// - user@[IPv6]:port:/path (bracketed IPv6 with port)
    /// - user@[IPv6]:~/path (bracketed IPv6 with home path)
    /// - user@IPv6:/path (bare IPv6 with explicit path)
    /// - user@IPv6:~/path (bare IPv6 with home path)
    /// - user@IPv6 (bare IPv6, default path ~)
    /// 
    /// Note: For IPv6 with custom port, use bracketed format: [IPv6]:port:path
    pub fn parse(input: &str) -> Option<Self> {
        // Handle user@host:path format
        let at_idx = input.find('@')?;
        let user = input[..at_idx].to_string();

        let rest = &input[at_idx + 1..];

        // Check if host is bracketed IPv6 like [::1] or [2a01::1]:port
        if rest.starts_with('[') {
            return Self::parse_bracketed_ipv6(&user, rest);
        }

        // Not bracketed - could be hostname or bare IPv6
        // Check if it looks like an IPv6 address (multiple consecutive colons :: or >2 colons)
        let colon_count = rest.chars().filter(|&c| c == ':').count();
        let is_likely_ipv6 = rest.contains("::") || colon_count > 2;
        
        if is_likely_ipv6 {
            // For bare IPv6, look for explicit path markers :/ or :~
            // We search from left to right to find the first occurrence
            // but we need to be careful not to match colons inside the IPv6 address
            // The key is that :/ and :~ are unambiguous path markers
            
            if let Some(path_sep_idx) = rest.find(":/").or_else(|| rest.find(":~")) {
                // Found path separator - everything before it is the host
                let host_part = &rest[..path_sep_idx];
                let path = &rest[path_sep_idx + 1..];
                
                return Some(Self {
                    user,
                    host: host_part.to_string(),
                    port: 22,
                    path: path.to_string(),
                });
            }
            
            // No path marker found - the whole thing is the IPv6 address
            // Use default path
            return Some(Self {
                user,
                host: rest.to_string(),
                port: 22,
                path: "~".to_string(),
            });
        }

        // Regular hostname - use standard parsing
        Self::parse_hostname(&user, rest)
    }
    
    /// Parse bracketed IPv6 format: [IPv6]:/path, [IPv6]:port:/path, [IPv6]:~/path
    fn parse_bracketed_ipv6(user: &str, rest: &str) -> Option<Self> {
        // Find closing bracket
        let bracket_end = rest.find(']')?;
        let host = rest[1..bracket_end].to_string();
        
        // After the bracket, we might have :port or :/path or :~path
        let after_bracket = &rest[bracket_end + 1..];
        
        if after_bracket.is_empty() {
            // No port or path specified
            return Some(Self {
                user: user.to_string(),
                host,
                port: 22,
                path: "~".to_string(),
            });
        }
        
        // Check what comes after the bracket
        // It should start with : (either :port or :/path or :~path)
        if !after_bracket.starts_with(':') {
            // Invalid format - bracketed IPv6 must be followed by :
            return None;
        }
        
        let after_colon = &after_bracket[1..]; // Skip the colon
        
        // Check if it's a path directly (starts with / or ~)
        if after_colon.starts_with('/') || after_colon.starts_with('~') {
            return Some(Self {
                user: user.to_string(),
                host,
                port: 22,
                path: after_colon.to_string(),
            });
        }
        
        // It might be a port number followed by a path
        // Format: [IPv6]:port:/path or [IPv6]:port:~/path
        if let Some(second_colon) = after_colon.find(':') {
            let potential_port = &after_colon[..second_colon];
            let after_port = &after_colon[second_colon + 1..];
            
            // Check if it's a valid port number
            if potential_port.chars().all(|c| c.is_ascii_digit()) {
                let port = potential_port.parse().unwrap_or(22);
                let path = if after_port.is_empty() { "~" } else { after_port };
                return Some(Self {
                    user: user.to_string(),
                    host,
                    port,
                    path: path.to_string(),
                });
            }
        }
        
        // If we can't parse it as port:path, treat the rest as path
        Some(Self {
            user: user.to_string(),
            host,
            port: 22,
            path: after_colon.to_string(),
        })
    }
    
    /// Parse regular hostname format: host:/path, host:port:/path, etc.
    fn parse_hostname(user: &str, rest: &str) -> Option<Self> {
        // Find the path separator - look for :/ or :~
        // This distinguishes host:port from host:path
        let path_sep_idx = rest.find(":/")
            .or_else(|| rest.find(":~"));
        
        let (host_part, path) = if let Some(idx) = path_sep_idx {
            // idx points to the colon, path starts at idx + 1
            (&rest[..idx], &rest[idx + 1..])
        } else {
            // No explicit path marker found, try to find the last colon
            // Format might be host:path where path doesn't start with / or ~
            // This is ambiguous, but we assume the last : separates host from path
            // if what's after looks like a path (contains /)
            if let Some(colon_idx) = rest.rfind(':') {
                let potential_path = &rest[colon_idx + 1..];
                if potential_path.contains('/') || potential_path.starts_with('~') {
                    (&rest[..colon_idx], potential_path)
                } else {
                    // Assume the whole thing is host with default path
                    (rest, "~")
                }
            } else {
                (rest, "~")
            }
        };

        // Parse host:port or just host
        let (host, port) = if let Some(port_idx) = host_part.rfind(':') {
            let before_port = &host_part[..port_idx];
            let after_port = &host_part[port_idx + 1..];
            
            // If after the colon is all digits, it's a port
            if after_port.chars().all(|c| c.is_ascii_digit()) {
                let port = after_port.parse().unwrap_or(22);
                (before_port.to_string(), port)
            } else {
                (host_part.to_string(), 22)
            }
        } else {
            (host_part.to_string(), 22)
        };

        Some(Self {
            user: user.to_string(),
            host,
            port,
            path: path.to_string(),
        })
    }

    /// Get the normalized path
    /// 
    /// Expands ~ to the user's home directory:
    /// - For root user: ~/ -> /root/
    /// - For other users: ~/ -> /home/{user}/
    pub fn normalize_path(&self) -> String {
        if self.path.starts_with("~/") {
            // Expand ~ to home directory
            if self.user == "root" {
                self.path.replacen("~/", "/root/", 1)
            } else {
                self.path.replacen("~/", &format!("/home/{}/", self.user), 1)
            }
        } else if self.path.starts_with('/') {
            // Already absolute path
            self.path.clone()
        } else if self.path == "~" {
            // Just ~ without trailing slash
            if self.user == "root" {
                "/root".to_string()
            } else {
                format!("/home/{}", self.user)
            }
        } else {
            // Relative path - assume relative to home
            format!("/{}", self.path)
        }
    }
}

/// SSH client handler (required by russh)
#[derive(Clone)]
struct ClientHandler;

impl Handler for ClientHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Accept any server key for now (in production, check against known_hosts)
        Ok(true)
    }

    async fn data(
        &mut self,
        _channel: ChannelId,
        _data: &[u8],
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn channel_close(
        &mut self,
        _channel: ChannelId,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn channel_open_confirmation(
        &mut self,
        _channel: ChannelId,
        _max_packet_size: u32,
        _window_size: u32,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn channel_open_failure(
        &mut self,
        _channel: ChannelId,
        _reason: russh::ChannelOpenFailure,
        _description: &str,
        _language: &str,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn auth_banner(
        &mut self,
        _banner: &str,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn channel_success(
        &mut self,
        _channel: ChannelId,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn channel_failure(
        &mut self,
        _channel: ChannelId,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// SFTP Protocol implementation
#[derive(Clone)]
pub struct SftpProtocol {
    connection_info: SftpConnectionInfo,
    ip_version: IpVersion,
    identity_file: Option<PathBuf>,
}

impl SftpProtocol {
    /// Create a new SFTP protocol instance
    #[allow(dead_code)]
    pub fn new(connection_info: SftpConnectionInfo) -> Self {
        Self {
            connection_info,
            ip_version: IpVersion::Any,
            identity_file: None,
        }
    }

    /// Create a new SFTP protocol instance with IP version preference
    pub fn new_with_ip_version(connection_info: SftpConnectionInfo, ip_version: IpVersion) -> Self {
        Self {
            connection_info,
            ip_version,
            identity_file: None,
        }
    }

    /// Create a new SFTP protocol instance with IP version and identity file
    pub fn new_with_ip_version_and_identity(
        connection_info: SftpConnectionInfo,
        ip_version: IpVersion,
        identity_file: Option<PathBuf>,
    ) -> Self {
        Self {
            connection_info,
            ip_version,
            identity_file,
        }
    }

    /// Resolve hostname to socket address with IP version preference
    async fn resolve_host(&self) -> anyhow::Result<SocketAddr> {
        let host = &self.connection_info.host;
        let port = self.connection_info.port;
        let addrs: Vec<SocketAddr> = lookup_host((host.as_str(), port)).await?.collect();

        let selected = match self.ip_version {
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
                match self.ip_version {
                    IpVersion::Ipv4 => "IPv4",
                    IpVersion::Ipv6 => "IPv6",
                    IpVersion::Any => "suitable",
                },
                host
            )
        })
    }

    /// Connect to the SSH server and return an SFTP session
    async fn connect(&self) -> anyhow::Result<SftpSession> {
        // Resolve host to specific IP address based on IP version preference
        let socket_addr = self.resolve_host().await?;
        eprintln!("Debug: Connecting to {} (resolved to {})", 
            self.connection_info.host, socket_addr);
        
        // Debug: show identity file setting
        match &self.identity_file {
            Some(path) => eprintln!("Debug: Identity file set to: {:?}", path),
            None => eprintln!("Debug: No identity file specified"),
        }

        let config = russh::client::Config::default();
        let config = Arc::new(config);

        let handler = ClientHandler;
        eprintln!("Debug: Starting SSH handshake...");
        let mut handle = russh::client::connect(config, socket_addr, handler).await?;
        eprintln!("Debug: SSH handshake completed");

        // Try key-based authentication first, then agent
        eprintln!("Debug: Starting authentication as user '{}'...", self.connection_info.user);
        let (auth_success, auth_errors) = self.authenticate(&mut handle).await?;

        if !auth_success {
            // Print detailed errors
            eprintln!("\nDebug: Authentication attempt details:");
            if !auth_errors.is_empty() {
                eprintln!("Debug: Errors encountered:");
                for (i, err) in auth_errors.iter().enumerate() {
                    eprintln!("  {}. {}", i + 1, err);
                }
            }
            
            anyhow::bail!(
                "Authentication failed for {}@{}\n\n\
                Tried methods:\n\
                1. Specified identity file (-i option)\n\
                2. SSH agent (SSH_AUTH_SOCK environment variable) - {} keys tried\n\
                3. SSH keys: ~/.ssh/id_ed25519, ~/.ssh/id_rsa, ~/.ssh/id_ecdsa\n\n\
                Suggestions:\n\
                - Use -i to specify a private key file: fastsync cp -i ~/.ssh/my_key file.txt user@host:~/\n\
                - Ensure your SSH key is added to the agent: ssh-add ~/.ssh/id_ed25519\n\
                - Check that the public key is in the server's ~/.ssh/authorized_keys\n\
                - Verify the username is correct\n\
                - Check if the server requires password authentication (not yet supported)",
                self.connection_info.user,
                self.connection_info.host,
                auth_errors.len()
            );
        }

        eprintln!("Debug: Authentication successful, starting SFTP subsystem...");
        
        // Start SFTP subsystem
        eprintln!("Debug: Opening channel...");
        let channel = handle.channel_open_session().await
            .map_err(|e| anyhow::anyhow!("Failed to open SSH channel: {}", e))?;
        
        eprintln!("Debug: Requesting SFTP subsystem...");
        channel.request_subsystem(true, "sftp").await
            .map_err(|e| anyhow::anyhow!("Failed to start SFTP subsystem: {}", e))?;

        eprintln!("Debug: Creating SFTP session...");
        let sftp = SftpSession::new(channel.into_stream()).await
            .map_err(|e| anyhow::anyhow!("Failed to create SFTP session: {:?}", e))?;
        
        eprintln!("Debug: SFTP session established");
        Ok(sftp)
    }

    /// Authenticate with the SSH server
    /// 
    /// Tries multiple authentication methods in order:
    /// 1. Specified identity file (if provided)
    /// 2. SSH agent (if SSH_AUTH_SOCK is set and has keys)
    /// 3. Private key files (~/.ssh/id_ed25519, ~/.ssh/id_rsa, ~/.ssh/id_ecdsa)
    /// 
    /// Returns (success, list of errors)
    async fn authenticate(&self, handle: &mut client::Handle<ClientHandler>) -> anyhow::Result<(bool, Vec<String>)> {
        let user = self.connection_info.user.clone();
        let mut auth_attempts = AuthAttempts {
            tried_agent: false,
            tried_keys: Vec::new(),
            errors: Vec::new(),
        };

        // 0. Try explicitly specified identity file first (highest priority)
        // If -i is specified, we ONLY try that key to avoid connection exhaustion
        if let Some(ref identity_path) = self.identity_file {
            eprintln!("Debug: Trying specified identity file: {:?}", identity_path);
            if identity_path.exists() {
                auth_attempts.tried_keys.push(identity_path.display().to_string());
                match self.try_key_file(handle, &user, identity_path).await {
                    Ok(true) => return Ok((true, Vec::new())),
                    Ok(false) => {
                        eprintln!("Debug: Specified identity file failed, not trying other methods (-i was used)");
                        return Ok((false, vec![format!("Specified identity file failed: {:?}", identity_path)]));
                    }
                    Err(e) => {
                        eprintln!("Debug: Specified identity file error: {}", e);
                        return Ok((false, vec![format!("Specified identity file error: {}", e)]));
                    }
                }
            } else {
                let err = format!("Specified identity file not found: {:?}", identity_path);
                eprintln!("Debug: {}", err);
                return Ok((false, vec![err]));
            }
        }

        // 1. Try SSH agent
        #[cfg(unix)]
        {
            tracing::debug!("Attempting to connect to SSH agent via SSH_AUTH_SOCK");
            match AgentClient::connect_env().await {
                Ok(mut agent) => {
                    auth_attempts.tried_agent = true;
                    tracing::debug!("Connected to SSH agent successfully");
                    
                    match agent.request_identities().await {
                        Ok(identities) => {
                            tracing::debug!("SSH agent returned {} identities", identities.len());
                            eprintln!("Debug: SSH agent has {} keys available (will try first 3)", identities.len());
                            
                            // Limit to first 3 keys to avoid connection exhaustion
                            for (idx, identity) in identities.iter().take(3).enumerate() {
                                let key_fingerprint = identity.fingerprint(Default::default());
                                let key_algo = identity.algorithm().to_string();
                                eprintln!("Debug: Trying agent key {}: {} (algo: {}, comment: {:?})", 
                                    idx, key_fingerprint, key_algo, identity.comment());
                                
                                // Try each key from the agent
                                let auth_result = handle
                                    .authenticate_publickey_with(
                                        &user,
                                        identity.clone(),
                                        None,
                                        &mut agent,
                                    )
                                    .await;
                                
                                match auth_result {
                                    Ok(russh::client::AuthResult::Success) => {
                                        tracing::info!("Authenticated via SSH agent with key: {:?}", identity.comment());
                                        eprintln!("Debug: Successfully authenticated with key {:?}", identity.comment());
                                        return Ok((true, Vec::new()));
                                    }
                                    Ok(russh::client::AuthResult::Failure { remaining_methods, partial_success }) => {
                                        let err_msg = format!(
                                            "Agent key {} ({:?}) rejected - partial_success: {}, remaining: {:?}",
                                            key_fingerprint, identity.comment(), partial_success, remaining_methods
                                        );
                                        eprintln!("Debug: {}", err_msg);
                                        auth_attempts.errors.push(err_msg);
                                        // Continue to next key
                                    }
                                    Err(e) => {
                                        let err_msg = format!(
                                            "Agent key {} error: {}",
                                            key_fingerprint, e
                                        );
                                        eprintln!("Debug: {}", err_msg);
                                        auth_attempts.errors.push(err_msg);
                                        // Connection might be dead, stop trying agent keys
                                        eprintln!("Debug: Connection error, stopping agent key attempts");
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to get identities from SSH agent: {}", e);
                            auth_attempts.errors.push(format!("Failed to get identities from SSH agent: {}", e));
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("SSH agent not available: {}", e);
                    auth_attempts.errors.push(format!("SSH agent not available: {}", e));
                }
            }
        }

        // 2. Try private key files
        let key_configs = vec![
            ("Ed25519", dirs::home_dir().map(|h| h.join(".ssh/id_ed25519"))),
            ("RSA", dirs::home_dir().map(|h| h.join(".ssh/id_rsa"))),
            ("ECDSA", dirs::home_dir().map(|h| h.join(".ssh/id_ecdsa"))),
        ];

        for (key_type, maybe_key_path) in key_configs {
            let key_path = match maybe_key_path {
                Some(p) => p,
                None => continue,
            };
            
            if key_path.exists() {
                auth_attempts.tried_keys.push(key_path.display().to_string());
                eprintln!("Debug: Trying {} key file: {:?}", key_type, key_path);
                
                match std::fs::read(&key_path) {
                    Ok(key) => {
                        match std::str::from_utf8(&key) {
                            Ok(key_str) => {
                                // Try without passphrase first
                                match russh::keys::decode_secret_key(key_str, None) {
                                    Ok(key_pair) => {
                                        // For RSA keys, use SHA-256 hash algorithm (required by modern SSH servers)
                                        let hash_alg = if format!("{:?}", key_pair.algorithm()).contains("Rsa") {
                                            Some(russh::keys::HashAlg::Sha256)
                                        } else {
                                            None
                                        };
                                        let key_with_hash = russh::keys::PrivateKeyWithHashAlg::new(
                                            Arc::new(key_pair),
                                            hash_alg,
                                        );
                                        
                                        match handle.authenticate_publickey(&user, key_with_hash).await {
                                            Ok(russh::client::AuthResult::Success) => {
                                                tracing::info!("Authenticated via {} key: {:?}", key_type, key_path);
                                                eprintln!("Debug: Authenticated with {} key {:?}", key_type, key_path);
                                                return Ok((true, Vec::new()));
                                            }
                                            Ok(russh::client::AuthResult::Failure { remaining_methods, partial_success }) => {
                                                let err = format!("{} key rejected - partial: {}, remaining: {:?}", key_type, partial_success, remaining_methods);
                                                eprintln!("Debug: {}", err);
                                                auth_attempts.errors.push(err);
                                            }
                                            Err(e) => {
                                                let err = format!("{} key auth error: {}", key_type, e);
                                                eprintln!("Debug: {}", err);
                                                auth_attempts.errors.push(err);
                                                // Connection might be dead
                                                if e.to_string().contains("Channel") || e.to_string().contains("event loop") {
                                                    eprintln!("Debug: Connection appears dead, stopping file key attempts");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        // Key might have a passphrase
                                        let err = format!("{} key requires passphrase or is invalid: {}", key_type, e);
                                        eprintln!("Debug: {}", err);
                                        auth_attempts.errors.push(err);
                                    }
                                }
                            }
                            Err(_) => {
                                let err = format!("{} key file contains invalid UTF-8: {:?}", key_type, key_path);
                                eprintln!("Debug: {}", err);
                                auth_attempts.errors.push(err);
                            }
                        }
                    }
                    Err(e) => {
                        let err = format!("Failed to read {} key file {:?}: {}", key_type, key_path, e);
                        eprintln!("Debug: {}", err);
                        auth_attempts.errors.push(err);
                    }
                }
            }
        }

        // Log authentication failures for debugging
        tracing::warn!("Authentication failed for {}@{}: {:?}", 
            self.connection_info.user, 
            self.connection_info.host,
            auth_attempts
        );

        Ok((false, auth_attempts.errors))
    }

    /// Try to authenticate with a specific key file
    async fn try_key_file(
        &self,
        handle: &mut client::Handle<ClientHandler>,
        user: &str,
        key_path: &Path,
    ) -> anyhow::Result<bool> {
        eprintln!("Debug: Reading key file: {:?}", key_path);
        let key_data = std::fs::read(key_path)?;
        eprintln!("Debug: Read {} bytes from key file", key_data.len());
        
        let key_str = std::str::from_utf8(&key_data)?;
        eprintln!("Debug: Key file is valid UTF-8, first line: {:?}", key_str.lines().next());
        
        // Try without passphrase first
        match russh::keys::decode_secret_key(key_str, None) {
            Ok(key_pair) => {
                let algo = key_pair.algorithm();
                eprintln!("Debug: Successfully decoded key, algorithm: {:?}", algo);
                
                // For RSA keys, use SHA-256 hash algorithm (required by modern SSH servers)
                // Check if algorithm is RSA by matching on the string representation
                let hash_alg = if format!("{:?}", algo).contains("Rsa") {
                    eprintln!("Debug: Using RSA-SHA2-256 for authentication");
                    Some(russh::keys::HashAlg::Sha256)
                } else {
                    None
                };
                
                let key_with_hash = russh::keys::PrivateKeyWithHashAlg::new(
                    Arc::new(key_pair),
                    hash_alg,
                );
                
                eprintln!("Debug: Attempting authentication with key...");
                match handle.authenticate_publickey(user, key_with_hash).await {
                    Ok(russh::client::AuthResult::Success) => {
                        eprintln!("Debug: Authentication successful!");
                        tracing::info!("Authenticated with key: {:?}", key_path);
                        Ok(true)
                    }
                    Ok(russh::client::AuthResult::Failure { remaining_methods, partial_success }) => {
                        eprintln!("Debug: Authentication failed - partial_success: {}, remaining_methods: {:?}", 
                            partial_success, remaining_methods);
                        Ok(false)
                    }
                    Err(e) => {
                        eprintln!("Debug: Authentication error: {}", e);
                        anyhow::bail!("Key authentication error: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Debug: Failed to decode key: {}", e);
                anyhow::bail!("Key requires passphrase or is invalid: {}", e);
            }
        }
    }

    /// Check if a string looks like an SFTP URL (user@host:path)
    pub fn is_sftp_url(input: &str) -> bool {
        SftpConnectionInfo::parse(input).is_some()
    }
}

#[async_trait]
impl Protocol for SftpProtocol {
    async fn upload(&self, source: &Path, _dest: &str) -> anyhow::Result<u64> {
        eprintln!("Debug: Starting SFTP upload...");
        let sftp = self.connect().await?;

        let metadata = tokio::fs::metadata(source).await?;
        let file_size = metadata.len();
        eprintln!("Debug: Source file size: {} bytes", file_size);

        let mut src_file = tokio::fs::File::open(source).await?;
        
        // Get destination path and append filename if destination is a directory
        let dest_path = self.connection_info.normalize_path();
        let dest_path = if dest_path.ends_with('/') {
            let filename = source.file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid source filename"))?
                .to_string_lossy();
            format!("{}{}", dest_path, filename)
        } else {
            dest_path
        };
        eprintln!("Debug: Destination path: {}", dest_path);

        // Create remote file
        eprintln!("Debug: Creating remote file...");
        let mut dest_file = sftp.create(&dest_path).await
            .map_err(|e| anyhow::anyhow!("Failed to create remote file: {:?}", e))?;

        // Copy data
        let mut buffer = vec![0u8; 64 * 1024];
        let mut total_written = 0u64;

        eprintln!("Debug: Starting data transfer...");
        while total_written < file_size {
            let to_read = buffer.len().min((file_size - total_written) as usize);
            let n = src_file.read(&mut buffer[..to_read]).await?;
            if n == 0 {
                break;
            }

            dest_file.write_all(&buffer[..n]).await
                .map_err(|e| anyhow::anyhow!("Failed to write to remote file: {:?}", e))?;
            total_written += n as u64;
        }

        eprintln!("Debug: Transfer complete, closing file...");
        dest_file.shutdown().await
            .map_err(|e| anyhow::anyhow!("Failed to close remote file: {:?}", e))?;

        eprintln!("Debug: Upload complete: {} bytes written", total_written);
        Ok(total_written)
    }

    async fn download<F>(
        &self,
        _source: &str,
        dest: &Path,
        _parallel: usize,
        progress: F,
    ) -> anyhow::Result<u64>
    where
        F: Fn(u64) + Send + Sync + 'static,
    {
        let sftp = self.connect().await?;

        let remote_path = self.connection_info.normalize_path();

        // Get file info
        let metadata = sftp.metadata(&remote_path).await?;
        let file_size = metadata.len();

        // Create local file
        let mut dest_file = tokio::fs::File::create(dest).await?;

        // Open remote file
        let mut src_file = sftp.open(&remote_path).await?;

        // Copy data
        let mut buffer = vec![0u8; 64 * 1024];
        let mut total_read = 0u64;

        while total_read < file_size {
            let to_read = buffer.len().min((file_size - total_read) as usize);
            let n = src_file.read(&mut buffer[..to_read]).await?;
            if n == 0 {
                break;
            }

            dest_file.write_all(&buffer[..n]).await?;
            total_read += n as u64;
            progress(total_read);
        }

        dest_file.sync_all().await?;

        Ok(total_read)
    }

    async fn get_file_info(&self, _url: &str) -> anyhow::Result<FileInfo> {
        let sftp = self.connect().await?;

        let remote_path = self.connection_info.normalize_path();
        let metadata = sftp.metadata(&remote_path).await?;

        let filename = remote_path
            .split('/')
            .next_back()
            .unwrap_or("download")
            .to_string();

        Ok(FileInfo {
            size: metadata.len(),
            filename,
            accepts_ranges: false, // SFTP doesn't support HTTP-style ranges
        })
    }

    async fn download_chunk(&self, _source: &str, _range: Range<u64>) -> anyhow::Result<Bytes> {
        // SFTP doesn't support HTTP-style range requests natively
        // We could implement this with seek, but for now return an error
        anyhow::bail!("SFTP download_chunk not implemented - use full download")
    }
}

#[allow(dead_code)]
impl SftpProtocol {
    /// List directory contents
    pub async fn list(&self) -> anyhow::Result<Vec<String>> {
        let sftp = self.connect().await?;

        let remote_path = self.connection_info.normalize_path();
        let entries = sftp.read_dir(&remote_path).await?;

        let mut names = Vec::new();
        for entry in entries {
            names.push(entry.file_name());
        }

        Ok(names)
    }

    /// Upload a file with progress callback
    pub async fn upload_with_progress<F>(
        &self,
        source: &Path,
        progress: F,
    ) -> anyhow::Result<u64>
    where
        F: Fn(u64) + Send + Sync + 'static,
    {
        eprintln!("Debug: upload_with_progress called");
        let sftp = self.connect().await?;
        eprintln!("Debug: Connected to SFTP");

        let metadata = tokio::fs::metadata(source).await?;
        let file_size = metadata.len();
        eprintln!("Debug: File size: {}", file_size);

        let mut src_file = tokio::fs::File::open(source).await?;
        
        // Get destination path and append filename if destination is a directory
        let dest_path = self.connection_info.normalize_path();
        let dest_path = if dest_path.ends_with('/') {
            // Destination is a directory, append source filename
            let filename = source.file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid source filename"))?
                .to_string_lossy();
            format!("{}{}", dest_path, filename)
        } else {
            dest_path
        };
        eprintln!("Debug: Destination: {}", dest_path);

        // Create remote file
        eprintln!("Debug: Creating remote file...");
        let mut dest_file = sftp.create(&dest_path).await
            .map_err(|e| anyhow::anyhow!("Failed to create remote file: {:?}", e))?;
        eprintln!("Debug: Remote file created");

        // Copy data with progress
        let mut buffer = vec![0u8; 64 * 1024];
        let mut total_written = 0u64;

        while total_written < file_size {
            let to_read = buffer.len().min((file_size - total_written) as usize);
            let n = src_file.read(&mut buffer[..to_read]).await?;
            if n == 0 {
                break;
            }

            dest_file.write_all(&buffer[..n]).await
                .map_err(|e| anyhow::anyhow!("Failed to write to remote file at offset {}: {:?}", total_written, e))?;
            total_written += n as u64;
            progress(total_written);
        }

        eprintln!("Debug: Closing remote file...");
        dest_file.shutdown().await
            .map_err(|e| anyhow::anyhow!("Failed to close remote file: {:?}", e))?;

        eprintln!("Debug: Upload complete: {} bytes", total_written);
        Ok(total_written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sftp_connection_info() {
        let info = SftpConnectionInfo::parse("user@host:/path/to/file").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "host");
        assert_eq!(info.port, 22);
        assert_eq!(info.path, "/path/to/file");

        let info = SftpConnectionInfo::parse("user@host:~/file").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "host");
        assert_eq!(info.path, "~/file");

        let info = SftpConnectionInfo::parse("user@host:22:~/file").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "host");
        assert_eq!(info.port, 22);
        assert_eq!(info.path, "~/file");

        let info = SftpConnectionInfo::parse("user@host:2222:/file").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "host");
        assert_eq!(info.port, 2222);
    }

    #[test]
    fn test_is_sftp_url() {
        assert!(SftpProtocol::is_sftp_url("user@host:/path"));
        assert!(SftpProtocol::is_sftp_url("user@host:~/path"));
        assert!(!SftpProtocol::is_sftp_url("/local/path"));
        assert!(!SftpProtocol::is_sftp_url("https://example.com/file"));
    }

    #[test]
    fn test_normalize_path() {
        // Root user home
        let info = SftpConnectionInfo {
            user: "root".to_string(),
            host: "host".to_string(),
            port: 22,
            path: "~/".to_string(),
        };
        assert_eq!(info.normalize_path(), "/root/");

        let info = SftpConnectionInfo {
            user: "root".to_string(),
            host: "host".to_string(),
            port: 22,
            path: "~/file.txt".to_string(),
        };
        assert_eq!(info.normalize_path(), "/root/file.txt");

        // Regular user home
        let info = SftpConnectionInfo {
            user: "john".to_string(),
            host: "host".to_string(),
            port: 22,
            path: "~/".to_string(),
        };
        assert_eq!(info.normalize_path(), "/home/john/");

        let info = SftpConnectionInfo {
            user: "john".to_string(),
            host: "host".to_string(),
            port: 22,
            path: "~/file.txt".to_string(),
        };
        assert_eq!(info.normalize_path(), "/home/john/file.txt");

        // Absolute paths
        let info = SftpConnectionInfo {
            user: "root".to_string(),
            host: "host".to_string(),
            port: 22,
            path: "/var/www/html".to_string(),
        };
        assert_eq!(info.normalize_path(), "/var/www/html");

        // Just ~
        let info = SftpConnectionInfo {
            user: "root".to_string(),
            host: "host".to_string(),
            port: 22,
            path: "~".to_string(),
        };
        assert_eq!(info.normalize_path(), "/root");
    }

    #[test]
    fn test_parse_ipv6_bare_address() {
        // Bare IPv6 address without brackets - with explicit path
        let info = SftpConnectionInfo::parse("root@2a01:4f8:c17:a568::1:/path/to/file").unwrap();
        assert_eq!(info.user, "root");
        assert_eq!(info.host, "2a01:4f8:c17:a568::1");
        assert_eq!(info.port, 22);
        assert_eq!(info.path, "/path/to/file");

        // Bare IPv6 with home directory
        let info = SftpConnectionInfo::parse("user@2a01:4f8:c17:a568::1:~/file.txt").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "2a01:4f8:c17:a568::1");
        assert_eq!(info.port, 22);
        assert_eq!(info.path, "~/file.txt");

        // Bare IPv6 localhost (::1) with path
        let info = SftpConnectionInfo::parse("user@::1:~/file").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "::1");
        assert_eq!(info.port, 22);
        assert_eq!(info.path, "~/file");

        // Bare IPv6 localhost (::1) with absolute path
        let info = SftpConnectionInfo::parse("user@::1:/tmp").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "::1");
        assert_eq!(info.port, 22);
        assert_eq!(info.path, "/tmp");

        // Bare IPv6 without path - uses default ~
        let info = SftpConnectionInfo::parse("user@2a01:4f8:c17:a568::1").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "2a01:4f8:c17:a568::1");
        assert_eq!(info.port, 22);
        assert_eq!(info.path, "~");

        // Note: For IPv6 with custom port, use bracketed format
        // Example: user@[2a01::1]:2222:/path
    }

    #[test]
    fn test_parse_ipv6_bracketed_address() {
        // Bracketed IPv6 address
        let info = SftpConnectionInfo::parse("root@[2a01:4f8:c17:a568::1]:/path/to/file").unwrap();
        assert_eq!(info.user, "root");
        assert_eq!(info.host, "2a01:4f8:c17:a568::1");
        assert_eq!(info.port, 22);
        assert_eq!(info.path, "/path/to/file");

        // Bracketed IPv6 with port
        let info = SftpConnectionInfo::parse("user@[2a01:4f8:c17:a568::1]:2222:/path").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "2a01:4f8:c17:a568::1");
        assert_eq!(info.port, 2222);
        assert_eq!(info.path, "/path");

        // Bracketed IPv6 localhost
        let info = SftpConnectionInfo::parse("user@[::1]:~/file").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "::1");
        assert_eq!(info.port, 22);
        assert_eq!(info.path, "~/file");

        // Bracketed IPv6 with home path
        let info = SftpConnectionInfo::parse("user@[2001:db8::1]:~").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "2001:db8::1");
        assert_eq!(info.port, 22);
        assert_eq!(info.path, "~");
    }

    #[test]
    fn test_parse_ipv6_edge_cases() {
        // IPv6 with zone ID (link-local) - bracketed
        let info = SftpConnectionInfo::parse("user@[fe80::1%eth0]:/path").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "fe80::1%eth0");
        assert_eq!(info.port, 22);
        assert_eq!(info.path, "/path");

        // IPv6 localhost shorthand - bare
        let info = SftpConnectionInfo::parse("user@::1:/tmp").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "::1");
        assert_eq!(info.path, "/tmp");

        // Full IPv6 address - bare
        let info = SftpConnectionInfo::parse("user@2001:0db8:85a3:0000:0000:8a2e:0370:7334:~/file").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "2001:0db8:85a3:0000:0000:8a2e:0370:7334");
        assert_eq!(info.path, "~/file");

        // Full IPv6 address - no path (default ~)
        let info = SftpConnectionInfo::parse("user@2001:0db8:85a3:0000:0000:8a2e:0370:7334").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "2001:0db8:85a3:0000:0000:8a2e:0370:7334");
        assert_eq!(info.path, "~");

        // IPv6 with zone ID (link-local) - bare with path
        let info = SftpConnectionInfo::parse("user@fe80::1%eth0:~/docs").unwrap();
        assert_eq!(info.user, "user");
        assert_eq!(info.host, "fe80::1%eth0");
        assert_eq!(info.path, "~/docs");
    }

    /// Test SSH agent connection - only runs if SSH_AUTH_SOCK is set
    #[tokio::test]
    #[cfg(unix)]
    async fn test_ssh_agent_connection() {
        use std::env;
        
        if env::var("SSH_AUTH_SOCK").is_err() {
            println!("Skipping SSH agent test: SSH_AUTH_SOCK not set");
            return;
        }

        match AgentClient::connect_env().await {
            Ok(mut agent) => {
                println!("Connected to SSH agent");
                
                match agent.request_identities().await {
                    Ok(identities) => {
                        println!("Found {} keys in agent", identities.len());
                        for (i, key) in identities.iter().enumerate() {
                            println!("Key {}: {:?}", i, key.comment());
                        }
                        assert!(!identities.is_empty(), "SSH agent has no keys");
                    }
                    Err(e) => {
                        panic!("Failed to request identities: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("SSH agent not available: {}", e);
                // Don't fail the test if agent is not available
            }
        }
    }
}
