{
  config,
  pkgs,
  lib,
  ...
}:
let
  cocoa = config.lib.getInput {
    name = "cocoa";
    url = "github:muni-corn/cocoa";
    attribute = "git-hooks.hooks.cocoa-lint.enable";
    follows = [ "nixpkgs" ];
  };
in
{
  git-hooks = {
    hooks = {
      cocoa-generate = {
        name = "cocoa generate";
        package = config.git-hooks.tools.cocoa;
        description = "Generates commit messages with cocoa";
        entry = "${lib.getExe config.git-hooks.hooks.cocoa-generate.package} generate";
        stages = [ "prepare-commit-msg" ];
      };

      cocoa-lint = {
        name = "cocoa lint";
        package = config.git-hooks.tools.cocoa;
        description = "Validates commit messages with cocoa";
        entry = "${lib.getExe config.git-hooks.hooks.cocoa-lint.package} lint";
        stages = [ "commit-msg" ];
      };
    };

    tools.cocoa = cocoa.packages.${pkgs.stdenv.system}.default;
  };
}
