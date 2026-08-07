# kip-rsync Test Suite

Comprehensive test suite for the kip-rsync crate.

## Running Tests

### All Tests (Local Only)
```bash
cargo test -p kip-rsync
```

### Include SSH Integration Tests (Requires Docker)
```bash
# Start Docker Desktop or docker daemon first
cargo test -p kip-rsync -- --ignored
```

### Run Specific Test Categories
```bash
# Unit tests only
cargo test -p kip-rsync --lib

# Integration tests only  
cargo test -p kip-rsync --test integration

# SSH unit tests only
cargo test -p kip-rsync --test ssh_unit

# SSH integration tests only (requires Docker)
cargo test -p kip-rsync --test ssh_integration -- --ignored
```

## Test Coverage

### Local Rsync Tests (16 tests)
- ✅ Basic backup operations
- ✅ Backup with excludes
- ✅ Incremental backup
- ✅ Empty source handling
- ✅ Empty directory preservation
- ✅ Symlink preservation (Unix)
- ✅ Unicode filenames
- ✅ Dry-run mode
- ✅ Delete behavior (--delete flag)
- ✅ Long/deep paths
- ✅ Large file transfer (10MB)
- ✅ Progress callbacks
- ✅ Progress byte tracking
- ✅ Stats formatting
- ✅ Parallel execution
- ✅ Error handling

### SSH Unit Tests (20 tests)
- ✅ SSH destination creation
- ✅ SSH destination with port
- ✅ SSH destination with identity
- ✅ SSH options building
- ✅ SSH command structure
- ✅ SSH error detection
- ✅ Command building variations

### SSH Integration Tests (4 tests, requires Docker)
- ⚠️ Basic SSH backup
- ⚠️ SSH backup with progress
- ⚠️ SSH incremental backup
- ⚠️ SSH large file backup

## SSH Integration Test Requirements

### Docker Required
The SSH integration tests require Docker to run an isolated SSH server container.

**Setup:**
1. Install Docker Desktop (macOS/Windows) or docker.io (Linux)
2. Start Docker daemon
3. Ensure you have permission to run Docker commands

**Test Container:**
- Image: `lscr.io/linuxserver/openssh-server:latest`
- Port: Random available port (22000-23000)
- User: `testuser`
- Password: `testpassword123`
- Authentication: SSH key (auto-generated per test)

**Cleanup:**
- Containers are automatically stopped and removed after each test
- SSH keys are automatically deleted
- If a test panics, run manually: `docker ps -a | grep kip_rsync_ssh_test | awk '{print $1}' | xargs docker rm -f`

## Test Infrastructure

### LocalTestTempDir
Automatic temporary directory management with test filesystem:

```rust
#[tokio::test]
async fn test_example() {
    let src = LocalTestTempDir::new("test_src").unwrap();
    let dst = LocalTestTempDir::empty("test_dst").unwrap();
    
    // Directories automatically cleaned up on drop
}
```

**Features:**
- Unique names prevent collisions
- Automatic cleanup on drop
- Preserve for debugging: `.no_cleanup()` or `KIP_RSYNC_KEEP_TEMP=1`
- Pre-populated test filesystem (10+ files, nested dirs, symlinks)

### Test Filesystem Structure
```
root/
├── file1.txt
├── file2.dat
├── subdir1/
│   ├── nested1.txt
│   └── nested2.txt
├── subdir2/deep/deep_file.txt
├── .hidden/secret.txt
├── large_file.bin (1MB)
├── empty_dir/
├── file with spaces.txt
└── symlink.txt -> file1.txt
```

## Debugging Failed Tests

### Keep Temp Directories
```bash
# Preserve all temp dirs
KIP_RSYNC_KEEP_TEMP=1 cargo test -p kip-rsync

# Or disable cleanup per-test
let src = LocalTestTempDir::new("test").unwrap().no_cleanup();
```

### Verbose Output
```bash
# Show container startup logs
RUST_LOG=debug cargo test -p kip-rsync -- --ignored --nocapture

# Show specific test output
cargo test -p kip-rsync --test ssh_integration -- --ignored --nocapture
```

### Common Issues

**Docker not running:**
```
Failed to connect to the docker API
```
→ Start Docker Desktop or `systemctl start docker`

**Permission denied:**
```
dial unix /var/run/docker.sock: permission denied
```
→ Add user to docker group: `sudo usermod -aG docker $USER`

**Port conflicts:**
```
Bind for 0.0.0.0:22000 failed: address already in use
```
→ Tests automatically find available ports, but you may need to close conflicting services

## CI/CD Integration

### GitHub Actions Example
```yaml
- name: Test kip-rsync
  run: cargo test -p kip-rsync

- name: Test SSH integration
  run: cargo test -p kip-rsync --test ssh_integration -- --ignored
  services:
    docker:
      image: docker:dind
```

### Test Matrix
```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest]
    rust: [stable, nightly]
```

## Performance

**Typical test run times:**
- Unit tests: < 0.1s
- Local integration: 0.5-1s
- SSH integration (with Docker): 10-30s per test

**Total suite:** ~40 tests in < 2 seconds (without SSH integration)
