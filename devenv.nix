{
  config,
  lib,
  pkgs,
  ...
}:
let
  buildInputs = with pkgs; [
    libressl
    libgit2
    libgcc
  ];
  nativeBuildInputs = with pkgs; [
    autoPatchelfHook
  ];
  libraryPath = lib.makeLibraryPath buildInputs;
in
{
  # needed for dynamic linking at runtime
  env.RUSTFLAGS = lib.mkForce "-C link-args=-Wl,-fuse-ld=mold,-rpath,${libraryPath}";

  languages.rust = {
    enable = true;
    channel = "nightly";
    mold.enable = true;
  };

  packages = buildInputs ++ nativeBuildInputs;

  outputs.default = config.languages.rust.crane.import ./. {
    inherit buildInputs nativeBuildInputs;
    runtimeDependencies = buildInputs;
  };
}
