{
  description = "A Rust project";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    git-hooks-nix = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-flake = {
      url = "github:juspay/rust-flake";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
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
        inputs.git-hooks-nix.flakeModule
        inputs.rust-flake.flakeModules.default
        inputs.rust-flake.flakeModules.nixpkgs
        inputs.treefmt-nix.flakeModule
      ];

      perSystem =
        {
          self',
          config,
          pkgs,
          system,
          ...
        }:
        let
          pname = "cocoa";
        in
        {
          # git hooks
          pre-commit.settings.hooks = {
            # commit linting
            commitlint-rs =
              let
                config = pkgs.writers.writeYAML "commitlintrc.yml" {
                  rules = {
                    description-empty.level = "error";
                    description-format = {
                      level = "error";
                      format = "^[a-z].*$";
                    };
                    description-max-length = {
                      level = "error";
                      length = 72;
                    };
                    scope-max-length = {
                      level = "warning";
                      length = 10;
                    };
                    scope-empty.level = "warning";
                    type = {
                      level = "error";
                      options = [
                        "build"
                        "chore"
                        "ci"
                        "docs"
                        "feat"
                        "fix"
                        "perf"
                        "refactor"
                        "style"
                        "test"
                      ];
                    };
                  };
                };

              in
              {
                enable = true;
                name = "commitlint-rs";
                package = pkgs.commitlint-rs;
                description = "Validate commit messages with commitlint-rs";
                entry = "${pkgs.lib.getExe pkgs.commitlint-rs} -g ${config} -e";
                always_run = true;
                stages = [ "commit-msg" ];
              };

            # format on commit
            treefmt.enable = true;
          };

          # formatting
          treefmt.programs = {
            nixfmt.enable = true;
            rustfmt.enable = true;
            taplo.enable = true;
          };

          # rust build settings
          rust-project = {
            # use fenix toolchain for nightly rust
            toolchain = inputs.fenix.packages.${system}.complete.withComponents [
              "cargo"
              "clippy"
              "rust-analyzer"
              "rust-docs"
              "rust-src"
              "rust-std"
              "rustc"
              "rustfmt"
            ];

            # setup build inputs for crane
            crates.${pname}.crane.args = {
              buildInputs = [ ];
              nativeBuildInputs = [ ];
            };
          };

          # package definitions
          packages.default = self'.packages.${pname};

          # development environment
          devShells.default = pkgs.mkShell {
            inputsFrom = [
              self'.devShells.rust
              config.pre-commit.devShell
            ];
            packages = with pkgs; [
              bacon
              cargo-outdated
            ];
          };
        };
    };
}
