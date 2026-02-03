# fastsync Commands (Short)

## Quick Reference

| Command | Short | Description |
|---------|-------|-------------|
| `fastsync cp` | `copy` | Copy files |
| `fastsync sync` | - | Sync directories |
| `fastsync ls` | - | List files |
| `fastsync dl` | `download` | Download files |

## Examples

### Copy (cp)
```bash
fastsync cp file.txt /backup/
fastsync cp -R ~/docs server:/backup/
fastsync cp -a ~/data s3://bucket/
```

### Sync
```bash
fastsync sync ~/docs server:/backup/
fastsync sync -d ~/www server:/www/
fastsync sync -n ~/data s3://bucket/  # dry run
```

### List (ls)
```bash
fastsync ls /tmp
fastsync ls -l -h s3://bucket/
fastsync ls -R ~/projects
```

### Download (dl)
```bash
fastsync dl https://example.com/file.zip
fastsync dl https://example.com/file.zip ./downloads/
fastsync dl https://example.com/file.zip -p 16 -r
fastsync dl url1 url2 url3 ./downloads/
```

## Global Flags

| Flag | Description |
|------|-------------|
| `-p N` | Parallel streams |
| `-c MB` | Chunk size (default: 8) |
| `-b MB` | Buffer size (default: 32) |
| `-r` | Resume transfers |
| `-C` | Verify checksums |
| `-z` | Enable compression |
| `-P` | Show progress |
| `-q` | Quiet mode |
| `-v` | Verbose (use -vv, -vvv) |

## Copy Flags

| Flag | Description |
|------|-------------|
| `-R` | Recursive |
| `-a` | Preserve attributes |
| `-L` | Follow symlinks |
| `-f` | Force overwrite |
| `-n` | Dry run |
| `-i PATTERN` | Include |
| `-e PATTERN` | Exclude |

## Sync Flags

| Flag | Description |
|------|-------------|
| `-d` | Delete extra files |
| `-u` | Update only |
| `-c` | Compare by checksum |
| `-a` | Preserve attributes |
| `-n` | Dry run |
| `-w` | Watch for changes |

## List Flags

| Flag | Description |
|------|-------------|
| `-l` | Long format |
| `-h` | Human-readable sizes |
| `-R` | Recursive |
| `-s FIELD` | Sort by (name/size/time) |

## Download Flags

| Flag | Description |
|------|-------------|
| `-p N` | Parallel streams |
| `-c MB` | Chunk size |
| `-r` | Resume |
| `-f` | Force overwrite |
