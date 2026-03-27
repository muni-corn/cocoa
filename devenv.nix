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
