# Essential Rqlite Storage
[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![license][apache-badge]][apache-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/essential-rqlite-storage.svg
[crates-url]: https://crates.io/crates/essential-rqlite-storage
[docs-badge]: https://docs.rs/essential-rqlite-storage/badge.svg
[docs-url]: https://docs.rs/essential-rqlite-storage
[apache-badge]: https://img.shields.io/badge/license-APACHE-blue.svg
[apache-url]: LICENSE
[actions-badge]: https://github.com/essential-contributions/essential-server/workflows/ci/badge.svg
[actions-url]:https://github.com/essential-contributions/essential-server/actions

An implementation of the Essential storage system backed by [rqlite](https://rqlite.io/), a distributed relational database. This crate provides a persistent, scalable storage solution for the Essential protocol, suitable for production environments requiring data durability and distribution.