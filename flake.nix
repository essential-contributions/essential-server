{
  description = ''
    A nix flake for the essential server.
  '';

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
    flake-utils.url = "github:numtide/flake-utils";
    yurt = {
      url = "git+ssh://git@github.com/essential-contributions/yurt.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    let
      overlays = [
        inputs.self.overlays.default
        inputs.yurt.overlays.default
      ];
      perSystemPkgs = f:
        inputs.nixpkgs.lib.genAttrs (import inputs.systems)
          (system: f (import inputs.nixpkgs { inherit overlays system; }));
    in
    {
      overlays = {
        essential-server = import ./overlay.nix { };
        default = inputs.self.overlays.essential-server;
      };

      packages = perSystemPkgs (pkgs: {
        essential-server = pkgs.essential-server;
        essential-rest-server = pkgs.essential-rest-server;
        default = inputs.self.packages.${pkgs.system}.essential-server;
      });

      devShells = perSystemPkgs (pkgs: {
        essential-server-dev = pkgs.callPackage ./shell.nix { };
        default = inputs.self.devShells.${pkgs.system}.essential-server-dev;
      });

      apps = perSystemPkgs (pkgs: {
        server-with-rqlite = pkgs.callPackage ./apps.nix {
          flake-utils = inputs.flake-utils;
        };
        default = inputs.self.apps.${pkgs.system}.server-with-rqlite;
      });

      formatter = perSystemPkgs (pkgs: pkgs.nixpkgs-fmt);
    };
}
