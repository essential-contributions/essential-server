# Essential Rest Server
This runs a basic HTTP REST server that can be used to interact with the essential application.
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
### POST `/deploy-predicate-contract`
Body: `SignedPredicates` as JSON \
Returns: `ContentAddress` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X POST -H "Content-Type: application/json" \
    -d '{"contract":[{"state_read":[],"constraints":[],"directive":"Satisfy"}],"signature":"721BD7C79A0F303B7EDA3319CE84ADD4AB37BBED21E0570E6334D7864E3B27F121C74A4D8991CB5966BE13BD54544AA81EE26D98E76A3ED6C4BB237529C1188901"}' \
    http://localhost:59498/deploy-predicate-contract
```
### GET `/get-predicate-contract/:address`
Parameters: 
- `:address` = `[u8; 32]` as hex string. This is the content address of the contract.

Returns: `Option<SignedContract>` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X GET -H "Content-Type: application/json" http://localhost:59498/get-predicate-contract/6649489D9791B73EAAF1C416B003E1CA6A01BB731EF5CA96BB090BF39004C312
```
### GET `/get-predicate/:contract/:address`
Parameters: 
- `:contract` = `[u8; 32]` as hex string. This is the content address of the contract.
- `:address` = `[u8; 32]` as hex string. This is the content address of the predicate.

Returns: `Option<Predicate>` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X GET -H "Content-Type: application/json" http://localhost:59498/get-predicate/6649489D9791B73EAAF1C416B003E1CA6A01BB731EF5CA96BB090BF39004C312/709E80C88487A2411E1EE4DFB9F22A861492D20C4765150C0C794ABD70F8147C
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
### POST `/submit-solution`
Body: `Solution` as JSON \
Returns: `Hash` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X POST -H "Content-Type: application/json" -d '{"data":[{"predicate_to_solve":{"contract":"6649489D9791B73EAAF1C416B003E1CA6A01BB731EF5CA96BB090BF39004C312","predicate":"709E80C88487A2411E1EE4DFB9F22A861492D20C4765150C0C794ABD70F8147C"},"decision_variables":[],"transient_data":[],"state_mutations":[]}]}' http://localhost:59498/submit-solution
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
curl --http2-prior-knowledge -X GET -H "Content-Type: application/json" http://localhost:59498/query-state/6649489D9791B73EAAF1C416B003E1CA6A01BB731EF5CA96BB090BF39004C312/00
```
### GET `/list-winning-blocks`
Query parameters: 
- *Optional* `{ start: u64, end: u64 }`. This is the time range to list contract within. It is inclusive of the start and exclusive of the end.
- *Optional* `{ page: u64 }`. This is the page number to list contracts from. The default is 0.

Returns: `Vec<Block>` as JSON

**Example:**
```bash
curl --http2-prior-knowledge -X GET -H "Content-Type: application/json" "http://localhost:59498/list-winning-blocks?start=0&end=1&page=0"
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
curl --http2-prior-knowledge -X GET -H "Content-Type: application/json" "http://localhost:59498/solution-outcome/421F1ED9E19132757E2DB127FD35E58E08EFE3D77EE2F96FEA60B75D36251EA2"
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
curl --http2-prior-knowledge -X POST -H "Content-Type: application/json" -d '{"data":[{"predicate_to_solve":{"contract":"6649489D9791B73EAAF1C416B003E1CA6A01BB731EF5CA96BB090BF39004C312","predicate":"709E80C88487A2411E1EE4DFB9F22A861492D20C4765150C0C794ABD70F8147C"},"decision_variables":[],"transient_data":[],"state_mutations":[]}]}' http://localhost:59498/check-solution
```
### Post `/check-solution-with-data`
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
curl --http2-prior-knowledge -X POST -H "Content-Type: application/json" -d '{"solution":{"data":[{"predicate_to_solve":{"contract":"6649489D9791B73EAAF1C416B003E1CA6A01BB731EF5CA96BB090BF39004C312","predicate":"709E80C88487A2411E1EE4DFB9F22A861492D20C4765150C0C794ABD70F8147C"},"decision_variables":[],"transient_data":[],"state_mutations":[]}]},"contract":[{"state_read":[],"constraints":[],"directive":"Satisfy"}]}' http://localhost:59498/check-solution-with-data
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
curl --http2-prior-knowledge -X POST -H "Content-Type: application/json" -d '{"state_read":[],"index":0,"solution":{"data":[{"predicate_to_solve":{"contract":"6649489D9791B73EAAF1C416B003E1CA6A01BB731EF5CA96BB090BF39004C312","predicate":"709E80C88487A2411E1EE4DFB9F22A861492D20C4765150C0C794ABD70F8147C"},"decision_variables":[],"transient_data":[],"state_mutations":[]}]},"request_type":{"All":"All"}}' http://localhost:59498/query-state-reads
```
