# Essential Server

A implementation of the Essential declarative protocol and constraint checking engine that builds blocks and runs as a centralized server.
This repository is designed for application developers to be able to experience building and running declarative applications while the decentralized node is being built.

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

## Overview

The Essential Server repository is a comprehensive implementation of the Essential declarative protocol as a centralized server. This project aims to provide a system for application developers to start building and experimenting with declarative applications. The decentralized node implementation with be built on top of the same foundation so will be a very similar experience.

Our server implementation offers various storage options, a REST API for interaction, and the core block building server that runs on top of [essential base](https://github.com/essential-contributions/essential-base).

## Crates

This repository is organized into several Rust crates.

1. **essential-server**: The core implementation of the Essential declarative protocol. It builds blocks and runs as a centralized server.

2. **essential-storage**: Defines the traits that abstract over various storage implementations used by the server. This allows for flexible and interchangeable storage backends.

3. **essential-memory-storage**: An in-memory implementation of the Essential storage system. Ideal for testing, development, or scenarios where persistence isn't required.

4. **essential-rqlite-storage**: A persistent storage implementation backed by rqlite, suitable for production environments requiring data durability and distribution.

5. **essential-transaction-storage**: A transactional layer that wraps any storage implementation, providing transactions that can span across await boundaries.

6. **essential-rest-server**: A lightweight HTTP REST server that facilitates interaction with the Essential application, allowing for easy integration and communication.

7. **essential-server-types**: A collection of common types and data structures used for communication between clients and the Essential REST server.

## Getting Started

See the [documentation](https://docs.essential.builders/).
