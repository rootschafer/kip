{
  description = "Kip — development shell for building on NixOS / Linux";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        inherit (pkgs) lib stdenv;

        # Tools that run on the build machine.
        nativeBuildInputs = with pkgs; [
          rustc
          cargo
          rustfmt
          clippy
          pkg-config

          # aws-lc-sys (pulled in via rustls) builds BoringSSL, which needs both.
          cmake
          perl
        ];

        # openssl-sys is in Cargo.lock, so this is genuinely required.
        commonBuildInputs = [ pkgs.openssl ];

        # The Dioxus desktop frontend uses wry/tao, which on Linux means GTK and
        # WebKit. None of this is needed for the CLI or daemon alone — but
        # `cargo build --workspace` includes `frontend`, so the shell needs it.
        guiBuildInputs = with pkgs; [
          glib
          gtk3
          webkitgtk_4_1
          libsoup_3
          cairo
          pango
          gdk-pixbuf
          atk
          libayatana-appindicator
          librsvg
          xdotool # provides libxdo, used by tao
        ];

        # Binaries kip shells out to. Without these the transfer code paths and
        # their tests can compile but not run.
        runtimeTools = with pkgs; [
          rsync
          rclone
          openssh
        ];

        buildInputs =
          commonBuildInputs
          ++ runtimeTools
          ++ lib.optionals stdenv.isLinux guiBuildInputs;
      in
      {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          shellHook = ''
            ${lib.optionalString stdenv.isLinux ''
              # WebKit dlopens parts of itself, so its libraries must be on the
              # runtime search path, not just the link path.
              export LD_LIBRARY_PATH="${lib.makeLibraryPath guiBuildInputs}:$LD_LIBRARY_PATH"
              # GTK resolves its settings schemas through XDG_DATA_DIRS.
              export XDG_DATA_DIRS="${pkgs.gtk3}/share:${pkgs.glib}/share:''${XDG_DATA_DIRS:-}"
            ''}

            echo "kip dev shell (rustc $(rustc --version | cut -d' ' -f2))"
            echo "  cargo build -p cli           # the kip CLI"
            echo "  cargo test --workspace       # full suite, no network needed"
            echo "  dx build --package frontend  # GUI — needs dioxus-cli, see flake.nix"
          '';
        };
      }
    );
}

# `dx` (dioxus-cli) is deliberately not included: nixpkgs' version regularly
# lags the 0.7.x this project pins, and a mismatched dx produces confusing
# asset-bundling failures. Install a matching one inside the shell with:
#
#     cargo install dioxus-cli --locked --version 0.7.10
#
# Only the `frontend` crate needs it; everything else is plain cargo.
