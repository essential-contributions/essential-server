# An overlay to make it easier to merge all essential-server related packages
# into nixpkgs.
{ nixos-generators }: final: prev: rec {
  essential-server = prev.callPackage ./pkgs/essential-server.nix { };
  essential-rest-server = prev.callPackage ./pkgs/essential-rest-server.nix { };
  server-with-rqlite = final.callPackage ./pkgs/server-with-rqlite.nix { };
  rqlite-node = final.callPackage ./pkgs/rqlite-node.nix { nixos-generators = nixos-generators; };
  # rqlite-node2 = final.callPackage ./pkgs/rqlite-node.nix { nixos-generators = nixos-generators; port-to-forward = 4003; raft-port = 4004; ssh-port-to-forward = 3023; disk-image-name = "test2"; };
  server-node = final.callPackage ./pkgs/server-node.nix { nixos-generators = nixos-generators; };
  etcd-node = final.callPackage ./pkgs/etcd-node.nix { nixos-generators = nixos-generators; };
  tofu = final.callPackage ./pkgs/deploy-rqlite.nix { rqlite-node = rqlite-node; server-node = server-node; etcd-node = etcd-node; };
}
