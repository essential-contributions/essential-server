# A derivation for the `essential-rest-server` crate.
{ lib
, stdenv
, darwin
, openssl
, pkg-config
, rustPlatform
}:
let
  src = builtins.path {
    path = ../.;
    filter = path: type:
      let
        keepFiles = [
          "Cargo.lock"
          "Cargo.toml"
          "crates"
        ];
        includeDirs = [
          "crates"
        ];
        isPathInIncludeDirs = dir: lib.strings.hasInfix dir path;
      in
      if lib.lists.any (p: p == (baseNameOf path)) keepFiles then
        true
      else
        lib.lists.any (dir: isPathInIncludeDirs dir) includeDirs
    ;
  };
  crateDir = "${src}/crates/rest-server";
  crateTOML = "${crateDir}/Cargo.toml";
  lockFile = "${src}/Cargo.lock";
in
rustPlatform.buildRustPackage {
  inherit src;
  pname = "essential-rest-server";
  version = (builtins.fromTOML (builtins.readFile crateTOML)).package.version;

  buildAndTestSubdir = "crates/rest-server";

  nativeBuildInputs = lib.optionals stdenv.isLinux [
    pkg-config
  ];

  buildInputs = lib.optionals stdenv.isLinux [
    openssl
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  # We run tests separately in CI.
  doCheck = false;

  cargoLock = {
    inherit lockFile;
  };
}
