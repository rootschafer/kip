# Cloud Backup Integration

Kip now supports cloud storage as a first-class backup destination, integrated directly into the normal backup flow.

## Setup

### 1. Install rclone

```bash
# macOS
brew install rclone

# Linux
curl https://rclone.org/install.sh | sudo bash
```

### 2. Configure Cloud Remotes

```bash
rclone config
```

Follow the interactive setup for your provider:
- **Google Drive**: Choose "google drive", follow OAuth flow
- **Nextcloud**: Choose "webdav", enter server URL and credentials
- **Amazon S3**: Choose "s3", enter AWS credentials
- etc.

### 3. Add Cloud Drives to Config

Edit `~/.config/kip/drives.toml`:

```toml
[[drives]]
name = "gdrive"
type = "cloud"
rclone_remote = "gdrive"      # Your rclone remote name
rclone_path = "kip_backups"   # Path within remote
```

### 4. Run Backup

```bash
kip backup
```

Cloud destinations are now backed up automatically along with local/SSH!

## Configuration

### drives.toml Format

```toml
[[drives]]
name = "mycloud"
type = "cloud"
rclone_remote = "gdrive"      # Required: rclone remote name
rclone_path = "backups"       # Optional: path within remote (default: "")
check_mounted = false         # Cloud is always available
```

### Complete Example

```toml
# Local backup (fast, complete)
[[drives]]
name = "flash"
type = "local"
mount_point = "/Volumes/BACKUP"

# SSH backup (offsite, complete)
[[drives]]
name = "server"
type = "ssh"
host = "backup.example.com"
user = "backup"
path = "/backups"

# Cloud backup (offsite, critical files only)
[[drives]]
name = "gdrive"
type = "cloud"
rclone_remote = "gdrive"
rclone_path = "kip_backups"

[[drives]]
name = "nextcloud"
type = "cloud"
rclone_remote = "nextcloud"
rclone_path = "backups"
```

## How It Works

When you run `kip backup`:

1. **Local drives** → rsync directly
2. **SSH drives** → rsync over SSH
3. **Cloud drives** → zip folder, upload via rclone

Cloud backups use zip compression to minimize transfer size and API calls.

## Supported Providers

Any provider supported by rclone (70+):

- Google Drive (15GB free)
- Nextcloud (self-hosted)
- Amazon S3 (paid)
- Dropbox (2GB free)
- OneDrive (5GB free)
- Backblaze B2 ($0.005/GB)
- pCloud (10GB free)
- Mega (20GB free)
- SFTP/FTP servers
- And 60+ more

## Best Practices

### 1. Use Cloud for Critical Files Only

Cloud storage is slower and often has costs. Recommended:

```toml
# In your app configs (apps/*.toml)

# Identity files → backup to ALL destinations
[[folder_configs]]
source = "~/.ssh"
priority = 1000
destinations = [
    { drive = "flash", path = "identity" },
    { drive = "server", path = "identity" },
    { drive = "gdrive", path = "identity" },  # ✅ Critical
]

# Large media → backup to local/SSH only
[[folder_configs]]
source = "~/Photos"
priority = 400
destinations = [
    { drive = "flash", path = "photos" },
    { drive = "server", path = "photos" },
    # Not cloud - too large
]
```

### 2. Monitor Storage Quotas

```bash
# Check cloud usage
kip cloud-usage gdrive
kip cloud-usage nextcloud
```

### 3. Encrypt Sensitive Data

Create encrypted rclone remote:

```bash
rclone config
n) New remote
name> encrypted_gdrive
Storage> Crypt
remote> gdrive:backups
filename_encryption> standard
password> YOUR_PASSWORD
```

Then use in config:
```toml
[[drives]]
name = "encrypted"
type = "cloud"
rclone_remote = "encrypted_gdrive"
rclone_path = "sensitive"
```

### 4. Bandwidth Limiting

For cloud backups, set bandwidth limits in rclone config or use:

```bash
rclone copy /local gdrive:backup --bwlimit 1M
```

## Troubleshooting

### "rclone not found"

Install rclone (see Setup above).

### "Remote not configured"

Run `rclone config` to set up the remote.

### "Authentication failed"

Re-authenticate:
```bash
rclone config
e) Edit existing remote
name> gdrive
...
Use auto config? > Y
```

### "Rate limit exceeded"

Wait a few minutes or reduce bandwidth:
```bash
rclone config
e) Edit remote
gdrive> Advanced config
yes> upload_rate_limit> 1M
```

## Commands

### Standard Backup (includes cloud)

```bash
kip backup                    # Backup to all configured drives
kip backup --filter identity  # Backup only folders matching "identity"
kip backup --limit 5          # Backup first 5 folders
```

### Cloud-Specific Commands

```bash
# Manual cloud copy
kip cloud-copy /path/to/file gdrive:backups

# Sync to cloud
kip cloud-sync /path/to/folder nextcloud:backups

# Check configured remotes
kip cloud-remotes

# Check storage usage
kip cloud-usage gdrive
```

## Limitations

- **Cloud restore not yet implemented** - Can backup to cloud, but restore requires `kip cloud-copy` or rclone directly
- **Zip-only for cloud** - Cloud backups always use zip compression
- **No incremental cloud backups** - Each backup uploads full zip (rclone deduplication may help)

## Future Work

- [ ] Cloud restore support
- [ ] Incremental cloud backups
- [ ] Direct cloud sync (no zip)
- [ ] Cloud-specific backup policies
- [ ] Storage quota warnings

## Resources

- [rclone Documentation](https://rclone.org/docs/)
- [rclone Cloud Providers](https://rclone.org/overview/)
- [Kip Documentation](../README.md)
