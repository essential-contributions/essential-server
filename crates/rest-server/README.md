# Essential Rest Server
[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![license][apache-badge]][apache-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/essential-rest-server.svg
[crates-url]: https://crates.io/crates/essential-rest-server
[docs-badge]: https://docs.rs/essential-rest-server/badge.svg
[docs-url]: https://docs.rs/essential-rest-server
[apache-badge]: https://img.shields.io/badge/license-APACHE-blue.svg
[apache-url]: LICENSE
[actions-badge]: https://github.com/essential-contributions/essential-server/workflows/ci/badge.svg
[actions-url]:https://github.com/essential-contributions/essential-server/actions

A lightweight HTTP REST server designed to facilitate interaction with the Essential protocol. This server acts as an interface between clients and the Essential declarative protocol, allowing for easy integration and communication from the Essential ecosystem.
## Running the server
### Nix
### Memory DB
```bash
nix run .#essential-rest-server
```
### Rqlite
With a rqlite sever already running:
```bash
nix run .#essential-rest-server -- --db rqlite -r ${server_address}
```
Run a rqlite sever and the essential-rest-server:
```bash
nix run .#server-with-rqlite -- /path/to/rqlite/data/dir
```
### Cargo
```bash
cargo run -p essential-rest-server --release -- --help
```
## API
> Note that this API is very likely to change as it's currently a WIP.
### POST `/deploy-contract`
Body: `SignedPredicates` as JSON \
Returns: `ContentAddress` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X POST -H "Content-Type: application/json" \
    -d '{"contract":{"predicates":[{"state_read":[],"constraints":[],"directive":"Satisfy"}],"salt":"0000000000000000000000000000000000000000000000000000000000000000"},"signature":"D7B64C906BD6CA28DB9F02F21A295A96E134C13DB31F86E6A8A9BA5680A073D61ED8039FA47C26F24D5ED08808854332723BA274D9E0BDE5276D79DE82C25C9901"}' \
    http://localhost:59498/deploy-contract
```
### GET `/get-contract/:address`
Parameters: 
- `:address` = `[u8; 32]` as hex string. This is the content address of the contract.

Returns: `Option<SignedContract>` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X GET -H "Content-Type: application/json" http://localhost:59498/get-contract/EE3F28F3E0396EEE29613AF73E65D2BA52AE606E5FFD14D5EBD02A0FB5B88236
```
### GET `/get-predicate/:contract/:address`
Parameters: 
- `:contract` = `[u8; 32]` as hex string. This is the content address of the contract.
- `:address` = `[u8; 32]` as hex string. This is the content address of the predicate.

Returns: `Option<Predicate>` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X GET -H "Content-Type: application/json" http://localhost:59498/get-predicate/EE3F28F3E0396EEE29613AF73E65D2BA52AE606E5FFD14D5EBD02A0FB5B88236/709E80C88487A2411E1EE4DFB9F22A861492D20C4765150C0C794ABD70F8147C
```

### GET `/list-contracts`
Query parameters: 
- *Optional* `{ start: u64, end: u64 }`. This is the time range to list contract within. It is inclusive of the start and exclusive of the end.
- *Optional* `{ page: u64 }`. This is the page number to list contracts from. The default is 0.

Returns: `Vec<Contract>` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X GET -H "Content-Type: application/json" "http://localhost:59498/list-contracts?start=0&end=1&page=0"
```

### GET `/subscribe-contracts`
This api is a server sent event api.\
This allows you to subscribe to new contracts as they are deployed.
Query parameters: 
- *Optional* `{ start: u64 }`. This is the time to start returning contracts from.
- *Optional* `{ page: u64 }`. This is the page number to return contracts from. The default is 0.

Returns: `Stream<Item = Result<Contract>>` where the result and contract are json.

**Example:**
```bash
curl --http2-prior-knowledge -N -X GET -H "Content-Type: application/json" "http://localhost:59498/subscribe-blocks?start=0&end=1&page=0&block=0"
```

### POST `/submit-solution`
Body: `Solution` as JSON \
Returns: `Hash` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X POST -H "Content-Type: application/json" -d '{"data":[{"predicate_to_solve":{"contract":"EE3F28F3E0396EEE29613AF73E65D2BA52AE606E5FFD14D5EBD02A0FB5B88236","predicate":"709E80C88487A2411E1EE4DFB9F22A861492D20C4765150C0C794ABD70F8147C"},"decision_variables":[],"transient_data":[],"state_mutations":[]}]}' http://localhost:59498/submit-solution
```
### GET `/list-solutions-pool`
Query parameters: 
- *Optional* `{ page: u64 }`. This is the page number to list contracts from. The default is 0.

Returns: `Vec<Solution>` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X GET -H "Content-Type: application/json" "http://localhost:59498/list-solutions-pool" 
```
### GET `/query-state/:address/:key`
Parameters: 
- `:address` = `[u8; 32]` as hex string. This is the content address of the contract.
- `:key` = `Vec<u8>` as hex string. This is the key of the state.

Returns: `Option<Word>` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X GET -H "Content-Type: application/json" http://localhost:59498/query-state/EE3F28F3E0396EEE29613AF73E65D2BA52AE606E5FFD14D5EBD02A0FB5B88236/00
```

### GET `/list-blocks`
Query parameters: 
- *Optional* `{ start: u64, end: u64 }`. This is the time range to list blocks within. It is inclusive of the start and exclusive of the end.
- *Optional* `{ page: u64 }`. This is the page number to list blocks from. The default is 0.
- *Optional* `{ block: u64 }`. This is the block number to list blocks from.

Returns: `Vec<Block>` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X GET -H "Content-Type: application/json" "http://localhost:59498/list-blocks?start=0&end=1&page=0&block=0"
```

### GET `/subscribe-blocks`
This api is a server sent event api.\
This allows you to subscribe to new blocks as they are added to the chain.
Query parameters: 
- *Optional* `{ start: u64 }`. This is the time to start returning blocks from.
- *Optional* `{ page: u64 }`. This is the page number to return blocks from. The default is 0.
- *Optional* `{ block: u64 }`. This is the block number to return blocks from.

Returns: `Stream<Item = Result<Block>>` where the result and block are json.

**Example:**
```bash
curl --http2-prior-knowledge -N -X GET -H "Content-Type: application/json" "http://localhost:59498/subscribe-blocks?start=0&end=1&page=0&block=0"
```

### GET `/solution-outcome/:hash`
Parameters: 
- `:hash` = `[u8; 32]` as hex string. This is the hash of the solution.

Returns: `Vec<SolutionOutcome>` as JSON
```rust
pub enum SolutionOutcome {
    Success(u64),
    Fail(String),
}
```

**Example:**
```bash
curl --http2-prior-knowledge -X GET -H "Content-Type: application/json" "http://localhost:59498/solution-outcome/11CAD716457F6D6524EF84FBA73D11BB5E18658F6EE72EBAC8A14323B37A68FC
```
### Post `/check-solution`
Check a solution against deployed contract without changing state.\
This is a dry run of the solution.\
Body: `Solution` as JSON \
Returns: `CheckSolutionOutput` as JSON
```rust
pub struct CheckSolutionOutput {
    pub utility: f64,
    pub gas: u64,
}
```

**Example:**
```bash
curl --http2-prior-knowledge -X POST -H "Content-Type: application/json" -d '{"data":[{"predicate_to_solve":{"contract":"EE3F28F3E0396EEE29613AF73E65D2BA52AE606E5FFD14D5EBD02A0FB5B88236","predicate":"709E80C88487A2411E1EE4DFB9F22A861492D20C4765150C0C794ABD70F8147C"},"decision_variables":[],"transient_data":[],"state_mutations":[]}]}' http://localhost:59498/check-solution
```
### Post `/check-solution-with-contracts`
Check a solution with all contract without changing state.\
This is a dry run of the solution.\
Body: `CheckSolution` as JSON \
```rust
struct CheckSolution {
    solution: Solution,
    contract: Contract,
}
```
Returns: `CheckSolutionOutput` as JSON
```rust
pub struct CheckSolutionOutput {
    pub utility: f64,
    pub gas: u64,
}
```

**Example:**
```bash
curl --http2-prior-knowledge -X POST -H "Content-Type: application/json" -d '{"solution":{"data":[{"predicate_to_solve":{"contract":"EE3F28F3E0396EEE29613AF73E65D2BA52AE606E5FFD14D5EBD02A0FB5B88236","predicate":"709E80C88487A2411E1EE4DFB9F22A861492D20C4765150C0C794ABD70F8147C"},"decision_variables":[],"transient_data":[],"state_mutations":[]}]},"contracts":[{"predicates":[{"state_read":[],"constraints":[],"directive":"Satisfy"}],"salt":"0000000000000000000000000000000000000000000000000000000000000000"}]}' http://localhost:59498/check-solution-with-contracts
```

### Post `/query-state-reads`
Run a query on state using state read programs,\
This allows you to use the state read parts of your pint program to query state.\
This is also useful for getting the pre state for a solution when debugging.\
Body: `QueryStateReads` as JSON \
```rust
pub struct QueryStateReads {
    pub state_read: Vec<StateReadBytecode>,
    pub index: SolutionDataIndex,
    pub solution: Solution,
    pub request_type: StateReadRequestType,
}
```
Returns: `QueryStateReadsOutput` as JSON
```rust
pub enum QueryStateReadsOutput {
    Reads(BTreeMap<ContentAddress, Vec<(Key, Value)>>),
    Slots(Slots),
    All(BTreeMap<ContentAddress, Vec<(Key, Value)>>, Slots),
    Failure(String),
}
```
These types are defined in the `essential-server-types` crate in this repo.\
**Example:**
```bash
curl --http2-prior-knowledge -X POST -H "Content-Type: application/json" -d '{"state_read":[],"index":0,"solution":{"data":[{"predicate_to_solve":{"contract":"EE3F28F3E0396EEE29613AF73E65D2BA52AE606E5FFD14D5EBD02A0FB5B88236","predicate":"709E80C88487A2411E1EE4DFB9F22A861492D20C4765150C0C794ABD70F8147C"},"decision_variables":[],"transient_data":[],"state_mutations":[]}]},"request_type":{"All":"All"}}' http://localhost:59498/query-state-reads
```
