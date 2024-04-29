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
### POST `/deploy-intent-set`
Body: `Signed<Vec<Intent>>` as JSON \
Returns: `ContentAddress` as JSON

**Example:**
```bash
curl -X POST -H "Content-Type: application/json" \
    -d '{"data":[{"slots":{"decision_variables":0,"state":[]},"state_read":[],"constraints":[],"directive":"Satisfy"}],"signature":[[]]}' \
    http://localhost:59498/deploy-intent-set
```
### GET `/get-intent-set/:address`
Parameters: 
- `:address` = `[u8; 32]` as base64 string. This is the content address of the intent set.

Returns: `Option<Signed<Vec<Intent>>>` as JSON

**Example:**
```bash
curl -X GET -H "Content-Type: application/json" http://localhost:59498/get-intent-set/NsFZ12tS4D5JY2NgfFlAIn9i9OBI3zRLBQFZvJe7o9c=
```
### GET `/get-intent/:set/:address`
Parameters: 
- `:set` = `[u8; 32]` as base64 string. This is the content address of the intent set.
- `:address` = `[u8; 32]` as base64 string. This is the content address of the intent.

Returns: `Option<Intent>` as JSON

**Example:**
```bash
curl -X GET -H "Content-Type: application/json" http://localhost:59498/get-intent/NsFZ12tS4D5JY2NgfFlAIn9i9OBI3zRLBQFZvJe7o9c=/iFVQiq3hbsVz0h5qSF39CnYkCFwaFLXs3WSF3gxoOaQ=
```
### GET `/list-intent-sets`
Query parameters: 
- *Optional* `{ start: u64, end: u64 }`. This is the time range to list set within. It is inclusive of the start and exclusive of the end.
- *Optional* `{ page: u64 }`. This is the page number to list sets from. The default is 0.

Returns: `Vec<Vec<Intent>>` as JSON

**Example:**
```bash
curl -X GET -H "Content-Type: application/json" "http://localhost:59498/list-intent-sets?start=0&end=1&page=0"
```
### POST `/submit-solution`
Body: `Signed<Solution>` as JSON \
Returns: `Hash` as JSON

**Example:**
```bash
curl -X POST -H "Content-Type: application/json" -d '{"data":{"data":[],"state_mutations":[],"partial_solutions":[]},"signature":[[]]}' http://localhost:59498/submit-solution
```
### GET `/list-solutions-pool`
Returns: `Vec<Signed<Solution>>` as JSON

**Example:**
```bash
curl -X GET -H "Content-Type: application/json" "http://localhost:59498/list-solutions-pool" 
```
### GET `/query-state/:address/:key`
Parameters: 
- `:address` = `[u8; 32]` as base64 string. This is the content address of the intent set.
- `:key` = `[u8; 32]` as base64 string. This is the key of the state.

Returns: `Option<Word>` as JSON

**Example:**
```bash
curl -X GET -H "Content-Type: application/json" http://localhost:59498/query-state/NsFZ12tS4D5JY2NgfFlAIn9i9OBI3zRLBQFZvJe7o9c=/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=
```
### GET `/list-winning-blocks`
Query parameters: 
- *Optional* `{ start: u64, end: u64 }`. This is the time range to list set within. It is inclusive of the start and exclusive of the end.
- *Optional* `{ page: u64 }`. This is the page number to list sets from. The default is 0.

Returns: `Vec<Block>` as JSON

**Example:**
```bash
curl -X GET -H "Content-Type: application/json" "http://localhost:59498/list-winning-blocks?start=0&end=1&page=0"
```