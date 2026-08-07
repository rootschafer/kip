# Cloud Backup with Kip

Kip now supports backing up to **70+ cloud storage providers** via [rclone](https://rclone.org).

## Quick Start

### 1. Install rclone

```bash
# macOS
brew install rclone

# Linux  
curl https://rclone.org/install.sh | sudo bash

# Windows
choco install rclone
```

### 2. Configure Cloud Storage

```bash
rclone config
```

Follow the interactive setup:

**Google Drive:**
```
n) New remote
name> gdrive
Storage> Google Drive
client_id> (leave blank)
client_secret> (leave blank)
scope> 1 (Full access)
root_folder_id> (leave blank)
service_account_file> (leave blank)
Use auto config? > Y (opens browser)
```

**Nextcloud (WebDAV):**
```
n) New remote
name> nextcloud
Storage> WebDAV
url> https://your-nextcloud.com/remote.php/dav/files/YOUR_USERNAME
vendor> other
user> your_username
password> your_password
```

**Amazon S3:**
```
n) New remote
name> s3
Storage> Amazon S3
provider> AWS
env_auth> false
access_key_id> YOUR_ACCESS_KEY
secret_access_key> YOUR_SECRET_KEY
region> us-east-1
```

### 3. Verify Configuration

```bash
kip cloud-remotes
```

Output:
```
📋 Configured rclone remotes:

   ✅ gdrive
   ✅ nextcloud
   ✅ s3
```

## Usage

### Copy Files to Cloud

```bash
# Copy to Google Drive
kip cloud-copy /path/to/backup gdrive:backups/myfiles

# Copy to Nextcloud
kip cloud-copy /path/to/backup nextcloud:backups

# With verbose output
kip cloud-copy /path/to/backup gdrive:backups -v
```

### Sync to Cloud (Recommended)

Sync works like `rsync --delete` - it makes the cloud match your local files:

```bash
# Sync to Google Drive
kip cloud-sync /path/to/backup gdrive:backups/myfiles

# Dry-run first (see what would change)
kip cloud-sync /path/to/backup gdrive:backups/myfiles --dry-run

# With verbose output
kip cloud-sync /path/to/backup gdrive:backups/myfiles -v
```

### Check Storage Usage

```bash
kip cloud-usage gdrive
```

Output:
```
📊 Checking cloud storage usage...

   Remote: gdrive
   Total:  15.00 GB
   Used:   5.23 GB
   Usage:  34%
   Free:   9.77 GB
```

## Examples

### Backup Critical Files to Multiple Clouds

```bash
# Backup to Google Drive
kip cloud-sync ~/.ssh gdrive:backups/identity
kip cloud-sync ~/.gnupg gdrive:backups/identity

# Also backup to S3 for redundancy
kip cloud-sync ~/.ssh s3:my-bucket/backups/identity
kip cloud-sync ~/.gnupg s3:my-bucket/backups/identity
```

### Backup Large Media Folder

```bash
# Sync photos to Google Drive (15GB free tier)
kip cloud-sync ~/Photos gdrive:photos

# Check usage after
kip cloud-usage gdrive
```

### Automated Backup Script

```bash
#!/bin/bash
# backup-to-cloud.sh

set -e

echo "🔄 Starting cloud backup..."

# Dry run first
echo "🔍 Checking what would change..."
kip cloud-sync ~/important-data gdrive:backups --dry-run

# Actual sync
echo "☁️  Syncing to cloud..."
kip cloud-sync ~/important-data gdrive:backups -v

# Check storage
echo "📊 Storage usage:"
kip cloud-usage gdrive

echo "✅ Backup complete!"
```

## Supported Providers

### First-Class Support

| Provider | Command | Notes |
|----------|---------|-------|
| Google Drive | `gdrive:path` | 15GB free, OAuth2 |
| Nextcloud | `nextcloud:path` | Self-hosted, WebDAV |
| Amazon S3 | `s3:bucket/path` | Pay per GB |
| Dropbox | `dropbox:path` | 2GB free |
| OneDrive | `onedrive:path` | 5GB free |
| Backblaze B2 | `b2:bucket/path` | Cheap, $0.005/GB |

### Any rclone Provider

Kip works with **all 70+ rclone providers**:
- Box, Mega, pCloud, Yandex
- SFTP, FTP, HTTP
- Local/Network drives
- Enterprise: Oracle Cloud, IBM COS, Alibaba OSS

See full list: https://rclone.org/overview/

## Best Practices

### 1. Use Sync, Not Copy

```bash
# ✅ Recommended - keeps cloud in sync
kip cloud-sync /local gdrive:backup

# ⚠️ Copy only adds, never deletes
kip cloud-copy /local gdrive:backup
```

### 2. Dry-Run First

```bash
# See what would change
kip cloud-sync /local gdrive:backup --dry-run

# Then execute
kip cloud-sync /local gdrive:backup
```

### 3. Monitor Storage

```bash
# Check before large backups
kip cloud-usage gdrive

# Set up alerts (external)
if [ $(kip cloud-usage gdrive | grep "Usage:" | awk '{print $2}' | tr -d '%') -gt 80 ]; then
    echo "Warning: Cloud storage over 80%!"
fi
```

### 4. Encrypt Sensitive Data

```bash
# Create encrypted remote
rclone config
n) New remote
name> encrypted_gdrive
Storage> Crypt
remote> gdrive:backups
filename_encryption> standard
directory_name_encryption> true
password> YOUR_PASSWORD
```

Then use: `kip cloud-sync /data encrypted_gdrive:`

### 5. Bandwidth Limiting

```bash
# Limit to 1MB/s to avoid throttling
rclone copy /local gdrive:backup --bwlimit 1M
```

## Troubleshooting

### "rclone not found"

Install rclone (see Quick Start above).

### "Authentication failed"

Re-run `rclone config` and re-authenticate:
```bash
rclone config
e) Edit existing remote
name> gdrive
...
Use auto config? > Y
```

### "Rate limit exceeded"

Wait a few minutes or use bandwidth limiting:
```bash
rclone sync /local gdrive:backup --bwlimit 500K
```

### "Insufficient storage"

Check usage and free up space:
```bash
kip cloud-usage gdrive
rclone delete gdrive:backups/old-files
```

## Advanced: Custom rclone Flags

For advanced options, use rclone directly:

```bash
# With custom transfer settings
rclone sync /local gdrive:backup \
  --transfers=8 \
  --checkers=16 \
  --s3-upload-chunk-size=64M

# With filters
rclone sync /local gdrive:backup \
  --include "*.txt" \
  --exclude "*.log"
```

## Integration with Kip Backup

Combine local and cloud backups:

```bash
# Local backup (fast, complete)
kip backup --limit 10

# Cloud backup (offsite, critical files only)
kip cloud-sync ~/.ssh gdrive:backups/identity
kip cloud-sync ~/.gnupg gdrive:backups/identity
kip cloud-sync ~/Documents nextcloud:backups/docs
```

## Security Notes

- OAuth tokens stored in `~/.config/rclone/rclone.conf` (mode 600)
- Use encrypted remotes for sensitive data
- Enable 2FA on cloud accounts
- Regularly audit access: `rclone config show`

## Resources

- [rclone Documentation](https://rclone.org/docs/)
- [rclone Download](https://rclone.org/downloads/)
- [Kip Documentation](../../README.md)
