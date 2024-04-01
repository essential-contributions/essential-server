# A dev shell providing the essentials for working on essential-server.
{ cargo-toml-lint
, clippy
, essential-server
, mkShell
, rust-analyzer
, rustfmt
, yurt
}:
mkShell {
  inputsFrom = [
    essential-server
  ];
  buildInputs = [
    cargo-toml-lint
    clippy
    rust-analyzer
    rustfmt
    yurt
  ];
}
