{
  config,
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
in
{
  languages.rust = {
    enable = true;
    channel = "nightly";
    wild.enable = true;
  };

  packages = buildInputs ++ nativeBuildInputs;

  outputs.default = config.languages.rust.crane.import ./. {
    inherit buildInputs nativeBuildInputs;
    runtimeDependencies = buildInputs;
    extraPaths = [ ./locales ];
    extraFileTypes = [ "jinja" ];
  };
}
