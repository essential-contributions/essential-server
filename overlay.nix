# An overlay to make it easier to merge all essential-server related packages
# into nixpkgs.
{ nixos-generators }: final: prev: {
  essential-server = prev.callPackage ./pkgs/essential-server.nix { };
  essential-rest-server = prev.callPackage ./pkgs/essential-rest-server.nix { };
  server-with-rqlite = final.callPackage ./pkgs/server-with-rqlite.nix { };
  rqlite-node = final.callPackage ./pkgs/rqlite-node.nix { nixos-generators = nixos-generators; };
}
