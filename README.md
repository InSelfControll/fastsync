# fastsync

High-performance file transfer tool.

## Commands

```bash
# Copy files
fastsync cp file.txt /backup/
fastsync cp -R ~/docs /backup/

# Sync directories
fastsync sync ~/docs /backup/

# List files
fastsync ls /tmp

# Download files
fastsync dl https://example.com/file.zip
fastsync dl https://example.com/file.zip ./downloads/ -p 16 -r
```

## Options

- `-p N` - Parallel streams (default: 4 for cp, 8 for dl)
- `-r` - Resume interrupted downloads
