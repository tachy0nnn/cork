{
  description = "Rust project";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    self.submodules = true;

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane.url = "github:ipetkov/crane";

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      crane,
      treefmt-nix,
      git-hooks,
      ...
    }:
    let
      inherit (nixpkgs) lib;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      outputsFor =
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };

          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          craneLib = (crane.mkLib pkgs).overrideToolchain (_: rustToolchain);
          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.cmake
          ];
          buildInputs = [
            pkgs.openssl
            pkgs.glib
            pkgs.gdk-pixbuf
            pkgs.pango
            pkgs.graphene
            pkgs.gtk4
          ];

          src = lib.cleanSource ./.;
          commonArgs = {
            inherit src nativeBuildInputs buildInputs;
            strictDeps = true;
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          crate = craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });

          treefmtEval = treefmt-nix.lib.evalModule pkgs ./treefmt.nix;

          pre-commit = git-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              treefmt = {
                enable = true;
                package = treefmtEval.config.build.wrapper;
              };
              cargo-fmt = {
                enable = true;
                name = "cargo fmt";
                entry = "cargo fmt --all --check";
                files = "\\.rs$";
                language = "system";
                pass_filenames = false;
              };
              cargo-clippy = {
                enable = true;
                name = "cargo clippy";
                entry = "cargo clippy --all-targets -- --deny warnings";
                files = "\\.rs$";
                language = "system";
                pass_filenames = false;
              };
            };
          };

          useMold = system == "x86_64-linux";
        in
        {
          packages.default = crate;

          checks = {
            inherit crate;
            clippy = craneLib.cargoClippy (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );
            fmt = craneLib.cargoFmt { inherit src; };
            nextest = craneLib.cargoNextest (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoNextestExtraArgs = "--no-tests=pass";
              }
            );
            formatting = treefmtEval.config.build.check self;
          };

          formatter = treefmtEval.config.build.wrapper;

          devShells.default = pkgs.mkShell {
            inherit nativeBuildInputs buildInputs;
            inherit (pre-commit) shellHook;

            packages = [
              rustToolchain
              treefmtEval.config.build.wrapper
              pkgs.cargo-nextest
              pkgs.cargo-deny
              pkgs.cargo-audit
              pkgs.bacon
              pkgs.binutils
              pkgs.elfutils
              pkgs.cargo-binutils
              pkgs.gdb
              pkgs.strace
              pkgs.file
            ]
            ++ lib.optionals useMold [ pkgs.mold ];

            env = {
              RUST_BACKTRACE = "1";
              # runtime libraries for gui/graphics crates, uncomment and extend if needed:
              # LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.wayland pkgs.libxkbcommon pkgs.vulkan-loader ];
            }
            // lib.optionalAttrs useMold {
              CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
            };
          };
        };

      perSystem = lib.genAttrs systems outputsFor;
    in
    {
      packages = lib.mapAttrs (_: o: o.packages) perSystem;
      checks = lib.mapAttrs (_: o: o.checks) perSystem;
      formatter = lib.mapAttrs (_: o: o.formatter) perSystem;
      devShells = lib.mapAttrs (_: o: o.devShells) perSystem;
    };
}
