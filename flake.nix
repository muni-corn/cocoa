{
  description = "cocoa, the conventional commit assistant";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    devenv = {
      url = "github:cachix/devenv";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    musicaloft-style = {
      url = "git+https://git.musicaloft.com/municorn/musicaloft-style";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-flake = {
      url = "github:juspay/rust-flake";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    devenv-root = {
      url = ./.devenv/root;
      flake = false;
    };
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      imports = [
        inputs.devenv.flakeModule
        inputs.rust-flake.flakeModules.default
        inputs.rust-flake.flakeModules.nixpkgs

        # sets up code formatting and commit linting
        inputs.musicaloft-style.flakeModule
      ];

      perSystem =
        {
          config,
          pkgs,
          ...
        }:
        let
          pname = "cocoa";

          buildInputs = with pkgs; [
            libressl
            libgit2
          ];
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
        in
        {
          # rust setup
          devenv.shells.default = {
            languages.rust = {
              enable = true;
              channel = "nightly";
              mold.enable = true;
            };

            packages = [
              pkgs.bacon
              pkgs.cargo-outdated
            ]
            ++ buildInputs
            ++ nativeBuildInputs;
          };

          # setup rust packages
          rust-project = {
            # use the same rust toolchain from the dev shell for consistency
            toolchain = config.devenv.shells.default.languages.rust.toolchainPackage;

            # specify dependencies
            defaults.perCrate.crane.args = {
              inherit nativeBuildInputs buildInputs;
            };
          };

          packages.default = config.rust-project.crates.${pname}.crane.outputs.packages.${pname};
        };
    };
}
