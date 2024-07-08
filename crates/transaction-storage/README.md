# Essential Transaction Storage
[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![license][apache-badge]][apache-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/essential-transaction-storage.svg
[crates-url]: https://crates.io/crates/essential-transaction-storage
[docs-badge]: https://docs.rs/essential-transaction-storage/badge.svg
[docs-url]: https://docs.rs/essential-transaction-storage
[apache-badge]: https://img.shields.io/badge/license-APACHE-blue.svg
[apache-url]: LICENSE
[actions-badge]: https://github.com/essential-contributions/essential-server/workflows/ci/badge.svg
[actions-url]: https://github.com/essential-contributions/essential-server/actions

A transactional layer that wraps any Essential storage implementation, providing transactions that can span across await boundaries. This crate allows blocks to be built up and validated before being committed atomically to the underlying storage system.