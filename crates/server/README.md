# Essential Server
[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![license][apache-badge]][apache-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/essential-server.svg
[crates-url]: https://crates.io/crates/essential-server
[docs-badge]: https://docs.rs/essential-server/badge.svg
[docs-url]: https://docs.rs/essential-server
[apache-badge]: https://img.shields.io/badge/license-APACHE-blue.svg
[apache-url]: LICENSE
[actions-badge]: https://github.com/essential-contributions/essential-server/workflows/ci/badge.svg
[actions-url]:https://github.com/essential-contributions/essential-server/actions

A centralized server implementation of the Essential declarative protocol. This crate is responsible for building blocks and managing the core functionality of the Essential application, serving as the backbone for the entire system.

## Block State Contract
The server uses a special contract to store state about the blocks. Currently this includes time and block number.
If you want to query this state you can do the following:
```pint
interface BlockState {
    storage {
        number: int,
        time: int,
    }
}

predicate MyPredicate {
    interface Block = BlockState(0xF745D1D91B68080CF398556DDD611B620964D2B1974C0283F03AC7C142B394DF);

    state time = Block::time;
    state number = Block::number;
}
```