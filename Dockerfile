# Kip development / evaluation container.
#
# Builds the full workspace — including the Dioxus desktop frontend — and
# pre-fetches every crate so the container needs no network at run time.
#
#   docker build -t kip-dev .
#   docker run --rm --network none kip-dev cargo test --workspace
#
# The image keeps the source tree and a warm target/ directory, so an
# incremental `cargo test` inside the container is fast and fully offline.

# Pinned so every run of the image sees an identical toolchain. The workspace
# uses no nightly features; stable is sufficient.
FROM rust:1.97.1-bookworm

# ---------------------------------------------------------------------------
# System dependencies
# ---------------------------------------------------------------------------
# Three groups:
#   * build toolchain  — C/C++ deps such as aws-lc-sys (reached via rustls)
#     which needs cmake and perl to build BoringSSL.
#   * GTK/WebKit stack — required to compile the Dioxus desktop frontend
#     (wry/tao) on Linux.
#   * runtime tools    — the binaries kip shells out to, so the transfer code
#     paths and their tests can actually run.
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        cmake \
        perl \
        pkg-config \
        libssl-dev \
        libglib2.0-dev \
        libgtk-3-dev \
        libwebkit2gtk-4.1-dev \
        libjavascriptcoregtk-4.1-dev \
        libsoup-3.0-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        libxdo-dev \
        rsync \
        rclone \
        openssh-client \
        ca-certificates \
        git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /kip

# ---------------------------------------------------------------------------
# Dependency fetch
# ---------------------------------------------------------------------------
# Copy only the manifests first so this layer is cached until a manifest or the
# lockfile actually changes. `--locked` makes the build fail loudly if
# Cargo.lock is stale rather than silently resolving different versions.
COPY Cargo.toml Cargo.lock ./
COPY .cargo/ .cargo/
COPY cli/Cargo.toml cli/
COPY daemon/Cargo.toml daemon/
COPY frontend/Cargo.toml frontend/
COPY kip-core/Cargo.toml kip-core/
COPY crates/kip-rsync/Cargo.toml crates/kip-rsync/
COPY crates/kip-rclone/Cargo.toml crates/kip-rclone/

# cargo fetch needs the target files named by each manifest to exist, even
# though their contents are irrelevant at this stage.
RUN mkdir -p cli/src daemon/src frontend/src kip-core/src \
             crates/kip-rsync/src crates/kip-rclone/src \
    && echo 'fn main() {}' > cli/src/main.rs \
    && touch daemon/src/lib.rs frontend/src/lib.rs kip-core/src/lib.rs \
             crates/kip-rsync/src/lib.rs crates/kip-rclone/src/lib.rs \
    && echo 'fn main() {}' > frontend/src/main.rs \
    && cargo fetch --locked

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
COPY . .

# Drop the placeholder sources the fetch step created so their mtimes don't
# shadow the real files copied above.
RUN touch cli/src/main.rs

# --offline proves the fetch above was complete: if anything is still missing,
# this fails at image build time rather than at run time inside a sandbox with
# no network. Building the tests too means `cargo test` in the container has
# nothing left to compile.
RUN cargo build --workspace --tests --locked --offline

# Fail fast on any accidental network use at run time.
ENV CARGO_NET_OFFLINE=true

# Keep kip's config out of the invoking user's real home and inside the
# container, so tests and manual runs never read or write a host config.
ENV KIP_CONFIG_DIR=/kip/.container-config
RUN mkdir -p /kip/.container-config

CMD ["cargo", "test", "--workspace", "--offline"]
