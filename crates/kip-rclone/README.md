# kip-rclone

Cloud storage backup support for Kip via [rclone](https://rclone.org).

## Features

- **70+ Cloud Providers**: Google Drive, Nextcloud, Dropbox, OneDrive, S3, SFTP, WebDAV, and more
- **Familiar API**: Similar to `kip-rsync` for easy integration
- **Type-Safe Destinations**: Compile-time checked cloud configuration
- **Progress Tracking**: Real-time transfer statistics
- **Error Handling**: Detailed error messages for common cloud issues

## Installation

### 1. Install rclone

```bash
# macOS
brew install rclone

# Linux
curl https://rclone.org/install.sh | sudo bash

# Windows
choco install rclone
```

### 2. Configure rclone remotes

```bash
rclone config
```

Follow the interactive setup to configure your cloud providers:
- Google Drive: Choose "google drive" type, follow OAuth flow
- Nextcloud: Choose "webdav" type, enter server URL and credentials
- S3: Choose "s3" type, enter AWS credentials
- etc.

### 3. Add to your project

```toml
[dependencies]
kip-rclone = { path = "../crates/kip-rclone" }
```

## Usage

### Basic Copy

```rust
use kip_rclone::{CloudDestination, Rclone};

let gdrive = CloudDestination::google_drive("my_gdrive", "backups");
let stats = Rclone::new().copy("/local/path", &gdrive, "").await?;
println!("Copied {} files", stats.files_transferred);
```

### Sync (with deletion)

```rust
use kip_rclone::{CloudDestination, Rclone};

let nextcloud = CloudDestination::nextcloud("nextcloud", "kip_backups");
let stats = Rclone::new()
    .sync("/local/path", &nextcloud, "")
    .await?;
println!("Synced {} bytes", stats.bytes_transferred);
```

### With Options

```rust
use kip_rclone::{CloudDestination, Rclone};

let rclone = Rclone::new()
    .with_verbose()      // Verbose output
    .with_dry_run();     // Test without actually transferring

let stats = rclone.copy("/local/path", &gdrive, "").await?;
```

### Check Disk Usage

```rust
use kip_rclone::{CloudDestination, Rclone};

let usage = Rclone::new().disk_usage(&gdrive).await?;
if let (Some(total), Some(used)) = (usage.total_bytes, usage.used_bytes) {
    println!("Using {} of {}", 
        CloudStats::format_bytes(used),
        CloudStats::format_bytes(total));
}
```

## Supported Providers

### First-Class Support
- **GoogleDrive** - `CloudDestination::google_drive(remote, path)`
- **Nextcloud** - `CloudDestination::nextcloud(remote, path)`
- **S3** - `CloudDestination::s3(remote, bucket)`
- **WebDAV** - `CloudDestination::webdav(remote, path, url)`

### Generic Remote
Any rclone remote can be used:
```rust
let dest = CloudDestination::generic("dropbox", "backups/kip");
```

## Examples

See `tests/integration.rs` for complete examples of:
- Google Drive backup
- Nextcloud sync
- S3 upload
- Disk usage monitoring
- Dry-run testing

## Testing

Tests require rclone to be installed and configured:

```bash
# Run unit tests (no rclone needed)
cargo test -p kip-rclone

# Run integration tests (requires rclone + configured remotes)
cargo test -p kip-rclone -- --ignored
```

## Error Handling

The crate provides detailed error types:

```rust
use kip_rclone::CloudError;

match Rclone::new().copy("/path", &dest, "").await {
    Ok(stats) => println!("Success!"),
    Err(CloudError::RcloneNotFound) => eprintln!("Please install rclone"),
    Err(CloudError::AuthenticationFailed(provider, msg)) => {
        eprintln!("Auth failed for {}: {}", provider, msg)
    }
    Err(CloudError::RateLimitExceeded(provider)) => {
        eprintln!("Rate limited by {}", provider)
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Performance Tips

1. **Parallel Transfers**: Default is 4 transfers, 8 checkers
2. **Chunk Size**: Large files benefit from `--s3-upload-chunk-size`
3. **Caching**: Use `--drive-use-trash` for faster deletes on Google Drive
4. **Bandwidth**: Use `--bwlimit` to avoid throttling

## Limitations

- Requires rclone binary in PATH
- OAuth tokens stored in rclone config (not managed by this crate)
- Rate limits depend on provider
- Some providers have file size limits

## License

Same as Kip project license.
