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
      self,
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

        manifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);

        rustPlatform = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };
      in
      {
        packages.default = rustPlatform.buildRustPackage {
          pname = "nuke";
          version = manifest.workspace.package.version;
          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "--package"
            "nuke-cli"
          ];

          doCheck = false;

          nativeBuildInputs = [ pkgs.installShellFiles ];

          postInstall = ''
            installManPage $releaseDir/build/nuke-cli-*/out/*.1
            installShellCompletion --cmd nuke \
              --bash $releaseDir/build/nuke-cli-*/out/nuke.bash \
              --fish $releaseDir/build/nuke-cli-*/out/nuke.fish \
              --zsh $releaseDir/build/nuke-cli-*/out/_nuke \
              --nushell $releaseDir/build/nuke-cli-*/out/nuke.nu
          '';

          meta = {
            description = "Render, inspect and format Nuke documents";
            mainProgram = "nuke";
            platforms = pkgs.lib.platforms.unix;
          };
        };

        packages.tree-sitter-nuke = pkgs.tree-sitter.buildGrammar {
          language = "nuke";
          version = manifest.workspace.package.version;
          src = ./tree-sitter-nuke;

          meta = {
            description = "Nuke grammar for tree-sitter";
            platforms = pkgs.lib.platforms.unix;
          };
        };

        devShells.default = pkgs.mkShell {
          packages = [
            toolchain
            pkgs.cargo-nextest
            pkgs.fd
            pkgs.nodejs
            pkgs.ripgrep
            pkgs.tree-sitter
          ];
        };

        formatter = pkgs.nixfmt-tree;
      }
    )
    // {
      homeModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.programs.nuke;
          packages = self.packages.${pkgs.stdenv.hostPlatform.system};
        in
        {
          options.programs.nuke = {
            enable = lib.mkEnableOption "the Nuke command line tool";

            package = lib.mkOption {
              type = lib.types.package;
              default = packages.default;
              defaultText = lib.literalMD "the flake's `packages.default`";
              description = "The package providing the `nuke` binary and its language server.";
            };

            helix.enable = lib.mkEnableOption "Nuke's grammar and queries for Helix";
          };

          config = lib.mkMerge [
            (lib.mkIf cfg.enable { home.packages = [ cfg.package ]; })

            (lib.mkIf cfg.helix.enable {
              programs.nuke.enable = lib.mkDefault true;

              xdg.configFile."helix/runtime/grammars/nuke.so".source = "${packages.tree-sitter-nuke}/parser";
              xdg.configFile."helix/runtime/queries/nuke".source = "${packages.tree-sitter-nuke}/queries";
            })
          ];
        };
    };
}
