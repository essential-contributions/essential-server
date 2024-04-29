# A derivation for the `essential-rest-server` crate.
{ lib
, stdenv
, openssl
, pkg-config
, rustPlatform
, darwin
}:
let
  src = ../.;
  crateDir = "${src}/crates/server";
  crateTOML = "${crateDir}/Cargo.toml";
  lockFile = "${src}/Cargo.lock";
in
rustPlatform.buildRustPackage {
  inherit src;
  pname = "essential-rest-server";
  version = (builtins.fromTOML (builtins.readFile crateTOML)).package.version;

  buildInputs = [
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  cargoLock = {
    inherit lockFile;
    # FIXME: This enables using `builtins.fetchGit` which uses the user's local
    # `git` (and hence ssh-agent for ssh support). Once the repos are public,
    # this should be removed.
    allowBuiltinFetchGit = true;
  };
}
