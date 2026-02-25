# OpenClaw Setup for Kip on NixOS (Headless)

**Date:** February 22, 2026

---

## The Challenge

Kip uses Dioxus desktop, which requires a display. On a headless NixOS server, you have three options:

1. **Xvfb** (X Virtual Framebuffer) — Run Dioxus desktop in a virtual display
2. **Dioxus Web Target** — Build for web, access via browser
3. **SSH Display Forwarding** — Forward X11 over SSH

**Recommended:** Use **Xvfb** for automated testing + **Web target** for manual inspection.

---

## Option 1: Xvfb (Recommended for Automation)

### NixOS Configuration

Add to your NixOS configuration (`configuration.nix`):

```nix
{ config, pkgs, ... }: {
  # Xvfb for headless testing
  services.xserver.enable = true;
  services.xserver.displayManager.auto.enable = false;
  
  # Required packages
  environment.systemPackages = with pkgs; [
    xvfb
    libxkbcommon
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libXext
    fontconfig
    freetype
  ];
}
```

### Running with Xvfb

```bash
# Start Xvfb on display :99
Xvfb :99 -screen 0 1920x1080x24 &
export DISPLAY=:99

# Now run Dioxus commands
dx check
dx build
dx serve --platform desktop  # Will run headlessly
```

### OpenClaw + Xvfb Wrapper

Create `scripts/run-with-xvfb.sh`:

```bash
#!/usr/bin/env bash
set -e

# Start Xvfb if not running
if [ -z "$DISPLAY" ]; then
    export DISPLAY=:99
    Xvfb $DISPLAY -screen 0 1920x1080x24 &
    XVFB_PID=$!
    trap "kill $XVFB_PID" EXIT
fi

# Wait for Xvfb to start
sleep 2

# Run the command
exec "$@"
```

Usage:
```bash
chmod +x scripts/run-with-xvfb.sh
./scripts/run-with-xvfb.sh dx check
./scripts/run-with-xvfb.sh dx build
```

---

## Option 2: Dioxus Web Target (Recommended for Inspection)

### Build for Web

```bash
# Install wasm-pack if not installed
cargo install wasm-pack

# Build for web
dx serve --platform web

# Access at http://localhost:8080
```

### NixOS Configuration

```nix
{ config, pkgs, ... }: {
  environment.systemPackages = with pkgs; [
    wasm-pack
    nodejs
    trunk  # Alternative to dx serve for web
  ];
}
```

### Access from Remote Machine

```bash
# On server, start web server
dx serve --platform web --host 0.0.0.0

# On your local machine, SSH tunnel
ssh -L 8080:localhost:8080 your-nixos-server

# Open browser to http://localhost:8080
```

---

## Option 3: SSH Display Forwarding

### Enable X11 Forwarding

On NixOS server (`/etc/ssh/sshd_config`):
```
X11Forwarding yes
```

### Connect with Forwarding

```bash
# From your local machine
ssh -X your-nixos-server

# Verify DISPLAY is set
echo $DISPLAY  # Should show localhost:10.0 or similar

# Run Dioxus
dx serve --platform desktop

# The window will appear on your local machine
```

**Note:** This requires your local machine to have X11 running and can be slow over network.

---

## Updated OpenClaw Configuration for NixOS

Create `openclaw-nixos.yaml`:

```yaml
project:
  name: kip
  root: .
  
# NixOS-specific environment
environment:
  DISPLAY: ":99"
  NIXOS_HEADLESS: "true"
  
# Wrapper script for all commands
command_wrapper: "./scripts/run-with-xvfb.sh"

agents:
  - name: architect
    role: design_and_planning
    model: claude-sonnet-4-20250514
    context_files:
      - notes/the_design/INTERACTION_MODEL.md
      - notes/the_design/COMPREHENSIVE_DEVELOPMENT_PLAN.md
      - notes/the_design/START_HERE.md
      
  - name: frontend_dev
    role: ui_implementation
    model: claude-sonnet-4-20250514
    context_files:
      - src/ui/graph.rs
      - src/ui/graph_nodes.rs
      - src/ui/graph_store.rs
      - assets/main.css
    build_command: "./scripts/run-with-xvfb.sh dx build"
    test_command: "./scripts/run-with-xvfb.sh dx check"
      
  - name: backend_dev
    role: backend_implementation
    model: claude-sonnet-4-20250514
    context_files:
      - src/api/
      - src/engine/
      - src/db/
    build_command: "cargo build --bin kip-cli"
    test_command: "cargo test --test api_tests -- --test-threads=1"
      
  - name: tester
    role: testing_and_validation
    model: claude-sonnet-4-20250514
    context_files:
      - tests/
      - notes/the_design/INTERACTION_MODEL.md
    test_command: "./scripts/run-with-xvfb.sh cargo test"

# Web inspection server (for manual testing)
web_inspection:
  enabled: true
  command: "dx serve --platform web --host 0.0.0.0"
  port: 8080
  ssh_tunnel: "ssh -L 8080:localhost:8080 your-nixos-server"

tasks:
  # Phase 1: Interaction Model
  - id: interaction_refactor
    name: "Fix Click/Drag Conflict"
    description: |
      Implement new interaction model:
      - Single click selects only (no drag start)
      - Click + drag moves node
      - Double click opens context menu
    assigned_to: frontend_dev
    dependencies: []
    files:
      - src/ui/graph.rs
      - src/ui/graph_nodes.rs
      - src/ui/graph_store.rs
    acceptance_criteria:
      - "Single click only selects node"
      - "Drag moves node without re-selecting"
      - "Double click opens context menu"
    test_command: "./scripts/run-with-xvfb.sh dx check"
      
  - id: context_menus
    name: "Implement Context Menus"
    description: |
      Add node-type-specific context menus
    assigned_to: frontend_dev
    dependencies: [interaction_refactor]
    files:
      - src/ui/graph.rs
      - src/ui/graph_store.rs
      - assets/main.css
    test_command: "./scripts/run-with-xvfb.sh dx check"
      
  - id: keyboard_shortcuts
    name: "Add Keyboard Shortcuts"
    description: |
      Implement keyboard shortcuts
    assigned_to: frontend_dev
    dependencies: [interaction_refactor]
    files:
      - src/ui/graph.rs
    test_command: "./scripts/run-with-xvfb.sh dx check"
      
  - id: test_interactions
    name: "Test Interaction Model"
    description: |
      Validate all interaction changes
    assigned_to: tester
    dependencies: [interaction_refactor, context_menus, keyboard_shortcuts]
    files:
      - tests/
    test_command: "./scripts/run-with-xvfb.sh cargo test"

workflow:
  - phase: interaction_model
    tasks: [interaction_refactor, context_menus, keyboard_shortcuts]
    
  - phase: validation
    tasks: [test_interactions]
    
coordination:
  sync_points:
    - after: interaction_refactor
      review_by: [architect, tester]
    - after: context_menus
      review_by: [architect]
    - after: keyboard_shortcuts
      review_by: [architect, tester]
      
  code_review:
    required: true
    reviewers: [architect]
    min_approvals: 1
    
  ci_checks:
    - "./scripts/run-with-xvfb.sh dx check"
    - "cargo test --test api_tests -- --test-threads=1"
    - "./scripts/run-with-xvfb.sh cargo test --test integration_tests -- --test-threads=1"
```

---

## NixOS Shell (Recommended Approach)

Create `shell.nix` in project root:

```nix
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "kip-dev";
  
  buildInputs = with pkgs; [
    # Rust toolchain
    rustup
    
    # Dioxus CLI
    (rustPlatform.buildRustPackage {
      pname = "dioxus-cli";
      version = "0.7.3";
      src = pkgs.fetchFromGitHub {
        owner = "DioxusLabs";
        repo = "dioxus";
        rev = "v0.7.3";
        sha256 = "sha256-CHANGE-ME";
      };
      cargoHash = "sha256-CHANGE-ME";
    })
    
    # Xvfb for headless testing
    xvfb-run
    libxkbcommon
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libXext
    
    # Web target
    wasm-pack
    nodejs
    
    # Font support
    fontconfig
    freetype
  ];
  
  shellHook = ''
    export DISPLAY=:99
    
    # Auto-start Xvfb if not running
    if ! pgrep -x Xvfb > /dev/null; then
        Xvfb $DISPLAY -screen 0 1920x1080x24 &
        echo "Started Xvfb on $DISPLAY"
    fi
  '';
}
```

Usage:
```bash
# Enter development shell
nix-shell

# Now all commands work headlessly
dx check
dx build
dx serve --platform desktop  # Runs in Xvfb
dx serve --platform web     # Access via browser
```

---

## Automated Testing Script

Create `scripts/test-headless.sh`:

```bash
#!/usr/bin/env bash
set -e

echo "=== Kip Headless Test Suite ==="

# Start Xvfb
export DISPLAY=:99
Xvfb $DISPLAY -screen 0 1920x1080x24 &
XVFB_PID=$!
trap "kill $XVFB_PID" EXIT

echo "Waiting for Xvfb to start..."
sleep 2

# Run checks
echo "Running dx check..."
dx check

echo "Running API tests..."
cargo test --test api_tests -- --test-threads=1

echo "Running integration tests..."
cargo test --test integration_tests -- --test-threads=1

echo "=== All tests passed! ==="
```

Usage:
```bash
chmod +x scripts/test-headless.sh
./scripts/test-headless.sh
```

---

## CI/CD Integration (GitHub Actions)

Create `.github/workflows/test.yml`:

```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-action@stable
      
    - name: Install Dioxus CLI
      run: cargo install --git https://github.com/DioxusLabs/dioxus dioxus-cli
      
    - name: Install Xvfb
      run: sudo apt-get install xvfb
      
    - name: Run tests headlessly
      run: |
        Xvfb :99 -screen 0 1920x1080x24 &
        export DISPLAY=:99
        sleep 2
        dx check
        cargo test --test api_tests -- --test-threads=1
        cargo test --test integration_tests -- --test-threads=1
```

---

## Quick Start for NixOS

```bash
# 1. Clone repository
git clone https://github.com/yourusername/kip.git
cd kip

# 2. Enter Nix shell (sets up everything)
nix-shell

# 3. Verify setup
dx --version
cargo --version

# 4. Run tests
./scripts/test-headless.sh

# 5. Start OpenClaw
openclaw start --config openclaw-nixos.yaml

# 6. For manual inspection, start web server
dx serve --platform web --host 0.0.0.0

# 7. From your local machine, SSH tunnel
ssh -L 8080:localhost:8080 your-nixos-server
# Then open http://localhost:8080 in browser
```

---

## Troubleshooting

### "Cannot open display"

```bash
# Check if Xvfb is running
pgrep -x Xvfb

# If not, start it
Xvfb :99 -screen 0 1920x1080x24 &
export DISPLAY=:99
```

### "Dioxus CLI not found"

```bash
# Install in nix-shell
cargo install --git https://github.com/DioxusLabs/dioxus dioxus-cli

# Or add to shell.nix buildInputs
```

### Web target not working

```bash
# Install wasm-pack
cargo install wasm-pack

# Or in nix-shell
nix-shell -p wasm-pack
```

### Fonts not rendering

```bash
# In nix-shell, add to buildInputs:
pkgs.fontconfig
pkgs.freetype
```

---

## Next Steps

1. **Set up NixOS configuration** — Add Xvfb and required packages
2. **Create `shell.nix`** — Development environment
3. **Test headless setup** — Run `./scripts/test-headless.sh`
4. **Configure OpenClaw** — Use `openclaw-nixos.yaml`
5. **Start coordination** — `openclaw start --config openclaw-nixos.yaml`
6. **Access web inspection** — SSH tunnel to port 8080

For questions, see:
- `notes/the_design/START_HERE.md` — Project overview
- `OPENCLAW_SETUP.md` — General OpenClaw setup (non-NixOS)


---

## Automated Visual Testing (Hands-Off)

This section covers **fully automated** visual testing — no manual intervention required.

### What's Automated

- ✅ Screenshot capture during tests
- ✅ Screenshot comparison (regression detection)
- ✅ UI element existence validation
- ✅ Render error detection
- ✅ Playwright-based interaction testing

### What's NOT Automated

- ❌ Visual design approval ("does this look good?")
- ❌ Layout aesthetics judgment
- ❌ Color scheme validation

---

## Setup: Automated Screenshot Testing

### 1. Add Dependencies to `shell.nix`

```nix
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "kip-dev";
  
  buildInputs = with pkgs; [
    # ... existing dependencies ...
    
    # Screenshot tools
    imagemagick  # For screenshot capture and comparison
    scrot        # Alternative screenshot tool
    
    # Playwright for browser automation
    playwright-driver
    nodejs
    
    # For headless Chrome/Chromium
    chromium
  ];
}
```

### 2. Create Screenshot Test Script

Create `scripts/visual-test.sh`:

```bash
#!/usr/bin/env bash
set -e

echo "=== Kip Visual Test Suite ==="

# Configuration
export DISPLAY=:99
SCREENSHOT_DIR="/tmp/kip-screenshots"
BASELINE_DIR="/tmp/kip-baselines"
TOLERANCE=2  # Pixel difference tolerance for comparison

# Create directories
mkdir -p "$SCREENSHOT_DIR" "$BASELINE_DIR"

# Start Xvfb
Xvfb $DISPLAY -screen 0 1920x1080x24 &
XVFB_PID=$!
trap "kill $XVFB_PID" EXIT

echo "Waiting for Xvfb to start..."
sleep 2

# Start web server in background
echo "Starting Kip web server..."
dx serve --platform web --host 127.0.0.1 --port 8080 &
SERVER_PID=$!
trap "kill $SERVER_PID; kill $XVFB_PID" EXIT

echo "Waiting for server to start..."
sleep 5

# Function to take screenshot
take_screenshot() {
    local name=$1
    echo "Taking screenshot: $name"
    
    # Use playwright to capture browser screenshot
    npx playwright screenshot \
        --wait-for-selector "#root" \
        --timeout 10000 \
        "http://localhost:8080" \
        "$SCREENSHOT_DIR/$name.png"
}

# Function to compare screenshots
compare_screenshots() {
    local name=$1
    local baseline="$BASELINE_DIR/$name.png"
    local current="$SCREENSHOT_DIR/$name.png"
    
    if [ ! -f "$baseline" ]; then
        echo "⚠️  No baseline for $name, creating..."
        cp "$current" "$baseline"
        return 0
    fi
    
    # Compare with ImageMagick
    echo "Comparing: $name"
    compare -metric AE \
        "$baseline" \
        "$current" \
        "$SCREENSHOT_DIR/${name}_diff.png" 2>&1 | {
        read diff_pixels
        if [ "$diff_pixels" -gt "$TOLERANCE" ]; then
            echo "❌ REGRESSION: $name ($diff_pixels pixels different)"
            return 1
        else
            echo "✅ PASS: $name"
            return 0
        fi
    }
}

# Run tests
echo ""
echo "=== Running Visual Tests ==="

# Test 1: App loads
take_screenshot "01_app_load"
if compare_screenshots "01_app_load"; then
    echo "✅ App load test passed"
else
    echo "❌ App load test failed"
    exit 1
fi

# Test 2: Nodes render
# (Wait for nodes to load)
sleep 3
take_screenshot "02_nodes_render"
if compare_screenshots "02_nodes_render"; then
    echo "✅ Node rendering test passed"
else
    echo "❌ Node rendering test failed"
    exit 1
fi

# Test 3: Toolbar visible
take_screenshot "03_toolbar"
if compare_screenshots "03_toolbar"; then
    echo "✅ Toolbar test passed"
else
    echo "❌ Toolbar test failed"
    exit 1
fi

echo ""
echo "=== All Visual Tests Passed ==="
```

Usage:
```bash
chmod +x scripts/visual-test.sh
./scripts/visual-test.sh
```

### 3. Create Baseline Screenshots

First run creates baselines:
```bash
./scripts/visual-test.sh
# Creates baselines in /tmp/kip-baselines/
```

Subsequent runs compare against baselines and detect regressions.

---

## Playwright UI Interaction Testing

### Create Playwright Test Script

Create `scripts/playwright-test.js`:

```javascript
const { test, expect } = require('@playwright/test');

test.describe('Kip UI Tests', () => {
    
    test('app loads successfully', async ({ page }) => {
        await page.goto('http://localhost:8080');
        
        // Wait for app to render
        await page.waitForSelector('#root');
        
        // Take screenshot for visual record
        await page.screenshot({ path: 'test-results/app-load.png' });
        
        expect(await page.title()).toContain('Kip');
    });

    test('toolbar is visible', async ({ page }) => {
        await page.goto('http://localhost:8080');
        await page.waitForSelector('.header');
        
        const header = await page.$('.header');
        expect(header).toBeTruthy();
        
        await header.screenshot({ path: 'test-results/toolbar.png' });
    });

    test('nodes render in workspace', async ({ page }) => {
        await page.goto('http://localhost:8080');
        await page.waitForSelector('.workspace');
        
        // Wait for nodes to load (check for node elements)
        await page.waitForSelector('.ws-node, .graph-node', { timeout: 10000 });
        
        const nodes = await page.$$('.ws-node, .graph-node');
        expect(nodes.length).toBeGreaterThan(0);
        
        await page.screenshot({ path: 'test-results/nodes.png' });
    });

    test('file picker can be opened', async ({ page }) => {
        await page.goto('http://localhost:8080');
        
        // Click the + button
        await page.click('.btn-add');
        
        // Wait for file picker to appear
        await page.waitForSelector('.file-picker, [class*="picker"]', { timeout: 5000 });
        
        await page.screenshot({ path: 'test-results/file-picker.png' });
    });

    test('no console errors', async ({ page }) => {
        const errors = [];
        
        page.on('console', msg => {
            if (msg.type() === 'error') {
                errors.push(msg.text());
            }
        });
        
        page.on('pageerror', error => {
            errors.push(error.message);
        });
        
        await page.goto('http://localhost:8080');
        await page.waitForTimeout(5000);
        
        expect(errors).toEqual([]);
    });
});
```

### Create Playwright Config

Create `playwright.config.js`:

```javascript
module.exports = {
    testDir: './scripts',
    testMatch: 'playwright-test.js',
    
    use: {
        baseURL: 'http://localhost:8080',
        browserName: 'chromium',
        headless: true,
        viewport: { width: 1920, height: 1080 },
        screenshot: 'only-on-failure',
        video: 'retain-on-failure',
    },

    reporter: [
        ['html', { outputFolder: 'test-results/playwright-report' }],
        ['list'],
    ],

    timeout: 30000,
};
```

### Run Playwright Tests

```bash
# Install Playwright browsers
npx playwright install chromium

# Start web server (in background)
dx serve --platform web --host 127.0.0.1 --port 8080 &

# Run tests
npx playwright test

# View HTML report
npx playwright show-report test-results/playwright-report
```

---

## Integrated Visual Test Command

Create `scripts/test-all.sh`:

```bash
#!/usr/bin/env bash
set -e

echo "=== Kip Complete Test Suite ==="
echo ""

# 1. Code checks
echo "=== Phase 1: Code Checks ==="
dx check
cargo check
echo "✅ Code checks passed"
echo ""

# 2. Unit tests
echo "=== Phase 2: Unit Tests ==="
cargo test --test api_tests -- --test-threads=1
echo "✅ Unit tests passed"
echo ""

# 3. Integration tests
echo "=== Phase 3: Integration Tests ==="
cargo test --test integration_tests -- --test-threads=1
echo "✅ Integration tests passed"
echo ""

# 4. Visual tests
echo "=== Phase 4: Visual Tests ==="
./scripts/visual-test.sh
echo "✅ Visual tests passed"
echo ""

# 5. Playwright tests
echo "=== Phase 5: Playwright Tests ==="
npx playwright test
echo "✅ Playwright tests passed"
echo ""

echo "=== ALL TESTS PASSED ==="
```

Usage:
```bash
chmod +x scripts/test-all.sh
./scripts/test-all.sh
```

---

## CI/CD Integration

Update `.github/workflows/test.yml`:

```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-action@stable
      
    - name: Install Dioxus CLI
      run: cargo install --git https://github.com/DioxusLabs/dioxus dioxus-cli
      
    - name: Install Xvfb
      run: sudo apt-get install xvfb imagemagick
      
    - name: Install Playwright
      run: |
        npm install -D @playwright/test
        npx playwright install chromium
      
    - name: Run all tests
      run: |
        Xvfb :99 -screen 0 1920x1080x24 &
        export DISPLAY=:99
        sleep 2
        ./scripts/test-all.sh
      
    - name: Upload test results
      uses: actions/upload-artifact@v4
      if: always()
      with:
        name: test-results
        path: test-results/
      
    - name: Upload screenshots
      uses: actions/upload-artifact@v4
      if: failure()
      with:
        name: screenshots
        path: /tmp/kip-screenshots/
```

---

## OpenClaw Integration

Update `openclaw-nixos.yaml`:

```yaml
# Add to tasks section
tasks:
  # ... existing tasks ...
  
  - id: visual_regression
    name: "Visual Regression Testing"
    description: |
      Run automated visual tests:
      - Screenshot capture
      - Baseline comparison
      - Playwright UI tests
    assigned_to: tester
    dependencies: [test_interactions]
    files:
      - scripts/visual-test.sh
      - scripts/playwright-test.js
    test_command: "./scripts/visual-test.sh && npx playwright test"
    acceptance_criteria:
      - "All screenshots match baselines"
      - "No Playwright test failures"
      - "No console errors"

# Add to coordination section
coordination:
  ci_checks:
    - "./scripts/run-with-xvfb.sh dx check"
    - "cargo test --test api_tests -- --test-threads=1"
    - "./scripts/run-with-xvfb.sh cargo test --test integration_tests -- --test-threads=1"
    - "./scripts/visual-test.sh"
    - "npx playwright test"
```

---

## Quick Start

```bash
# 1. Set up environment
nix-shell

# 2. Install Playwright
npm install -D @playwright/test
npx playwright install chromium

# 3. Create baselines (first run only)
./scripts/visual-test.sh

# 4. Run all tests
./scripts/test-all.sh

# 5. Run just visual tests
./scripts/visual-test.sh

# 6. Run just Playwright tests
npx playwright test
```

---

## Troubleshooting

### Screenshots differ slightly (false positives)

Increase tolerance in `scripts/visual-test.sh`:
```bash
TOLERANCE=10  # Was 2
```

### Playwright can't find elements

Increase wait timeouts in `playwright-test.js`:
```javascript
await page.waitForSelector('.ws-node', { timeout: 30000 });
```

### Tests fail in CI but pass locally

Ensure consistent environment:
- Same screen resolution (1920x1080)
- Same browser version
- Same fonts installed

Add to CI:
```yaml
- name: Install fonts
  run: sudo apt-get install fonts-liberation
```

---

## What This Gives You

**Fully Automated:**
- ✅ No manual screenshot taking
- ✅ No manual comparison
- ✅ No manual UI interaction
- ✅ Runs in CI/CD automatically
- ✅ Catches visual regressions

**Reports:**
- 📊 Screenshot diffs on failure
- 📊 Playwright HTML reports
- 📊 Console error detection
- 📊 Video recordings of failures

**Integration:**
- 🔗 OpenClaw can run visual tests
- 🔗 GitHub Actions runs on every PR
- 🔗 Baselines update automatically (when intended)

