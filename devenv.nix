{
  config,
  lib,
  pkgs,
  ...
}:
let
  pname = "cocoa";
  buildInputs = with pkgs; [
    libressl
    libgit2
  ];
  nativeBuildInputs = with pkgs; [ autoPatchelfHook ];
  libraryPath = lib.makeLibraryPath buildInputs;
  toolchain = config.languages.rust.toolchainPackage;
in
{
  # needed for dynamic linking at runtime
  env.RUSTFLAGS = lib.mkForce "-C link-args=-Wl,-fuse-ld=mold,-rpath,${libraryPath}";

  git-hooks.hooks.clippy = {
    enable = true;
    packageOverrides = {
      cargo = toolchain;
      clippy = toolchain;
    };
  };

  languages.rust = {
    enable = true;
    channel = "nightly";
    mold.enable = true;
  };

  packages =
    with pkgs;
    [
      bacon
      cargo-outdated
      cargo-tarpaulin
    ]
    ++ buildInputs
    ++ nativeBuildInputs;

  scripts.tarp.exec = ''cargo tarpaulin --engine llvm "$@"'';

  outputs.default =
    let
      args = {
        crateOverrides = pkgs.defaultCrateOverrides // {
          ${pname} = attrs: {
            inherit buildInputs nativeBuildInputs;
            runtimeDependencies = buildInputs;
          };
        };
      };
    in
    config.languages.rust.import ./. args;
}
