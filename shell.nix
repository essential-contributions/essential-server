# A dev shell providing the essentials for working on essential-server.
{ cargo-toml-lint
, clippy
, essential-server
, essential-rest-server
, mkShell
, rqlite
, rust-analyzer
, rustfmt
, curl
, cargo
, rustc
}:
mkShell {
  inputsFrom = [
    essential-server
    essential-rest-server
  ];
  buildInputs = [
    curl
    cargo-toml-lint
    clippy
    rqlite
    rust-analyzer
    rustfmt
    cargo
    rustc
  ];
}
