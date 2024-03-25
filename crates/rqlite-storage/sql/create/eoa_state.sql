CREATE TABLE IF NOT EXISTS eoa_state (
    id INTEGER PRIMARY KEY,
    eoa_id INTEGER NOT NULL,
    key BLOB NOT NULL,
    value INTEGER NOT NULL,
    FOREIGN KEY (eoa_id) REFERENCES eoa (id),
    UNIQUE(eoa_id, key)
);