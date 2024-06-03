# A dev shell providing the essentials for working on essential-server.
{ cargo-toml-lint
, clippy
, essential-server
, essential-rest-server
, mkShell
, rqlite
, rust-analyzer
, rustfmt
}:
mkShell {
  inputsFrom = [
    essential-server
    essential-rest-server
  ];
  buildInputs = [
    cargo-toml-lint
    clippy
    rust-analyzer
    rustfmt
  ];
}
