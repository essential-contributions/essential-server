CREATE TABLE IF NOT EXISTS contract_state (
    id INTEGER PRIMARY KEY,
    contract_id INTEGER NOT NULL,
    key BLOB NOT NULL,
    value BLOB NOT NULL,
    FOREIGN KEY (contract_id) REFERENCES contracts (id),
    UNIQUE(contract_id, key)
);