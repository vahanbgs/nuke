{
  description = "Nuke, a simple total configuration language";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        toolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "clippy"
            "rust-analyzer"
            "rust-src"
            "rustfmt"
          ];
        };

        manifest = builtins.fromTOML (builtins.readFile ./crates/nuke-cli/Cargo.toml);

        rustPlatform = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };
      in
      {
        packages.default = rustPlatform.buildRustPackage {
          pname = "nuke";
          version = manifest.package.version;
          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "--package"
            "nuke-cli"
          ];

          doCheck = false;

          meta = {
            description = "Render, inspect and format Nuke documents";
            mainProgram = "nuke";
            platforms = pkgs.lib.platforms.unix;
          };
        };

        devShells.default = pkgs.mkShell {
          packages = [
            toolchain
            pkgs.cargo-nextest
            pkgs.fd
            pkgs.ripgrep
          ];
        };

        formatter = pkgs.nixfmt-tree;
      }
    );
}
