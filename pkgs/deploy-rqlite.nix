{ pkgs
, system
, rqlite-node
, server-node
, etcd-node
}:
let
  rqlite-image-name = "rqlite-${system}";
  rqlite-img-path = "${rqlite-node}/${rqlite-image-name}.vhd";
  essential-image-name = "essential-${system}";
  essential-img-path = "${server-node}/${essential-image-name}.vhd";
  etcd-image-name = "etcd-${system}";
  etcd-img-path = "${etcd-node}/${etcd-image-name}.vhd";
in
pkgs.writeShellScriptBin "tofu" ''
  export TF_VAR_rqlite_img_path="${rqlite-img-path}"
  export TF_VAR_essential_img_path="${essential-img-path}"
  export TF_VAR_etcd_img_path="${etcd-img-path}"
  ${pkgs.opentofu}/bin/tofu $@
''
