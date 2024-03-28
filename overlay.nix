# An overlay to make it easier to merge all essential-server related packages
# into nixpkgs.
{}: final: prev: {
  essential-server = prev.callPackage ./default.nix { };
}
