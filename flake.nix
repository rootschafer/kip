{
  description = "Rust project flake with Cargo & OpenSSL working";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };

        minimalLlvmPkgs = with pkgs.llvmPackages; [
          libcxxStdenv
          clangWithLibcAndBasicRtAndLibcxx
          lld
          compiler-rt
          libcxx
          libunwind
          openmp
          llvm
          mlir
          clang-tools
        ];

        fullLlvmPkgs =
          minimalLlvmPkgs
          ++ (with pkgs; [
            lldb
            libclc
            libclang.lib
          ])
          ++ (with pkgs.llvmPackages; [
            bolt
            lldbPlugins.llef
          ]);

        gtkPkgs = with pkgs; [
          glib
          gst_all_1.gstreamer
          gst_all_1.gst-devtools
          gst_all_1.gst-plugins-good
          gst_all_1.gst-plugins-bad
        ];

        basePackages =
          with pkgs;
          [
            # rustc
            # cargo
            # rustfmt
            # clippy
            openssl
            openssl.dev
            pkg-config
            # clang
            libclang.lib
            git
            gitRepo
            gnupg
            curl
            procps
            gnumake
            util-linux
            m4
            gperf
            unzip
            gdb
            nodejs
            yarn
            curl
            bash
            # findutils
            ncurses
            stdenv.cc
            binutils
          ]
          ++ fullLlvmPkgs
          ++ gtkPkgs;
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = basePackages;

          shellHook = ''
            export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
            export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"
          '';
        };
      }
    );
}
